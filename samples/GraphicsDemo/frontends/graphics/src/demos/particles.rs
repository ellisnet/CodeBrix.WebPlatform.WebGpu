//! "Particles": three GPU particle systems (`ParticleSystem`: a compute step per frame, drawn as
//! camera-facing sprites with the glow layer's drawing) on the HDR path with bloom:
//!
//! - SPARKS, additive, from a cone emitter that circles the camp fire (`emitter.place` every
//!   frame): spawns are spread along the emitter's path, so it leaves a smooth trail; gravity and
//!   drag bend them down; intensity 4, so they bloom;
//! - a MIST, transparent (ordinary alpha blending, unsorted), from a flat box over the ground: slow,
//!   growing, fading to nothing;
//! - BURSTS: a system with no rate that bursts 300 sparks where you CLICK on the ground (the
//!   canvas point's ray met with the ground plane), or over the fire with the button.
//!
//! They follow the canvas clock: each step takes the frame's delta (0 while paused: nothing
//! moves, nothing spawns; half or quarter speed slows them). The read-outs show what each system
//! spawned, how much time it has simulated and how many are alive at most
//! (`ParticleSystem::alive_at_most`, worked out on the CPU from the spawn history - the system
//! does not read its live count back from the GPU).

use codebrix_webgpu::glam::{Mat4, Vec3};
use codebrix_webgpu::{CanvasInput, PointerButton};
use codebrix_webgpu_graphics::{BloomRenderer, BloomSettings, Camera, EmitterShape, GlowBlend, GpuTexture, GraphicsException, GraphicsFrame, ModelInstance, ModelRenderStats, ModelRenderer, ParticleSettings, ParticleSystem, ParticleSystemOptions, RenderPass, SceneLighting};

use crate::demos::kit::Kit;
use crate::demos::{dim_lighting, draw_models, ground_point, placed, Control, Demo, DemoFrame, DrawContext, Framing, Readout, Settings};

/// The controls.
pub const CONTROLS: &[Control] = &[
    Control::Slider { key: "rate", label: "Spark rate (per s)", min: 0.0, max: 1500.0, step: 10.0, default: 400.0 },
    Control::Slider { key: "gravity", label: "Gravity", min: 0.0, max: 20.0, step: 0.1, default: 9.81 },
    Control::Slider { key: "drag", label: "Drag", min: 0.0, max: 3.0, step: 0.05, default: 0.4 },
    Control::Slider { key: "angle", label: "Cone angle (rad)", min: 0.0, max: 1.2, step: 0.05, default: 0.35 },
    Control::Toggle { key: "mist", label: "Mist", default: true },
    Control::Toggle { key: "bloom", label: "Bloom", default: true },
    Control::Button { key: "burst", label: "Burst over the fire" },
    Control::Button { key: "reset", label: "Reset every system" },
];

/// What to look at.
pub const LEGEND: &[&str] = &[
    "A steady fountain arcs down under gravity and fades out; the circling emitter leaves a smooth trail.",
    "Click the ground: a burst of sparks there. Pause the clock: everything freezes exactly.",
    "Quarter speed: slow motion. Particles hide behind the models (depth-tested).",
    "The mist overlaps in buffer order (transparent particles are not sorted): expected.",
];

/// How many sparks a burst spawns.
const BURST: u32 = 300;

/// One system and its read-outs.
struct Tracked {
    system: ParticleSystem,
}

impl Tracked {
    fn new(system: ParticleSystem) -> Tracked {
        Tracked { system }
    }

