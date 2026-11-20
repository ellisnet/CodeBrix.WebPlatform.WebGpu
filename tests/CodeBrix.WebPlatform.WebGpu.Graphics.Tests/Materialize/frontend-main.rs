//! The test frontend of the Graphics package's real-materialize tests: it names the crate
//! codebrix-webgpu-graphics (and codebrix-webgpu, which it builds on), so that
//! `cargo check --offline --target wasm32-unknown-unknown` proves a frontend can build with it
//! from the materialized vendor folder alone. It is compiled, never run. It uses the crate's
//! public items the way an application does: a layer of the canvas host that draws through the
//! GraphicsContext, its resources and its pipeline builder, the sprite batch, and the lit-mesh
//! renderer under a camera driven by the orbit controller from the canvas's input; models loaded
//! from model-data bytes and drawn by the model renderer; the HDR path (an environment from the
//! procedural sky and one from Radiance .hdr bytes, image-based lighting, the skybox, an
//! RGBA16Float HeadlessRenderTarget and the tone mapper); animation playback on the instances
//! (play, cross-fade and blend by name and index, the frame-time and frame-delta updates, a pose
//! edit, channel sampling); the rest of the C# library a browser can do (an animated texture
//! drawn by the sprite batch, a render bundle, a compute pass writing indirect arguments, an
//! indirect draw, occlusion and timestamp queries with the GPU timer, buffer mapping, texture
//! regions and the async texture readback); and the async checks.

use codebrix_webgpu::glam::{Affine2, Mat4, Vec2};
use codebrix_webgpu::{wgpu, CanvasFrame, CanvasInput, CanvasLayers, CanvasRenderer, CanvasTarget, GpuContext, RendererHandle, WebGpuCanvas, WebGpuUnavailable};
use codebrix_webgpu_graphics::shaders::{BuiltInShaderSource, BuiltInShaders};
use codebrix_webgpu_graphics::{
    AnimatedTexture, AnimatedTextureOptions, BufferMapState, BufferRange, ComputePassOptions, DispatchIndirectArgs, DrawIndirectArgs, GpuBuffer, GpuComputePipeline, GpuQuerySet, GpuRenderBundle, GpuTimer, GraphicsContextOptions, PassTimestamps, RenderBundleOptions, RenderPassOptions,
    Texture2DOptions, TextureCopyLayout, TextureRegion,
    BindingEntry, BindingLayoutEntry, BlendPreset, Camera, CircleOptions, Color4, ColorTargetOptions, DepthTextureOptions, DrawOptions, EnvironmentMap, EnvironmentOptions, FloatImage, GpuBindGroup, GpuRenderPipeline, GpuTexture, GraphicsContext,
    AnimationLoopMode, AnimationPlayer, HeadlessRenderTarget, HeadlessTargetOptions, LitMaterial, MaterialOptions, Mesh, MeshRenderer, MeshShapes, Model, ModelDataSet, ModelInstance, ModelLoadOptions, ModelRenderer, NodeTransform, OrbitController, PbrFeatures, PlayOptions, ProceduralSkyOptions, RadianceHdrDecoder,
    RectangleF, RgbaTextureOptions, SamplerDescription, SceneLighting, SkyboxDrawOptions, SkyboxRenderer, SpriteBatch, SpriteBatchBeginOptions, ToneMapOperator, ToneMapper,
};
use dioxus::prelude::*;

struct Layer {
    graphics: GraphicsContext,
    pipeline: Option<GpuRenderPipeline>,
    material: Option<GpuBindGroup>,
}

