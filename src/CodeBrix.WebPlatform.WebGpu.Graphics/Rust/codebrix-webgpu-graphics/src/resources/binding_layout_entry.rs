//! [`BindingLayoutEntry`]: one binding of a bind-group layout, made by its factories (C#:
//! `BindingLayoutEntry`, `BindingKind`).

/// What a binding holds (C#: `BindingKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKind {
    /// A uniform or storage buffer.
    Buffer,
    /// A sampled texture.
    Texture,
    /// A sampler.
    Sampler,
    /// A storage texture.
    StorageTexture,
}

/// One binding of a bind-group layout (C#: `BindingLayoutEntry`). Make it with a factory
/// ([`BindingLayoutEntry::uniform`], [`BindingLayoutEntry::texture`], ...); the C# factories'
/// default arguments are the `with_*` methods.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BindingLayoutEntry {
    /// The binding number (`@binding(n)`) (C#: `Binding`).
    pub binding: u32,
    /// The shader stages that see it (C#: `Visibility`).
    pub visibility: wgpu::ShaderStages,
    /// What it holds (C#: `Kind`).
    pub kind: BindingKind,
    /// The `wgpu` binding type.
    pub ty: wgpu::BindingType,
}

impl BindingLayoutEntry {
    /// A uniform buffer (C#: `Uniform(binding, visibility)`), with no dynamic offset and no
    /// minimum size.
    pub fn uniform(binding: u32, visibility: wgpu::ShaderStages) -> BindingLayoutEntry {
        BindingLayoutEntry::buffer(binding, visibility, wgpu::BufferBindingType::Uniform)
    }

    /// A storage buffer (C#: `Storage(binding, visibility, readOnly)`).
    pub fn storage(binding: u32, visibility: wgpu::ShaderStages, read_only: bool) -> BindingLayoutEntry {
        BindingLayoutEntry::buffer(binding, visibility, wgpu::BufferBindingType::Storage { read_only })
    }

    fn buffer(binding: u32, visibility: wgpu::ShaderStages, ty: wgpu::BufferBindingType) -> BindingLayoutEntry {
        BindingLayoutEntry { binding, visibility, kind: BindingKind::Buffer, ty: wgpu::BindingType::Buffer { ty, has_dynamic_offset: false, min_binding_size: None } }
    }

    /// A sampled 2D float texture (C#: `Texture(binding, visibility)`); change the sample type,
    /// dimension or multisampling with the `with_*` methods.
    pub fn texture(binding: u32, visibility: wgpu::ShaderStages) -> BindingLayoutEntry {
        BindingLayoutEntry { binding, visibility, kind: BindingKind::Texture, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false } }
    }

    /// A filtering sampler (C#: `Sampler(binding, visibility)`); another type with
    /// [`BindingLayoutEntry::with_sampler_type`].
    pub fn sampler(binding: u32, visibility: wgpu::ShaderStages) -> BindingLayoutEntry {
        BindingLayoutEntry { binding, visibility, kind: BindingKind::Sampler, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering) }
    }

    /// A write-only 2D storage texture of `format` (C#: `StorageTexture(binding, visibility, format)`).
    pub fn storage_texture(binding: u32, visibility: wgpu::ShaderStages, format: wgpu::TextureFormat) -> BindingLayoutEntry {
        BindingLayoutEntry { binding, visibility, kind: BindingKind::StorageTexture, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format, view_dimension: wgpu::TextureViewDimension::D2 } }
    }

    /// A buffer binding with a dynamic offset (C#: `hasDynamicOffset`); no effect on other kinds.
    pub fn with_dynamic_offset(mut self, has_dynamic_offset: bool) -> BindingLayoutEntry {
        if let wgpu::BindingType::Buffer { has_dynamic_offset: value, .. } = &mut self.ty {
            *value = has_dynamic_offset;
        }
        self
    }

    /// A buffer binding with a minimum size in bytes, 0 for none (C#: `minBindingSize`).
    pub fn with_min_binding_size(mut self, min_binding_size: u64) -> BindingLayoutEntry {
        if let wgpu::BindingType::Buffer { min_binding_size: value, .. } = &mut self.ty {
            *value = wgpu::BufferSize::new(min_binding_size);
        }
        self
    }

    /// A texture binding with another sample type (C#: `sampleType`).
    pub fn with_sample_type(mut self, sample_type: wgpu::TextureSampleType) -> BindingLayoutEntry {
        if let wgpu::BindingType::Texture { sample_type: value, .. } = &mut self.ty {
            *value = sample_type;
        }
        self
    }

    /// A texture or storage-texture binding with another view dimension (C#: `viewDimension`).
    pub fn with_view_dimension(mut self, view_dimension: wgpu::TextureViewDimension) -> BindingLayoutEntry {
        match &mut self.ty {
            wgpu::BindingType::Texture { view_dimension: value, .. } | wgpu::BindingType::StorageTexture { view_dimension: value, .. } => *value = view_dimension,
            _ => {}
        }
        self
    }

    /// A multisampled texture binding (C#: `multisampled`).
    pub fn with_multisampled(mut self, multisampled: bool) -> BindingLayoutEntry {
        if let wgpu::BindingType::Texture { multisampled: value, .. } = &mut self.ty {
            *value = multisampled;
        }
        self
    }

    /// A sampler binding of another type (C#: `type`).
    pub fn with_sampler_type(mut self, sampler_type: wgpu::SamplerBindingType) -> BindingLayoutEntry {
        if let wgpu::BindingType::Sampler(value) = &mut self.ty {
            *value = sampler_type;
        }
        self
    }

    /// A storage texture with another access (C#: `access`).
    pub fn with_storage_access(mut self, access: wgpu::StorageTextureAccess) -> BindingLayoutEntry {
        if let wgpu::BindingType::StorageTexture { access: value, .. } = &mut self.ty {
            *value = access;
        }
        self
    }

    /// The `wgpu` layout entry.
    pub fn to_wgpu(&self) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry { binding: self.binding, visibility: self.visibility, ty: self.ty, count: None }
    }
}

#[cfg(test)]
mod tests {
    use super::{BindingKind, BindingLayoutEntry};

    #[test]
    fn a_uniform_with_a_dynamic_offset() {
        let entry = BindingLayoutEntry::uniform(0, wgpu::ShaderStages::VERTEX).with_dynamic_offset(true).with_min_binding_size(64);
        assert_eq!(entry.kind, BindingKind::Buffer);
        assert_eq!(entry.ty, wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: wgpu::BufferSize::new(64) });
    }

    #[test]
    fn texture_and_sampler_defaults_and_changes() {
        let texture = BindingLayoutEntry::texture(1, wgpu::ShaderStages::FRAGMENT).with_view_dimension(wgpu::TextureViewDimension::Cube);
        assert_eq!(texture.to_wgpu().ty, wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::Cube, multisampled: false });
        let sampler = BindingLayoutEntry::sampler(2, wgpu::ShaderStages::FRAGMENT).with_sampler_type(wgpu::SamplerBindingType::Comparison).with_dynamic_offset(true);
        assert_eq!(sampler.ty, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison));
    }

    #[test]
    fn storage_is_read_only_when_asked() {
        let entry = BindingLayoutEntry::storage(3, wgpu::ShaderStages::VERTEX, true);
        assert_eq!(entry.ty, wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None });
    }
}
