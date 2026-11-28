//! `AnimationLayer`: this crate's renderer as a LAYER of the codebrix-webgpu canvas host. Each
//! frame it asks the application for the frame's drawing (a [`VectorScene`], which a
//! [`DrawingSession`](crate::DrawingSession) records itself into) and draws it OVER what the
//! layers below drew, in the host's convention (linear, premultiplied, through the sRGB view).
//! It never clears the target, and what it draws never depends on a clock: the time is the
//! frame's (its CPU clock only MEASURES the frame for [`RenderStats`]). A layer made with
//! [`AnimationLayer::to_picture`] draws INTO a [`PictureTarget`] instead (Not in C#).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use codebrix_webgpu::{CanvasFrame, CanvasRenderer, CanvasTarget, FrameInfo, GpuContext, TimePoints};

use crate::drawing_session::DrawingSession;
use crate::rendering::core::{RenderStats, VectorRenderer, CACHE_BUDGET_BYTES};
use crate::rendering::cpu_clock;
use crate::rendering::picture_target::{self, PictureTarget};
use crate::rendering::vector_scene::VectorScene;

type PixelsCallback = Box<dyn FnOnce(Vec<u8>, u32, u32)>;

#[derive(Default)]
struct Shared {
    stats: Cell<RenderStats>,
    request: RefCell<Option<PixelsCallback>>,
    /// 0 = idle, 1 = copy encoded this frame (map at the next), 2 = mapping.
    readback_state: Cell<u8>,
    waiting: RefCell<Option<PixelsCallback>>,
    /// The GPU timing asked for (off by default).
    gpu_timing: Cell<bool>,
    /// True once the layer is prepared on a device with `TIMESTAMP_QUERY`.
    gpu_timing_supported: Cell<bool>,
    /// The geometry cache's budget asked for (None: the default).
    cache_budget: Cell<Option<usize>>,
    /// True: internal targets sized to the drawing (`set_content_sized_targets(true)`; off by
    /// default).
    content_sized: Cell<bool>,
}

/// A handle to a running [`AnimationLayer`] (clone it freely): the last frame's counters, the GPU
/// timing switch, the geometry cache's budget, the opt-in content-sized targets, and the readback
/// that the crate keeps FOR TESTING ONLY.
#[derive(Clone, Default)]
pub struct AnimationLayerControl {
    shared: Rc<Shared>,
}

impl AnimationLayerControl {
    /// The counters and the CPU timings of the last frame drawn.
    pub fn stats(&self) -> RenderStats {
        self.shared.stats.get()
    }

    /// Switches the GPU TIMING of the layer's work on or off (Not in C#; off by default): while on,
    /// [`RenderStats::gpu_ms`] of [`AnimationLayerControl::stats`] holds the GPU milliseconds of
    /// the layer's passes in a recent frame - a few frames late, as the GPU hands results back
    /// asynchronously (nothing waits for it). It needs the device feature `TIMESTAMP_QUERY`: ask
    /// for it with `PageGpuOptions::optional_features` of codebrix-webgpu
    /// (`set_page_gpu_options`); where the browser does not grant it the timing is a quiet no-op
    /// (`gpu_ms` stays None; see [`AnimationLayerControl::gpu_timing_supported`]). Off, the layer
    /// records its passes exactly as it does without timing.
    pub fn set_gpu_timing(&self, on: bool) {
        self.shared.gpu_timing.set(on);
    }

    /// True while the GPU timing is switched on (asked for, whether or not the device can).
    pub fn gpu_timing(&self) -> bool {
        self.shared.gpu_timing.get()
    }

    /// True when the layer's device can time its work (it has `TIMESTAMP_QUERY`); false until the
    /// layer is prepared.
    pub fn gpu_timing_supported(&self) -> bool {
        self.shared.gpu_timing_supported.get()
    }

    /// Sets the byte budget of the layer's GEOMETRY CACHE (Not in C#; default 32 MiB, 33,554,432
    /// bytes), from the next frame on. The cache keeps the flattened outlines and stroked fans of
    /// the shapes drawn (8 bytes a point, and about 100 bytes a fan), so that a shape that moves,
    /// turns or scales a little - or comes back after being hidden - is not flattened again.
    /// Above the budget the fans used least recently are dropped, down to three quarters of it,
    /// never one the frame just drew (a frame that needs more than the budget keeps all it drew).
    /// A SMALLER budget holds less memory but flattens again what it dropped when it is drawn
    /// again: CPU time each time a scene with many shapes, much text or many scale steps comes
    /// back to what it showed before (0 keeps only what the last frame drew). A LARGER one keeps
    /// more of a long, varied scene ready, at the cost of the memory. Nothing drawn changes.
    pub fn set_geometry_cache_budget(&self, bytes: usize) {
        self.shared.cache_budget.set(Some(bytes));
    }

