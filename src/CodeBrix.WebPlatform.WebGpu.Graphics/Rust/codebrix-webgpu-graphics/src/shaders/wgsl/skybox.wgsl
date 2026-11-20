struct Sky {
    inverse_view_projection: mat4x4<f32>,
    params: vec4<f32>,
    lods: vec4<f32>,
};

@group(0) @binding(0) var<uniform> sky: Sky;
@group(0) @binding(1) var radiance: texture_cube<f32>;
@group(0) @binding(2) var prefiltered: texture_cube<f32>;
@group(0) @binding(3) var sky_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    var output: VertexOutput;
    output.clip = vec4<f32>(ndc, 1.0, 1.0);
    output.ndc = ndc;
    return output;
}

fn encode_srgb(value: vec3<f32>) -> vec3<f32> {
    let c = clamp(value, vec3<f32>(0.0), vec3<f32>(1.0));
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(high, low, c <= vec3<f32>(0.0031308));
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let near = sky.inverse_view_projection * vec4<f32>(input.ndc, 0.0, 1.0);
    let far = sky.inverse_view_projection * vec4<f32>(input.ndc, 1.0, 1.0);
    let world = normalize(far.xyz / far.w - near.xyz / near.w);
    // Rotate the world direction into the environment (the environment turns by params.y about +Y).
    let c = cos(sky.params.y);
    let s = sin(sky.params.y);
    let d = vec3<f32>(c * world.x - s * world.z, world.y, s * world.x + c * world.z);
    let sharp = textureSampleLevel(radiance, sky_sampler, d, sky.lods.x).rgb;
    let blurred = textureSampleLevel(prefiltered, sky_sampler, d, sky.lods.y).rgb;
    let color = select(sharp, blurred, sky.lods.z > 0.5) * sky.params.x * sky.params.z;
    if (sky.params.w > 0.5) {
        return vec4<f32>(encode_srgb(color), 1.0);
    }
    return vec4<f32>(color, 1.0);
}