impl CanvasRenderer for Layer {
    fn prepare(&mut self, _context: &GpuContext, target: &CanvasTarget) {
        let shader = self.graphics.create_shader(BuiltInShaders::SPRITE, Some("sprite"));
        let layout = self.graphics.create_bind_group_layout(None, &[BindingLayoutEntry::texture(0, wgpu::ShaderStages::FRAGMENT), BindingLayoutEntry::sampler(1, wgpu::ShaderStages::FRAGMENT)]);
        let pixels = Color4::CORNFLOWER_BLUE.to_packed_rgba().to_le_bytes();
        if let Ok(texture) = self.graphics.create_texture_from_rgba(1, 1, &pixels, RgbaTextureOptions { srgb: true, ..RgbaTextureOptions::default() }) {
            let sampler = self.graphics.create_sampler(&SamplerDescription::nearest_clamp());
            self.material = Some(self.graphics.create_bind_group(&layout, None, &[BindingEntry::for_texture(0, &texture), BindingEntry::for_sampler(1, &sampler)]));
        }
        self.pipeline = self.graphics.create_render_pipeline(&shader).with_color_target(target.format, ColorTargetOptions::blended(BlendPreset::PremultipliedAlpha)).build().ok();
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let graphics = self.graphics.clone();
        let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
        if let (Ok(mut pass), Some(pipeline)) = (graphics_frame.begin_render_pass(None, None), self.pipeline.as_ref()) {
            pass.set_pipeline(pipeline);
            pass.draw(0, DrawOptions::default());
        };
    }
}

/// A layer with a sprite batch over lit shapes, under an orbit camera.
struct Shapes {
    graphics: GraphicsContext,
    batch: Option<SpriteBatch>,
    meshes: Option<(MeshRenderer, Mesh, LitMaterial)>,
    depth: Option<GpuTexture>,
    controller: OrbitController,
    camera: Camera,
}

impl CanvasRenderer for Shapes {
    fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
        self.batch = SpriteBatch::new(&self.graphics).ok();
        if let (Ok(renderer), Ok(mesh)) = (MeshRenderer::new(&self.graphics), self.graphics.create_mesh(&MeshShapes::sphere(0.5, 32, 16, None), Some("sphere"))) {
            if let Ok(material) = renderer.create_material(Color4::RED, MaterialOptions { double_sided: true, ..MaterialOptions::default() }) {
                self.meshes = Some((renderer, mesh, material));
            }
        }
    }

    fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
        self.depth = self.graphics.create_depth_texture(width, height, DepthTextureOptions::default()).ok();
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        self.controller.update(frame.info.time);
        self.controller.apply(&mut self.camera);
        let ratio = frame.info.pixel_ratio as f32;
        let graphics = self.graphics.clone();
        let mut graphics_frame = graphics.begin_canvas_frame(frame, self.depth.as_ref());
        if let Ok(mut pass) = graphics_frame.begin_render_pass(None, Some(1.0)) {
            if let Some((renderer, mesh, material)) = self.meshes.as_mut() {
                let _ = renderer.begin(&pass, &self.camera, Some(&SceneLighting::default()));
                let _ = renderer.draw_instanced(mesh, material, &[Mat4::IDENTITY, Mat4::from_translation(codebrix_webgpu::glam::Vec3::X)]);
                let _ = renderer.end(&mut pass);
            }
            if let Some(batch) = self.batch.as_mut() {
                let _ = batch.begin(&pass, SpriteBatchBeginOptions { transform: Affine2::from_scale(Vec2::splat(ratio)), blend: BlendPreset::AlphaBlend });
                batch.fill_rectangle(RectangleF::new(0.0, 0.0, 10.0, 10.0), Color4::WHITE);
                batch.draw_circle(Vec2::new(20.0, 20.0), 8.0, Color4::GREEN, CircleOptions::default());
                let _ = batch.end(&mut pass);
            }
        };
        // An application shows these; the frontend is compiled, never run.
        let _errors = graphics.take_errors();
    }

    fn input(&mut self, input: &CanvasInput) {
        self.controller.input(input);
    }
}

/// A layer that loads models from model-data bytes (as an application hands over what the
/// generated `_bytes` call of its backend returned) and draws instances of them.
struct Models {
    graphics: GraphicsContext,
    bytes: Vec<u8>,
    drawn: Option<(ModelRenderer, Vec<Model>, Vec<ModelInstance>)>,
    depth: Option<GpuTexture>,
}

