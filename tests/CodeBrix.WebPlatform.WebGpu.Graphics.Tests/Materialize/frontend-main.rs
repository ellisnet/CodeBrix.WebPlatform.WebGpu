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
    AnimationLoopMode, AnimationPlayer, HeadlessRenderTarget, HeadlessTargetOptions, LitMaterial, MaterialOptions, Mesh, MeshRenderer, MeshShapes, Model, ModelDataSet, ModelInstance, ModelLoadOptions, ModelRenderer, ModelTextureKind, NodeTransform, OrbitController, PbrFeatures, PlayOptions, ProceduralSkyOptions, RadianceHdrDecoder,
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
        // The same models from a glTF file served as it is (the loader's two steps; see gltf_models below).
        let _from_gltf = gltf_models(&self.graphics, &self.bytes, &std::collections::HashMap::new());
        if let (Ok(renderer), Ok(mut models)) = (self.graphics.create_model_renderer(), self.graphics.load_models(&self.bytes, &ModelLoadOptions { label: Some(format!("{textures} textures")), ..ModelLoadOptions::default() })) {
            // The model facts, and the application's own textures and material options put into
            // the models right after loading (before any instance shares them).
            let _facts: Vec<(bool, bool, usize, usize, usize)> = models.iter().map(|m| (m.has_blended_parts(), m.has_opaque_parts(), m.animation_names().len(), m.node_names().len(), m.animated_nodes().len())).collect();
            let png = self.graphics.create_texture_from_png(&self.bytes, RgbaTextureOptions { srgb: true, mipmaps: true, label: None }).map(std::rc::Rc::new);
            let jpeg = self.graphics.create_texture_from_jpeg(&self.bytes, RgbaTextureOptions::default());
            let image = self.graphics.create_texture_from_image(&self.bytes, RgbaTextureOptions::default());
            for model in models.iter_mut() {
                if let Ok(shared) = &png {
                    let _ = model.set_material_texture(None, ModelTextureKind::BaseColor, shared.clone());
                    let _ = model.set_material_texture(Some(0), ModelTextureKind::Emissive, shared.clone());
                }
                if let Ok(material) = model.material_mut(None) {
                    material.emissive_vertex_color = true;
                }
            }
            if let (Some(model), Ok(jpeg), Ok(image)) = (models.first_mut(), jpeg, image) {
                let _ = model.set_material_texture(None, ModelTextureKind::Normal, jpeg);
                let _ = model.set_material_texture(None, ModelTextureKind::Occlusion, image);
            }
            let mut instances: Vec<ModelInstance> = models.iter().map(|model| model.create_instance_at(Mat4::from_translation(model.bounds_center()))).collect();
            for instance in instances.iter_mut() {
                // The per-instance look and node visibility, and a higher clip limit.
                instance.tint = codebrix_webgpu::glam::Vec3::new(1.0, 0.8, 0.8);
                instance.alpha = 0.75;
                instance.emissive_strength = 2.0;
                instance.set_node_visible(0, false);
                let _ = instance.set_node_visible_named("lid", false);
                let _visible = instance.is_node_visible(0);
                instance.show_all_nodes();
                instance.animation().set_max_states(12);
                let _limit = instance.animation().max_states();
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
            AnimationPlayer::update_all_to_since_zero(instances, frame.info.time);
            if let Some(instance) = instances.first_mut() {
                instance.animation().update_to_since_zero(frame.info.time);
                // Where an animated node is now, by index and by name.
                let animated = instance.model().animated_nodes();
                let _at: Vec<Mat4> = animated.iter().map(|&node| instance.get_node_world_matrix(node)).collect();
                let _named: Option<Mat4> = instance.get_node_world_matrix_named("lid");
            }
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
                // ---- graphics-visibility (second GO): frustum culling with bounds grown at load,
                // the never-cull switch, the culling statistics, the reusable volume test, and the
                // two-pass drawing of blended double-sided parts. ----
                renderer.set_frustum_culling(true);
                renderer.set_two_pass_double_sided(true);
                let _settings: (bool, bool) = (renderer.frustum_culling(), renderer.two_pass_double_sided());
                if let Some(first) = instances.first_mut() {
                    first.never_cull = true;
                }
                let camera = Camera::new();
                if renderer.begin(&pass, &camera, Some(&SceneLighting::default())).is_ok() {
                    // What draw tests: the model's culling bounds, placed by the world matrix,
                    // against the camera's view (a level-of-detail choice can use the same test).
                    let _in_view: Vec<bool> = instances
                        .iter()
                        .map(|instance| {
                            let (min, max) = instance.model().culling_bounds();
                            renderer.view_volume().is_none_or(|volume| volume.intersects_transformed_box(min, max, instance.world))
                        })
                        .collect();
                    for instance in instances.iter() {
                        let _ = renderer.draw(instance);
                    }
                    let _ = renderer.end_opaque(&mut pass);
                    let _ = renderer.end_blended(&mut pass);
                }
                let _culled: usize = renderer.last_culled_instance_count();
                let stats: codebrix_webgpu_graphics::ModelRenderStats = renderer.frame_stats();
                let _per_frame = (stats.draw_calls, stats.instances, stats.culled_instances);
                // The volume test on its own: a camera's view, or a light's from its matrix.
                let volume = codebrix_webgpu_graphics::CullVolume::from_camera(&camera, 16.0 / 9.0);
                let light = codebrix_webgpu_graphics::CullVolume::from_view_projection(camera.get_view_projection_matrix(1.0));
                let _planes = (volume.planes().len(), light.contains_point(camera.target), volume.intersects_box(-codebrix_webgpu::glam::Vec3::ONE, codebrix_webgpu::glam::Vec3::ONE));
                let _features = PbrFeatures::BACK_FACES | PbrFeatures::FRONT_FACES | PbrFeatures::PIPELINE_ONLY;
                // ---- end graphics-visibility ----
                // ---- graphics-lighting (second GO): point lights and spotlights, the sun's shadow
                // and spotlight shadows, the per-instance shadow switches, the shadow statistics. The
                // calls are the ones without shadows: the renderer draws the shadow passes itself. ----
                {
                    use codebrix_webgpu::glam::Vec3;
                    use codebrix_webgpu_graphics::{LocalLight, PointLight, ShadowMapSize, SpotLight, SpotShadowSettings, SunShadowSettings};
                    // (An application moves its lights by the frame's time, read before the frame is begun.)
                    let time = 0.5f32;
                    let mut lighting = SceneLighting::default();
                    // A lamp that moves, and a torch-like spot that casts shadows.
                    lighting.local_lights.push(PointLight::new(Vec3::new(time.sin() * 2.0, 1.5, 0.0), Color4::new(1.0, 0.8, 0.6, 1.0), 4.0, 6.0).into());
                    let spot = SpotLight::new(Vec3::new(0.0, 4.0, 2.0), Vec3::new(0.0, -1.0, -0.5), Color4::WHITE, 20.0, 12.0, 15f32.to_radians(), 25f32.to_radians()).with_shadows(true);
                    lighting.local_lights.push(LocalLight::Spot(spot));
                    let _curves = (LocalLight::falloff(2.0, 6.0), spot.cone(0.3), lighting.local_lights[0].light_at(Vec3::ZERO), lighting.local_lights[1].importance(Camera::new().position));
                    // The sun's shadow over the models' region, with every setting.
                    lighting.sun_shadows = SunShadowSettings { map_size: ShadowMapSize::Size2048, softness: 9, strength: 0.85, ..SunShadowSettings::covering(Vec3::splat(-4.0), Vec3::splat(4.0)) };
                    lighting.spot_shadows = SpotShadowSettings { map_size: ShadowMapSize::Size1024, softness: 4, strength: 1.0 };
                    let _limits = (SceneLighting::MAX_LOCAL_LIGHTS, SceneLighting::MAX_SHADOWED_SPOTLIGHTS, ShadowMapSize::Size4096.texels());
                    if let Some(last) = instances.last_mut() {
                        // A ground that receives but does not cast; the others cast and receive.
                        last.casts_shadows = false;
                        last.receives_shadows = true;
                    }
                    if renderer.begin(&pass, &camera, Some(&lighting)).is_ok() {
                        for instance in instances.iter() {
                            let _ = renderer.draw(instance);
                        }
                        let _ = renderer.end_opaque(&mut pass);
                        let _ = renderer.end_blended(&mut pass);
                    }
                    let stats = renderer.frame_stats();
                    let _shadows = (stats.shadow_passes, stats.shadow_draw_calls, renderer.last_shadow_draw_call_count(), renderer.shadow_pipeline_count());
                    let _variants = (PbrFeatures::LOCAL_LIGHTS | PbrFeatures::SHADOWS, codebrix_webgpu_graphics::ShadowDepthShader::source(PbrFeatures::ALPHA_MASK).len());
                }
                // ---- end graphics-lighting ----
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
                // The documented order: opaque parts, the skybox, then the blended parts.
                if models.begin(&pass, &camera, Some(&lighting)).is_ok() {
                    let _ = models.end_opaque(&mut pass);
                    let _ = skybox.draw(&mut pass, &camera, Some(&lighting));
                    let _ = models.end_blended(&mut pass);
                }
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
        CanvasLayers::new(vec![RendererHandle::new(Layer { graphics: graphics.clone(), pipeline: None, material: None }), RendererHandle::new(shapes), RendererHandle::new(models), RendererHandle::new(hdr), RendererHandle::new(rest), effects_layer::layer(graphics.clone()), msaa_layer::layer(graphics.clone()), lod_layer::layer(graphics.clone()), water_layer::layer(graphics.clone()), water_surface_layer::layer(graphics.clone())])
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

/// The glTF loader as an application uses it: parse the model file, ask which files it names
/// (the application has fetched them into `fetched`, by address - the crate does no networking),
/// build the model data, and make the GPU models; a second model added to the same set later
/// shares the textures the first upload made.
fn gltf_models(graphics: &GraphicsContext, model_file: &[u8], fetched: &std::collections::HashMap<String, Vec<u8>>) -> Result<Vec<Model>, codebrix_webgpu_graphics::GraphicsException> {
    use codebrix_webgpu_graphics::{GltfDocument, GltfExternalFile, GltfFileKind, GltfLoadOptions, GltfModelSet};
    let document = GltfDocument::parse(model_file)?;
    let wanted: Vec<GltfExternalFile> = document.external_files(Some("models/forest"), true);
    let _buffers = wanted.iter().filter(|file| file.kind == GltfFileKind::Buffer).map(|file| (file.index, file.uri.as_str(), file.address.as_str())).count();
    let _notes = (document.is_binary(), document.warnings().len(), document.extensions_used().len(), document.extensions_required().len());
    let options = GltfLoadOptions { max_texture_size: 1024, generate_tangents: true, color_textures_srgb: true, mipmaps: true, load_textures: true, share_vertex_data: true };
    let mut set = GltfModelSet::new();
    let files = |address: &str| fetched.get(address).map(Vec::as_slice);
    let first = set.add(&document, "tree", Some("models/forest"), &files, &options)?;
    let _facts = (set.model_count(), set.texture_count(), set.model_name(first).map(str::len), set.warnings(first).len(), set.byte_size());
    // Everything at once through the ordinary model-data call ...
    let all_at_once = graphics.create_models(&set.data_set(), &ModelLoadOptions::default())?;
    drop(all_at_once);
    // ... or step by step from the set, which keeps the GPU textures for models added later.
    let mut models = set.create_models(graphics, &ModelLoadOptions::default())?;
    set.add(&document, "tree again", Some("models/forest"), &files, &GltfLoadOptions::default())?;
    models.extend(set.create_models(graphics, &ModelLoadOptions::default())?);
    let _shared = set.uploaded_texture("models/forest/Textures/bark.jpg", true, true).is_some();
    Ok(models)
}

// ===== BEGIN graphics-effects block (frame timer, glow layer, bloom, particles) =====
/// The effects as an application uses them, on the HDR path: a frame timer around the frame
/// (CPU time, GPU time of the marked passes, the renderers' counts), opaque parts, the skybox,
/// blended parts, a glow layer with pulsing glows, two particle systems (additive sparks from a
/// moving cone, transparent textured smoke from a box), bloom on the RGBA16Float target, then
/// tone mapping into the canvas frame.
mod effects_layer {
    use codebrix_webgpu::glam::{Mat4, Vec3};
    use codebrix_webgpu::{wgpu, CanvasFrame, CanvasRenderer, CanvasTarget, GpuContext, RendererHandle};
    use codebrix_webgpu_graphics::shaders::EffectShaders;
    use codebrix_webgpu_graphics::{
        BloomRenderer, BloomSettings, Camera, Color4, EmitterShape, FrameTimer, FrameTimerOptions, FrameTimings, Glow, GlowBlend, GlowLayer, GlowPulse, GraphicsContext, GraphicsException, HeadlessRenderTarget, HeadlessTargetOptions, ModelRenderer, ParticleSettings, ParticleSystem, ParticleSystemOptions, PassTiming,
        RenderPassOptions, RgbaTextureOptions, SceneLighting, SkyboxRenderer, ToneMapper,
    };

    struct Parts {
        models: ModelRenderer,
        skybox: SkyboxRenderer,
        tone: ToneMapper,
        bloom: BloomRenderer,
        glows: GlowLayer,
        sparks: ParticleSystem,
        smoke: ParticleSystem,
        timer: FrameTimer,
    }

    struct Effects {
        graphics: GraphicsContext,
        parts: Option<Parts>,
        target: Option<HeadlessRenderTarget>,
        camera: Camera,
    }

    impl Effects {
        fn create(&self) -> Result<Parts, GraphicsException> {
            let g = &self.graphics;
            for (name, source) in EffectShaders::all() {
                let _ = g.create_shader(source, Some(name));
            }
            let mut bloom = g.create_bloom_renderer()?;
            bloom.settings = BloomSettings { threshold: 1.2, intensity: 1.0, ..BloomSettings::default() };
            let _chain = BloomRenderer::chain_sizes(1280, 720, bloom.settings.radius);
            let _bright = BloomRenderer::bright_pass(Vec3::splat(3.0), bloom.settings.threshold, bloom.settings.soft_knee);
            let mut glows = g.create_glow_layer()?;
            glows.set_texture(None);
            let _same = GlowLayer::with_capacity(g, 64)?.pipeline_count();
            let mut sparks = g.create_particle_system(&ParticleSystemOptions { capacity: 4096, seed: 7, ..ParticleSystemOptions::default() })?;
            sparks.emitter.shape = EmitterShape::Cone { radius: 0.05, angle: 0.35 };
            sparks.emitter.rate = 400.0;
            sparks.emitter.speed_min = 2.0;
            sparks.emitter.speed_max = 4.0;
            sparks.settings = ParticleSettings { acceleration: Vec3::new(0.0, -9.81, 0.0), drag: 0.4, intensity: 4.0, ..ParticleSettings::default() };
            let mut smoke = ParticleSystem::new(g, &ParticleSystemOptions { capacity: 512, blend: GlowBlend::Transparent, max_step: 0.05, label: Some("smoke".to_string()), ..ParticleSystemOptions::default() })?;
            smoke.emitter.shape = EmitterShape::Box { half_extents: Vec3::new(0.5, 0.1, 0.5) };
            smoke.emitter.rate = 20.0;
            smoke.emitter.spread = 0.2;
            smoke.emitter.lifetime_min = 3.0;
            smoke.emitter.lifetime_max = 5.0;
            smoke.settings = ParticleSettings { size_at_birth: 0.3, size_at_death: 1.5, color_at_birth: Vec3::splat(0.4), color_at_death: Vec3::splat(0.6), opacity_at_birth: 0.6, opacity_at_death: 0.0, fade_out: 0.5, ..ParticleSettings::default() };
            smoke.set_texture(Some(g.create_texture_from_rgba(1, 1, &[255, 255, 255, 128], RgbaTextureOptions::default())?));
            let _shape = (smoke.texture().is_some(), smoke.blend(), smoke.capacity(), EmitterShape::Point, EmitterShape::Sphere { radius: 1.0 });
            let timer = g.create_frame_timer(FrameTimerOptions { max_passes: 4, smoothing_frames: 60, label: Some("frame".to_string()) })?;
            Ok(Parts { models: g.create_model_renderer()?, skybox: g.create_skybox_renderer()?, tone: g.create_tone_mapper()?, bloom, glows, sparks, smoke, timer })
        }
    }

    impl CanvasRenderer for Effects {
        fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
            self.parts = self.create().ok();
        }

        fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
            let options = HeadlessTargetOptions { format: wgpu::TextureFormat::Rgba16Float, with_depth: true, depth_format: wgpu::TextureFormat::Depth32Float };
            self.target = self.graphics.create_headless_target(width, height, options).ok();
        }

        fn render(&mut self, frame: &mut CanvasFrame<'_>) {
            let (time, delta, restart) = (frame.info.time, frame.info.delta, frame.point_crossed("restart"));
            let graphics = self.graphics.clone();
            let camera = self.camera;
            let (Some(parts), Some(target)) = (self.parts.as_mut(), self.target.as_ref()) else { return };
            parts.timer.begin_frame();
            // The sparks' emitter circles and follows a transform; a burst on the first frame.
            parts.sparks.emitter.place(Mat4::from_translation(Vec3::new(time.cos() as f32, 0.5, time.sin() as f32)));
            if frame.info.number == 1 {
                parts.sparks.burst(200);
            }
            let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
            if restart {
                let _ = parts.sparks.reset(&mut graphics_frame);
            }
            // The particle steps (compute passes) before the pass that draws them.
            let _ = parts.sparks.update(&mut graphics_frame, delta);
            let _ = parts.smoke.update(&mut graphics_frame, delta);
            let lighting = SceneLighting::default();
            let scene = RenderPassOptions { timestamps: parts.timer.pass("scene"), ..RenderPassOptions::default() };
            if let Ok(mut pass) = graphics_frame.begin_render_pass_with_options(Some(&target.view()), Some(Color4::BLACK), Some(1.0), &scene) {
                if parts.models.begin(&pass, &camera, Some(&lighting)).is_ok() {
                    let _ = parts.models.end_opaque(&mut pass);
                    let _ = parts.skybox.draw(&mut pass, &camera, Some(&lighting));
                    let _ = parts.models.end_blended(&mut pass);
                }
                if parts.glows.begin(&pass, &camera, time).is_ok() {
                    parts.glows.add(&Glow::new(Vec3::new(0.0, 1.0, 0.0), 0.6, Vec3::new(1.0, 0.7, 0.3), 3.0));
                    let lamps: Vec<Glow> = (0..8).map(|i| Glow::new(Vec3::new(i as f32 - 3.5, 0.2, -2.0), 0.3, Vec3::new(0.3, 0.6, 1.0), 2.5).with_pulse(GlowPulse { rate: 0.5, depth: 0.8, phase: i as f32 / 8.0 })).collect();
                    parts.glows.add_all(&lamps);
                    let _ = parts.glows.end(&mut pass);
                }
                let _ = parts.sparks.draw(&mut pass, &camera);
                let _ = parts.smoke.draw(&mut pass, &camera);
            };
            // Bloom on the HDR target, after the scene and before tone mapping.
            let _ = parts.bloom.apply(&mut graphics_frame, target.color());
            let tone = RenderPassOptions { timestamps: parts.timer.pass("tone map"), ..RenderPassOptions::default() };
            if let Ok(mut pass) = graphics_frame.begin_render_pass_with_options(None, None, None, &tone) {
                let _ = parts.tone.apply_in_pass(&mut pass, target.color());
            };
            parts.timer.set_count("draw calls", parts.models.last_draw_call_count() as u64);
            parts.timer.set_count("glows", parts.glows.last_glow_count() as u64);
            parts.timer.set_count("bloom levels", parts.bloom.last_level_count() as u64);
            parts.timer.end_frame(&mut graphics_frame);
            let timings: &FrameTimings = parts.timer.timings();
            let _passes: Vec<&PassTiming> = timings.passes.iter().collect();
            let _gpu = (parts.timer.gpu_available(), timings.gpu_total_ms, timings.gpu_total_ms_smoothed, timings.gpu_frame, timings.cpu_ms_smoothed, parts.sparks.spawned_count(), parts.sparks.simulated_time());
            // An application shows this line over the canvas.
            let _overlay: String = timings.to_string();
        }
    }

    /// The effects layer for the canvas.
    pub(super) fn layer(graphics: GraphicsContext) -> RendererHandle {
        RendererHandle::new(Effects { graphics, parts: None, target: None, camera: Camera::new() })
    }
}
// ===== END graphics-effects block =====

