
// ---- paint.wgsl: stencil, cover (the paints) and clear ----

const KIND_SOLID: u32 = 0u;
const KIND_LINEAR: u32 = 1u;
const KIND_RADIAL: u32 = 2u;
const KIND_SWEEP: u32 = 3u;
const KIND_IMAGE: u32 = 4u;
const KIND_NOISE: u32 = 6u;
const KIND_OWNERS: u32 = 7u;
const RAMP_SAMPLES: u32 = 512u;

// Stencil passes write no colour (their pipelines' write mask is empty).
@fragment
fn fs_stencil() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}

// The clear of a target's used area (colour and stencil).
@fragment
fn fs_clear() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}

// From vello's fine.wgsl.
const EXTEND_PAD: u32 = 0u;
const EXTEND_REPEAT: u32 = 1u;
const EXTEND_REFLECT: u32 = 2u;
fn extend_mode_normalized(t: f32, mode: u32) -> f32 {
    switch mode {
        case EXTEND_PAD: {
            return clamp(t, 0.0, 1.0);
        }
        case EXTEND_REPEAT: {
            return fract(t);
        }
        case EXTEND_REFLECT, default: {
            return abs(t - 2.0 * round(0.5 * t));
        }
    }
}

fn ramp_at(slot: u32, t: f32) -> vec4<f32> {
    let x = t * f32(RAMP_SAMPLES - 1u);
    let i = u32(clamp(floor(x), 0.0, f32(RAMP_SAMPLES - 1u)));
    let j = min(i + 1u, RAMP_SAMPLES - 1u);
    let base = slot * RAMP_SAMPLES;
    return mix(ramps[base + i], ramps[base + j], clamp(x - f32(i), 0.0, 1.0));
}

const RAD_GRAD_KIND_CIRCULAR: u32 = 1u;
const RAD_GRAD_KIND_STRIP: u32 = 2u;
const RAD_GRAD_KIND_FOCAL_ON_CIRCLE: u32 = 3u;
const RAD_GRAD_SWAPPED: u32 = 1u;

// The gradient parameter at a paint-space point and whether there is one (y), after vello.
fn gradient_t(p: Paint, local_xy: vec2<f32>) -> vec2<f32> {
    let kind = p.info.x;
    let extend = p.info.y & 15u;
    if (kind == KIND_LINEAR) {
        return vec2<f32>(extend_mode_normalized(local_xy.x, extend), 1.0);
    }
    if (kind == KIND_RADIAL) {
        let focal_x = p.m1.z;
        let radius = p.m1.w;
        let rad_kind = p.info.w & 7u;
        let is_strip = rad_kind == RAD_GRAD_KIND_STRIP;
        let is_circular = rad_kind == RAD_GRAD_KIND_CIRCULAR;
        let is_focal_on_circle = rad_kind == RAD_GRAD_KIND_FOCAL_ON_CIRCLE;
        let is_swapped = ((p.info.w >> 3u) & RAD_GRAD_SWAPPED) != 0u;
        let r1_recip = select(1.0 / radius, 0.0, is_circular);
        let less_scale = select(1.0, -1.0, is_swapped || (1.0 - focal_x) < 0.0);
        let t_sign = sign(1.0 - focal_x);
        let x = local_xy.x;
        let y = local_xy.y;
        let xx = x * x;
        let yy = y * y;
        var t = 0.0;
        var is_valid = true;
        if (is_strip) {
            let a = radius - yy;
            t = sqrt(a) + x;
            is_valid = a >= 0.0;
        } else if (is_focal_on_circle) {
            t = (xx + yy) / x;
            is_valid = t >= 0.0 && x != 0.0;
        } else if (radius > 1.0) {
            t = sqrt(xx + yy) - x * r1_recip;
        } else {
            let a = xx - yy;
            t = less_scale * sqrt(a) - x * r1_recip;
            is_valid = a >= 0.0 && t >= 0.0;
        }
        if (!is_valid) {
            return vec2<f32>(0.0, 0.0);
        }
        t = extend_mode_normalized(focal_x + t_sign * t, extend);
        t = select(t, 1.0 - t, is_swapped);
        return vec2<f32>(t, 1.0);
    }
    // Sweep: xy_to_unit_angle from Skia (as vello has it), a 7th degree polynomial for atan.
    let x = local_xy.x;
    let y = local_xy.y;
    let xabs = abs(x);
    let yabs = abs(y);
    let slope = min(xabs, yabs) / max(xabs, yabs);
    let s = slope * slope;
    var phi = slope * (0.15912117063999176025390625f + s * (-5.185396969318389892578125e-2f + s * (2.476101927459239959716796875e-2f + s * (-7.0547382347285747528076171875e-3f))));
    phi = select(phi, 1.0 / 4.0 - phi, xabs < yabs);
    phi = select(phi, 1.0 / 2.0 - phi, x < 0.0);
    phi = select(phi, 1.0 - phi, y < 0.0);
    phi = select(phi, 0.0, phi != phi);
    phi = (phi - p.m1.z) / (p.m1.w - p.m1.z);
    return vec2<f32>(extend_mode_normalized(phi, extend), 1.0);
}