impl CanvasRenderer for Models {
    fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
        let textures = ModelDataSet::read(&self.bytes).map(|set| set.textures.len()).unwrap_or(0);
        if let (Ok(renderer), Ok(models)) = (self.graphics.create_model_renderer(), self.graphics.load_models(&self.bytes, &ModelLoadOptions { label: Some(format!("{textures} textures")), ..ModelLoadOptions::default() })) {
            let mut instances: Vec<ModelInstance> = models.iter().map(|model| model.create_instance_at(Mat4::from_translation(model.bounds_center()))).collect();
            for instance in instances.iter_mut() {
                // Clips by name and by index, a cross-fade and a weighted blend on a higher layer.
                let mut player = instance.animation();
                if player.play_named("walk", PlayOptions::looping(AnimationLoopMode::PingPong)).is_ok() {
                    let _ = player.cross_fade(0, 0.25, PlayOptions { start_time: 0.5, ..PlayOptions::default() });
                    if let Ok(state) = player.blend(0, 0.5, PlayOptions { layer: 1, ..PlayOptions::default() }) {
                        state.fade_to(0.0, 1.0, true);
                    }
                }
                let mut pose = instance.begin_pose_edit();
                if let Some(first) = pose.local_transforms().first_mut() {
                    *first = NodeTransform::IDENTITY;
                }
                let _ = pose.get_morph_weights(0).first_mut().map(|w| *w = 0.5);
            }
            self.drawn = Some((renderer, models, instances));
        }
    }

    fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
        self.depth = self.graphics.create_depth_texture(width, height, DepthTextureOptions::default()).ok();
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let graphics = self.graphics.clone();
        if let Some((_, models, instances)) = self.drawn.as_mut() {
            // The pose at the frame's time, or advanced by the frame's delta.
            AnimationPlayer::update_all_to(instances, frame.info.time);
            AnimationPlayer::update_all(instances, frame.info.delta as f32);
            if let Some(channel) = models.first().and_then(|m| m.animations().first()).and_then(|a| a.channels.first()) {
                let mut value = [0.0f32; 4];
                let _ = channel.sample(frame.info.time as f32, &mut value);
            }
            let _finished: usize = instances.iter_mut().map(|i| i.animation().finished_clips().len()).sum();
        }
        if let (Some((renderer, _, instances)), Some(depth)) = (self.drawn.as_mut(), self.depth.as_ref()) {
            let mut graphics_frame = graphics.begin_canvas_frame(frame, Some(depth));
            if let Ok(mut pass) = graphics_frame.begin_render_pass(None, Some(1.0)) {
                if renderer.begin(&pass, &Camera::new(), Some(&SceneLighting::default())).is_ok() {
                    for instance in instances.iter() {
                        let _ = renderer.draw(instance);
                    }
                    let _ = renderer.end(&mut pass);
                }
            };
        }
    }
}

/// A layer on the HDR path: models lit by an environment (the procedural sky, or one made from
/// Radiance .hdr bytes as an application hands them over), the skybox, all into an RGBA16Float
/// offscreen target, then tone mapped into the canvas frame.
struct Hdr {
    graphics: GraphicsContext,
    hdr_bytes: Vec<u8>,
    parts: Option<(ModelRenderer, SkyboxRenderer, ToneMapper, EnvironmentMap)>,
    target: Option<HeadlessRenderTarget>,
}

