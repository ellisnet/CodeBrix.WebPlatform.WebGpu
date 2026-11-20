//! `DrawingLottieShape`: a playing document fitted into a destination (the same shape as the
//! `svg` module's `DrawingSvgShape`).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use kurbo::{Affine, Rect, Shape};

use crate::geometry::RectangleF;
use crate::images::ImageSource;
use crate::lottie::drawing_lottie::DrawingLottie;
use crate::lottie::drawing_lottie_aspect::DrawingLottieAspect;
use crate::lottie::lottie_draw_command::{LottieDrawCommand, LottieDrawList};
use crate::lottie::lottie_play_options::LottiePlayOptions;

/// A document playing in a destination rectangle: [`draw_list`](DrawingLottieShape::draw_list)
/// at a session time gives what it draws then, in calibrated units. As an ELEMENT of a session
/// ([`DrawingSession::draw_lottie`](crate::DrawingSession::draw_lottie), or
/// `DrawingShape::from(shape)` on any layer) it is drawn by the crate's renderer at the session's
/// time, under the element's transform, opacity, time window and effects; the pictures of its
/// image layers are the DECODED pixels the application hands over per asset id
/// ([`set_image`](DrawingLottieShape::set_image)). A clone shares the document and the pictures
/// and has its own play options.
#[derive(Debug)]
pub struct DrawingLottieShape {
    lottie: Arc<DrawingLottie>,
    destination: RectangleF,
    aspect: DrawingLottieAspect,
    options: LottiePlayOptions,
    range: (f64, f64),
    /// velato's renderer: scratch storage only (its batch of paths), reused from frame to frame.
    renderer: RefCell<velato::Renderer>,
    /// The decoded pictures of the document's image assets, by asset id.
    images: BTreeMap<String, ImageSource>,
    /// The shape's opacity, 0..1 (default 1), multiplied into everything it draws.
    pub opacity: f32,
    /// Folds groups that hold one draw into that draw (default true; see
    /// [`LottieDrawList::folded_groups`]).
    pub fold_groups: bool,
}

impl DrawingLottieShape {
    /// A document fitted into `destination`, playing as `options` says. An unknown marker in
    /// the range, or a range without frames, is an error.
    pub fn new(lottie: Arc<DrawingLottie>, destination: RectangleF, aspect: DrawingLottieAspect, options: LottiePlayOptions) -> Result<DrawingLottieShape, String> {
        let range = lottie.resolve_range(&options.range)?;
        Ok(DrawingLottieShape { lottie, destination, aspect, options, range, renderer: RefCell::new(velato::Renderer::new()), images: BTreeMap::new(), opacity: 1.0, fold_groups: true })
    }

    /// The document.
    pub fn lottie(&self) -> &Arc<DrawingLottie> {
        &self.lottie
    }

    /// The destination.
    pub fn destination(&self) -> RectangleF {
        self.destination
    }

    /// The fit.
    pub fn aspect(&self) -> DrawingLottieAspect {
        self.aspect
    }

    /// The play options.
    pub fn play_options(&self) -> &LottiePlayOptions {
        &self.options
    }

    /// Changes the play options (to seek, change the speed or the range from a given moment:
    /// the application computes the new `begin_time` / `start_offset` from the session time it
    /// is at). An unknown marker or an empty range is an error and changes nothing.
    pub fn set_play_options(&mut self, options: LottiePlayOptions) -> Result<(), String> {
        self.range = self.lottie.resolve_range(&options.range)?;
        self.options = options;
        Ok(())
    }

    /// The frames that play, `(start, end)`, end exclusive.
    pub fn range(&self) -> (f64, f64) {
        self.range
    }

    /// The matrix from document coordinates to calibrated units.
    pub fn content_transform(&self) -> Affine {
        self.aspect.fit(self.lottie.width(), self.lottie.height(), self.destination)
    }

    /// Where the document's box lands.
    pub fn content_bounds(&self) -> RectangleF {
        let r = self.content_transform().transform_rect_bbox(Rect::new(0.0, 0.0, self.lottie.width(), self.lottie.height()));
        RectangleF::new(r.x0 as f32, r.y0 as f32, r.width() as f32, r.height() as f32)
    }

    /// The fitted box (Slice: the destination).
    pub fn get_bounds(&self) -> RectangleF {
        if self.aspect == DrawingLottieAspect::Slice {
            self.destination
        } else {
            self.content_bounds()
        }
    }

