// The "More" scene's compute pass: every frame it moves the particles along a turning double
// helix (a function of the frame's time) and writes the INDIRECT DRAW ARGUMENTS of the particle
// draw - six vertices per particle, and an instance count that rises and falls with time - so
// the GPU decides how many particles the render pass draws.

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
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(2) var<storage, read_write> draw_args: array<u32, 4>;

fn hue(t: f32) -> vec3<f32> {
    return clamp(abs(fract(vec3<f32>(t, t + 0.6667, t + 0.3333)) * 6.0 - 3.0) - 1.0, vec3<f32>(0.0), vec3<f32>(1.0));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let count = arrayLength(&particles);
    let time = globals.time.x;
    if (id.x == 0u) {
        let visible = u32(f32(count) * (0.55 + 0.45 * sin(time * 0.7)));
        draw_args[0] = 6u;
        draw_args[1] = max(visible, 1u);
        draw_args[2] = 0u;
        draw_args[3] = 0u;
    }
    if (id.x >= count) {
        return;
    }
    let f = f32(id.x) / f32(count);
    let strand = f32(id.x % 2u);
    let angle = f * 31.4159 + strand * 3.14159 + time * 1.1;
    let radius = 1.0 + 1.2 * f;
    let height = 0.4 + 3.2 * f + 0.12 * sin(time * 3.0 + f * 40.0);
    particles[id.x].position = vec4<f32>(cos(angle) * radius, height, sin(angle) * radius, 0.07 + 0.03 * sin(time * 4.0 + f * 25.0));
    particles[id.x].color = vec4<f32>(pow(hue(f + time * 0.05), vec3<f32>(2.2)), 1.0);
}
