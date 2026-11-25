// The model renderer's shadow passes (Not in C#): depth only, from a light's view-projection. The
// records, layouts and vertex movement are the PBR shader's (groups 1 to 3 are its material,
// instance and draw groups, bound unchanged); group 0 holds only the light's matrix.

struct ShadowUniforms {
    view_projection: mat4x4<f32>,
};

struct MaterialUniforms {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    params: vec4<f32>,
    extra: vec4<f32>,
    specular: vec4<f32>,
    flags: vec4<u32>,
    uv: array<vec4<f32>, 10>,
};

struct Instance {
    world: mat4x4<f32>,
    node_base: u32,
    joint_base: u32,
    weight_base: u32,
    emissive_scale: f32,
    tint: vec4<f32>,
    flags: vec4<u32>,
};

struct DrawUniforms {
    node: u32,
    base_vertex: u32,
    skin_base: u32,
    joint_offset: u32,
    morph_targets: u32,
    morph_weight_offset: u32,
    morph_delta_base: u32,
    morph_vertex_count: u32,
    morph_channels: u32,
    morph_stride: u32,
    pad0: u32,
    pad1: u32,
};

@group(0) @binding(0) var<uniform> shadow: ShadowUniforms;

@group(1) @binding(0) var<uniform> material: MaterialUniforms;
@group(1) @binding(1) var base_texture: texture_2d<f32>;
@group(1) @binding(6) var base_sampler: sampler;

@group(2) @binding(0) var<storage, read> instances: array<Instance>;
// Matrices as four vec4 columns each (see the PBR shader's load_matrix).
@group(2) @binding(1) var<storage, read> matrices: array<vec4<f32>>;
@group(2) @binding(2) var<storage, read> morph_weights: array<f32>;

@group(3) @binding(0) var<uniform> draw: DrawUniforms;
@group(3) @binding(1) var<storage, read> skin_stream: array<u32>;
@group(3) @binding(2) var<storage, read> morph_deltas: array<vec4<f32>>;

fn load_matrix(index: u32) -> mat4x4<f32> {
    let at = index * 4u;
    return mat4x4<f32>(matrices[at], matrices[at + 1u], matrices[at + 2u], matrices[at + 3u]);
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(3) uv0: vec2<f32>,
    @location(4) uv1: vec2<f32>,
    @location(5) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv0: vec2<f32>,
    @location(1) uv1: vec2<f32>,
    @location(2) color: vec4<f32>,
    // The level-of-detail fade's dither code (instance flags.y; 0: none).
    @location(3) @interpolate(flat) dither: u32,
};

@vertex
fn vs_shadow(input: VertexInput, @builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    let instance = instances[instance_index];
    var position = input.position;
    if (MORPHED) {
        if ((draw.morph_channels & 1u) != 0u) {
            let local_vertex = vertex_index - draw.base_vertex;
            for (var morph = 0u; morph < draw.morph_targets; morph++) {
                let weight = morph_weights[instance.weight_base + draw.morph_weight_offset + morph];
                if (weight != 0.0) {
                    // Positions are the first channel of each vertex's deltas.
                    position += weight * morph_deltas[draw.morph_delta_base + (morph * draw.morph_vertex_count + local_vertex) * draw.morph_stride].xyz;
                }
            }
        }
    }
    var model: mat4x4<f32>;
    if (SKINNED) {
        let at = (vertex_index - draw.base_vertex + draw.skin_base) * 6u;
        let joints01 = skin_stream[at];
        let joints23 = skin_stream[at + 1u];
        let weights = vec4<f32>(bitcast<f32>(skin_stream[at + 2u]), bitcast<f32>(skin_stream[at + 3u]), bitcast<f32>(skin_stream[at + 4u]), bitcast<f32>(skin_stream[at + 5u]));
        let base = instance.joint_base + draw.joint_offset;
        let skin = weights.x * load_matrix(base + (joints01 & 0xFFFFu))
            + weights.y * load_matrix(base + (joints01 >> 16u))
            + weights.z * load_matrix(base + (joints23 & 0xFFFFu))
            + weights.w * load_matrix(base + (joints23 >> 16u));
        model = instance.world * skin;
    } else {
        model = instance.world * load_matrix(instance.node_base + draw.node);
    }
    var output: VertexOutput;
    output.clip = shadow.view_projection * (model * vec4<f32>(position, 1.0));
    output.uv0 = input.uv0;
    output.uv1 = input.uv1;
    output.color = input.color;
    output.dither = instance.flags.y;
    return output;
}

// The level-of-detail fade's dither, the PBR shader's (the shadow fades with the object, in the map's texels).
fn dither_rank(pixel: vec2<f32>) -> u32 {
    let x = u32(pixel.x) & 7u;
    let y = u32(pixel.y) & 7u;
    let v = x ^ y;
    return ((v & 1u) << 5u) | ((y & 1u) << 4u) | ((v & 2u) << 2u) | ((y & 2u) << 1u) | ((v & 4u) >> 1u) | ((y & 4u) >> 2u);
}

// True when a fading instance keeps this pixel. code bits 0-7: the step (0..64); bit 8: the level coming in keeps the
// ranks below the step; bit 9: the level going out keeps the others, so the two levels of a cross-fade are complementary.
fn dither_keeps(pixel: vec2<f32>, code: u32) -> bool {
    let rank = dither_rank(pixel);
    let step = code & 0xFFu;
    if ((code & 0x100u) != 0u) {
        return rank < step;
    }
    if ((code & 0x200u) != 0u) {
        return rank >= step;
    }
    return true;
}


// Only alpha-tested (MASK) parts and fading parts (DITHER_FADE) have a fragment stage: the cutout's holes, and the
// dither pattern's, let the light through.
@fragment
fn fs_shadow(input: VertexOutput) {
    if (DITHER_FADE) {
        if (!dither_keeps(input.clip.xy, input.dither)) {
            discard;
        }
    }
    if (ALPHA_MASK) {
        let source = vec3<f32>(select(input.uv0, input.uv1, (material.flags.y & 1u) != 0u), 1.0);
        let uv = vec2<f32>(dot(material.uv[0].xyz, source), dot(material.uv[1].xyz, source));
        let alpha = material.base_color.a * input.color.a * textureSampleLevel(base_texture, base_sampler, uv, 0.0).a;
        if (alpha < material.extra.x) {
            discard;
        }
    }
}
