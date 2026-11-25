//! The renderer core (INTERNAL): our own 2D renderer for wgpu on kurbo - stencil-then-cover
//! with 4x MSAA, non-zero and even-odd fills, strokes with caps, joins, miter limit and dashes,
//! solid colours, linear / radial / sweep gradients with pad / repeat / reflect, images,
//! opacity, clips, groups with every mix and compose mode; stroke sets in one instanced draw each
//! (strokes.wgsl), their colours picked from a live picture; a layer's surface finish (a height
//! pass and the lit composite); drawing into a picture target instead of the canvas; a stroke
//! set's checkpoint cache (saved multisampled states restored by a texture copy) and stroke-level
//! transitions (two record buffers per draw); a layer's water - refraction and reflection - in its
//! effect group's composite, ripple fields evaluated per pixel. The consumer-facing API is NoSkia's
//! (the session and its shapes) and [`VectorScene`](crate::rendering::VectorScene).

mod build;
mod gpu;
mod paint;
mod texture_ledger;
#[cfg(test)]
mod tests_checkpoints;
#[cfg(test)]
mod tests_gpu_half;
#[cfg(test)]
mod tests_ownership_upload;
#[cfg(test)]
mod tests_strokes;

pub use build::RenderStats;
pub(crate) use build::FxPassData;
#[cfg(test)]
pub(crate) use build::Builder as TestBuilder;
#[cfg(test)]
pub(crate) use build::StrokeSource as TestStrokeSource;
#[cfg(test)]
pub(crate) use build::{CompositeMode as TestCompositeMode, Op as TestOp, WaterBinding as TestWaterBinding};
pub(crate) use paint::srgb_to_linear;

use codebrix_webgpu::{CanvasFrame, CanvasTarget, GpuContext};

use crate::rendering::cpu_clock;
use crate::rendering::picture_target::PictureTarget;
use crate::rendering::vector_scene::VectorScene;

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
        let Some(gpu) = self.gpu.as_mut() else {
            return RenderStats::default();
        };
        let start = cpu_clock::now_ms();
        let (width, height) = picture.map_or((frame.info.width, frame.info.height), |p| p.size());
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

    /// Maps the last readback copy (call after the frame that made it was submitted).
    pub(crate) fn map_readback(&self, done: Box<dyn FnOnce(Vec<u8>, u32, u32)>) {
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.map_readback(done);
        }
    }
}
