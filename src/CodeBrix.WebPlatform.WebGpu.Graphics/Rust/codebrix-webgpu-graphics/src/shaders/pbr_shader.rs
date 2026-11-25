//! [`PbrShader`] and [`PbrFeatures`]: the model renderer's metallic-roughness shader, one source
//! specialised by WGSL `const` booleans put in front of it (C#: `PbrShader`, `PbrFeatures`).

use std::fmt;
use std::ops::{BitOr, BitOrAssign};

/// What a variant of the model shader compiles in - one shader module per combination (C#: the
/// `[Flags]` enum `PbrFeatures`). Combine with `|`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PbrFeatures(u32);

impl PbrFeatures {
    /// Nothing beyond the base metallic-roughness shading.
    pub const NONE: PbrFeatures = PbrFeatures(0);
    /// GPU skinning from the skin stream and the instance's joint palette.
    pub const SKINNED: PbrFeatures = PbrFeatures(1 << 0);
    /// Morph targets from the model's delta buffer and the instance's weights.
    pub const MORPHED: PbrFeatures = PbrFeatures(1 << 1);
    /// A normal map (tangent frame from TANGENT, else from screen-space derivatives).
    pub const NORMAL_MAP: PbrFeatures = PbrFeatures(1 << 2);
    /// KHR_materials_unlit: the base colour is the output.
    pub const UNLIT: PbrFeatures = PbrFeatures(1 << 3);
    /// Alpha mode MASK: discard below the cutoff, write alpha 1.
    pub const ALPHA_MASK: PbrFeatures = PbrFeatures(1 << 4);
    /// Alpha mode BLEND: keep alpha.
    pub const ALPHA_BLEND: PbrFeatures = PbrFeatures(1 << 5);
    /// doubleSided: back faces shade with flipped normals.
    pub const DOUBLE_SIDED: PbrFeatures = PbrFeatures(1 << 6);
    /// KHR_materials_clearcoat (approximated: a second GGX lobe on the geometric normal).
    pub const CLEAR_COAT: PbrFeatures = PbrFeatures(1 << 7);
    /// The target is a non-sRGB 8-bit format: the shader encodes linear to sRGB itself.
    pub const ENCODE_SRGB: PbrFeatures = PbrFeatures(1 << 8);
    /// The frame has local lights (point lights, spotlights): the shader loops over them. Without
    /// it the loop is not compiled in, so a frame without local lights costs nothing. Not in C#.
    pub const LOCAL_LIGHTS: PbrFeatures = PbrFeatures(1 << 9);
    /// The frame has shadow maps (the sun's, spotlights'): the shader tests against them for the
    /// instances that receive shadows. Without it nothing of the shadows is compiled in. Not in C#.
    pub const SHADOWS: PbrFeatures = PbrFeatures(1 << 10);
    /// A level-of-detail fade (Not in C#): the instance record's dither code (`flags[1]`) decides
    /// which pixels are kept - a fine 8 x 8 dot pattern, the two levels of a cross-fade
    /// complementary - and a BLENDED part multiplies its alpha by the coverage instead. Only the
    /// draws of fading levels use it, so nothing else pays for the discard.
    pub const DITHER_FADE: PbrFeatures = PbrFeatures(1 << 11);
    /// Pipeline-only (no shader change): the draw's model matrix mirrors, so clockwise triangles
    /// are the front faces (glTF 2.0 rule).
    pub const MIRRORED: PbrFeatures = PbrFeatures(1 << 12);
    /// Pipeline-only (no shader change): an opaque or alpha-tested part of an instance whose alpha
    /// is below 1, drawn with alpha blending and a read-only depth test like a blended part. Not
    /// in C#.
    pub const FADED: PbrFeatures = PbrFeatures(1 << 13);
    /// Pipeline-only (no shader change): the FIRST of the two draws of a part that is both blended
    /// and double-sided - only its back faces (front faces culled). Not in C#.
    pub const BACK_FACES: PbrFeatures = PbrFeatures(1 << 14);
    /// Pipeline-only (no shader change): the SECOND of the two draws of a part that is both
    /// blended and double-sided - only its front faces (back faces culled). Not in C#.
    pub const FRONT_FACES: PbrFeatures = PbrFeatures(1 << 15);
    /// Every pipeline-only feature: [`PbrFeatures::MIRRORED`], [`PbrFeatures::FADED`],
    /// [`PbrFeatures::BACK_FACES`] and [`PbrFeatures::FRONT_FACES`].
    pub const PIPELINE_ONLY: PbrFeatures = PbrFeatures((1 << 12) | (1 << 13) | (1 << 14) | (1 << 15));

