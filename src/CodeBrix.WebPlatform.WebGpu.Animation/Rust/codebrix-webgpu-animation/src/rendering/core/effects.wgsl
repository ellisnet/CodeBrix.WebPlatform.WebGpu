// The effect passes (blur, glow, drop shadow - C#'s EffectProcessor on the GPU), appended to
// common.wgsl. A group with effects is drawn into an offscreen multisampled target; then:
//   fs_resolve    averages its samples into a float texture (transparent outside the content),
//   fs_down       box-filters a texture into a coarser level (wide blurs run at 1/2, 1/4 ...),
//   fs_blur       one direction of a separable Gaussian (the planes' kernel, from a buffer),
//   fs_composite  puts down shadow, glow and content (C#'s order: content over glow over
//                 shadow) into the target below, at the group's alpha - with a layer's
//                 whole-frame operations (tone curve, gamma, vignette on the straight sRGB
//                 colour; a bloom added) and its weight in a transition (dissolve, wipe, fade
//                 through a colour) applied to the content first.
// Everything is LINEAR and PREMULTIPLIED (the renderer's convention); the passes read only the
// texels they know to be valid (zero elsewhere), as C#'s planes see transparency outside the
// region it processes. The parts below the "//@part" lines are separate shader modules (each
// with this shared head), so that their group-1 bindings never meet.

struct FxPass {
    v0: vec4<f32>,
    v1: vec4<f32>,
    v2: vec4<f32>,
    v3: vec4<f32>,
    v4: vec4<f32>,
    u0: vec4<u32>,
    v5: vec4<f32>,
    v6: vec4<f32>,
    v7: vec4<f32>,
    v8: vec4<f32>,
    v9: vec4<f32>,
    u1: vec4<u32>,
};

@group(2) @binding(0) var<storage, read> fx: array<FxPass>;
@group(2) @binding(1) var<storage, read> fx_kernel: array<f32>;

// Down and blur passes draw into a texture of a coarser level: device pixels / level, mapped
// onto that texture (v4.xy is its size in texels).
@vertex
fn vs_fx(@location(0) p: vec2<f32>, @location(1) pass_index: u32) -> VsOut {
    let fp = fx[pass_index];
    let q = p / fp.v1.y;
    var out: VsOut;
    out.position = vec4<f32>(q.x / fp.v4.x * 2.0 - 1.0, 1.0 - q.y / fp.v4.y * 2.0, 0.0, 1.0);
    out.paint = pass_index;
    return out;
}

fn inside(c: vec2<f32>, r: vec4<f32>) -> bool {
    return c.x >= r.x && c.y >= r.y && c.x < r.z && c.y < r.w;
}

//@part resolve
@group(1) @binding(0) var fx_ms: texture_multisampled_2d<f32>;

@fragment
fn fs_resolve(in: VsOut) -> @location(0) vec4<f32> {
    let fp = fx[in.paint];
    let xy = in.position.xy;
    if (!inside(xy, fp.v0)) {
        return vec4<f32>(0.0);
    }
    let c = vec2<i32>(xy);
    return (textureLoad(fx_ms, c, 0) + textureLoad(fx_ms, c, 1) + textureLoad(fx_ms, c, 2) + textureLoad(fx_ms, c, 3)) * 0.25;
}

//@part filter
@group(1) @binding(0) var fx_src: texture_2d<f32>;

fn load_valid(c: vec2<i32>, valid: vec4<f32>) -> vec4<f32> {
    if (!inside(vec2<f32>(c) + vec2<f32>(0.5), valid)) {
        return vec4<f32>(0.0);
    }
    return textureLoad(fx_src, c, 0);
}

@fragment
fn fs_down(in: VsOut) -> @location(0) vec4<f32> {
    let fp = fx[in.paint];
    let ratio = fp.v1.y / fp.v1.x;
    let texel = vec2<i32>(in.position.xy);
    if (ratio <= 1.0) {
        // A finer target (rare): the source texel under this one.
        return load_valid(vec2<i32>(in.position.xy * ratio), fp.v0);
    }
    // The exact box of source texels under this texel (zero outside the valid ones).
    let n = i32(ratio + 0.5);
    let origin = texel * n;
    var sum = vec4<f32>(0.0);
    for (var y = 0; y < n; y++) {
        for (var x = 0; x < n; x++) {
            sum += load_valid(origin + vec2<i32>(x, y), fp.v0);
        }
    }
    return sum / f32(n * n);
}

@fragment
fn fs_blur(in: VsOut) -> @location(0) vec4<f32> {
    let fp = fx[in.paint];
    let texel = vec2<i32>(in.position.xy);
    let dir = vec2<i32>(fp.v1.zw);
    let offset = fp.u0.x;
    let radius = i32(fp.u0.y);
    // C#'s order of summation: from -radius to +radius.
    var sum = vec4<f32>(0.0);
    for (var i = -radius; i <= radius; i++) {
        sum += fx_kernel[offset + u32(i + radius)] * load_valid(texel + dir * i, fp.v0);
    }
    return sum;
}

