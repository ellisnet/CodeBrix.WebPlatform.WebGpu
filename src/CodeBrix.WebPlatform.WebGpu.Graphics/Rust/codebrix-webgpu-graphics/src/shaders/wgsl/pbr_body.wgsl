const PI: f32 = 3.14159265358979;
const MAX_LIGHTS: u32 = 8u;
const MAX_LOCAL_LIGHTS: u32 = 16u;

struct Light {
    direction: vec4<f32>,
    color: vec4<f32>,
};

// A point light or spotlight (Not in C#).
struct LocalLight {
    // xyz: where it is; w: 1 / range (0: no end).
    position: vec4<f32>,
    // rgb: colour times intensity; w: 1 for a spotlight.
    color: vec4<f32>,
    // xyz: the spotlight's axis (unit); w: cos(outer angle).
    direction: vec4<f32>,
    // x: cos(inner angle); y: shadow layer, below 0 for none; z: the shadow test's normal offset per unit of distance.
    extra: vec4<f32>,
};

struct FrameUniforms {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    ambient_sky: vec4<f32>,
    ambient_ground: vec4<f32>,
    ambient_up: vec4<f32>,
    environment: vec4<f32>,
    lights: array<Light, 8>,
    // x: local light count; y: 1 when light 0 (the key light) casts the sun's shadow; z: the sun's normal offset (the
    // first cascade's); w: the sun's cascades (0 or 1: the single map over the region).
    local: vec4<f32>,
    // x: strength; y: samples per side; z: 1 / map size; w: depth bias (the first cascade's).
    sun_shadow: vec4<f32>,
    spot_shadow: vec4<f32>,
    // The sun's view-projection (the first cascade's).
    sun_matrix: mat4x4<f32>,
    // Four view-projection matrices, four columns each (no array of matrices: see load_matrix).
    spot_matrices: array<vec4<f32>, 16>,
    local_lights: array<LocalLight, 16>,
    // The sun's cascades (Not in C#; read when local.w is above 1): xyz the camera's view direction, the axis the
    // cascades are cut along; w the blend band, a share of a slice's depth.
    sun_view: vec4<f32>,
    // The far end of each cascade, depth along sun_view.xyz from the camera.
    sun_splits: vec4<f32>,
    // Each cascade's depth bias and normal offset (the first's are also sun_shadow.w and local.z).
    sun_biases: vec4<f32>,
    sun_offsets: vec4<f32>,
    // Cascades 1 to 3's view-projections, four columns each (cascade 0's is sun_matrix).
    sun_cascade_matrices: array<vec4<f32>, 12>,
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
    // The instance's emissive strength; tint.rgb multiplies the base colour, tint.a the opacity.
    emissive_scale: f32,
    tint: vec4<f32>,
    // x bit 0: the instance receives shadows; y: the level-of-detail fade's dither code (0: none); z, w reserved.
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

@group(0) @binding(0) var<uniform> frame: FrameUniforms;
@group(0) @binding(1) var irradiance_map: texture_cube<f32>;
@group(0) @binding(2) var prefiltered_map: texture_cube<f32>;
@group(0) @binding(3) var brdf_lut: texture_2d<f32>;
@group(0) @binding(4) var environment_sampler: sampler;
// One layer per cascade of the sun's shadow (one: the single map).
@group(0) @binding(5) var sun_shadow_maps: texture_depth_2d_array;
@group(0) @binding(6) var spot_shadow_maps: texture_depth_2d_array;
@group(0) @binding(7) var shadow_sampler: sampler_comparison;

@group(1) @binding(0) var<uniform> material: MaterialUniforms;
@group(1) @binding(1) var base_texture: texture_2d<f32>;
@group(1) @binding(2) var metallic_roughness_texture: texture_2d<f32>;
@group(1) @binding(3) var normal_texture: texture_2d<f32>;
@group(1) @binding(4) var occlusion_texture: texture_2d<f32>;
@group(1) @binding(5) var emissive_texture: texture_2d<f32>;
@group(1) @binding(6) var base_sampler: sampler;
@group(1) @binding(7) var metallic_roughness_sampler: sampler;
@group(1) @binding(8) var normal_sampler: sampler;
@group(1) @binding(9) var occlusion_sampler: sampler;
@group(1) @binding(10) var emissive_sampler: sampler;

@group(2) @binding(0) var<storage, read> instances: array<Instance>;
// Matrices are read as four vec4 columns, not as an array of mat4x4: some mobile drivers (Adreno 6xx Vulkan) read a
// runtime-sized storage array of bare matrices wrongly. Same memory layout (64-byte stride, column-major).
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
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv0: vec2<f32>,
    @location(4) uv1: vec2<f32>,
    @location(5) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) uv0: vec2<f32>,
    @location(4) uv1: vec2<f32>,
    @location(5) color: vec4<f32>,
    @location(6) @interpolate(flat) tint: vec4<f32>,
    @location(7) @interpolate(flat) emissive_scale: f32,
    @location(8) @interpolate(flat) instance_flags: u32,
    @location(9) @interpolate(flat) dither: u32,
};

