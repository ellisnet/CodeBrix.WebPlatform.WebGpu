//! The layer of the "More" scene: the graphics crate's remaining features in one picture, each
//! plain to see.
//!
//! - A PANORAMA BACKGROUND: an equirectangular HDR panorama made in code (a dusk sky with coloured
//!   posts around the horizon and a bright sun), baked into an environment and drawn behind
//!   everything by the SkyboxRenderer; the camera orbits by itself, so the background turns.
//! - A RENDER BUNDLE: a ring of twelve cubes recorded ONCE at start and replayed every frame.
//! - A COMPUTE PASS that visibly drives something: every frame it moves the particles of a
//!   turning double helix and writes the arguments of their draw.
//! - An INDIRECT DRAW: the particles are drawn with the arguments the compute pass wrote - the
//!   number drawn rises and falls with time, decided on the GPU.
//! - ANIMATED TEXTURES: two sprites in the top right corner, the same twelve frames made in code -
//!   one stored as a texture array with GPU mipmaps (looping), one streaming through a ring of
//!   layers (ping-pong).
//! - QUERIES: where the browser grants `timestamp-query`, a GPU timer of the render pass and the
//!   compute pass, resolved every frame and read by the page (more_scene.rs) twice a second.
//!   (Occlusion queries are refused by this version of the graphics crate: `wgpu`'s browser
//!   backend does not pass a pass's occlusion query set on; the page shows the crate's message.)
//!
//! Everything takes its time from the frame (the compute pass, the animated textures, the
//! camera), so pausing the clock freezes the whole picture. It draws straight into the canvas
//! frame, loading the colour and clearing only its own depth.

use std::cell::RefCell;
use std::rc::Rc;

use codebrix_webgpu::glam::{Affine2, Mat4, Vec2, Vec3};
use codebrix_webgpu::{wgpu, CanvasFrame, CanvasInput, CanvasRenderer, CanvasTarget, GpuContext};
use codebrix_webgpu_graphics::{
    bytemuck, AnimatedTexture, AnimatedTextureOptions, AnimatedTextureStorage, AnimationLoopMode, BindingEntry, BindingLayoutEntry, BlendPreset, BufferRange, Camera, Color4, ColorTargetOptions, ComputePassOptions, CullPreset, DepthOptions, DepthTextureOptions,
    DrawIndexedOptions, DrawIndirectArgs, EnvironmentOptions, FloatImage, GpuBindGroup, GpuBuffer, GpuComputePipeline, GpuRenderBundle, GpuRenderPipeline, GpuTexture, GpuTimer, GraphicsContext, GraphicsException, MeshShapes, MeshVertex, OrbitCamera, OrbitController,
    RectangleF, RenderPassOptions, SceneLighting, SkyboxRenderer, SpriteBatch, SpriteBatchBeginOptions, VertexLayoutOptions,
};
use dioxus::prelude::*;

/// The number of particles the compute pass moves.
pub const PARTICLES: u32 = 4096;
/// The side of the animated texture's frames, in pixels.
pub const FRAME_SIZE: u32 = 64;
/// The number of frames of the animated textures.
pub const FRAME_COUNT: usize = 12;
/// The depth format of the layer.
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
/// The size of the uniform block the three shaders share.
const GLOBALS_SIZE: u64 = std::mem::size_of::<Globals>() as u64;

/// The uniform block of the three shaders (`Globals` in the .wgsl files).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Globals {
    view_proj: [f32; 16],
    camera_right: [f32; 4],
    camera_up: [f32; 4],
    time: [f32; 4],
}

// SAFETY: 28 f32 fields, #[repr(C)], no padding: every bit pattern is a valid value.
unsafe impl bytemuck::Zeroable for Globals {}
// SAFETY: as above; the type is Copy and 'static.
unsafe impl bytemuck::Pod for Globals {}

/// What the page reads back (twice a second): the layer hands these out once it has made them.
#[derive(Clone)]
pub struct MoreReadback {
    /// The indirect arguments the compute pass writes.
    pub indirect: Rc<GpuBuffer>,
    /// The GPU timer (slot 0 the render pass, slot 1 the compute pass), where timestamps exist.
    pub timer: Option<GpuTimer>,
}

