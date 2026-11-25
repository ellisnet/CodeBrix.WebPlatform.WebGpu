//! [`ShadowDepthShader`]: the depth-only shader of the model renderer's shadow passes (Not in C#),
//! specialised like the PBR shader by WGSL `const` booleans put in front of one source.

use crate::shaders::PbrFeatures;

/// The depth-only shader the model renderer draws shadow casters with (Not in C#): the PBR
/// shader's vertex movement (skinning, morph targets' positions) from a light's view-projection,
/// for alpha-tested parts the cutout test, and for the parts of a fading level of detail
/// ([`PbrFeatures::DITHER_FADE`]) the same dither pattern as the PBR shader, in the shadow map's
/// texels, so the shadow fades with the object. It reads the PBR shader's material, instance and
/// draw records unchanged. Entry points `vs_shadow` and `fs_shadow` (the fragment stage is used
/// only by alpha-tested and fading parts).
pub struct ShadowDepthShader;

impl ShadowDepthShader {
    /// The shared body every variant follows.
    pub const BODY: &'static str = include_str!("wgsl/shadow_depth.wgsl");

    /// The [`PbrFeatures`] that change this shader, with their WGSL constants' names.
    pub const FEATURES: [(&'static str, PbrFeatures); 4] = [("SKINNED", PbrFeatures::SKINNED), ("MORPHED", PbrFeatures::MORPHED), ("ALPHA_MASK", PbrFeatures::ALPHA_MASK), ("DITHER_FADE", PbrFeatures::DITHER_FADE)];

    /// The features of a part's PBR features that this shader keeps (the others do not matter to
    /// depth).
    pub fn features_of(features: PbrFeatures) -> PbrFeatures {
        ShadowDepthShader::FEATURES.iter().filter(|(_, flag)| features.contains(*flag)).fold(PbrFeatures::NONE, |all, (_, flag)| all | *flag)
    }

    /// Every combination of [`ShadowDepthShader::FEATURES`] (sixteen).
    pub fn every_variant() -> impl Iterator<Item = PbrFeatures> {
        (0u32..(1 << ShadowDepthShader::FEATURES.len())).map(|bits| ShadowDepthShader::FEATURES.iter().enumerate().filter(|(i, _)| bits & (1 << i) != 0).fold(PbrFeatures::NONE, |all, (_, (_, flag))| all | *flag))
    }

    /// True when a variant has a fragment stage: alpha-tested parts (the cutout lets the light
    /// through) and fading parts (the dither pattern); the others are depth only.
    pub fn needs_fragment(features: PbrFeatures) -> bool {
        features.contains(PbrFeatures::ALPHA_MASK) || features.contains(PbrFeatures::DITHER_FADE)
    }

    /// The WGSL of one variant: one `const NAME: bool = ...;` line per feature of
    /// [`ShadowDepthShader::FEATURES`], then the body. Other features are ignored.
    pub fn source(features: PbrFeatures) -> String {
        let mut source = String::with_capacity(ShadowDepthShader::BODY.len() + 128);
        for (name, flag) in ShadowDepthShader::FEATURES {
            source.push_str(&format!("const {name}: bool = {};\n", features.contains(flag)));
        }
        source.push('\n');
        source.push_str(ShadowDepthShader::BODY);
        source
    }
}

#[cfg(test)]
mod tests {
    use super::ShadowDepthShader;
    use crate::shaders::PbrFeatures;

    #[test]
    fn sixteen_variants_from_four_features() {
        let variants: Vec<PbrFeatures> = ShadowDepthShader::every_variant().collect();
        assert_eq!(variants.len(), 16);
        assert_eq!(variants[0], PbrFeatures::NONE);
        assert_eq!(variants[7], PbrFeatures::SKINNED | PbrFeatures::MORPHED | PbrFeatures::ALPHA_MASK);
        assert_eq!(variants[15], PbrFeatures::SKINNED | PbrFeatures::MORPHED | PbrFeatures::ALPHA_MASK | PbrFeatures::DITHER_FADE);
        let all = PbrFeatures::from_bits_truncate(u32::MAX);
        assert_eq!(ShadowDepthShader::features_of(all), variants[15]);
        assert!(ShadowDepthShader::needs_fragment(PbrFeatures::ALPHA_MASK) && ShadowDepthShader::needs_fragment(PbrFeatures::DITHER_FADE) && !ShadowDepthShader::needs_fragment(PbrFeatures::SKINNED | PbrFeatures::MORPHED));
        assert_eq!(ShadowDepthShader::features_of(PbrFeatures::DOUBLE_SIDED | PbrFeatures::SHADOWS), PbrFeatures::NONE);
        let source = ShadowDepthShader::source(PbrFeatures::ALPHA_MASK | PbrFeatures::UNLIT);
        assert!(source.starts_with("const SKINNED: bool = false;\nconst MORPHED: bool = false;\nconst ALPHA_MASK: bool = true;\nconst DITHER_FADE: bool = false;\n"));
        assert!(source.contains("fn vs_shadow(") && source.contains("fn fs_shadow(") && !source.contains("array<mat"));
    }
}
