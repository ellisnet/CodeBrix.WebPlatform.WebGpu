// The water surface (Not in C#): a grid of quads made from the vertex index, moved by a wave
// field's heights and by open-water (Gerstner) waves; per pixel the colour of the scene captured
// behind it, refracted by the slopes, tinted by the real depth of water, with caustics on the
// bottom, a Fresnel reflection of the environment and of what is on screen, sun glints and local
// lights' highlights, the sun's shadow, an optional thin-film sheen and a soft shore; optionally
// shaped by a coverage mask (entry point fs_masked) and carried by a flow. The CPU twins of the
// formulas are in src/water/water_math.rs and gerstner_math.rs: keep them in step.
// THIS BODY IS NOT A WHOLE SHADER: the sun's map (group 3, binding 0) and `sun_compare` come from
// water_sun_map.wgsl (one map, texture_depth_2d) or water_sun_cascades.wgsl (a layer per cascade,
// texture_depth_2d_array), put after it (WaterShaders::SURFACE and SURFACE_CASCADED).
// Every texture read is textureSampleLevel / textureLoad / textureSampleCompareLevel (no implicit
// derivatives inside branches); the only derivatives are taken first, in uniform control flow.

const PI: f32 = 3.14159265358979;
const TAU: f32 = 6.28318530717959;
const MAX_WAVES: u32 = 16u;
const MAX_LOCAL_LIGHTS: u32 = 16u;
const FAR_PATH: f32 = 10000.0;
const WATER_F0: f32 = 0.02;
const MARCH_STEPS: u32 = 24u;
const REFINE_STEPS: u32 = 4u;

struct LocalLight {
    position: vec4<f32>,
    color: vec4<f32>,
    direction: vec4<f32>,
    extra: vec4<f32>,
};

struct Wave {
    // cos, sin of the direction, wavenumber, amplitude.
    direction: vec4<f32>,
    // horizontal radius, current phase, wavelength, 0.
    motion: vec4<f32>,
};

struct Water {
    view_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    camera: vec4<f32>,
    viewport: vec4<f32>,
    key_direction: vec4<f32>,
    key_color: vec4<f32>,
    environment: vec4<f32>,
    ambient_sky: vec4<f32>,
    ambient_ground: vec4<f32>,
    ambient_up: vec4<f32>,
    surface: vec4<f32>,
    extent: vec4<f32>,
    field: vec4<f32>,
    field_extent: vec4<f32>,
    absorption: vec4<f32>,
    scattering: vec4<f32>,
    look: vec4<f32>,
    caustics: vec4<f32>,
    // x, y: the net's two motion phases; z, w: the flow's shifts of its two warps.
    caustic_phase: vec4<f32>,
    detail_phase: vec4<f32>,
    detail_waves: array<vec4<f32>, 4>,
    sheen: vec4<f32>,
    sheen_phase: vec4<f32>,
    switches: vec4<u32>,
    counts: vec4<u32>,
    sun_matrix: mat4x4<f32>,
    sun_shadow: vec4<f32>,
    waves: array<Wave, 16>,
    local_lights: array<LocalLight, 16>,
    // From here on all zero when unused. x: 1 with a coverage mask; y: its threshold.
    mask: vec4<f32>,
    // x: 1 when the bound cube is the reflection environment; y: its intensity; z: its last mip
    // level; w: its rotation about +Y.
    reflection: vec4<f32>,
    // xy: the flow's velocity (world x, z; for a later flow map); zw: the caustic net's offset by
    // the flow within a cell.
    flow: vec4<f32>,
    // x: the sun's cascades (0 or 1: the single map); y, z: the caustic net's whole-cell offset by
    // the flow (i32 bits).
    extra: vec4<u32>,
    // The sun's cascades, as the model shader's: xyz the camera's view direction, w the blend band;
    // each cascade's far end, depth bias, normal offset and view-projection.
    sun_view: vec4<f32>,
    sun_splits: vec4<f32>,
    sun_biases: vec4<f32>,
    sun_offsets: vec4<f32>,
    sun_cascade_matrices: array<mat4x4<f32>, 4>,
};

