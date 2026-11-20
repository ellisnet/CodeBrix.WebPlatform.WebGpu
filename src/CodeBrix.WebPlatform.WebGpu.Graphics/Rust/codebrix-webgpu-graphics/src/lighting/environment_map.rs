//! [`EnvironmentMap`]: an environment for image-based lighting and sky backgrounds, three
//! RGBA16Float cube maps built on the GPU (C#: `EnvironmentMap`).

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

/// An environment for image-based lighting and sky backgrounds (C#: `EnvironmentMap`), built on
/// the GPU from an HDR panorama ([`crate::GraphicsContext::create_environment`]) or an analytic
/// sky ([`crate::GraphicsContext::create_procedural_sky`]): three RGBA16Float cube maps -
/// [`EnvironmentMap::radiance`] (the environment itself, full mip chain),
/// [`EnvironmentMap::irradiance`] (diffuse irradiance / PI) and [`EnvironmentMap::prefiltered`]
/// (GGX-prefiltered specular radiance, roughness 0..1 over its mips). Use it through
/// [`crate::SceneLighting::environment`] (model lighting) and [`crate::SkyboxRenderer`]
/// (background), or bind the cube views with [`crate::GraphicsContext::brdf_lut`] in your own
/// shaders.
///
/// It is a shared HANDLE: clones are the same environment, and the GPU textures are released when
/// the last clone (and the last bind group a renderer keeps for it) is dropped - C#'s `Dispose`.
/// Immutable once built. The cube views are `wgpu` views (C# wraps them as `GpuTexture`s).
///
/// Cube layout: layers +X, -X, +Y, -Y, +Z, -Z (WebGPU). A panorama maps with its horizontal
/// centre towards -Z and its top row straight up (+Y), the u coordinate growing towards +X; turn
/// it about +Y at draw time with [`crate::SceneLighting::environment_rotation`].
#[derive(Clone)]
pub struct EnvironmentMap {
    inner: Rc<EnvironmentInner>,
}

struct EnvironmentInner {
    id: u64,
    label: String,
    radiance_array: wgpu::Texture,
    irradiance_array: wgpu::Texture,
    prefiltered_array: wgpu::Texture,
    radiance: wgpu::TextureView,
    irradiance: wgpu::TextureView,
    prefiltered: wgpu::TextureView,
    is_procedural: bool,
}

thread_local! {
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
}

impl EnvironmentMap {
    /// An environment over three six-layer textures (the baker's).
    pub(crate) fn new(label: &str, radiance_array: wgpu::Texture, irradiance_array: wgpu::Texture, prefiltered_array: wgpu::Texture, is_procedural: bool) -> EnvironmentMap {
        let cube = |texture: &wgpu::Texture, what: &str| texture.create_view(&wgpu::TextureViewDescriptor { label: Some(&format!("{label} {what} cube")), dimension: Some(wgpu::TextureViewDimension::Cube), array_layer_count: Some(6), ..wgpu::TextureViewDescriptor::default() });
        let radiance = cube(&radiance_array, "radiance");
        let irradiance = cube(&irradiance_array, "irradiance");
        let prefiltered = cube(&prefiltered_array, "prefiltered");
        let id = NEXT_ID.with(|next| {
            let id = next.get();
            next.set(id + 1);
            id
        });
        EnvironmentMap { inner: Rc::new(EnvironmentInner { id, label: label.to_string(), radiance_array, irradiance_array, prefiltered_array, radiance, irradiance, prefiltered, is_procedural }) }
    }

    /// The environment: an RGBA16Float cube view with the full mip chain (C#: `Radiance`).
    pub fn radiance(&self) -> &wgpu::TextureView {
        &self.inner.radiance
    }

    /// Diffuse irradiance divided by PI: an RGBA16Float cube view of one mip; a white matte
    /// surface with normal n shows irradiance(n) (C#: `Irradiance`).
    pub fn irradiance(&self) -> &wgpu::TextureView {
        &self.inner.irradiance
    }

    /// GGX-prefiltered specular radiance: an RGBA16Float cube view; mip m holds roughness m /
    /// ([`EnvironmentMap::prefiltered_mip_levels`] - 1) (C#: `Prefiltered`).
    pub fn prefiltered(&self) -> &wgpu::TextureView {
        &self.inner.prefiltered
    }

    /// The face size of the radiance cube (C#: `Size`).
    pub fn size(&self) -> u32 {
        self.inner.radiance_array.width()
    }

    /// The face size of the irradiance cube (C#: `IrradianceSize`).
    pub fn irradiance_size(&self) -> u32 {
        self.inner.irradiance_array.width()
    }

    /// The face size of mip 0 of the prefiltered cube (C#: `PrefilteredSize`).
    pub fn prefiltered_size(&self) -> u32 {
        self.inner.prefiltered_array.width()
    }

    /// The mip levels of the prefiltered cube (C#: `PrefilteredMipLevels`).
    pub fn prefiltered_mip_levels(&self) -> u32 {
        self.inner.prefiltered_array.mip_level_count()
    }

    /// The mip levels of the radiance cube.
    pub fn radiance_mip_levels(&self) -> u32 {
        self.inner.radiance_array.mip_level_count()
    }

    /// True for an environment made by [`crate::GraphicsContext::create_procedural_sky`] (C#:
    /// `IsProcedural`).
    pub fn is_procedural(&self) -> bool {
        self.inner.is_procedural
    }

    /// The label the GPU objects carry.
    pub fn label(&self) -> &str {
        &self.inner.label
    }

    /// The six-layer array texture behind [`EnvironmentMap::radiance`] (C#: `RadianceArray`).
    pub fn radiance_array(&self) -> &wgpu::Texture {
        &self.inner.radiance_array
    }

    /// The six-layer array texture behind [`EnvironmentMap::irradiance`] (C#: `IrradianceArray`).
    pub fn irradiance_array(&self) -> &wgpu::Texture {
        &self.inner.irradiance_array
    }

    /// The six-layer array texture behind [`EnvironmentMap::prefiltered`] (C#:
    /// `PrefilteredArray`).
    pub fn prefiltered_array(&self) -> &wgpu::Texture {
        &self.inner.prefiltered_array
    }

    /// A number unique to this environment in the page (renderers key their bind groups by it).
    pub(crate) fn id(&self) -> u64 {
        self.inner.id
    }
}

impl PartialEq for EnvironmentMap {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl fmt::Debug for EnvironmentMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EnvironmentMap '{}' ({} cube, {})", self.inner.label, self.size(), if self.inner.is_procedural { "procedural" } else { "panorama" })
    }
}
