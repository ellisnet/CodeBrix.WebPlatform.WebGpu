//! [`RenderPass`]: an open render pass - pipeline, bind groups, vertex and index buffers, draws,
//! indirect draws, bundles, blend constant and stencil reference, occlusion queries, debug
//! groups, viewport and scissor (C#: `RenderPass`), with the options structs for the C# default
//! arguments.

use crate::bundles::GpuRenderBundle;
use crate::common::Color4;
use crate::context::GraphicsException;
use crate::frames::indirect_args::check_indirect;
use crate::frames::pass_attachments::pipeline_mismatch;
use crate::frames::{DrawIndexedIndirectArgs, DrawIndirectArgs};
use crate::pipelines::GpuRenderPipeline;
use crate::presentation::RenderTargetView;
use crate::resources::{GpuBindGroup, GpuBuffer};

/// A byte range of a buffer (C#: the default arguments `offset = 0`, `size = WholeSize`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BufferRange {
    /// The first byte.
    pub offset: u64,
    /// The number of bytes; None for the rest of the buffer.
    pub size: Option<u64>,
}

/// The defaulted arguments of [`RenderPass::draw`] (C#: `instanceCount = 1`, `firstVertex = 0`,
/// `firstInstance = 0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawOptions {
    /// The number of instances.
    pub instance_count: u32,
    /// The first vertex.
    pub first_vertex: u32,
    /// The first instance.
    pub first_instance: u32,
}

impl Default for DrawOptions {
    fn default() -> DrawOptions {
        DrawOptions { instance_count: 1, first_vertex: 0, first_instance: 0 }
    }
}

/// The defaulted arguments of [`RenderPass::draw_indexed`] (C#: `instanceCount = 1`,
/// `firstIndex = 0`, `baseVertex = 0`, `firstInstance = 0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawIndexedOptions {
    /// The number of instances.
    pub instance_count: u32,
    /// The first index.
    pub first_index: u32,
    /// The value added to every index.
    pub base_vertex: i32,
    /// The first instance.
    pub first_instance: u32,
}

impl Default for DrawIndexedOptions {
    fn default() -> DrawIndexedOptions {
        DrawIndexedOptions { instance_count: 1, first_index: 0, base_vertex: 0, first_instance: 0 }
    }
}

/// The depth range of [`RenderPass::set_viewport`] (C#: `minDepth = 0`, `maxDepth = 1`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthRange {
    /// The near depth.
    pub min_depth: f32,
    /// The far depth.
    pub max_depth: f32,
}

impl Default for DepthRange {
    fn default() -> DepthRange {
        DepthRange { min_depth: 0.0, max_depth: 1.0 }
    }
}

/// An open render pass of a [`GraphicsFrame`](crate::GraphicsFrame) (C#: `RenderPass`). It ends
/// when it is dropped or [`RenderPass::end`]ed (C#: `End` / `Dispose`).
pub struct RenderPass<'a> {
    pass: wgpu::RenderPass<'a>,
    target: RenderTargetView,
    frame_id: u64,
}

impl<'a> RenderPass<'a> {
    pub(crate) fn new(pass: wgpu::RenderPass<'a>, target: RenderTargetView, frame_id: u64) -> RenderPass<'a> {
        RenderPass { pass, target, frame_id }
    }

    /// The number of the frame the pass belongs to (C#: `Frame.Id`): the batching renderers
    /// (SpriteBatch, MeshRenderer) start their buffers over when it changes.
    pub fn frame_id(&self) -> u64 {
        self.frame_id
    }

    /// The target the pass draws into (C#: `Target`).
    pub fn target(&self) -> &RenderTargetView {
        &self.target
    }

    /// The `wgpu` render pass, for commands of the caller's own (C#: `Handle`).
    pub fn handle(&mut self) -> &mut wgpu::RenderPass<'a> {
        &mut self.pass
    }

    /// Sets the pipeline (C#: `SetPipeline`). The pipeline must agree with the pass's target
    /// ([`RenderTargetView::formats`]: colour format, depth format and sample count - what WebGPU
    /// requires): a pipeline that does not PANICS here with a message saying what differs and how
    /// to make the right one, instead of the pass failing WebGPU's validation later with the whole
    /// frame lost. The library's renderers key their pipelines by the target, so they never hit
    /// it; a pipeline of the application's own on a target whose sample count can change (a
    /// [`crate::CanvasFrameTarget`]) needs one per count. [`RenderPass::try_set_pipeline`]
    /// returns the message instead.
    pub fn set_pipeline(&mut self, pipeline: &GpuRenderPipeline) {
        if let Err(error) = self.try_set_pipeline(pipeline) {
            panic!("{error}");
        }
    }