    /// The geometry cache's byte budget (32 MiB unless set).
    pub fn geometry_cache_budget(&self) -> usize {
        self.shared.cache_budget.get().unwrap_or(CACHE_BUDGET_BYTES)
    }

    /// OPT-IN (Not in C#; off by default): sizes the layer's INTERNAL TARGETS to the frame's
    /// drawing (true) instead of the canvas (false, the default: the layer draws exactly as it
    /// always has), from the next frame on. The targets are the renderer's own: a 4-sample colour
    /// target and stencil per nesting level of groups (20 bytes a pixel each), the mix modes'
    /// backdrop copy, the effect textures, a finished layer's height record and the stroke
    /// checkpoints. USE IT for a small drawing on a large canvas: memory and fill work then go in
    /// proportion to the drawing (its box, grown by its effects' reach, rounded out to 16 pixels
    /// with a little slack). THE TARGETS follow the drawing: kept while it stays inside them, moved
    /// with it, made again only when its size needs it (at least half as large again), and made
    /// smaller once it has used a quarter of them or less for 120 frames in a row. WHAT IT COSTS:
    /// a drawing that fills the canvas draws exactly as with canvas-sized targets, but elsewhere
    /// pixels on stroke-set edges and filtered water samples can differ slightly (by float
    /// rounding) from the canvas-sized rendering; stroke checkpoints are kept per target place, so
    /// a checkpointed painting is drawn whole once more when the targets move or change size.
    pub fn set_content_sized_targets(&self, on: bool) {
        self.shared.content_sized.set(on);
    }

    /// True while the internal targets are sized to the drawing (false by default).
    pub fn content_sized_targets(&self) -> bool {
        self.shared.content_sized.get()
    }

    /// FOR TESTING ONLY (not an application feature): copies the layer's OWN pixels (what it
    /// drew, without the layers below) of the next frame and hands them to `done` a frame or
    /// two later as straight-alpha RGBA rows, with the width and height. Asynchronous: a
    /// browser cannot wait for the GPU. A request made while one is pending replaces it.
    pub fn read_pixels_for_testing(&self, done: impl FnOnce(Vec<u8>, u32, u32) + 'static) {
        *self.shared.request.borrow_mut() = Some(Box::new(done));
    }
}

/// What the drawing closure of an [`AnimationLayer`] is handed each frame: the frame's timing
/// and size from the host's ONE clock, and the clock's named points in time.
pub struct AnimationFrame<'a> {
    /// The frame's time, delta, number and size (device pixels).
    pub info: FrameInfo,
    /// The clock's named points, as they were when the frame began.
    pub points: &'a TimePoints,
}

