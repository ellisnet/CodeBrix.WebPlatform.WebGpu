//! [`BlendPreset`], [`DepthPreset`] and [`CullPreset`]: the usual blend, depth and culling
//! settings by name (C#: the enums of the same names and the internal `Presets`).

/// How a colour target blends a fragment with what is already there (C#: `BlendPreset`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BlendPreset {
    /// No blending: the fragment replaces the pixel.
    #[default]
    Opaque,
    /// Straight (non-premultiplied) alpha: `src * a + dst * (1 - a)`.
    AlphaBlend,
    /// Premultiplied alpha: `src + dst * (1 - a)` - what a canvas layer uses (see
    /// `codebrix_webgpu::CanvasTarget`).
    PremultipliedAlpha,
    /// Additive: `src * a + dst` (glows, particles).
    Additive,
    /// Multiply: `src * dst`.
    Multiply,
}

impl BlendPreset {
    /// The `wgpu` blend state (None for `Opaque`).
    pub fn to_wgpu(self) -> Option<wgpu::BlendState> {
        use wgpu::{BlendComponent as C, BlendFactor as F, BlendOperation as O};
        let state = |color: C, alpha: C| Some(wgpu::BlendState { color, alpha });
        match self {
            BlendPreset::Opaque => None,
            BlendPreset::AlphaBlend => state(C { src_factor: F::SrcAlpha, dst_factor: F::OneMinusSrcAlpha, operation: O::Add }, C { src_factor: F::One, dst_factor: F::OneMinusSrcAlpha, operation: O::Add }),
            BlendPreset::PremultipliedAlpha => state(C { src_factor: F::One, dst_factor: F::OneMinusSrcAlpha, operation: O::Add }, C { src_factor: F::One, dst_factor: F::OneMinusSrcAlpha, operation: O::Add }),
            BlendPreset::Additive => state(C { src_factor: F::SrcAlpha, dst_factor: F::One, operation: O::Add }, C { src_factor: F::One, dst_factor: F::One, operation: O::Add }),
            BlendPreset::Multiply => state(C { src_factor: F::Dst, dst_factor: F::Zero, operation: O::Add }, C { src_factor: F::DstAlpha, dst_factor: F::Zero, operation: O::Add }),
        }
    }
}

/// How a pipeline tests and writes depth (C#: `DepthPreset`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DepthPreset {
    /// No depth test; the pipeline still declares the pass's depth format (compare Always, no
    /// writes).
    None,
    /// Nearer fragments win (`Less`) and write depth - the usual 3D setting.
    #[default]
    ReadWrite,
    /// Like `ReadWrite` with `LessEqual` (for re-drawing the same geometry).
    ReadWriteLessEqual,
    /// Test against depth (`LessEqual`) without writing (transparent geometry after opaque).
    ReadOnly,
}

impl DepthPreset {
    /// The compare function and whether depth is written.
    pub fn to_wgpu(self) -> (wgpu::CompareFunction, bool) {
        match self {
            DepthPreset::ReadWrite => (wgpu::CompareFunction::Less, true),
            DepthPreset::ReadWriteLessEqual => (wgpu::CompareFunction::LessEqual, true),
            DepthPreset::ReadOnly => (wgpu::CompareFunction::LessEqual, false),
            DepthPreset::None => (wgpu::CompareFunction::Always, false),
        }
    }
}

/// Which triangles a pipeline discards (C#: `CullPreset`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CullPreset {
    /// Draw both faces.
    #[default]
    None,
    /// Discard back faces (counter-clockwise triangles are front faces) - the usual setting for
    /// closed meshes.
    Back,
    /// Discard front faces.
    Front,
}

impl CullPreset {
    /// The `wgpu` cull mode.
    pub fn to_wgpu(self) -> Option<wgpu::Face> {
        match self {
            CullPreset::None => None,
            CullPreset::Back => Some(wgpu::Face::Back),
            CullPreset::Front => Some(wgpu::Face::Front),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BlendPreset, CullPreset, DepthPreset};

    #[test]
    fn premultiplied_alpha_is_wgpus_premultiplied_blending() {
        assert_eq!(BlendPreset::PremultipliedAlpha.to_wgpu(), Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING));
        assert_eq!(BlendPreset::Opaque.to_wgpu(), None);
    }

    #[test]
    fn depth_and_cull_presets() {
        assert_eq!(DepthPreset::ReadOnly.to_wgpu(), (wgpu::CompareFunction::LessEqual, false));
        assert_eq!(DepthPreset::None.to_wgpu(), (wgpu::CompareFunction::Always, false));
        assert_eq!(CullPreset::Back.to_wgpu(), Some(wgpu::Face::Back));
    }
}
