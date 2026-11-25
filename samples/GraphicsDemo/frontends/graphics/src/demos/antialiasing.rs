//! "Anti-aliasing": multisampling (MSAA) at 1 or 4 samples per pixel, chosen by a control that
//! takes effect WHILE THE PAGE RUNS, on a scene full of thin, high-contrast edges - rows of fences
//! receding to 120 m, tall thin trees far away and posts against a bright sky - under a camera
//! that turns slowly, so the stair-stepping and shimmer of an un-smoothed edge are easy to see.
//! A SECOND CANVAS beside it draws the same scene on the same clock with its camera following the
//! first canvas's, its anti-aliasing HELD Off (each canvas has its own setting).
//!
//! Two paths:
//! - the HDR path (the shell's): the scene into an RGBA16Float target of 1 or 4 samples
//!   (`HeadlessRenderTarget::set_sample_count`), resolved, bloomed (optional) and tone mapped into
//!   the canvas;
//! - the DIRECT path: the scene straight into the canvas through the layer's `CanvasFrameTarget`
//!   (`begin_canvas_frame_on`, `set_sample_count`), with no skybox, OVER A 2D LAYER BELOW it (the
//!   [`Backdrop`]: bands of colour drawn by a SpriteBatch at 1 sample). At 4 samples the resolved
//!   picture is laid over the canvas, so the layer below must stay visible wherever the scene
//!   drew nothing.
//!
//! The shell's read-outs say how many samples each canvas drew with and how many buffer sets each
//! target has made: switching back to a count used before makes nothing new.

use codebrix_webgpu::glam::{Affine2, Vec2, Vec3};
use codebrix_webgpu::{CanvasFrame, CanvasRenderer, CanvasTarget, GpuContext};
use codebrix_webgpu_graphics::{BloomRenderer, BloomSettings, Color4, GpuTexture, GraphicsContext, GraphicsException, GraphicsFrame, ModelInstance, ModelRenderStats, ModelRenderer, PlayOptions, RectangleF, RenderPass, SceneLighting, SpriteBatch, SpriteBatchBeginOptions};

use crate::demos::kit::Kit;
use crate::demos::{dim_lighting, draw_models, on_circle, placed, world, Control, Demo, DemoFrame, DrawContext, Framing, Readout, Settings};

/// The controls.
pub const CONTROLS: &[Control] = &[
    Control::Choice { key: "aa", label: "Anti-aliasing", options: &[("Off", 1.0), ("4x", 4.0)], default: 4.0 },
    Control::Choice { key: "path", label: "Path", options: &[("HDR, tone mapped", 0.0), ("Direct to the canvas, over a layer below", 1.0)], default: 0.0 },
    Control::Toggle { key: "bloom", label: "Bloom (HDR path)", default: true },
    Control::Toggle { key: "walker", label: "Archer walking", default: true },
];

/// What the second canvas holds: anti-aliasing Off.
pub const COMPARISON: &[(&str, f32)] = &[("aa", 1.0)];

/// What to look at.
pub const LEGEND: &[&str] = &[
    "Left: the setting you choose. Right: the same scene, anti-aliasing held Off.",
    "4x: the fence rails, the far trunks and the horizon are smooth ramps, not stairs, and they do not crawl as the camera turns.",
    "Nothing else changes: same colours, same brightness, same places. Very bright edges on the HDR path stay slightly rough (the resolve averages before tone mapping).",
    "Direct path: no sky is drawn; the bands of colour are a 2D layer BELOW the scene, and they stay visible at 4x.",
    "Switching back and forth is instant; the buffer-set counts stop growing after the first switch.",
];

/// The anti-aliasing demo.
#[derive(Default)]
pub struct AntiAliasingDemo {
    renderer: Option<ModelRenderer>,
    bloom: Option<BloomRenderer>,
    instances: Vec<ModelInstance>,
    archer: usize,
    lighting: SceneLighting,
    samples: u32,
    direct: bool,
    bloom_on: bool,
    walking: bool,
}