@group(0) @binding(0) var<uniform> water: Water;
@group(1) @binding(0) var scene_color: texture_2d<f32>;
@group(1) @binding(1) var scene_depth: texture_2d<f32>;
@group(1) @binding(2) var linear_sampler: sampler;
@group(2) @binding(0) var environment_map: texture_cube<f32>;
@group(2) @binding(1) var field_map: texture_2d<f32>;
// The coverage mask (read by fs_masked only; its pipelines' layout has it).
@group(2) @binding(2) var mask_map: texture_2d<f32>;
// group 3, binding 0: the sun's map (water_sun_map.wgsl or water_sun_cascades.wgsl).
@group(3) @binding(1) var shadow_sampler: sampler_comparison;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    // The displaced surface point.
    @location(0) world: vec3<f32>,
    // The still-water point it came from, in world x and z.
    @location(1) still: vec2<f32>,
    // Where it lies in the wave field's picture.
    @location(2) field_uv: vec2<f32>,
};

// A local vector of the surface in world axes (rotation about +Y; glam's Mat3::from_rotation_y).
fn rotate_y(v: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(c * v.x + s * v.z, v.y, -s * v.x + c * v.z);
}

// The share of a wave's normal kept where a pixel covers `footprint` units (OpenWaterWaves::normal_fade).
fn normal_fade(wavelength: f32, footprint: f32) -> f32 {
    if (footprint <= 0.0) {
        return 1.0;
    }
    let x = clamp((wavelength / footprint - 2.0) / 2.0, 0.0, 1.0);
    return x * x * (3.0 - 2.0 * x);
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let grid = max(u32(water.extent.z), 1u);
    let quad = index / 6u;
    let corner = index % 6u;
    var corner_x = array<u32, 6>(0u, 1u, 1u, 0u, 1u, 0u);
    var corner_y = array<u32, 6>(0u, 0u, 1u, 0u, 1u, 1u);
    let u = f32((quad % grid) + corner_x[corner]) / f32(grid);
    let v = f32((quad / grid) + corner_y[corner]) / f32(grid);
    let local = vec3<f32>((u - 0.5) * water.extent.x, 0.0, (v - 0.5) * water.extent.y);
    let still = water.surface.xyz + rotate_y(local, water.surface.w);
    var position = still;
    // The wave field's height (the 1 x 1 placeholder reads 0 without a field).
    let field = textureSampleLevel(field_map, linear_sampler, vec2<f32>(u, v), 0.0);
    position.y += field.r * water.field.y * water.field.x * water.field.z;
    // The open-water waves long enough for the grid (shorter ones only turn the normals).
    let p = still.xz;
    let count = min(water.counts.x, MAX_WAVES);
    for (var i = 0u; i < count; i++) {
        let w = water.waves[i];
        if (w.motion.z >= 4.0 * water.extent.w) {
            let a = w.direction.z * dot(w.direction.xy, p) - w.motion.y;
            let s = sin(a);
            position += vec3<f32>(-w.motion.x * w.direction.x * s, w.direction.w * cos(a), -w.motion.x * w.direction.y * s);
        }
    }
    var output: VertexOutput;
    output.clip = water.view_projection * vec4<f32>(position, 1.0);
    output.world = position;
    output.still = p;
    output.field_uv = vec2<f32>(u, v);
    return output;
}

