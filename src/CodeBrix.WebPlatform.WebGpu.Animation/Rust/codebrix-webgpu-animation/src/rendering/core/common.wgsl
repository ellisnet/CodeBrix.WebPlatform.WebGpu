// The shaders of the codebrix-webgpu-animation renderer: stencil-then-cover with 4x MSAA. This
// part is shared by both shader modules (paint.wgsl and composite.wgsl are appended to it).
// Every colour that reaches a target is LINEAR and PREMULTIPLIED (the canvas host's convention);
// the targets are Rgba8UnormSrgb (MSAA) and the host's sRGB view, so fixed-function blending
// happens in linear space. Gradient ramps hold premultiplied sRGB-ENCODED samples (as vello's),
// converted to linear per pixel. The mix (blend) modes work in sRGB-encoded values, as the W3C
// compositing formulas and NoSkia do.
//
// The gradient shading in paint.wgsl (linear, two-point radial, sweep, and the extend modes) is
// TRANSLATED FROM vello 0.12.0's vello_shaders/shader/fine.wgsl, and the blend functions in
// composite.wgsl (blend_mix, blend_compose, blend_mix_compose and their helpers) are COPIED FROM
// vello_shaders/shader/shared/blend.wgsl - Copyright 2022 the Vello Authors, licensed
// Apache-2.0 OR MIT OR Unlicense; used here under Apache-2.0 (see the notices).

struct View {
    size: vec4<f32>,
};

struct Paint {
    info: vec4<u32>,   // kind, extend (x | y << 4), ramp slot, flags
    color: vec4<f32>,  // solid: linear premultiplied; image: width, height
    m0: vec4<f32>,     // device -> paint space: a b c d (x' = a x + c y + e; y' = b x + d y + f)
    m1: vec4<f32>,     // e f, radial: focal_x radius; sweep: t0 t1
    extra: vec4<f32>,  // alpha
};

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage, read> paints: array<Paint>;
@group(0) @binding(2) var<storage, read> ramps: array<vec4<f32>>;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) paint: u32,
};

@vertex
fn vs_main(@location(0) p: vec2<f32>, @location(1) paint: u32) -> VsOut {
    var out: VsOut;
    out.position = vec4<f32>(p.x / view.size.x * 2.0 - 1.0, 1.0 - p.y / view.size.y * 2.0, 0.0, 1.0);
    out.paint = paint;
    return out;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let low = c * 12.92;
    let high = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, c <= vec3<f32>(0.0031308));
}

// A premultiplied sRGB-encoded colour as linear premultiplied.
fn premul_srgb_to_linear(c: vec4<f32>) -> vec4<f32> {
    if (c.a <= 0.0) {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(srgb_to_linear(c.rgb / c.a) * c.a, c.a);
}

fn premul_linear_to_srgb(c: vec4<f32>) -> vec4<f32> {
    if (c.a <= 0.0) {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(linear_to_srgb(c.rgb / c.a) * c.a, c.a);
}
