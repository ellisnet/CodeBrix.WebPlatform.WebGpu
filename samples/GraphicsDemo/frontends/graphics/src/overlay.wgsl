// The graphics application's overlay: one translucent square, made in the vertex shader (no
// vertex buffer), that glides left and right with the canvas clock's time. Its colour is LINEAR
// and PREMULTIPLIED by its alpha, as the canvas host's blending convention asks; the canvas's
// sRGB view encodes it and the pipeline blends it over the layers below.

struct Overlay {
    // The square's centre, in clip space (x, y).
    center: vec2<f32>,
    // Half the square's side, in clip space (x, y: kept square on any canvas).
    half_size: vec2<f32>,
    // Its colour, linear and premultiplied by its alpha.
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> overlay: Overlay;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    return vec4<f32>(overlay.center + corners[index] * overlay.half_size, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return overlay.color;
}