// ---- noise drawn on the GPU: noise_value maps a paint's parameters onto noise_once (noise.wgsl,
// the piece this module shares with the stroke shader).
fn noise_value(p: Paint, local_xy: vec2<f32>) -> f32 {
    let mode = p.info.y;
    let gradient = (mode & 1u) != 0u;
    let three = (mode & 2u) != 0u;
    let caustics = (mode & 4u) != 0u;
    let octaves = max((mode >> 8u) & 63u, 1u);
    let frequency = p.color.x;
    var q = vec3<f32>((local_xy + p.m1.zw) * frequency, select(0.0, p.color.y, three));
    var total = 0.0;
    var norm = 0.0;
    var amplitude = 1.0;
    var scale = 1.0;
    for (var o = 0u; o < octaves; o++) {
        // 2D noise samples one plane of the lattice (z = 0); 3D also scales z per octave.
        let qo = vec3<f32>(q.xy * scale, q.z * select(0.0, scale, three));
        // The caustics kind (mode bit 4, noise.wgsl caustics_once): z is its time-like coordinate.
        var v = 0.0;
        if (caustics) {
            v = caustics_once(p.info.w + o, qo.xy, qo.z);
        } else {
            v = noise_once(p.info.w + o, qo, gradient, three);
        }
        total += amplitude * v;
        norm += amplitude;
        amplitude *= p.color.w;
        scale *= p.color.z;
    }
    if (norm == 0.0) {
        return 0.0;
    }
    return clamp(total / norm, -1.0, 1.0);
}

// ---- particles: soft discs, one instance each ----
struct ParticleOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
    @location(2) @interpolate(flat) softness: f32,
};

@vertex
fn vs_particle(@builtin(vertex_index) index: u32, @location(0) center: vec2<f32>, @location(1) axes: vec4<f32>, @location(2) soft: vec2<f32>, @location(3) color: vec4<f32>) -> ParticleOut {
    // Two triangles over the disc's square, grown a little for the edge's anti-aliasing.
    var corners = array<vec2<f32>, 6>(vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));
    let c = corners[index] * 1.08;
    let p = center + axes.xy * c.x + axes.zw * c.y;
    var out: ParticleOut;
    out.position = to_clip(p);
    out.local = c;
    out.color = color;
    out.softness = soft.x;
    return out;
}

@fragment
fn fs_particle(in: ParticleOut) -> @location(0) vec4<f32> {
    // The colour held to 1 - softness of the radius, then fading linearly to transparent at the
    // edge (C#'s radial gradient from the colour to the colour at alpha 0); the edge itself
    // anti-aliased over about a pixel.
    let d = length(in.local);
    let w = max(fwidth(d), 1e-4);
    let hold = 1.0 - in.softness;
    var a = 1.0;
    if (in.softness > 0.0) {
        a = clamp(1.0 - (d - hold) / max(in.softness, 1e-4), 0.0, 1.0);
    }
    a = a * clamp((1.0 - d) / w + 0.5, 0.0, 1.0);
    return in.color * a;
}

@group(1) @binding(0) var image_tex: texture_2d<f32>;
@group(1) @binding(1) var image_linear: sampler;
@group(1) @binding(2) var image_nearest: sampler;

fn extend_coord(u: f32, mode: u32) -> f32 {
    switch mode {
        case EXTEND_PAD: {
            return clamp(u, 0.0, 1.0);
        }
        case EXTEND_REPEAT: {
            return fract(u);
        }
        default: {
            return abs(u - 2.0 * round(0.5 * u));
        }
    }
}

// ---- an ownership map coloured on the GPU: the picture holds the owner numbers (0 = nobody,
// otherwise index + 1, in R G B) and, in the rows below the map, the colours; seams where the
// owner differs from the left or upper neighbour's, softened over 3 x 3.
fn owner_at(c: vec2<i32>, size: vec2<i32>) -> u32 {
    let q = clamp(c, vec2<i32>(0), size - vec2<i32>(1));
    let t = textureLoad(image_tex, q, 0);
    return u32(round(t.r * 255.0)) + (u32(round(t.g * 255.0)) << 8u) + (u32(round(t.b * 255.0)) << 16u);
}