/// Where the layer puts its [`MoreReadback`] for the page.
pub type ReadbackSlot = Rc<RefCell<Option<MoreReadback>>>;

/// What the page shows about the layer every frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoreView {
    /// The frame number on the clock.
    pub frame: u64,
    /// The frame's time on the clock, in seconds (rounded to milliseconds).
    pub time: f64,
    /// The frame the texture-array animated texture shows.
    pub array_frame: usize,
    /// The frame the streaming animated texture shows.
    pub streaming_frame: usize,
}

/// What the layer draws with.
struct Gpu {
    globals: GpuBuffer,
    bundle: GpuRenderBundle,
    compute: GpuComputePipeline,
    compute_group: GpuBindGroup,
    particles_pipeline: GpuRenderPipeline,
    particles_group: GpuBindGroup,
    indirect: Rc<GpuBuffer>,
    timer: Option<GpuTimer>,
    skybox: SkyboxRenderer,
    lighting: SceneLighting,
    sprites: SpriteBatch,
    animated: AnimatedTexture,
    streaming: AnimatedTexture,
}

/// Draws the "More" scene.
pub struct MoreLayer {
    graphics: GraphicsContext,
    readback: ReadbackSlot,
    gpu: Option<Gpu>,
    depth: Option<GpuTexture>,
    controller: OrbitController,
    camera: Camera,
    view: Signal<MoreView>,
    errors: Signal<Vec<String>>,
}

impl MoreLayer {
    /// A layer drawing with `graphics`, handing its read-back buffers to `readback`, publishing
    /// its view into `view` and every error into `errors`.
    pub fn new(graphics: GraphicsContext, readback: ReadbackSlot, view: Signal<MoreView>, errors: Signal<Vec<String>>) -> MoreLayer {
        let mut orbit = OrbitCamera::new();
        orbit.target = Vec3::new(0.0, 1.4, 0.0);
        orbit.set_distance(10.0);
        orbit.set_pitch(0.28);
        let mut controller = OrbitController::new(orbit);
        controller.auto_orbit_speed = 0.22;
        let mut camera = Camera::new();
        camera.field_of_view = 50f32.to_radians();
        MoreLayer { graphics, readback, gpu: None, depth: None, controller, camera, view, errors }
    }

    fn report(&mut self, error: String) {
        let mut errors = self.errors.write();
        if errors.len() < 20 {
            errors.push(error);
        }
    }
}

/// The twelve frames of the animated textures: a dark disc with a white rim in which a coloured
/// wedge grows by one twelfth per frame. RGBA, sRGB-encoded, straight alpha.
pub fn animation_frames() -> Vec<Vec<u8>> {
    let size = FRAME_SIZE as f32;
    (0..FRAME_COUNT)
        .map(|frame| {
            let mut pixels = Vec::with_capacity((FRAME_SIZE * FRAME_SIZE * 4) as usize);
            let sweep = (frame + 1) as f32 / FRAME_COUNT as f32 * std::f32::consts::TAU;
            for y in 0..FRAME_SIZE {
                for x in 0..FRAME_SIZE {
                    let dx = x as f32 + 0.5 - size / 2.0;
                    let dy = y as f32 + 0.5 - size / 2.0;
                    let distance = (dx * dx + dy * dy).sqrt() / (size / 2.0);
                    // Clockwise from twelve o'clock (y points down).
                    let angle = dx.atan2(-dy).rem_euclid(std::f32::consts::TAU);
                    let pixel: [u8; 4] = if distance > 1.0 {
                        [0, 0, 0, 0]
                    } else if distance > 0.86 {
                        [245, 245, 245, 255]
                    } else if angle <= sweep && distance > 0.18 {
                        [250, 170, 40, 255]
                    } else {
                        [20, 28, 48, 220]
                    };
                    pixels.extend_from_slice(&pixel);
                }
            }
            pixels
        })
        .collect()
}

