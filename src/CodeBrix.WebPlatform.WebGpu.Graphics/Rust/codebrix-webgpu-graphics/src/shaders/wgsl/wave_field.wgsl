// The wave field (Not in C#): a grid of water heights stepped by the wave equation. Three entry
// points, 8 x 8 cells per workgroup: disturb_field (drops and wake paths pressed into both
// states), step_field (one leapfrog step: the next state overwrites the previous one) and
// publish_field (the picture: height interpolated between the last two states, slopes,
// Laplacian). The same arithmetic as src/water/wave_math.rs - keep the two in step.

struct WaveParams {
    // columns, rows, edge (0 reflect, 1 absorb, 2 wrap), flags (1: a mask)
    grid: vec4<u32>,
    // drops, wake stretches, the absorbing band's width, 0
    counts: vec4<u32>,
    // wx, wy, velocity kept per step, level kept per step
    weights: vec4<f32>,
    // cell x, cell y, interpolation fraction, the band's outermost damping
    cells: vec4<f32>,
    // the shallows' damping, size x, size y, 0
    extra: vec4<f32>,
    // x, y, radius, depth
    drops: array<vec4<f32>, 32>,
    // per stretch: a.xy, b.xy / radius, strength, bob depth, 0
    wakes: array<vec4<f32>, 16>,
}

@group(0) @binding(0) var<uniform> params: WaveParams;
@group(0) @binding(1) var<storage, read_write> current: array<f32>;
@group(0) @binding(2) var<storage, read_write> previous: array<f32>;
@group(0) @binding(3) var<storage, read> solid_mask: array<f32>;
@group(0) @binding(4) var surface: texture_storage_2d<rgba16float, write>;

const SOLID: f32 = 0.99;
const CUTOFF_RADII: f32 = 4.0;

fn mask_at(index: u32) -> f32 {
    if ((params.grid.w & 1u) == 0u) {
        return 0.0;
    }
    return solid_mask[index];
}

// The index of cell (i, j) as a neighbour, or -1 when the cell itself stands in for it (outside
// a Reflect or Absorb grid, or solid).
fn neighbour_index(i: i32, j: i32) -> i32 {
    let columns = i32(params.grid.x);
    let rows = i32(params.grid.y);
    var x = i;
    var y = j;
    if (params.grid.z == 2u) {
        x = ((x % columns) + columns) % columns;
        y = ((y % rows) + rows) % rows;
    } else if (x < 0 || y < 0 || x >= columns || y >= rows) {
        return -1;
    }
    let index = y * columns + x;
    if (mask_at(u32(index)) >= SOLID) {
        return -1;
    }
    return index;
}

fn now(i: i32, j: i32, center: f32) -> f32 {
    let n = neighbour_index(i, j);
    if (n < 0) {
        return center;
    }
    return current[n];
}

fn shown_at(index: u32) -> f32 {
    return previous[index] + (current[index] - previous[index]) * params.cells.z;
}

fn shown(i: i32, j: i32, center: f32) -> f32 {
    let n = neighbour_index(i, j);
    if (n < 0) {
        return center;
    }
    return shown_at(u32(n));
}

fn sponge_at(i: u32, j: u32) -> f32 {
    let band = params.counts.z;
    if (band == 0u) {
        return 0.0;
    }
    let d = min(min(i, j), min(params.grid.x - 1u - i, params.grid.y - 1u - j));
    if (d >= band) {
        return 0.0;
    }
    let x = f32(band - d) / f32(band);
    return params.cells.w * x * x;
}

// Abramowitz and Stegun 7.1.26.
fn erf_approx(value: f32) -> f32 {
    let s = select(1.0, -1.0, value < 0.0);
    let x = abs(value);
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let poly = t * (0.2548296 + t * (-0.28449674 + t * (1.4214137 + t * (-1.453152 + t * 1.0614054))));
    return s * (1.0 - poly * exp(-x * x));
}

