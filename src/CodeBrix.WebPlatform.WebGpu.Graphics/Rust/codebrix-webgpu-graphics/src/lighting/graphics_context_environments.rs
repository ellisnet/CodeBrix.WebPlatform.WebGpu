//! The environment factories of [`GraphicsContext`]: environments from panoramas and from the
//! analytic sky, the default environment, the BRDF LUT, and the skybox and tone-map renderers
//! (C#: `GraphicsContext.Environments.cs`).

use crate::context::{GraphicsContext, GraphicsException};
use crate::lighting::{EnvironmentMap, EnvironmentOptions, FloatImage, ProceduralSkyOptions, RadianceHdrDecoder, SkyboxRenderer, ToneMapper};
use crate::resources::GpuTexture;

impl GraphicsContext {
    /// The environment the model renderer and the skybox use when
    /// [`crate::SceneLighting::environment`] is None (C#: `DefaultEnvironment`): a procedural sky
    /// with [`ProceduralSkyOptions`] defaults (256-texel cube), built on first use and kept by the
    /// context. The first call bakes it on the GPU (a few submissions of its own).
    pub fn default_environment(&self) -> Result<EnvironmentMap, GraphicsException> {
        if let Some(environment) = &self.environment_cache().borrow().default_environment {
            return Ok(environment.clone());
        }
        let options = EnvironmentOptions { cube_size: 256, prefiltered_size: 128, label: Some("default sky".to_string()), ..EnvironmentOptions::default() };
        let environment = self.create_procedural_sky(None, Some(&options))?;
        self.environment_cache().borrow_mut().default_environment = Some(environment.clone());
        Ok(environment)
    }

    /// The split-sum BRDF lookup table every environment shares (C#: `BrdfLut`): 128 x 128
    /// Rg16Float; u = N.V, v = perceptual roughness; R = scale and G = bias of F0, so specular =
    /// prefiltered x (F0 x R + G). Rendered on first use and kept by the context. For your own
    /// image-based-lighting shaders.
    pub fn brdf_lut(&self) -> Result<GpuTexture, GraphicsException> {
        let (texture, _) = self.environment_cache().borrow_mut().brdf_lut(self)?;
        Ok(self.wrap_texture(texture, Some("BRDF LUT")))
    }

    /// The BRDF LUT's view, for the renderers.
    pub(crate) fn brdf_lut_view(&self) -> Result<wgpu::TextureView, GraphicsException> {
        Ok(self.environment_cache().borrow_mut().brdf_lut(self)?.1)
    }

    /// A skybox renderer (C#: `CreateSkyboxRenderer()`).
    pub fn create_skybox_renderer(&self) -> Result<SkyboxRenderer, GraphicsException> {
        SkyboxRenderer::new(self)
    }

    /// A tone mapper for the HDR rendering path (C#: `CreateToneMapper()`).
    pub fn create_tone_mapper(&self) -> Result<ToneMapper, GraphicsException> {
        ToneMapper::new(self)
    }

    /// Builds an environment from a linear equirectangular (2:1 latitude-longitude) panorama in
    /// memory (C#: `CreateEnvironment(FloatImage, options)`); None for the default options.
    /// Panoramas wider than [`EnvironmentOptions::max_source_width`] are halved on the CPU first.
    pub fn create_environment(&self, panorama: &FloatImage, options: Option<&EnvironmentOptions>) -> Result<EnvironmentMap, GraphicsException> {
        let defaults = EnvironmentOptions::default();
        self.environment_cache().borrow_mut().from_panorama(self, panorama, options.unwrap_or(&defaults))
    }

    /// Builds an environment from the BYTES of a Radiance `.hdr` panorama file (C#:
    /// `CreateEnvironment(Stream, options)` for a `.hdr`; the usual HDRI download). The bytes come
    /// from wherever the application has them - the platform's generated `_bytes` call to its
    /// backend, or `include_bytes!` in the frontend; the crate never fetches anything. Other image
    /// formats (PNG, JPEG, JPEG XL) are not decoded in the browser: decode them where they are
    /// (the backend) into linear floats and use [`GraphicsContext::create_environment`]. OpenEXR
    /// is not supported (convert it to `.hdr`).
    pub fn create_environment_from_bytes(&self, data: &[u8], options: Option<&EnvironmentOptions>) -> Result<EnvironmentMap, GraphicsException> {
        if data.starts_with(&[0x76, 0x2f, 0x31, 0x01]) {
            return Err(GraphicsException::new("The environment image is OpenEXR, which is not supported; convert it to Radiance .hdr."));
        }
        if !RadianceHdrDecoder::is_radiance_hdr(data) {
            return Err(GraphicsException::new("The environment image is not a Radiance .hdr file; pass .hdr bytes here, or decode other formats into linear floats (a FloatImage) and call create_environment."));
        }
        let panorama = RadianceHdrDecoder::decode(data)?;
        self.create_environment(&panorama, options)
    }

    /// Builds an analytic sky environment - gradient, ground, haze, sun glow and an optional sun
    /// disc, the look of [`GraphicsContext::default_environment`] with your parameters (C#:
    /// `CreateProceduralSky(sky, options)`); None for the defaults.
    pub fn create_procedural_sky(&self, sky: Option<&ProceduralSkyOptions>, options: Option<&EnvironmentOptions>) -> Result<EnvironmentMap, GraphicsException> {
        let sky_defaults = ProceduralSkyOptions::default();
        let defaults = EnvironmentOptions::default();
        self.environment_cache().borrow_mut().from_sky(self, sky.unwrap_or(&sky_defaults), options.unwrap_or(&defaults))
    }
}