/// The equirectangular panorama of the background, in linear light: a dusk sky from a warm
/// horizon to a deep blue zenith, a dark ground, twelve coloured posts standing on the horizon,
/// and a small bright sun (well above 1: it is an HDR image).
pub fn panorama(width: u32, height: u32) -> Result<FloatImage, GraphicsException> {
    let mut image = FloatImage::black(width, height)?;
    for y in 0..height {
        let elevation = (0.5 - (y as f32 + 0.5) / height as f32) * std::f32::consts::PI;
        for x in 0..width {
            let heading = (x as f32 + 0.5) / width as f32;
            let mut color = if elevation >= 0.0 {
                let t = (elevation / std::f32::consts::FRAC_PI_2).sqrt();
                Vec3::new(1.1, 0.55, 0.3).lerp(Vec3::new(0.05, 0.09, 0.3), t)
            } else {
                Vec3::new(0.09, 0.08, 0.07)
            };
            let post = heading * 12.0;
            if (post - post.round()).abs() < 0.03 && elevation >= 0.0 && elevation < 0.35 {
                let index = (post.round() as usize) % 12;
                let hue = index as f32 / 12.0;
                color = Vec3::new((hue * 6.0 - 3.0).abs() - 1.0, 2.0 - (hue * 6.0 - 2.0).abs(), 2.0 - (hue * 6.0 - 4.0).abs()).clamp(Vec3::ZERO, Vec3::ONE) * 2.5;
            }
            let sun = Vec2::new((heading - 0.12) * std::f32::consts::TAU, elevation - 0.3).length();
            if sun < 0.06 {
                color = Vec3::new(9.0, 7.0, 4.5);
            }
            image.set_pixel(x, y, color.x, color.y, color.z);
        }
    }
    Ok(image)
}

