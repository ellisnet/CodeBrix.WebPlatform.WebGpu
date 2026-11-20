//! The drawing half of Lottie: a [`DrawingLottieShape`] as an element of a session
//! ([`DrawingSession::draw_lottie`]) and its draw list mapped command by command onto the crate's
//! renderer ([`VectorScene`]).

use std::collections::BTreeMap;
use std::sync::Arc;

use kurbo::{Affine, Rect};
use peniko::{Brush, ColorStops, Fill, Gradient};

use crate::drawing_session::DrawingSession;
use crate::geometry::RectangleF;
use crate::images::ImageSource;
use crate::lottie::drawing_lottie::DrawingLottie;
use crate::lottie::drawing_lottie_aspect::DrawingLottieAspect;
use crate::lottie::drawing_lottie_shape::DrawingLottieShape;
use crate::lottie::lottie_draw_command::{LottieDrawCommand, LottieDrawList};
use crate::lottie::lottie_paint::{LottieGradientStop, LottiePaint};
use crate::lottie::lottie_play_options::LottiePlayOptions;
use crate::models::ElementId;
use crate::rendering::VectorScene;
use crate::shapes::{DrawingShape, ShapeKind};

impl DrawingSession {
    /// A Lottie animation on the active layer: `lottie` fitted into `destination` (calibrated
    /// units) with `aspect`, playing as `options` say AT THE SESSION'S TIME (loop, ping-pong or
    /// hold, speed, a range of frames or a marker, a begin time and a start offset); returns its
    /// id. The document is shared, not copied. An unknown marker in the range, or a range without
    /// frames, is an error. Hand over the pictures of image layers afterwards with
    /// `session.find_element_mut(id).and_then(|e| e.as_lottie_mut())` and
    /// [`DrawingLottieShape::set_image`].
    pub fn draw_lottie(&mut self, lottie: Arc<DrawingLottie>, destination: RectangleF, aspect: DrawingLottieAspect, options: LottiePlayOptions) -> Result<ElementId, String> {
        let shape = DrawingLottieShape::new(lottie, destination, aspect, options)?;
        Ok(self.draw_shape(shape))
    }
}

impl From<DrawingLottieShape> for DrawingShape {
    fn from(shape: DrawingLottieShape) -> DrawingShape {
        DrawingShape::new(ShapeKind::Lottie(shape))
    }
}

impl DrawingShape {
    /// The Lottie animation, when this element is one.
    pub fn as_lottie(&self) -> Option<&DrawingLottieShape> {
        match self.kind() {
            ShapeKind::Lottie(l) => Some(l),
            _ => None,
        }
    }

    /// The Lottie animation, to change it (its play options, its pictures), when this element is
    /// one.
    pub fn as_lottie_mut(&mut self) -> Option<&mut DrawingLottieShape> {
        match self.kind_mut() {
            ShapeKind::Lottie(l) => Some(l),
            _ => None,
        }
    }
}

impl DrawingLottieShape {
    /// Records the shape at session `time` into a scene: `transform` takes calibrated units to
    /// the output, `opacity` (the element's) multiplies everything, as Lottie's own opacities do.
    pub(crate) fn encode(&self, scene: &mut VectorScene, transform: Affine, time: f64, opacity: f32) {
        let list = self.list_at(self.frame_at(time), transform, opacity);
        encode_list(scene, &list, self.images());
    }
}

/// Maps a draw list onto the renderer, one call per command: groups to `push_layer` with the
/// command's blend mode (mix modes, Plus for "add", SrcIn / SrcOut for track mattes), clips to
/// `push_clip_layer` (non-zero), fills (non-zero) and strokes with their brushes, images to
/// pictures when the application has handed over their pixels, pops to `pop_layer`.
pub(crate) fn encode_list(scene: &mut VectorScene, list: &LottieDrawList, images: &BTreeMap<String, ImageSource>) {
    for command in &list.commands {
        match command {
            LottieDrawCommand::PushGroup { alpha, blend, area, area_rect, transform } => match area_rect {
                Some(rect) => scene.push_layer(blend.to_peniko(), *alpha, *transform, rect),
                None => scene.push_layer(blend.to_peniko(), *alpha, *transform, &**area),
            },
            LottieDrawCommand::PushClip { path, rect, transform } => match rect {
                Some(rect) => scene.push_clip_layer(*transform, rect),
                None => scene.push_clip_layer(*transform, &**path),
            },
            LottieDrawCommand::Fill { path, transform, paint, opacity } => scene.fill(Fill::NonZero, *transform, &brush(paint, *opacity), None, &**path),
            LottieDrawCommand::Stroke { path, transform, stroke } => scene.stroke(&stroke.to_kurbo(), *transform, &brush(&stroke.paint, stroke.opacity), None, &**path),
            LottieDrawCommand::Image { asset_id, opacity, .. } => {
                if let Some(source) = images.get(asset_id) {
                    let (pw, ph) = source.size();
                    if let Some(place) = command.image_transform(pw, ph) {
                        scene.fill_picture(place, source.image_data(), Affine::IDENTITY, &Rect::new(0.0, 0.0, pw as f64, ph as f64), false, None, *opacity);
                    }
                }
            }
            LottieDrawCommand::Pop => scene.pop_layer(),
        }
    }
}

/// The renderer's brush for a paint at an opacity (gradients in the path's own coordinates, pad
/// at both ends, as velato hands them to vello).
fn brush(paint: &LottiePaint, opacity: f32) -> Brush {
    match paint {
        LottiePaint::Solid(color) => Brush::Solid(color.to_peniko(opacity)),
        LottiePaint::Linear(g) => Brush::Gradient(Gradient::new_linear(g.start, g.end).with_stops(stops(&g.stops, opacity))),
        LottiePaint::Radial(g) => Brush::Gradient(Gradient::new_radial(g.center, g.radius as f32).with_stops(stops(&g.stops, opacity))),
    }
}

fn stops(stops: &[LottieGradientStop], opacity: f32) -> ColorStops {
    let mut list = ColorStops::new();
    for stop in stops {
        list.0.push(peniko::ColorStop::from((stop.offset, stop.color.to_peniko(opacity))));
    }
    list
}
