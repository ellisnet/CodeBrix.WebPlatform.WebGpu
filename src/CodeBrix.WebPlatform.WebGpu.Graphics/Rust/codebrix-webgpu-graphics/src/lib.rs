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
//! - Frames: [`GraphicsFrame`] (its own encoder, a borrowed one, or a canvas frame's; with a target
//!   of its own without a canvas, [`GraphicsContext::begin_frame_on`]) with its
//!   copies, clears and query resolves, [`RenderPass`] (with indirect draws - [`DrawIndirectArgs`],
//!   [`DrawIndexedIndirectArgs`] - and bundles) and [`ComputePass`] ([`DispatchIndirectArgs`]).
//! - Render bundles: [`RenderBundleBuilder`] records a [`GpuRenderBundle`] once; a pass replays it.
//! - Queries: timestamp [`GpuQuerySet`]s, [`PassTimestamps`] and the [`GpuTimer`] (where the browser
//!   grants `timestamp-query`); occlusion sets are refused, with the reason.
//! - Frame timings (Not in C#): the [`FrameTimer`] - CPU time for the frame, GPU time per marked
//!   pass and in total, smoothed, beside the renderers' counts ([`FrameTimings`]); GPU values are
//!   "not available" where the browser lacks `timestamp-query`.
//! - Effects (Not in C#): the [`GlowLayer`] (soft additive [`Glow`]s in the 3D scene, optionally
//!   pulsing, in one instanced draw), the [`BloomRenderer`] on the HDR path ([`BloomSettings`]) and
//!   the GPU [`ParticleSystem`] ([`ParticleEmitter`], [`EmitterShape`], [`ParticleSettings`]),
//!   drawn with the glow layer's drawing ([`GlowBlend`]).
//! - 2D: [`SpriteBatch`] - sprites from whole textures or atlas regions, tinted, rotated, scaled
//!   about an origin, in drawing order, with a transform stack and shapes ([`RectangleF`]).
//! - 3D: [`MeshShapes`] (cube, box, plane, sphere) as [`MeshData`] of [`MeshVertex`], [`Mesh`],
//!   [`MeshRenderer`] with [`LitMaterial`]s and [`SceneLighting`] (Blinn-Phong, one key light,
//!   ambient, instancing, opaque then translucent, mirrored instances), a [`Camera`]
//!   (perspective or orthographic) with [`Camera::world_to_canvas`] ([`CanvasProjection`]: a
//!   world point in CSS pixels, for page content over the scene) and [`Camera::canvas_to_ray`]
//!   (a picking [`Ray`], tested against boxes and transformed boxes), [`OrbitCamera`], and
//!   [`OrbitController`], which drives the orbit from the canvas host's input (drag to orbit,
//!   right/middle/Shift drag to pan, wheel, one-finger orbit, two-finger pan and pinch) within
//!   [`OrbitLimits`], auto-orbiting until the first drag, and frames a bounding box
//!   ([`OrbitController::frame_box`]).
//! - Models: [`ModelDataSet`] read from model-data bytes, [`Model`], [`ModelInstance`] and the
//!   [`ModelRenderer`] (glTF metallic-roughness shading, GPU skinning and morph targets; the
//!   opaque and the blended parts in two halves, [`ModelRenderer::end_opaque`] and
//!   [`ModelRenderer::end_blended`], with the skybox between them; instances outside the view
//!   culled against bounds grown at load to hold every pose, [`Model::culling_bounds`] and
//!   [`CullVolume`]; blended double-sided parts drawn back faces then front faces; up to 16
//!   [`LocalLight`]s - [`PointLight`]s and [`SpotLight`]s - and shadows from the sun
//!   ([`SunShadowSettings`]) and up to four spotlights ([`SpotShadowSettings`]), drawn by the
//!   renderer itself with no change to how it is called; level of detail with a fade band,
//!   [`LodInstance`] of [`LodLevel`]s drawn by [`ModelRenderer::draw_lod`] - a dither cross-fade
//!   at each switch and at the cut-off, culled first, scaled by one quality setting); per
//!   instance a tint, an
//!   alpha, an emissive strength and hidden nodes; the application's own textures in a model's
//!   material slots ([`Model::set_material_texture`]), made from PNG or JPEG files
//!   ([`GraphicsContext::create_texture_from_png`] / `_jpeg` / `_image`) or from pixels.
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
//! - Multisampling (MSAA, Not in C#), off by default and set PER TARGET: a
//!   [`HeadlessRenderTarget`]'s sample count ([`HeadlessRenderTarget::set_sample_count`]) and a
//!   canvas layer's [`CanvasFrameTarget`] ([`GraphicsContext::begin_canvas_frame_on`]), 1 or 4
//!   ([`SAMPLE_COUNTS`]), changeable while the page runs; every pass on a 4-sample target ends by
//!   resolving, and every renderer picks its pipelines by the target's [`TargetFormats`].
//! - Water (Not in C#): the simulated [`WaveField`] - a grid of heights stepped on the GPU by the
//!   wave equation at a fixed rate in clock seconds, disturbed by [`WaveDrop`]s, moving
//!   [`WaveWake`]s, seeded [`WaveRain`] and the pointer ([`WaveField::stir`]), walled by a
//!   [`WaveMask`], read as ONE `Rgba16Float` picture ([`WaveField::surface`]: height, slopes,
//!   Laplacian), with its CPU twin [`CpuWaveField`]; and [`OpenWaterWaves`], a sum of travelling
//!   [`GerstnerWave`]s, an exact function of time with `height_at` / `normal_at` on the CPU; and
//!   the water SURFACE for 3D scenes: [`WaterRenderer`] draws [`WaterSurface`]s (shaped by a
//!   [`WaterField`] and open-water waves, looking as their [`WaterLook`] says) on the offscreen
//!   path after a per-frame scene capture - refraction, tint by the real depth of water, caustics,
//!   environment and screen-space reflections, glints, the sun's shadow through
//!   [`ModelRenderer::sun_shadow_map`], an optional sheen.
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
mod effects;
mod frames;
mod gltf;
mod images;
mod internal;
mod lighting;
mod models;
mod pipelines;
mod presentation;
mod queries;
mod resources;
pub mod shaders;
mod visibility;
mod water;