    /// Every feature that changes the shader, with its WGSL constant's name, in the order the
    /// constants are written (bit i is entry i: the shader features are the low bits, the
    /// pipeline-only ones above them).
    pub const SHADER_FEATURES: [(&'static str, PbrFeatures); 12] = [
        ("SKINNED", PbrFeatures::SKINNED),
        ("MORPHED", PbrFeatures::MORPHED),
        ("NORMAL_MAP", PbrFeatures::NORMAL_MAP),
        ("UNLIT", PbrFeatures::UNLIT),
        ("ALPHA_MASK", PbrFeatures::ALPHA_MASK),
        ("ALPHA_BLEND", PbrFeatures::ALPHA_BLEND),
        ("DOUBLE_SIDED", PbrFeatures::DOUBLE_SIDED),
        ("CLEARCOAT", PbrFeatures::CLEAR_COAT),
        ("ENCODE_SRGB", PbrFeatures::ENCODE_SRGB),
        ("LOCAL_LIGHTS", PbrFeatures::LOCAL_LIGHTS),
        ("SHADOWS", PbrFeatures::SHADOWS),
        ("DITHER_FADE", PbrFeatures::DITHER_FADE),
    ];

    /// The features as bits.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// The features from bits (unknown bits are dropped).
    pub const fn from_bits_truncate(bits: u32) -> PbrFeatures {
        PbrFeatures(bits & 0xFFFF)
    }

    /// True when every feature of `other` is in this set.
    pub const fn contains(self, other: PbrFeatures) -> bool {
        self.0 & other.0 == other.0
    }

    /// This set without the features of `other`.
    pub const fn without(self, other: PbrFeatures) -> PbrFeatures {
        PbrFeatures(self.0 & !other.0)
    }

    /// Every combination of the shader features (2^12 of them), from none to all - the variants
    /// `PbrShader::source` can make.
    pub fn every_shader_variant() -> impl Iterator<Item = PbrFeatures> {
        (0u32..(1 << PbrFeatures::SHADER_FEATURES.len())).map(PbrFeatures)
    }
}

impl BitOr for PbrFeatures {
    type Output = PbrFeatures;

    fn bitor(self, other: PbrFeatures) -> PbrFeatures {
        PbrFeatures(self.0 | other.0)
    }
}

impl BitOrAssign for PbrFeatures {
    fn bitor_assign(&mut self, other: PbrFeatures) {
        self.0 |= other.0;
    }
}

impl fmt::Display for PbrFeatures {
    /// The names of the features, joined by `+`, or `NONE`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut names: Vec<&str> = PbrFeatures::SHADER_FEATURES.iter().filter(|(_, flag)| self.contains(*flag)).map(|(name, _)| *name).collect();
        if self.contains(PbrFeatures::MIRRORED) {
            names.push("MIRRORED");
        }
        if self.contains(PbrFeatures::FADED) {
            names.push("FADED");
        }
        if self.contains(PbrFeatures::BACK_FACES) {
            names.push("BACK_FACES");
        }
        if self.contains(PbrFeatures::FRONT_FACES) {
            names.push("FRONT_FACES");
        }
        if names.is_empty() {
            f.write_str("NONE")
        } else {
            f.write_str(&names.join("+"))
        }
    }
}

/// The model renderer's metallic-roughness shader (glTF 2.0 reference behaviour: Lambert diffuse,
/// GGX distribution, height-correlated Smith visibility, Schlick Fresnel) (C#: `PbrShader`).
/// Variants are specialised by WGSL `const`s put in front of one source ([`PbrShader::source`]).
pub struct PbrShader;

impl PbrShader {
    /// The shared body every variant follows.
    pub const BODY: &'static str = include_str!("wgsl/pbr_body.wgsl");

    /// The WGSL of one variant (C#: `Source(features)`): one `const NAME: bool = true|false;` line
    /// per shader feature, then the body. `MIRRORED`, `FADED`, `BACK_FACES` and `FRONT_FACES`
    /// change nothing here (pipeline only).
    pub fn source(features: PbrFeatures) -> String {
        let mut source = String::with_capacity(PbrShader::BODY.len() + 512);
        for (name, flag) in PbrFeatures::SHADER_FEATURES {
            source.push_str("const ");
            source.push_str(name);
            source.push_str(": bool = ");
            source.push_str(if features.contains(flag) { "true" } else { "false" });
            source.push_str(";\n");
        }
        source.push('\n');
        source.push_str(PbrShader::BODY);
        source
    }
}

