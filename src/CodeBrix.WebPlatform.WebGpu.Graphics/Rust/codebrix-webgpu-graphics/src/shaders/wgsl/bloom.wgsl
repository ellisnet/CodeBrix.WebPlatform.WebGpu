// Bloom on the HDR path (Not in C#): a bright pass with a soft knee into the first (half-size)
// level of a chain, a 13-tap downsample into each smaller level, a 3 x 3 tent upsample added
// back up the chain, and the largest level added to the scene. One full-screen triangle per step.

struct Params {
    // x: threshold; y: soft knee (0..1, a fraction of the threshold); z: scale of what is
    // written (the composite's intensity / levels; 1 otherwise); w: unused.
    params: vec4<f32>,
    // xy: one texel of the source; zw: 1 / the destination's size in pixels.
    texel: vec4<f32>,
};

@group(0) @binding(0) var<uniform> bloom: Params;
@group(0) @binding(1) var source: texture_2d<f32>;
@group(0) @binding(2) var source_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
}

// One tap; values are kept finite (half-float infinities and NaNs would spread over the screen).
fn tap(uv: vec2<f32>) -> vec3<f32> {
    let c = textureSampleLevel(source, source_sampler, uv, 0.0).rgb;
    return clamp(select(c, vec3<f32>(0.0), c != c), vec3<f32>(0.0), vec3<f32>(60000.0));
}

// The 13-tap downsample (a 4 x 4 texel box with overlapping 2 x 2 boxes weighted towards the
// centre): smooth, and stable under motion.
fn downsample(uv: vec2<f32>) -> vec3<f32> {
    let t = bloom.texel.xy;
    let a = tap(uv + t * vec2<f32>(-2.0, -2.0));
    let b = tap(uv + t * vec2<f32>(0.0, -2.0));
    let c = tap(uv + t * vec2<f32>(2.0, -2.0));
    let d = tap(uv + t * vec2<f32>(-1.0, -1.0));
    let e = tap(uv + t * vec2<f32>(1.0, -1.0));
    let f = tap(uv + t * vec2<f32>(-2.0, 0.0));
    let g = tap(uv);
    let h = tap(uv + t * vec2<f32>(2.0, 0.0));
    let i = tap(uv + t * vec2<f32>(-1.0, 1.0));
    let j = tap(uv + t * vec2<f32>(1.0, 1.0));
    let k = tap(uv + t * vec2<f32>(-2.0, 2.0));
    let l = tap(uv + t * vec2<f32>(0.0, 2.0));
    let m = tap(uv + t * vec2<f32>(2.0, 2.0));
    return (d + e + i + j) * 0.125 + (a + c + k + m) * 0.03125 + (b + f + h + l) * 0.0625 + g * 0.125;
}

// The 3 x 3 tent upsample (weights 1 2 1 / 2 4 2 / 1 2 1, over 16), one source texel apart.
fn upsample(uv: vec2<f32>) -> vec3<f32> {
    let t = bloom.texel.xy;
    var sum = tap(uv) * 4.0;
    sum += (tap(uv + vec2<f32>(-t.x, 0.0)) + tap(uv + vec2<f32>(t.x, 0.0)) + tap(uv + vec2<f32>(0.0, -t.y)) + tap(uv + vec2<f32>(0.0, t.y))) * 2.0;
    sum += tap(uv + vec2<f32>(-t.x, -t.y)) + tap(uv + vec2<f32>(t.x, -t.y)) + tap(uv + vec2<f32>(-t.x, t.y)) + tap(uv + vec2<f32>(t.x, t.y));
    return sum * (1.0 / 16.0);
}

// The bright pass: what lies above the threshold, eased in over the soft knee (a quadratic from
// threshold - knee to threshold + knee, then linear), so there is no hard edge where bloom starts.
fn bright(color: vec3<f32>) -> vec3<f32> {
    let threshold = bloom.params.x;
    let knee = threshold * clamp(bloom.params.y, 0.0, 1.0);
    let brightness = max(color.r, max(color.g, color.b));
    var soft = clamp(brightness - threshold + knee, 0.0, 2.0 * knee);
    soft = soft * soft / (4.0 * knee + 1e-5);
    let contribution = max(soft, brightness - threshold) / max(brightness, 1e-5);
    return color * contribution;
}

fn destination_uv(position: vec4<f32>) -> vec2<f32> {
    return position.xy * bloom.texel.zw;
}

@fragment
fn fs_bright(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(bright(downsample(destination_uv(position))), 1.0);
}

@fragment
fn fs_down(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(downsample(destination_uv(position)), 1.0);
}

// Added (blend one + one) to the level above; also the composite into the scene, scaled. The
// alpha written is 0 and the pipelines keep the destination's alpha.
@fragment
fn fs_up(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(upsample(destination_uv(position)) * bloom.params.z, 0.0);
}
