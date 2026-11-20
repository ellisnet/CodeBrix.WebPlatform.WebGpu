//! The graphics context - the device the library draws with, owned or shared - with its options,
//! its errors and its resource factories (C#: `Context/`).

mod graphics_context;
mod graphics_context_options;
mod graphics_context_resources;
mod graphics_error;
mod graphics_exception;

pub(crate) use graphics_context::label_text;
pub(crate) use graphics_context_resources::buffer_usage;
pub use graphics_context::GraphicsContext;
pub use graphics_context_options::GraphicsContextOptions;
pub use graphics_context_resources::{DepthTextureOptions, RenderTextureOptions, RgbaTextureOptions, Texture2DOptions};
pub use graphics_error::{ErrorType, GraphicsError};
pub use graphics_exception::{GraphicsException, ShaderCompilationException};
