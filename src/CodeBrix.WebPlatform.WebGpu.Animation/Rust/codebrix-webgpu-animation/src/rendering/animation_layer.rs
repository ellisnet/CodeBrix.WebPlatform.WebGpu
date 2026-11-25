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
use crate::rendering::core::{RenderStats, VectorRenderer};
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
}

/// A handle to a running [`AnimationLayer`] (clone it freely): the last frame's counters, and
/// the readback that the crate keeps FOR TESTING ONLY.
#[derive(Clone, Default)]
pub struct AnimationLayerControl {
    shared: Rc<Shared>,
}

impl AnimationLayerControl {
    /// The counters and the CPU timings of the last frame drawn.
    pub fn stats(&self) -> RenderStats {
        self.shared.stats.get()
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
    /// the picture's size).
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

    /// The layer's control handle (counters, testing readback).
    pub fn control(&self) -> AnimationLayerControl {
        self.control.clone()
    }
}

impl CanvasRenderer for AnimationLayer {
    fn prepare(&mut self, context: &GpuContext, target: &CanvasTarget) {
        self.renderer.prepare(context, target);
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