impl CanvasRenderer for Hdr {
    fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
        let sky = ProceduralSkyOptions { turbidity: 2.0, ..ProceduralSkyOptions::default() };
        let environment = if RadianceHdrDecoder::is_radiance_hdr(&self.hdr_bytes) {
            self.graphics.create_environment_from_bytes(&self.hdr_bytes, Some(&EnvironmentOptions { cube_size: 256, ..EnvironmentOptions::default() }))
        } else {
            FloatImage::black(64, 32).and_then(|image| self.graphics.create_environment(&image, None)).and_then(|_| self.graphics.create_procedural_sky(Some(&sky), None))
        };
        if let (Ok(models), Ok(skybox), Ok(mut tone), Ok(environment), Ok(_lut)) = (self.graphics.create_model_renderer(), self.graphics.create_skybox_renderer(), self.graphics.create_tone_mapper(), environment, self.graphics.brdf_lut()) {
            tone.operator = ToneMapOperator::KhronosPbrNeutral;
            tone.exposure = 1.2;
            self.parts = Some((models, skybox, tone, environment));
        }
        let _ = self.graphics.default_environment();
    }

    fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
        let options = HeadlessTargetOptions { format: wgpu::TextureFormat::Rgba16Float, with_depth: true, depth_format: wgpu::TextureFormat::Depth24Plus };
        self.target = self.graphics.create_headless_target(width, height, options).ok();
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let graphics = self.graphics.clone();
        if let (Some((models, skybox, tone, environment)), Some(target)) = (self.parts.as_mut(), self.target.as_ref()) {
            let lighting = SceneLighting { environment: Some(environment.clone()), ..SceneLighting::default() };
            let camera = Camera::new();
            let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
            if let Ok(mut pass) = graphics_frame.begin_render_pass_to(&target.view(), Some(Color4::BLACK), Some(1.0)) {
                if models.begin(&pass, &camera, Some(&lighting)).is_ok() {
                    let _ = models.end(&mut pass);
                }
                let _ = skybox.draw(&mut pass, &camera, Some(&lighting));
                let _ = skybox.draw_environment(&mut pass, &camera, environment, SkyboxDrawOptions { rotation: 0.5, ..SkyboxDrawOptions::default() });
            };
            if let Ok(mut pass) = graphics_frame.begin_render_pass(None, None) {
                let _ = tone.apply_in_pass(&mut pass, target.color());
            };
        }
    }
}

/// A layer on the rest of the C# library a browser can do: a compute pass that writes the indirect
/// arguments of the following draw, a render bundle, occlusion queries and pass timestamps, an
/// animated texture drawn by the sprite batch, and the query results resolved for reading.
struct Rest {
    graphics: GraphicsContext,
    parts: Option<RestParts>,
}

struct RestParts {
    compute: GpuComputePipeline,
    compute_group: GpuBindGroup,
    indirect: GpuBuffer,
    dispatch: GpuBuffer,
    bundle: GpuRenderBundle,
    occlusion: GpuQuerySet,
    resolve: GpuBuffer,
    timer: Option<GpuTimer>,
    animated: AnimatedTexture,
    batch: SpriteBatch,
}

impl Rest {
    fn create(&self, target: &CanvasTarget) -> Result<RestParts, codebrix_webgpu_graphics::GraphicsException> {
        let g = &self.graphics;
        let shader = g.create_shader("@group(0) @binding(0) var<storage, read_write> args: array<u32, 4>; @compute @workgroup_size(1) fn main() { args[0] = 3u; args[1] = 1u; }", Some("args"));
        let layout = g.create_bind_group_layout(None, &[BindingLayoutEntry::storage(0, wgpu::ShaderStages::COMPUTE, false)]);
        let indirect = g.create_indirect_buffer_from(&[DrawIndirectArgs::new(3)], None)?;
        let dispatch = g.create_indirect_buffer_from(&[DispatchIndirectArgs::new(1, 1, 1)], None)?;
        let compute_group = g.create_bind_group(&layout, None, &[BindingEntry::for_buffer(0, &indirect)]);
        let compute = g.create_compute_pipeline(&shader).with_bind_group_layouts(&[&layout]).with_label("args").build()?;
        let draw_shader = g.create_shader(BuiltInShaders::SPRITE, None);
        let pipeline = g.create_render_pipeline(&draw_shader).with_color_target(target.format, ColorTargetOptions::default()).build()?;
        let mut recorder = g.create_render_bundle_for_pipeline(&pipeline, Some("bundle"));
        recorder.set_pipeline(&pipeline)?;
        recorder.draw(3, DrawOptions::default());
        recorder.draw_indirect(&indirect, 0)?;
        let bundle = recorder.finish(None);
        let _other = g.create_render_bundle_with_formats(&[target.format], &RenderBundleOptions::default());
        let occlusion = g.create_occlusion_query_set(1, None)?;
        let resolve = g.create_query_resolve_buffer(&occlusion, None)?;
        let timer = if g.has_feature(wgpu::Features::TIMESTAMP_QUERY) { Some(g.create_timer(2, None)?) } else { None };
        let frames = [vec![255u8; 16], vec![0u8; 16]];
        let frame_slices: Vec<&[u8]> = frames.iter().map(Vec::as_slice).collect();
        let animated = g.create_animated_texture(2, 2, &frame_slices, &AnimatedTextureOptions::default())?;
        let batch = SpriteBatch::new(g)?;
        Ok(RestParts { compute, compute_group, indirect, dispatch, bundle, occlusion, resolve, timer, animated, batch })
    }
}