@vertex
fn vs_main(input: VertexInput, @builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    let instance = instances[instance_index];
    var position = input.position;
    var normal = input.normal;
    var tangent = input.tangent.xyz;
    if (MORPHED) {
        let local_vertex = vertex_index - draw.base_vertex;
        for (var morph = 0u; morph < draw.morph_targets; morph++) {
            let weight = morph_weights[instance.weight_base + draw.morph_weight_offset + morph];
            if (weight != 0.0) {
                var at = draw.morph_delta_base + (morph * draw.morph_vertex_count + local_vertex) * draw.morph_stride;
                if ((draw.morph_channels & 1u) != 0u) {
                    position += weight * morph_deltas[at].xyz;
                    at++;
                }
                if ((draw.morph_channels & 2u) != 0u) {
                    normal += weight * morph_deltas[at].xyz;
                    at++;
                }
                if ((draw.morph_channels & 4u) != 0u) {
                    tangent += weight * morph_deltas[at].xyz;
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

    let world = model * vec4<f32>(position, 1.0);
    let basis = mat3x3<f32>(model[0].xyz, model[1].xyz, model[2].xyz);
    let cofactor = mat3x3<f32>(cross(basis[1], basis[2]), cross(basis[2], basis[0]), cross(basis[0], basis[1]));
    let handedness = select(-1.0, 1.0, dot(basis[0], cofactor[0]) >= 0.0);
    var output: VertexOutput;
    output.clip = frame.view_projection * world;
    output.world_position = world.xyz;
    output.normal = cofactor * normal * handedness;
    // A mirroring model matrix flips cross(N, T), so the bitangent sign flips with it.
    output.tangent = vec4<f32>(basis * tangent, input.tangent.w * handedness);
    output.uv0 = input.uv0;
    output.uv1 = input.uv1;
    output.color = input.color;
    output.tint = instance.tint;
    output.emissive_scale = instance.emissive_scale;
    output.instance_flags = instance.flags.x;
    output.dither = instance.flags.y;
    return output;
}

fn slot_uv(slot: u32, uv0: vec2<f32>, uv1: vec2<f32>) -> vec2<f32> {
    let source = vec3<f32>(select(uv0, uv1, ((material.flags.y >> slot) & 1u) != 0u), 1.0);
    return vec2<f32>(dot(material.uv[slot * 2u].xyz, source), dot(material.uv[slot * 2u + 1u].xyz, source));
}

fn fresnel_schlick(f0: vec3<f32>, cosine: f32) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(clamp(1.0 - cosine, 0.0, 1.0), 5.0);
}

fn distribution_ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let alpha2 = alpha * alpha;
    let f = n_dot_h * n_dot_h * (alpha2 - 1.0) + 1.0;
    return alpha2 / (PI * f * f);
}

fn visibility_smith(n_dot_l: f32, n_dot_v: f32, alpha: f32) -> f32 {
    let alpha2 = alpha * alpha;
    let ggx_v = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - alpha2) + alpha2);
    let ggx_l = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - alpha2) + alpha2);
    let sum = ggx_v + ggx_l;
    return select(0.0, 0.5 / sum, sum > 0.0);
}

