//! Images on the GPU (C#: `Images/`): animated textures and the mipmap generator.

mod animated_texture;
mod mipmap_generator;

pub use animated_texture::{AnimatedTexture, AnimatedTextureOptions, AnimatedTextureStorage};
pub(crate) use mipmap_generator::MipmapGenerator;
