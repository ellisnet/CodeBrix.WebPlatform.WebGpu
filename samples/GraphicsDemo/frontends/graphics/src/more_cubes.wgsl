// The "More" scene's ring of cubes, recorded ONCE into a render bundle: twelve instances of one
// cube, placed on a ring by their instance index, lit by one fixed light. The bundle reads the
// camera from the uniform buffer every time it is replayed, so the cubes stay put while the
// camera moves.

struct Globals {
    view_proj: mat4x4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    time: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
};

const CUBES: f32 = 12.0;
const RING_RADIUS: f32 = 4.2;

fn hue(t: f32) -> vec3<f32> {
    return clamp(abs(fract(vec3<f32>(t, t + 0.6667, t + 0.3333)) * 6.0 - 3.0) - 1.0, vec3<f32>(0.0), vec3<f32>(1.0));
}

@vertex
fn vs_main(input: VertexInput, @builtin(instance_index) instance: u32) -> VertexOutput {
    let angle = f32(instance) / CUBES * 6.2831853;
    let center = vec3<f32>(cos(angle) * RING_RADIUS, 0.45, sin(angle) * RING_RADIUS);
    let world = input.position * 0.7 + center;
    let light = 0.3 + 0.7 * max(dot(input.normal, normalize(vec3<f32>(0.4, 1.0, 0.3))), 0.0);
    var output: VertexOutput;
    output.clip = globals.view_proj * vec4<f32>(world, 1.0);
    output.color = pow(hue(f32(instance) / CUBES) * 0.85 + 0.1, vec3<f32>(2.2)) * light;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