fn environment_brdf_approx(f0: vec3<f32>, roughness: f32, n_dot_v: f32) -> vec3<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * n_dot_v)) * r.x + r.y;
    let ab = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    return f0 * ab.x + vec3<f32>(ab.y);
}

// World direction -> environment direction (the environment turns by frame.environment.w about +Y).
fn to_environment(direction: vec3<f32>) -> vec3<f32> {
    let c = cos(frame.environment.w);
    let s = sin(frame.environment.w);
    return vec3<f32>(c * direction.x - s * direction.z, direction.y, s * direction.x + c * direction.z);
}

fn environment_specular(direction: vec3<f32>, roughness: f32) -> vec3<f32> {
    return textureSampleLevel(prefiltered_map, environment_sampler, to_environment(direction), roughness * frame.environment.z).rgb * frame.environment.y;
}

fn hemisphere(direction: vec3<f32>) -> vec3<f32> {
    let t = clamp(dot(direction, frame.ambient_up.xyz) * 0.5 + 0.5, 0.0, 1.0);
    return mix(frame.ambient_ground.rgb, frame.ambient_sky.rgb, t);
}

fn encode_srgb(value: vec3<f32>) -> vec3<f32> {
    let c = clamp(value, vec3<f32>(0.0), vec3<f32>(1.0));
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(high, low, c <= vec3<f32>(0.0031308));
}

// What the light of one light does on the surface (the same for directional and local lights).
struct Surface {
    n: vec3<f32>,
    v: vec3<f32>,
    n_dot_v: f32,
    f0: vec3<f32>,
    diffuse_color: vec3<f32>,
    alpha: f32,
    coat_normal: vec3<f32>,
    clearcoat: f32,
    clearcoat_alpha: f32,
    clearcoat_n_dot_v: f32,
};

// The light `light_color` arriving from direction `l` (towards the light) reflects off the surface.
fn direct_light(s: Surface, l: vec3<f32>, light_color: vec3<f32>) -> vec3<f32> {
    let n_dot_l = dot(s.n, l);
    // (Not `<= 0.0`: a NaN must skip the light, as the loop this came from did.)
    if (!(n_dot_l > 0.0)) {
        return vec3<f32>(0.0);
    }
    let h = normalize(l + s.v);
    let v_dot_h = clamp(dot(s.v, h), 0.0, 1.0);
    let n_dot_h = clamp(dot(s.n, h), 0.0, 1.0);
    let f = fresnel_schlick(s.f0, v_dot_h);
    let diffuse = (vec3<f32>(1.0) - f) * s.diffuse_color / PI;
    let specular = f * distribution_ggx(n_dot_h, s.alpha) * visibility_smith(n_dot_l, s.n_dot_v, s.alpha);
    var radiance = (diffuse + specular) * light_color * (PI * n_dot_l);
    if (CLEARCOAT) {
        let c_dot_l = clamp(dot(s.coat_normal, l), 0.0, 1.0);
        let c_dot_h = clamp(dot(s.coat_normal, h), 0.0, 1.0);
        let coat_f = fresnel_schlick(vec3<f32>(0.04), v_dot_h).x * s.clearcoat;
        let coat = coat_f * distribution_ggx(c_dot_h, s.clearcoat_alpha) * visibility_smith(c_dot_l, s.clearcoat_n_dot_v, s.clearcoat_alpha) * PI * c_dot_l;
        radiance = radiance * (1.0 - coat_f) + light_color * coat;
    }
    return radiance;
}

// A local light's distance curve: the inverse square, eased to exactly zero at the range (Not in C#).
fn local_falloff(distance: f32, inverse_range: f32) -> f32 {
    let x = distance * inverse_range;
    let x2 = x * x;
    let window = clamp(1.0 - x2 * x2, 0.0, 1.0);
    let d = max(distance, 0.01);
    return window * window / (d * d);
}

