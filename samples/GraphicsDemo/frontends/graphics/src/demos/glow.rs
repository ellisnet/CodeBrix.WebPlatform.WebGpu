//! "Glow and bloom": the glow layer (`GlowLayer`: soft additive sprites in the 3D scene, every
//! glow of a frame in ONE instanced draw, pulses worked out on the GPU from the frame's time) and
//! the bloom renderer (`BloomRenderer`: bright parts of the HDR picture spread into a soft halo,
//! after the scene pass and before tone mapping), at night on the HDR path. EMISSIVE models: a copy
//! of the Avian and of the camp fire whose emissive was set to their own colour map (right after
//! loading), their strength per instance (`emissive_strength`); the Avian flies a circle playing its
//! clip with a glow riding on each wing tip (the nodes' world matrices). A row of eight blue lamps
//! pulses out of step, a large warm glow pulses over the fire, and a swarm of fireflies (hundreds of
//! glows, still one draw) drifts round the scene.

use codebrix_webgpu::glam::Vec3;
use codebrix_webgpu_graphics::{BloomRenderer, BloomSettings, GpuTexture, GraphicsException, GraphicsFrame, Glow, GlowLayer, GlowPulse, ModelInstance, ModelRenderStats, ModelRenderer, PlayOptions, RenderPass, SceneLighting};

use crate::demos::kit::Kit;
use crate::demos::{dim_lighting, draw_models, on_circle, placed, world, Control, Demo, DemoFrame, DrawContext, Framing, Readout, Settings};

/// The controls.
pub const CONTROLS: &[Control] = &[
    Control::Toggle { key: "bloom", label: "Bloom", default: true },
    Control::Slider { key: "threshold", label: "Bloom threshold", min: 0.5, max: 5.0, step: 0.05, default: 1.5 },
    Control::Slider { key: "knee", label: "Soft knee", min: 0.0, max: 1.0, step: 0.05, default: 0.25 },
    Control::Slider { key: "bloom-intensity", label: "Bloom intensity", min: 0.0, max: 3.0, step: 0.05, default: 0.8 },
    Control::Slider { key: "radius", label: "Bloom radius (levels)", min: 1.0, max: 8.0, step: 1.0, default: 6.0 },
    Control::Slider { key: "glow-intensity", label: "Glow intensity", min: 0.0, max: 10.0, step: 0.1, default: 3.0 },
    Control::Slider { key: "glow-size", label: "Glow size", min: 0.25, max: 3.0, step: 0.05, default: 1.0 },
    Control::Slider { key: "pulse-rate", label: "Pulse rate (per s)", min: 0.0, max: 3.0, step: 0.05, default: 0.5 },
    Control::Slider { key: "pulse-depth", label: "Pulse depth", min: 0.0, max: 1.0, step: 0.05, default: 0.8 },
    Control::Slider { key: "fireflies", label: "Fireflies", min: 0.0, max: 2000.0, step: 10.0, default: 300.0 },
    Control::Slider { key: "emissive", label: "Emissive strength (Avian, fire)", min: 0.0, max: 10.0, step: 0.1, default: 4.0 },
];

/// What to look at.
pub const LEGEND: &[&str] = &[
    "Glows are round and soft, brightest in the middle, no square edges; hidden behind solid things.",
    "The lamps pulse out of step; pausing the clock freezes the pulse.",
    "The Avian carries a glow on each wing tip; however many fireflies, the glows are one draw.",
    "Bloom: the bright glows and the glowing Avian get a wide soft halo; lit surfaces do not.",
];

/// The glow and bloom demo.
#[derive(Default)]
pub struct GlowDemo {
    renderer: Option<ModelRenderer>,
    glows: Option<GlowLayer>,
    bloom: Option<BloomRenderer>,
    instances: Vec<ModelInstance>,
    avian: usize,
    fire: usize,
    frame_glows: Vec<Glow>,
    lighting: SceneLighting,
    bloom_on: bool,
    wing_tips: usize,
}

