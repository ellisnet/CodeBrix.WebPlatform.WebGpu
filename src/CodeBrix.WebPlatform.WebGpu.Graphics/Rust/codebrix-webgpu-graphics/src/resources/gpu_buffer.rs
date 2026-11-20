//! [`GpuBuffer`]: a WebGPU buffer with its size and usages, written from plain values and read
//! back asynchronously (C#: `GpuBuffer`).

use std::cell::Cell;
use std::rc::Rc;

use crate::context::label_text;
use crate::context::{GraphicsContext, GraphicsException};
use crate::internal::oneshot;

/// A WebGPU buffer (C#: `GpuBuffer`). Made by the context's `create_*_buffer` factories; dropped
/// to release it (C#: `Dispose`).
pub struct GpuBuffer {
    context: GraphicsContext,
    handle: wgpu::Buffer,
    size: u64,
    usage: wgpu::BufferUsages,
    label: Option<String>,
    map_state: Rc<Cell<BufferMapState>>,
}

/// Where a buffer's mapping stands (C#: `BufferMapState`, WebGPU's `mapState`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferMapState {
    /// Not mapped: the GPU may use it.
    Unmapped,
    /// A [`GpuBuffer::map_async`] waits for the GPU.
    Pending,
    /// Mapped: the CPU may read or write the mapped range; unmap it before the GPU uses it.
    Mapped,
}

impl GpuBuffer {
    pub(crate) fn new(context: GraphicsContext, handle: wgpu::Buffer, size: u64, usage: wgpu::BufferUsages, label: Option<&str>) -> GpuBuffer {
        GpuBuffer { context, handle, size, usage, label: label.map(str::to_string), map_state: Rc::new(Cell::new(BufferMapState::Unmapped)) }
    }

    /// The same buffer, recorded as mapped (a buffer made mapped at creation).
    pub(crate) fn mapped(self) -> GpuBuffer {
        self.map_state.set(BufferMapState::Mapped);
        self
    }

    /// The shared mapping state.
    pub(crate) fn map_state_cell(&self) -> &Rc<Cell<BufferMapState>> {
        &self.map_state
    }

    /// The `wgpu` buffer (C#: `Handle`).
    pub fn handle(&self) -> &wgpu::Buffer {
        &self.handle
    }

    /// The size in bytes (a multiple of 4) (C#: `Size`).
    pub fn size(&self) -> u64 {
        self.size
    }

    /// The usages, including the copy usages the factory added (C#: `Usage`).
    pub fn usage(&self) -> wgpu::BufferUsages {
        self.usage
    }

    /// The label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The context the buffer belongs to (C#: `Context`).
    pub fn context(&self) -> &GraphicsContext {
        &self.context
    }

    /// Writes plain values at `byte_offset` (a multiple of 4) through the queue (C#:
    /// `Write<T>(data, byteOffset)`). Data whose length is not a multiple of 4 is padded with
    /// zero bytes. Nothing is written for empty data.
    pub fn write<T: bytemuck::Pod>(&self, data: &[T], byte_offset: u64) -> Result<(), GraphicsException> {
        let bytes: &[u8] = bytemuck::cast_slice(data);
        if bytes.is_empty() {
            return Ok(());
        }
        let padded = check_write(bytes.len() as u64, byte_offset, self.size)?;
        if padded == bytes.len() as u64 {
            self.context.queue().write_buffer(&self.handle, byte_offset, bytes);
        } else {
            let mut copy = bytes.to_vec();
            copy.resize(padded as usize, 0);
            self.context.queue().write_buffer(&self.handle, byte_offset, &copy);
        }
        Ok(())
    }

    /// Writes one plain value at `byte_offset` (C#: `WriteValue<T>(in value, byteOffset)`).
    pub fn write_value<T: bytemuck::Pod>(&self, value: &T, byte_offset: u64) -> Result<(), GraphicsException> {
        self.write(std::slice::from_ref(value), byte_offset)
    }