// ===== BEGIN graphics-msaa block (multisampling per target: canvas frame target, HDR target) =====
/// Multisampling as an application uses it: a canvas layer keeps a CanvasFrameTarget and switches
/// its sample count while the page runs; the renderers pick their 4-sample pipelines by
/// themselves; a pipeline of the layer's own is kept per target formats; an HDR target drawn with
/// 4 samples is bloomed and tone mapped from its resolved colour into the multisampled canvas
/// frame; a depth-only pipeline with a discarding fragment stage is built.
mod msaa_layer {
    use std::collections::HashMap;

    use codebrix_webgpu::glam::Vec2;
    use codebrix_webgpu::{wgpu, CanvasFrame, CanvasRenderer, CanvasTarget, GpuContext, RendererHandle};
    use codebrix_webgpu_graphics::{
        BloomRenderer, Camera, CanvasFrameTarget, CanvasFrameTargetOptions, CircleOptions, Color4, ColorTargetOptions, DepthOptions, DrawOptions, GlowLayer, GpuRenderPipeline, GpuShader, GraphicsContext, GraphicsException, HeadlessRenderTarget, HeadlessTargetOptions, ModelRenderer, RectangleF, RenderTargetView, SceneLighting,
        SkyboxRenderer, SpriteBatch, SpriteBatchBeginOptions, TargetFormats, ToneMapper, SAMPLE_COUNTS,
    };