fn decode_srgb(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

fn encode_srgb(value: vec3<f32>) -> vec3<f32> {
    let c = clamp(value, vec3<f32>(0.0), vec3<f32>(1.0));
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(high, low, c <= vec3<f32>(0.0031308));
}

// The captured scene's colour at `uv`, linear (an 8-bit linear target holds sRGB-encoded values).
fn scene_at(uv: vec2<f32>) -> vec3<f32> {
    let c = textureSampleLevel(scene_color, linear_sampler, uv, 0.0).rgb;
    if (water.switches.w != 0u) {
        return decode_srgb(c);
    }
    return c;
}

fn pixel_of(uv: vec2<f32>) -> vec2<i32> {
    let size = vec2<i32>(water.viewport.xy);
    return clamp(vec2<i32>(uv * water.viewport.xy), vec2<i32>(0), size - vec2<i32>(1));
}

fn depth_at(uv: vec2<f32>) -> f32 {
    return textureLoad(scene_depth, pixel_of(uv), 0).r;
}

fn reconstruct(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let world = water.inverse_view_projection * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    return world.xyz / world.w;
}

// A world point's screen uv (xy), depth (z), and w < 0 behind the camera.
fn project(world: vec3<f32>) -> vec4<f32> {
    let clip = water.view_projection * vec4<f32>(world, 1.0);
    if (clip.w <= 1e-6) {
        return vec4<f32>(0.0, 0.0, 0.0, -1.0);
    }
    let ndc = clip.xyz / clip.w;
    return vec4<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5, ndc.z, 1.0);
}

// World direction -> environment direction (the environment turns about +Y).
fn to_environment(direction: vec3<f32>) -> vec3<f32> {
    let c = cos(water.environment.w);
    let s = sin(water.environment.w);
    return vec3<f32>(c * direction.x - s * direction.z, direction.y, s * direction.x + c * direction.z);
}

fn hemisphere(direction: vec3<f32>) -> vec3<f32> {
    let t = clamp(dot(direction, water.ambient_up.xyz) * 0.5 + 0.5, 0.0, 1.0);
    return mix(water.ambient_ground.rgb, water.ambient_sky.rgb, t);
}

// The sky's light in `direction` at `roughness` (before the exposure).
fn environment_light(direction: vec3<f32>, roughness: f32) -> vec3<f32> {
    let sampled = textureSampleLevel(environment_map, linear_sampler, to_environment(direction), roughness * water.environment.z).rgb * water.environment.y;
    return select(hemisphere(direction), sampled, water.environment.x > 0.5);
}

// What the water reflects in `direction` at `roughness` (before the exposure): the reflection
// environment when one is bound (whatever the ambient), else the sky's light.
fn reflection_light(direction: vec3<f32>, roughness: f32) -> vec3<f32> {
    if (water.reflection.x > 0.5) {
        let c = cos(water.reflection.w);
        let s = sin(water.reflection.w);
        let turned = vec3<f32>(c * direction.x - s * direction.z, direction.y, s * direction.x + c * direction.z);
        return textureSampleLevel(environment_map, linear_sampler, turned, roughness * water.reflection.z).rgb * water.reflection.y;
    }
    return environment_light(direction, roughness);
}

// How much of the sun's light reaches a point through one map (layer `layer` of the cascades; 0
// for the single map) seen through `matrix`, with its bias and normal offset (1 outside it).
fn sun_map_visibility(layer: i32, matrix: mat4x4<f32>, bias: f32, normal_offset: f32, world: vec3<f32>, normal: vec3<f32>) -> f32 {
    let grid = i32(water.counts.z);
    let clip = matrix * vec4<f32>(world + normal * normal_offset, 1.0);
    if (clip.w <= 0.0) {
        return 1.0;
    }
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || ndc.z < 0.0 || ndc.z > 1.0) {
        return 1.0;
    }
    let start = -0.5 * f32(grid - 1);
    var sum = 0.0;
    for (var y = 0; y < grid; y++) {
        for (var x = 0; x < grid; x++) {
            let offset = vec2<f32>(start + f32(x), start + f32(y)) * water.sun_shadow.y;
            sum += sun_compare(uv + offset, layer, ndc.z - bias);
        }
    }
    return mix(1.0, sum / f32(grid * grid), water.sun_shadow.x);
}

