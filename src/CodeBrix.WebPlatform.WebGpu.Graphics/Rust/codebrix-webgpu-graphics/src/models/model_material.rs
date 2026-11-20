//! [`ModelMaterial`] and [`ModelTextureSlot`]: a model's glTF materials, captured completely as
//! metallic-roughness parameters (C#: the types of the same names). Immutable data.

use glam::{Affine2, Vec2, Vec3, Vec4};

use crate::drawing3d::MaterialAlphaMode;

/// One texture reference of a [`ModelMaterial`] (C#: `ModelTextureSlot`): the texture and its
/// sampler (as indices into the model's lists), the texture-coordinate set and the
/// KHR_texture_transform (identity when absent; its texCoord override is already applied to
/// [`ModelTextureSlot::tex_coord`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelTextureSlot {
    /// The texture's index in [`Model::textures`](crate::Model::textures), or None when the slot
    /// is empty or its image could not be decoded (C#: `Texture`, the object; read it with
    /// [`Model::slot_texture`](crate::Model::slot_texture)).
    pub texture: Option<usize>,
    /// The sampler's index in [`Model::samplers`](crate::Model::samplers) (C#: `Sampler`).
    pub sampler: Option<usize>,
    /// The texture-coordinate set, 0 or 1 (C#: `TexCoord`).
    pub tex_coord: u32,
    /// The texture-coordinate transform as C#'s `Matrix3x2` (M11, M12, M21, M22, M31, M32),
    /// identity when none (C#: `Transform`); see [`ModelTextureSlot::transform_affine`].
    pub transform: [f32; 6],
    /// True when the file gave a KHR_texture_transform (C#: `HasTransform`).
    pub has_transform: bool,
    /// The glTF texture index, or -1 for an empty slot (C#: `TextureIndex`).
    pub texture_index: i32,
}

impl ModelTextureSlot {
    /// The identity transform in `Matrix3x2` order.
    pub const IDENTITY_TRANSFORM: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

    /// The empty slot (C#: the internal `ModelTextureSlot.Empty`).
    pub const EMPTY: ModelTextureSlot = ModelTextureSlot { texture: None, sampler: None, tex_coord: 0, transform: ModelTextureSlot::IDENTITY_TRANSFORM, has_transform: false, texture_index: -1 };

    /// True when the slot names a texture (C#: `IsPresent`).
    pub fn is_present(&self) -> bool {
        self.texture_index >= 0
    }

    /// The transform as glam's `Affine2`: `transform_affine().transform_point2(uv)` is C#'s
    /// `Vector2.Transform(uv, Transform)`.
    pub fn transform_affine(&self) -> Affine2 {
        let t = self.transform;
        Affine2::from_cols(Vec2::new(t[0], t[1]), Vec2::new(t[2], t[3]), Vec2::new(t[4], t[5]))
    }
}

impl Default for ModelTextureSlot {
    fn default() -> ModelTextureSlot {
        ModelTextureSlot::EMPTY
    }
}

