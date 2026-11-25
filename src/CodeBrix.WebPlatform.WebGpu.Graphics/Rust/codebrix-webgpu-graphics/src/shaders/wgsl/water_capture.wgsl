// The water's scene capture (Not in C#): one full-screen pass that writes the raw depth of the
// target drawn so far into an R32Float texture the water surface reads with textureLoad.
// fs_single reads a 1-sample depth (texture_depth_2d), fs_multi sample 0 of a 4-sample depth
// (texture_depth_multisampled_2d); each pipeline binds only its own texture.

@group(0) @binding(0) var depth_single: texture_depth_2d;
@group(0) @binding(1) var depth_multi: texture_depth_multisampled_2d;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
}

@fragment
fn fs_single(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let depth = textureLoad(depth_single, vec2<i32>(position.xy), 0);
    return vec4<f32>(depth, 0.0, 0.0, 1.0);
}

@fragment
fn fs_multi(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let depth = textureLoad(depth_multi, vec2<i32>(position.xy), 0);
    return vec4<f32>(depth, 0.0, 0.0, 1.0);
}