impl CanvasRenderer for Rest {
    fn prepare(&mut self, _context: &GpuContext, target: &CanvasTarget) {
        self.parts = self.create(target).ok();
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let time = frame.info.time;
        let graphics = self.graphics.clone();
        let Some(parts) = self.parts.as_mut() else { return };
        let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
        {
            let timestamps: Option<PassTimestamps> = parts.timer.as_ref().and_then(|timer| timer.pass(1).ok());
            let mut pass = graphics_frame.begin_compute_pass(&ComputePassOptions { label: None, timestamps });
            pass.set_pipeline(&parts.compute);
            pass.set_bind_group(0, &parts.compute_group);
            pass.dispatch(1, 1, 1);
            let _ = pass.dispatch_indirect(&parts.dispatch, 0);
        }
        let options = RenderPassOptions { occlusion_query_set: Some(parts.occlusion.clone()), timestamps: parts.timer.as_ref().and_then(|timer| timer.pass(0).ok()), ..RenderPassOptions::default() };
        if let Ok(mut pass) = graphics_frame.begin_render_pass_with_options(None, None, None, &options) {
            pass.begin_occlusion_query(0);
            pass.execute_bundles(&[&parts.bundle]);
            pass.end_occlusion_query();
            let _ = pass.multi_draw_indirect(&parts.indirect, 0, 1);
            pass.set_blend_constant(Color4::WHITE);
            pass.push_debug_group("sprites");
            if parts.batch.begin(&pass, SpriteBatchBeginOptions::default()).is_ok() {
                parts.batch.draw_animated(&parts.animated, time, RectangleF::new(0.0, 0.0, 8.0, 8.0), Color4::WHITE);
                let _ = parts.batch.end(&mut pass);
            }
            pass.pop_debug_group();
        };
        let _ = graphics_frame.resolve_query_set(&parts.occlusion, 0, 1, &parts.resolve, 0);
        let _ = graphics_frame.clear_buffer(&parts.dispatch, 0, None);
        if let Some(timer) = parts.timer.as_ref() {
            timer.resolve(&mut graphics_frame);
        }
    }
}

