//! The built-in WGSL shaders of the library's renderers, as `.wgsl` files compiled into the crate
//! (`include_str!`). They are public so that a page can compile every one of them once in its
//! browser ([`BuiltInShaderSource::all`]) and so that an application can build on them.

mod built_in_shaders;
mod pbr_shader;

pub use built_in_shaders::{BuiltInShaderSource, BuiltInShaders, EnvironmentShaders, MipmapShader};
pub use pbr_shader::{PbrFeatures, PbrShader};
