//! The uniform and storage records of the PBR model shader and the CPU-only parts of the model
//! renderer (C#: the internal `PbrFrameUniforms`, `PbrMaterialUniforms`, `PbrInstance`,
//! `PbrDrawUniforms` and the packing of `PbrModelRenderPath`). Tested on the host.

use glam::{Mat3, Mat4, Vec3, Vec4};

use crate::drawing3d::{Camera, MaterialAlphaMode, SceneLighting};
use crate::lighting::local_lights::LocalLight;
use crate::lighting::shadows::{sample_grid, ShadowView};
use crate::models::{ModelMaterial, ModelMorphTarget, ModelPrimitive};
use crate::pipelines::CullPreset;
use crate::shaders::PbrFeatures;

/// The frame uniforms of the model shader (1792 bytes; C#: `PbrFrameUniforms`, 400 bytes: the
/// local lights and the shadows after the directional lights are Not in C#).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PbrFrameUniforms {
    pub(crate) view_projection: Mat4,
    pub(crate) camera_position: Vec4,
    pub(crate) ambient_sky: Vec4,
    pub(crate) ambient_ground: Vec4,
    pub(crate) ambient_up: Vec4,
    pub(crate) environment: Vec4,
    pub(crate) lights: [Vec4; 16],
    /// x: the local light count; y: 1 when light 0 (the key light) casts the sun's shadow; z: the
    /// sun's normal offset (world units).
    pub(crate) local: Vec4,
    /// The sun's shadow: strength, samples per side, 1 / map size, depth bias.
    pub(crate) sun_shadow: Vec4,
    /// The spotlights' shadows: as `sun_shadow`.
    pub(crate) spot_shadow: Vec4,
    pub(crate) sun_matrix: Mat4,
    pub(crate) spot_matrices: [Mat4; 4],
    /// Four Vec4 per local light: position + 1 / range, colour x intensity + spot flag, axis +
    /// cos(outer), cos(inner) + shadow layer + normal offset per unit distance + 0.
    pub(crate) local_lights: [Vec4; 64],
}

pub(crate) const FRAME_UNIFORM_SIZE: u64 = 1792;
const _: () = assert!(std::mem::size_of::<PbrFrameUniforms>() == FRAME_UNIFORM_SIZE as usize);
/// The bytes between two frames' uniforms in the frame buffer: the size rounded up to the WebGPU
/// dynamic-offset alignment.
pub(crate) const FRAME_SLOT_SIZE: u64 = FRAME_UNIFORM_SIZE.div_ceil(256) * 256;
/// The depth bias of the sun's shadow test, in the map's depth (0..1).
const SUN_DEPTH_BIAS: f32 = 0.0002;
/// The depth bias of a spotlight's shadow test.
const SPOT_DEPTH_BIAS: f32 = 0.00005;
/// The normal offset of the shadow test, in texels of the map.
const NORMAL_OFFSET_TEXELS: f32 = 1.5;
// SAFETY: #[repr(C)]; Mat4s and Vec4s only, all 16-byte multiples at 16-byte offsets, no padding
// (size asserted above); every bit pattern of f32 is a valid value.
unsafe impl bytemuck::Zeroable for PbrFrameUniforms {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for PbrFrameUniforms {}

impl PbrFrameUniforms {
    /// C#'s `Begin`: the camera, the exposure, the hemispheric ambient and up to eight
    /// directional lights (the key light first, then the additional ones; a light with no
    /// intensity or no direction is skipped). `environment.x` is 0 (the hemispheric ambient):
    /// the renderer sets `environment` when it picks an environment for image-based lighting
    /// (C#'s `SelectEnvironment`).
    pub(crate) fn new(camera: &Camera, lighting: &SceneLighting, aspect_ratio: f32) -> PbrFrameUniforms {
        let sky = lighting.ambient_color;
        let ground = lighting.ambient_ground_color.unwrap_or(sky);
        let up = if lighting.ambient_up.length_squared() > 0.0 { lighting.ambient_up.normalize() } else { Vec3::Y };
        let mut frame = PbrFrameUniforms {
            view_projection: camera.get_view_projection_matrix(aspect_ratio),
            camera_position: camera.position.extend(lighting.exposure),
            ambient_sky: sky.to_vector4(),
            ambient_ground: ground.to_vector4(),
            ambient_up: Vec4::ZERO,
            environment: Vec4::new(0.0, 0.0, 0.0, lighting.environment_rotation),
            lights: [Vec4::ZERO; 16],
            local: Vec4::ZERO,
            sun_shadow: Vec4::ZERO,
            spot_shadow: Vec4::ZERO,
            sun_matrix: Mat4::IDENTITY,
            spot_matrices: [Mat4::IDENTITY; 4],
            local_lights: [Vec4::ZERO; 64],
        };
        let mut count = 0;
        let mut add = |direction: Vec3, color: crate::Color4, intensity: f32| {
            if count >= SceneLighting::MAX_DIRECTIONAL_LIGHTS || intensity == 0.0 || direction.length_squared() == 0.0 {
                return;
            }
            frame.lights[count * 2] = direction.normalize().extend(0.0);
            frame.lights[count * 2 + 1] = Vec4::new(color.r * intensity, color.g * intensity, color.b * intensity, 0.0);
            count += 1;
        };
        add(lighting.light_direction, lighting.light_color, lighting.light_intensity);
        for light in &lighting.additional_lights {
            add(light.direction, light.color, light.intensity);
        }
        frame.ambient_up = up.extend(count as f32);
        frame
    }