pub use animation::{AnimationLoopMode, AnimationPlayer, AnimationState, PlayOptions};
pub use bundles::{GpuRenderBundle, RenderBundleBuilder, RenderBundleOptions};
pub use common::{Color4, RectangleF};
pub use drawing2d::{CircleOptions, SpriteBatch, SpriteBatchBeginOptions};
pub use drawing3d::{AmbientLighting, Camera, DirectionalLight, LitMaterial, MaterialAlphaMode, MaterialOptions, Mesh, MeshData, MeshRenderer, MeshShapes, MeshVertex, OrbitCamera, OrbitController, OrbitLimits, ProjectionKind, SceneLighting};
pub use drawing3d::{CanvasProjection, Ray};
pub use context::{DepthTextureOptions, ErrorType, GraphicsContext, GraphicsContextOptions, GraphicsError, GraphicsException, RenderTextureOptions, RgbaTextureOptions, ShaderCompilationException, Texture2DOptions};
pub use effects::{BloomRenderer, BloomSettings, EmitterShape, Glow, GlowBlend, GlowFacing, GlowLayer, GlowNearFade, GlowPulse, ParticleEmitter, ParticleSettings, ParticleStretch, ParticleSystem, ParticleSystemOptions};
pub use frames::{BufferRange, ComputePass, ComputePassOptions, DepthRange, DispatchIndirectArgs, DrawIndexedIndirectArgs, DrawIndexedOptions, DrawIndirectArgs, DrawOptions, GraphicsFrame, RenderPass, RenderPassOptions};
pub use gltf::{GltfDocument, GltfExternalFile, GltfFileKind, GltfLoadOptions, GltfModelSet};
pub use images::{AnimatedTexture, AnimatedTextureOptions, AnimatedTextureStorage};
pub use pipelines::{BlendPreset, ColorTargetOptions, ComputePipelineBuilder, CullPreset, DepthOptions, DepthPreset, GpuComputePipeline, GpuPipelineLayout, GpuRenderPipeline, RenderPipelineBuilder, StencilMasks, VertexElement, VertexFormats, VertexLayout, VertexLayoutBuilder, VertexLayoutOptions, VertexType};
pub use models::{AnimationInterpolation, AnimationPath, LodInstance, LodLevel, LodState, Model, ModelAnimation, ModelAnimationChannel, ModelData, ModelDataFormat, ModelDataSet, ModelInstance, ModelLoadOptions, ModelMaterial, ModelMesh, ModelMorphTarget, ModelNode, ModelPoseEdit, ModelPrimitive, ModelRenderStats, ModelRenderer, ModelSkin, ModelSkinVertex, ModelTextureData, ModelTextureKind, ModelTextureSlot, ModelVertex, NodeTransform, SunShadowMap};
pub use lighting::{EnvironmentMap, EnvironmentOptions, FloatImage, LocalLight, PointLight, ProceduralSkyOptions, RadianceHdrDecoder, ShadowMapSize, SkyboxDrawOptions, SkyboxRenderer, SpotLight, SpotShadowSettings, SunShadowSettings, ToneMapOperator, ToneMapper};
pub use presentation::{CanvasFrameTarget, CanvasFrameTargetOptions, HeadlessRenderTarget, HeadlessTargetOptions, RenderTargetView, TargetFormats, SAMPLE_COUNTS};
pub use queries::{FrameTimer, FrameTimerOptions, FrameTimings, GpuQuerySet, GpuTimer, PassTiming, PassTimestamps};
pub use resources::{BindingEntry, BindingKind, BindingLayoutEntry, BindingResource, BufferMapState, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuShader, GpuTexture, SamplerDescription, ShaderDiagnostics, ShaderMessage, ShaderMessageKind, TextureCopyLayout, TextureFormats, TextureRegion};
pub use shaders::{PbrFeatures, PbrShader, ShadowDepthShader};
pub use visibility::CullVolume;
pub use water::{CpuWaveField, GerstnerWave, GerstnerWaveRecord, OpenWaterWaves, WaveDrop, WaveEdge, WaveField, WaveFieldOptions, WaveMask, WaveRain, WaveStir, WaveWake};
pub use water::{WaterCaustics, WaterDetail, WaterField, WaterFrame, WaterLook, WaterRenderStats, WaterRenderer, WaterSheen, WaterSurface};
pub use water::{WaterCoverage, WaterMask};
pub use models::SunShadowCascades;

/// The `glam` crate (vectors, matrices, quaternions), the version `codebrix-webgpu` was built with.
pub use codebrix_webgpu::glam;
/// The `wgpu` crate, the version `codebrix-webgpu` was built with.
pub use codebrix_webgpu::wgpu;
/// The `bytemuck` crate: [`GpuBuffer::write`] takes `bytemuck::Pod` values.
pub use bytemuck;
