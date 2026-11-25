//! "Culling": frustum culling (`ModelRenderer::set_frustum_culling`, on by default) over a field
//! of 576 trees, rocks and barrels, twelve foxes walking among them and the archer, with the
//! camera standing INSIDE the field and turning, so most of it is outside the view at any time.
//! The read-outs give the renderer's counts (`frame_stats()`: draw calls, primitive instances,
//! instances culled) beside the page's OWN count of what lies outside the view (the same test:
//! `CullVolume::from_camera` against each instance's `Model::culling_bounds()` placed by its world),
//! and an OVERVIEW INSET in the corner shows the field from above - drawn by a second renderer with
//! culling off - with every instance the main view skipped tinted red and the camera as a white glow
//! with its view direction. The archer can be marked `never_cull`.

use codebrix_webgpu::glam::Vec3;
use codebrix_webgpu::wgpu;
use codebrix_webgpu_graphics::{Camera, Color4, CullVolume, Glow, GlowLayer, GraphicsException, GraphicsFrame, HeadlessRenderTarget, HeadlessTargetOptions, ModelInstance, ModelRenderStats, ModelRenderer, PlayOptions, ProjectionKind, RectangleF, RenderPass, SceneLighting, SpriteBatch, SpriteBatchBeginOptions, ToneMapOperator, ToneMapper};

use crate::demos::kit::Kit;
use crate::demos::{aspect, dim_lighting, draw_models, on_circle, placed, world, Control, Demo, DemoFrame, DrawContext, Framing, Readout, Settings};

/// The controls.
pub const CONTROLS: &[Control] = &[
    Control::Toggle { key: "culling", label: "Frustum culling", default: true },
    Control::Toggle { key: "never-cull", label: "Never cull the archer", default: false },
    Control::Toggle { key: "overview", label: "Overview inset", default: true },
];

/// What to look at.
pub const LEGEND: &[&str] = &[
    "Turning culling off does not change the picture: only the counts (culled 0, more draws).",
    "The culled count follows the camera as it turns; nothing pops in at the edges.",
    "The inset shows the field from above: red = skipped by the main view, light = drawn, white glow = the camera.",
];

/// The overview inset's size in texels.
const INSET: (u32, u32) = (480, 270);

/// The culling demo.
#[derive(Default)]
pub struct CullingDemo {
    renderer: Option<ModelRenderer>,
    overview_renderer: Option<ModelRenderer>,
    glows: Option<GlowLayer>,
    inset: Option<HeadlessRenderTarget>,
    inset_ldr: Option<HeadlessRenderTarget>,
    inset_tone_mapper: Option<ToneMapper>,
    sprites: Option<SpriteBatch>,
    instances: Vec<ModelInstance>,
    foxes: Vec<usize>,
    archer: usize,
    outside: Vec<bool>,
    lighting: SceneLighting,
    overview: bool,
    camera_at: Vec3,
    camera_forward: Vec3,
}