    /// Sets the pipeline if it agrees with the pass's target (Not in C#; see
    /// [`RenderPass::set_pipeline`]); Err with what differs, and nothing set, if it does not.
    pub fn try_set_pipeline(&mut self, pipeline: &GpuRenderPipeline) -> Result<(), GraphicsException> {
        self.check_pipeline(pipeline)?;
        self.pass.set_pipeline(pipeline.handle());
        Ok(())
    }

    /// Ok when `pipeline` agrees with the pass's target - colour format, depth format and sample
    /// count (Not in C#); Err saying what differs.
    pub fn check_pipeline(&self, pipeline: &GpuRenderPipeline) -> Result<(), GraphicsException> {
        match pipeline_mismatch(pipeline.color_formats(), pipeline.depth_format(), pipeline.sample_count(), pipeline.label(), self.target.formats(), &self.target.to_string()) {
            Some(message) => Err(GraphicsException::new(message)),
            None => Ok(()),
        }
    }

    /// Sets bind group `index` (C#: `SetBindGroup(index, group)`).
    pub fn set_bind_group(&mut self, index: u32, group: &GpuBindGroup) {
        self.pass.set_bind_group(index, group.handle(), &[]);
    }

    /// Sets bind group `index` with dynamic offsets (C#: `SetBindGroup(index, group, dynamicOffsets)`).
    pub fn set_bind_group_with_offsets(&mut self, index: u32, group: &GpuBindGroup, dynamic_offsets: &[u32]) {
        self.pass.set_bind_group(index, group.handle(), dynamic_offsets);
    }