//@part composite
@group(1) @binding(0) var fx_content: texture_2d<f32>;
@group(1) @binding(1) var fx_glow: texture_2d<f32>;
@group(1) @binding(2) var fx_shadow: texture_2d<f32>;
@group(1) @binding(3) var fx_linear: sampler;

// A texture of a level sampled at a device position (bilinear between its texels).
fn sample_level(t: texture_2d<f32>, device: vec2<f32>, level: f32) -> vec4<f32> {
    let size = vec2<f32>(textureDimensions(t));
    return textureSampleLevel(t, fx_linear, device / level / size, 0.0);
}

// The planes' colour operations on a linear premultiplied colour: unpremultiplied and encoded
// to sRGB (the values a plane image holds), the tone curve, the gamma and the vignette (at the
// pixel's centre), then back. Alpha is left alone.
fn frame_colour(c: vec4<f32>, p: vec2<f32>, fp: FxPass) -> vec4<f32> {
    let flags = fp.u1.x;
    if ((flags & 7u) == 0u || c.a <= 0.0) {
        return c;
    }
    var s = linear_to_srgb(c.rgb / c.a);
    if ((flags & 1u) != 0u) {
        s = s / (vec3<f32>(1.0) + fp.v5.x * s) * fp.v5.y;
    }
    if ((flags & 2u) != 0u) {
        s = select(s, pow(max(s, vec3<f32>(1e-30)), vec3<f32>(fp.v5.z)), s > vec3<f32>(0.0));
    }
    if ((flags & 4u) != 0u) {
        let d = (p - fp.v6.xy) / fp.v6.zw;
        s = s * clamp(1.0 - fp.v7.x * dot(d, d), fp.v7.y, 1.0);
    }
    return vec4<f32>(srgb_to_linear(max(s, vec3<f32>(0.0))) * c.a, c.a);
}

// The incoming layer of a transition, drawn over the outgoing one: its weight at this pixel
// (dissolve: the fraction; wipe: the planes' hard or soft edge from the side it starts at) - or,
// for a fade through a colour, the colour in front of the outgoing frame below 0.5 and mixed into
// the incoming one above it. Exact for opaque frames, as the planes' transitions are of frames.
fn transition_colour(c: vec4<f32>, p: vec2<f32>, fp: FxPass) -> vec4<f32> {
    let kind = (fp.u1.x >> 8u) & 15u;
    if (kind == 0u) {
        return c;
    }
    let f = clamp(fp.v7.z, 0.0, 1.0);
    let direction = (fp.u1.x >> 12u) & 3u;
    if (kind == 1u) {
        return c * f;
    }
    if (kind == 2u) {
        let horizontal = direction == 0u || direction == 1u;
        let extent = select(fp.v9.w - fp.v9.y, fp.v9.z - fp.v9.x, horizontal);
        // 0 Left, 1 Right, 2 Up, 3 Down: Right and Down start at the left and the top.
        var centre = 0.0;
        if (direction == 1u) { centre = p.x - fp.v9.x; }
        if (direction == 0u) { centre = fp.v9.z - p.x; }
        if (direction == 3u) { centre = p.y - fp.v9.y; }
        if (direction == 2u) { centre = fp.v9.w - p.y; }
        let s = fp.v7.w;
        var w = select(0.0, 1.0, centre < f * extent);
        if (s > 0.0) {
            w = clamp((f * (extent + s) - centre) / s, 0.0, 1.0);
        }
        return c * w;
    }
    if (kind == 5u) {
        if (f < 0.5) {
            return fp.v8 * (2.0 * f);
        }
        let w = 2.0 * f - 1.0;
        return c * w + fp.v8 * (1.0 - w);
    }
    return c;
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let fp = fx[in.paint];
    let p = in.position.xy;
    let flags = u32(fp.v1.w);
    var content = frame_colour(sample_level(fx_content, p, fp.v1.x), p, fp);
    if ((fp.u1.x & 8u) != 0u) {
        // A bloom: the blurred content (in the glow's texture), times the gain, added.
        let light = sample_level(fx_glow, p, fp.v1.y) * fp.v5.w;
        content = vec4<f32>(content.rgb + light.rgb, clamp(content.a + light.a * (1.0 - content.a), 0.0, 1.0));
    }
    let q = p - fp.v4.xy;
    let shadow_cov = sample_level(fx_shadow, q, fp.v1.z).a;
    let glow_cov = sample_level(fx_glow, p, fp.v1.y).a;
    var result = vec4<f32>(0.0);
    if ((flags & 2u) != 0u && inside(q, fp.v0)) {
        result = fp.v3 * shadow_cov;
    }
    if ((flags & 1u) != 0u) {
        let glow = fp.v2 * min(glow_cov * fp.v4.z, 1.0);
        result = glow + result * (1.0 - glow.a);
    }
    result = content + result * (1.0 - content.a);
    return transition_colour(result, p, fp) * fp.v4.w;
}
