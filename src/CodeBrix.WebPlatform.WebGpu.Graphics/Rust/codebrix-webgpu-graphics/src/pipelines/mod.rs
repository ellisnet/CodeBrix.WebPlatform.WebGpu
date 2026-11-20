//! Pipelines: the render and compute pipeline builders, pipeline layouts, the presets and vertex
//! layouts (C#: `Pipelines/`).

mod compute_pipeline_builder;
mod presets;
mod render_pipeline_builder;
mod vertex_layout;

pub use compute_pipeline_builder::{ComputePipelineBuilder, GpuComputePipeline, GpuPipelineLayout};
pub use presets::{BlendPreset, CullPreset, DepthPreset};
pub use render_pipeline_builder::{ColorTargetOptions, DepthOptions, GpuRenderPipeline, RenderPipelineBuilder, StencilMasks};
pub use vertex_layout::{VertexElement, VertexFormats, VertexLayout, VertexLayoutBuilder, VertexLayoutOptions, VertexType};
