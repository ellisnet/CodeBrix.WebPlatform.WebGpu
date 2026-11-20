//! The velato `RenderSink` that turns velato's calls into plain-data draw commands
//! (crate-internal). It also applies TOP-LEVEL layers' blend modes, which velato reads but never
//! applies: velato announces each layer it renders (`begin_layer_group` / `end_layer_group`), and
//! at nesting depth 0 the index it passes is the layer's index in the composition, so a layer
//! with a blend mode is wrapped in a group of that mode.

use std::sync::Arc;

use kurbo::{Affine, BezPath, Rect, Shape};
use peniko::color::Srgb;
use peniko::{BlendMode, Brush, GradientKind};
use velato::model::{fixed, ImageAsset};
use velato::RenderSink;

use crate::color::Color;
use crate::lottie::lottie_blend_mode::LottieBlendMode;
use crate::lottie::lottie_draw_command::LottieDrawCommand;
use crate::lottie::lottie_paint::{LottieGradientStop, LottieLinearGradient, LottiePaint, LottieRadialGradient};
use crate::lottie::lottie_stroke::{LottieLineCap, LottieLineJoin, LottieStroke};

/// Records velato's calls.
pub(crate) struct RecordingSink<'a> {
    pub(crate) commands: Vec<LottieDrawCommand>,
    pub(crate) unsupported: usize,
    layer_blends: &'a [Option<LottieBlendMode>],
    frame_area: Arc<BezPath>,
    frame_rect: Rect,
    root: Affine,
    depth: usize,
    blend_groups: Vec<bool>,
}

impl<'a> RecordingSink<'a> {
    /// A sink for a composition of `width` x `height` drawn under `root`.
    pub(crate) fn new(layer_blends: &'a [Option<LottieBlendMode>], width: f64, height: f64, root: Affine) -> RecordingSink<'a> {
        let frame_rect = Rect::new(0.0, 0.0, width, height);
        RecordingSink { commands: Vec::new(), unsupported: 0, layer_blends, frame_area: Arc::new(frame_rect.to_path(0.1)), frame_rect, root, depth: 0, blend_groups: Vec::new() }
    }
}

fn to_path(shape: &impl Shape) -> (Arc<BezPath>, Option<Rect>) {
    (Arc::new(BezPath::from_vec(shape.path_elements(0.1).collect())), shape.as_rect())
}

fn channel(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// A peniko brush as a paint and an opacity; None for brushes velato never makes (sweep
/// gradients, images).
pub(crate) fn to_paint(brush: &Brush) -> Option<(LottiePaint, f32)> {
    match brush {
        Brush::Solid(c) => {
            let [r, g, b, a] = c.components;
            Some((LottiePaint::Solid(Color::from_rgb(channel(r), channel(g), channel(b))), a.clamp(0.0, 1.0)))
        }
        Brush::Gradient(gradient) => {
            let stops: Vec<LottieGradientStop> = gradient
                .stops
                .iter()
                .map(|s| {
                    let [r, g, b, a] = s.color.to_alpha_color::<Srgb>().components;
                    LottieGradientStop { offset: s.offset, color: Color::from_rgba(channel(r), channel(g), channel(b), channel(a)) }
                })
                .collect();
            match gradient.kind {
                GradientKind::Linear(p) => Some((LottiePaint::Linear(Arc::new(LottieLinearGradient { start: (p.start.x, p.start.y), end: (p.end.x, p.end.y), stops })), 1.0)),
                GradientKind::Radial(p) => Some((LottiePaint::Radial(Arc::new(LottieRadialGradient { center: (p.end_center.x, p.end_center.y), radius: p.end_radius as f64, stops })), 1.0)),
                GradientKind::Sweep(_) => None,
            }
        }
        Brush::Image(_) => None,
    }
}

impl RenderSink for RecordingSink<'_> {
    fn push_layer(&mut self, blend: impl Into<BlendMode>, alpha: f32, transform: Affine, shape: &impl Shape) {
        let blend = match LottieBlendMode::from_peniko(blend.into()) {
            Some(b) => b,
            None => {
                self.unsupported += 1;
                LottieBlendMode::Normal
            }
        };
        let (area, area_rect) = to_path(shape);
        self.commands.push(LottieDrawCommand::PushGroup { alpha: alpha.clamp(0.0, 1.0), blend, area, area_rect, transform });
    }

    fn push_clip_layer(&mut self, transform: Affine, shape: &impl Shape) {
        let (path, rect) = to_path(shape);
        self.commands.push(LottieDrawCommand::PushClip { path, rect, transform });
    }

    fn pop_layer(&mut self) {
        self.commands.push(LottieDrawCommand::Pop);
    }

    fn draw(&mut self, stroke: Option<&fixed::Stroke>, transform: Affine, brush: &fixed::Brush, shape: &impl Shape) {
        let Some((paint, opacity)) = to_paint(brush) else {
            self.unsupported += 1;
            return;
        };
        let (path, _) = to_path(shape);
        match stroke {
            None => self.commands.push(LottieDrawCommand::Fill { path, transform, paint, opacity }),
            Some(s) => {
                let stroke = LottieStroke {
                    paint,
                    opacity,
                    width: s.width,
                    cap: match s.start_cap {
                        kurbo::Cap::Butt => LottieLineCap::Butt,
                        kurbo::Cap::Round => LottieLineCap::Round,
                        kurbo::Cap::Square => LottieLineCap::Square,
                    },
                    join: match s.join {
                        kurbo::Join::Miter => LottieLineJoin::Miter,
                        kurbo::Join::Round => LottieLineJoin::Round,
                        kurbo::Join::Bevel => LottieLineJoin::Bevel,
                    },
                    miter_limit: s.miter_limit,
                };
                self.commands.push(LottieDrawCommand::Stroke { path, transform, stroke });
            }
        }
    }

    fn draw_image(&mut self, image: &ImageAsset, transform: Affine, alpha: f64) {
        let opacity = if alpha.is_finite() { alpha.clamp(0.0, 1.0) as f32 } else { 0.0 };
        self.commands.push(LottieDrawCommand::Image { asset_id: image.id.clone(), size: image.width.zip(image.height), transform, opacity });
    }

    fn begin_layer_group(&mut self, _name: &str, index: usize) {
        let blend = if self.depth == 0 { self.layer_blends.get(index).copied().flatten() } else { None };
        match blend {
            Some(blend) if blend != LottieBlendMode::Normal => {
                self.commands.push(LottieDrawCommand::PushGroup { alpha: 1.0, blend, area: self.frame_area.clone(), area_rect: Some(self.frame_rect), transform: self.root });
                self.blend_groups.push(true);
            }
            _ => self.blend_groups.push(false),
        }
        self.depth += 1;
    }

    fn end_layer_group(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        if self.blend_groups.pop() == Some(true) {
            self.commands.push(LottieDrawCommand::Pop);
        }
    }
}
