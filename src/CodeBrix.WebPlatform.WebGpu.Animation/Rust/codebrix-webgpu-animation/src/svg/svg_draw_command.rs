//! `SvgDrawCommand` and `SvgDrawList`: the walked scene for one time - a flat list of the
//! renderer's primitives in DRAWING order, each with its final transform. The drawing half maps
//! each command onto one `VectorScene` call (see the crate's SVG integration notes).

use std::sync::Arc;

use kurbo::{Affine, BezPath, Rect};

use crate::svg::svg_group_node::{SvgBlendMode, SvgMaskKind};
use crate::svg::svg_leaf_nodes::SvgImageSource;
use crate::svg::svg_paint::{SvgFillRule, SvgPaint};
use crate::svg::svg_stroke::SvgStroke;

/// One shape of a clip: geometry under its final transform with its rule.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgClipShape {
    /// The geometry in its own coordinates.
    pub path: Arc<BezPath>,
    /// Its final transform.
    pub transform: Affine,
    /// `clip-rule`.
    pub rule: SvgFillRule,
}

/// One drawing primitive. Every `PushGroup` and `PushClip` is closed by a `Pop`;
/// `MaskBegin`..`MaskEnd` sits inside the `PushGroup` it masks, after that group's content.
#[derive(Clone, Debug, PartialEq)]
pub enum SvgDrawCommand {
    /// Starts a group composited with `blend` at `alpha` (opacity, blend mode, isolation, a
    /// mask that follows, or a blur).
    PushGroup {
        /// The group's opacity, 0..1 (own x part override x file animation).
        alpha: f32,
        /// The blend mode.
        blend: SvgBlendMode,
        /// True when the group must be drawn offscreen (isolation, a mask or a blur).
        isolate: bool,
        /// A Gaussian blur's standard deviation, in the group's coordinates (scale it by the
        /// transform's scale for device pixels).
        blur_std_deviation: Option<f32>,
        /// The group's transform (for the blur's scale).
        transform: Affine,
    },
    /// Starts a clip: what follows until the matching `Pop` shows only inside the UNION of the
    /// shapes (one shape: a direct stencil clip; several: their union).
    PushClip {
        /// The shapes.
        shapes: Vec<SvgClipShape>,
    },
    /// Fills a path.
    Fill {
        /// The geometry in its own coordinates.
        path: Arc<BezPath>,
        /// The final transform.
        transform: Affine,
        /// The rule.
        rule: SvgFillRule,
        /// The paint (gradients are in the path's own coordinates, placed by their transform).
        paint: SvgPaint,
        /// The paint's opacity, 0..1 (`fill-opacity`).
        opacity: f32,
        /// False when the document asked for no anti-aliasing.
        anti_alias: bool,
    },
    /// Strokes a path.
    Stroke {
        /// The geometry in its own coordinates (already trimmed when a part trim applies).
        path: Arc<BezPath>,
        /// The final transform (the stroke is expanded in the path's own space).
        transform: Affine,
        /// Width, caps, joins, dashes, paint and opacity.
        stroke: SvgStroke,
        /// False when the document asked for no anti-aliasing.
        anti_alias: bool,
    },
    /// "An image is wanted here": the picture scaled to 0,0 .. size under `transform`.
    Image {
        /// Where the pixels come from.
        source: SvgImageSource,
        /// Width and height in the image's own coordinates.
        size: (f64, f64),
        /// The final transform.
        transform: Affine,
        /// Bilinear (true) or nearest sampling.
        smooth: bool,
    },
    /// Starts the content of the mask of the group being drawn: what follows until `MaskEnd`
    /// is the mask, applied to everything drawn since the group's `PushGroup`, inside `rect`.
    MaskBegin {
        /// Luminance or alpha.
        kind: SvgMaskKind,
        /// The mask region under `transform`.
        rect: Rect,
        /// The transform of the region.
        transform: Affine,
    },
    /// Ends a mask's content.
    MaskEnd,
    /// Ends the innermost `PushGroup` or `PushClip`.
    Pop,
}

/// A walked scene: the commands and where each part ended up.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SvgDrawList {
    /// The commands in drawing order.
    pub commands: Vec<SvgDrawCommand>,
    /// For each part (in [`SvgScene::parts`](crate::svg::SvgScene::parts) order) the
    /// axis-aligned box of what it drew, in the walk's output coordinates (None: nothing).
    pub part_bounds: Vec<Option<Rect>>,
}

impl SvgDrawList {
    /// The number of fills, strokes and images.
    pub fn draw_count(&self) -> usize {
        self.commands.iter().filter(|c| matches!(c, SvgDrawCommand::Fill { .. } | SvgDrawCommand::Stroke { .. } | SvgDrawCommand::Image { .. })).count()
    }
}