    fn readout(&self, out: &mut Vec<Readout>, keys: [&'static str; 3], name: &'static str) {
        let _ = name;
        out.push(Readout::new(keys[0], "spawned", self.system.spawned_count()));
        out.push(Readout::new(keys[1], "simulated seconds", format!("{:.3}", self.system.simulated_time())));
        out.push(Readout::new(keys[2], "alive (at most)", self.system.alive_at_most()));
    }
}

/// The particles demo.
#[derive(Default)]
pub struct ParticlesDemo {
    renderer: Option<ModelRenderer>,
    bloom: Option<BloomRenderer>,
    sparks: Option<Tracked>,
    mist: Option<Tracked>,
    bursts: Option<Tracked>,
    instances: Vec<ModelInstance>,
    lighting: SceneLighting,
    mist_on: bool,
    bloom_on: bool,
    burst_presses: f32,
    reset_presses: f32,
    reset_now: bool,
    pending: Option<Vec3>,
    burst_next: bool,
    pressed_at: Option<(f64, f64)>,
    clicks: u32,
    last_delta: f64,
}

impl Demo for ParticlesDemo {
    fn build(&mut self, kit: &mut Kit<'_>) -> Result<Framing, String> {
        let ground = kit.ground()?;
        let mut instances = vec![ground.create_instance()];
        for (name, scale, yaw, at) in [("campfire", 2.4, 0.0, Vec3::ZERO), ("tree", 2.6, 0.3, Vec3::new(-4.0, 0.0, -3.5)), ("tree-tall", 2.6, 1.1, Vec3::new(3.5, 0.0, -4.0)), ("rock-a", 2.2, 0.4, Vec3::new(2.6, 0.0, 2.2)), ("barrel", 2.2, 0.0, Vec3::new(-2.4, 0.0, 1.8)), ("chest", 3.2, 0.8, Vec3::new(-0.8, 0.0, -2.2))] {
            instances.push(placed(&kit.model(name)?, scale, yaw, at));
        }
        self.instances = instances;
        let graphics = kit.graphics;
        let mut sparks = graphics.create_particle_system(&ParticleSystemOptions { capacity: 4096, seed: 7, ..ParticleSystemOptions::default() }).map_err(|e| e.to_string())?;
        sparks.emitter.shape = EmitterShape::Cone { radius: 0.05, angle: 0.35 };
        sparks.emitter.speed_min = 2.0;
        sparks.emitter.speed_max = 4.0;
        sparks.settings = ParticleSettings { acceleration: Vec3::new(0.0, -9.81, 0.0), drag: 0.4, intensity: 4.0, size_at_birth: 0.08, size_at_death: 0.03, ..ParticleSettings::default() };
        let mut mist = graphics.create_particle_system(&ParticleSystemOptions { capacity: 2048, seed: 3, blend: GlowBlend::Transparent, ..ParticleSystemOptions::default() }).map_err(|e| e.to_string())?;
        mist.emitter.shape = EmitterShape::Box { half_extents: Vec3::new(4.0, 0.15, 4.0) };
        mist.emitter.position = Vec3::new(0.0, 0.3, 0.0);
        mist.emitter.rate = 60.0;
        mist.emitter.spread = 1.2;
        mist.emitter.speed_min = 0.05;
        mist.emitter.speed_max = 0.2;
        mist.emitter.lifetime_min = 4.0;
        mist.emitter.lifetime_max = 6.0;
        mist.settings = ParticleSettings { size_at_birth: 0.6, size_at_death: 2.2, color_at_birth: Vec3::new(0.55, 0.6, 0.7), color_at_death: Vec3::new(0.45, 0.5, 0.6), opacity_at_birth: 0.3, opacity_at_death: 0.0, fade_out: 0.5, ..ParticleSettings::default() };
        let mut bursts = graphics.create_particle_system(&ParticleSystemOptions { capacity: 4096, seed: 11, ..ParticleSystemOptions::default() }).map_err(|e| e.to_string())?;
        bursts.emitter.shape = EmitterShape::Sphere { radius: 0.1 };
        bursts.emitter.rate = 0.0;
        bursts.emitter.spread = std::f32::consts::PI;
        bursts.emitter.speed_min = 2.0;
        bursts.emitter.speed_max = 5.0;
        bursts.emitter.lifetime_min = 0.8;
        bursts.emitter.lifetime_max = 1.6;
        bursts.emitter.position = Vec3::new(0.0, 0.6, 0.0);
        bursts.settings = ParticleSettings { acceleration: Vec3::new(0.0, -4.0, 0.0), drag: 0.8, intensity: 5.0, size_at_birth: 0.1, size_at_death: 0.03, color_at_birth: Vec3::new(1.0, 0.85, 0.4), color_at_death: Vec3::new(1.0, 0.2, 0.05), ..ParticleSettings::default() };
        self.sparks = Some(Tracked::new(sparks));
        self.mist = Some(Tracked::new(mist));
        self.bursts = Some(Tracked::new(bursts));
        self.renderer = Some(graphics.create_model_renderer().map_err(|e| e.to_string())?);
        self.bloom = Some(graphics.create_bloom_renderer().map_err(|e| e.to_string())?);
        Ok(Framing { min: Vec3::new(-5.0, 0.0, -5.0), max: Vec3::new(5.0, 3.0, 5.0), yaw: 0.5, pitch: 0.3, distance_scale: 0.85, auto_orbit: false })
    }

