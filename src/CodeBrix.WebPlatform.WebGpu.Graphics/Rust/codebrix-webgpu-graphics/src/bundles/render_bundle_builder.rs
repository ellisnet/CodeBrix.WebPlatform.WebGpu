//! [`RenderBundleBuilder`]: records a [`GpuRenderBundle`] with the calls a render pass has (C#:
//! `RenderBundleBuilder`), and the context's bundle factories (C#: `GraphicsContext.Bundles.cs`).

use std::ops::Range;

use crate::bundles::GpuRenderBundle;
use crate::context::{label_text, GraphicsContext, GraphicsException};
use crate::frames::indirect_args::check_indirect;
use crate::frames::{BufferRange, DrawIndexedIndirectArgs, DrawIndexedOptions, DrawIndirectArgs, DrawOptions};
use crate::pipelines::GpuRenderPipeline;
use crate::presentation::RenderTargetView;
use crate::resources::{GpuBindGroup, GpuBuffer, TextureFormats};

/// The defaulted arguments of [`GraphicsContext::create_render_bundle_with_formats`] (C#:
/// `depthFormat = Undefined`, `sampleCount = 1`, `depthReadOnly = false`, `stencilReadOnly =
/// false`, `label = null`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderBundleOptions {
    /// The depth (or depth-stencil) format of the passes the bundle runs in, or None.
    pub depth_format: Option<wgpu::TextureFormat>,
    /// The sample count of those passes.
    pub sample_count: u32,
    /// True when the bundle never writes depth (its pipelines must not either).
    pub depth_read_only: bool,
    /// True when the bundle never writes stencil.
    pub stencil_read_only: bool,
    /// The bundle's label.
    pub label: Option<String>,
}

impl Default for RenderBundleOptions {
    fn default() -> RenderBundleOptions {
        RenderBundleOptions { depth_format: None, sample_count: 1, depth_read_only: false, stencil_read_only: false, label: None }
    }
}

/// One recorded command.
enum BundleCommand {
    SetPipeline(wgpu::RenderPipeline),
    SetBindGroup(u32, wgpu::BindGroup, Vec<u32>),
    SetVertexBuffer(u32, wgpu::Buffer, BufferRange),
    SetIndexBuffer(wgpu::Buffer, wgpu::IndexFormat, BufferRange),
    Draw(Range<u32>, Range<u32>),
    DrawIndexed(Range<u32>, i32, Range<u32>),
    DrawIndirect(wgpu::Buffer, u64),
    DrawIndexedIndirect(wgpu::Buffer, u64),
}

/// Records a [`GpuRenderBundle`] - a list of draw commands that a render pass replays cheaply
/// with [`RenderPass::execute_bundle`](crate::RenderPass::execute_bundle), for geometry drawn the
/// same way every frame (C#: `RenderBundleBuilder`). Start one with
/// [`GraphicsContext::create_render_bundle`] (or `_for_pipeline` / `_with_formats`), record, then
/// [`RenderBundleBuilder::finish`].
///
/// As in C#, the builder RECORDS the calls and replays them into WebGPU's bundle encoder when it
/// finishes; the bundle keeps what it uses alive. C# rehearses the commands to throw WebGPU's
/// message from `Finish`; here WebGPU's verdict arrives through `take_errors`, or await
/// [`RenderBundleBuilder::finish_checked`]. A bundle inherits nothing from the pass and leaves
/// the pass's pipeline, bind groups and buffers unset afterwards. Debug groups and markers are
/// accepted for symmetry with passes and dropped (C# drops them too).
pub struct RenderBundleBuilder {
    context: GraphicsContext,
    color_formats: Vec<wgpu::TextureFormat>,
    depth_format: Option<wgpu::TextureFormat>,
    sample_count: u32,
    depth_read_only: bool,
    stencil_read_only: bool,
    label: Option<String>,
    commands: Vec<BundleCommand>,
}

impl GraphicsContext {
    /// Starts a bundle for passes into targets like `target` - its colour and depth formats (C#:
    /// `CreateRenderBundle(target, sampleCount = 1, label)`). For a canvas layer, pass the target
    /// of the layer's graphics frame (`frame.target()`), with the layer's depth if it has one.
    pub fn create_render_bundle(&self, target: &RenderTargetView, sample_count: u32, label: Option<&str>) -> RenderBundleBuilder {
        self.new_bundle(vec![target.color_format()], target.depth_format(), sample_count, false, false, label)
    }

    /// Starts a bundle with the target formats and sample count of a pipeline (C#:
    /// `CreateRenderBundle(pipeline, label)`).
    pub fn create_render_bundle_for_pipeline(&self, pipeline: &GpuRenderPipeline, label: Option<&str>) -> RenderBundleBuilder {
        self.new_bundle(pipeline.color_formats().to_vec(), pipeline.depth_format(), pipeline.sample_count().max(1), false, false, label)
    }

