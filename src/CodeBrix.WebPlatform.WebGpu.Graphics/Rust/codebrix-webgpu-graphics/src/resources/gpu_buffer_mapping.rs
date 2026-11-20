//! Mapping of [`GpuBuffer`]s (C#: `GpuBuffer.Mapping.cs` - `MapAsync`, `ReadMapped`,
//! `WriteMapped`, `Unmap`, `MapState`), and the context's mapped, initialised and indirect buffer
//! factories (C#: `GraphicsContext.MoreResources.cs`).

use crate::context::{label_text, GraphicsContext, GraphicsException};
use crate::internal::oneshot;
use crate::resources::BufferMapState;
use crate::resources::GpuBuffer;

impl GpuBuffer {
    /// Where the buffer's mapping stands (C#: `MapState`).
    pub fn map_state(&self) -> BufferMapState {
        self.map_state_cell().get()
    }

    /// Maps `size` bytes from `offset` (size None: to the end) for reading or writing, once the GPU
    /// is done with the buffer (C#: `MapAsync(mode, offset, size)`; C#'s synchronous `Map` has no
    /// counterpart - a browser cannot wait). The buffer needs `MAP_READ` or `MAP_WRITE`
    /// (`create_buffer` with that usage, or `create_mapped_buffer`). Then
    /// [`GpuBuffer::read_mapped`] / [`GpuBuffer::write_mapped`], and [`GpuBuffer::unmap`] before
    /// the GPU uses it again.
    pub async fn map_async(&self, mode: wgpu::MapMode, offset: u64, size: Option<u64>) -> Result<(), GraphicsException> {
        let needed = match mode {
            wgpu::MapMode::Read => wgpu::BufferUsages::MAP_READ,
            wgpu::MapMode::Write => wgpu::BufferUsages::MAP_WRITE,
        };
        if !self.usage().contains(needed) {
            return Err(GraphicsException::new(format!("The buffer{} was not created with {needed:?}, so it cannot be mapped that way; create it with that usage.", label_text(self.label()))));
        }
        if self.map_state() != BufferMapState::Unmapped {
            return Err(GraphicsException::new(format!("The buffer{} is already mapped or being mapped; unmap it first.", label_text(self.label()))));
        }
        let end = match size {
            Some(size) => offset + size,
            None => self.size(),
        };
        if offset % 8 != 0 || end > self.size() || end < offset {
            return Err(GraphicsException::new(format!("The mapped range {offset}..{end} must start at a multiple of 8 inside the {}-byte buffer{}.", self.size(), label_text(self.label()))));
        }
        let state = self.map_state_cell().clone();
        state.set(BufferMapState::Pending);
        let (sender, receiver) = oneshot::channel();
        self.handle().map_async(mode, offset..end, move |result| sender.send(result));
        match receiver.await {
            Ok(()) => {
                state.set(BufferMapState::Mapped);
                Ok(())
            }
            Err(error) => {
                state.set(BufferMapState::Unmapped);
                Err(GraphicsException::new(format!("The buffer{} could not be mapped ({error}); the device may be lost - check GraphicsContext::lost_reason.", label_text(self.label()))))
            }
        }
    }

    /// Copies values out of the mapped range, starting `byte_offset` bytes into the buffer (C#:
    /// `ReadMapped<T>(destination, byteOffset)`). The buffer must be mapped for reading.
    pub fn read_mapped<T: bytemuck::Pod>(&self, destination: &mut [T], byte_offset: u64) -> Result<(), GraphicsException> {
        let bytes: &mut [u8] = bytemuck::cast_slice_mut(destination);
        self.check_mapped(byte_offset, bytes.len() as u64)?;
        let view = self.handle().get_mapped_range(byte_offset..byte_offset + bytes.len() as u64).map_err(|error| GraphicsException::new(format!("The mapped range of the buffer{} could not be read ({error}); map the range for reading first.", label_text(self.label()))))?;
        bytes.copy_from_slice(&view);
        Ok(())
    }

