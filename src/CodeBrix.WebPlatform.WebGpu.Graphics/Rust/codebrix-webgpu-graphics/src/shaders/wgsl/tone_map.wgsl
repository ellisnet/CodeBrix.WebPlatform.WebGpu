struct Params {
    params: vec4<f32>,
    source: vec4<f32>,
};

@group(0) @binding(0) var<uniform> tone: Params;
@group(0) @binding(1) var hdr: texture_2d<f32>;
@group(0) @binding(2) var hdr_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
}

fn encode_srgb(value: vec3<f32>) -> vec3<f32> {
    let c = clamp(value, vec3<f32>(0.0), vec3<f32>(1.0));
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(high, low, c <= vec3<f32>(0.0031308));
}

fn aces_narkowicz(x: vec3<f32>) -> vec3<f32> {
    return clamp(x * (2.51 * x + vec3<f32>(0.03)) / (x * (2.43 * x + vec3<f32>(0.59)) + vec3<f32>(0.14)), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Stephen Hill's fit of the ACES RRT + ODT (sRGB primaries in, sRGB primaries out).
fn aces_fitted(color: vec3<f32>) -> vec3<f32> {
    let input = mat3x3<f32>(
        vec3<f32>(0.59719, 0.07600, 0.02840),
        vec3<f32>(0.35458, 0.90834, 0.13383),
        vec3<f32>(0.04823, 0.01566, 0.83777));
    let output = mat3x3<f32>(
        vec3<f32>(1.60475, -0.10208, -0.00327),
        vec3<f32>(-0.53108, 1.10813, -0.07276),
        vec3<f32>(-0.07367, -0.00605, 1.07602));
    let v = input * color;
    let a = v * (v + vec3<f32>(0.0245786)) - vec3<f32>(0.000090537);
    let b = v * (0.983729 * v + vec3<f32>(0.4329510)) + vec3<f32>(0.238081);
    return clamp(output * (a / b), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Khronos PBR Neutral (Khronos 3D Commerce reference).
fn pbr_neutral(input: vec3<f32>) -> vec3<f32> {
    let start_compression = 0.8 - 0.04;
    let desaturation = 0.15;
    var color = input;
    let x = min(color.r, min(color.g, color.b));
    let offset = select(0.04, x - 6.25 * x * x, x < 0.08);
    color -= vec3<f32>(offset);
    let peak = max(color.r, max(color.g, color.b));
    if (peak < start_compression) {
        return color;
    }
    let d = 1.0 - start_compression;
    let new_peak = 1.0 - d * d / (peak + d - start_compression);
    color *= new_peak / peak;
    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(color, vec3<f32>(new_peak), g);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = position.xy * tone.source.zw;
    let sampled = textureSampleLevel(hdr, hdr_sampler, uv, 0.0);
    // PRESERVE ALPHA (params.w): the source holds premultiplied colour; the curve is applied to
    // the straight colour and the result premultiplied again, so edges and see-through parts
    // keep their colour, and the pipeline blends it over what lies below.
    let preserve = tone.params.w > 0.5;
    if (preserve && sampled.a <= 0.0) {
        return vec4<f32>(0.0);
    }
    let straight = select(sampled.rgb, sampled.rgb / max(sampled.a, 1e-6), preserve);
    let exposed = max(straight * tone.params.x, vec3<f32>(0.0));
    var mapped: vec3<f32>;
    switch u32(tone.params.y) {
        case 0u: { mapped = aces_fitted(exposed); }
        case 1u: { mapped = pbr_neutral(exposed); }
        case 2u: { mapped = exposed / (vec3<f32>(1.0) + exposed); }
        case 3u: { mapped = aces_narkowicz(exposed); }
        default: { mapped = min(exposed, vec3<f32>(1.0)); }
    }
    mapped = clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0));
    var display = mapped;
    if (tone.params.z > 0.5) {
        display = encode_srgb(mapped);
    }
    if (preserve) {
        return vec4<f32>(display * sampled.a, sampled.a);
    }
    return vec4<f32>(display, sampled.a);
}