    const TRIANGLE: &str = "@vertex fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> { let x = f32(i32(i) - 1) * 0.5; let y = f32(i32(i & 1u) * 2 - 1) * 0.5; return vec4<f32>(x, y, 0.5, 1.0); } @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0, 0.5, 0.0, 1.0); } @fragment fn fs_depth() { }";

    struct Parts {
        canvas: CanvasFrameTarget,
        hdr: Option<HeadlessRenderTarget>,
        models: ModelRenderer,
        skybox: SkyboxRenderer,
        sprites: SpriteBatch,
        glows: GlowLayer,
        bloom: BloomRenderer,
        tone: ToneMapper,
        shader: GpuShader,
        /// The layer's own pipeline, one per target formats (so per sample count).
        own: HashMap<TargetFormats, GpuRenderPipeline>,
    }

    struct Msaa {
        graphics: GraphicsContext,
        parts: Option<Parts>,
        camera: Camera,
        size: Option<(u32, u32)>,
    }

    impl Msaa {
        fn create(&self) -> Result<Parts, GraphicsException> {
            let g = &self.graphics;
            let canvas = g.create_canvas_frame_target(CanvasFrameTargetOptions { sample_count: 4, ..CanvasFrameTargetOptions::default() })?;
            let refused = g.create_canvas_frame_target(CanvasFrameTargetOptions { sample_count: 2, depth_format: None }).is_err();
            let _ = (refused, SAMPLE_COUNTS, canvas.depth_format(), canvas.sample_count());
            let shader = g.create_shader(TRIANGLE, Some("msaa triangle"));
            // A depth-only pipeline whose fragment stage only discards: depth, and no colour target.
            let _depth_only = g.create_render_pipeline(&shader).with_fragment_entry("fs_depth", None).with_depth(wgpu::TextureFormat::Depth32Float, DepthOptions::default()).with_sample_count(4).build()?;
            Ok(Parts { canvas, hdr: None, models: g.create_model_renderer()?, skybox: g.create_skybox_renderer()?, sprites: SpriteBatch::new(g)?, glows: g.create_glow_layer()?, bloom: g.create_bloom_renderer()?, tone: g.create_tone_mapper()?, shader, own: HashMap::new() })
        }
    }

