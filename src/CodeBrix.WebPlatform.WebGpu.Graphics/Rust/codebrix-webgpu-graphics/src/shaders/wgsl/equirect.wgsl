@group(0) @binding(0) var<uniform> bake: Bake;
@group(0) @binding(1) var panorama: texture_2d<f32>;
@group(0) @binding(2) var panorama_sampler: sampler;

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let d = face_direction(bake.face, position.xy / f32(bake.size));
    let uv = vec2<f32>(0.5 + atan2(d.x, -d.z) / (2.0 * PI), 0.5 - asin(clamp(d.y, -1.0, 1.0)) / PI);
    return vec4<f32>(textureSampleLevel(panorama, panorama_sampler, uv, bake.a.x).rgb, 1.0);
}