/// The async paths of step 10: mapping, texture regions and readback, the timer, awaited builds.
async fn rest_checks(graphics: GraphicsContext) -> usize {
    let mut passed = 0;
    if let Ok(buffer) = graphics.create_mapped_buffer(16, wgpu::BufferUsages::MAP_WRITE, None) {
        if buffer.write_mapped(&[1u32, 2, 3, 4], 0).is_ok() {
            buffer.unmap();
            passed += usize::from(buffer.map_state() == BufferMapState::Unmapped);
            if buffer.map_async(wgpu::MapMode::Write, 0, None).await.is_ok() {
                buffer.unmap();
            }
        }
    }
    let usage = wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING;
    if let Ok(texture) = graphics.create_texture_2d(4, 4, wgpu::TextureFormat::Rgba8Unorm, usage, Texture2DOptions::default()) {
        let region = TextureRegion::new(0, 0, 2, 2);
        if texture.write_region(&region, &[7u8; 16]).is_ok() {
            passed += texture.read_region(&region).await.map(|pixels| pixels.len()).unwrap_or(0);
            passed += texture.read_rgba().await.map(|pixels| pixels.len()).unwrap_or(0);
            passed += texture.read_pixels(0).await.map(|pixels| pixels.len()).unwrap_or(0);
        }
        let mut frame = graphics.begin_frame();
        if let (Ok(staging), Ok(row)) = (graphics.create_buffer(TextureCopyLayout::buffer_size(4, 4, texture.format()).unwrap_or(1024), wgpu::BufferUsages::COPY_DST, None), TextureCopyLayout::aligned_bytes_per_row(4, texture.format())) {
            let _ = frame.copy_texture_to_buffer(&texture, &TextureRegion::whole(&texture, 0), &staging, 0, row);
            let _ = frame.copy_buffer_to_texture_level(&staging, &texture, 0, 0);
            let _ = frame.upload_texture(&texture, &region, &[1u8; 16]);
        }
        let _ = frame.submit();
        graphics.when_submitted_work_done().await;
    }
    if let Ok(target) = graphics.create_headless_target(4, 4, HeadlessTargetOptions::default()) {
        passed += target.read_pixels().await.map(|p| p.len()).unwrap_or(0);
        passed += target.read_depth().await.map(|d| d.len()).unwrap_or(0);
    }
    if let Ok(timer) = graphics.create_timer(1, None) {
        passed += timer.read_milliseconds().await.ok().flatten().map(|ms| ms.len()).unwrap_or(0);
    }
    let shader = graphics.create_shader("@compute @workgroup_size(1) fn main() {}", None);
    passed += usize::from(graphics.create_compute_pipeline(&shader).build_checked().await.is_ok());
    let layout = graphics.create_pipeline_layout(None, &[]);
    passed += usize::from(graphics.create_compute_pipeline(&shader).with_pipeline_layout(&layout).build().is_ok());
    let bundle = graphics.create_render_bundle_with_formats(&[wgpu::TextureFormat::Rgba8Unorm], &RenderBundleOptions::default());
    passed += bundle.finish_checked(None).await.map(|b| b.command_count()).unwrap_or(0);
    let _period = graphics.timestamp_period();
    let _adapter = (graphics.adapter_has_feature(wgpu::Features::TIMESTAMP_QUERY), graphics.adapter_features(), graphics.adapter_limits());
    let _ = BufferRange::default();
    passed
}

/// The context options of a page that uses timestamps where the browser grants them.
fn options_with_timestamps() -> GraphicsContextOptions {
    GraphicsContextOptions { optional_features: wgpu::Features::TIMESTAMP_QUERY, ..GraphicsContextOptions::default() }
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let gpu = use_resource(GpuContext::request);
    let state = gpu.read();
    match &*state {
        None => rsx! { p { "Asking the browser for a WebGPU adapter..." } },
        Some(Err(reason)) => rsx! { WebGpuUnavailable { reason: reason.clone(), class: "notice" } },
        Some(Ok(context)) => rsx! { Scene { context: context.clone() } },
    }
}

#[component]
fn Scene(context: GpuContext) -> Element {
    let graphics = use_hook(|| GraphicsContext::wrap_gpu_context(&context));
    let layers = use_hook(|| {
        let shapes = Shapes { graphics: graphics.clone(), batch: None, meshes: None, depth: None, controller: OrbitController::default(), camera: Camera::new() };
        let models = Models { graphics: graphics.clone(), bytes: Vec::new(), drawn: None, depth: None };
        let hdr = Hdr { graphics: graphics.clone(), hdr_bytes: Vec::new(), parts: None, target: None };
        let rest = Rest { graphics: graphics.clone(), parts: None };
        CanvasLayers::new(vec![RendererHandle::new(Layer { graphics: graphics.clone(), pipeline: None, material: None }), RendererHandle::new(shapes), RendererHandle::new(models), RendererHandle::new(hdr), RendererHandle::new(rest)])
    });
    let mut compiled = use_signal(|| 0usize);
    let mut rest = use_signal(|| 0usize);
    {
        let graphics = graphics.clone();
        use_hook(move || {
            spawn(async move {
                rest.set(rest_checks(graphics).await);
                let _ = GraphicsContext::create(options_with_timestamps()).await.is_ok();
            });
        });
    }
    use_hook(move || {
        spawn(async move {
            for source in BuiltInShaderSource::all(PbrFeatures::every_shader_variant()) {
                if graphics.try_create_shader(&source.source, Some(&source.name)).await.is_ok() {
                    compiled += 1;
                }
            }
        });
    });
    rsx! {
        WebGpuCanvas { context: context.clone(), layers, class: "canvas" }
        p { "{compiled} {rest}" }
    }
}