    /// Sets the vertex buffer of `slot` (C#: `SetVertexBuffer(slot, buffer, offset, size)`).
    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: &GpuBuffer, range: BufferRange) {
        self.pass.set_vertex_buffer(slot, slice(buffer, range));
    }

    /// Sets the index buffer (C#: `SetIndexBuffer(buffer, format, offset, size)`).
    pub fn set_index_buffer(&mut self, buffer: &GpuBuffer, format: wgpu::IndexFormat, range: BufferRange) {
        self.pass.set_index_buffer(slice(buffer, range), format);
    }

    /// Draws vertices (C#: `Draw(vertexCount, instanceCount, firstVertex, firstInstance)`).
    pub fn draw(&mut self, vertex_count: u32, options: DrawOptions) {
        let first_vertex = options.first_vertex;
        let first_instance = options.first_instance;
        self.pass.draw(first_vertex..first_vertex + vertex_count, first_instance..first_instance + options.instance_count);
    }

    /// Draws indexed vertices (C#: `DrawIndexed(indexCount, instanceCount, firstIndex, baseVertex,
    /// firstInstance)`).
    pub fn draw_indexed(&mut self, index_count: u32, options: DrawIndexedOptions) {
        let first_index = options.first_index;
        let first_instance = options.first_instance;
        self.pass.draw_indexed(first_index..first_index + index_count, options.base_vertex, first_instance..first_instance + options.instance_count);
    }

    /// Sets the viewport (C#: `SetViewport`).
    pub fn set_viewport(&mut self, x: f32, y: f32, width: f32, height: f32, depth: DepthRange) {
        self.pass.set_viewport(x, y, width, height, depth.min_depth, depth.max_depth);
    }

    /// Sets the scissor rectangle (C#: `SetScissorRect`).
    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.pass.set_scissor_rect(x, y, width, height);
    }

    /// Draws with arguments the GPU reads from a buffer (one [`DrawIndirectArgs`] at `offset`)
    /// (C#: `DrawIndirect`). The buffer needs `INDIRECT` (`GraphicsContext::create_indirect_buffer`).
    pub fn draw_indirect(&mut self, indirect_buffer: &GpuBuffer, offset: u64) -> Result<(), GraphicsException> {
        check_indirect(indirect_buffer, offset, DrawIndirectArgs::SIZE, 1)?;
        self.pass.draw_indirect(indirect_buffer.handle(), offset);
        Ok(())
    }

    /// Draws indexed vertices with arguments read from a buffer (one [`DrawIndexedIndirectArgs`])
    /// (C#: `DrawIndexedIndirect`).
    pub fn draw_indexed_indirect(&mut self, indirect_buffer: &GpuBuffer, offset: u64) -> Result<(), GraphicsException> {
        check_indirect(indirect_buffer, offset, DrawIndexedIndirectArgs::SIZE, 1)?;
        self.pass.draw_indexed_indirect(indirect_buffer.handle(), offset);
        Ok(())
    }

    /// Issues `count` indirect draws from consecutive [`DrawIndirectArgs`] records (C#:
    /// `MultiDrawIndirect`; in a browser this is `count` single indirect draws).
    pub fn multi_draw_indirect(&mut self, indirect_buffer: &GpuBuffer, offset: u64, count: u32) -> Result<(), GraphicsException> {
        check_indirect(indirect_buffer, offset, DrawIndirectArgs::SIZE, count)?;
        self.pass.multi_draw_indirect(indirect_buffer.handle(), offset, count);
        Ok(())
    }

    /// Issues `count` indexed indirect draws from consecutive [`DrawIndexedIndirectArgs`] records
    /// (C#: `MultiDrawIndexedIndirect`).
    pub fn multi_draw_indexed_indirect(&mut self, indirect_buffer: &GpuBuffer, offset: u64, count: u32) -> Result<(), GraphicsException> {
        check_indirect(indirect_buffer, offset, DrawIndexedIndirectArgs::SIZE, count)?;
        self.pass.multi_draw_indexed_indirect(indirect_buffer.handle(), offset, count);
        Ok(())
    }

    /// Replays a recorded render bundle (C#: `ExecuteBundle`); its formats must be the pass
    /// target's. The pass's pipeline, bind groups and buffers are unset afterwards. A bundle
    /// recorded for another colour format, depth format or SAMPLE COUNT panics here with a message
    /// saying what differs (as [`RenderPass::set_pipeline`]); a wrapped bundle (formats unknown)
    /// is not checked.
    pub fn execute_bundle(&mut self, bundle: &GpuRenderBundle) {
        self.check_bundle(bundle);
        self.pass.execute_bundles(std::iter::once(bundle.handle()));
    }

    /// Replays render bundles in order (C#: `ExecuteBundles`); each is checked as in
    /// [`RenderPass::execute_bundle`].
    pub fn execute_bundles(&mut self, bundles: &[&GpuRenderBundle]) {
        for bundle in bundles {
            self.check_bundle(bundle);
        }
        self.pass.execute_bundles(bundles.iter().map(|bundle| bundle.handle()));
    }

    fn check_bundle(&self, bundle: &GpuRenderBundle) {
        if bundle.sample_count() == 0 {
            return;
        }
        if let Some(message) = pipeline_mismatch(bundle.color_formats(), bundle.depth_format(), bundle.sample_count(), bundle.label(), self.target.formats(), &self.target.to_string()) {
            panic!("{}", message.replacen("The render pipeline", "The render bundle", 1).replace("Make the pipeline with .with_sample_count(pass.target().sample_count())", "Record the bundle with the pass target's sample count (create_render_bundle(target, target.sample_count(), ..))"));
        }
    }

    /// Sets the constant colour of the `Constant` blend factors (C#: `SetBlendConstant`).
    pub fn set_blend_constant(&mut self, color: Color4) {
        self.pass.set_blend_constant(color.to_wgpu());
    }

    /// Sets the stencil reference value (C#: `SetStencilReference`).
    pub fn set_stencil_reference(&mut self, reference: u32) {
        self.pass.set_stencil_reference(reference);
    }

    /// Starts counting the samples that pass the depth test into query `query_index` of the pass's
    /// `RenderPassOptions::occlusion_query_set` (C#: `BeginOcclusionQuery`). NOT USABLE with this
    /// version of `wgpu` in a browser (its WebGPU backend drops the pass's occlusion query set, so
    /// the pass fails validation): see `GraphicsContext::create_query_set`.
    pub fn begin_occlusion_query(&mut self, query_index: u32) {
        self.pass.begin_occlusion_query(query_index);
    }

    /// Stops the current occlusion query (C#: `EndOcclusionQuery`).
    pub fn end_occlusion_query(&mut self) {
        self.pass.end_occlusion_query();
    }

    /// Opens a named debug group around the following commands (C#: `PushDebugGroup`).
    pub fn push_debug_group(&mut self, label: &str) {
        self.pass.push_debug_group(label);
    }

    /// Closes the innermost debug group (C#: `PopDebugGroup`).
    pub fn pop_debug_group(&mut self) {
        self.pass.pop_debug_group();
    }

    /// Inserts a named marker between commands (C#: `InsertDebugMarker`).
    pub fn insert_debug_marker(&mut self, label: &str) {
        self.pass.insert_debug_marker(label);
    }

    /// Ends the pass (C#: `End`); dropping it does the same.
    pub fn end(self) {}
}

fn slice(buffer: &GpuBuffer, range: BufferRange) -> wgpu::BufferSlice<'_> {
    match range.size {
        Some(size) => buffer.handle().slice(range.offset..range.offset + size),
        None => buffer.handle().slice(range.offset..),
    }
}

#[cfg(test)]
mod tests {
    use super::{BufferRange, DepthRange, DrawIndexedOptions, DrawOptions};

    #[test]
    fn the_options_carry_the_csharp_defaults() {
        assert_eq!(DrawOptions::default(), DrawOptions { instance_count: 1, first_vertex: 0, first_instance: 0 });
        assert_eq!(DrawIndexedOptions::default().instance_count, 1);
        assert_eq!(BufferRange::default(), BufferRange { offset: 0, size: None });
        assert_eq!(DepthRange::default().max_depth, 1.0);
    }
}