// A spotlight's cone: 1 inside the inner cone, 0 outside the outer one, a smooth step between.
fn local_cone(cosine: f32, cos_inner: f32, cos_outer: f32) -> f32 {
    let t = clamp((cosine - cos_outer) / max(cos_inner - cos_outer, 1e-4), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

// The light a shadow map lets through at `uv`, depth `depth`: the average of a square grid of filtered comparisons one
// texel apart, `grid` per side (Not in C#); `layer` the sun's cascade.
fn sun_shadow_light(uv: vec2<f32>, layer: i32, depth: f32) -> f32 {
    let grid = i32(frame.sun_shadow.y);
    let texel = frame.sun_shadow.z;
    let start = -0.5 * f32(grid - 1);
    var sum = 0.0;
    for (var y = 0; y < grid; y++) {
        for (var x = 0; x < grid; x++) {
            let offset = vec2<f32>(start + f32(x), start + f32(y)) * texel;
            sum += textureSampleCompareLevel(sun_shadow_maps, shadow_sampler, uv + offset, layer, depth);
        }
    }
    return sum / f32(grid * grid);
}

fn spot_shadow_light(uv: vec2<f32>, layer: i32, depth: f32) -> f32 {
    let grid = i32(frame.spot_shadow.y);
    let texel = frame.spot_shadow.z;
    let start = -0.5 * f32(grid - 1);
    var sum = 0.0;
    for (var y = 0; y < grid; y++) {
        for (var x = 0; x < grid; x++) {
            let offset = vec2<f32>(start + f32(x), start + f32(y)) * texel;
            sum += textureSampleCompareLevel(spot_shadow_maps, shadow_sampler, uv + offset, layer, depth);
        }
    }
    return sum / f32(grid * grid);
}

// Clip -> shadow map: x, y to 0..1 (y down), z the depth; false outside the map.
fn shadow_coordinates(clip: vec4<f32>) -> vec4<f32> {
    if (clip.w <= 0.0) {
        return vec4<f32>(0.0, 0.0, 0.0, -1.0);
    }
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let inside = all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)) && ndc.z >= 0.0 && ndc.z <= 1.0;
    return vec4<f32>(uv, ndc.z, select(-1.0, 1.0, inside));
}

// How much of the sun's light reaches a point through one cascade's map (1 outside it): cascade 0 with sun_matrix,
// local.z and sun_shadow.w (the single map's numbers), the others with theirs.
fn sun_cascade_visibility(cascade: u32, world: vec3<f32>, normal: vec3<f32>) -> f32 {
    var matrix = frame.sun_matrix;
    var offset = frame.local.z;
    var bias = frame.sun_shadow.w;
    if (cascade > 0u) {
        let at4 = (cascade - 1u) * 4u;
        matrix = mat4x4<f32>(frame.sun_cascade_matrices[at4], frame.sun_cascade_matrices[at4 + 1u], frame.sun_cascade_matrices[at4 + 2u], frame.sun_cascade_matrices[at4 + 3u]);
        offset = frame.sun_offsets[cascade];
        bias = frame.sun_biases[cascade];
    }
    let at = shadow_coordinates(matrix * vec4<f32>(world + normal * offset, 1.0));
    if (at.w < 0.0) {
        return 1.0;
    }
    return mix(1.0, sun_shadow_light(at.xy, i32(cascade), at.z - bias), frame.sun_shadow.x);
}

// The cascade for a point at `depth` along the camera's view (Not in C#; twin: lighting::shadows::cascade_pick): x the
// first cascade whose far end is beyond the depth (-1 beyond the reach); y how much the next one weighs there, 0 before
// the cascade's blend band (sun_view.w x its slice's depth, the first slice starting at 0), rising to 1 at its far end.
fn cascade_pick(depth: f32, cascades: u32) -> vec2<f32> {
    var start = 0.0;
    for (var index = 0u; index < cascades; index++) {
        let end = frame.sun_splits[index];
        if (depth < end) {
            let band = frame.sun_view.w * (end - start);
            let weight = select(0.0, clamp((depth - (end - band)) / max(band, 1e-6), 0.0, 1.0), band > 0.0);
            return vec2<f32>(f32(index), weight);
        }
        start = end;
    }
    return vec2<f32>(-1.0, 0.0);
}