    /// Starts a bundle with explicit attachment formats (C#: `CreateRenderBundle(colorFormats,
    /// depthFormat, sampleCount, depthReadOnly, stencilReadOnly, label)`).
    pub fn create_render_bundle_with_formats(&self, color_formats: &[wgpu::TextureFormat], options: &RenderBundleOptions) -> RenderBundleBuilder {
        self.new_bundle(color_formats.to_vec(), options.depth_format, options.sample_count, options.depth_read_only, options.stencil_read_only, options.label.as_deref())
    }

    /// Wraps a `wgpu` render bundle made elsewhere (C#: `WrapRenderBundle`); its formats read as
    /// unknown (empty, no depth, sample count 0).
    pub fn wrap_render_bundle(&self, bundle: wgpu::RenderBundle, label: Option<&str>) -> GpuRenderBundle {
        GpuRenderBundle::new(bundle, Vec::new(), None, 0, 0, label.map(str::to_string))
    }

    fn new_bundle(&self, color_formats: Vec<wgpu::TextureFormat>, depth_format: Option<wgpu::TextureFormat>, sample_count: u32, depth_read_only: bool, stencil_read_only: bool, label: Option<&str>) -> RenderBundleBuilder {
        // A format without a stencil aspect is stencil-read-only in every pass (the browser's
        // pass says so), and a bundle must agree with the pass to run in it.
        let stencil_read_only = stencil_read_only || !depth_format.is_some_and(TextureFormats::has_stencil);
        RenderBundleBuilder { context: self.clone(), color_formats, depth_format, sample_count: sample_count.max(1), depth_read_only, stencil_read_only, label: label.map(str::to_string), commands: Vec::new() }
    }
}

impl RenderBundleBuilder {
    /// The context (C#: `Context`).
    pub fn context(&self) -> &GraphicsContext {
        &self.context
    }

    /// The colour formats of the passes the bundle will run in (C#: `ColorFormats`).
    pub fn color_formats(&self) -> &[wgpu::TextureFormat] {
        &self.color_formats
    }

    /// The depth format, or None (C#: `DepthFormat`).
    pub fn depth_format(&self) -> Option<wgpu::TextureFormat> {
        self.depth_format
    }

    /// The sample count (C#: `SampleCount`).
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// True when the bundle never writes depth (C#: `DepthReadOnly`).
    pub fn depth_read_only(&self) -> bool {
        self.depth_read_only
    }

    /// True when the bundle never writes stencil (C#: `StencilReadOnly`); always true for a depth
    /// format without a stencil aspect, or none, as the browser's passes have it.
    pub fn stencil_read_only(&self) -> bool {
        self.stencil_read_only
    }

    /// The number of commands recorded so far (C#: `CommandCount`).
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Sets the pipeline (C#: `SetPipeline`). Its target formats must be the bundle's: the error
    /// says which differ.
    pub fn set_pipeline(&mut self, pipeline: &GpuRenderPipeline) -> Result<(), GraphicsException> {
        if pipeline.color_formats() != self.color_formats.as_slice() || pipeline.depth_format() != self.depth_format || pipeline.sample_count().max(1) != self.sample_count {
            return Err(GraphicsException::new(format!(
                "The pipeline{} draws into {:?} / depth {:?} / {} sample(s), but the bundle{} is for {:?} / depth {:?} / {} sample(s); build the pipeline for the bundle's target (or start the bundle with create_render_bundle_for_pipeline).",
                label_text(pipeline.label()),
                pipeline.color_formats(),
                pipeline.depth_format(),
                pipeline.sample_count(),
                label_text(self.label.as_deref()),
                self.color_formats,
                self.depth_format,
                self.sample_count
            )));
        }
        self.commands.push(BundleCommand::SetPipeline(pipeline.handle().clone()));
        Ok(())
    }

    /// Sets bind group `index` (C#: `SetBindGroup(index, group)`).
    pub fn set_bind_group(&mut self, index: u32, group: &GpuBindGroup) {
        self.commands.push(BundleCommand::SetBindGroup(index, group.handle().clone(), Vec::new()));
    }

    /// Sets bind group `index` with dynamic offsets (C#: `SetBindGroup(index, group, offsets)`).
    pub fn set_bind_group_with_offsets(&mut self, index: u32, group: &GpuBindGroup, dynamic_offsets: &[u32]) {
        self.commands.push(BundleCommand::SetBindGroup(index, group.handle().clone(), dynamic_offsets.to_vec()));
    }

    /// Sets the vertex buffer of `slot` (C#: `SetVertexBuffer`).
    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: &GpuBuffer, range: BufferRange) {
        self.commands.push(BundleCommand::SetVertexBuffer(slot, buffer.handle().clone(), range));
    }

    /// Sets the index buffer (C#: `SetIndexBuffer`).
    pub fn set_index_buffer(&mut self, buffer: &GpuBuffer, format: wgpu::IndexFormat, range: BufferRange) {
        self.commands.push(BundleCommand::SetIndexBuffer(buffer.handle().clone(), format, range));
    }

    /// Draws vertices (C#: `Draw`).
    pub fn draw(&mut self, vertex_count: u32, options: DrawOptions) {
        let (v, i) = (options.first_vertex, options.first_instance);
        self.commands.push(BundleCommand::Draw(v..v + vertex_count, i..i + options.instance_count));
    }

