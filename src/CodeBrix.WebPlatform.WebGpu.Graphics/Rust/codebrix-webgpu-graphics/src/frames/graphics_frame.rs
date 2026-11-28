//! [`GraphicsFrame`]: one frame's command recording - on an encoder of its own, on an encoder it
//! is handed, or on the encoder of a canvas frame (as a LAYER of the canvas host) - and the
//! render and compute passes it begins (C#: `GraphicsFrame`, `RenderPassOptions`). The copies,
//! clears, query resolves and debug groups are in `graphics_frame_commands.rs`.
//!
//! A FRAME WITH A TARGET OF ITS OWN WITHOUT A CANVAS (Not in C#):
//! [`GraphicsContext::begin_frame_on`] and [`GraphicsContext::begin_frame_with_encoder_on`] give
//! a frame on any [`RenderTargetView`] (a headless target, a texture of the application's) the
//! target that a canvas frame has, so that code written for "the frame's own target"
//! ([`GraphicsFrame::begin_render_pass`]) runs offscreen unchanged.
//!
//! MULTISAMPLING (Not in C#): a pass on a multisampled target ends by resolving into the target's
//! resolve view; a frame begun with [`GraphicsContext::begin_canvas_frame_on`] on a 4-sample
//! [`CanvasFrameTarget`] draws into the target's multisampled buffers and, when it is dropped,
//! lays the resolved picture over the canvas.

use crate::common::Color4;
use crate::context::{GraphicsContext, GraphicsException};
use crate::frames::pass_attachments::{plan_color_attachment, ColorLoad, ResolveInto};
use crate::frames::{ComputePass, ComputePassOptions, RenderPass};
use crate::presentation::{CanvasFrameTarget, CanvasLayerResolve, RenderTargetView};
use crate::queries::{GpuQuerySet, PassTimestamps};
use crate::resources::{GpuBuffer, GpuTexture, TextureFormats};

/// The rarely used settings of a render pass (C#: `RenderPassOptions`).
#[derive(Clone, Debug, Default)]
pub struct RenderPassOptions {
    /// The pass's label.
    pub label: Option<String>,
    /// Timestamps written when the pass begins and ends (C#: `Timestamps`; a
    /// [`GpuTimer::pass`](crate::GpuTimer::pass) slot, or a [`PassTimestamps`] of your own). Needs
    /// the device feature `TIMESTAMP_QUERY`.
    pub timestamps: Option<PassTimestamps>,
    /// The occlusion query set the pass's occlusion queries write (C#: `OcclusionQuerySet`; see
    /// [`RenderPass::begin_occlusion_query`] - not usable with this version of `wgpu` in a browser).
    pub occlusion_query_set: Option<GpuQuerySet>,
    /// The value the stencil is cleared to.
    pub stencil_clear_value: u32,
    /// True to keep the stencil instead of clearing it.
    pub load_stencil: bool,
    /// True when the pass does not write depth.
    pub depth_read_only: bool,
    /// True when the pass does not write stencil.
    pub stencil_read_only: bool,
    /// The view a multisampled colour target resolves into, in place of the target's own
    /// [`RenderTargetView::resolve_view`] (Err on a 1-sample target: only a multisampled target
    /// resolves). Usually None: a multisampled target names its resolve view itself.
    pub resolve_target: Option<wgpu::TextureView>,
    /// True to discard the colour target after the pass (it was only an intermediate).
    pub discard_color: bool,
}

/// The encoder a frame records into.
enum FrameEncoder<'a> {
    Owned(wgpu::CommandEncoder),
    Borrowed(&'a mut wgpu::CommandEncoder),
}

