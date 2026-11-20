//! [`GpuSampler`] and [`SamplerDescription`]: how textures are filtered and addressed (C#:
//! `GpuSampler`, `SamplerDescription`).

/// What a sampler does (C#: `SamplerDescription`). Start from a preset or from `default()`
/// (linear filtering, clamp to edge) and change fields.
#[derive(Clone, Debug, PartialEq)]
pub struct SamplerDescription {
    /// The address mode of every axis that has none of its own (C#: `AddressMode`).
    pub address_mode: wgpu::AddressMode,
    /// The address mode across (u), overriding `address_mode` (C#: `AddressModeU`).
    pub address_mode_u: Option<wgpu::AddressMode>,
    /// The address mode down (v), overriding `address_mode` (C#: `AddressModeV`).
    pub address_mode_v: Option<wgpu::AddressMode>,
    /// The highest mip level of detail used (C#: `MaxLod`, default 32).
    pub max_lod: f32,
    /// Magnification filter (C#: `MagFilter`).
    pub mag_filter: wgpu::FilterMode,
    /// Minification filter (C#: `MinFilter`).
    pub min_filter: wgpu::FilterMode,
    /// Filter between mip levels (C#: `MipmapFilter`).
    pub mipmap_filter: wgpu::MipmapFilterMode,
    /// A comparison function for a comparison (shadow) sampler; None for an ordinary sampler (C#:
    /// `Compare`, Undefined).
    pub compare: Option<wgpu::CompareFunction>,
    /// The anisotropy clamp, 1 for none (C#: `MaxAnisotropy`).
    pub max_anisotropy: u16,
    /// The label (C#: `Label`).
    pub label: Option<String>,
}

impl Default for SamplerDescription {
    fn default() -> SamplerDescription {
        SamplerDescription {
            address_mode: wgpu::AddressMode::ClampToEdge,
            address_mode_u: None,
            address_mode_v: None,
            max_lod: 32.0,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            compare: None,
            max_anisotropy: 1,
            label: None,
        }
    }
}

impl SamplerDescription {
    /// Linear filtering, clamp to edge (C#: `LinearClamp`).
    pub fn linear_clamp() -> SamplerDescription {
        SamplerDescription::default()
    }

    /// Linear filtering, repeat (C#: `LinearRepeat`).
    pub fn linear_repeat() -> SamplerDescription {
        SamplerDescription { address_mode: wgpu::AddressMode::Repeat, ..SamplerDescription::default() }
    }

    /// Nearest filtering, clamp to edge (C#: `NearestClamp`).
    pub fn nearest_clamp() -> SamplerDescription {
        SamplerDescription { mag_filter: wgpu::FilterMode::Nearest, min_filter: wgpu::FilterMode::Nearest, mipmap_filter: wgpu::MipmapFilterMode::Nearest, ..SamplerDescription::default() }
    }

    /// Nearest filtering, repeat (C#: `NearestRepeat`).
    pub fn nearest_repeat() -> SamplerDescription {
        SamplerDescription { address_mode: wgpu::AddressMode::Repeat, ..SamplerDescription::nearest_clamp() }
    }

    /// The `wgpu` descriptor of this description.
    pub fn to_wgpu(&self) -> wgpu::SamplerDescriptor<'_> {
        wgpu::SamplerDescriptor {
            label: self.label.as_deref(),
            address_mode_u: self.address_mode_u.unwrap_or(self.address_mode),
            address_mode_v: self.address_mode_v.unwrap_or(self.address_mode),
            address_mode_w: self.address_mode,
            mag_filter: self.mag_filter,
            min_filter: self.min_filter,
            mipmap_filter: self.mipmap_filter,
            lod_min_clamp: 0.0,
            lod_max_clamp: self.max_lod,
            compare: self.compare,
            anisotropy_clamp: self.max_anisotropy.max(1),
            border_color: None,
        }
    }
}

/// A WebGPU sampler and what it was made with (C#: `GpuSampler`).
pub struct GpuSampler {
    handle: wgpu::Sampler,
    description: SamplerDescription,
}

impl GpuSampler {
    pub(crate) fn new(handle: wgpu::Sampler, description: SamplerDescription) -> GpuSampler {
        GpuSampler { handle, description }
    }

    /// The `wgpu` sampler (C#: `Handle`).
    pub fn handle(&self) -> &wgpu::Sampler {
        &self.handle
    }

    /// The address mode (C#: `AddressMode`).
    pub fn address_mode(&self) -> wgpu::AddressMode {
        self.description.address_mode
    }

    /// The magnification filter (C#: `MagFilter`).
    pub fn mag_filter(&self) -> wgpu::FilterMode {
        self.description.mag_filter
    }

    /// The minification filter (C#: `MinFilter`).
    pub fn min_filter(&self) -> wgpu::FilterMode {
        self.description.min_filter
    }

    /// True for a comparison sampler (C#: `IsComparison`).
    pub fn is_comparison(&self) -> bool {
        self.description.compare.is_some()
    }

    /// The label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.description.label.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::SamplerDescription;

    #[test]
    fn presets_match_the_csharp_presets() {
        let repeat = SamplerDescription::nearest_repeat();
        assert_eq!(repeat.address_mode, wgpu::AddressMode::Repeat);
        assert_eq!(repeat.min_filter, wgpu::FilterMode::Nearest);
        assert_eq!(SamplerDescription::linear_clamp().mag_filter, wgpu::FilterMode::Linear);
    }

    #[test]
    fn per_axis_modes_override_the_common_one() {
        let description = SamplerDescription { address_mode_u: Some(wgpu::AddressMode::MirrorRepeat), ..SamplerDescription::linear_repeat() };
        let wgpu = description.to_wgpu();
        assert_eq!(wgpu.address_mode_u, wgpu::AddressMode::MirrorRepeat);
        assert_eq!(wgpu.address_mode_v, wgpu::AddressMode::Repeat);
        assert_eq!(wgpu.lod_max_clamp, 32.0);
    }
}