    /// True when the key light took the first light slot (it is lit), so the sun's shadow can
    /// darken it.
    pub(crate) fn key_light_shades(lighting: &SceneLighting) -> bool {
        lighting.light_intensity != 0.0 && lighting.light_direction.length_squared() > 0.0
    }

    /// Writes the chosen local lights (`chosen`, indices into `lighting.local_lights`, at most 16)
    /// and their shadow layers (`spot_layers[i]`: the layer of `chosen[i]`, if it casts) (Not in
    /// C#). The spots' views give each casting spot its normal offset.
    pub(crate) fn set_local_lights(&mut self, lights: &[LocalLight], chosen: &[usize], spot_layers: &[Option<usize>], spot_views: &[ShadowView]) {
        let count = chosen.len().min(SceneLighting::MAX_LOCAL_LIGHTS);
        for (slot, &index) in chosen.iter().take(count).enumerate() {
            let light = &lights[index];
            let range = light.range();
            let inverse_range = if range.is_finite() { 1.0 / range } else { 0.0 };
            let radiance = light.radiance();
            let at = slot * 4;
            self.local_lights[at] = light.position().extend(inverse_range);
            match light.as_spot() {
                Some(spot) => {
                    let layer = spot_layers.get(slot).copied().flatten();
                    let offset = layer.and_then(|l| spot_views.get(l)).map_or(0.0, |view| view.texel * NORMAL_OFFSET_TEXELS);
                    self.local_lights[at + 1] = radiance.extend(1.0);
                    self.local_lights[at + 2] = spot.direction.normalize().extend(spot.clamped_outer_angle().cos());
                    self.local_lights[at + 3] = Vec4::new(spot.clamped_inner_angle().cos(), layer.map_or(-1.0, |l| l as f32), offset, 0.0);
                }
                None => {
                    self.local_lights[at + 1] = radiance.extend(0.0);
                    self.local_lights[at + 2] = Vec4::new(0.0, -1.0, 0.0, -1.0);
                    self.local_lights[at + 3] = Vec4::new(1.0, -1.0, 0.0, 0.0);
                }
            }
        }
        self.local.x = count as f32;
    }

