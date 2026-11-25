// The glow layer's and the particle system's drawing (Not in C#): one camera-facing quad per
// instance, all instances of a layer in one instanced draw. Instance record (48 bytes, three
// vec4): position + size (scene units, the quad's width), linear colour + opacity, and intensity,
// pulse rate (cycles per second), pulse depth (0..1), pulse phase (cycles).

struct Globals {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    // x: clock time in seconds; y: 1 = encode to sRGB in the shader; z: 1 = shape from the
    // texture instead of the round falloff; w: unused.
    params: vec4<f32>,
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

@vertex
fn vs_main(@builtin(vertex_index) index: u32, @location(0) position_size: vec4<f32>, @location(1) color: vec4<f32>, @location(2) pulse: vec4<f32>) -> VertexOutput {
    let corner = quad_corner(index);
    // Camera-facing: the corner is offset in view space, so the quad always faces the camera
    // (also under an orthographic projection). A size of 0 collapses the quad: nothing is drawn.
    let view_position = globals.view * vec4<f32>(position_size.xyz, 1.0);
    let offset = vec4<f32>(corner * (position_size.w * 0.5), 0.0, 0.0);
    var output: VertexOutput;
    output.clip = globals.projection * (view_position + offset);
    output.corner = corner;
    let strength = max(pulse.x, 0.0) * pulse_factor(pulse.y, pulse.z, pulse.w, globals.params.x);
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