    /// Copies values into the mapped range at `byte_offset` (C#: `WriteMapped<T>(data,
    /// byteOffset)`). The buffer must be mapped for writing (or made by `create_mapped_buffer`).
    pub fn write_mapped<T: bytemuck::Pod>(&self, data: &[T], byte_offset: u64) -> Result<(), GraphicsException> {
        let bytes: &[u8] = bytemuck::cast_slice(data);
        self.check_mapped(byte_offset, bytes.len() as u64)?;
        let mut view = self.handle().get_mapped_range_mut(byte_offset..byte_offset + bytes.len() as u64).map_err(|error| GraphicsException::new(format!("The mapped range of the buffer{} could not be written ({error}); map the range for writing first.", label_text(self.label()))))?;
        view.copy_from_slice(bytes);
        Ok(())
    }

    /// Ends the mapping so that the GPU may use the buffer (C#: `Unmap`).
    pub fn unmap(&self) {
        if self.map_state() == BufferMapState::Mapped {
            self.handle().unmap();
        }
        self.map_state_cell().set(BufferMapState::Unmapped);
    }

    fn check_mapped(&self, byte_offset: u64, length: u64) -> Result<(), GraphicsException> {
        if self.map_state() != BufferMapState::Mapped {
            return Err(GraphicsException::new(format!("The buffer{} is not mapped; await map_async first (or make it with create_mapped_buffer).", label_text(self.label()))));
        }
        if byte_offset % 8 != 0 || length % 4 != 0 || byte_offset + length > self.size() {
            return Err(GraphicsException::new(format!("{length} bytes at offset {byte_offset} are not a mapped range of the {}-byte buffer{}: the offset must be a multiple of 8 and the length of 4.", self.size(), label_text(self.label()))));
        }
        Ok(())
    }
}

impl GraphicsContext {
    /// A buffer that is MAPPED from the start, to fill with [`GpuBuffer::write_mapped`] and then
    /// [`GpuBuffer::unmap`] (C#: `CreateMappedBuffer`). The size rounds up to 4; the copy usages are
    /// added as `create_buffer` adds them.
    pub fn create_mapped_buffer(&self, size: u64, usage: wgpu::BufferUsages, label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        if size == 0 {
            return Err(GraphicsException::new(format!("The buffer{} needs at least one byte; ask for a size of 1 or more.", label_text(label))));
        }
        let size = (size + 3) & !3;
        let usage = crate::context::buffer_usage(usage);
        let handle = self.device().create_buffer(&wgpu::BufferDescriptor { label, size, usage, mapped_at_creation: true });
        Ok(GpuBuffer::new(self.clone(), handle, size, usage, label).mapped())
    }

    /// A buffer holding `data`, written through a mapping at creation (C#:
    /// `CreateInitializedBuffer<T>`; the same result as [`GraphicsContext::create_buffer_from`]).
    pub fn create_initialized_buffer<T: bytemuck::Pod>(&self, data: &[T], usage: wgpu::BufferUsages, label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        self.create_buffer_from(data, usage, label)
    }

    /// An indirect buffer holding argument records ([`crate::DrawIndirectArgs`],
    /// [`crate::DrawIndexedIndirectArgs`] or [`crate::DispatchIndirectArgs`]) (C#:
    /// `CreateIndirectBuffer<T>(arguments)`): `INDIRECT | STORAGE`, so a compute pass can write it.
    pub fn create_indirect_buffer_from<T: bytemuck::Pod>(&self, arguments: &[T], label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        self.create_buffer_from(arguments, wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE, label)
    }

    /// An empty indirect buffer of `size` bytes, `INDIRECT | STORAGE` (C#: `CreateIndirectBuffer(size)`).
    pub fn create_indirect_buffer(&self, size: u64, label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        self.create_buffer(size, wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE, label)
    }
}