    fn update(&mut self, frame: &DemoFrame<'_>, settings: &Settings) -> Result<(), String> {
        let t = frame.time;
        if let Some(sparks) = self.sparks.as_mut() {
            let system = &mut sparks.system;
            system.emitter.place(Mat4::from_translation(Vec3::new((t.cos() * 2.5) as f32, 0.6, (t.sin() * 2.5) as f32)));
            system.emitter.rate = settings.get("rate");
            system.emitter.shape = EmitterShape::Cone { radius: 0.05, angle: settings.get("angle") };
            system.settings.acceleration = Vec3::new(0.0, -settings.get("gravity"), 0.0);
            system.settings.drag = settings.get("drag");
        }
        if settings.get("burst") != self.burst_presses {
            self.burst_presses = settings.get("burst");
            self.pending = Some(Vec3::new(0.0, 0.6, 0.0));
        }
        if settings.get("reset") != self.reset_presses {
            self.reset_presses = settings.get("reset");
            self.reset_now = true;
        }
        // A burst goes where the emitter IS: move it first, burst at the next step (spawns are
        // spread along the emitter's path since its last step, so moving and bursting in one step
        // would draw a line of sparks from the old place to the new one).
        if let (Some(at), Some(bursts)) = (self.pending.take(), self.bursts.as_mut()) {
            bursts.system.emitter.position = at;
            self.burst_next = true;
        }
        self.mist_on = settings.on("mist");
        self.bloom_on = settings.on("bloom");
        self.last_delta = frame.delta;
        self.lighting = dim_lighting(frame.environment, 0.12, 0.08, Vec3::new(0.3, -1.0, 0.4));
        Ok(())
    }

    fn before_scene(&mut self, frame: &mut GraphicsFrame<'_>, delta: f64, _settings: &Settings) -> Result<(), GraphicsException> {
        for tracked in [self.sparks.as_mut(), self.mist.as_mut(), self.bursts.as_mut()].into_iter().flatten() {
            if self.reset_now {
                tracked.system.reset(frame)?;
            }
        }
        self.reset_now = false;
        if let Some(sparks) = self.sparks.as_mut() {
            sparks.system.update(frame, delta)?;
        }
        if let Some(mist) = self.mist.as_mut() {
            if self.mist_on {
                mist.system.update(frame, delta)?;
            }
        }
        if let Some(bursts) = self.bursts.as_mut() {
            bursts.system.update(frame, delta)?;
            if self.burst_next && delta > 0.0 {
                bursts.system.burst(BURST);
                self.burst_next = false;
            }
        }
        Ok(())
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, context: &mut DrawContext<'_>) -> Result<(), GraphicsException> {
        if let Some(renderer) = self.renderer.as_mut() {
            draw_models(renderer, &self.instances, pass, context, &self.lighting)?;
        }
        if self.mist_on {
            if let Some(mist) = self.mist.as_mut() {
                mist.system.draw(pass, context.camera)?;
            }
        }
        for tracked in [self.sparks.as_mut(), self.bursts.as_mut()].into_iter().flatten() {
            tracked.system.draw(pass, context.camera)?;
        }
        Ok(())
    }

    fn after_scene(&mut self, frame: &mut GraphicsFrame<'_>, scene: &GpuTexture, _settings: &Settings) -> Result<(), GraphicsException> {
        match self.bloom.as_mut() {
            Some(bloom) if self.bloom_on => {
                bloom.settings = BloomSettings { threshold: 1.2, intensity: 1.0, ..BloomSettings::default() };
                bloom.apply(frame, scene)
            }
            _ => Ok(()),
        }
    }

    fn stats(&self) -> ModelRenderStats {
        self.renderer.as_ref().map(|r| r.frame_stats()).unwrap_or_default()
    }

    fn readouts(&self, out: &mut Vec<Readout>) {
        if let Some(sparks) = self.sparks.as_ref() {
            sparks.readout(out, ["sparks-spawned", "sparks-time", "sparks-alive"], "sparks");
        }
        if let Some(mist) = self.mist.as_ref() {
            mist.readout(out, ["mist-spawned", "mist-time", "mist-alive"], "mist");
        }
        if let Some(bursts) = self.bursts.as_ref() {
            bursts.readout(out, ["bursts-spawned", "bursts-time", "bursts-alive"], "bursts");
            let p = bursts.system.emitter.position;
            out.push(Readout::new("burst-at", "Last burst place", format!("({:.2}, {:.2}, {:.2})", p.x, p.y, p.z)));
        }
        out.push(Readout::new("clicks", "Clicks on the ground", self.clicks));
        out.push(Readout::new("delta", "Last frame's clock delta (s)", format!("{:.4}", self.last_delta)));
    }

    fn input(&mut self, input: &CanvasInput, camera: &Camera, css: (f32, f32)) -> bool {
        match input {
            CanvasInput::PointerDown { x, y, button: PointerButton::Primary, modifiers, .. } if !modifiers.shift => {
                self.pressed_at = Some((*x, *y));
                false
            }
            CanvasInput::PointerUp { x, y, .. } => {
                // A click (not a drag): burst where the pointer meets the ground.
                if let Some((x0, y0)) = self.pressed_at.take() {
                    if (x - x0).abs() < 4.0 && (y - y0).abs() < 4.0 {
                        if let Some(point) = ground_point(camera, *x as f32, *y as f32, css) {
                            self.pending = Some(point + Vec3::new(0.0, 0.2, 0.0));
                            self.clicks += 1;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }
}