    /// Writes the shadows of the pair (Not in C#): the sun's view (when its shadow is drawn) and
    /// the spotlights' views (layer order), with their settings.
    pub(crate) fn set_shadows(&mut self, lighting: &SceneLighting, sun: Option<&ShadowView>, spots: &[ShadowView]) {
        if let Some(view) = sun {
            let settings = &lighting.sun_shadows;
            self.local.y = 1.0;
            self.local.z = view.texel * NORMAL_OFFSET_TEXELS;
            self.sun_matrix = view.view_projection;
            self.sun_shadow = Vec4::new(settings.strength.clamp(0.0, 1.0), sample_grid(settings.softness) as f32, 1.0 / settings.map_size.texels() as f32, SUN_DEPTH_BIAS);
        }
        for (layer, view) in spots.iter().take(SceneLighting::MAX_SHADOWED_SPOTLIGHTS).enumerate() {
            self.spot_matrices[layer] = view.view_projection;
        }
        if !spots.is_empty() {
            let settings = &lighting.spot_shadows;
            self.spot_shadow = Vec4::new(settings.strength.clamp(0.0, 1.0), sample_grid(settings.softness) as f32, 1.0 / settings.map_size.texels() as f32, SPOT_DEPTH_BIAS);
        }
    }
}

/// The material uniforms of the model shader (256 bytes; C#: `PbrMaterialUniforms`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PbrMaterialUniforms {
    pub(crate) base_color: Vec4,
    pub(crate) emissive: Vec4,
    pub(crate) params: Vec4,
    pub(crate) extra: Vec4,
    pub(crate) specular: Vec4,
    pub(crate) flags: [u32; 4],
    pub(crate) uv: [Vec4; 10],
}

pub(crate) const MATERIAL_UNIFORM_SIZE: u64 = 256;
const _: () = assert!(std::mem::size_of::<PbrMaterialUniforms>() == MATERIAL_UNIFORM_SIZE as usize);
// SAFETY: #[repr(C)]; Vec4s and four u32 at 16-byte offsets, no padding (size asserted above).
unsafe impl bytemuck::Zeroable for PbrMaterialUniforms {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for PbrMaterialUniforms {}

impl PbrMaterialUniforms {
    /// C#'s `CreateMaterial`: the factors, the dielectric reflectance from the index of
    /// refraction, the texture-coordinate set of each slot (a bit each) and each slot's
    /// transform as two rows; `flags[2]` is 1 when the emissive is multiplied by the vertex
    /// colour ([`ModelMaterial::emissive_vertex_color`], Not in C#).
    pub(crate) fn new(source: &ModelMaterial) -> PbrMaterialUniforms {
        let ior = source.index_of_refraction.max(1.0);
        let f0 = (ior - 1.0) / (ior + 1.0);
        let mut tex_coords = 0u32;
        let mut uv = [Vec4::ZERO; 10];
        for (slot, texture) in source.slots().iter().enumerate() {
            if texture.tex_coord == 1 {
                tex_coords |= 1 << slot;
            }
            let t = texture.transform;
            uv[slot * 2] = Vec4::new(t[0], t[2], t[4], 0.0);
            uv[slot * 2 + 1] = Vec4::new(t[1], t[3], t[5], 0.0);
        }
        PbrMaterialUniforms {
            base_color: source.base_color_factor,
            emissive: (source.emissive_factor * source.emissive_strength).extend(source.clear_coat_roughness),
            params: Vec4::new(source.metallic_factor, source.roughness_factor, source.normal_scale, source.occlusion_strength),
            extra: Vec4::new(source.alpha_cutoff, f0 * f0, source.specular_factor, source.clear_coat_factor),
            specular: source.specular_color.extend(0.0),
            flags: [0, tex_coords, u32::from(source.emissive_vertex_color), 0],
            uv,
        }
    }
}

/// The shader features a material needs (C#: the feature half of `CreateMaterial`); a normal
/// map counts only when its texture was loaded.
pub(crate) fn material_features(source: &ModelMaterial, has_normal_texture: bool) -> PbrFeatures {
    let mut features = PbrFeatures::NONE;
    if has_normal_texture {
        features |= PbrFeatures::NORMAL_MAP;
    }
    if source.unlit {
        features |= PbrFeatures::UNLIT;
    }
    match source.alpha_mode {
        MaterialAlphaMode::Mask => features |= PbrFeatures::ALPHA_MASK,
        MaterialAlphaMode::Blend => features |= PbrFeatures::ALPHA_BLEND,
        _ => {}
    }
    if source.double_sided {
        features |= PbrFeatures::DOUBLE_SIDED;
    }
    if source.clear_coat_factor > 0.0 {
        features |= PbrFeatures::CLEAR_COAT;
    }
    features
}

/// One instance record of the instance storage buffer (112 bytes; C#: `PbrInstance`, 80 bytes:
/// the emissive strength, the tint and the flags are Not in C#).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PbrInstance {
    pub(crate) world: Mat4,
    pub(crate) node_base: u32,
    pub(crate) joint_base: u32,
    pub(crate) weight_base: u32,
    /// The instance's emissive strength (multiplies the material's emissive).
    pub(crate) emissive: f32,
    /// The instance's tint (rgb, multiplies the base colour) and alpha (a, multiplies the opacity).
    pub(crate) tint: Vec4,
    /// `flags[0]` bit 0: the instance receives shadows ([`INSTANCE_RECEIVES_SHADOWS`]);
    /// `flags[1]`: the level-of-detail fade's dither code (0: none; `lod_dither`, Not in C#);
    /// `flags[2..]` reserved (0).
    pub(crate) flags: [u32; 4],
}