// The cascade for a point at `depth` along the camera's view: the model shader's cascade_pick
// (pbr_body.wgsl; the same text but for the uniform's name - a host test compares them; twin:
// lighting::shadows::cascade_pick).
fn cascade_pick(depth: f32, cascades: u32) -> vec2<f32> {
    var start = 0.0;
    for (var index = 0u; index < cascades; index++) {
        let end = water.sun_splits[index];
        if (depth < end) {
            let band = water.sun_view.w * (end - start);
            let weight = select(0.0, clamp((depth - (end - band)) / max(band, 1e-6), 0.0, 1.0), band > 0.0);
            return vec2<f32>(f32(index), weight);
        }
        start = end;
    }
    return vec2<f32>(-1.0, 0.0);
}

fn sun_cascade_visibility(cascade: u32, world: vec3<f32>, normal: vec3<f32>) -> f32 {
    return sun_map_visibility(i32(cascade), water.sun_cascade_matrices[cascade], water.sun_biases[cascade], water.sun_offsets[cascade], world, normal);
}

// How much of the sun's light reaches a point (1 without the map or outside it): the single map,
// or with cascades the one the point's depth picks, blended into the next in its band as the
// model shader does (the last fading out to 1 towards the reach; 1 beyond it).
fn sun_visibility(world: vec3<f32>, normal: vec3<f32>) -> f32 {
    if (i32(water.counts.z) <= 0) {
        return 1.0;
    }
    let cascades = min(water.extra.x, 4u);
    if (cascades <= 1u) {
        return sun_map_visibility(0, water.sun_matrix, water.sun_shadow.z, water.sun_shadow.w, world, normal);
    }
    let pick = cascade_pick(dot(world - water.camera.xyz, water.sun_view.xyz), cascades);
    if (pick.x < 0.0) {
        return 1.0;
    }
    let cascade = u32(pick.x);
    let near = sun_cascade_visibility(cascade, world, normal);
    if (pick.y <= 0.0) {
        return near;
    }
    var next = 1.0;
    if (cascade + 1u < cascades) {
        next = sun_cascade_visibility(cascade + 1u, world, normal);
    }
    return mix(near, next, pick.y);
}

fn hash_u32(x: u32) -> u32 {
    let state = x * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn hash_cell(cell: vec2<i32>, seed: u32) -> vec2<f32> {
    let h = hash_u32(bitcast<u32>(cell.x) ^ hash_u32(bitcast<u32>(cell.y) + seed * 0x9E3779B9u));
    let g = hash_u32(h ^ 0x68E31DA4u);
    return vec2<f32>(f32(h >> 8u), f32(g >> 8u)) / 16777216.0;
}

// The caustic net (water_math::caustic_net_moved): bright thin edges of moving cells, 0..1;
// carried by the flow through `shifts` (its warps' phases) and `cells` (whole cells taken from the
// hash), `p` already less the offset within a cell (all zero without flow).
fn caustic_net(p: vec2<f32>, phase_a: f32, phase_b: f32, shifts: vec2<f32>, cells: vec2<i32>, seed: u32) -> f32 {
    let q = p + 0.35 * vec2<f32>(sin(p.y * 1.7 + phase_b + shifts.x), sin(p.x * 1.3 - phase_b + 1.0 + shifts.y));
    let cell = floor(q);
    let f = q - cell;
    var d1 = 8.0;
    var d2 = 8.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let c = vec2<i32>(cell) - cells + vec2<i32>(x, y);
            let h = hash_cell(c, seed);
            let point = vec2<f32>(f32(x), f32(y)) + vec2<f32>(0.5) + 0.38 * vec2<f32>(sin(phase_a + TAU * h.x), sin(phase_b + TAU * h.y));
            let d = distance(point, f);
            if (d < d1) {
                d2 = d1;
                d1 = d;
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    let x = clamp((d2 - d1) / 0.18, 0.0, 1.0);
    let line = 1.0 - x * x * (3.0 - 2.0 * x);
    return line * line;
}

fn distribution_ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let alpha2 = alpha * alpha;
    let f = n_dot_h * n_dot_h * (alpha2 - 1.0) + 1.0;
    return alpha2 / (PI * f * f);
}

fn visibility_smith(n_dot_l: f32, n_dot_v: f32, alpha: f32) -> f32 {
    let alpha2 = alpha * alpha;
    let ggx_v = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - alpha2) + alpha2);
    let ggx_l = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - alpha2) + alpha2);
    let sum = ggx_v + ggx_l;
    return select(0.0, 0.5 / sum, sum > 0.0);
}

