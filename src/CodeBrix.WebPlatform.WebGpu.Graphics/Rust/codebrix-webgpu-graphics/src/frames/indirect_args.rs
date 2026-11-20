//! The argument records of indirect draws and dispatches (C#: `DrawIndirectArgs`,
//! `DrawIndexedIndirectArgs`, `DispatchIndirectArgs`), and the checks the passes and bundles make
//! before they hand an indirect buffer to WebGPU.

use crate::context::GraphicsException;
use crate::resources::GpuBuffer;

/// The arguments of one indirect draw (C#: `DrawIndirectArgs`): 16 bytes, the layout WebGPU reads
/// from an indirect buffer for [`RenderPass::draw_indirect`](crate::RenderPass::draw_indirect).
/// A compute shader may write them (the indirect buffer factories add `STORAGE`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrawIndirectArgs {
    /// The vertex count.
    pub vertex_count: u32,
    /// The instance count.
    pub instance_count: u32,
    /// The first vertex.
    pub first_vertex: u32,
    /// The first instance (must be 0 unless the device has `INDIRECT_FIRST_INSTANCE`).
    pub first_instance: u32,
}

impl DrawIndirectArgs {
    /// The size in bytes (C#: `Size`, 16).
    pub const SIZE: u64 = 16;

    /// Arguments that draw `vertex_count` vertices once (C#: `new DrawIndirectArgs(vertexCount)`;
    /// set the other fields for more).
    pub fn new(vertex_count: u32) -> DrawIndirectArgs {
        DrawIndirectArgs { vertex_count, instance_count: 1, first_vertex: 0, first_instance: 0 }
    }
}

// SAFETY: four u32 fields, repr(C), no padding: every bit pattern is a valid value.
unsafe impl bytemuck::Zeroable for DrawIndirectArgs {}
// SAFETY: as above; the type is Copy and 'static.
unsafe impl bytemuck::Pod for DrawIndirectArgs {}

/// The arguments of one indexed indirect draw (C#: `DrawIndexedIndirectArgs`): 20 bytes, for
/// [`RenderPass::draw_indexed_indirect`](crate::RenderPass::draw_indexed_indirect).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrawIndexedIndirectArgs {
    /// The index count.
    pub index_count: u32,
    /// The instance count.
    pub instance_count: u32,
    /// The first index.
    pub first_index: u32,
    /// The value added to every index.
    pub base_vertex: i32,
    /// The first instance (must be 0 unless the device has `INDIRECT_FIRST_INSTANCE`).
    pub first_instance: u32,
}

impl DrawIndexedIndirectArgs {
    /// The size in bytes (C#: `Size`, 20).
    pub const SIZE: u64 = 20;

    /// Arguments that draw `index_count` indices once (C#: `new DrawIndexedIndirectArgs(indexCount)`).
    pub fn new(index_count: u32) -> DrawIndexedIndirectArgs {
        DrawIndexedIndirectArgs { index_count, instance_count: 1, first_index: 0, base_vertex: 0, first_instance: 0 }
    }
}

// SAFETY: five 32-bit fields, repr(C), no padding: every bit pattern is a valid value.
unsafe impl bytemuck::Zeroable for DrawIndexedIndirectArgs {}
// SAFETY: as above; the type is Copy and 'static.
unsafe impl bytemuck::Pod for DrawIndexedIndirectArgs {}

/// The workgroup counts of one indirect dispatch (C#: `DispatchIndirectArgs`): 12 bytes, for
/// [`ComputePass::dispatch_indirect`](crate::ComputePass::dispatch_indirect).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DispatchIndirectArgs {
    /// Workgroups in x.
    pub x: u32,
    /// Workgroups in y.
    pub y: u32,
    /// Workgroups in z.
    pub z: u32,
}

impl DispatchIndirectArgs {
    /// The size in bytes (C#: `Size`, 12).
    pub const SIZE: u64 = 12;

    /// The counts (C#: `new DispatchIndirectArgs(x, y, z)`; C#'s defaults are y = 1, z = 1).
    pub fn new(x: u32, y: u32, z: u32) -> DispatchIndirectArgs {
        DispatchIndirectArgs { x, y, z }
    }
}

// SAFETY: three u32 fields, repr(C), no padding: every bit pattern is a valid value.
unsafe impl bytemuck::Zeroable for DispatchIndirectArgs {}
// SAFETY: as above; the type is Copy and 'static.
unsafe impl bytemuck::Pod for DispatchIndirectArgs {}

/// Checks that `buffer` can hold `count` records of `stride` bytes at `offset` (C#:
/// `IndirectChecks.Check`).
pub(crate) fn check_indirect(buffer: &GpuBuffer, offset: u64, stride: u64, count: u32) -> Result<(), GraphicsException> {
    check_indirect_parts(buffer.usage(), buffer.size(), offset, stride, count)
}

/// The arithmetic of [`check_indirect`], on the buffer's usage and size.
pub(crate) fn check_indirect_parts(usage: wgpu::BufferUsages, size: u64, offset: u64, stride: u64, count: u32) -> Result<(), GraphicsException> {
    if !usage.contains(wgpu::BufferUsages::INDIRECT) {
        return Err(GraphicsException::new("The buffer needs the INDIRECT usage; make it with GraphicsContext::create_indirect_buffer (or _from)."));
    }
    if offset % 4 != 0 {
        return Err(GraphicsException::new(format!("The indirect offset {offset} must be a multiple of 4.")));
    }
    if offset + stride * count as u64 > size {
        return Err(GraphicsException::new(format!("{count} record(s) of {stride} bytes at offset {offset} overrun the {size}-byte buffer; make the buffer larger or read fewer records.")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{check_indirect_parts, DispatchIndirectArgs, DrawIndexedIndirectArgs, DrawIndirectArgs};

    #[test]
    fn the_records_have_the_sizes_webgpu_reads() {
        assert_eq!(std::mem::size_of::<DrawIndirectArgs>() as u64, DrawIndirectArgs::SIZE);
        assert_eq!(std::mem::size_of::<DrawIndexedIndirectArgs>() as u64, DrawIndexedIndirectArgs::SIZE);
        assert_eq!(std::mem::size_of::<DispatchIndirectArgs>() as u64, DispatchIndirectArgs::SIZE);
    }

    #[test]
    fn new_uses_the_csharp_defaults() {
        assert_eq!(DrawIndirectArgs::new(6), DrawIndirectArgs { vertex_count: 6, instance_count: 1, first_vertex: 0, first_instance: 0 });
        assert_eq!(DrawIndexedIndirectArgs::new(36).instance_count, 1);
        assert_eq!(bytemuck::bytes_of(&DispatchIndirectArgs::new(2, 1, 1)), &[2, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0]);
    }

    #[test]
    fn check_indirect_refuses_what_webgpu_would_refuse() {
        let usage = wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE;
        assert!(check_indirect_parts(usage, 32, 16, 16, 1).is_ok());
        assert!(check_indirect_parts(usage, 32, 0, 16, 2).is_ok());
        assert!(check_indirect_parts(usage, 32, 2, 16, 1).unwrap_err().message().contains("multiple of 4"));
        assert!(check_indirect_parts(usage, 32, 20, 16, 1).unwrap_err().message().contains("overrun"));
        assert!(check_indirect_parts(wgpu::BufferUsages::STORAGE, 32, 0, 16, 1).unwrap_err().message().contains("INDIRECT"));
    }
}
