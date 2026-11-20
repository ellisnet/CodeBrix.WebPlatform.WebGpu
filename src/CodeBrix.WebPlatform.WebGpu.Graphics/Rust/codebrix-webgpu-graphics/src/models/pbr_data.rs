//! The uniform and storage records of the PBR model shader and the CPU-only parts of the model
//! renderer (C#: the internal `PbrFrameUniforms`, `PbrMaterialUniforms`, `PbrInstance`,
//! `PbrDrawUniforms` and the packing of `PbrModelRenderPath`). Tested on the host.

use glam::{Mat3, Mat4, Vec3, Vec4};

use crate::drawing3d::{Camera, MaterialAlphaMode, SceneLighting};
use crate::models::{ModelMaterial, ModelMorphTarget, ModelPrimitive};
use crate::shaders::PbrFeatures;

/// The frame uniforms of the model shader (400 bytes; C#: `PbrFrameUniforms`).
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
}

pub(crate) const FRAME_UNIFORM_SIZE: u64 = 400;
const _: () = assert!(std::mem::size_of::<PbrFrameUniforms>() == FRAME_UNIFORM_SIZE as usize);
// SAFETY: #[repr(C)]; a Mat4 and 21 Vec4, all 16-byte multiples at 16-byte offsets, no padding
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
    /// transform as two rows.
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
            flags: [0, tex_coords, 0, 0],
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

/// One instance record of the instance storage buffer (80 bytes; C#: `PbrInstance`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PbrInstance {
    pub(crate) world: Mat4,
    pub(crate) node_base: u32,
    pub(crate) joint_base: u32,
    pub(crate) weight_base: u32,
    pub(crate) reserved: u32,
}

pub(crate) const INSTANCE_SIZE: u64 = 80;
const _: () = assert!(std::mem::size_of::<PbrInstance>() == INSTANCE_SIZE as usize);
// SAFETY: #[repr(C)]; a Mat4 then four u32: 80 bytes, a multiple of the Mat4's 16-byte alignment,
// no padding (size asserted above).
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

/// The determinant of a matrix's upper 3 x 3: negative = the transform mirrors (C#: `Determinant3`).
pub(crate) fn determinant3(matrix: &Mat4) -> f32 {
    Mat3::from_mat4(*matrix).determinant()
}

/// The runs of equal flags in `flags`: (start, end, flag) for each run (C#: one draw per run of
/// instances with the same winding).
pub(crate) fn runs(flags: impl Iterator<Item = bool>) -> Vec<(usize, usize, bool)> {
    let mut result: Vec<(usize, usize, bool)> = Vec::new();
    for (index, flag) in flags.enumerate() {
        match result.last_mut() {
            Some(last) if last.2 == flag => last.1 = index + 1,
            _ => result.push((index, index + 1, flag)),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec3, Vec4};

    use super::{determinant3, material_features, needs_shader_encoding, pack_morph, runs, PbrFrameUniforms, PbrMaterialUniforms};
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
    fn mirroring_and_encoding_and_runs() {
        assert!(determinant3(&Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0))) < 0.0);
        assert!(needs_shader_encoding(wgpu::TextureFormat::Bgra8Unorm));
        assert!(!needs_shader_encoding(wgpu::TextureFormat::Bgra8UnormSrgb));
        assert_eq!(runs([false, true, true, false].into_iter()), vec![(0, 1, false), (1, 3, true), (3, 4, false)]);
    }
}
