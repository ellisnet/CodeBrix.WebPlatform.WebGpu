//! `OffscreenRenderer` (Not in C#): this crate's renderer WITHOUT a canvas. It draws a
//! [`VectorScene`] (a [`DrawingSession`](crate::DrawingSession) records itself into one) into a
//! texture view the application owns, on the application's command encoder - for a program with
//! no window (a renderer of stills, a test that reads its pixels back), or a page that wants a
//! drawing in a texture of its own. It is the same renderer [`AnimationLayer`] draws a canvas
//! frame with, and it draws the same pixels.
//!
//! THE CONVENTION is the canvas host's ([`CanvasTarget`], here [`CanvasTarget::offscreen`]): the
//! renderer writes LINEAR colour, PREMULTIPLIED by its alpha, over what the view holds (it LOADS
//! the view and never clears it), so the application clears the view first - to transparent, or
//! to a colour - in a pass of its own on the same encoder. With an sRGB format (`Rgba8UnormSrgb`,
//! `Bgra8UnormSrgb`) the view encodes on write, exactly as on a canvas; with a linear format
//! (`Rgba16Float`) the texture holds the linear values.
//!
//! A [`PictureTarget`](crate::rendering::PictureTarget) is drawn only by its canvas layer
//! ([`AnimationLayer::to_picture`]): a scene drawn here that reads one reads it as that layer
//! last left it.
//!
//! GPU TIMING ([`OffscreenRenderer::set_gpu_timing`], off by default) works as a layer's, with one
//! step more: the renderer does not see the application submit its encoder, so the application
//! says so with [`OffscreenRenderer::encoder_submitted`] after `queue.submit`; only then are the
//! results of the renders recorded into that encoder mapped (mapping earlier would be an error at
//! the submit). Results are read back asynchronously, as everywhere in WebGPU: in a browser the map
//! finishes by itself a frame or so later; in a NATIVE program the map finishes only when the
//! device is polled - call `context.device().poll(wgpu::PollType::wait_indefinitely())` after
//! `encoder_submitted`, then read [`OffscreenRenderer::gpu_ms`]. Without `encoder_submitted` the
//! results are never mapped: after a few renders every read-back buffer is waiting and nothing more
//! is timed (`gpu_ms` stays None; never an error).
//!
//! [`AnimationLayer`]: crate::rendering::AnimationLayer
//! [`AnimationLayer::to_picture`]: crate::rendering::AnimationLayer::to_picture

use codebrix_webgpu::{wgpu, CanvasTarget, GpuContext};

use crate::rendering::core::{FrameOutput, RenderStats, VectorRenderer};
use crate::rendering::vector_scene::VectorScene;

/// The renderer drawing into the application's own textures (Not in C#; see the module's text).
/// Keep one for many draws: it holds its pipelines, its buffers and the pictures it has uploaded,
/// as a layer does from frame to frame.
pub struct OffscreenRenderer {
    renderer: VectorRenderer,
    format: wgpu::TextureFormat,
    gpu_timing: bool,
    gpu_timing_supported: bool,
}

impl OffscreenRenderer {
    /// A renderer on `context`'s device whose pipelines draw into views of `format` (the format
    /// of every view handed to [`OffscreenRenderer::render`]; the view's texture needs
    /// `RENDER_ATTACHMENT`).
    pub fn new(context: &GpuContext, format: wgpu::TextureFormat) -> OffscreenRenderer {
        let mut renderer = VectorRenderer::default();
        renderer.prepare(context, &CanvasTarget::offscreen(format));
        let mut gpu_timing_supported = false;
        if let Some(timing) = renderer.timing() {
            // Mapped when the application says it submitted, never at the next render.
            timing.set_map_on_next_frame(false);
            gpu_timing_supported = timing.supported();
        }
        OffscreenRenderer { renderer, format, gpu_timing: false, gpu_timing_supported }
    }

