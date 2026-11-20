//! # codebrix-webgpu-graphics
//!
//! 2D and 3D graphics through WebGPU for a CodeBrix.WebPlatform frontend: the Rust counterpart of
//! the C# library CodeBrix.WebGpu.Graphics, with the same type names, members in snake_case, the
//! same argument order, and an options struct (with `Default`) where C# has default arguments.
//! It is built on `codebrix-webgpu` - its context, canvas host, layers and clock - and draws as a
//! LAYER of that host: into the frame it is handed, over what is already there, with the frame's
//! time.
//!
//! What it offers (every area of the C# library a browser can do; what it cannot, the
//! repository's AGENT-README lists in one place):
//! - [`GraphicsContext`]: the device, owned ([`GraphicsContext::create`]) or shared with the canvas
//!   host ([`GraphicsContext::wrap_gpu_context`]); WebGPU's uncaptured errors as a queue
//!   ([`GraphicsContext::take_errors`]); the async validation scope ([`GraphicsContext::validate`]).
//! - Resources: [`GpuBuffer`] (with its mapping, [`BufferMapState`]), [`GpuTexture`] (with
//!   [`TextureRegion`]s, layer views and the async readback; [`TextureCopyLayout`]), [`GpuSampler`]
//!   with [`SamplerDescription`], [`GpuShader`] with the browser compiler's
//!   [`ShaderDiagnostics`], [`GpuBindGroupLayout`] from [`BindingLayoutEntry`]s and
//!   [`GpuBindGroup`] from [`BindingEntry`]s.
//! - Pipelines: [`RenderPipelineBuilder`] with [`BlendPreset`], [`DepthPreset`], [`CullPreset`] and
//!   [`VertexLayout`] (described element by element, or by a [`VertexType`]);
//!   [`ComputePipelineBuilder`] and [`GpuComputePipeline`]; [`GpuPipelineLayout`].
//! - Frames: [`GraphicsFrame`] (its own encoder, a borrowed one, or a canvas frame's) with its
//!   copies, clears and query resolves, [`RenderPass`] (with indirect draws - [`DrawIndirectArgs`],
//!   [`DrawIndexedIndirectArgs`] - and bundles) and [`ComputePass`] ([`DispatchIndirectArgs`]).
//! - Render bundles: [`RenderBundleBuilder`] records a [`GpuRenderBundle`] once; a pass replays it.
//! - Queries: timestamp [`GpuQuerySet`]s, [`PassTimestamps`] and the [`GpuTimer`] (where the browser
//!   grants `timestamp-query`); occlusion sets are refused, with the reason.
//! - 2D: [`SpriteBatch`] - sprites from whole textures or atlas regions, tinted, rotated, scaled
//!   about an origin, in drawing order, with a transform stack and shapes ([`RectangleF`]).
//! - 3D: [`MeshShapes`] (cube, box, plane, sphere) as [`MeshData`] of [`MeshVertex`], [`Mesh`],
//!   [`MeshRenderer`] with [`LitMaterial`]s and [`SceneLighting`] (Blinn-Phong, one key light,
//!   ambient, instancing, opaque then translucent, mirrored instances), a [`Camera`]
//!   (perspective or orthographic), [`OrbitCamera`], and [`OrbitController`], which drives the
//!   orbit from the canvas host's input (drag, wheel, one-finger orbit, pinch) within
//!   [`OrbitLimits`], auto-orbiting until the first drag.
//! - Models: [`ModelDataSet`] read from model-data bytes, [`Model`], [`ModelInstance`] and the
//!   [`ModelRenderer`] (glTF metallic-roughness shading, GPU skinning and morph targets).
//! - Animation playback: each instance's [`AnimationPlayer`] ([`ModelInstance::animation`]) plays
//!   the model's clips - [`AnimationState`]s with [`PlayOptions`] and an [`AnimationLoopMode`],
//!   cross-fades and layered, weighted blends - driven by the frame's time or delta (never a
//!   clock of its own); [`ModelPoseEdit`] for a pose set by hand.
//! - Image-based lighting, the skybox and tone mapping: [`EnvironmentMap`]s baked on the GPU from
//!   the analytic sky ([`GraphicsContext::create_procedural_sky`], [`ProceduralSkyOptions`]; the
//!   context's [`GraphicsContext::default_environment`]) or from an HDR panorama given as pixels
//!   ([`FloatImage`], [`GraphicsContext::create_environment`]) or as Radiance `.hdr` bytes
//!   ([`RadianceHdrDecoder`], [`GraphicsContext::create_environment_from_bytes`]); the BRDF LUT;
//!   [`SkyboxRenderer`]; the HDR path through an RGBA16Float [`HeadlessRenderTarget`] and the
//!   [`ToneMapper`] with every [`ToneMapOperator`]; [`GraphicsContext::generate_mipmaps`] on the
//!   GPU.
//! - Animated textures: [`AnimatedTexture`] from RGBA frames with durations
//!   ([`AnimatedTextureOptions`], [`AnimatedTextureStorage`]), drawn by `SpriteBatch::draw_animated`.
//! - [`shaders`]: every built-in WGSL source of the C# library, including the PBR shader's
//!   feature variants ([`PbrShader::source`]).
//!
//! THE ONE DELIBERATE DIFFERENCE FROM C#: a browser cannot wait synchronously. Device creation,
//! buffer and texture readback, buffer mapping, the GPU timer's results and shader diagnostics
//! are `async`; a factory returns its object at once, and
//! a validation error WebGPU finds later is queued for [`GraphicsContext::take_errors`] (C#
//! `TakeErrors()`; the device's one error handler is `codebrix-webgpu`'s, shared with any other
//! library on the device, and every context sees every error). Where C# would throw with WebGPU's message, await a `_checked` variant, or
//! [`GraphicsContext::try_create_shader`], or wrap the calls in [`GraphicsContext::validate`].
//! Checks the library makes itself (a size of 0, a missing colour target) return a
//! [`GraphicsException`] at once.
//!
//! LOW-LEVEL `wgpu` STAYS AVAILABLE: every type hands out its `wgpu` handle, and the crate
//! re-exports [`wgpu`] and [`glam`] (the versions of `codebrix-webgpu`). The crate never talks to
//! the application's backend: pixels and data are handed to it.
//!
//! ```ignore
//! use codebrix_webgpu::{CanvasFrame, CanvasRenderer, CanvasTarget, GpuContext};
//! use codebrix_webgpu_graphics::{ColorTargetOptions, BlendPreset, DrawOptions, GpuRenderPipeline, GraphicsContext};
//!
//! struct Layer { graphics: GraphicsContext, pipeline: Option<GpuRenderPipeline> }
//!
//! impl CanvasRenderer for Layer {
//!     fn prepare(&mut self, _gpu: &GpuContext, target: &CanvasTarget) {
//!         let shader = self.graphics.create_shader(MY_WGSL, Some("layer"));
//!         self.pipeline = self.graphics.create_render_pipeline(&shader)
//!             .with_color_target(target.format, ColorTargetOptions::blended(BlendPreset::PremultipliedAlpha))
//!             .build().ok();
//!     }
//!     fn render(&mut self, frame: &mut CanvasFrame<'_>) {
//!         let graphics = self.graphics.clone();
//!         let mut gframe = graphics.begin_canvas_frame(frame, None);
//!         let mut pass = gframe.begin_render_pass(None, None).unwrap(); // None: LOAD, never clear
//!         pass.set_pipeline(self.pipeline.as_ref().unwrap());
//!         pass.draw(3, DrawOptions::default());
//!     }
//! }
//! ```

