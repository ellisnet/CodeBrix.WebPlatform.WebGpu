// ---- noise.wgsl: the GPU noise shared by the paint module (NoiseShape's paint, paint.wgsl
// noise_value) and the stroke module (a stroke set's flow and boil, strokes.wgsl). Appended after
// common.wgsl, before the module's own part.
//
// The planes' value and gradient noise (lattice corners hashed with the seed, quintic fade, fractal octaves with the seed advanced per octave, the sum divided by
// the amplitudes and clamped to -1..1) - the same parameters and character as the CPU's
// DrawingNoise, NOT its exact pattern (32-bit hashing and floats here, 64-bit there). At the end,
// the caustics kind, which IS the CPU's pattern (its CPU twin uses this hash and these floats).
fn noise_hash(seed: u32, x: i32, y: i32, z: i32) -> u32 {
    var h = seed ^ 0x9E3779B9u;
    h = (h ^ u32(x)) * 0x85EBCA6Bu;
    h = (h ^ (h >> 13u) ^ u32(y)) * 0xC2B2AE35u;
    h = (h ^ (h >> 16u) ^ u32(z)) * 0x27D4EB2Fu;
    h = h ^ (h >> 15u);
    h = h * 0x2C1B3C6Du;
    return h ^ (h >> 12u);
}

fn noise_fade(t: vec3<f32>) -> vec3<f32> {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

// The corner's contribution: a value in -1..1, or a gradient (one of 12 directions in 3D, 8 in
// 2D) dotted with the offset.
fn noise_corner(seed: u32, c: vec3<i32>, d: vec3<f32>, gradient: bool, three: bool) -> f32 {
    let h = noise_hash(seed, c.x, c.y, c.z);
    if (!gradient) {
        return f32(h & 0xFFFFu) / 32767.5 - 1.0;
    }
    if (!three) {
        let a = f32(h & 7u) * 0.78539816;
        return (cos(a) * d.x + sin(a) * d.y) * 1.41421356;
    }
    let g = h % 12u;
    let u = select(d.y, d.x, g < 8u);
    let v = select(select(d.z, d.x, g == 12u || g == 14u), d.y, g < 4u);
    return (select(-u, u, (g & 1u) == 0u) + select(-v, v, (g & 2u) == 0u));
}

fn noise_once(seed: u32, q: vec3<f32>, gradient: bool, three: bool) -> f32 {
    let i = vec3<i32>(floor(q));
    let f = q - floor(q);
    let u = noise_fade(f);
    var corners: array<f32, 8>;
    for (var k = 0u; k < 8u; k++) {
        let o = vec3<i32>(i32(k & 1u), i32((k >> 1u) & 1u), i32((k >> 2u) & 1u));
        corners[k] = noise_corner(seed, i + o, f - vec3<f32>(o), gradient, three);
    }
    let x00 = mix(corners[0], corners[1], u.x);
    let x10 = mix(corners[2], corners[3], u.x);
    let x01 = mix(corners[4], corners[5], u.x);
    let x11 = mix(corners[6], corners[7], u.x);
    let y0 = mix(x00, x10, u.y);
    let y1 = mix(x01, x11, u.y);
    return clamp(select(y0, mix(y0, y1, u.z), three), -1.0, 1.0);
}


// The caustics kind (Not in C#): the light net of a pool floor. THE SAME CODE as the CPU's
// planes/drawing_caustics.rs (DrawingNoise::caustics), so the CPU and the GPU show the same net
// within float rounding. Change both together.
// One cell's point at a time, as an offset from the cell's corner (0.2 to 0.8 on each axis).
fn caustics_cell_point(seed: u32, cell: vec2<i32>, z: f32) -> vec2<f32> {
    let h = noise_hash(seed, cell.x, cell.y, 0);
    let rest = vec2<f32>(0.35) + 0.3 * vec2<f32>(f32(h & 255u), f32((h >> 8u) & 255u)) / 255.0;
    let start = f32((h >> 16u) & 1023u) / 1024.0 * 6.28318530717958647692;
    let rate = 0.5 + f32((h >> 26u) & 31u) / 31.0;
    let turn = select(rate, -rate, (h >> 31u) != 0u);
    let angle = start + turn * z;
    return rest + 0.15 * vec2<f32>(cos(angle), sin(angle));
}

// One octave of the caustics pattern in -1..1: a two-sine warp, the nearest two of the 3 x 3
// cells' points, the line along F2 - F1 = 0.
fn caustics_once(seed: u32, q: vec2<f32>, z: f32) -> f32 {
    let w = vec2<f32>(q.x + 0.3 * sin(0.73 * q.y + 0.61 * z), q.y + 0.3 * sin(0.67 * q.x - 0.53 * z));
    let i = floor(w);
    let f = w - i;
    let c = vec2<i32>(i);
    // Any distance beyond the 3 x 3 cells (all are under 3); a literal at f32's very limit is
    // refused by the browser's WGSL compiler.
    var f1 = 1.0e9;
    var f2 = 1.0e9;
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let o = vec2<f32>(f32(dx), f32(dy)) + caustics_cell_point(seed, c + vec2<i32>(dx, dy), z) - f;
            let d = sqrt(o.x * o.x + o.y * o.y);
            if (d < f1) {
                f2 = f1;
                f1 = d;
            } else if (d < f2) {
                f2 = d;
            }
        }
    }
    let line = 1.0 - smoothstep(0.0, 0.22, f2 - f1);
    return 2.0 * line * line - 1.0;
}