impl Demo for CullingDemo {
    fn build(&mut self, kit: &mut Kit<'_>) -> Result<Framing, String> {
        let ground = kit.ground()?;
        let mut instances = vec![ground.create_instance()];
        let kinds = ["tree", "tree-tall", "tree-autumn", "rock-a", "rock-b", "barrel"];
        let scales = [2.6, 2.6, 2.8, 2.2, 2.0, 2.4];
        let models: Vec<_> = kinds.iter().map(|name| kit.model(name)).collect::<Result<_, _>>()?;
        for row in 0..24 {
            for column in 0..24 {
                let k = (row * 7 + column * 3) % kinds.len();
                let jitter = Vec3::new(((row * 13 + column * 7) % 5) as f32 * 0.3 - 0.6, 0.0, ((row * 3 + column * 11) % 5) as f32 * 0.3 - 0.6);
                let at = Vec3::new(-46.0 + column as f32 * 4.0, 0.0, -46.0 + row as f32 * 4.0) + jitter;
                // Leave the middle clear for the camera.
                if at.length() < 5.0 {
                    continue;
                }
                instances.push(placed(&models[k], scales[k], (row * 5 + column) as f32 * 0.7, at));
            }
        }
        let fox = kit.model("fox")?;
        for i in 0..12 {
            let mut walker = fox.create_instance();
            walker.animation().play_named("walk", PlayOptions { start_time: i as f32 * 0.17, ..PlayOptions::default() }).map_err(|e| e.to_string())?;
            self.foxes.push(instances.len());
            instances.push(walker);
        }
        let mut archer = kit.model("archer")?.create_instance();
        archer.animation().play_named("walk", PlayOptions::default()).map_err(|e| e.to_string())?;
        self.archer = instances.len();
        instances.push(archer);
        self.instances = instances;
        self.renderer = Some(kit.graphics.create_model_renderer().map_err(|e| e.to_string())?);
        let mut overview = kit.graphics.create_model_renderer().map_err(|e| e.to_string())?;
        overview.set_frustum_culling(false);
        self.overview_renderer = Some(overview);
        self.glows = Some(kit.graphics.create_glow_layer().map_err(|e| e.to_string())?);
        let options = HeadlessTargetOptions { format: wgpu::TextureFormat::Rgba16Float, with_depth: true, depth_format: wgpu::TextureFormat::Depth24Plus };
        self.inset = Some(kit.graphics.create_headless_target(INSET.0, INSET.1, options).map_err(|e| e.to_string())?);
        // The inset is tone mapped into an 8-bit picture of its own size (the tone mapper fills a
        // whole target of its source's size), which a sprite batch then puts in the corner.
        let ldr = HeadlessTargetOptions { format: wgpu::TextureFormat::Rgba8UnormSrgb, with_depth: false, depth_format: wgpu::TextureFormat::Depth24Plus };
        self.inset_ldr = Some(kit.graphics.create_headless_target(INSET.0, INSET.1, ldr).map_err(|e| e.to_string())?);
        let mut tone_mapper = kit.graphics.create_tone_mapper().map_err(|e| e.to_string())?;
        tone_mapper.operator = ToneMapOperator::KhronosPbrNeutral;
        self.inset_tone_mapper = Some(tone_mapper);
        self.sprites = Some(kit.graphics.create_sprite_batch().map_err(|e| e.to_string())?);
        Ok(Framing { min: Vec3::new(-3.0, 0.0, -3.0), max: Vec3::new(3.0, 2.0, 3.0), yaw: 0.0, pitch: 0.12, distance_scale: 0.9, auto_orbit: true })
    }