fn boundary_at(c: vec2<i32>, size: vec2<i32>) -> f32 {
    let o = owner_at(c, size);
    let left = select(o, owner_at(c - vec2<i32>(1, 0), size), c.x > 0);
    let up = select(o, owner_at(c - vec2<i32>(0, 1), size), c.y > 0);
    return select(0.0, 1.0, o != left || o != up);
}

fn unpack_color(v: u32) -> vec4<f32> {
    let c = vec4<f32>(f32(v & 255u), f32((v >> 8u) & 255u), f32((v >> 16u) & 255u), f32(v >> 24u)) / 255.0;
    return vec4<f32>(c.rgb * c.a, c.a);
}

fn owners_color(p: Paint, local_xy: vec2<f32>) -> vec4<f32> {
    let size = vec2<i32>(p.color.xy);
    let c = vec2<i32>(floor(local_xy));
    let o = owner_at(c, size);
    var col = unpack_color(p.info.y);
    if (o > 0u) {
        let i = i32(o - 1u);
        let t = textureLoad(image_tex, vec2<i32>(i % size.x, size.y + i / size.x), 0);
        col = vec4<f32>(t.rgb * t.a, t.a);
    }
    if ((p.info.w & 1u) != 0u) {
        // The boundary softened by a 3 x 3 Gaussian of sigma 0.8 (weights 0.2195, 0.5611, 0.2195).
        var b = 0.0;
        let w = array<f32, 3>(0.2195, 0.5611, 0.2195);
        for (var dy = -1; dy <= 1; dy++) {
            for (var dx = -1; dx <= 1; dx++) {
                b += w[dx + 1] * w[dy + 1] * boundary_at(c + vec2<i32>(dx, dy), size);
            }
        }
        let k = clamp(b * p.color.z, 0.0, 1.0);
        col = mix(col, unpack_color(p.info.z), k);
    }
    return premul_srgb_to_linear(col) * p.extra.x;
}

fn paint_color(i: u32, xy: vec2<f32>) -> vec4<f32> {
    let p = paints[i];
    let kind = p.info.x;
    if (kind == KIND_SOLID) {
        return p.color;
    }
    let local_xy = vec2<f32>(p.m0.x * xy.x + p.m0.z * xy.y + p.m1.x, p.m0.y * xy.x + p.m0.w * xy.y + p.m1.y);
    if (kind == KIND_OWNERS) {
        return owners_color(p, local_xy);
    }
    if (kind == KIND_IMAGE) {
        let size = p.color.xy;
        let uv = vec2<f32>(extend_coord(local_xy.x / size.x, p.info.y & 15u), extend_coord(local_xy.y / size.y, (p.info.y >> 4u) & 15u));
        // Both samplers are sampled (uniform control flow); the flag picks one.
        let nearest = textureSampleLevel(image_tex, image_nearest, uv, 0.0);
        let filtered = textureSampleLevel(image_tex, image_linear, uv, 0.0);
        let texel = select(nearest, filtered, (p.info.w & 1u) != 0u);
        // The texture holds sRGB-encoded bytes, straight or premultiplied.
        let c = select(vec4<f32>(texel.rgb * texel.a, texel.a), texel, (p.info.w & 2u) != 0u);
        // A tint multiplies the straight values (rgb by tint.rgb, alpha by tint.a): on the
        // premultiplied colour that is rgb * tint.rgb * tint.a and alpha * tint.a.
        let tint = select(vec4<f32>(1.0), vec4<f32>(p.color.z, p.color.w, p.extra.y, p.extra.z), (p.info.w & 4u) != 0u);
        let tinted = vec4<f32>(c.rgb * tint.rgb * tint.a, c.a * tint.a);
        return premul_srgb_to_linear(tinted) * p.extra.x;
    }
    if (kind == KIND_NOISE) {
        // Noise computed here per pixel, its value (-1..1) mapped through the ramp.
        let v = noise_value(p, local_xy);
        return premul_srgb_to_linear(ramp_at(p.info.z, clamp(v * 0.5 + 0.5, 0.0, 1.0))) * p.extra.x;
    }
    let t = gradient_t(p, local_xy);
    if (t.y == 0.0) {
        return vec4<f32>(0.0);
    }
    return premul_srgb_to_linear(ramp_at(p.info.z, t.x)) * p.extra.x;
}

// The paints are evaluated at the fragment's CANVAS position (the paint matrices are canvas
// pixels to paint space).
@fragment
fn fs_paint(in: VsOut) -> @location(0) vec4<f32> {
    return paint_color(in.paint, canvas_xy(in.position.xy));
}
