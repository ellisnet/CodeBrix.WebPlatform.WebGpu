//! [`BindingEntry`]: what one binding of a bind group points at (C#: `BindingEntry`).

use crate::resources::{GpuBuffer, GpuSampler, GpuTexture};

/// The resource one binding of a bind group points at (C#: `BindingEntry`). It holds clones of
/// the `wgpu` handles, so the entry does not borrow the resource.
#[derive(Clone, Debug)]
pub struct BindingEntry {
    /// The binding number (C#: `Binding`).
    pub binding: u32,
    /// The resource.
    pub resource: BindingResource,
}

/// The resource of a [`BindingEntry`].
#[derive(Clone, Debug)]
pub enum BindingResource {
    /// A range of a buffer: the buffer, the offset, and the size (None: to the end).
    Buffer(wgpu::Buffer, u64, Option<u64>),
    /// A texture view.
    TextureView(wgpu::TextureView),
    /// A sampler.
    Sampler(wgpu::Sampler),
}

impl BindingEntry {
    /// The whole of a buffer (C#: `ForBuffer(binding, buffer)`); a range with
    /// [`BindingEntry::with_range`].
    pub fn for_buffer(binding: u32, buffer: &GpuBuffer) -> BindingEntry {
        BindingEntry { binding, resource: BindingResource::Buffer(buffer.handle().clone(), 0, None) }
    }

    /// The texture's default view (C#: `ForTexture`).
    pub fn for_texture(binding: u32, texture: &GpuTexture) -> BindingEntry {
        BindingEntry::for_texture_view(binding, texture.view())
    }

    /// A texture view (C#: `ForTextureView`).
    pub fn for_texture_view(binding: u32, view: &wgpu::TextureView) -> BindingEntry {
        BindingEntry { binding, resource: BindingResource::TextureView(view.clone()) }
    }

    /// A sampler (C#: `ForSampler`).
    pub fn for_sampler(binding: u32, sampler: &GpuSampler) -> BindingEntry {
        BindingEntry { binding, resource: BindingResource::Sampler(sampler.handle().clone()) }
    }

    /// A buffer entry limited to `size` bytes (None: to the end) from `offset` (C#: `offset`,
    /// `size`); no effect on other kinds.
    pub fn with_range(mut self, offset: u64, size: Option<u64>) -> BindingEntry {
        if let BindingResource::Buffer(_, value_offset, value_size) = &mut self.resource {
            *value_offset = offset;
            *value_size = size;
        }
        self
    }

    /// The `wgpu` entry, borrowing this one's handles.
    pub fn to_wgpu(&self) -> wgpu::BindGroupEntry<'_> {
        let resource = match &self.resource {
            BindingResource::Buffer(buffer, offset, size) => wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer, offset: *offset, size: size.and_then(wgpu::BufferSize::new) }),
            BindingResource::TextureView(view) => wgpu::BindingResource::TextureView(view),
            BindingResource::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
        };
        wgpu::BindGroupEntry { binding: self.binding, resource }
    }
}