/// The instance flag: the instance receives shadows.
pub(crate) const INSTANCE_RECEIVES_SHADOWS: u32 = 1;

pub(crate) const INSTANCE_SIZE: u64 = 112;
const _: () = assert!(std::mem::size_of::<PbrInstance>() == INSTANCE_SIZE as usize);
// SAFETY: #[repr(C)]; a Mat4, three u32 and an f32, a Vec4, four u32: 112 bytes, every field at a
// multiple of its alignment, no padding (size asserted above).
unsafe impl bytemuck::Zeroable for PbrInstance {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for PbrInstance {}

/// The per-draw-item uniforms (48 bytes, one 256-byte dynamic-offset slot each; C#:
/// `PbrDrawUniforms`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PbrDrawUniforms {
    pub(crate) node: u32,
    pub(crate) base_vertex: u32,
    pub(crate) skin_base: u32,
    pub(crate) joint_offset: u32,
    pub(crate) morph_targets: u32,
    pub(crate) morph_weight_offset: u32,
    pub(crate) morph_delta_base: u32,
    pub(crate) morph_vertex_count: u32,
    pub(crate) morph_channels: u32,
    pub(crate) morph_stride: u32,
    pub(crate) reserved0: u32,
    pub(crate) reserved1: u32,
}

pub(crate) const DRAW_UNIFORM_SIZE: u64 = 48;
const _: () = assert!(std::mem::size_of::<PbrDrawUniforms>() == DRAW_UNIFORM_SIZE as usize);
// SAFETY: #[repr(C)]; twelve u32, no padding.
unsafe impl bytemuck::Zeroable for PbrDrawUniforms {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for PbrDrawUniforms {}

/// Appends a primitive's morph deltas (target-major, then vertex, then the channels present) and
/// returns (first delta, channels, stride) - (0, 0, 0) when no target moves anything (C#:
/// `PackMorph`).
pub(crate) fn pack_morph(primitive: &ModelPrimitive, deltas: &mut Vec<Vec4>) -> (u32, u32, u32) {
    let mut channels = 0u32;
    for target in &primitive.morph_targets {
        if !target.position_deltas.is_empty() {
            channels |= 1;
        }
        if !target.normal_deltas.is_empty() {
            channels |= 2;
        }
        if !target.tangent_deltas.is_empty() {
            channels |= 4;
        }
    }
    let stride = channels.count_ones();
    if stride == 0 {
        return (0, 0, 0);
    }
    let start = deltas.len() as u32;
    let at = |list: &Vec<Vec3>, vertex: usize| list.get(vertex).copied().unwrap_or(Vec3::ZERO).extend(0.0);
    for target in &primitive.morph_targets {
        let ModelMorphTarget { position_deltas, normal_deltas, tangent_deltas } = target;
        for vertex in 0..primitive.vertex_count as usize {
            if channels & 1 != 0 {
                deltas.push(at(position_deltas, vertex));
            }
            if channels & 2 != 0 {
                deltas.push(at(normal_deltas, vertex));
            }
            if channels & 4 != 0 {
                deltas.push(at(tangent_deltas, vertex));
            }
        }
    }
    (start, channels, stride)
}

/// True for the 8- and 10-bit UNORM colour formats, where the shader encodes to sRGB itself
/// (C#: `NeedsShaderEncoding`; sRGB formats encode in hardware, float formats stay linear).
pub(crate) fn needs_shader_encoding(format: wgpu::TextureFormat) -> bool {
    matches!(format, wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgb10a2Unorm)
}

/// A model pipeline's face culling and front face for its features: clockwise front faces when
/// the draw mirrors (glTF 2.0 rule); [`PbrFeatures::BACK_FACES`] culls the front faces and
/// [`PbrFeatures::FRONT_FACES`] the back faces (the two draws of a blended double-sided part, Not
/// in C#), else a double-sided material culls nothing and every other culls back faces. "Front"
/// is always the front of the mirrored-or-not winding, so a mirrored instance's two draws still
/// draw its far side first.
pub(crate) fn pipeline_culling(features: PbrFeatures) -> (CullPreset, wgpu::FrontFace) {
    let front = if features.contains(PbrFeatures::MIRRORED) { wgpu::FrontFace::Cw } else { wgpu::FrontFace::Ccw };
    let cull = if features.contains(PbrFeatures::BACK_FACES) {
        CullPreset::Front
    } else if features.contains(PbrFeatures::FRONT_FACES) {
        CullPreset::Back
    } else if features.contains(PbrFeatures::DOUBLE_SIDED) {
        CullPreset::None
    } else {
        CullPreset::Back
    };
    (cull, front)
}

/// The determinant of a matrix's upper 3 x 3: negative = the transform mirrors (C#: `Determinant3`).
pub(crate) fn determinant3(matrix: &Mat4) -> f32 {
    Mat3::from_mat4(*matrix).determinant()
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec3, Vec4};