#[cfg(test)]
mod tests {
    use super::{PbrFeatures, PbrShader};

    #[test]
    fn source_starts_with_one_constant_per_feature_then_the_body() {
        let source = PbrShader::source(PbrFeatures::SKINNED | PbrFeatures::CLEAR_COAT);
        let lines: Vec<&str> = source.lines().take(12).collect();
        assert_eq!(lines[0], "const SKINNED: bool = true;");
        assert_eq!(lines[1], "const MORPHED: bool = false;");
        assert_eq!(lines[7], "const CLEARCOAT: bool = true;");
        assert_eq!(lines[8], "const ENCODE_SRGB: bool = false;");
        assert_eq!(lines[9], "const LOCAL_LIGHTS: bool = false;");
        assert_eq!(lines[10], "const SHADOWS: bool = false;");
        assert_eq!(lines[11], "const DITHER_FADE: bool = false;");
        assert!(PbrShader::source(PbrFeatures::DITHER_FADE).contains("const DITHER_FADE: bool = true;"));
        assert!(PbrShader::source(PbrFeatures::LOCAL_LIGHTS | PbrFeatures::SHADOWS).starts_with("const SKINNED: bool = false;"));
        assert!(PbrShader::source(PbrFeatures::SHADOWS).contains("const SHADOWS: bool = true;"));
        assert!(source.ends_with(PbrShader::BODY));
    }

    #[test]
    fn the_pipeline_only_features_do_not_change_the_shader() {
        assert_eq!(PbrShader::source(PbrFeatures::MIRRORED | PbrFeatures::UNLIT), PbrShader::source(PbrFeatures::UNLIT));
        assert_eq!(PbrShader::source(PbrFeatures::FADED | PbrFeatures::UNLIT), PbrShader::source(PbrFeatures::UNLIT));
        assert_eq!((PbrFeatures::FADED | PbrFeatures::MIRRORED).to_string(), "MIRRORED+FADED");
        assert_eq!(PbrFeatures::from_bits_truncate(u32::MAX).without(PbrFeatures::FADED).bits(), 0xDFFF);
        assert_eq!(PbrFeatures::from_bits_truncate(u32::MAX).without(PbrFeatures::PIPELINE_ONLY).bits(), 0xFFF);
        for side in [PbrFeatures::BACK_FACES, PbrFeatures::FRONT_FACES] {
            assert_eq!(PbrShader::source(side | PbrFeatures::DOUBLE_SIDED | PbrFeatures::ALPHA_BLEND), PbrShader::source(PbrFeatures::DOUBLE_SIDED | PbrFeatures::ALPHA_BLEND));
        }
        assert_eq!((PbrFeatures::MIRRORED | PbrFeatures::BACK_FACES).to_string(), "MIRRORED+BACK_FACES");
        assert_eq!(PbrFeatures::FRONT_FACES.to_string(), "FRONT_FACES");
    }

    #[test]
    fn every_variant_is_listed_once_and_named() {
        let variants: Vec<PbrFeatures> = PbrFeatures::every_shader_variant().collect();
        assert_eq!(variants.len(), 4096);
        assert_eq!(variants[0].to_string(), "NONE");
        assert_eq!(variants[4095].to_string(), "SKINNED+MORPHED+NORMAL_MAP+UNLIT+ALPHA_MASK+ALPHA_BLEND+DOUBLE_SIDED+CLEARCOAT+ENCODE_SRGB+LOCAL_LIGHTS+SHADOWS+DITHER_FADE");
        assert!(!variants.iter().any(|v| v.contains(PbrFeatures::MIRRORED) || v.contains(PbrFeatures::BACK_FACES) || v.contains(PbrFeatures::FRONT_FACES)));
    }

    // The C# library's source-level guard (PbrShaderTests): no variant declares an array of
    // matrices (one mobile driver read such a storage buffer wrongly); matrices are read through
    // load_matrix.
    #[test]
    fn no_variant_declares_an_array_of_matrices() {
        for features in PbrFeatures::every_shader_variant() {
            let source = PbrShader::source(features);
            let compact: String = source.chars().filter(|c| !c.is_whitespace()).collect();
            assert!(!compact.contains("array<mat"), "variant {features} declares array<mat...>");
            assert!(source.contains("fn load_matrix("));
        }
    }
}
