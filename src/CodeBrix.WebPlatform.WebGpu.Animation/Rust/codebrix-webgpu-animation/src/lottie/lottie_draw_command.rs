//! `LottieDrawCommand` and `LottieDrawList`: a document evaluated for one frame - a flat list of
//! the renderer's primitives in DRAWING order, each with its final transform. The drawing half
//! maps each command onto one renderer call (see the crate's Lottie integration notes).

use std::sync::Arc;

use kurbo::{Affine, BezPath, Rect};

use crate::lottie::lottie_blend_mode::LottieBlendMode;
use crate::lottie::lottie_paint::LottiePaint;
use crate::lottie::lottie_stroke::LottieStroke;

/// One drawing primitive. Every `PushGroup` and `PushClip` is closed by a `Pop`. Fills are
/// always non-zero (velato asks for nothing else).
#[derive(Clone, Debug, PartialEq)]
pub enum LottieDrawCommand {
    /// Starts a group drawn offscreen and composited with `blend` at `alpha`, clipped to `area`
    /// under `transform` (a repeater copy's fade, a track matte's two groups, a top-level layer's
    /// blend mode).
    PushGroup {
        /// The group's opacity, 0..1.
        alpha: f32,
        /// How the group is composited.
        blend: LottieBlendMode,
        /// The area the group is clipped to, in its own coordinates.
        area: Arc<BezPath>,
        /// `area` as a rectangle when it is one (a cheaper clip for the drawing half).
        area_rect: Option<Rect>,
        /// The transform of `area`.
        transform: Affine,
    },
    /// Starts a clip: what follows until the matching `Pop` shows only inside `path` under
    /// `transform` (non-zero). The composition's frame, a layer mask, an image's bounds.
    PushClip {
        /// The clip's geometry in its own coordinates.
        path: Arc<BezPath>,
        /// `path` as a rectangle when it is one.
        rect: Option<Rect>,
        /// The clip's transform.
        transform: Affine,
    },
    /// Fills a path (non-zero).
    Fill {
        /// The geometry in its own coordinates.
        path: Arc<BezPath>,
        /// The final transform (a gradient is in the path's own coordinates, under it too).
        transform: Affine,
        /// The paint.
        paint: LottiePaint,
        /// The opacity, 0..1 (1 for gradients: their opacity is in the stops).
        opacity: f32,
    },
    /// Strokes a path.
    Stroke {
        /// The geometry in its own coordinates.
        path: Arc<BezPath>,
        /// The final transform (the stroke is expanded in the path's own space).
        transform: Affine,
        /// Width, caps, joins, paint and opacity.
        stroke: LottieStroke,
    },
    /// "An image is wanted here": the image asset `asset_id` at `opacity`, placed by
    /// [`LottieDrawCommand::image_transform`] once the application has the decoded pixels (the
    /// document lists its assets: [`DrawingLottie::images`](crate::lottie::DrawingLottie::images)).
    /// velato has already pushed a clip of the asset's authored box around it when it has one.
    Image {
        /// The asset's `id`.
        asset_id: String,
        /// The asset's authored width and height, when it gives them.
        size: Option<(f64, f64)>,
        /// The final transform.
        transform: Affine,
        /// The opacity, 0..1.
        opacity: f32,
    },
    /// Ends the innermost `PushGroup` or `PushClip`.
    Pop,
}

impl LottieDrawCommand {
    /// For an `Image`, the transform that maps the decoded picture's pixels (`pixel_width` x
    /// `pixel_height`) to the output, as velato's own sink places them: one pixel per unit when
    /// the asset has no authored size, otherwise the picture fitted inside the authored box,
    /// centred, keeping its aspect ratio. None for other commands.
    pub fn image_transform(&self, pixel_width: u32, pixel_height: u32) -> Option<Affine> {
        let LottieDrawCommand::Image { size, transform, .. } = self else {
            return None;
        };
        let (pw, ph) = (pixel_width as f64, pixel_height as f64);
        match size {
            Some((w, h)) if *w > 0.0 && *h > 0.0 && pw > 0.0 && ph > 0.0 => {
                let scale = (w / pw).min(h / ph);
                Some(*transform * Affine::translate(((w - pw * scale) * 0.5, (h - ph * scale) * 0.5)) * Affine::scale(scale))
            }
            _ => Some(*transform),
        }
    }

    /// True for a fill, a stroke or an image.
    pub fn is_draw(&self) -> bool {
        matches!(self, LottieDrawCommand::Fill { .. } | LottieDrawCommand::Stroke { .. } | LottieDrawCommand::Image { .. })
    }
}

/// A document evaluated for one frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LottieDrawList {
    /// The commands in drawing order.
    pub commands: Vec<LottieDrawCommand>,
    /// The Lottie frame number the list was evaluated at.
    pub frame: f64,
    /// How many groups that held a single draw were folded into that draw (its opacity
    /// multiplied, its area kept as a clip only when the draw could reach outside it).
    pub folded_groups: usize,
    /// How many group or clip calls velato made that could not be represented (none are known;
    /// counted so that a future velato cannot drop them silently).
    pub unsupported: usize,
}

impl LottieDrawList {
    /// The number of fills, strokes and images.
    pub fn draw_count(&self) -> usize {
        self.commands.iter().filter(|c| c.is_draw()).count()
    }

    /// The number of groups (`PushGroup`).
    pub fn group_count(&self) -> usize {
        self.commands.iter().filter(|c| matches!(c, LottieDrawCommand::PushGroup { .. })).count()
    }

    /// The number of clips (`PushClip`).
    pub fn clip_count(&self) -> usize {
        self.commands.iter().filter(|c| matches!(c, LottieDrawCommand::PushClip { .. })).count()
    }

    /// The deepest nesting of groups and clips.
    pub fn max_depth(&self) -> usize {
        let mut depth = 0usize;
        let mut max = 0usize;
        for c in &self.commands {
            match c {
                LottieDrawCommand::PushGroup { .. } | LottieDrawCommand::PushClip { .. } => {
                    depth += 1;
                    max = max.max(depth);
                }
                LottieDrawCommand::Pop => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        max
    }
}