    use super::{determinant3, material_features, needs_shader_encoding, pack_morph, pipeline_culling, PbrFrameUniforms, PbrMaterialUniforms};
    use crate::pipelines::CullPreset;
    use crate::drawing3d::{Camera, DirectionalLight, MaterialAlphaMode, SceneLighting};
    use crate::models::{ModelMaterial, ModelMorphTarget, ModelPrimitive};
    use crate::shaders::PbrFeatures;
    use crate::Color4;

    #[test]
    fn the_frame_carries_the_key_light_then_the_additional_ones() {
        let mut lighting = SceneLighting::default();
        lighting.light_intensity = 2.0;
        lighting.additional_lights.push(DirectionalLight::new(Vec3::ZERO, Color4::WHITE, 1.0));
        lighting.additional_lights.push(DirectionalLight::new(Vec3::X, Color4::new(1.0, 0.5, 0.0, 1.0), 1.0));
        lighting.exposure = 1.5;
        let frame = PbrFrameUniforms::new(&Camera::default(), &lighting, 1.0);
        assert_eq!(frame.ambient_up.w, 2.0);
        assert_eq!(frame.lights[1], Vec4::new(2.0, 2.0, 2.0, 0.0));
        assert_eq!(frame.lights[2], Vec4::new(1.0, 0.0, 0.0, 0.0));
        assert_eq!(frame.lights[3], Vec4::new(1.0, 0.5, 0.0, 0.0));
        assert_eq!(frame.camera_position.w, 1.5);
        assert_eq!(frame.environment.x, 0.0);
        assert_eq!(frame.ambient_ground, lighting.ambient_color.to_vector4());
    }

    #[test]
    fn without_local_lights_or_shadows_the_frame_after_the_directional_lights_is_empty() {
        // The uniforms of C#'s 400 bytes are as before; the rest says "no local lights, no sun
        // shadow", and the shader variant without LOCAL_LIGHTS / SHADOWS does not read it.
        let frame = PbrFrameUniforms::new(&Camera::default(), &SceneLighting::default(), 1.0);
        assert_eq!(frame.local, Vec4::ZERO);
        assert_eq!(frame.sun_shadow, Vec4::ZERO);
        assert!(frame.local_lights.iter().all(|v| *v == Vec4::ZERO));
        assert_eq!(std::mem::offset_of!(PbrFrameUniforms, local), 400);
        assert_eq!(super::FRAME_SLOT_SIZE, 1792);
        assert_eq!(super::FRAME_SLOT_SIZE % 256, 0);
    }

