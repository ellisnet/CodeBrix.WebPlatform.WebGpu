//! [`EnvironmentOptions`]: the sizes and sample counts an environment is built with (C#:
//! `EnvironmentOptions`).

/// How an [`crate::EnvironmentMap`] is built: the sizes of its three cube maps and the sample
/// counts of the filtering passes (C#: `EnvironmentOptions`; its properties are the public fields,
/// with the same defaults). The defaults suit a first-class look at a modest build cost; every
/// value is clamped to a sensible range when the environment is built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentOptions {
    /// The face size of the radiance cube (the environment itself, drawn by the
    /// [`crate::SkyboxRenderer`] and the source of the filtered maps), rounded up to a power of
    /// two, 16 to 4096 (and the device's limit). Default 512 (a 2048 x 1024 panorama maps about
    /// one to one) (C#: `CubeSize`).
    pub cube_size: u32,
    /// The face size of the diffuse irradiance cube, a power of two from 4 to 128. Default 32
    /// (C#: `IrradianceSize`).
    pub irradiance_size: u32,
    /// The face size of the radiance mip the irradiance pass integrates (every texel of it,
    /// weighted by solid angle and the cosine), a power of two from 4 to 64. Default 32 (C#:
    /// `IrradianceSourceSize`).
    pub irradiance_source_size: u32,
    /// The face size of mip 0 of the GGX-prefiltered specular cube (mip 0 = roughness 0), a power
    /// of two from 16 to the cube size, or 0 for half the cube size (at most 256). Default 0 (C#:
    /// `PrefilteredSize`).
    pub prefiltered_size: u32,
    /// The mip levels of the prefiltered cube; mip m holds roughness m / (levels - 1). 2 to 8, and
    /// the smallest mip is never below 4 x 4. Default 6 (C#: `PrefilteredMipLevels`).
    pub prefiltered_mip_levels: u32,
    /// GGX importance samples per texel of each rough prefiltered mip, 8 to 4096. Default 128 (C#:
    /// `SpecularSampleCount`).
    pub specular_sample_count: u32,
    /// The widest panorama the GPU receives: wider sources are halved on the CPU until they fit
    /// (and never beyond the device's 2D texture limit). Default 8192 (C#: `MaxSourceWidth`).
    pub max_source_width: u32,
    /// A label for the GPU objects (C#: `Label`).
    pub label: Option<String>,
}

impl Default for EnvironmentOptions {
    fn default() -> EnvironmentOptions {
        EnvironmentOptions { cube_size: 512, irradiance_size: 32, irradiance_source_size: 32, prefiltered_size: 0, prefiltered_mip_levels: 6, specular_sample_count: 128, max_source_width: 8192, label: None }
    }
}
