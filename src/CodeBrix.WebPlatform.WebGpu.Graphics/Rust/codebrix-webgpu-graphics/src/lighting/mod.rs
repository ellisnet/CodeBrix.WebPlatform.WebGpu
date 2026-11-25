//! Image-based lighting, the skybox and tone mapping (C#: `Lighting/`): environments from the
//! analytic sky or from HDR panoramas (pixels, or Radiance `.hdr` bytes through the decoder
//! ported from the C# library), the skybox renderer, the tone mapper and the context's
//! environment factories; and (Not in C#) the local lights and the shadow settings of the model
//! renderer.

mod environment_baker;
mod environment_map;
mod environment_options;
mod float_image;
mod graphics_context_environments;
pub(crate) mod local_lights;
mod procedural_sky_options;
mod radiance_hdr_decoder;
pub(crate) mod shadows;
mod skybox_renderer;
mod tone_mapper;

pub(crate) use environment_baker::EnvironmentCache;
pub use environment_map::EnvironmentMap;
pub use environment_options::EnvironmentOptions;
pub use float_image::FloatImage;
pub use local_lights::{LocalLight, PointLight, SpotLight};
pub use procedural_sky_options::ProceduralSkyOptions;
pub use radiance_hdr_decoder::RadianceHdrDecoder;
pub use shadows::{ShadowMapSize, SpotShadowSettings, SunShadowSettings};
pub use skybox_renderer::{SkyboxDrawOptions, SkyboxRenderer};
pub use tone_mapper::{ToneMapOperator, ToneMapper};
