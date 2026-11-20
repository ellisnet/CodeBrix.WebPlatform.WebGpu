//! [`ComputePass`]: an open compute pass - pipeline, bind groups, dispatches (C#: `ComputePass`),
//! and [`ComputePassOptions`] (C#: `ComputePassOptions`).

use crate::context::GraphicsException;
use crate::frames::indirect_args::{check_indirect, DispatchIndirectArgs};
use crate::pipelines::GpuComputePipeline;
use crate::queries::PassTimestamps;
use crate::resources::{GpuBindGroup, GpuBuffer};

/// The settings of a compute pass (C#: `ComputePassOptions`).
#[derive(Clone, Debug, Default)]
pub struct ComputePassOptions {
    /// The pass's label (C#: `Label`).
    pub label: Option<String>,
    /// Timestamps written when the pass begins and ends (C#: `Timestamps`); needs a timestamp
    /// query set (the device feature `TIMESTAMP_QUERY`).
    pub timestamps: Option<PassTimestamps>,
}

/// An open compute pass of a [`GraphicsFrame`](crate::GraphicsFrame) (C#: `ComputePass`), begun
/// with [`GraphicsFrame::begin_compute_pass`](crate::GraphicsFrame::begin_compute_pass). It ends
/// when it is dropped or [`ComputePass::end`]ed. Work a compute pass writes (a storage buffer, an
/// indirect buffer) is seen by the passes recorded after it in the same frame.
pub struct ComputePass<'a> {
    pass: wgpu::ComputePass<'a>,
}

impl<'a> ComputePass<'a> {
    pub(crate) fn new(pass: wgpu::ComputePass<'a>) -> ComputePass<'a> {
        ComputePass { pass }
    }

    /// The `wgpu` compute pass, for commands of the caller's own (C#: `Handle`).
    pub fn handle(&mut self) -> &mut wgpu::ComputePass<'a> {
        &mut self.pass
    }

    /// Sets the pipeline (C#: `SetPipeline`).
    pub fn set_pipeline(&mut self, pipeline: &GpuComputePipeline) {
        self.pass.set_pipeline(pipeline.handle());
    }

    /// Sets bind group `index` (C#: `SetBindGroup(index, group)`).
    pub fn set_bind_group(&mut self, index: u32, group: &GpuBindGroup) {
        self.pass.set_bind_group(index, group.handle(), &[]);
    }

    /// Sets bind group `index` with dynamic offsets (C#: `SetBindGroup(index, group, dynamicOffsets)`).
    pub fn set_bind_group_with_offsets(&mut self, index: u32, group: &GpuBindGroup, dynamic_offsets: &[u32]) {
        self.pass.set_bind_group(index, group.handle(), dynamic_offsets);
    }

    /// Dispatches `x` x `y` x `z` workgroups (C#: `Dispatch(x, y = 1, z = 1)`).
    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        self.pass.dispatch_workgroups(x, y, z);
    }

    /// Dispatches with workgroup counts the GPU reads from a buffer (one [`DispatchIndirectArgs`]
    /// at `offset`) (C#: `DispatchIndirect`). The buffer needs `INDIRECT`
    /// (`GraphicsContext::create_indirect_buffer`).
    pub fn dispatch_indirect(&mut self, indirect_buffer: &GpuBuffer, offset: u64) -> Result<(), GraphicsException> {
        check_indirect(indirect_buffer, offset, DispatchIndirectArgs::SIZE, 1)?;
        self.pass.dispatch_workgroups_indirect(indirect_buffer.handle(), offset);
        Ok(())
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
