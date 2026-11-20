// The graphics application's triangle: three corners made in the vertex shader (no vertex
// buffer), turned, scaled and kept in shape on any canvas by one small uniform block, with
// colours that pulse slowly over time (the canvas clock's time) and turn lighter once the
// "title in" point of the clock has been reached. Colours are linear: the canvas's sRGB view
// encodes them.

struct Scene {
    // Rotation in radians.
    angle: f32,
    // Size factor (1 = the triangle's natural size).
    scale: f32,
    // The surface's width divided by its height.
    aspect: f32,
    // The canvas clock's time, in seconds.
    time: f32,
    // 0 before the clock's "title in" point, 1 from it on.
    lit: f32,
    // Padding to the uniform block's 16-byte size steps.
    unused_1: f32,
    unused_2: f32,
    unused_3: f32,
};

@group(0) @binding(0) var<uniform> scene: Scene;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.8),
        vec2<f32>(-0.7, -0.45),
        vec2<f32>(0.7, -0.45),
    );
    var colors = array<vec3<f32>, 3>(
        vec3<f32>(1.0, 0.25, 0.3),
        vec3<f32>(0.2, 0.85, 0.4),
        vec3<f32>(0.25, 0.45, 1.0),
    );
    let c = cos(scene.angle);
    let s = sin(scene.angle);
    let p = corners[index] * scene.scale;
    let turned = vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
    // Keep the triangle's shape: shrink the longer side of the surface to the shorter one.
    let fitted = vec2<f32>(turned.x * min(1.0, 1.0 / scene.aspect), turned.y * min(1.0, scene.aspect));
    var out: VertexOut;
    out.position = vec4<f32>(fitted, 0.0, 1.0);
    let pulse = 0.7 + 0.3 * sin(scene.time * 2.0 + f32(index) * 2.094);
    out.color = mix(colors[index] * pulse, vec3<f32>(1.0, 1.0, 1.0), 0.35 * scene.lit);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
