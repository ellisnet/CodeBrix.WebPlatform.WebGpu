//! [`BuiltInShaders`], [`EnvironmentShaders`] and [`MipmapShader`]: the WGSL sources the library's
//! renderers use, compiled into the crate (C#: the string constants of `BuiltInShaders`,
//! `EnvironmentShaders` and `MipmapGenerator`), and [`BuiltInShaderSource`], the list of every one
//! of them for a page that compiles them all.

use crate::shaders::{PbrFeatures, PbrShader, ShadowDepthShader};

/// The sprite and lit-mesh shaders (C#: `BuiltInShaders`).
pub struct BuiltInShaders;

impl BuiltInShaders {
    /// The sprite batch's shader: a 2D transform, a texture and a tint (C#: `Sprite`).
    pub const SPRITE: &'static str = include_str!("wgsl/sprite.wgsl");
    /// The mesh renderer's Blinn-Phong shader (C#: `LitMesh`).
    pub const LIT_MESH: &'static str = include_str!("wgsl/lit_mesh.wgsl");
}

/// The image-based lighting, skybox and tone-mapping shaders (C#: `EnvironmentShaders`). The
/// baking shaders share a common part, put in front of each as in C#.
pub struct EnvironmentShaders;

impl EnvironmentShaders {
    /// The equirectangular panorama to cube face baker.
    pub const EQUIRECT: &'static str = concat!(include_str!("wgsl/environment_common.wgsl"), "\n", include_str!("wgsl/equirect.wgsl"));
    /// The procedural sky baker.
    pub const PROCEDURAL_SKY: &'static str = concat!(include_str!("wgsl/environment_common.wgsl"), "\n", include_str!("wgsl/procedural_sky.wgsl"));
    /// The diffuse irradiance baker.
    pub const IRRADIANCE: &'static str = concat!(include_str!("wgsl/environment_common.wgsl"), "\n", include_str!("wgsl/irradiance.wgsl"));
    /// The prefiltered specular baker.
    pub const PREFILTER: &'static str = concat!(include_str!("wgsl/environment_common.wgsl"), "\n", include_str!("wgsl/prefilter.wgsl"));
    /// The BRDF look-up table baker.
    pub const BRDF_LUT: &'static str = concat!(include_str!("wgsl/environment_common.wgsl"), "\n", include_str!("wgsl/brdf_lut.wgsl"));
    /// The skybox.
    pub const SKYBOX: &'static str = include_str!("wgsl/skybox.wgsl");
    /// The tone mapper (ACES fitted, Khronos PBR Neutral, Reinhard, ACES Narkowicz, clamp).
    pub const TONE_MAP: &'static str = include_str!("wgsl/tone_map.wgsl");
}

/// The mipmap generator's shader: a full-screen triangle sampling the level above (C#:
/// `MipmapGenerator.Wgsl`).
pub struct MipmapShader;

impl MipmapShader {
    /// The source.
    pub const SOURCE: &'static str = include_str!("wgsl/mipmap.wgsl");
}

/// One built-in WGSL source, by name: what a page lists when it compiles every built-in shader
/// once (the first step of a new browser or a new crate version, because each browser has its
/// own WGSL compiler).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltInShaderSource {
    /// The name ("sprite", "pbr SKINNED+UNLIT", ...).
    pub name: String,
    /// The WGSL.
    pub source: String,
}

impl BuiltInShaderSource {
    /// Every built-in source: the sprite, lit-mesh, mipmap and environment shaders, then the PBR
    /// shader's variants for `pbr_variants` (for example
    /// [`PbrFeatures::every_shader_variant`]), then the sixteen variants of the model renderer's
    /// shadow depth shader ([`ShadowDepthShader`], Not in C#).
    pub fn all(pbr_variants: impl IntoIterator<Item = PbrFeatures>) -> Vec<BuiltInShaderSource> {
        let named = [
            ("sprite", BuiltInShaders::SPRITE),
            ("lit mesh", BuiltInShaders::LIT_MESH),
            ("mipmap", MipmapShader::SOURCE),
            ("environment equirect", EnvironmentShaders::EQUIRECT),
            ("environment procedural sky", EnvironmentShaders::PROCEDURAL_SKY),
            ("environment irradiance", EnvironmentShaders::IRRADIANCE),
            ("environment prefilter", EnvironmentShaders::PREFILTER),
            ("environment BRDF LUT", EnvironmentShaders::BRDF_LUT),
            ("skybox", EnvironmentShaders::SKYBOX),
            ("tone map", EnvironmentShaders::TONE_MAP),
        ];
        let mut list: Vec<BuiltInShaderSource> = named.iter().map(|(name, source)| BuiltInShaderSource { name: (*name).to_string(), source: (*source).to_string() }).collect();
        list.extend(pbr_variants.into_iter().map(|features| BuiltInShaderSource { name: format!("pbr {features}"), source: PbrShader::source(features) }));
        list.extend(ShadowDepthShader::every_variant().map(|features| BuiltInShaderSource { name: format!("pbr shadow depth {features}"), source: ShadowDepthShader::source(features) }));
        list
    }
}

#[cfg(test)]
mod tests {
    use super::{BuiltInShaderSource, BuiltInShaders, EnvironmentShaders, MipmapShader};
    use crate::shaders::PbrFeatures;

    #[test]
    fn every_source_has_its_entry_points() {
        for source in [BuiltInShaders::SPRITE, BuiltInShaders::LIT_MESH, MipmapShader::SOURCE, EnvironmentShaders::SKYBOX, EnvironmentShaders::TONE_MAP] {
            assert!(source.contains("@vertex") && source.contains("@fragment"));
        }
    }

    #[test]
    fn the_bakers_start_with_the_common_part() {
        let common = include_str!("wgsl/environment_common.wgsl");
        for source in [EnvironmentShaders::EQUIRECT, EnvironmentShaders::PROCEDURAL_SKY, EnvironmentShaders::IRRADIANCE, EnvironmentShaders::PREFILTER, EnvironmentShaders::BRDF_LUT] {
            assert!(source.starts_with(common));
            assert!(source.len() > common.len() + 100);
        }
    }

    #[test]
    fn the_tone_map_has_the_three_named_curves() {
        for name in ["fn aces_fitted(", "fn aces_narkowicz(", "fn pbr_neutral("] {
            assert!(EnvironmentShaders::TONE_MAP.contains(name));
        }
    }

    #[test]
    fn all_lists_ten_sources_then_the_pbr_variants_then_the_shadow_depth_ones() {
        let list = BuiltInShaderSource::all(PbrFeatures::every_shader_variant());
        assert_eq!(list.len(), 10 + 4096 + 16);
        assert_eq!(list[0].name, "sprite");
        assert_eq!(list[10].name, "pbr NONE");
        assert_eq!(list[10 + 4096].name, "pbr shadow depth NONE");
        let mut names: Vec<&str> = list.iter().map(|s| s.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), list.len());
    }
}
