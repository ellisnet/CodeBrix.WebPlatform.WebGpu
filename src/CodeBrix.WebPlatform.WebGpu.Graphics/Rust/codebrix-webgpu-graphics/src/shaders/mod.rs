//! The built-in WGSL shaders of the library's renderers, as `.wgsl` files compiled into the crate
//! (`include_str!`). They are public so that a page can compile every one of them once in its
//! browser ([`BuiltInShaderSource::all`]) and so that an application can build on them.

mod built_in_shaders;
mod effect_shaders;
mod pbr_shader;
mod shadow_depth_shader;
mod water_shaders;

pub use built_in_shaders::{BuiltInShaderSource, BuiltInShaders, EnvironmentShaders, MipmapShader};
pub use effect_shaders::EffectShaders;
pub use pbr_shader::{PbrFeatures, PbrShader};
pub use shadow_depth_shader::ShadowDepthShader;
pub use water_shaders::WaterShaders;