/// The renderer as a canvas layer, drawing whatever its closure records each frame.
pub struct AnimationLayer {
    renderer: VectorRenderer,
    scene: VectorScene,
    draw: Box<dyn FnMut(&AnimationFrame<'_>, &mut VectorScene)>,
    control: AnimationLayerControl,
    /// The picture this layer draws into instead of the canvas (None: the canvas).
    picture: Option<PictureTarget>,
}

impl AnimationLayer {
    /// A layer whose closure records each frame's drawing into the (emptied) scene. Put it on
    /// the canvas with `RendererHandle::new(layer)`; keep [`AnimationLayer::control`] first.
    pub fn new(draw: impl FnMut(&AnimationFrame<'_>, &mut VectorScene) + 'static) -> AnimationLayer {
        AnimationLayer { renderer: VectorRenderer::default(), scene: VectorScene::new(), draw: Box::new(draw), control: AnimationLayerControl::default(), picture: None }
    }

    /// A layer that draws its closure's scene INTO a picture target instead of onto the canvas
    /// (it draws nothing on the canvas; Not in C#): every frame the picture is cleared to
    /// transparent and the scene drawn into it, at the picture's size - the closure's
    /// `frame.info.width` and `height` ARE the picture's, so `session.render(scene,
    /// frame.info.width, frame.info.height)` fits a session into it. Put it BELOW the layers that
    /// read the picture in `CanvasLayers` (all layers share one encoder, so they read this frame's
    /// picture; listed above them they read the previous frame's). It has its own renderer and
    /// buffers (a second drawing's whole cost: record, build, encode, its multisampled targets at
    /// the picture's size - or the drawing's, opted in with
    /// [`AnimationLayerControl::set_content_sized_targets`]).
    pub fn to_picture(target: PictureTarget, draw: impl FnMut(&AnimationFrame<'_>, &mut VectorScene) + 'static) -> AnimationLayer {
        AnimationLayer { picture: Some(target), ..AnimationLayer::new(draw) }
    }

    /// The picture target the layer draws into (None: it draws onto the canvas).
    pub fn picture(&self) -> Option<&PictureTarget> {
        self.picture.as_ref()
    }

    /// A layer that draws a shared session every frame, as it is (set its time and change its
    /// layers from the page or another layer; the session never reads a clock), WITHOUT its
    /// background (the default of [`DrawingSession::render`]): it floats over the layers below,
    /// or over the page on a transparent canvas. Use [`AnimationLayer::new`] with
    /// `render_with_background` to draw the background too.
    pub fn for_session(session: Rc<RefCell<DrawingSession>>) -> AnimationLayer {
        AnimationLayer::new(move |frame, scene| session.borrow().render(scene, frame.info.width, frame.info.height))
    }

    /// The layer's control handle (counters, settings, testing readback).
    pub fn control(&self) -> AnimationLayerControl {
        self.control.clone()
    }

    /// The settings of the control handle, passed down to the renderer before a frame.
    fn apply_settings(&mut self) {
        let shared = &self.control.shared;
        self.renderer.set_cache_budget(shared.cache_budget.get().unwrap_or(CACHE_BUDGET_BYTES));
        self.renderer.set_content_sized_targets(shared.content_sized.get());
    }
}

impl CanvasRenderer for AnimationLayer {
    fn prepare(&mut self, context: &GpuContext, target: &CanvasTarget) {
        self.renderer.prepare(context, target);
        let supported = self.renderer.timing().is_some_and(|t| t.supported());
        self.control.shared.gpu_timing_supported.set(supported);
    }

    fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
        // A picture target's layer is sized by its picture, not the canvas.
        if self.picture.is_none() {
            self.renderer.resize(width, height);
        }
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let shared = self.control.shared.clone();
        // The canvas frame being drawn, for picture-target readers; a picture layer's picture is
        // this frame's from here on.
        picture_target::set_canvas_frame(frame.info.number);
        if let Some(p) = &self.picture {
            p.mark_drawn(frame.info.number);
        }
        // The copy made last frame has been submitted: map it now.
        if shared.readback_state.get() == 1 {
            if let Some(done) = shared.waiting.borrow_mut().take() {
                shared.readback_state.set(2);
                let state = shared.clone();
                self.renderer.map_readback(Box::new(move |pixels, w, h| {
                    state.readback_state.set(0);
                    done(pixels, w, h);
                }));
            } else {
                shared.readback_state.set(0);
            }
        }
        let read = shared.readback_state.get() == 0 && shared.request.borrow().is_some();
        if let Some(timing) = self.renderer.timing() {
            timing.set_enabled(shared.gpu_timing.get());
        }
        self.apply_settings();
        self.scene.reset();
        let record_start = cpu_clock::now_ms();
        {
            let mut info = frame.info;
            if let Some(p) = &self.picture {
                (info.width, info.height) = p.size();
            }
            let animation_frame = AnimationFrame { info, points: frame.points };
            (self.draw)(&animation_frame, &mut self.scene);
        }
        let record_ms = cpu_clock::since_ms(record_start);
        let mut stats = self.renderer.render(frame, &self.scene, read, self.picture.as_ref());
        stats.record_ms = record_ms;
        if read {
            *shared.waiting.borrow_mut() = shared.request.borrow_mut().take();
            shared.readback_state.set(1);
        }
        shared.stats.set(stats);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_control_handles_settings_have_todays_defaults_and_reach_the_renderer() {
        //Arrange
        let mut layer = AnimationLayer::new(|_, _| {});
        let control = layer.control();
        //Act + Assert: the defaults - today's: the 32 MiB budget, canvas-sized targets.
        assert_eq!(control.geometry_cache_budget(), 32 << 20);
        assert!(!control.content_sized_targets());
        layer.apply_settings();
        assert_eq!(layer.renderer.cache_budget(), 32 << 20);
        assert!(!layer.renderer.content_sized_targets());
        // Set through a clone of the handle: read back, and passed down before the next frame.
        let other = control.clone();
        other.set_geometry_cache_budget(4 << 20);
        other.set_content_sized_targets(true);
        assert_eq!(control.geometry_cache_budget(), 4 << 20);
        assert!(control.content_sized_targets());
        layer.apply_settings();
        assert_eq!(layer.renderer.cache_budget(), 4 << 20);
        assert!(layer.renderer.content_sized_targets());
        // And off again.
        other.set_content_sized_targets(false);
        layer.apply_settings();
        assert!(!layer.renderer.content_sized_targets());
    }
}