mod animation;
mod bundles;
mod common;
mod context;
mod drawing2d;
mod drawing3d;
mod frames;
mod images;
mod internal;
mod lighting;
mod models;
mod pipelines;
mod presentation;
mod queries;
mod resources;
pub mod shaders;

pub use animation::{AnimationLoopMode, AnimationPlayer, AnimationState, PlayOptions};
pub use bundles::{GpuRenderBundle, RenderBundleBuilder, RenderBundleOptions};
pub use common::{Color4, RectangleF};
pub use drawing2d::{CircleOptions, SpriteBatch, SpriteBatchBeginOptions};
pub use drawing3d::{AmbientLighting, Camera, DirectionalLight, LitMaterial, MaterialAlphaMode, MaterialOptions, Mesh, MeshData, MeshRenderer, MeshShapes, MeshVertex, OrbitCamera, OrbitController, OrbitLimits, ProjectionKind, SceneLighting};
pub use context::{DepthTextureOptions, ErrorType, GraphicsContext, GraphicsContextOptions, GraphicsError, GraphicsException, RenderTextureOptions, RgbaTextureOptions, ShaderCompilationException, Texture2DOptions};
pub use frames::{BufferRange, ComputePass, ComputePassOptions, DepthRange, DispatchIndirectArgs, DrawIndexedIndirectArgs, DrawIndexedOptions, DrawIndirectArgs, DrawOptions, GraphicsFrame, RenderPass, RenderPassOptions};
pub use images::{AnimatedTexture, AnimatedTextureOptions, AnimatedTextureStorage};
pub use pipelines::{BlendPreset, ColorTargetOptions, ComputePipelineBuilder, CullPreset, DepthOptions, DepthPreset, GpuComputePipeline, GpuPipelineLayout, GpuRenderPipeline, RenderPipelineBuilder, StencilMasks, VertexElement, VertexFormats, VertexLayout, VertexLayoutBuilder, VertexLayoutOptions, VertexType};
pub use models::{AnimationInterpolation, AnimationPath, Model, ModelAnimation, ModelAnimationChannel, ModelData, ModelDataFormat, ModelDataSet, ModelInstance, ModelLoadOptions, ModelMaterial, ModelMesh, ModelMorphTarget, ModelNode, ModelPoseEdit, ModelPrimitive, ModelRenderer, ModelSkin, ModelSkinVertex, ModelTextureData, ModelTextureSlot, ModelVertex, NodeTransform};
pub use lighting::{EnvironmentMap, EnvironmentOptions, FloatImage, ProceduralSkyOptions, RadianceHdrDecoder, SkyboxDrawOptions, SkyboxRenderer, ToneMapOperator, ToneMapper};
pub use presentation::{HeadlessRenderTarget, HeadlessTargetOptions, RenderTargetView};
pub use queries::{GpuQuerySet, GpuTimer, PassTimestamps};
pub use resources::{BindingEntry, BindingKind, BindingLayoutEntry, BindingResource, BufferMapState, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuShader, GpuTexture, SamplerDescription, ShaderDiagnostics, ShaderMessage, ShaderMessageKind, TextureCopyLayout, TextureFormats, TextureRegion};
pub use shaders::{PbrFeatures, PbrShader};

/// The `glam` crate (vectors, matrices, quaternions), the version `codebrix-webgpu` was built with.
pub use codebrix_webgpu::glam;
/// The `wgpu` crate, the version `codebrix-webgpu` was built with.
pub use codebrix_webgpu::wgpu;
/// The `bytemuck` crate: [`GpuBuffer::write`] takes `bytemuck::Pod` values.
pub use bytemuck;