/// Makes everything the layer draws with, for colour target `format`.
fn create_gpu(graphics: &GraphicsContext, format: wgpu::TextureFormat) -> Result<Gpu, GraphicsException> {
    let globals = graphics.create_uniform_buffer(GLOBALS_SIZE, Some("more globals"))?;

    // The ring of cubes, recorded once into a bundle.
    let cube = MeshShapes::cube(1.0, None);
    let cube_vertices = graphics.create_vertex_buffer(&cube.vertices, Some("more cube vertices"))?;
    let cube_indices = graphics.create_index_buffer_u32(&cube.indices, Some("more cube indices"))?;
    let cubes_shader = graphics.create_shader(include_str!("more_cubes.wgsl"), Some("more cubes"));
    let cubes_layout = graphics.create_bind_group_layout(Some("more cubes"), &[BindingLayoutEntry::uniform(0, wgpu::ShaderStages::VERTEX)]);
    let cubes_group = graphics.create_bind_group(&cubes_layout, Some("more cubes"), &[BindingEntry::for_buffer(0, &globals)]);
    let cubes_pipeline = graphics
        .create_render_pipeline(&cubes_shader)
        .with_vertex_buffer_of::<MeshVertex>(VertexLayoutOptions::default())
        .with_color_target(format, ColorTargetOptions::blended(BlendPreset::Opaque))
        .with_depth(DEPTH_FORMAT, DepthOptions::default())
        .with_culling(CullPreset::Back, wgpu::FrontFace::Ccw)
        .with_bind_group_layouts(&[&cubes_layout])
        .with_label("more cubes")
        .build()?;
    let mut recorder = graphics.create_render_bundle_for_pipeline(&cubes_pipeline, Some("cube ring"));
    recorder.set_pipeline(&cubes_pipeline)?;
    recorder.set_bind_group(0, &cubes_group);
    recorder.set_vertex_buffer(0, &cube_vertices, BufferRange::default());
    recorder.set_index_buffer(&cube_indices, wgpu::IndexFormat::Uint32, BufferRange::default());
    recorder.draw_indexed(cube.indices.len() as u32, DrawIndexedOptions { instance_count: 12, ..DrawIndexedOptions::default() });
    let bundle = recorder.finish(None);

    // The particles: a storage buffer the compute pass writes, the indirect arguments it writes,
    // and the render pipeline that draws them.
    let particles = graphics.create_storage_buffer(PARTICLES as u64 * 32, Some("more particles"))?;
    let indirect = Rc::new(graphics.create_indirect_buffer_from(&[DrawIndirectArgs::new(6)], Some("more particle draw"))?);
    let compute_shader = graphics.create_shader(include_str!("more_compute.wgsl"), Some("more compute"));
    let compute_layout = graphics.create_bind_group_layout(
        Some("more compute"),
        &[BindingLayoutEntry::uniform(0, wgpu::ShaderStages::COMPUTE), BindingLayoutEntry::storage(1, wgpu::ShaderStages::COMPUTE, false), BindingLayoutEntry::storage(2, wgpu::ShaderStages::COMPUTE, false)],
    );
    let compute_group = graphics.create_bind_group(&compute_layout, Some("more compute"), &[BindingEntry::for_buffer(0, &globals), BindingEntry::for_buffer(1, &particles), BindingEntry::for_buffer(2, &indirect)]);
    let compute = graphics.create_compute_pipeline(&compute_shader).with_bind_group_layouts(&[&compute_layout]).with_label("more compute").build()?;
    let particles_shader = graphics.create_shader(include_str!("more_particles.wgsl"), Some("more particles"));
    let particles_layout = graphics.create_bind_group_layout(Some("more particles"), &[BindingLayoutEntry::uniform(0, wgpu::ShaderStages::VERTEX), BindingLayoutEntry::storage(1, wgpu::ShaderStages::VERTEX, true)]);
    let particles_group = graphics.create_bind_group(&particles_layout, Some("more particles"), &[BindingEntry::for_buffer(0, &globals), BindingEntry::for_buffer(1, &particles)]);
    let particles_pipeline = graphics
        .create_render_pipeline(&particles_shader)
        .with_color_target(format, ColorTargetOptions::blended(BlendPreset::Opaque))
        .with_depth(DEPTH_FORMAT, DepthOptions::default())
        .with_bind_group_layouts(&[&particles_layout])
        .with_label("more particles")
        .build()?;

    // The GPU timer where the browser granted timestamps.
    let timer = if graphics.has_feature(wgpu::Features::TIMESTAMP_QUERY) { Some(graphics.create_timer(2, Some("more timer"))?) } else { None };

    // The panorama background.
    let environment = graphics.create_environment(&panorama(1024, 512)?, Some(&EnvironmentOptions { label: Some("more panorama".to_string()), ..EnvironmentOptions::default() }))?;
    let lighting = SceneLighting { environment: Some(environment), ..SceneLighting::default() };
    let skybox = graphics.create_skybox_renderer()?;

    // The animated textures: the same frames, stored two ways.
    let frames = animation_frames();
    let frame_slices: Vec<&[u8]> = frames.iter().map(Vec::as_slice).collect();
    let animated = graphics.create_animated_texture(FRAME_SIZE, FRAME_SIZE, &frame_slices, &AnimatedTextureOptions { mipmaps: true, frame_durations: vec![0.1], label: Some("more wedge".to_string()), ..AnimatedTextureOptions::default() })?;
    let streaming = graphics.create_animated_texture(
        FRAME_SIZE,
        FRAME_SIZE,
        &frame_slices,
        &AnimatedTextureOptions { loop_mode: AnimationLoopMode::PingPong, storage: Some(AnimatedTextureStorage::Streaming), frame_durations: vec![0.08], label: Some("more wedge (streaming)".to_string()), ..AnimatedTextureOptions::default() },
    )?;
    let sprites = graphics.create_sprite_batch()?;
    Ok(Gpu { globals, bundle, compute, compute_group, particles_pipeline, particles_group, indirect, timer, skybox, lighting, sprites, animated, streaming })
}

impl CanvasRenderer for MoreLayer {
    fn prepare(&mut self, _context: &GpuContext, target: &CanvasTarget) {
        match create_gpu(&self.graphics, target.format) {
            Ok(gpu) => {
                *self.readback.borrow_mut() = Some(MoreReadback { indirect: gpu.indirect.clone(), timer: gpu.timer.clone() });
                self.gpu = Some(gpu);
            }
            Err(error) => self.report(error.to_string()),
        }
        for error in self.graphics.take_errors() {
            self.report(error.to_string());
        }
    }

    fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
        match self.graphics.create_depth_texture(width, height, DepthTextureOptions { format: DEPTH_FORMAT, label: Some("more depth".to_string()) }) {
            Ok(depth) => self.depth = Some(depth),
            Err(error) => self.report(error.to_string()),
        }
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let time = frame.info.time;
        let number = frame.info.number;
        let pixel_ratio = frame.info.pixel_ratio as f32;
        let aspect = frame.info.width.max(1) as f32 / frame.info.height.max(1) as f32;
        let css_width = frame.info.width as f32 / pixel_ratio.max(0.01);
        self.controller.update(time);
        self.controller.apply(&mut self.camera);
        let mut failures = Vec::new();
        let mut view = MoreView { frame: number, time: (time * 1000.0).round() / 1000.0, ..MoreView::default() };
        if let (Some(gpu), Some(depth)) = (self.gpu.as_mut(), self.depth.as_ref()) {
            let forward = self.camera.forward();
            let right = forward.cross(Vec3::Y).normalize_or_zero();
            let up = right.cross(forward).normalize_or_zero();
            let view_proj: Mat4 = self.camera.get_view_projection_matrix(aspect);
            let globals = Globals { view_proj: view_proj.to_cols_array(), camera_right: right.extend(0.0).to_array(), camera_up: up.extend(0.0).to_array(), time: [time as f32, PARTICLES as f32, 0.0, 0.0] };
            if let Err(error) = gpu.globals.write_value(&globals, 0) {
                failures.push(error.to_string());
            }
            view.array_frame = gpu.animated.get_frame_index(time);
            view.streaming_frame = gpu.streaming.get_frame_index(time);
            let graphics = self.graphics.clone();
            let camera = &self.camera;
            let result: Result<(), GraphicsException> = (|| {
                let mut graphics_frame = graphics.begin_canvas_frame(frame, Some(depth));
                {
                    // The compute pass first: what it writes, the render pass reads.
                    let timestamps = gpu.timer.as_ref().map(|timer| timer.pass(1)).transpose()?;
                    let mut pass = graphics_frame.begin_compute_pass(&ComputePassOptions { label: Some("more particles".to_string()), timestamps });
                    pass.set_pipeline(&gpu.compute);
                    pass.set_bind_group(0, &gpu.compute_group);
                    pass.dispatch(PARTICLES.div_ceil(64), 1, 1);
                }
                {
                    let timestamps = gpu.timer.as_ref().map(|timer| timer.pass(0)).transpose()?;
                    let options = RenderPassOptions { label: Some("more scene".to_string()), timestamps, ..RenderPassOptions::default() };
                    let mut pass = graphics_frame.begin_render_pass_with_options(None, None, Some(1.0), &options)?;
                    pass.execute_bundle(&gpu.bundle);
                    pass.set_pipeline(&gpu.particles_pipeline);
                    pass.set_bind_group(0, &gpu.particles_group);
                    pass.draw_indirect(&gpu.indirect, 0)?;
                    gpu.skybox.draw(&mut pass, camera, Some(&gpu.lighting))?;
                    gpu.sprites.begin(&pass, SpriteBatchBeginOptions { transform: Affine2::from_scale(Vec2::splat(pixel_ratio)), ..SpriteBatchBeginOptions::default() })?;
                    gpu.sprites.draw_animated(&gpu.animated, time, RectangleF::new(css_width - 112.0, 16.0, 96.0, 96.0), Color4::WHITE);
                    gpu.sprites.draw_animated(&gpu.streaming, time, RectangleF::new(css_width - 224.0, 16.0, 96.0, 96.0), Color4::WHITE);
                    gpu.sprites.end(&mut pass)?;
                }
                if let Some(timer) = gpu.timer.as_ref() {
                    timer.resolve(&mut graphics_frame);
                }
                Ok(())
            })();
            if let Err(error) = result {
                failures.push(error.to_string());
            }
        }
        for failure in failures {
            self.report(failure);
        }
        for error in self.graphics.take_errors() {
            self.report(error.to_string());
        }
        if *self.view.peek() != view {
            self.view.set(view);
        }
    }

    fn input(&mut self, input: &CanvasInput) {
        self.controller.input(input);
    }
}