/// One frame's recording (C#: `GraphicsFrame`): begin render passes, copy, then submit. Made by
/// [`GraphicsContext::begin_frame`] (its own encoder, which it submits),
/// [`GraphicsContext::begin_frame_with_encoder`] (an encoder the caller owns and submits), their
/// twins with a target of the frame's own, [`GraphicsContext::begin_frame_on`] and
/// [`GraphicsContext::begin_frame_with_encoder_on`] (Not in C#), or
/// [`GraphicsContext::begin_canvas_frame`] (the canvas host's frame: the graphics are a LAYER
/// that draws over what is there; the host submits and presents).
///
/// Rust's borrows enforce what C# checks at run time: one pass open at a time, nothing recorded
/// after submit.
///
/// A frame on a multisampled canvas layer ([`GraphicsContext::begin_canvas_frame_on`] with 4
/// samples) records one more draw when it is DROPPED: the layer's resolved picture laid over the
/// canvas. Let it go out of scope (as every canvas frame) once the layer's passes are recorded.
pub struct GraphicsFrame<'a> {
    context: GraphicsContext,
    /// Always Some; taken only by `finish` (the frame implements Drop).
    encoder: Option<FrameEncoder<'a>>,
    target: Option<RenderTargetView>,
    id: u64,
    canvas_resolve: Option<CanvasLayerResolve>,
}

impl GraphicsContext {
    /// A frame with an encoder of its own and no target of its own (C#: `BeginFrame()`); pass a
    /// target to each render pass, then [`GraphicsFrame::submit`].
    pub fn begin_frame(&self) -> GraphicsFrame<'static> {
        let encoder = self.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("codebrix-webgpu-graphics frame") });
        GraphicsFrame { context: self.clone(), encoder: Some(FrameEncoder::Owned(encoder)), target: None, id: self.next_frame_id(), canvas_resolve: None }
    }

    /// A frame that records into an encoder the caller owns and submits (C#:
    /// `BeginFrame(CommandEncoder)`).
    pub fn begin_frame_with_encoder<'a>(&self, encoder: &'a mut wgpu::CommandEncoder) -> GraphicsFrame<'a> {
        GraphicsFrame { context: self.clone(), encoder: Some(FrameEncoder::Borrowed(encoder)), target: None, id: self.next_frame_id(), canvas_resolve: None }
    }

    /// A frame with an encoder of its own whose OWN target is `target` (Not in C#): the frame a
    /// canvas layer gets, without a canvas - [`GraphicsFrame::begin_render_pass`] draws into
    /// `target` (its colour, its depth if it has one, resolving if it is multisampled) and
    /// [`GraphicsFrame::target`] returns it, so a renderer written for a canvas frame's target
    /// draws into a headless target or a texture of the application's unchanged. Submit it with
    /// [`GraphicsFrame::submit`]. Unlike a canvas frame nothing has cleared the target: clear it
    /// in the first pass (`clear_color: Some(..)`).
    pub fn begin_frame_on(&self, target: &RenderTargetView) -> GraphicsFrame<'static> {
        let encoder = self.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("codebrix-webgpu-graphics frame") });
        GraphicsFrame { context: self.clone(), encoder: Some(FrameEncoder::Owned(encoder)), target: Some(target.clone()), id: self.next_frame_id(), canvas_resolve: None }
    }

    /// [`GraphicsContext::begin_frame_on`] recording into an encoder the caller owns and submits
    /// (Not in C#; the twin of [`GraphicsContext::begin_frame_with_encoder`]): for drawing into
    /// `target` on an encoder shared with other work - another crate's renderer, the caller's own
    /// copies and readback.
    pub fn begin_frame_with_encoder_on<'a>(&self, encoder: &'a mut wgpu::CommandEncoder, target: &RenderTargetView) -> GraphicsFrame<'a> {
        GraphicsFrame { context: self.clone(), encoder: Some(FrameEncoder::Borrowed(encoder)), target: Some(target.clone()), id: self.next_frame_id(), canvas_resolve: None }
    }

    /// A frame on a canvas host's frame (C#: `BeginFrame(IWebGpuTextureTarget, depth)`): it records
    /// into the frame's encoder (the host submits it once and presents), and its target is the
    /// frame's view in the canvas's draw format, with `depth` if given (a depth texture of the
    /// frame's size that the layer keeps). Begin passes with `clear_color: None` so that the
    /// layer LOADS what the host and the layers below drew. Always 1 sample per pixel; for
    /// multisampling, or a depth buffer that follows the canvas's size by itself, use
    /// [`GraphicsContext::begin_canvas_frame_on`].
    pub fn begin_canvas_frame<'a>(&self, frame: &'a mut codebrix_webgpu::CanvasFrame<'_>, depth: Option<&GpuTexture>) -> GraphicsFrame<'a> {
        let mut target = RenderTargetView::from_canvas(frame);
        if let Some(depth) = depth {
            target = target.with_depth(depth);
        }
        GraphicsFrame { context: self.clone(), encoder: Some(FrameEncoder::Borrowed(&mut *frame.encoder)), target: Some(target), id: self.next_frame_id(), canvas_resolve: None }
    }

    /// A frame on a canvas host's frame that draws through a [`CanvasFrameTarget`] the layer keeps
    /// (Not in C#): with the target's depth buffer (at the canvas's size, made again after a
    /// resize) and with its SAMPLE COUNT, which may change between frames. With 1 sample this is
    /// [`GraphicsContext::begin_canvas_frame`] with the target's depth. With 4 the frame's target
    /// ([`GraphicsFrame::target`]) is the target's multisampled colour and depth; every pass ends
    /// by resolving, the first pass of the frame clears the multisampled colour (to its clear
    /// colour, or to transparent when it asks to load), and when the frame is dropped the resolved
    /// picture is laid over the canvas with premultiplied-alpha blending - the layer draws over
    /// what is below exactly as with 1 sample, its edges smoothed. The library's renderers pick
    /// the matching pipelines by themselves; a pipeline of the application's own must be made with
    /// `.with_sample_count(pass.target().sample_count())` (one per count). Err when a buffer
    /// cannot be made.
    pub fn begin_canvas_frame_on<'a>(&self, frame: &'a mut codebrix_webgpu::CanvasFrame<'_>, target: &mut CanvasFrameTarget) -> Result<GraphicsFrame<'a>, GraphicsException> {
        let (view, canvas_resolve) = target.prepare(frame.view, frame.info.width, frame.info.height, frame.target.format)?;
        Ok(GraphicsFrame { context: self.clone(), encoder: Some(FrameEncoder::Borrowed(&mut *frame.encoder)), target: Some(view), id: self.next_frame_id(), canvas_resolve })
    }
}

