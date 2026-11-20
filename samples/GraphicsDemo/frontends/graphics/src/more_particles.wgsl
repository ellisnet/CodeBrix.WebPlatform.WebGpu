// The "More" scene's particles, drawn with ONE INDIRECT draw whose arguments the compute pass
// wrote: each instance is a camera-facing disc at the position the compute pass gave it.

struct Globals {
    view_proj: mat4x4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    time: vec4<f32>,
};

struct Particle {
    position: vec4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) corner: vec2<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> VertexOutput {
    var corners = array<vec2<f32>, 6>(vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));
    let particle = particles[instance];
    let corner = corners[vertex];
    let world = particle.position.xyz + (globals.camera_right.xyz * corner.x + globals.camera_up.xyz * corner.y) * particle.position.w;
    var output: VertexOutput;
    output.clip = globals.view_proj * vec4<f32>(world, 1.0);
    output.corner = corner;
    output.color = particle.color.rgb;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let distance = length(input.corner);
    if (distance > 1.0) {
        discard;
    }
    return vec4<f32>(input.color * (1.15 - 0.6 * distance * distance), 1.0);
}