    #[test]
    fn the_frame_carries_the_chosen_local_lights_and_the_shadows() {
        use crate::lighting::shadows::{spot_shadow_view, sun_shadow_view};
        use crate::lighting::{LocalLight, PointLight, SpotLight, SunShadowSettings};
        let mut lighting = SceneLighting::default();
        let spot = SpotLight::new(Vec3::new(0.0, 4.0, 0.0), Vec3::new(0.0, -2.0, 0.0), Color4::new(1.0, 0.5, 0.25, 1.0), 2.0, 10.0, 0.2, 0.4).with_shadows(true);
        lighting.local_lights = vec![LocalLight::from(PointLight::new(Vec3::new(1.0, 2.0, 3.0), Color4::WHITE, 3.0, f32::INFINITY)), LocalLight::from(spot)];
        lighting.sun_shadows = SunShadowSettings { softness: 16, strength: 0.75, ..SunShadowSettings::covering(Vec3::splat(-5.0), Vec3::splat(5.0)) };
        let mut frame = PbrFrameUniforms::new(&Camera::default(), &lighting, 1.0);
        let spot_view = spot_shadow_view(&spot, 1000.0, 1024).unwrap();
        // Chosen: the spot first, then the point light; the spot casts into layer 0.
        frame.set_local_lights(&lighting.local_lights, &[1, 0], &[Some(0), None], std::slice::from_ref(&spot_view));
        let sun = sun_shadow_view(lighting.light_direction, Vec3::splat(-5.0), Vec3::splat(5.0), 2048).unwrap();
        frame.set_shadows(&lighting, Some(&sun), std::slice::from_ref(&spot_view));
        assert_eq!(frame.local.x, 2.0);
        // The spot: position + 1 / range, colour x intensity + spot flag, unit axis + cos(outer), cos(inner) + layer.
        assert_eq!(frame.local_lights[0], Vec4::new(0.0, 4.0, 0.0, 0.1));
        assert_eq!(frame.local_lights[1], Vec4::new(2.0, 1.0, 0.5, 1.0));
        assert_eq!(frame.local_lights[2], Vec4::new(0.0, -1.0, 0.0, 0.4f32.cos()));
        assert_eq!((frame.local_lights[3].x, frame.local_lights[3].y), (0.2f32.cos(), 0.0));
        assert!((frame.local_lights[3].z - spot_view.texel * 1.5).abs() < 1e-9);
        // The point light: no end (inverse range 0), no spot flag, no shadow layer.
        assert_eq!(frame.local_lights[4], Vec4::new(1.0, 2.0, 3.0, 0.0));
        assert_eq!(frame.local_lights[5], Vec4::new(3.0, 3.0, 3.0, 0.0));
        assert_eq!(frame.local_lights[7].y, -1.0);
        // The shadows: the sun on (light 0 is the key light), its matrix and settings; the spot's matrix.
        assert_eq!(frame.local.y, 1.0);
        assert!((frame.local.z - sun.texel * 1.5).abs() < 1e-9);
        assert_eq!(frame.sun_matrix, sun.view_projection);
        assert_eq!((frame.sun_shadow.x, frame.sun_shadow.y, frame.sun_shadow.z), (0.75, 4.0, 1.0 / 2048.0));
        assert_eq!(frame.spot_matrices[0], spot_view.view_projection);
        assert_eq!((frame.spot_shadow.x, frame.spot_shadow.y, frame.spot_shadow.z), (1.0, 3.0, 1.0 / 1024.0));
        // Without the key light the sun's shadow has no light to darken.
        lighting.light_intensity = 0.0;
        assert!(!PbrFrameUniforms::key_light_shades(&lighting));
    }