impl Demo for AntiAliasingDemo {
    fn build(&mut self, kit: &mut Kit<'_>) -> Result<Framing, String> {
        let ground = kit.ground()?;
        let fence = kit.model("fence")?;
        let tall = kit.model("tree-tall")?;
        let barrel = kit.model("barrel")?;
        let mut instances = vec![ground.create_instance()];
        // Rows of fences, nearer to farther, each a little turned: thin rails and posts against the
        // sky, down to a pixel wide in the distance.
        let (min, max) = (fence.bounds_min(), fence.bounds_max());
        let scale = 2.4;
        let width = ((max.x - min.x) * scale).max(0.5);
        for (row, (z, turn, count)) in [(-5.0f32, 0.0f32, 9), (-12.0, 0.08, 15), (-24.0, -0.05, 25), (-45.0, 0.04, 40), (-80.0, 0.0, 60), (-120.0, -0.03, 80)].into_iter().enumerate() {
            let half = count as f32 * width * 0.5;
            for i in 0..count {
                let x = -half + (i as f32 + 0.5) * width;
                let at = Vec3::new(x * turn.cos(), 0.0, z + x * turn.sin() + if row % 2 == 0 { 0.0 } else { 0.3 });
                instances.push(placed(&fence, scale, -turn, at));
            }
        }
        // Tall thin trees far away, and a few barrels near.
        for i in 0..24 {
            let x = -66.0 + i as f32 * 5.7;
            instances.push(placed(&tall, 2.6, i as f32 * 0.9, Vec3::new(x, 0.0, -62.0 - (i % 3) as f32 * 9.0)));
        }
        for (i, x) in [-6.0f32, -2.5, 3.0, 7.0].into_iter().enumerate() {
            instances.push(placed(&barrel, 2.2, i as f32, Vec3::new(x, 0.0, -2.0 + (i % 2) as f32 * 1.5)));
        }
        let mut walker = kit.model("archer")?.create_instance();
        walker.animation().play_named("walk", PlayOptions::default()).map_err(|e| e.to_string())?;
        self.archer = instances.len();
        instances.push(walker);
        self.instances = instances;
        self.renderer = Some(kit.graphics.create_model_renderer().map_err(|e| e.to_string())?);
        self.bloom = Some(kit.graphics.create_bloom_renderer().map_err(|e| e.to_string())?);
        self.samples = 1;
        Ok(Framing { min: Vec3::new(-8.0, 0.0, -8.0), max: Vec3::new(8.0, 3.0, 4.0), yaw: 0.0, pitch: 0.1, distance_scale: 0.75, auto_orbit: true })
    }

    fn update(&mut self, frame: &DemoFrame<'_>, settings: &Settings) -> Result<(), String> {
        self.samples = if settings.get("aa") >= 2.0 { 4 } else { 1 };
        self.direct = settings.get("path") >= 0.5;
        self.bloom_on = settings.on("bloom");
        self.walking = settings.on("walker");
        let t = if self.walking { frame.time as f32 } else { 0.0 };
        let (at, yaw) = on_circle(Vec3::new(0.0, 0.0, 1.0), 3.0, 0.3 * t);
        let archer = &mut self.instances[self.archer];
        archer.world = world(2.0, yaw, at);
        archer.animation().update_to(if self.walking { frame.time } else { 0.0 });
        // A bright day: on the direct path there is no tone mapping, so the light stays moderate.
        self.lighting = if self.direct { dim_lighting(frame.environment, 0.8, 1.6, Vec3::new(-0.4, -1.0, -0.6)) } else { dim_lighting(frame.environment, 1.2, 3.2, Vec3::new(-0.4, -1.0, -0.6)) };
        if let Some(bloom) = self.bloom.as_mut() {
            bloom.settings = BloomSettings { threshold: 1.5, ..BloomSettings::default() };
        }
        Ok(())
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, context: &mut DrawContext<'_>) -> Result<(), GraphicsException> {
        let Some(renderer) = self.renderer.as_mut() else { return Ok(()) };
        if self.direct {
            // No skybox on the direct path: the layer below shows wherever the scene drew nothing.
            renderer.begin(pass, context.camera, Some(&self.lighting))?;
            for instance in &self.instances {
                renderer.draw(instance)?;
            }
            renderer.end_opaque(pass)?;
            renderer.end_blended(pass)
        } else {
            draw_models(renderer, &self.instances, pass, context, &self.lighting)
        }
    }