    /// The layer's own pipeline for a pass's target, made once per target formats.
    fn own_pipeline(graphics: &GraphicsContext, shader: &GpuShader, own: &mut HashMap<TargetFormats, GpuRenderPipeline>, target: &RenderTargetView) -> Result<GpuRenderPipeline, GraphicsException> {
        let formats = target.formats();
        if let Some(pipeline) = own.get(&formats) {
            return Ok(pipeline.clone());
        }
        let mut builder = graphics.create_render_pipeline(shader).with_color_target(formats.color, ColorTargetOptions::default()).with_sample_count(formats.sample_count);
        if let Some(depth) = formats.depth {
            builder = builder.with_depth(depth, DepthOptions::default());
        }
        let pipeline = builder.build()?;
        own.insert(formats, pipeline.clone());
        Ok(pipeline)
    }

    impl CanvasRenderer for Msaa {
        fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
            self.parts = self.create().ok();
        }

        fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
            // The canvas frame target follows the canvas by itself; the HDR target is made per size.
            self.size = Some((width, height));
            if let Some(parts) = self.parts.as_mut() {
                parts.hdr = None;
            }
        }

        fn render(&mut self, frame: &mut CanvasFrame<'_>) {
            let graphics = self.graphics.clone();
            let camera = self.camera;
            let time = frame.info.time;
            let Some(parts) = self.parts.as_mut() else { return };
            if let (None, Some((width, height))) = (parts.hdr.as_ref(), self.size) {
                let options = HeadlessTargetOptions { format: wgpu::TextureFormat::Rgba16Float, with_depth: true, depth_format: wgpu::TextureFormat::Depth32Float };
                parts.hdr = graphics.create_headless_target_with_samples(width, height, options, 4).ok();
            }
            // Switched while the page runs: every 120 frames between 4 and 1 samples.
            let count = if (frame.info.number / 120) % 2 == 0 { 4 } else { 1 };
            let _ = parts.canvas.set_sample_count(count);
            if frame.info.number % 600 == 0 {
                parts.canvas.release_unused_buffers();
            }
            let lighting = SceneLighting::default();
            let Ok(mut graphics_frame) = graphics.begin_canvas_frame_on(frame, &mut parts.canvas) else { return };
            // The HDR path: the scene into the 4-sample target (its passes resolve into color()),
            // bloom and tone mapping from the resolved colour into the multisampled canvas frame.
            if let Some(hdr) = parts.hdr.as_mut() {
                let _ = hdr.set_sample_count(count);
                let _ = (hdr.sample_count(), hdr.multisampled_color().is_some(), hdr.depth().is_some());
                if let Ok(mut pass) = graphics_frame.begin_render_pass_to(&hdr.view(), Some(Color4::BLACK), Some(1.0)) {
                    if parts.models.begin(&pass, &camera, Some(&lighting)).is_ok() {
                        let _ = parts.models.end_opaque(&mut pass);
                        let _ = parts.skybox.draw(&mut pass, &camera, Some(&lighting));
                        let _ = parts.models.end_blended(&mut pass);
                    }
                }
                let _ = parts.bloom.apply(&mut graphics_frame, hdr.color());
            }
            let target = graphics_frame.target().cloned();
            if let (Some(target), Some(hdr)) = (target.as_ref(), parts.hdr.as_ref()) {
                let _ = (target.sample_count(), target.is_multisampled(), target.resolve_view().is_some(), target.without_depth().has_depth());
                let _ = parts.tone.apply(&mut graphics_frame, hdr.color(), target);
            }
            // Over it, in a pass that loads: 2D, glows and the layer's own pipeline, all at the
            // canvas frame's sample count.
            if let Ok(mut pass) = graphics_frame.begin_render_pass(None, Some(1.0)) {
                if parts.sprites.begin(&pass, SpriteBatchBeginOptions::default()).is_ok() {
                    parts.sprites.fill_rectangle(RectangleF::new(10.0, 10.0, 120.0, 40.0), Color4::new(0.2, 0.6, 1.0, 1.0));
                    parts.sprites.draw_circle(Vec2::new(200.0, 60.0), 30.0, Color4::new(1.0, 1.0, 1.0, 1.0), CircleOptions::default());
                    let _ = parts.sprites.end(&mut pass);
                }
                if parts.glows.begin(&pass, &camera, time).is_ok() {
                    let _ = parts.glows.end(&mut pass);
                }
                let target = pass.target().clone();
                if let Ok(pipeline) = own_pipeline(&graphics, &parts.shader, &mut parts.own, &target) {
                    if pass.check_pipeline(&pipeline).is_ok() && pass.try_set_pipeline(&pipeline).is_ok() {
                        pass.draw(3, DrawOptions::default());
                    }
                }
            }
            // Dropping the frame lays the resolved layer over the canvas (4 samples).
            drop(graphics_frame);
            let _ = parts.canvas.buffer_sets_made();
        }
    }

    /// The multisampling layer for the canvas.
    pub(super) fn layer(graphics: GraphicsContext) -> RendererHandle {
        RendererHandle::new(Msaa { graphics, parts: None, camera: Camera::new(), size: None })
    }
}
// ===== END graphics-msaa block =====

