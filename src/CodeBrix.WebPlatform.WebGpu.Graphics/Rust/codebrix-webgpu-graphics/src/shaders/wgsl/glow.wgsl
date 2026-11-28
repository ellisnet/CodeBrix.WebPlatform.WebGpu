// The glow layer's and the particle system's drawing (Not in C#): one quad per instance, all
// instances of a draw in one instanced draw. Instance record (48 bytes, three vec4): position +
// size (scene units, the quad's width), linear colour + opacity, and intensity, pulse rate (cycles
// per second), pulse depth (0..1), pulse phase (cycles) - for particles (params.w = 1) the pulse's
// yzw are the particle's velocity instead (scene units per second) and there is no pulse.
// Three vertex entry points, one per pipeline: vs_main (camera-facing, as it has always been),
// vs_lit (camera-facing, lit by the key light and the ambient) and vs_oriented (a quad fixed in the
// scene by a right and an up axis, from a second instance stream of 24 bytes: right.xyz, up.xyz).

struct Globals {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    // x: clock time in seconds; y: 1 = encode to sRGB in the shader; z: 1 = shape from the
    // texture instead of the round falloff; w: 1 = the instances are particles (no pulse; the
    // pulse's yzw are the velocity).
    params: vec4<f32>,
    // x, y: the near fade's start and end, in glow sizes of the camera's distance from the
    // centre (y = 0: no near fade); z: the stretch's seconds of motion; w: the stretch's longest
    // length in sizes (0: no stretch).
    effects: vec4<f32>,
    // vs_lit only, all in VIEW space: the ambient's sky and ground colours, the up direction of
    // the ambient's hemisphere, the direction the key light travels (unit), the key light's colour
    // times its intensity (w: the lighting's exposure, which scales the lit result as the model
    // renderer's does).
    ambient_sky: vec4<f32>,
    ambient_ground: vec4<f32>,
    ambient_up: vec4<f32>,
    light_direction: vec4<f32>,
    light_color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var shape_texture: texture_2d<f32>;
@group(1) @binding(1) var shape_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    // The corner of the quad, -1..1 on both axes.
    @location(0) corner: vec2<f32>,
    // Linear colour times intensity and pulse; alpha = opacity.
    @location(1) color: vec4<f32>,
};

fn quad_corner(index: u32) -> vec2<f32> {
    // Two triangles, counter-clockwise: (-1,-1) (1,-1) (1,1) / (-1,-1) (1,1) (-1,1).
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));
    return corners[index % 6u];
}

// 1 at the pulse's peak, 1 - depth at its trough; a raised cosine over each cycle.
fn pulse_factor(rate: f32, depth: f32, phase: f32, time: f32) -> f32 {
    let wave = 0.5 - 0.5 * cos(6.28318530718 * (rate * time + phase));
    return 1.0 - clamp(depth, 0.0, 1.0) * wave;
}

// The round soft shape: 1 at the centre, falling smoothly to exactly 0 at the quad's inscribed
// circle (r = 1) with a flat edge, (1 - r^2)^3.
fn glow_falloff(corner: vec2<f32>) -> f32 {
    let f = max(1.0 - dot(corner, corner), 0.0);
    return f * f * f;
}

fn encode_srgb(value: vec3<f32>) -> vec3<f32> {
    let c = clamp(value, vec3<f32>(0.0), vec3<f32>(1.0));
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(high, low, c <= vec3<f32>(0.0031308));
}