    #[test]
    fn the_material_uniforms_follow_csharp() {
        let mut material = ModelMaterial::new(0, None);
        material.base_color_texture.transform = [2.0, 0.0, 0.0, 3.0, 0.5, 0.25];
        material.normal_texture.tex_coord = 1;
        material.emissive_factor = Vec3::new(1.0, 0.5, 0.25);
        material.emissive_strength = 2.0;
        let uniforms = PbrMaterialUniforms::new(&material);
        assert_eq!(uniforms.uv[0], Vec4::new(2.0, 0.0, 0.5, 0.0));
        assert_eq!(uniforms.uv[1], Vec4::new(0.0, 3.0, 0.25, 0.0));
        assert_eq!(uniforms.flags[1], 1 << 2);
        assert_eq!(uniforms.emissive, Vec4::new(2.0, 1.0, 0.5, 0.0));
        assert_eq!(uniforms.flags[2], 0);
        material.emissive_vertex_color = true;
        assert_eq!(PbrMaterialUniforms::new(&material).flags[2], 1);
        assert!((uniforms.extra.y - 0.04).abs() < 1e-6);
        material.alpha_mode = MaterialAlphaMode::Blend;
        material.double_sided = true;
        assert_eq!(material_features(&material, true), PbrFeatures::NORMAL_MAP | PbrFeatures::ALPHA_BLEND | PbrFeatures::DOUBLE_SIDED);
    }

    #[test]
    fn morph_deltas_pack_target_major_with_only_the_channels_present() {
        let target = |p: f32| ModelMorphTarget { position_deltas: vec![Vec3::splat(p), Vec3::splat(p + 1.0)], normal_deltas: Vec::new(), tangent_deltas: vec![Vec3::X; 2] };
        let primitive = ModelPrimitive { index: 0, material: None, first_index: 0, index_count: 3, base_vertex: 0, vertex_count: 2, skin_base_vertex: None, bounds_min: Vec3::ZERO, bounds_max: Vec3::ONE, has_vertex_colors: false, has_tex_coord1: false, has_tangents: false, tangents_generated: false, normals_generated: false, morph_targets: vec![target(1.0), target(10.0)] };
        let mut deltas = vec![Vec4::ONE];
        assert_eq!(pack_morph(&primitive, &mut deltas), (1, 5, 2));
        assert_eq!(deltas.len(), 1 + 2 * 2 * 2);
        assert_eq!(deltas[1], Vec4::new(1.0, 1.0, 1.0, 0.0));
        assert_eq!(deltas[2], Vec4::new(1.0, 0.0, 0.0, 0.0));
        assert_eq!(deltas[5], Vec4::new(10.0, 10.0, 10.0, 0.0));
    }

    #[test]
    fn the_two_draws_of_a_blended_double_sided_part_cull_opposite_faces_mirrored_or_not() {
        let part = PbrFeatures::ALPHA_BLEND | PbrFeatures::DOUBLE_SIDED;
        assert_eq!(pipeline_culling(part), (CullPreset::None, wgpu::FrontFace::Ccw));
        assert_eq!(pipeline_culling(part | PbrFeatures::BACK_FACES), (CullPreset::Front, wgpu::FrontFace::Ccw));
        assert_eq!(pipeline_culling(part | PbrFeatures::FRONT_FACES), (CullPreset::Back, wgpu::FrontFace::Ccw));
        // Mirrored: clockwise triangles are the front, so culling "front" still drops the near side.
        assert_eq!(pipeline_culling(part | PbrFeatures::BACK_FACES | PbrFeatures::MIRRORED), (CullPreset::Front, wgpu::FrontFace::Cw));
        assert_eq!(pipeline_culling(part | PbrFeatures::FRONT_FACES | PbrFeatures::MIRRORED), (CullPreset::Back, wgpu::FrontFace::Cw));
        assert_eq!(pipeline_culling(PbrFeatures::NONE), (CullPreset::Back, wgpu::FrontFace::Ccw));
        assert_eq!(pipeline_culling(PbrFeatures::DOUBLE_SIDED | PbrFeatures::MIRRORED), (CullPreset::None, wgpu::FrontFace::Cw));
    }

    #[test]
    fn mirroring_and_encoding() {
        assert!(determinant3(&Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0))) < 0.0);
        assert!(needs_shader_encoding(wgpu::TextureFormat::Bgra8Unorm));
        assert!(!needs_shader_encoding(wgpu::TextureFormat::Bgra8UnormSrgb));
    }
}