// ===== BEGIN graphics-lod block (level of detail with a fade band) =====
/// Level of detail as an application uses it: an object of three levels sharing one position - a
/// detailed model out to 40 m, a SET of simpler instances out to 160 m, and a last level with a
/// cut-off at 600 m - each switch and the cut-off with a dither fade band; the shown levels'
/// animation advanced (only theirs), the choice made before drawing with select_lod, the quality
/// scale, the per-object settings, the counts, a 4-sample canvas frame and the sun's shadow (a
/// fading level's shadow fades with it).
mod lod_layer {
    use codebrix_webgpu::glam::{Mat4, Vec3};
    use codebrix_webgpu::{CanvasFrame, CanvasRenderer, CanvasTarget, GpuContext, RendererHandle};
    use codebrix_webgpu_graphics::{AnimationPlayer, Camera, CanvasFrameTarget, CanvasFrameTargetOptions, GraphicsContext, GraphicsException, LodInstance, LodLevel, LodState, ModelLoadOptions, ModelRenderStats, ModelRenderer, SceneLighting, SunShadowSettings};

    struct Parts {
        canvas: CanvasFrameTarget,
        models: ModelRenderer,
        objects: Vec<LodInstance>,
    }

    struct Lod {
        graphics: GraphicsContext,
        bytes: Vec<u8>,
        parts: Option<Parts>,
        camera: Camera,
    }

    impl Lod {
        fn create(&self) -> Result<Parts, GraphicsException> {
            let g = &self.graphics;
            // The application supplies the simpler models: here the detailed model and a simple one
            // from the same model-data bytes.
            let models = g.load_models(&self.bytes, &ModelLoadOptions::default())?;
            let (Some(detailed), Some(simple)) = (models.first(), models.last()) else {
                return Err(GraphicsException::new("no models"));
            };
            let mut objects = Vec::new();
            for row in 0..4 {
                for column in 0..4 {
                    let place = Mat4::from_translation(Vec3::new(column as f32 * 30.0, 0.0, -(row as f32) * 30.0));
                    // Level 1 is a set: three simple instances placed within the object.
                    let set: Vec<_> = (0..3).map(|i| simple.create_instance_at(Mat4::from_translation(Vec3::new(i as f32 - 1.0, 0.0, 0.0)))).collect();
                    let levels = vec![LodLevel::single(detailed.create_instance(), 40.0), LodLevel::new(set, 160.0), LodLevel::single(simple.create_instance_at(Mat4::from_scale(Vec3::splat(3.0))), 600.0)];
                    let mut object = LodInstance::new(place, levels)?;
                    object.fade_band = 0.15;
                    object.hysteresis = LodInstance::DEFAULT_HYSTERESIS;
                    object.never_cull = false;
                    object.set_max_distance(2, f32::INFINITY)?;
                    object.set_max_distance(2, 600.0)?;
                    let _ = (object.levels().len(), object.levels()[0].max_distance(), object.position(), object.bounds(), LodInstance::DEFAULT_FADE_BAND);
                    objects.push(object);
                }
            }
            let canvas = g.create_canvas_frame_target(CanvasFrameTargetOptions { sample_count: 4, ..CanvasFrameTargetOptions::default() })?;
            let mut renderer = g.create_model_renderer()?;
            // Quality: keep detail out further on a fast machine (or less far on a slow one).
            renderer.set_lod_distance_scale(1.25);
            let _scale = renderer.lod_distance_scale();
            Ok(Parts { canvas, models: renderer, objects })
        }
    }

    impl CanvasRenderer for Lod {
        fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
            self.parts = self.create().ok();
        }

        fn render(&mut self, frame: &mut CanvasFrame<'_>) {
            let graphics = self.graphics.clone();
            let time = frame.info.time;
            let camera = self.camera;
            let Some(parts) = self.parts.as_mut() else { return };
            // Animate only what was shown last frame (one frame late, before the frame is begun) ...
            for object in parts.objects.iter_mut() {
                for level in object.shown_levels_mut() {
                    AnimationPlayer::update_all_to(&mut level.instances, time);
                }
            }
            // ... or choose first and animate what is about to be drawn: an object choosing
            // without a renderer (no culling).
            if let Some(first) = parts.objects.first_mut() {
                let state = first.select(camera.position, 1.25);
                let _shown: Vec<(usize, f32)> = state.shown_levels();
            }
            let mut lighting = SceneLighting::default();
            lighting.sun_shadows = SunShadowSettings::covering(Vec3::new(-20.0, -1.0, -120.0), Vec3::new(120.0, 10.0, 20.0));
            let Ok(mut graphics_frame) = graphics.begin_canvas_frame_on(frame, &mut parts.canvas) else { return };
            if let Ok(mut pass) = graphics_frame.begin_render_pass(None, Some(1.0)) {
                if parts.models.begin(&pass, &camera, Some(&lighting)).is_ok() {
                    for object in parts.objects.iter_mut() {
                        // The choice for this pair's camera (culled first), then animation of
                        // exactly the levels about to be drawn, then the draw (which finds the same).
                        if let Ok(state) = parts.models.select_lod(object) {
                            if state != LodState::Culled {
                                for level in object.shown_levels_mut() {
                                    AnimationPlayer::update_all_to(&mut level.instances, time);
                                }
                            }
                        }
                        let drawn = parts.models.draw_lod(object);
                        let _what = match drawn {
                            Ok(LodState::Fading { from, to, progress }) => (from, to, progress),
                            Ok(LodState::Level(level)) => (level, None, 0.0),
                            Ok(LodState::Culled | LodState::BeyondCutOff | LodState::NotDrawn) | Err(_) => (0, None, 0.0),
                        };
                        let state = object.state();
                        let _detail = (state.coverage(0), state.is_shown(1), state.is_fading());
                    }
                    let _ = parts.models.end(&mut pass);
                }
            }
            let stats: ModelRenderStats = parts.models.frame_stats();
            let _counts = (stats.culled_lods, stats.fading_lods, stats.culled_instances, stats.draw_calls);
        }
    }

    /// The level-of-detail layer for the canvas.
    pub(super) fn layer(graphics: GraphicsContext) -> RendererHandle {
        RendererHandle::new(Lod { graphics, bytes: Vec::new(), parts: None, camera: Camera::new() })
    }
}
// ===== END graphics-lod block =====

