//! The renderer core (INTERNAL): our own 2D renderer for wgpu on kurbo - stencil-then-cover
//! with 4x MSAA, non-zero and even-odd fills, strokes with caps, joins, miter limit and dashes,
//! solid colours, linear / radial / sweep gradients with pad / repeat / reflect, images,
//! opacity, clips, groups with every mix and compose mode; stroke sets in one instanced draw each
//! (strokes.wgsl), their colours picked from a live picture; a layer's surface finish (a height
//! pass and the lit composite); drawing into a picture target instead of the canvas; a stroke
//! set's checkpoint cache (saved multisampled states restored by a texture copy) and stroke-level
//! transitions (two record buffers per draw); a layer's water - refraction and reflection - in its
//! effect group's composite, ripple fields evaluated per pixel; internal targets sized to the
//! frame's drawing instead of the canvas, opted in (target_window.rs). The consumer-facing API is NoSkia's
//! (the session and its shapes) and [`VectorScene`](crate::rendering::VectorScene).

mod build;
mod gpu;
mod gpu_timer;
mod paint;
mod target_window;
mod texture_ledger;
#[cfg(test)]
mod tests_checkpoints;
#[cfg(test)]
mod tests_gpu_half;
#[cfg(test)]
mod tests_ownership_upload;
#[cfg(test)]
mod tests_strokes;
#[cfg(test)]
mod tests_target_window;

pub use build::RenderStats;
pub(crate) use build::CACHE_BUDGET_BYTES;
pub(crate) use build::FxPassData;
#[cfg(test)]
pub(crate) use build::Builder as TestBuilder;
#[cfg(test)]
pub(crate) use build::StrokeSource as TestStrokeSource;
#[cfg(test)]
pub(crate) use build::{CompositeMode as TestCompositeMode, Op as TestOp, WaterBinding as TestWaterBinding};
pub(crate) use paint::srgb_to_linear;

use codebrix_webgpu::{wgpu, CanvasFrame, CanvasTarget, GpuContext};

use crate::rendering::cpu_clock;
use crate::rendering::picture_target::PictureTarget;
use crate::rendering::vector_scene::VectorScene;

/// What one draw of the renderer records into, and all it needs of a frame: the context (device
/// and queue), the encoder, the view to draw over and its size in device pixels. A canvas frame
/// gives them ([`VectorRenderer::render`]); so can a texture the application owns, with no
/// canvas and no surface ([`OffscreenRenderer`](crate::rendering::OffscreenRenderer)).
pub(crate) struct FrameOutput<'a> {
    /// The device and queue.
    pub(crate) context: &'a GpuContext,
    /// The encoder the passes are recorded into.
    pub(crate) encoder: &'a mut wgpu::CommandEncoder,
    /// The view drawn over (loaded, never cleared), in the format the renderer was prepared for.
    pub(crate) view: &'a wgpu::TextureView,
    /// The view's width in device pixels.
    pub(crate) width: u32,
    /// The view's height in device pixels.
    pub(crate) height: u32,
}

/// The renderer: the CPU builder and the GPU state.
#[derive(Default)]
pub(crate) struct VectorRenderer {
    builder: build::Builder,
    gpu: Option<gpu::Gpu>,
    target: Option<CanvasTarget>,
}

impl VectorRenderer {
    /// Makes the pipelines for this target (again when the target changes).
    pub(crate) fn prepare(&mut self, context: &GpuContext, target: &CanvasTarget) {
        if self.target != Some(*target) || self.gpu.is_none() {
            self.gpu = Some(gpu::Gpu::new(context.device(), target));
            self.target = Some(*target);
            self.builder.ramps.clear();
        }
    }

    /// The canvas has a new size.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.resize(width, height);
        }
    }

    /// Draws the scene over the frame - or, for a picture target's layer, into the picture (at
    /// its size). `read` also copies the layer's own pixels for [`VectorRenderer::map_readback`].
    pub(crate) fn render(&mut self, frame: &mut CanvasFrame<'_>, scene: &VectorScene, read: bool, picture: Option<&PictureTarget>) -> RenderStats {
        let mut output = FrameOutput { context: frame.context, encoder: &mut *frame.encoder, view: frame.view, width: frame.info.width, height: frame.info.height };
        self.render_output(&mut output, scene, read, picture)
    }

    /// [`VectorRenderer::render`] into any output: a canvas frame's, or an offscreen texture's.
    pub(crate) fn render_output(&mut self, frame: &mut FrameOutput<'_>, scene: &VectorScene, read: bool, picture: Option<&PictureTarget>) -> RenderStats {
        let Some(gpu) = self.gpu.as_mut() else {
            return RenderStats::default();
        };
        let start = cpu_clock::now_ms();
        let (width, height) = picture.map_or((frame.width, frame.height), |p| p.size());
        self.builder.build(scene, width, height);
        let built = cpu_clock::now_ms();
        let (draw_calls, passes) = gpu.encode(frame, &mut self.builder, read, picture);
        let mut stats = self.builder.stats;
        stats.draw_calls = draw_calls;
        stats.passes = passes;
        stats.build_ms = (built - start).max(0.0);
        stats.encode_ms = cpu_clock::since_ms(built);
        stats
    }

    /// Sets the geometry cache's byte budget (build.rs `CACHE_BUDGET_BYTES` by default); the
    /// next frame keeps to it.
    pub(crate) fn set_cache_budget(&mut self, bytes: usize) {
        self.builder.set_cache_budget(bytes);
    }

    /// The geometry cache's byte budget.
    pub(crate) fn cache_budget(&self) -> usize {
        self.builder.cache_budget()
    }

    /// Internal targets sized to the frame's drawing (true, opt-in) or to the canvas (false, the
    /// default; target_window.rs; Not in C#).
    pub(crate) fn set_content_sized_targets(&mut self, on: bool) {
        self.builder.set_content_sized_targets(on);
    }

    /// True while the internal targets are sized to the drawing (off by default).
    pub(crate) fn content_sized_targets(&self) -> bool {
        self.builder.content_sized_targets()
    }

    /// The GPU timing of the renderer's work (gpu_timer.rs; Not in C#), once prepared.
    pub(crate) fn timing(&mut self) -> Option<&mut gpu_timer::GpuTiming> {
        self.gpu.as_mut().map(|gpu| &mut gpu.timing)
    }

    /// Maps the last readback copy (call after the frame that made it was submitted).
    pub(crate) fn map_readback(&self, done: Box<dyn FnOnce(Vec<u8>, u32, u32)>) {
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.map_readback(done);
        }
    }
}