// The near fade: 0 up to `start` glow sizes from the centre, 1 from `end` on, a smooth step
// between (the CPU makes end > start).
fn near_fade(sizes: f32, start: f32, end: f32) -> f32 {
    let t = clamp((sizes - start) / (end - start), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

// Intensity times the pulse (none for particles), times the near fade when it is on.
fn strength_of(pulse: vec4<f32>, view_position: vec3<f32>, size: f32) -> f32 {
    let pulsing = select(pulse_factor(pulse.y, pulse.z, pulse.w, globals.params.x), 1.0, globals.params.w > 0.5);
    var strength = max(pulse.x, 0.0) * pulsing;
    if (globals.effects.y > 0.0) {
        strength = strength * near_fade(length(view_position) / max(size, 1e-6), globals.effects.x, globals.effects.y);
    }
    return strength;
}

fn is_perspective() -> bool {
    return globals.projection[3][3] < 0.5;
}

// The camera-facing corner's offset in view space: a square of the size, or - with the stretch on
// - longer along the particle's motion as seen on the screen (`velocity` in scene units).
fn facing_offset(corner: vec2<f32>, view_position: vec3<f32>, size: f32, velocity: vec3<f32>) -> vec2<f32> {
    let half = size * 0.5;
    if (globals.effects.w <= 0.0) {
        return corner * half;
    }
    let v = (globals.view * vec4<f32>(velocity, 0.0)).xyz;
    let depth = max(-view_position.z, 1e-4);
    let seen = select(v.xy, v.xy + view_position.xy * (v.z / depth), is_perspective());
    let speed = length(seen);
    let along = select(vec2<f32>(1.0, 0.0), seen / max(speed, 1e-12), speed > 1e-6);
    let across = vec2<f32>(-along.y, along.x);
    let length_along = clamp(size + globals.effects.z * speed, size, globals.effects.w * size);
    return along * (corner.x * length_along * 0.5) + across * (corner.y * half);
}

// The light on a camera-facing particle: the ambient's hemisphere and the key light's half-Lambert
// term ((n.l / 2 + 1/2)^2), on the normal that faces the camera from the particle, times the
// exposure.
fn lighting_at(view_position: vec3<f32>) -> vec3<f32> {
    let to_camera = length(view_position);
    let normal = select(vec3<f32>(0.0, 0.0, 1.0), -view_position / max(to_camera, 1e-6), is_perspective() && to_camera > 1e-6);
    let t = clamp(dot(normal, globals.ambient_up.xyz) * 0.5 + 0.5, 0.0, 1.0);
    let ambient = mix(globals.ambient_ground.rgb, globals.ambient_sky.rgb, t);
    let wrap = dot(normal, -globals.light_direction.xyz) * 0.5 + 0.5;
    return (ambient + globals.light_color.rgb * (wrap * wrap)) * globals.light_color.w;
}

fn facing_vertex(index: u32, position_size: vec4<f32>, color: vec4<f32>, pulse: vec4<f32>, lit: bool) -> VertexOutput {
    let corner = quad_corner(index);
    // Camera-facing: the corner is offset in view space, so the quad always faces the camera
    // (also under an orthographic projection). A size of 0 collapses the quad: nothing is drawn.
    let view_position = globals.view * vec4<f32>(position_size.xyz, 1.0);
    let offset = vec4<f32>(facing_offset(corner, view_position.xyz, position_size.w, pulse.yzw), 0.0, 0.0);
    var output: VertexOutput;
    output.clip = globals.projection * (view_position + offset);
    output.corner = corner;
    let strength = strength_of(pulse, view_position.xyz, position_size.w);
    var rgb = max(color.rgb, vec3<f32>(0.0)) * strength;
    if (lit) {
        rgb = rgb * lighting_at(view_position.xyz);
    }
    output.color = vec4<f32>(rgb, clamp(color.a, 0.0, 1.0));
    return output;
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32, @location(0) position_size: vec4<f32>, @location(1) color: vec4<f32>, @location(2) pulse: vec4<f32>) -> VertexOutput {
    return facing_vertex(index, position_size, color, pulse, false);
}

@vertex
fn vs_lit(@builtin(vertex_index) index: u32, @location(0) position_size: vec4<f32>, @location(1) color: vec4<f32>, @location(2) pulse: vec4<f32>) -> VertexOutput {
    return facing_vertex(index, position_size, color, pulse, true);
}

@vertex
fn vs_oriented(@builtin(vertex_index) index: u32, @location(0) position_size: vec4<f32>, @location(1) color: vec4<f32>, @location(2) pulse: vec4<f32>, @location(3) right: vec3<f32>, @location(4) up: vec3<f32>) -> VertexOutput {
    let corner = quad_corner(index);
    // Fixed in the scene: the corner is offset along the glow's own axes (their lengths stretch
    // it), so the quad lies or stands where it was put and is seen at an angle.
    let half = position_size.w * 0.5;
    let world = position_size.xyz + (right * corner.x + up * corner.y) * half;
    let view_position = globals.view * vec4<f32>(position_size.xyz, 1.0);
    var output: VertexOutput;
    output.clip = globals.projection * (globals.view * vec4<f32>(world, 1.0));
    output.corner = corner;
    let strength = strength_of(pulse, view_position.xyz, position_size.w);
    output.color = vec4<f32>(max(color.rgb, vec3<f32>(0.0)) * strength, clamp(color.a, 0.0, 1.0));
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // The texture's texel at this corner (top of the texture at the top of the quad), as
    // premultiplied colour; or the round falloff.
    let uv = vec2<f32>(input.corner.x * 0.5 + 0.5, 0.5 - input.corner.y * 0.5);
    let texel = textureSample(shape_texture, shape_sampler, uv);
    let falloff = glow_falloff(input.corner);
    let shape = select(vec4<f32>(falloff), vec4<f32>(texel.rgb * texel.a, texel.a), globals.params.z > 0.5);
    let alpha = shape.a * input.color.a;
    var rgb = input.color.rgb * shape.rgb * input.color.a;
    if (globals.params.y > 0.5) {
        rgb = encode_srgb(rgb);
    }
    // Premultiplied: the additive pipeline adds rgb (and keeps the target's alpha); the
    // transparent pipeline blends it over what is there.
    return vec4<f32>(rgb, alpha);
}