// ===== BEGIN water-g1 block (wave field, its CPU twin, masks, rain, wakes, stirring, open-water waves) =====
/// The water as an application uses it: a wave field over the canvas, walled by an island and a
/// beach made from a mask, with rain, a circling bobbing wake and the pointer stirring it,
/// stepped by the frame's delta in a compute pass before the pass that reads its picture (a
/// shading pipeline of the layer's own binding `WaveField::surface`); a reset at a time point; a
/// CPU twin; open-water waves floating a point with `height_at` / `normal_at` and their per-frame
/// records for a shader.
mod water_layer {
    use codebrix_webgpu::glam::{Vec2, Vec3};
    use codebrix_webgpu::{wgpu, CanvasFrame, CanvasInput, CanvasRenderer, CanvasTarget, DecodedImage, GpuContext, RendererHandle};
    use codebrix_webgpu_graphics::shaders::WaterShaders;
    use codebrix_webgpu_graphics::{
        BindingEntry, BindingLayoutEntry, ColorTargetOptions, CpuWaveField, DrawOptions, GerstnerWave, GerstnerWaveRecord, GpuBindGroup, GpuRenderPipeline, GraphicsContext, GraphicsException, OpenWaterWaves, SamplerDescription, WaveDrop, WaveEdge, WaveField, WaveFieldOptions, WaveMask, WaveRain, WaveStir, WaveWake,
    };

    const SHADE: &str = r#"
@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var linear: sampler;
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> Out {
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var o: Out;
    o.position = vec4<f32>(p[i], 0.0, 1.0);
    o.uv = vec2<f32>(0.5 * p[i].x + 0.5, 0.5 - 0.5 * p[i].y);
    return o;
}
@fragment fn fs_main(v: Out) -> @location(0) vec4<f32> {
    let s = textureSampleLevel(picture, linear, v.uv, 0.0);
    let n = normalize(vec3<f32>(-s.g, 1.0, -s.b));
    let focus = max(s.a * 0.02, 0.0);
    return vec4<f32>(vec3<f32>(0.05, 0.25, 0.45) * max(n.y, 0.0) + vec3<f32>(focus), 1.0);
}
"#;

    struct Parts {
        field: WaveField,
        twin: CpuWaveField,
        sea: OpenWaterWaves,
        pipeline: GpuRenderPipeline,
        group: GpuBindGroup,
    }

    struct Water {
        graphics: GraphicsContext,
        parts: Option<Parts>,
        stir: WaveStir,
        css_size: Vec2,
    }

    impl Water {
        fn create(&self, format: wgpu::TextureFormat) -> Result<Parts, GraphicsException> {
            let g = &self.graphics;
            for (name, source) in WaterShaders::all() {
                let _ = g.create_shader(source, Some(name));
            }
            let options = WaveFieldOptions { columns: 256, rows: 192, size: Vec2::new(10.0, 7.5), wave_speed: 1.0, damping: 0.6, edge: WaveEdge::Reflect, steps_per_second: 120.0, max_steps_per_frame: 8, label: Some("pond".to_string()) };
            let mut field = g.create_wave_field(&options)?;
            // Shores and islands: a beach ramp on the right, an island, a picture's alpha.
            let mut mask = WaveMask::from_fn(256, 192, |u, _v| ((u - 0.9) * 10.0).clamp(0.0, 1.0));
            mask.add_disc(Vec2::new(0.3, 0.4), 0.06, 1.0);
            mask.set(0, 0, 1.0);
            let picture = DecodedImage { width: 2, height: 2, rgba: vec![0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0] };
            let from_picture = WaveMask::from_picture(&picture, 3, 256, 192)?;
            let _facts = (from_picture.columns(), from_picture.rows(), from_picture.cells().len(), from_picture.get(255, 0));
            field.set_mask(Some(&mask))?;
            field.set_rain(Some(WaveRain { rate: 1.5, radius: (0.04, 0.08), depth: (0.01, 0.03), area: None, seed: 1 }));
            field.drop(WaveDrop { position: Vec2::new(5.0, 3.75), radius: 0.12, depth: 0.04 });
            field.drop(WaveDrop::at(Vec2::new(2.0, 2.0)));
            field.set_wave_speed(1.2)?;
            field.set_damping(0.7)?;
            field.set_edge(WaveEdge::ABSORB);
            field.set_edge(WaveEdge::Absorb { width: 12 });
            field.set_edge(WaveEdge::Wrap);
            field.set_edge(WaveEdge::Reflect);
            let _read_out = (field.options().columns, field.size(), field.cell_size(), field.rain().is_some(), WaveField::MAX_DROPS_PER_UPDATE, WaveField::MAX_WAKES, WaveField::GRID_RANGE, field.context().is_wrapped());
            // The same calls on the CPU twin (for a GPU-against-CPU read-out).
            let mut twin = CpuWaveField::new(&WaveFieldOptions { columns: 64, rows: 64, ..options.clone() })?;
            twin.drop(WaveDrop::default());
            twin.set_wake(0, Some(WaveWake::at(Vec2::ONE)))?;
            twin.set_rain(None);
            twin.set_mask(None)?;
            twin.set_edge(WaveEdge::Wrap);
            twin.set_wave_speed(1.0)?;
            twin.set_damping(0.6)?;
            twin.update(1.0 / 60.0);
            let _twin_facts = (twin.surface().len(), twin.heights().len(), twin.simulated_time(), twin.steps_taken(), twin.last_step_count(), twin.size(), twin.cell_size(), twin.options().rows, twin.height_difference(&[0u8; 8]).is_err());
            twin.reset();
            // Open-water waves: a seeded wind sea and a hand-made one.
            let sea = OpenWaterWaves::from_wind(0.4, 6.0, 8, 3).with_gravity(9.81);
            let swell = OpenWaterWaves::new(vec![GerstnerWave { direction: 1.0, wavelength: 12.0, amplitude: 0.2, steepness: 0.6, phase: 0.0, speed: None }, GerstnerWave::default()])?;
            let _sea_facts = (sea.waves().len(), sea.gravity(), swell.max_height(), OpenWaterWaves::MAX_WAVES, OpenWaterWaves::EARTH_GRAVITY, OpenWaterWaves::displaces_grid(4.0, 0.5), OpenWaterWaves::normal_fade(0.3, 0.1));
            // A pipeline of the layer's own that reads the field's picture.
            let shader = g.create_shader(SHADE, Some("water shade"));
            let layout = g.create_bind_group_layout(Some("water shade"), &[BindingLayoutEntry::texture(0, wgpu::ShaderStages::FRAGMENT), BindingLayoutEntry::sampler(1, wgpu::ShaderStages::FRAGMENT)]);
            let pipeline = g.create_render_pipeline(&shader).with_color_target(format, ColorTargetOptions::default()).with_bind_group_layouts(&[&layout]).build()?;
            let sampler = g.create_sampler(&SamplerDescription::linear_clamp());
            let group = g.create_bind_group(&layout, Some("water shade"), &[BindingEntry::for_texture(0, field.surface()), BindingEntry::for_sampler(1, &sampler)]);
            Ok(Parts { field, twin, sea, pipeline, group })
        }
    }