// How much of the sun's light reaches a point (1 outside the covered region): the single map, or the cascade its depth
// picks, blended into the next one in the cascade's band (the last one fades out to 1 towards the reach; 1 beyond it).
fn sun_visibility(world: vec3<f32>, normal: vec3<f32>) -> f32 {
    let cascades = min(u32(frame.local.w), 4u);
    if (cascades <= 1u) {
        return sun_cascade_visibility(0u, world, normal);
    }
    let pick = cascade_pick(dot(world - frame.camera_position.xyz, frame.sun_view.xyz), cascades);
    if (pick.x < 0.0) {
        return 1.0;
    }
    let cascade = u32(pick.x);
    let near = sun_cascade_visibility(cascade, world, normal);
    if (pick.y <= 0.0) {
        return near;
    }
    var next = 1.0;
    if (cascade + 1u < cascades) {
        next = sun_cascade_visibility(cascade + 1u, world, normal);
    }
    return mix(near, next, pick.y);
}

// How much of a spotlight's light reaches a point, from its layer of the spot shadow maps.
fn spot_visibility(layer: i32, world: vec3<f32>, normal: vec3<f32>, offset: f32) -> f32 {
    let at4 = u32(layer) * 4u;
    let matrix = mat4x4<f32>(frame.spot_matrices[at4], frame.spot_matrices[at4 + 1u], frame.spot_matrices[at4 + 2u], frame.spot_matrices[at4 + 3u]);
    let at = shadow_coordinates(matrix * vec4<f32>(world + normal * offset, 1.0));
    if (at.w < 0.0) {
        return 1.0;
    }
    return mix(1.0, spot_shadow_light(at.xy, layer, at.z - frame.spot_shadow.w), frame.spot_shadow.x);
}

// The level-of-detail fade's dither (Not in C#): the rank 0..63 of a pixel in an 8 x 8 ordered (Bayer) matrix.
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

// The share of pixels a fading instance keeps: what a blended part multiplies its alpha by instead of dithering.
fn dither_coverage(code: u32) -> f32 {
    let step = f32(min(code & 0xFFu, 64u)) / 64.0;
    if ((code & 0x100u) != 0u) {
        return step;
    }
    if ((code & 0x200u) != 0u) {
        return 1.0 - step;
    }
    return 1.0;
}

fn finish(rgb: vec3<f32>, alpha: f32) -> vec4<f32> {
    let exposed = rgb * frame.camera_position.w;
    if (ENCODE_SRGB) {
        return vec4<f32>(encode_srgb(exposed), alpha);
    }
    return vec4<f32>(exposed, alpha);
}