    /// The format of the views it draws into.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Records the drawing of `scene` over `view` (`width` x `height` device pixels: the size of
    /// the view's texture) into `encoder`, on `context` (the context the renderer was made with),
    /// and returns the frame's counters and CPU timings (`record_ms` stays 0: recording the scene
    /// is the caller's). Nothing is submitted: submit the encoder, then read or sample the
    /// texture.
    pub fn render(&mut self, context: &GpuContext, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, width: u32, height: u32, scene: &VectorScene) -> RenderStats {
        let mut output = FrameOutput { context, encoder, view, width, height };
        self.renderer.render_output(&mut output, scene, false, None)
    }

    /// Switches the GPU TIMING of the renders on or off (Not in C#; off by default): while on, each
    /// render's [`RenderStats::gpu_ms`] and [`OffscreenRenderer::gpu_ms`] hold the GPU
    /// milliseconds of the latest render whose result has come back (see the module's text: call
    /// [`OffscreenRenderer::encoder_submitted`] after each submit). Needs the device feature
    /// `TIMESTAMP_QUERY` (see [`OffscreenRenderer::gpu_timing_supported`]); without it the timing
    /// is a quiet no-op. Off, the renders record their passes exactly as without timing.
    pub fn set_gpu_timing(&mut self, on: bool) {
        self.gpu_timing = on;
        if let Some(timing) = self.renderer.timing() {
            timing.set_enabled(on);
        }
    }

    /// True while the GPU timing is switched on (asked for, whether or not the device can).
    pub fn gpu_timing(&self) -> bool {
        self.gpu_timing
    }

    /// True when the renderer's device can time its work (it has `TIMESTAMP_QUERY`).
    pub fn gpu_timing_supported(&self) -> bool {
        self.gpu_timing_supported
    }

    /// Sets the byte budget of the renderer's GEOMETRY CACHE (Not in C#; default 32 MiB,
    /// 33,554,432 bytes), from the next render on: what a smaller or larger budget costs is said
    /// at [`AnimationLayerControl::set_geometry_cache_budget`](crate::rendering::AnimationLayerControl::set_geometry_cache_budget),
    /// which sets the same for a layer. Nothing drawn changes.
    pub fn set_geometry_cache_budget(&mut self, bytes: usize) {
        self.renderer.set_cache_budget(bytes);
    }

    /// The geometry cache's byte budget (32 MiB unless set).
    pub fn geometry_cache_budget(&self) -> usize {
        self.renderer.cache_budget()
    }

    /// OPT-IN (Not in C#; off by default): sizes the renderer's INTERNAL TARGETS to the drawing
    /// (true) instead of the view (false, the default: drawn exactly as always), from the next
    /// render on, as
    /// [`AnimationLayerControl::set_content_sized_targets`](crate::rendering::AnimationLayerControl::set_content_sized_targets)
    /// does for a layer (when to use it, the rule the targets follow and what it costs are said
    /// there): the view handed to [`OffscreenRenderer::render`] is drawn over at the same places
    /// either way.
    pub fn set_content_sized_targets(&mut self, on: bool) {
        self.renderer.set_content_sized_targets(on);
    }

    /// True while the internal targets are sized to the drawing (false by default).
    pub fn content_sized_targets(&self) -> bool {
        self.renderer.content_sized_targets()
    }

    /// Says that the encoder of the renders recorded so far has been SUBMITTED (call it right after
    /// `queue.submit`): their timing results are mapped now, to be read a little later (natively:
    /// after the device is polled). Nothing happens while the timing is off and nothing waits.
    pub fn encoder_submitted(&mut self) {
        if let Some(timing) = self.renderer.timing() {
            timing.collect();
            timing.submitted();
        }
    }

    /// The GPU milliseconds of the latest timed render whose result has come back (taking the
    /// results that have arrived first); None while the timing is off, where the device lacks
    /// `TIMESTAMP_QUERY`, and before the first result.
    pub fn gpu_ms(&mut self) -> Option<f64> {
        let timing = self.renderer.timing()?;
        timing.collect();
        timing.latest_ms()
    }
}