// The highlight of light `color` from direction `l` (towards the light) on the water.
fn highlight(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, color: vec3<f32>, alpha: f32) -> vec3<f32> {
    let n_dot_l = dot(n, l);
    if (!(n_dot_l > 0.0)) {
        return vec3<f32>(0.0);
    }
    let h = normalize(l + v);
    let n_dot_v = clamp(dot(n, v), 0.0001, 1.0);
    let v_dot_h = clamp(dot(v, h), 0.0, 1.0);
    let f = WATER_F0 + (1.0 - WATER_F0) * pow(1.0 - v_dot_h, 5.0);
    return color * (f * distribution_ggx(clamp(dot(n, h), 0.0, 1.0), alpha) * visibility_smith(n_dot_l, n_dot_v, alpha) * PI * n_dot_l);
}

fn local_falloff(distance: f32, inverse_range: f32) -> f32 {
    let x = distance * inverse_range;
    let x2 = x * x;
    let window = clamp(1.0 - x2 * x2, 0.0, 1.0);
    let d = max(distance, 0.01);
    return window * window / (d * d);
}

fn local_cone(cosine: f32, cos_inner: f32, cos_outer: f32) -> f32 {
    let t = clamp((cosine - cos_outer) / max(cos_inner - cos_outer, 1e-4), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

// How far a reflection found at `uv` along `r` counts (water_math::edge_fade x facing_fade).
fn reflection_fade(uv: vec2<f32>, r: vec3<f32>, v: vec3<f32>) -> f32 {
    let e = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    let x = clamp(e / 0.1, 0.0, 1.0);
    let edge = x * x * (3.0 - 2.0 * x);
    let back = clamp(1.0 - 2.0 * dot(r, v), 0.0, 1.0);
    let up = clamp((r.y + 0.05) / 0.1, 0.0, 1.0);
    return edge * back * up;
}

// How far the ray at `t` lies behind the scene surface on screen (xy: uv; z: the distance behind,
// below -1e30 for sky; w < 0 off the screen or behind the camera). water_math::march's `behind`.
fn behind(start: vec3<f32>, r: vec3<f32>, t: f32) -> vec4<f32> {
    let q = start + r * t;
    let at = project(q);
    if (at.w < 0.0 || any(at.xy < vec2<f32>(0.0)) || any(at.xy > vec2<f32>(1.0))) {
        return vec4<f32>(0.0, 0.0, 0.0, -1.0);
    }
    let scene = depth_at(at.xy);
    if (scene >= 1.0) {
        return vec4<f32>(at.xy, -1e30, 1.0);
    }
    let eye = reconstruct(at.xy, 0.0);
    let surface = reconstruct(at.xy, scene);
    return vec4<f32>(at.xy, distance(eye, q) - distance(eye, surface), 1.0);
}

// The screen-space reflection's march (water_math::march): xy the hit's uv, z 1 for a hit.
fn march(start: vec3<f32>, r: vec3<f32>, max_distance: f32) -> vec3<f32> {
    var previous = 0.0;
    for (var i = 1u; i <= MARCH_STEPS; i++) {
        let s = f32(i) / f32(MARCH_STEPS);
        let t = max_distance * s * s;
        let b = behind(start, r, t);
        if (b.w < 0.0) {
            break;
        }
        let thickness = max(2.0 * (t - previous), 0.05);
        if (b.z > 0.0 && b.z < thickness) {
            var low = previous;
            var high = t;
            var found = b.xy;
            for (var k = 0u; k < REFINE_STEPS; k++) {
                let middle = 0.5 * (low + high);
                let m = behind(start, r, middle);
                if (m.w >= 0.0 && m.z > 0.0) {
                    high = middle;
                    found = m.xy;
                } else {
                    low = middle;
                }
            }
            return vec3<f32>(found, 1.0);
        }
        previous = t;
    }
    return vec3<f32>(0.0);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Derivatives first, in uniform control flow: how much of the world one pixel covers.
    let footprint = max(length(dpdx(input.world)), length(dpdy(input.world)));
    return shade(input, footprint, 1.0);
}

// The surface shaped by its coverage mask (water_math::mask_opacity): nothing at or below the
// threshold (no colour, no depth), above it the water faded in over the scene behind it.
@fragment
fn fs_masked(input: VertexOutput) -> @location(0) vec4<f32> {
    let footprint = max(length(dpdx(input.world)), length(dpdy(input.world)));
    let coverage = textureSampleLevel(mask_map, linear_sampler, input.field_uv, 0.0).r;
    let threshold = water.mask.y;
    if (!(coverage > threshold)) {
        discard;
    }
    let x = clamp((coverage - threshold) / max(1.0 - threshold, 1e-6), 0.0, 1.0);
    return shade(input, footprint, x * x * (3.0 - 2.0 * x));
}

// One pixel of water; `opacity` (1 without a mask) fades it over the scene behind it.
fn shade(input: VertexOutput, footprint: f32, opacity: f32) -> vec4<f32> {
    let uv = input.clip.xy * water.viewport.zw;
    let exposure = water.camera.w;
    let up = vec3<f32>(0.0, 1.0, 0.0);

    // THE NORMAL: the field's slopes, the open-water waves' analytic normal, the detail ripples.
    let field = textureSampleLevel(field_map, linear_sampler, input.field_uv, 0.0);
    let field_scale = water.field.x * water.field.y * water.field_extent.xy / max(water.extent.xy, vec2<f32>(1e-6));
    let local_slope = field.gb * field_scale;
    let field_slope = rotate_y(vec3<f32>(local_slope.x, 0.0, local_slope.y), water.surface.w);
    var slope = field_slope.xz;
    var tx = vec3<f32>(1.0, 0.0, 0.0);
    var tz = vec3<f32>(0.0, 0.0, 1.0);
    let count = min(water.counts.x, MAX_WAVES);
    for (var i = 0u; i < count; i++) {
        let w = water.waves[i];
        let fade = normal_fade(w.motion.z, footprint);
        let a = w.direction.z * dot(w.direction.xy, input.still) - w.motion.y;
        let hk = w.motion.x * w.direction.z * cos(a) * fade;
        let ak = w.direction.w * w.direction.z * sin(a) * fade;
        let dx = w.direction.x;
        let dz = w.direction.y;
        tx += vec3<f32>(-hk * dx * dx, -ak * dx, -hk * dx * dz);
        tz += vec3<f32>(-hk * dx * dz, -ak * dz, -hk * dz * dz);
    }
    let wave_normal = normalize(cross(tz, tx));
    slope += -wave_normal.xz / max(wave_normal.y, 0.05);
    for (var i = 0u; i < 4u; i++) {
        let w = water.detail_waves[i];
        let fade = normal_fade(TAU / max(w.z, 1e-6), footprint);
        let argument = w.z * dot(w.xy, input.world.xz) - water.detail_phase[i];
        slope += w.xy * (w.w * cos(argument) * fade);
    }
    let n = normalize(vec3<f32>(-slope.x, 1.0, -slope.y));
    let steepness = min(length(slope), 0.6);

    // THE VIEW: from the pixel's ray (right for both projections).
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let near4 = water.inverse_view_projection * vec4<f32>(ndc, 0.0, 1.0);
    let far4 = water.inverse_view_projection * vec4<f32>(ndc, 1.0, 1.0);
    let v = normalize(near4.xyz / near4.w - far4.xyz / far4.w);
    let p = input.world;

    // THE KEY LIGHT and the light the water itself is lit by.
    let key_lit = water.key_direction.w > 0.5;
    let l = -water.key_direction.xyz;
    var surface_shadow = 1.0;
    if (key_lit && water.switches.z != 0u) {
        surface_shadow = sun_visibility(p, up);
    }
    let sky_light = environment_light(up, 1.0);
    let light = select(vec3<f32>(0.0), water.key_color.rgb * max(l.y, 0.0) * surface_shadow, key_lit) + sky_light;

    // REFRACTION: the view of the bottom shifted by the slopes, unless that finds something in
    // front of the water (then the unshifted view).
    let water_depth = input.clip.z;
    let depth0 = depth_at(uv);
    let bottom0 = reconstruct(uv, depth0);
    let path0 = select(distance(p, bottom0), FAR_PATH, depth0 >= 1.0);
    let shift = -vec3<f32>(n.x, 0.0, n.z) * (water.scattering.w * min(path0, 2.0));
    let shifted = project(p + shift);
    let uv1 = clamp(select(uv, shifted.xy, shifted.w > 0.0), vec2<f32>(0.0), vec2<f32>(1.0));
    let depth1 = depth_at(uv1);
    let use_shift = depth1 >= water_depth;
    let uv_r = select(uv, uv1, use_shift);
    let depth = select(depth0, depth1, use_shift);
    let sky_behind = depth >= 1.0;
    let bottom = reconstruct(uv_r, depth);
    let path = select(distance(p, bottom), FAR_PATH, sky_behind);
    var refracted = scene_at(uv_r);
    let unshifted = scene_at(uv);

    // CAUSTICS on the bottom: the net where the sun's light entered the water above it.
    if (water.caustics.x > 0.0 && key_lit && !sky_behind && l.y > 0.0) {
        let pixel = pixel_of(uv_r);
        let size = vec2<i32>(water.viewport.xy);
        let px = min(pixel + vec2<i32>(1, 0), size - vec2<i32>(1));
        let py = min(pixel + vec2<i32>(0, 1), size - vec2<i32>(1));
        let uv_x = (vec2<f32>(px) + vec2<f32>(0.5)) * water.viewport.zw;
        let uv_y = (vec2<f32>(py) + vec2<f32>(0.5)) * water.viewport.zw;
        let bx = reconstruct(uv_x, textureLoad(scene_depth, px, 0).r);
        let by = reconstruct(uv_y, textureLoad(scene_depth, py, 0).r);
        var bottom_n = normalize(cross(by - bottom, bx - bottom));
        if (dot(bottom_n, v) < 0.0) {
            bottom_n = -bottom_n;
        }
        let vertical = max(water.surface.y - bottom.y, 0.0);
        let sun_in_water = refract(water.key_direction.xyz, up, 1.0 / 1.33);
        let entry = bottom - sun_in_water * (vertical / min(sun_in_water.y, -0.05));
        let cells = bitcast<vec2<i32>>(water.extra.yz);
        let net = caustic_net(entry.xz * water.caustics.y - water.flow.zw, water.caustic_phase.x, water.caustic_phase.y, water.caustic_phase.zw, cells, water.counts.w);
        // Focusing by the wave field's curvature: brighter under crests (negative Laplacian).
        let entry_local = rotate_y(entry - water.surface.xyz, -water.surface.w);
        let entry_uv = entry_local.xz / max(water.extent.xy, vec2<f32>(1e-6)) + vec2<f32>(0.5);
        let ratio = water.field_extent.x / max(water.extent.x, 1e-6);
        let laplacian = textureSampleLevel(field_map, linear_sampler, entry_uv, 0.0).a * water.field.x * water.field.y * ratio * ratio;
        let focus = clamp(1.0 - water.caustics.z * laplacian, 0.2, 3.0);
        let fade = exp(-vertical / max(water.caustics.w, 1e-3));
        let facing = max(dot(bottom_n, l), 0.0);
        var shadow = 1.0;
        if (water.switches.z != 0u) {
            shadow = sun_visibility(bottom, bottom_n);
        }
        refracted *= 1.0 + water.caustics.x * net * focus * fade * facing * shadow;
    }

    // TINT BY THE REAL DEPTH: absorbed on the way, the scattered colour taking over.
    let transmit = exp(-max(water.absorption.rgb, vec3<f32>(0.0)) * path);
    let scattered = water.scattering.rgb * light * exposure * (1.0 - exp(-max(water.absorption.w, 0.0) * path));
    var color = refracted * transmit + scattered;

    // REFLECTION: the environment, and what is on screen.
    let roughness = clamp(water.look.y, 0.0, 1.0);
    let r = reflect(-v, n);
    var reflected = reflection_light(r, roughness) * exposure;
    if (water.switches.x != 0u) {
        let reach = clamp(distance(water.camera.xyz, p) * 2.0, 1.0, 500.0);
        let hit = march(p, r, reach);
        if (hit.z > 0.5) {
            let weight = reflection_fade(hit.xy, r, v);
            reflected = mix(reflected, scene_at(hit.xy), weight);
        }
    }
    let n_dot_v = clamp(dot(n, v), 0.0, 1.0);
    let fresnel = clamp((WATER_F0 + (1.0 - WATER_F0) * pow(1.0 - n_dot_v, 5.0)) * water.look.x, 0.0, 1.0);
    color = color * (1.0 - fresnel) + reflected * fresnel;

    // HIGHLIGHTS: the sun's glints and the local lights'.
    let alpha = max(roughness * roughness, 0.0001);
    var glints = vec3<f32>(0.0);
    if (key_lit) {
        glints += highlight(n, v, l, water.key_color.rgb, alpha) * (water.look.z * surface_shadow);
    }
    if (water.switches.y != 0u) {
        let local_count = min(water.counts.y, MAX_LOCAL_LIGHTS);
        for (var i = 0u; i < local_count; i++) {
            let light_i = water.local_lights[i];
            let to_light = light_i.position.xyz - p;
            let distance_i = length(to_light);
            let li = to_light / max(distance_i, 1e-6);
            var attenuation = local_falloff(distance_i, light_i.position.w);
            if (light_i.color.w > 0.5) {
                attenuation *= local_cone(dot(-li, light_i.direction.xyz), light_i.extra.x, light_i.direction.w);
            }
            if (attenuation > 0.0) {
                glints += highlight(n, v, li, light_i.color.rgb * attenuation, alpha);
            }
        }
    }
    color += glints * exposure;

    // THE SHEEN (optional): thin-film colours drifting in bands, stronger where the water slopes.
    if (water.sheen.x > 0.0) {
        let height = p.y - water.surface.y;
        let band = sin(dot(p.xz, vec2<f32>(2.2, 1.7)) * water.sheen.y + water.sheen_phase.y);
        let film = 0.3 + height * water.sheen.w + steepness * water.sheen.z + water.sheen_phase.x + 0.25 * band;
        let tint = vec3<f32>(0.5) + 0.5 * cos(TAU * (vec3<f32>(film) + vec3<f32>(0.0, 0.33, 0.67)));
        color = max(color + (tint - vec3<f32>(0.45)) * ((0.10 + 0.9 * steepness) * 0.5 * water.sheen.x) * light * exposure, vec3<f32>(0.0));
    }

    // THE SOFT SHORE: water too thin to show its own look shows the scene as it is.
    var shore = 1.0;
    if (water.look.w > 0.0) {
        let x = clamp(path / water.look.w, 0.0, 1.0);
        shore = x * x * (3.0 - 2.0 * x);
    }
    color = mix(unshifted, color, shore * opacity);

    if (water.switches.w != 0u) {
        return vec4<f32>(encode_srgb(color), 1.0);
    }
    return vec4<f32>(color, 1.0);
}