    impl CanvasRenderer for Water {
        fn prepare(&mut self, _context: &GpuContext, target: &CanvasTarget) {
            self.parts = self.create(target.format).ok();
        }

        fn input(&mut self, input: &CanvasInput) {
            let (css, stir) = (self.css_size, self.stir);
            let Some(parts) = self.parts.as_mut() else { return };
            let size = parts.field.size();
            // The field fills the canvas: CSS pixels scale to field units.
            let to_field = |x: f64, y: f64| {
                let p = Vec2::new(x as f32 / css.x, y as f32 / css.y);
                (css.x > 0.0 && p.cmpge(Vec2::ZERO).all() && p.cmple(Vec2::ONE).all()).then(|| p * size)
            };
            parts.field.stir(input, &stir, &to_field);
            parts.twin.stir(input, &stir, &to_field);
        }

        fn render(&mut self, frame: &mut CanvasFrame<'_>) {
            let (time, delta, restart) = (frame.info.time, frame.info.delta, frame.point_crossed("restart"));
            self.css_size = Vec2::new((frame.info.width as f64 / frame.info.pixel_ratio) as f32, (frame.info.height as f64 / frame.info.pixel_ratio) as f32);
            let graphics = self.graphics.clone();
            let Some(parts) = self.parts.as_mut() else { return };
            // A floating thing circles the pond, leaving a wake and bobbing.
            let t = time as f32;
            let _ = parts.field.set_wake(0, Some(WaveWake { position: Vec2::new(5.0 + 3.0 * (0.3 * t).cos(), 3.75 + 2.5 * (0.3 * t).sin()), radius: 0.15, strength: 0.05, bob: 0.01, bob_rate: 1.2 }));
            let _ = parts.field.set_wake(1, None);
            // A float on the open-water waves: placed by height_at, tilted by normal_at.
            let height = parts.sea.height_at(1.0, 2.0, time);
            let normal: Vec3 = parts.sea.normal_at(1.0, 2.0, time);
            let moved = parts.sea.displacement_at(1.0, 2.0, time);
            let records: Vec<GerstnerWaveRecord> = parts.sea.records(time);
            let _float = (height, normal, moved, records.len(), codebrix_webgpu_graphics::bytemuck::cast_slice::<GerstnerWaveRecord, u8>(&records).len());
            let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
            if restart {
                let _ = parts.field.reset(&mut graphics_frame);
            }
            // The step (a compute pass) before the pass that reads the picture.
            let _ = parts.field.update(&mut graphics_frame, delta);
            if let Ok(mut pass) = graphics_frame.begin_render_pass(None, None) {
                pass.set_pipeline(&parts.pipeline);
                pass.set_bind_group(0, &parts.group);
                pass.draw(3, DrawOptions::default());
            }
            let _read_out = (parts.field.simulated_time(), parts.field.steps_taken(), parts.field.last_step_count(), parts.field.surface().format());
        }
    }

    /// The water layer for the canvas.
    pub(super) fn layer(graphics: GraphicsContext) -> RendererHandle {
        RendererHandle::new(Water { graphics, parts: None, stir: WaveStir::default(), css_size: Vec2::ZERO })
    }
}
// ===== END water-g1 block =====

// ===== BEGIN water-g2 block (the water surface, the scene capture, the sun's shadow map, the light and particle read-outs) =====
/// The water surface as an application uses it, on the HDR path at the canvas's sample setting:
/// pass A draws the opaque parts and the sky; the scene is captured; pass B (loading colour and
/// depth) draws a water surface shaped by a wave field and open-water waves, lit with the sun's
/// shadow map of the model renderer's pair, then finishes that pair's blended parts; bloom and the
/// tone mapper follow. The pointer stirs the field through `WaterSurface::field_point`; a float
/// rides the waves with `height_at` / `normal_at`; the model renderer's light read-outs and the
/// particle system's live bound are read.
mod water_surface_layer {
    use codebrix_webgpu::glam::{Mat4, Vec2, Vec3};
    use codebrix_webgpu::{wgpu, CanvasFrame, CanvasInput, CanvasRenderer, CanvasTarget, GpuContext, RendererHandle};
    use codebrix_webgpu_graphics::shaders::WaterShaders;
    use codebrix_webgpu_graphics::{
        BloomRenderer, Camera, Color4, GraphicsContext, GraphicsException, HeadlessRenderTarget, HeadlessTargetOptions, ModelInstance, ModelLoadOptions, ModelRenderStats, ModelRenderer, OpenWaterWaves, ParticleSystem, ParticleSystemOptions, PointLight, SceneLighting, SkyboxRenderer, SunShadowMap, SunShadowSettings, ToneMapper,
        WaterCaustics, WaterDetail, WaterField, WaterFrame, WaterLook, WaterRenderStats, WaterRenderer, WaterSheen, WaterSurface, WaveField, WaveFieldOptions, WaveStir,
    };

    struct Parts {
        models: ModelRenderer,
        instances: Vec<ModelInstance>,
        skybox: SkyboxRenderer,
        water: WaterRenderer,
        field: WaveField,
        surface: WaterSurface,
        bloom: BloomRenderer,
        tone: ToneMapper,
        particles: ParticleSystem,
        hdr: Option<HeadlessRenderTarget>,
    }

    struct Pond {
        graphics: GraphicsContext,
        parts: Option<Parts>,
        camera: Camera,
        canvas: Vec2,
    }