    fn update(&mut self, frame: &DemoFrame<'_>, settings: &Settings) -> Result<(), String> {
        let t = frame.time as f32;
        for (k, &index) in self.foxes.iter().enumerate() {
            let (at, yaw) = on_circle(Vec3::ZERO, 8.0 + (k % 4) as f32 * 7.0, 0.12 * t + k as f32 * 0.52);
            let fox = &mut self.instances[index];
            fox.world = world(0.55, yaw, at);
            fox.animation().update_to(frame.time);
        }
        let (at, yaw) = on_circle(Vec3::ZERO, 6.0, 0.25 * t);
        let archer = &mut self.instances[self.archer];
        archer.world = world(2.0, yaw, at);
        archer.never_cull = settings.on("never-cull");
        archer.animation().update_to(frame.time);
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.set_frustum_culling(settings.on("culling"));
        }
        // The page's own test of each instance against the camera's view (the renderer's rule).
        let view = CullVolume::from_camera(frame.camera, aspect(frame.css));
        self.outside = self.instances.iter().map(|i| {
            let (min, max) = i.model().culling_bounds();
            !view.intersects_transformed_box(min, max, i.world)
        }).collect();
        self.overview = settings.on("overview");
        self.camera_at = frame.camera.position;
        self.camera_forward = (frame.camera.target - frame.camera.position).normalize_or_zero();
        self.lighting = dim_lighting(frame.environment, 1.0, 2.5, Vec3::new(-0.4, -1.0, -0.3));
        Ok(())
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, context: &mut DrawContext<'_>) -> Result<(), GraphicsException> {
        match self.renderer.as_mut() {
            Some(renderer) => draw_models(renderer, &self.instances, pass, context, &self.lighting),
            None => Ok(()),
        }
    }

    fn before_tone_map(&mut self, frame: &mut GraphicsFrame<'_>, _camera: &Camera, _settings: &Settings) -> Result<(), GraphicsException> {
        let (Some(renderer), Some(inset), Some(glows)) = (self.overview_renderer.as_mut(), self.inset.as_ref(), self.glows.as_mut()) else {
            return Ok(());
        };
        if !self.overview {
            return Ok(());
        }
        let mut camera = Camera::new();
        camera.projection = ProjectionKind::Orthographic;
        camera.position = Vec3::new(0.0, 150.0, 0.0);
        camera.target = Vec3::ZERO;
        camera.up = Vec3::NEG_Z;
        camera.orthographic_height = 70.0;
        camera.near_plane = 1.0;
        camera.far_plane = 400.0;
        let mut pass = frame.begin_render_pass_to(&inset.view(), Some(Color4::new(0.02, 0.03, 0.05, 1.0)), Some(1.0))?;
        renderer.begin(&pass, &camera, Some(&self.lighting))?;
        // Every instance but the ground: those the main view skipped tinted red, the others made
        // light (the tint is taken at draw, so the main view's own draws are not affected).
        for (index, instance) in self.instances.iter_mut().enumerate().skip(1) {
            let skipped = self.outside.get(index).copied().unwrap_or(false) && !instance.never_cull;
            let tint = instance.tint;
            instance.tint = if skipped { Vec3::new(1.0, 0.12, 0.1) } else { Vec3::new(1.6, 1.6, 1.6) };
            let result = renderer.draw(instance);
            instance.tint = tint;
            result?;
        }
        renderer.end(&mut pass)?;
        glows.begin(&pass, &camera, 0.0)?;
        glows.add(&Glow::new(self.camera_at + Vec3::Y * 3.0, 5.0, Vec3::ONE, 6.0));
        let flat = Vec3::new(self.camera_forward.x, 0.0, self.camera_forward.z).normalize_or_zero();
        for step in 1..12 {
            glows.add(&Glow::new(self.camera_at + flat * step as f32 * 2.5 + Vec3::Y * 3.0, 2.0, Vec3::new(0.6, 0.9, 1.0), 3.0));
        }
        glows.end(&mut pass)?;
        drop(pass);
        match (self.inset_tone_mapper.as_mut(), self.inset_ldr.as_ref()) {
            (Some(tone_mapper), Some(ldr)) => tone_mapper.apply(frame, inset.color(), &ldr.view()),
            _ => Ok(()),
        }
    }

    fn in_tone_map(&mut self, pass: &mut RenderPass<'_>, _tone_mapper: &mut ToneMapper, size: (u32, u32), _settings: &Settings) -> Result<(), GraphicsException> {
        let (Some(ldr), Some(sprites)) = (self.inset_ldr.as_ref(), self.sprites.as_mut()) else { return Ok(()) };
        if !self.overview {
            return Ok(());
        }
        // The inset in the bottom-right corner, a third of the canvas wide, with a frame.
        let width = size.0 as f32 / 3.0;
        let height = width * INSET.1 as f32 / INSET.0 as f32;
        let place = RectangleF::new(size.0 as f32 - width - 8.0, size.1 as f32 - height - 8.0, width, height);
        sprites.begin(pass, SpriteBatchBeginOptions::default())?;
        sprites.draw(ldr.color(), place, Color4::WHITE);
        sprites.draw_rectangle(place, Color4::new(0.8, 0.85, 1.0, 1.0), 2.0);
        sprites.end(pass)
    }

    fn stats(&self) -> ModelRenderStats {
        self.renderer.as_ref().map(|r| r.frame_stats()).unwrap_or_default()
    }

    fn readouts(&self, out: &mut Vec<Readout>) {
        let stats = self.stats();
        let predicted = self.outside.iter().zip(self.instances.iter()).filter(|(outside, instance)| **outside && !instance.never_cull).count();
        let culling = self.renderer.as_ref().is_some_and(|r| r.frustum_culling());
        out.push(Readout::new("queued", "Instances queued", self.instances.len()));
        out.push(Readout::new("culled", "Culled by the renderer", stats.culled_instances));
        out.push(Readout::new("outside", "Outside the view (the page's own test, never_cull excluded)", predicted));
        out.push(Readout::new("agree", "The two agree", if !culling { "culling is off" } else if predicted == stats.culled_instances { "yes" } else { "NO" }));
        out.push(Readout::new("drawn", "Primitive instances drawn", stats.instances));
        out.push(Readout::new("draw-calls", "Draw calls", stats.draw_calls));
        out.push(Readout::new("archer-outside", "The archer is outside the view", if self.outside.get(self.archer).copied().unwrap_or(false) { "yes" } else { "no" }));
    }
}