impl Demo for GlowDemo {
    fn build(&mut self, kit: &mut Kit<'_>) -> Result<Framing, String> {
        let glowing = |name: &str, factor: Vec3, kit: &mut Kit<'_>| {
            kit.changed(name, move |model| {
                let material = model.material_mut(Some(0))?;
                material.emissive_texture = material.base_color_texture.clone();
                material.emissive_factor = factor;
                Ok(())
            })
        };
        let avian = glowing("avian", Vec3::ONE, kit)?;
        let fire = glowing("campfire", Vec3::new(1.0, 0.45, 0.15), kit)?;
        let ground = kit.ground()?;
        let mut instances = vec![ground.create_instance()];
        for (name, scale, yaw, at) in [("tree", 2.6, 0.3, Vec3::new(-5.0, 0.0, -4.0)), ("tree-tall", 2.6, 1.1, Vec3::new(4.5, 0.0, -5.0)), ("tree-autumn", 2.8, 2.0, Vec3::new(-6.0, 0.0, 3.5)), ("rock-a", 2.2, 0.4, Vec3::new(3.0, 0.0, 3.0)), ("barrel", 2.2, 0.0, Vec3::new(1.8, 0.0, 0.6)), ("fox", 0.55, 2.5, Vec3::new(-1.6, 0.0, 1.4))] {
            instances.push(placed(&kit.model(name)?, scale, yaw, at));
        }
        self.fire = instances.len();
        instances.push(placed(&fire, 2.4, 0.0, Vec3::ZERO));
        let mut bird = avian.create_instance();
        bird.animation().play_named("fly", PlayOptions::default()).map_err(|e| e.to_string())?;
        self.avian = instances.len();
        instances.push(bird);
        self.instances = instances;
        self.renderer = Some(kit.graphics.create_model_renderer().map_err(|e| e.to_string())?);
        self.glows = Some(kit.graphics.create_glow_layer().map_err(|e| e.to_string())?);
        self.bloom = Some(kit.graphics.create_bloom_renderer().map_err(|e| e.to_string())?);
        Ok(Framing { min: Vec3::new(-7.0, 0.0, -6.0), max: Vec3::new(7.0, 4.0, 6.0), yaw: 0.4, pitch: 0.3, distance_scale: 0.6, auto_orbit: true })
    }

    fn update(&mut self, frame: &DemoFrame<'_>, settings: &Settings) -> Result<(), String> {
        let t = frame.time as f32;
        let (at, yaw) = on_circle(Vec3::new(0.0, 3.2, 0.0), 4.0, -0.45 * t);
        let bird = &mut self.instances[self.avian];
        bird.world = world(0.36, yaw + std::f32::consts::PI, at);
        bird.animation().update_to(frame.time);
        let emissive = settings.get("emissive");
        self.instances[self.avian].emissive_strength = emissive;
        self.instances[self.fire].emissive_strength = emissive;
        let intensity = settings.get("glow-intensity");
        let size = settings.get("glow-size");
        let pulse = |phase: f32| GlowPulse { rate: settings.get("pulse-rate"), depth: settings.get("pulse-depth"), phase };
        let mut glows = Vec::new();
        for i in 0..8 {
            glows.push(Glow::new(Vec3::new(i as f32 * 1.4 - 4.9, 0.35, -3.0), 0.5 * size, Vec3::new(0.3, 0.6, 1.0), intensity).with_pulse(pulse(i as f32 / 8.0)));
        }
        glows.push(Glow::new(Vec3::new(0.0, 0.9, 0.0), 2.2 * size, Vec3::new(1.0, 0.6, 0.25), intensity * 0.8).with_pulse(GlowPulse { rate: 0.7, depth: 0.35, phase: 0.0 }));
        // A glow on each wing tip of the Avian, from the nodes' current world matrices.
        let bird = &self.instances[self.avian];
        self.wing_tips = 0;
        for name in ["wing-tip-left", "wing-tip-right"] {
            if let Some(matrix) = bird.get_node_world_matrix_named(name) {
                glows.push(Glow::new(matrix.transform_point3(Vec3::ZERO), 0.45 * size, Vec3::new(1.0, 0.95, 0.6), intensity * 1.2));
                self.wing_tips += 1;
            }
        }
        let fireflies = settings.get("fireflies").round() as usize;
        for i in 0..fireflies {
            let k = i as f32;
            let angle = k * 2.399 + t * (0.05 + (k * 0.37).fract() * 0.1);
            let radius = 1.5 + (k * 0.618).fract() * 7.0;
            let height = 0.3 + (k * 0.271).fract() * 3.0 + 0.3 * (t * 0.8 + k).sin();
            glows.push(Glow::new(Vec3::new(angle.cos() * radius, height, angle.sin() * radius), 0.12 * size, Vec3::new(0.7, 1.0, 0.35), intensity).with_pulse(GlowPulse { rate: 0.8, depth: 0.9, phase: (k * 0.13).fract() }));
        }
        self.frame_glows = glows;
        self.bloom_on = settings.on("bloom");
        if let Some(bloom) = self.bloom.as_mut() {
            bloom.settings = BloomSettings { threshold: settings.get("threshold"), soft_knee: settings.get("knee"), intensity: settings.get("bloom-intensity"), radius: settings.get("radius").round() as u32 };
        }
        self.lighting = dim_lighting(frame.environment, 0.08, 0.04, Vec3::new(0.3, -1.0, 0.4));
        Ok(())
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, context: &mut DrawContext<'_>) -> Result<(), GraphicsException> {
        if let Some(renderer) = self.renderer.as_mut() {
            draw_models(renderer, &self.instances, pass, context, &self.lighting)?;
        }
        if let Some(glows) = self.glows.as_mut() {
            glows.begin(pass, context.camera, context.time)?;
            glows.add_all(&self.frame_glows);
            glows.end(pass)?;
        }
        Ok(())
    }

    fn after_scene(&mut self, frame: &mut GraphicsFrame<'_>, scene: &GpuTexture, _settings: &Settings) -> Result<(), GraphicsException> {
        match self.bloom.as_mut() {
            Some(bloom) if self.bloom_on => bloom.apply(frame, scene),
            _ => Ok(()),
        }
    }

    fn stats(&self) -> ModelRenderStats {
        self.renderer.as_ref().map(|r| r.frame_stats()).unwrap_or_default()
    }

    fn readouts(&self, out: &mut Vec<Readout>) {
        let glows = self.glows.as_ref();
        out.push(Readout::new("glow-count", "Glows drawn", glows.map_or(0, |g| g.last_glow_count())));
        out.push(Readout::new("glow-draw-calls", "Glow draw calls", glows.map_or(0, |g| g.last_draw_call_count())));
        out.push(Readout::new("wing-tips", "Glows riding on wing-tip nodes", self.wing_tips));
        out.push(Readout::new("bloom-on", "Bloom", if self.bloom_on { "on" } else { "off" }));
        out.push(Readout::new("bloom-levels", "Bloom chain levels (last apply)", self.bloom.as_ref().map_or(0, |b| b.last_level_count())));
        out.push(Readout::new("draw-calls", "Model draw calls", self.stats().draw_calls));
    }
}