/// A glTF material, captured completely as metallic-roughness parameters (C#: `ModelMaterial`).
/// Renderers read what they support; [`ModelMaterial::approximated_features`] lists the
/// extensions that are captured but only approximated. The defaults are glTF's.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelMaterial {
    /// The index in the model's materials, or -1 for the glTF default material (C#: `Index`).
    pub index: i32,
    /// The name, or None (C#: `Name`).
    pub name: Option<String>,
    /// baseColorFactor, linear RGBA; default white (C#: `BaseColorFactor`).
    pub base_color_factor: Vec4,
    /// baseColorTexture, sRGB (C#: `BaseColorTexture`).
    pub base_color_texture: ModelTextureSlot,
    /// metallicFactor; default 1 (C#: `MetallicFactor`).
    pub metallic_factor: f32,
    /// roughnessFactor; default 1 (C#: `RoughnessFactor`).
    pub roughness_factor: f32,
    /// metallicRoughnessTexture, linear; roughness in G, metallic in B (C#: `MetallicRoughnessTexture`).
    pub metallic_roughness_texture: ModelTextureSlot,
    /// normalTexture, linear, tangent space (C#: `NormalTexture`).
    pub normal_texture: ModelTextureSlot,
    /// normalTexture.scale; default 1 (C#: `NormalScale`).
    pub normal_scale: f32,
    /// occlusionTexture, linear; occlusion in R (C#: `OcclusionTexture`).
    pub occlusion_texture: ModelTextureSlot,
    /// occlusionTexture.strength; default 1 (C#: `OcclusionStrength`).
    pub occlusion_strength: f32,
    /// emissiveFactor, linear RGB; default black (C#: `EmissiveFactor`).
    pub emissive_factor: Vec3,
    /// KHR_materials_emissive_strength; default 1 (C#: `EmissiveStrength`).
    pub emissive_strength: f32,
    /// emissiveTexture, sRGB (C#: `EmissiveTexture`).
    pub emissive_texture: ModelTextureSlot,
    /// alphaMode: Opaque, Mask or Blend; default Opaque (C#: `AlphaMode`).
    pub alpha_mode: MaterialAlphaMode,
    /// alphaCutoff, used by Mask; default 0.5 (C#: `AlphaCutoff`).
    pub alpha_cutoff: f32,
    /// doubleSided; default false (C#: `DoubleSided`).
    pub double_sided: bool,
    /// KHR_materials_unlit: the base colour is shown without lighting (C#: `Unlit`).
    pub unlit: bool,
    /// KHR_materials_ior; default 1.5 (C#: `IndexOfRefraction`).
    pub index_of_refraction: f32,
    /// KHR_materials_clearcoat factor, 0 = none; approximated (C#: `ClearCoatFactor`).
    pub clear_coat_factor: f32,
    /// KHR_materials_clearcoat roughness; approximated (C#: `ClearCoatRoughness`).
    pub clear_coat_roughness: f32,
    /// KHR_materials_specular factor; default 1; approximated (C#: `SpecularFactor`).
    pub specular_factor: f32,
    /// KHR_materials_specular colour; default white; approximated (C#: `SpecularColor`).
    pub specular_color: Vec3,
    /// KHR_materials_transmission factor, 0 = none; approximated (C#: `TransmissionFactor`).
    pub transmission_factor: f32,
    /// The extensions captured but only approximated by the renderers, for example
    /// "KHR_materials_clearcoat" (C#: `ApproximatedFeatures`).
    pub approximated_features: Vec<String>,
}

impl ModelMaterial {
    /// A material with glTF's defaults (C#: the internal constructor).
    pub fn new(index: i32, name: Option<String>) -> ModelMaterial {
        ModelMaterial {
            index,
            name,
            base_color_factor: Vec4::ONE,
            base_color_texture: ModelTextureSlot::EMPTY,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            metallic_roughness_texture: ModelTextureSlot::EMPTY,
            normal_texture: ModelTextureSlot::EMPTY,
            normal_scale: 1.0,
            occlusion_texture: ModelTextureSlot::EMPTY,
            occlusion_strength: 1.0,
            emissive_factor: Vec3::ZERO,
            emissive_strength: 1.0,
            emissive_texture: ModelTextureSlot::EMPTY,
            alpha_mode: MaterialAlphaMode::Opaque,
            alpha_cutoff: 0.5,
            double_sided: false,
            unlit: false,
            index_of_refraction: 1.5,
            clear_coat_factor: 0.0,
            clear_coat_roughness: 0.0,
            specular_factor: 1.0,
            specular_color: Vec3::ONE,
            transmission_factor: 0.0,
            approximated_features: Vec::new(),
        }
    }

    /// True when the base colour is alpha-blended (C#: `IsTranslucent`).
    pub fn is_translucent(&self) -> bool {
        self.alpha_mode == MaterialAlphaMode::Blend
    }

    /// The five texture slots in the PBR shader's order: base colour, metallic-roughness, normal,
    /// occlusion, emissive.
    pub fn slots(&self) -> [&ModelTextureSlot; 5] {
        [&self.base_color_texture, &self.metallic_roughness_texture, &self.normal_texture, &self.occlusion_texture, &self.emissive_texture]
    }
}

impl std::fmt::Display for ModelMaterial {
    /// The index and name, for example `ModelMaterial 2 'colormap'` (C#: `ToString`).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(name) => write!(f, "ModelMaterial {} '{}'", self.index, name),
            None => write!(f, "ModelMaterial {}", self.index),
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use super::{ModelMaterial, ModelTextureSlot};

    #[test]
    fn the_defaults_are_gltf_s() {
        let material = ModelMaterial::new(-1, Some("default".to_string()));
        assert_eq!(material.metallic_factor, 1.0);
        assert_eq!(material.index_of_refraction, 1.5);
        assert!(!material.base_color_texture.is_present());
        assert_eq!(material.to_string(), "ModelMaterial -1 'default'");
    }

    #[test]
    fn the_transform_is_csharp_s_matrix3x2() {
        let slot = ModelTextureSlot { transform: [2.0, 0.0, 0.0, 3.0, 0.5, 0.25], has_transform: true, ..ModelTextureSlot::EMPTY };
        assert_eq!(slot.transform_affine().transform_point2(Vec2::new(1.0, 1.0)), Vec2::new(2.5, 3.25));
    }
}