    /// Reads `count` values (None: as many as fit after `byte_offset`) back from the GPU, after
    /// the work submitted so far (C#: `Read<T>(count)`; async here, because a browser maps a
    /// buffer with a promise - the C# call waits). The buffer needs `COPY_SRC`, which the
    /// factories add unless it is a map buffer.
    pub async fn read<T: bytemuck::Pod>(&self, count: Option<usize>, byte_offset: u64) -> Result<Vec<T>, GraphicsException> {
        let element = std::mem::size_of::<T>().max(1) as u64;
        if byte_offset % 4 != 0 || byte_offset > self.size {
            return Err(GraphicsException::new(format!("The read offset {byte_offset} must be a multiple of 4 inside the {}-byte buffer{}.", self.size, label_text(self.label()))));
        }
        let count = count.map(|c| c as u64).unwrap_or((self.size - byte_offset) / element);
        let length = count * element;
        if byte_offset + length > self.size {
            return Err(GraphicsException::new(format!("Reading {count} values of {element} bytes at offset {byte_offset} overruns the {}-byte buffer{}; read fewer.", self.size, label_text(self.label()))));
        }
        if length == 0 {
            return Ok(Vec::new());
        }
        if !self.usage.contains(wgpu::BufferUsages::COPY_SRC) {
            return Err(GraphicsException::new(format!("The buffer{} was not created with COPY_SRC, so it cannot be read; create it with a factory that adds it (every factory but a map buffer's).", label_text(self.label()))));
        }
        let padded = (length + 3) & !3;
        let device = self.context.device();
        let staging = device.create_buffer(&wgpu::BufferDescriptor { label: Some("codebrix-webgpu-graphics readback"), size: padded, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("codebrix-webgpu-graphics readback") });
        encoder.copy_buffer_to_buffer(&self.handle, byte_offset, &staging, 0, Some(padded));
        self.context.queue().submit([encoder.finish()]);
        let (sender, receiver) = oneshot::channel();
        staging.map_async(wgpu::MapMode::Read, .., move |result| sender.send(result));
        if receiver.await.is_err() {
            return Err(GraphicsException::new(format!("The buffer{} could not be mapped for reading (the device may be lost); check GraphicsContext::lost_reason.", label_text(self.label()))));
        }
        let values = match staging.get_mapped_range(..) {
            Ok(view) => bytemuck::pod_collect_to_vec::<u8, T>(&view[..length as usize]),
            Err(error) => return Err(GraphicsException::new(format!("The mapped copy of the buffer{} could not be read ({error}); read it again.", label_text(self.label())))),
        };
        staging.unmap();
        Ok(values)
    }

    /// Releases the buffer's memory now (C#: `Destroy`); dropping it releases it too.
    pub fn destroy(&self) {
        self.handle.destroy();
    }
}

/// The padded length of a write of `length` bytes at `offset` into a buffer of `size`, or the
/// error that says what is wrong.
pub(crate) fn check_write(length: u64, offset: u64, size: u64) -> Result<u64, GraphicsException> {
    if offset % 4 != 0 {
        return Err(GraphicsException::new(format!("The write offset {offset} is not a multiple of 4; WebGPU writes buffers in steps of 4 bytes.")));
    }
    let padded = (length + 3) & !3;
    if offset + padded > size {
        return Err(GraphicsException::new(format!("Writing {length} bytes at offset {offset} overruns the {size}-byte buffer; create a larger buffer or write less.")));
    }
    Ok(padded)
}

#[cfg(test)]
mod tests {
    use super::check_write;

    #[test]
    fn writes_are_padded_to_four_bytes() {
        assert_eq!(check_write(6, 0, 8).unwrap(), 8);
        assert_eq!(check_write(8, 8, 16).unwrap(), 8);
    }

    #[test]
    fn a_write_must_fit_and_be_aligned() {
        assert!(check_write(4, 2, 16).unwrap_err().to_string().contains("not a multiple of 4"));
        assert!(check_write(10, 8, 16).unwrap_err().to_string().contains("overruns the 16-byte buffer"));
    }
}
