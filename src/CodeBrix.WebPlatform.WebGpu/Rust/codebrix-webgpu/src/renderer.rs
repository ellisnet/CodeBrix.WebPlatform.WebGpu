//! What a page plugs into the canvas host: a [`CanvasRenderer`] that draws each frame with the
//! `wgpu` device directly, the handle the host component takes it by, and what the host hands it
//! (the frame, its timing, and the reason a canvas could not become a surface).

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::context::GpuContext;
use crate::input::CanvasInput;
use crate::time_points::TimePoints;

/// The timing and size of one frame. The time comes from ONE clock, the canvas's
/// [`CanvasClock`](crate::CanvasClock), derived from the browser's animation-frame timestamps
/// and scaled, paused or moved by that clock: every layer of the canvas sees the same values in
/// the same frame, so layers drawn by different crates stay in step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameInfo {
    /// The frame's number on the clock, from 1 (frames are counted while the clock is paused).
    pub number: u64,
    /// The clock's time, in seconds (0 at the first frame, unless the clock was moved).
    pub time: f64,
    /// Clock seconds since the previous frame (0 for the first, while paused, and for the frame
    /// after a resume or a seek).
    pub delta: f64,
    /// The clock's time at the previous frame (None for the first frame).
    pub previous_time: Option<f64>,
    /// The surface's width in device pixels.
    pub width: u32,
    /// The surface's height in device pixels.
    pub height: u32,
    /// Device pixels per CSS pixel (the browser's `devicePixelRatio`).
    pub pixel_ratio: f64,
}

/// What every renderer of a canvas draws into, decided in ONE place - the canvas host - so that
/// two renderers (two crates) can never disagree about colour encoding or alpha:
///
/// - `format` is the format of the frame's view and of every render pipeline that draws into
///   it: the sRGB form of the browser's preferred canvas format (`Bgra8UnormSrgb` or
///   `Rgba8UnormSrgb`). Shaders write LINEAR colour; the view encodes it to sRGB.
/// - `surface_format` is the canvas's own storage format (the same without the sRGB suffix);
///   the host configures the surface with it and lists `format` as its view format.
/// - `alpha_mode` is `Opaque` (the canvas covers what is behind it; alpha is ignored when the
///   page composites it) or, for a canvas with `transparent: true`, `PreMultiplied` (the page
///   shows through; colours must then be premultiplied by their alpha).
///
/// THE BLENDING CONVENTION, which every layer follows so that layers from independent crates
/// compose: a layer writes LINEAR colour, PREMULTIPLIED by its alpha, into a view of `format`
/// (never sRGB-encoding in its own shader: the view encodes on write and decodes for blending),
/// with `wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING` (or `REPLACE` where it is opaque), in
/// render passes that LOAD the target (`LoadOp::Load`): the host has cleared it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasTarget {
    /// The format of the frame's view and of the pipelines that draw into it (sRGB).
    pub format: wgpu::TextureFormat,
    /// The canvas's storage format, as the surface is configured.
    pub surface_format: wgpu::TextureFormat,
    /// How the page composites the canvas.
    pub alpha_mode: wgpu::CompositeAlphaMode,
}

/// One frame to draw, the same for every layer of the canvas. The host has already recorded the
/// clear of the target (to the canvas's clear colour) into `encoder`; each layer, in order,
/// records render passes that LOAD `view` and draw over what is there; then the host submits the
/// encoder once and presents the texture.
pub struct CanvasFrame<'a> {
    /// The context the surface belongs to (device and queue).
    pub context: &'a GpuContext,
    /// The canvas's surface.
    pub surface: &'a wgpu::Surface<'static>,
    /// The surface's configuration, as the host applied it.
    pub config: &'a wgpu::SurfaceConfiguration,
    /// The formats and alpha mode every renderer of this canvas draws with.
    pub target: CanvasTarget,
    /// The texture the layers draw into: this frame's surface texture, or - on a transparent
    /// canvas - the host's own texture of the same size and format, which the host writes onto
    /// the surface after the last layer (see `WebGpuCanvas`).
    pub texture: &'a wgpu::Texture,
    /// The view of this frame's texture in `target.format`: render into it.
    pub view: &'a wgpu::TextureView,
    /// The frame's command encoder, shared by every layer of the canvas.
    pub encoder: &'a mut wgpu::CommandEncoder,
    /// Timing and size, from the canvas's one clock.
    pub info: FrameInfo,
    /// The clock's named points in time, as they were when the frame began.
    pub points: &'a TimePoints,
}

impl CanvasFrame<'_> {
    /// The named point's time on the clock; None for a name the clock does not define.
    pub fn point_time(&self, name: &str) -> Option<f64> {
        self.points.time(name)
    }

    /// Clock seconds since the named point (negative before it); None for an undefined name.
    pub fn since_point(&self, name: &str) -> Option<f64> {
        self.points.since(name, self.info.time)
    }

    /// True when this frame's time is at or past the named point (false for an undefined name).
    pub fn point_reached(&self, name: &str) -> bool {
        self.points.reached(name, self.info.time)
    }

    /// True in the ONE frame whose step of the clock reaches the named point (from the previous
    /// frame's time, exclusive, to this frame's, inclusive); false for an undefined name. Every
    /// layer sees it in the same frame.
    pub fn point_crossed(&self, name: &str) -> bool {
        self.points.crossed(name, self.info.previous_time, self.info.time)
    }
}