    /// The Lottie frame shown at session `time`.
    pub fn frame_at(&self, time: f64) -> f64 {
        self.options.frame_in(self.range.0, self.range.1, self.lottie.frame_rate(), time)
    }

    /// True when a `Hold` playback has played its range at session `time`.
    pub fn is_finished(&self, time: f64) -> bool {
        self.options.finished_in(self.range.0, self.range.1, self.lottie.frame_rate(), time)
    }

    /// The session time at which each marker inside the range is first reached (playing
    /// forwards from `begin_time`; markers outside the range, or a speed of 0 or less, give
    /// none) - the times to define as the clock's NAMED POINTS IN TIME
    /// (`CanvasClock::define_point(name, time)`), so that other layers can follow the
    /// animation's markers by name.
    pub fn marker_times(&self) -> Vec<(String, f64)> {
        self.lottie.markers().iter().filter_map(|m| self.options.time_of_frame(self.range.0, self.range.1, self.lottie.frame_rate(), m.frame).map(|t| (m.name.clone(), t))).collect()
    }

    /// What the shape draws at session `time`, in calibrated units.
    pub fn draw_list(&mut self, time: f64) -> LottieDrawList {
        let frame = self.frame_at(time);
        self.draw_list_at_frame(frame)
    }

    /// What the shape draws at Lottie `frame`, in calibrated units (with the Slice clip).
    pub fn draw_list_at_frame(&mut self, frame: f64) -> LottieDrawList {
        self.list_at(frame, Affine::IDENTITY, 1.0)
    }

    /// What the shape draws at Lottie `frame` under `outer` (calibrated units to the output) at
    /// `alpha` times the shape's own opacity - the list the element's drawing maps onto the
    /// renderer.
    pub(crate) fn list_at(&self, frame: f64, outer: Affine, alpha: f32) -> LottieDrawList {
        let transform = outer * self.content_transform();
        let alpha = (self.opacity.clamp(0.0, 1.0) * alpha.clamp(0.0, 1.0)) as f64;
        let mut list = self.lottie.draw_list_with(&mut self.renderer.borrow_mut(), frame, transform, alpha, self.fold_groups);
        if self.aspect == DrawingLottieAspect::Slice {
            let d = self.destination;
            let rect = Rect::new(d.x as f64, d.y as f64, d.right() as f64, d.bottom() as f64);
            list.commands.insert(0, LottieDrawCommand::PushClip { path: Arc::new(rect.to_path(0.1)), rect: Some(rect), transform: outer });
            list.commands.push(LottieDrawCommand::Pop);
        }
        list
    }

    /// Hands over the picture of the image asset `asset_id` (the document lists its assets:
    /// [`DrawingLottie::images`]) - any [`ImageSource`]: decoded pixels (the picture-pixels form
    /// from the backend, or pixels made in the frontend) or a PNG file read with
    /// [`ImageSource::from_png`]. Its image layers draw it, placed as velato's own renderer places
    /// them (fitted, centred, into the asset's authored box). Without a picture an image layer
    /// draws nothing. The picture is shared, not copied.
    pub fn set_image(&mut self, asset_id: &str, source: ImageSource) {
        self.images.insert(asset_id.to_string(), source);
    }

    /// The pixels handed over for an image asset.
    pub fn image(&self, asset_id: &str) -> Option<&ImageSource> {
        self.images.get(asset_id)
    }

    /// The decoded pictures handed over, by asset id.
    pub(crate) fn images(&self) -> &BTreeMap<String, ImageSource> {
        &self.images
    }
}

impl Clone for DrawingLottieShape {
    /// The same document, fit, play options, pictures and opacity, with renderer scratch storage
    /// of its own.
    fn clone(&self) -> DrawingLottieShape {
        DrawingLottieShape { lottie: self.lottie.clone(), destination: self.destination, aspect: self.aspect, options: self.options.clone(), range: self.range, renderer: RefCell::new(velato::Renderer::new()), images: self.images.clone(), opacity: self.opacity, fold_groups: self.fold_groups }
    }
}

impl PartialEq for DrawingLottieShape {
    /// The same document (the same shared `DrawingLottie`), destination, fit, play options,
    /// pictures, opacity and folding (velato's scratch storage is not compared).
    fn eq(&self, other: &DrawingLottieShape) -> bool {
        Arc::ptr_eq(&self.lottie, &other.lottie) && self.destination == other.destination && self.aspect == other.aspect && self.options == other.options && self.images == other.images && self.opacity == other.opacity && self.fold_groups == other.fold_groups
    }
}