    impl Pond {
        fn create(&self) -> Result<Parts, GraphicsException> {
            let g = &self.graphics;
            for (name, source) in WaterShaders::all() {
                let _ = g.create_shader(source, Some(name));
            }
            let _sources = (WaterShaders::SURFACE.len(), WaterShaders::CAPTURE.len());
            let models = g.create_model_renderer()?;
            // The application's models, loaded as in the other blocks (none here: the pair still
            // draws the sun's map only when it draws something).
            let instances: Vec<ModelInstance> = g.load_models(&[], &ModelLoadOptions::default()).unwrap_or_default().iter().map(|model| model.create_instance_at(Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0)))).collect();
            let field = g.create_wave_field(&WaveFieldOptions { columns: 256, rows: 256, size: Vec2::new(10.0, 10.0), ..WaveFieldOptions::default() })?;
            let look = WaterLook {
                absorption: Vec3::new(0.45, 0.09, 0.05),
                scattering_color: Vec3::new(0.01, 0.05, 0.08),
                turbidity: 0.12,
                refraction: 0.25,
                reflectivity: 1.0,
                roughness: 0.06,
                sun_glint: 1.0,
                caustics: Some(WaterCaustics { strength: 0.7, scale: 1.4, speed: 0.35, focus: 0.5, depth_fade: 4.0 }),
                screen_reflections: true,
                local_light_highlights: true,
                receives_shadows: true,
                soft_shore: 0.12,
                sheen: WaterSheen { strength: 0.3, ..WaterSheen::default() },
            };
            let surface = WaterSurface {
                center: Vec3::ZERO,
                size: Vec2::new(10.0, 10.0),
                rotation: 0.0,
                grid: 128,
                field: Some(WaterField { height_scale: 1.0, displace: true, ..WaterField::of(&field) }),
                open_waves: Some(OpenWaterWaves::from_wind(0.3, 4.0, 8, 1)),
                detail: WaterDetail { strength: 0.06, wavelength: 0.35, speed: 0.4, seed: 1 },
                look,
            };
            let _limits = (WaterSurface::MAX_GRID, surface.grid_cell(), WaterLook::default() == look, WaterSheen::default().strength);
            let particles = g.create_particle_system(&ParticleSystemOptions::default())?;
            Ok(Parts { models, instances, skybox: g.create_skybox_renderer()?, water: g.create_water_renderer()?, field, surface, bloom: g.create_bloom_renderer()?, tone: g.create_tone_mapper()?, particles, hdr: None })
        }
    }

    impl CanvasRenderer for Pond {
        fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
            self.parts = self.create().ok();
        }

        fn input(&mut self, input: &CanvasInput) {
            let (camera, canvas) = (self.camera, self.canvas);
            let Some(parts) = self.parts.as_mut() else { return };
            let surface = &parts.surface;
            // The pointer stirs the water where its ray meets the surface.
            let to_field = |x: f64, y: f64| surface.field_point(&camera.canvas_to_ray(x as f32, y as f32, canvas.x, canvas.y));
            parts.field.stir(input, &WaveStir::default(), &to_field);
            let _mapped = (surface.to_field(Vec3::ONE), surface.to_world(Vec2::ONE));
        }

        fn render(&mut self, frame: &mut CanvasFrame<'_>) {
            let (time, delta, width, height) = (frame.info.time, frame.info.delta, frame.info.width, frame.info.height);
            self.canvas = Vec2::new((width as f64 / frame.info.pixel_ratio) as f32, (height as f64 / frame.info.pixel_ratio) as f32);
            let graphics = self.graphics.clone();
            let camera = self.camera;
            let Some(parts) = self.parts.as_mut() else { return };
            if parts.hdr.as_ref().is_none_or(|hdr| hdr.width() != width || hdr.height() != height) {
                parts.hdr = graphics.create_headless_target_with_samples(width, height, HeadlessTargetOptions { format: wgpu::TextureFormat::Rgba16Float, ..HeadlessTargetOptions::default() }, 4).ok();
            }
            let Some(hdr) = parts.hdr.as_ref() else { return };
            let mut lighting = SceneLighting::default();
            lighting.sun_shadows = SunShadowSettings::covering(Vec3::splat(-6.0), Vec3::splat(6.0));
            lighting.local_lights.push(PointLight::new(Vec3::new(1.0, 1.0, 1.0), Color4::WHITE, 4.0, 8.0).into());
            // A float on the waves: placed by height_at, tilted by normal_at.
            let float_at = Vec3::new(1.5, parts.surface.height_at(1.5, -2.0, time), -2.0);
            let tilt = parts.surface.normal_at(1.5, -2.0, time);
            let _float = (float_at, tilt);
            let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
            let _ = parts.field.update(&mut graphics_frame, delta);
            let _ = parts.particles.update(&mut graphics_frame, delta);
            let target = hdr.view();
            let sun: Option<SunShadowMap>;
            {
                let Ok(mut pass) = graphics_frame.begin_render_pass_to(&target, Some(Color4::BLACK), Some(1.0)) else { return };
                let _ = parts.models.begin(&pass, &camera, Some(&lighting));
                for instance in &parts.instances {
                    let _ = parts.models.draw(instance);
                }
                let _ = parts.models.end_opaque(&mut pass);
                let _ = parts.skybox.draw(&mut pass, &camera, Some(&lighting));
                sun = parts.models.sun_shadow_map();
            }
            if let Some(map) = &sun {
                let _facts = (&map.view, map.matrix, map.texel, map.depth_bias, map.normal_offset, map.softness, map.strength);
            }
            let _ = parts.water.capture_scene(&mut graphics_frame, hdr);
            {
                let Ok(mut pass) = graphics_frame.begin_render_pass_to(&target, None, None) else { return };
                let water_frame = WaterFrame { camera: &camera, lighting: Some(&lighting), time, sun_shadow: sun.as_ref() };
                let _ = parts.water.draw(&mut pass, &water_frame, &parts.surface);
                // The same pair, finished in this later pass on the same target.
                let _ = parts.models.end_blended(&mut pass);
                let _ = parts.particles.draw(&mut pass, &camera);
            }
            let _ = parts.bloom.apply(&mut graphics_frame, hdr.color());
            if let Some(canvas) = graphics_frame.target().cloned() {
                let _ = parts.tone.apply(&mut graphics_frame, hdr.color(), &canvas);
            }
            let stats: WaterRenderStats = parts.water.last_stats();
            let model_stats: ModelRenderStats = parts.models.frame_stats();
            let _read_out = (
                stats.captures,
                stats.surfaces,
                stats.vertices,
                stats.capture_size,
                stats.local_lights,
                parts.water.pipeline_count(),
                parts.water.context().is_wrapped(),
                model_stats.local_lights,
                model_stats.shadowed_spotlights,
                parts.models.last_local_light_count(),
                parts.models.last_shadowed_spotlight_count(),
                parts.particles.alive_at_most(),
            );
        }
    }

    /// The water-surface layer for the canvas.
    pub(super) fn layer(graphics: GraphicsContext) -> RendererHandle {
        let camera = Camera { position: Vec3::new(0.0, 5.0, 9.0), target: Vec3::ZERO, ..Camera::default() };
        RendererHandle::new(Pond { graphics, parts: None, camera, canvas: Vec2::ONE })
    }
}
// ===== END water-g2 block =====