impl<'a> GraphicsFrame<'a> {
    /// The context (C#: `Context`).
    pub fn context(&self) -> &GraphicsContext {
        &self.context
    }

    /// The encoder, for `wgpu` commands of the caller's own (C#: `Encoder`).
    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        match self.encoder.as_mut().expect("a frame keeps its encoder until finish") {
            FrameEncoder::Owned(encoder) => encoder,
            FrameEncoder::Borrowed(encoder) => encoder,
        }
    }

    /// True when the frame made its encoder and submits it (C#: `OwnsEncoder`).
    pub fn owns_encoder(&self) -> bool {
        matches!(self.encoder, Some(FrameEncoder::Owned(_)))
    }

    /// The frame's own target, if it has one (a canvas frame's, or the target of
    /// [`GraphicsContext::begin_frame_on`]) (C#: `Target`).
    pub fn target(&self) -> Option<&RenderTargetView> {
        self.target.as_ref()
    }

    /// The frame's number in its context, from 1 (C#: `Id`).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Begins a render pass on the frame's own target (C#: `BeginRenderPass(clearColor,
    /// clearDepth)`). `clear_color` None LOADS the colour (what a canvas layer must do);
    /// `clear_depth` None loads depth (C# passes 1 by default: write `Some(1.0)`).
    pub fn begin_render_pass(&mut self, clear_color: Option<Color4>, clear_depth: Option<f32>) -> Result<RenderPass<'_>, GraphicsException> {
        self.begin_render_pass_with_options(None, clear_color, clear_depth, &RenderPassOptions::default())
    }

    /// Begins a render pass on `target` (C#: `BeginRenderPass(target, clearColor, clearDepth)`).
    pub fn begin_render_pass_to(&mut self, target: &RenderTargetView, clear_color: Option<Color4>, clear_depth: Option<f32>) -> Result<RenderPass<'_>, GraphicsException> {
        self.begin_render_pass_with_options(Some(target), clear_color, clear_depth, &RenderPassOptions::default())
    }

    /// Begins a render pass on `target` (None: the frame's own) with options (C#:
    /// `BeginRenderPass(target, clearColor, clearDepth, options)`).
    pub fn begin_render_pass_with_options(&mut self, target: Option<&RenderTargetView>, clear_color: Option<Color4>, clear_depth: Option<f32>, options: &RenderPassOptions) -> Result<RenderPass<'_>, GraphicsException> {
        let target = pass_target(target, self.target.as_ref())?.clone();
        // A multisampled canvas layer's first pass clears (the layers below are not in its buffer).
        let first_layer_pass = match &mut self.canvas_resolve {
            Some(resolve) if !resolve.started && &resolve.multisampled_view == target.color_view() => {
                resolve.started = true;
                true
            }
            _ => false,
        };
        let plan = plan_color_attachment(target.sample_count(), target.resolve_view().is_some(), options.resolve_target.is_some(), clear_color, first_layer_pass)?;
        let resolve_target = match plan.resolve {
            ResolveInto::Nothing => None,
            ResolveInto::TargetResolve => target.resolve_view(),
            ResolveInto::OptionResolve => options.resolve_target.as_ref(),
        };
        let color = wgpu::RenderPassColorAttachment {
            view: target.color_view(),
            depth_slice: None,
            resolve_target,
            ops: wgpu::Operations {
                load: match plan.load {
                    ColorLoad::Clear(color) => wgpu::LoadOp::Clear(color.to_wgpu()),
                    ColorLoad::Load => wgpu::LoadOp::Load,
                },
                store: if options.discard_color { wgpu::StoreOp::Discard } else { wgpu::StoreOp::Store },
            },
        };
        let depth_stencil = target.depth_view().map(|view| {
            let depth_ops = if options.depth_read_only {
                None
            } else {
                Some(wgpu::Operations { load: clear_depth.map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear), store: wgpu::StoreOp::Store })
            };
            let has_stencil = target.depth_format().is_some_and(TextureFormats::has_stencil);
            let stencil_ops = if !has_stencil || options.stencil_read_only {
                None
            } else {
                Some(wgpu::Operations { load: if options.load_stencil { wgpu::LoadOp::Load } else { wgpu::LoadOp::Clear(options.stencil_clear_value) }, store: wgpu::StoreOp::Store })
            };
            wgpu::RenderPassDepthStencilAttachment { view, depth_ops, stencil_ops }
        });
        let frame_id = self.id;
        let pass = self
            .encoder()
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: options.label.as_deref(),
                color_attachments: &[Some(color)],
                depth_stencil_attachment: depth_stencil,
                timestamp_writes: options.timestamps.as_ref().map(PassTimestamps::to_render_pass),
                occlusion_query_set: options.occlusion_query_set.as_ref().map(GpuQuerySet::handle),
                multiview_mask: None,
            });
        Ok(RenderPass::new(pass, target, frame_id))
    }

    /// Begins a compute pass (C#: `BeginComputePass()` / `BeginComputePass(options)`; pass
    /// `&ComputePassOptions::default()` for C#'s first form). A compute pass works on the frame's
    /// encoder like a render pass, so a canvas LAYER may run one before its render pass - what it
    /// writes, the render pass reads.
    pub fn begin_compute_pass(&mut self, options: &ComputePassOptions) -> ComputePass<'_> {
        let pass = self.encoder().begin_compute_pass(&wgpu::ComputePassDescriptor { label: options.label.as_deref(), timestamp_writes: options.timestamps.as_ref().map(PassTimestamps::to_compute_pass) });
        ComputePass::new(pass)
    }

    /// Copies bytes between buffers (C#: `CopyBuffer`).
    pub fn copy_buffer(&mut self, source: &GpuBuffer, source_offset: u64, destination: &GpuBuffer, destination_offset: u64, size: u64) {
        self.encoder().copy_buffer_to_buffer(source.handle(), source_offset, destination.handle(), destination_offset, Some(size));
    }

    /// Submits the frame's commands (C#: `Submit`). A frame on a borrowed encoder (a canvas
    /// frame's, or the caller's) is submitted by the encoder's owner: dropping the frame is
    /// enough, and this returns an error that says so.
    pub fn submit(self) -> Result<(), GraphicsException> {
        let context = self.context.clone();
        let buffer = self.finish()?;
        context.queue().submit([buffer]);
        Ok(())
    }

    /// Finishes the frame's commands without submitting them (C#: `Finish`), for
    /// `GraphicsContext::queue().submit(...)` later.
    pub fn finish(mut self) -> Result<wgpu::CommandBuffer, GraphicsException> {
        match self.encoder.take().expect("a frame keeps its encoder until finish") {
            FrameEncoder::Owned(encoder) => Ok(encoder.finish()),
            FrameEncoder::Borrowed(_) => Err(GraphicsException::new("This frame records into an encoder it does not own (the canvas host's, or the caller's); the owner submits it - let the frame go out of scope instead of submitting it.")),
        }
    }
}