/// Why a canvas did not become a drawable surface. The host stops drawing that canvas and hands
/// this to [`CanvasRenderer::surface_failed`].
#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceFailure {
    /// The canvas could not be made a `wgpu` surface (wgpu's reason).
    CreateSurface(String),
    /// The adapter of the context cannot present to this canvas.
    NotPresentable,
}

impl fmt::Display for SurfaceFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SurfaceFailure::CreateSurface(reason) => write!(f, "The canvas could not be made a WebGPU surface ({reason}). Make sure the GpuContext handed to WebGpuCanvas was requested in this page (GpuContext::request) and not in another window."),
            SurfaceFailure::NotPresentable => write!(f, "The WebGPU adapter cannot present to this canvas. Request the GpuContext again (GpuContext::request) and hand the new one to WebGpuCanvas."),
        }
    }
}

impl std::error::Error for SurfaceFailure {}

/// Drawing code for a canvas: one LAYER of it ([`CanvasLayers`](crate::CanvasLayers)). The host
/// calls, in this order: `prepare` once, before the layer's first frame on a configured surface;
/// `resize` after `prepare` and whenever the surface's size changes; `render` once per animation
/// frame; and `input` with every pointer, wheel and key event on the canvas. Only `render` must
/// be written. Nothing assumes a layer is alone: it draws over the layers below it (see
/// [`CanvasTarget`] for the blending convention) and never clears the target.
pub trait CanvasRenderer {
    /// Creates what the renderer needs (pipelines, buffers) for this target (`target.format` is
    /// the format of its pipelines' colour target).
    fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {}

    /// The surface now has this size in device pixels (also called once after `prepare`).
    fn resize(&mut self, _context: &GpuContext, _width: u32, _height: u32) {}

    /// Records this frame's drawing into `frame.encoder` (render passes that load `frame.view`).
    /// It may also submit work of its own on the queue; that runs before the frame's encoder.
    fn render(&mut self, frame: &mut CanvasFrame<'_>);

    /// One input event on the canvas.
    fn input(&mut self, _input: &CanvasInput) {}

    /// The canvas did not become a drawable surface and nothing will be drawn into it. By
    /// default the reason (which says what to do) is written to the browser's console as an
    /// error; a renderer may show it on the page instead.
    fn surface_failed(&mut self, failure: &SurfaceFailure) {
        web_sys::console::error_1(&failure.to_string().into());
    }
}

/// A shared handle to a renderer, as the canvas host component takes it. Two handles compare
/// equal when they are the same renderer, so a page that keeps its handle (for example in
/// `use_hook`) does not restart the canvas when it renders again.
#[derive(Clone)]
pub struct RendererHandle {
    inner: Rc<RefCell<dyn CanvasRenderer>>,
}

impl RendererHandle {
    /// Wraps a renderer.
    pub fn new(renderer: impl CanvasRenderer + 'static) -> RendererHandle {
        RendererHandle { inner: Rc::new(RefCell::new(renderer)) }
    }

    /// Runs `f` with the renderer; does nothing if the renderer is busy (an input event that
    /// arrives while it renders).
    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut dyn CanvasRenderer) -> R) -> Option<R> {
        let mut renderer = self.inner.try_borrow_mut().ok()?;
        Some(f(&mut *renderer))
    }
}

impl fmt::Debug for RendererHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RendererHandle({:p})", Rc::as_ptr(&self.inner) as *const ())
    }
}

impl PartialEq for RendererHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::{CanvasFrame, CanvasRenderer, RendererHandle, SurfaceFailure};
    use crate::input::CanvasInput;

    struct Counting {
        inputs: usize,
    }

    impl CanvasRenderer for Counting {
        fn render(&mut self, _frame: &mut CanvasFrame<'_>) {}

        fn input(&mut self, _input: &CanvasInput) {
            self.inputs += 1;
        }
    }

    #[test]
    fn a_handle_equals_its_clones_only() {
        let first = RendererHandle::new(Counting { inputs: 0 });
        let second = RendererHandle::new(Counting { inputs: 0 });
        assert!(first == first.clone());
        assert!(first != second);
    }

    #[test]
    fn a_busy_renderer_is_skipped() {
        let handle = RendererHandle::new(Counting { inputs: 0 });
        let inner = handle.clone();
        let nested = handle.with(|_| inner.with(|r| r.input(&CanvasInput::KeyDown { key: "a".to_string() })));
        assert_eq!(nested, Some(None));
        assert_eq!(handle.with(|r| r.input(&CanvasInput::KeyUp { key: "a".to_string() })), Some(()));
    }

    #[test]
    fn surface_failures_say_what_to_do() {
        assert!(SurfaceFailure::CreateSurface("no context".to_string()).to_string().contains("(no context)"));
        assert!(SurfaceFailure::NotPresentable.to_string().contains("Request the GpuContext again"));
    }
}
