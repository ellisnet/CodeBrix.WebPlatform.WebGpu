//! `SvgGroupNode`: a group of a scene with its opacity, blend mode, clip, mask and blur.

use kurbo::{Affine, Rect};

/// How a group's content is put down over what is below (SVG `mix-blend-mode`; the names of the
/// W3C compositing modes, the same set peniko's `Mix` has).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SvgBlendMode {
    /// `normal`.
    #[default]
    Normal,
    /// `multiply`.
    Multiply,
    /// `screen`.
    Screen,
    /// `overlay`.
    Overlay,
    /// `darken`.
    Darken,
    /// `lighten`.
    Lighten,
    /// `color-dodge`.
    ColorDodge,
    /// `color-burn`.
    ColorBurn,
    /// `hard-light`.
    HardLight,
    /// `soft-light`.
    SoftLight,
    /// `difference`.
    Difference,
    /// `exclusion`.
    Exclusion,
    /// `hue`.
    Hue,
    /// `saturation`.
    Saturation,
    /// `color`.
    Color,
    /// `luminosity`.
    Luminosity,
    /// `plus-lighter` (peniko's compose mode `Plus`).
    PlusLighter,
}

/// How a mask's content becomes coverage (`mask-type`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SvgMaskKind {
    /// `luminance` (the SVG default): the content's luminance times its alpha.
    #[default]
    Luminance,
    /// `alpha`: the content's alpha.
    Alpha,
}

/// A clip path: its content (a detached group subtree of the arena) under `transform`, itself
/// possibly clipped again.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgClip {
    /// The clip path's id in the document.
    pub id: String,
    /// The clip path's `transform` (objectBoundingBox units already folded in).
    pub transform: Affine,
    /// The arena index of the group that holds the clip shapes.
    pub content: usize,
    /// A clip path on the clip path.
    pub clip: Option<Box<SvgClip>>,
}

/// A mask: its region, kind and content (a detached group subtree of the arena).
#[derive(Clone, Debug, PartialEq)]
pub struct SvgMask {
    /// The mask's id in the document.
    pub id: String,
    /// The mask region in the masked group's coordinates; nothing outside it shows.
    pub rect: Rect,
    /// Luminance or alpha.
    pub kind: SvgMaskKind,
    /// The arena index of the group that holds the mask content.
    pub content: usize,
    /// A mask on the mask.
    pub mask: Option<Box<SvgMask>>,
}

/// A group (an SVG `g`, `use`, nested `svg`, or a group usvg made for an element's transform,
/// opacity, clip, mask or filter).
#[derive(Clone, Debug, PartialEq)]
pub struct SvgGroupNode {
    /// `opacity`, 0..1.
    pub opacity: f32,
    /// `mix-blend-mode`.
    pub blend: SvgBlendMode,
    /// `isolation: isolate`.
    pub isolate: bool,
    /// `clip-path`.
    pub clip: Option<SvgClip>,
    /// `mask`.
    pub mask: Option<SvgMask>,
    /// The blur when the group's filter is exactly ONE `feGaussianBlur` of the source graphic with
    /// one standard deviation (the renderer's blur effect can draw it); None otherwise.
    pub blur_std_deviation: Option<f32>,
    /// True when the group has a filter that is not drawn (anything beyond the one blur above):
    /// the content is drawn without the filter and the report says so.
    pub has_undrawn_filter: bool,
    /// The children, arena indices in paint order.
    pub children: Vec<usize>,
}
