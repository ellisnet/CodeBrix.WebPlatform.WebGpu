//! GPU resources: buffers (and their mapping), textures (with regions and async readback),
//! samplers, shaders, bind-group layouts and bind groups (C#: `Resources/`).

mod binding_entry;
mod binding_layout_entry;
mod gpu_bind_group;
mod gpu_buffer;
mod gpu_buffer_mapping;
mod gpu_sampler;
mod gpu_shader;
mod gpu_texture;
mod gpu_texture_readback;
mod texture_formats;
pub(crate) mod texture_region;

pub use binding_entry::{BindingEntry, BindingResource};
pub use binding_layout_entry::{BindingKind, BindingLayoutEntry};
pub use gpu_bind_group::{GpuBindGroup, GpuBindGroupLayout};
pub use gpu_buffer::{BufferMapState, GpuBuffer};
pub use gpu_sampler::{GpuSampler, SamplerDescription};
pub use gpu_shader::{GpuShader, ShaderDiagnostics, ShaderMessage, ShaderMessageKind};
pub use gpu_texture::GpuTexture;
pub use texture_formats::TextureFormats;
pub use texture_region::{TextureCopyLayout, TextureRegion};