fn offset(p: vec2<f32>, c: vec2<f32>, wrap: bool) -> vec2<f32> {
    let d = p - c;
    if (wrap) {
        let size = params.extra.yz;
        return d - floor(d / size + 0.5) * size;
    }
    return d;
}

fn gaussian(d: vec2<f32>, radius: f32) -> f32 {
    let r2 = radius * radius;
    let d2 = dot(d, d);
    if (r2 <= 0.0 || d2 >= CUTOFF_RADII * CUTOFF_RADII * r2) {
        return 0.0;
    }
    return exp(-d2 / r2);
}

fn path_profile(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, radius: f32) -> f32 {
    let length_ab = length(b - a);
    if (length_ab < 1e-6 || radius <= 0.0) {
        return 0.0;
    }
    let along = (b - a) / length_ab;
    let w = p - a;
    let t = dot(w, along);
    let d2 = max(dot(w, w) - t * t, 0.0);
    let r2 = radius * radius;
    if (d2 >= CUTOFF_RADII * CUTOFF_RADII * r2) {
        return 0.0;
    }
    return 0.5 * exp(-d2 / r2) * (erf_approx((length_ab - t) / radius) + erf_approx(t / radius));
}

@compute @workgroup_size(8, 8)
fn disturb_field(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.grid.x || id.y >= params.grid.y) {
        return;
    }
    let index = id.y * params.grid.x + id.x;
    if (mask_at(index) >= SOLID) {
        return;
    }
    let p = (vec2<f32>(f32(id.x), f32(id.y)) + 0.5) * params.cells.xy;
    let wrap = params.grid.z == 2u;
    var add = 0.0;
    for (var k = 0u; k < params.counts.x; k = k + 1u) {
        let d = params.drops[k];
        add = add - d.w * gaussian(offset(p, d.xy, wrap), d.z);
    }
    for (var k = 0u; k < params.counts.y; k = k + 1u) {
        let ends = params.wakes[2u * k];
        let look = params.wakes[2u * k + 1u];
        let shift = offset(p, ends.zw, wrap) - (p - ends.zw);
        add = add - look.y * path_profile(p + shift, ends.xy, ends.zw, look.x);
        add = add - look.z * gaussian(offset(p, ends.zw, wrap), look.x);
    }
    current[index] = current[index] + add;
    previous[index] = previous[index] + add;
}

@compute @workgroup_size(8, 8)
fn step_field(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.grid.x || id.y >= params.grid.y) {
        return;
    }
    let index = id.y * params.grid.x + id.x;
    let m = mask_at(index);
    if (m >= SOLID) {
        previous[index] = 0.0;
        return;
    }
    let h = current[index];
    let i = i32(id.x);
    let j = i32(id.y);
    let left = now(i - 1, j, h);
    let right = now(i + 1, j, h);
    let up = now(i, j - 1, h);
    let down = now(i, j + 1, h);
    let w = params.weights;
    let kept = w.z * (1.0 - params.extra.x * max(m, 0.0)) * (1.0 - sponge_at(id.x, id.y));
    previous[index] = w.w * h + kept * (h - previous[index]) + w.x * (left + right - 2.0 * h) + w.y * (up + down - 2.0 * h);
}

@compute @workgroup_size(8, 8)
fn publish_field(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.grid.x || id.y >= params.grid.y) {
        return;
    }
    let index = id.y * params.grid.x + id.x;
    if (mask_at(index) >= SOLID) {
        textureStore(surface, id.xy, vec4<f32>(0.0));
        return;
    }
    let h = shown_at(index);
    let i = i32(id.x);
    let j = i32(id.y);
    let left = shown(i - 1, j, h);
    let right = shown(i + 1, j, h);
    let up = shown(i, j - 1, h);
    let down = shown(i, j + 1, h);
    let cell = params.cells.xy;
    let laplacian = (left + right - 2.0 * h) / (cell.x * cell.x) + (up + down - 2.0 * h) / (cell.y * cell.y);
    textureStore(surface, id.xy, vec4<f32>(h, (right - left) / (2.0 * cell.x), (down - up) / (2.0 * cell.y), laplacian));
}