@fragment
fn fs_main(input: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let base_uv = slot_uv(0u, input.uv0, input.uv1);
    let metallic_roughness_uv = slot_uv(1u, input.uv0, input.uv1);
    let normal_uv = slot_uv(2u, input.uv0, input.uv1);
    let occlusion_uv = slot_uv(3u, input.uv0, input.uv1);
    let emissive_uv = slot_uv(4u, input.uv0, input.uv1);
    let base_sample = textureSample(base_texture, base_sampler, base_uv);
    let metallic_roughness_sample = textureSample(metallic_roughness_texture, metallic_roughness_sampler, metallic_roughness_uv);
    let normal_sample = textureSample(normal_texture, normal_sampler, normal_uv);
    let occlusion_sample = textureSample(occlusion_texture, occlusion_sampler, occlusion_uv);
    let emissive_sample = textureSample(emissive_texture, emissive_sampler, emissive_uv);
    let position_dx = dpdx(input.world_position);
    let position_dy = dpdy(input.world_position);
    let uv_dx = dpdx(normal_uv);
    let uv_dy = dpdy(normal_uv);

    var base = material.base_color * input.color * base_sample;
    if (ALPHA_MASK) {
        if (base.a < material.extra.x) {
            discard;
        }
        base.a = 1.0;
    } else if (!ALPHA_BLEND) {
        base.a = 1.0;
    }
    // The instance's tint and alpha (after the cutoff, so a fade keeps a mask's outline).
    base = vec4<f32>(base.rgb * input.tint.rgb, base.a * input.tint.a);
    // A fading level of detail (Not in C#): its dither pattern, per pixel (the same at 1 and 4 samples); a blended part
    // fades by alpha instead.
    if (DITHER_FADE) {
        if (ALPHA_BLEND) {
            base.a = base.a * dither_coverage(input.dither);
        } else if (!dither_keeps(input.clip.xy, input.dither)) {
            discard;
        }
    }

    if (UNLIT) {
        return finish(base.rgb, base.a);
    }

    let geometric = normalize(input.normal);
    var n = geometric;
    var face = 1.0;
    if (DOUBLE_SIDED) {
        face = select(-1.0, 1.0, front_facing);
    }
    if (NORMAL_MAP) {
        var t: vec3<f32>;
        var b: vec3<f32>;
        let denominator = uv_dx.x * uv_dy.y - uv_dy.x * uv_dx.y;
        let inverse = select(0.0, 1.0 / denominator, abs(denominator) > 1e-12);
        let derived = (uv_dy.y * position_dx - uv_dx.y * position_dy) * inverse;
        let derived_bitangent = (uv_dx.x * position_dy - uv_dy.x * position_dx) * inverse;
        let supplied = input.tangent.xyz;
        if (dot(supplied, supplied) > 1e-12) {
            t = normalize(supplied - geometric * dot(geometric, supplied));
            b = cross(geometric, t) * select(-1.0, 1.0, input.tangent.w >= 0.0);
        } else {
            let orthogonal = derived - geometric * dot(geometric, derived);
            t = select(vec3<f32>(1.0, 0.0, 0.0), normalize(orthogonal), dot(orthogonal, orthogonal) > 1e-20);
            // The bitangent points up the image (towards v = 0), i.e. against dP/dv: mirrored UV layouts and mirroring
            // transforms flip it.
            b = cross(geometric, t) * select(1.0, -1.0, dot(cross(geometric, t), derived_bitangent) > 0.0);
        }
        let tangent_normal = (normal_sample.xyz * 2.0 - vec3<f32>(1.0)) * vec3<f32>(material.params.z, material.params.z, 1.0);
        n = normalize(mat3x3<f32>(t, b, geometric) * tangent_normal);
    }
    n = n * face;
    let surface_geometric = geometric * face;

    let metallic = clamp(material.params.x * metallic_roughness_sample.b, 0.0, 1.0);
    let roughness = clamp(material.params.y * metallic_roughness_sample.g, 0.0, 1.0);
    let alpha = max(roughness * roughness, 0.0001);
    let occlusion = 1.0 + material.params.w * (occlusion_sample.r - 1.0);
    let dielectric_f0 = min(vec3<f32>(material.extra.y) * material.specular.rgb, vec3<f32>(1.0)) * material.extra.z;
    let f0 = mix(dielectric_f0, base.rgb, metallic);
    let diffuse_color = base.rgb * (1.0 - metallic);

    let v = normalize(frame.camera_position.xyz - input.world_position);
    let n_dot_v = clamp(dot(n, v), 0.0001, 1.0);
    let clearcoat = material.extra.w;
    let clearcoat_alpha = max(material.emissive.w * material.emissive.w, 0.0001);
    let clearcoat_n_dot_v = clamp(dot(surface_geometric, v), 0.0001, 1.0);

    let surface = Surface(n, v, n_dot_v, f0, diffuse_color, alpha, surface_geometric, clearcoat, clearcoat_alpha, clearcoat_n_dot_v);
    // SHADOWS: whether this instance receives them (instance flag bit 0).
    let receives = SHADOWS && (input.instance_flags & 1u) != 0u;
    var color = vec3<f32>(0.0);
    let light_count = min(u32(frame.ambient_up.w), MAX_LIGHTS);
    for (var index = 0u; index < light_count; index++) {
        let light = frame.lights[index];
        let l = -light.direction.xyz;
        var light_color = light.color.rgb;
        if (SHADOWS) {
            if (receives && index == 0u && frame.local.y > 0.5 && dot(n, l) > 0.0) {
                light_color *= sun_visibility(input.world_position, surface_geometric);
            }
        }
        color += direct_light(surface, l, light_color);
    }
    if (LOCAL_LIGHTS) {
        let local_count = min(u32(frame.local.x), MAX_LOCAL_LIGHTS);
        for (var index = 0u; index < local_count; index++) {
            let light = frame.local_lights[index];
            let to_light = light.position.xyz - input.world_position;
            let distance = length(to_light);
            let l = to_light / max(distance, 1e-6);
            var attenuation = local_falloff(distance, light.position.w);
            if (light.color.w > 0.5) {
                attenuation *= local_cone(dot(-l, light.direction.xyz), light.extra.x, light.direction.w);
            }
            if (attenuation > 0.0 && dot(n, l) > 0.0) {
                if (SHADOWS) {
                    if (receives && light.extra.y >= 0.0) {
                        attenuation *= spot_visibility(i32(light.extra.y), input.world_position, surface_geometric, light.extra.z * distance);
                    }
                }
                color += direct_light(surface, l, light.color.rgb * attenuation);
            }
        }
    }

    let r = reflect(-v, n);
    var ambient_diffuse: vec3<f32>;
    var ambient_specular: vec3<f32>;
    if (frame.environment.x > 0.5) {
        let irradiance = textureSampleLevel(irradiance_map, environment_sampler, to_environment(n), 0.0).rgb * frame.environment.y;
        let prefiltered = environment_specular(r, roughness);
        let scale_bias = textureSampleLevel(brdf_lut, environment_sampler, vec2<f32>(n_dot_v, roughness), 0.0).rg;
        // Split-sum single scattering plus multiple-scattering energy compensation (Fdez-Aguera 2019): a white
        // furnace stays white for every roughness and metalness.
        let single = f0 * scale_bias.x + vec3<f32>(scale_bias.y);
        let multiple_energy = 1.0 - (scale_bias.x + scale_bias.y);
        let average_f = f0 + (vec3<f32>(1.0) - f0) / 21.0;
        let multiple = single * average_f / (vec3<f32>(1.0) - multiple_energy * average_f);
        let diffuse_weight = diffuse_color * (vec3<f32>(1.0) - single - multiple * multiple_energy);
        ambient_diffuse = irradiance * diffuse_weight + irradiance * multiple * multiple_energy;
        ambient_specular = prefiltered * single;
    } else {
        ambient_diffuse = hemisphere(n) * diffuse_color;
        ambient_specular = hemisphere(r) * environment_brdf_approx(f0, roughness, n_dot_v);
    }
    var ambient = ambient_diffuse + ambient_specular;
    if (CLEARCOAT) {
        let coat_f = fresnel_schlick(vec3<f32>(0.04), clearcoat_n_dot_v).x * clearcoat;
        var coat_light: vec3<f32>;
        if (frame.environment.x > 0.5) {
            coat_light = environment_specular(reflect(-v, surface_geometric), material.emissive.w);
        } else {
            coat_light = hemisphere(reflect(-v, surface_geometric));
        }
        ambient = ambient * (1.0 - coat_f) + coat_light * coat_f;
    }
    color += ambient * occlusion;
    // flags.z: the material multiplies its emissive by the vertex colour.
    let emissive_color = select(vec3<f32>(1.0), input.color.rgb, material.flags.z != 0u);
    color += material.emissive.rgb * emissive_sample.rgb * emissive_color * input.emissive_scale;
    return finish(color, base.a);
}