    /// Draws indexed vertices (C#: `DrawIndexed`).
    pub fn draw_indexed(&mut self, index_count: u32, options: DrawIndexedOptions) {
        let (f, i) = (options.first_index, options.first_instance);
        self.commands.push(BundleCommand::DrawIndexed(f..f + index_count, options.base_vertex, i..i + options.instance_count));
    }

    /// Draws with arguments read from a buffer (one [`DrawIndirectArgs`]) (C#: `DrawIndirect`).
    pub fn draw_indirect(&mut self, indirect_buffer: &GpuBuffer, offset: u64) -> Result<(), GraphicsException> {
        check_indirect(indirect_buffer, offset, DrawIndirectArgs::SIZE, 1)?;
        self.commands.push(BundleCommand::DrawIndirect(indirect_buffer.handle().clone(), offset));
        Ok(())
    }

    /// Draws indexed vertices with arguments read from a buffer (one [`DrawIndexedIndirectArgs`])
    /// (C#: `DrawIndexedIndirect`).
    pub fn draw_indexed_indirect(&mut self, indirect_buffer: &GpuBuffer, offset: u64) -> Result<(), GraphicsException> {
        check_indirect(indirect_buffer, offset, DrawIndexedIndirectArgs::SIZE, 1)?;
        self.commands.push(BundleCommand::DrawIndexedIndirect(indirect_buffer.handle().clone(), offset));
        Ok(())
    }

    /// Accepted for symmetry with passes; dropped inside bundles (C#: `PushDebugGroup`, dropped too).
    pub fn push_debug_group(&mut self, _label: &str) {}

    /// Accepted for symmetry with passes; dropped inside bundles (C#: `PopDebugGroup`).
    pub fn pop_debug_group(&mut self) {}

    /// Accepted for symmetry with passes; dropped inside bundles (C#: `InsertDebugMarker`).
    pub fn insert_debug_marker(&mut self, _label: &str) {}

    /// Finishes the bundle (C#: `Finish(label)`; None keeps the builder's label). WebGPU's
    /// verdict on the commands arrives through `take_errors`; await
    /// [`RenderBundleBuilder::finish_checked`] where it must be known.
    pub fn finish(self, label: Option<&str>) -> GpuRenderBundle {
        let label = label.map(str::to_string).or(self.label.clone());
        let color_formats: Vec<Option<wgpu::TextureFormat>> = self.color_formats.iter().map(|format| Some(*format)).collect();
        let depth_stencil = self.depth_format.map(|format| wgpu::RenderBundleDepthStencil { format, depth_read_only: self.depth_read_only, stencil_read_only: self.stencil_read_only });
        let mut encoder = self.context.device().create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor { label: label.as_deref(), color_formats: &color_formats, depth_stencil, sample_count: self.sample_count, multiview: None });
        for command in &self.commands {
            match command {
                BundleCommand::SetPipeline(pipeline) => encoder.set_pipeline(pipeline),
                BundleCommand::SetBindGroup(index, group, offsets) => encoder.set_bind_group(*index, group, offsets),
                BundleCommand::SetVertexBuffer(slot, buffer, range) => encoder.set_vertex_buffer(*slot, slice(buffer, *range)),
                BundleCommand::SetIndexBuffer(buffer, format, range) => encoder.set_index_buffer(slice(buffer, *range), *format),
                BundleCommand::Draw(vertices, instances) => encoder.draw(vertices.clone(), instances.clone()),
                BundleCommand::DrawIndexed(indices, base_vertex, instances) => encoder.draw_indexed(indices.clone(), *base_vertex, instances.clone()),
                BundleCommand::DrawIndirect(buffer, offset) => encoder.draw_indirect(buffer, *offset),
                BundleCommand::DrawIndexedIndirect(buffer, offset) => encoder.draw_indexed_indirect(buffer, *offset),
            }
        }
        let handle = encoder.finish(&wgpu::RenderBundleDescriptor { label: label.as_deref() });
        GpuRenderBundle::new(handle, self.color_formats.clone(), self.depth_format, self.sample_count, self.commands.len(), label)
    }

    /// As [`RenderBundleBuilder::finish`], and waits for WebGPU's verdict: Err carries WebGPU's
    /// validation message (C#: what `Finish` throws).
    pub async fn finish_checked(self, label: Option<&str>) -> Result<GpuRenderBundle, GraphicsException> {
        let context = self.context.clone();
        let what = format!("The render bundle{}", label_text(label.or(self.label.as_deref())));
        let label = label.map(str::to_string);
        context.validate_named(&what, move |_| self.finish(label.as_deref())).await
    }
}

/// The slice of `buffer` a range names.
fn slice(buffer: &wgpu::Buffer, range: BufferRange) -> wgpu::BufferSlice<'_> {
    match range.size {
        Some(size) => buffer.slice(range.offset..range.offset + size),
        None => buffer.slice(range.offset..),
    }
}