/// The target a render pass draws into: the one it is given, else the frame's own; Err, saying
/// how to give the frame one, when there is neither.
fn pass_target<'t, T>(given: Option<&'t T>, own: Option<&'t T>) -> Result<&'t T, GraphicsException> {
    given.or(own).ok_or_else(|| GraphicsException::new("The frame has no target of its own; pass a RenderTargetView (begin_render_pass_to), begin the frame on a target (begin_frame_on, or begin_frame_with_encoder_on with an encoder of yours), or begin it on a canvas frame (begin_canvas_frame)."))
}

impl Drop for GraphicsFrame<'_> {
    /// A multisampled canvas layer lays its resolved picture over the canvas (premultiplied
    /// alpha), once, if the frame drew into it.
    fn drop(&mut self) {
        let Some(resolve) = self.canvas_resolve.take() else {
            return;
        };
        if !resolve.started {
            return;
        }
        let Some(encoder) = self.encoder.as_mut() else {
            return;
        };
        let encoder: &mut wgpu::CommandEncoder = match encoder {
            FrameEncoder::Owned(encoder) => encoder,
            FrameEncoder::Borrowed(encoder) => encoder,
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("canvas resolved layer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &resolve.canvas_view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&resolve.pipeline);
        pass.set_bind_group(0, &resolve.group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::pass_target;

    #[test]
    fn a_pass_draws_into_the_target_it_is_given_else_the_frames_own() {
        //Arrange: the frame's own target, as begin_frame_on and the canvas frames set it
        let (own, given) = ("the frame's own", "a pass's own");
        //Act
        let default_pass = pass_target(None, Some(&own));
        let pass_to = pass_target(Some(&given), Some(&own));
        let pass_to_without_own = pass_target(Some(&given), None);
        //Assert
        assert_eq!(default_pass.ok(), Some(&own));
        assert_eq!(pass_to.ok(), Some(&given));
        assert_eq!(pass_to_without_own.ok(), Some(&given));
    }

    #[test]
    fn a_frame_without_a_target_says_how_to_give_it_one() {
        let error = pass_target::<&str>(None, None).expect_err("no target");
        let message = error.to_string();
        for call in ["begin_render_pass_to", "begin_frame_on", "begin_frame_with_encoder_on", "begin_canvas_frame"] {
            assert!(message.contains(call), "{call}: {message}");
        }
    }
}