    fn after_scene(&mut self, frame: &mut GraphicsFrame<'_>, scene: &GpuTexture, _settings: &Settings) -> Result<(), GraphicsException> {
        // Bloom works on the resolved scene (the HDR target's color()), after its last scene pass.
        match self.bloom.as_mut() {
            Some(bloom) if self.bloom_on => bloom.apply(frame, scene),
            _ => Ok(()),
        }
    }

    fn sample_count(&self, settings: &Settings) -> u32 {
        if settings.get("aa") >= 2.0 { 4 } else { 1 }
    }

    fn direct_to_canvas(&self, settings: &Settings) -> bool {
        settings.get("path") >= 0.5
    }

    fn stats(&self) -> ModelRenderStats {
        self.renderer.as_ref().map(|r| r.frame_stats()).unwrap_or_default()
    }

    fn readouts(&self, out: &mut Vec<Readout>) {
        let stats = self.stats();
        out.push(Readout::new("asked", "Anti-aliasing asked for", if self.samples == 4 { "4x" } else { "Off" }));
        out.push(Readout::new("instances", "Instances (fences, trees, barrels, the archer)", self.instances.len()));
        out.push(Readout::new("draw-calls", "Draw calls", stats.draw_calls));
        out.push(Readout::new("pipelines", "Pipelines (one per features and target: colour, depth, samples)", self.renderer.as_ref().map_or(0, |r| r.pipeline_count())));
        out.push(Readout::new("bloom-on", "Bloom", if self.bloom_on && !self.direct { "on" } else { "off" }));
    }
}

/// The 2D layer BELOW the anti-aliasing scene: horizontal bands of colour and a disc, drawn by a
/// SpriteBatch straight into the canvas (1 sample). The HDR path's tone mapping covers it; the
/// direct path draws over it and leaves it visible wherever the scene drew nothing.
pub struct Backdrop {
    graphics: GraphicsContext,
    sprites: Option<SpriteBatch>,
}

impl Backdrop {
    /// The layer, drawing on `graphics`.
    pub fn new(graphics: GraphicsContext) -> Backdrop {
        Backdrop { graphics, sprites: None }
    }

    /// The colour of band `index` of [`BACKDROP_BANDS`], dusk violet at the top to amber at the bottom.
    pub fn band_color(index: usize) -> Color4 {
        let k = index as f32 / (BACKDROP_BANDS - 1) as f32;
        Color4::new(0.18 + 0.75 * k, 0.12 + 0.38 * k, 0.42 - 0.3 * k, 1.0)
    }
}

/// How many bands the backdrop has.
pub const BACKDROP_BANDS: usize = 12;

impl CanvasRenderer for Backdrop {
    fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
        self.sprites = self.graphics.create_sprite_batch().ok();
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let (width, height) = (frame.info.width as f32, frame.info.height as f32);
        let Some(sprites) = self.sprites.as_mut() else { return };
        let mut graphics_frame = self.graphics.begin_canvas_frame(frame, None);
        let result: Result<(), GraphicsException> = (|| {
            let mut pass = graphics_frame.begin_render_pass(None, None)?;
            sprites.begin(&pass, SpriteBatchBeginOptions { transform: Affine2::IDENTITY, ..SpriteBatchBeginOptions::default() })?;
            let band = height / BACKDROP_BANDS as f32;
            for i in 0..BACKDROP_BANDS {
                sprites.fill_rectangle(RectangleF::new(0.0, i as f32 * band, width, band.ceil()), Backdrop::band_color(i));
            }
            sprites.fill_circle(Vec2::new(width * 0.78, height * 0.22), height * 0.09, Color4::new(1.0, 0.92, 0.7, 1.0), 64);
            sprites.end(&mut pass)
        })();
        // A failure here only loses the backdrop; the demo layer reports the device's errors.
        let _ = result;
    }
}
