//! "Level of detail": a 13 x 13 field of GROVES, each one `LodInstance` of three levels drawn with
//! `ModelRenderer::draw_lod`:
//!
//! - level 0, out to 40 m: a SET of models - three trees, a rock and a fox walking on the spot
//!   (animated only while the level is shown: `LodInstance::shown_levels_mut`);
//! - level 1, out to 110 m: ONE simple surface standing in for the whole grove (a rock of the kit
//!   stretched to the grove's size: the "canopy");
//! - level 2, out to 260 m (the CUT-OFF, which a control turns off: `set_max_distance(2,
//!   f32::INFINITY)`): a lower, flatter shape.
//!
//! Inside a FADE BAND (the last `fade_band` part of each level's range) both levels are drawn,
//! the one going out and the one coming in dissolving through a per-pixel 8 x 8 dither with
//! complementary patterns, so nothing pops; at the cut-off the last level dissolves to nothing.
//! The camera FLIES in and out along one line from the marked grove in the middle (10 m to 300 m
//! and back, on the scene's clock), or HOLDS at a distance a slider sets, or is free (orbit).
//! Controls: the band (0 = the hard switch, to see the pop), the quality scale
//! (`set_lod_distance_scale`), the hysteresis, levels tinted (level 1 green, level 2 blue) or as
//! they are, the cut-off, sun shadows (a fading level's shadow fades with it), anti-aliasing Off /
//! 4x (the dither looks the same), a TEST set with the same model on every level (red, green, blue:
//! where the levels have the same shape a mid-fade shows only the two colours, never a hole), and
//! "only the marked grove" (with the ground and sky, or on black for counting its pixels). The read-outs
//! count the groves at each level, fading, culled and beyond the cut-off, beside the renderer's
//! own counts (`frame_stats().culled_lods` / `fading_lods`).

use codebrix_webgpu::glam::{Mat4, Quat, Vec3};
use codebrix_webgpu_graphics::{AnimationPlayer, Camera, GraphicsException, LodInstance, LodLevel, LodState, Model, ModelInstance, ModelRenderStats, ModelRenderer, PlayOptions, RenderPass, SceneLighting, SunShadowSettings};

use crate::demos::kit::Kit;
use crate::demos::{dim_lighting, Control, Demo, DemoFrame, DrawContext, Framing, Readout, Settings};

/// The controls.
pub const CONTROLS: &[Control] = &[
    Control::Choice { key: "camera", label: "Camera", options: &[("fly in and out", 0.0), ("hold at a distance", 1.0), ("free (orbit)", 2.0)], default: 0.0 },
    Control::Slider { key: "distance", label: "Hold distance (m)", min: 5.0, max: 400.0, step: 0.1, default: 38.0 },
    Control::Choice { key: "band", label: "Fade band", options: &[("0 (hard switch)", 0.0), ("0.05", 0.05), ("0.1", 0.1), ("0.2", 0.2), ("0.4", 0.4)], default: 0.1 },
    Control::Choice { key: "quality", label: "Quality (distance scale)", options: &[("0.25", 0.25), ("0.5", 0.5), ("1", 1.0), ("1.5", 1.5), ("2", 2.0)], default: 1.0 },
    Control::Choice { key: "hysteresis", label: "Hysteresis", options: &[("0", 0.0), ("0.02", 0.02), ("0.05", 0.05)], default: 0.02 },
    Control::Toggle { key: "tint", label: "Tint the levels", default: true },
    Control::Toggle { key: "cutoff", label: "Cut-off at 260 m", default: true },
    Control::Toggle { key: "shadows", label: "Sun shadows", default: true },
    Control::Choice { key: "aa", label: "Anti-aliasing", options: &[("Off", 1.0), ("4x", 4.0)], default: 1.0 },
    Control::Choice { key: "objects", label: "Levels", options: &[("groves: models, surface, far shape", 0.0), ("test: one model on every level", 1.0)], default: 0.0 },
    Control::Choice { key: "isolate", label: "Groves drawn", options: &[("all", 0.0), ("only the marked one", 1.0), ("only the marked one, on black", 2.0)], default: 0.0 },
    Control::Toggle { key: "shown", label: "Groves shown", default: true },
    Control::Toggle { key: "culling", label: "Frustum culling", default: true },
];

/// What to look at.
pub const LEGEND: &[&str] = &[
    "As the camera flies out and in, NOTHING POPS: each grove dissolves through a fine dot pattern into its next level, and at the cut-off into nothing.",
    "Tinted: level 0 as authored (trees, rock, a fox), level 1 green (one surface for the grove), level 2 blue; band 0 shows the hard switch.",
    "No holes while dissolving, no doubled pixels; the dots look the same with anti-aliasing Off and 4x.",
    "Sun shadows: a dissolving grove's shadow dissolves with it.",
    "Hysteresis: hold the camera at a boundary and nudge the distance - nothing flickers.",
];

/// The level distances, in metres (before the quality scale).
const LEVEL_DISTANCES: [f32; 3] = [40.0, 110.0, 260.0];
/// The groves per side of the field, and their spacing.
const GRID: i32 = 13;
const SPACING: f32 = 24.0;
/// The fly path's nearest and farthest distance, and its period in seconds (out and back).
const FLY_NEAR: f32 = 10.0;
const FLY_FAR: f32 = 300.0;
const FLY_PERIOD: f64 = 48.0;
/// The camera's heading and elevation from the marked grove.
const HEADING: f32 = 0.55;
const ELEVATION: f32 = 0.16;

/// The tints of the levels: (groves tinted, groves as they are, the test set).
const TINTS: [[Vec3; 3]; 3] = [
    [Vec3::ONE, Vec3::new(0.35, 1.7, 0.35), Vec3::new(0.35, 0.5, 1.9)],
    [Vec3::ONE, Vec3::new(0.55, 0.95, 0.45), Vec3::new(0.5, 0.85, 0.45)],
    [Vec3::new(3.0, 0.06, 0.06), Vec3::new(0.06, 2.2, 0.06), Vec3::new(0.08, 0.08, 3.0)],
];

/// The level-of-detail demo.
#[derive(Default)]
pub struct LodDemo {
    renderer: Option<ModelRenderer>,
    ground: Option<ModelInstance>,
    groves: Vec<LodInstance>,
    tests: Vec<LodInstance>,
    marked: usize,
    lighting: SceneLighting,
    samples: u32,
    test: bool,
    /// Only the marked grove is drawn.
    isolate: bool,
    /// ... and on black: no ground, no sky, no shadows (for counting its pixels).
    on_black: bool,
    shown: bool,
    /// The tint, cut-off and set the levels were last set up for.
    applied: Option<(bool, bool, bool)>,
    camera_distance: f32,
    fly_distance: f32,
}

/// The distance along the fly path at `time`: out from FLY_NEAR to FLY_FAR and back, evenly on a
/// logarithmic scale (so a band takes about as long to cross near as far).
fn fly_distance(time: f64) -> f32 {
    let phase = (time / FLY_PERIOD).rem_euclid(1.0) as f32;
    let s = if phase < 0.5 { phase * 2.0 } else { 2.0 - phase * 2.0 };
    FLY_NEAR * (FLY_FAR / FLY_NEAR).powf(s)
}

/// The world matrix that stretches `model` to `size` (x, y, z) with its base on the ground at `at`.
fn stretched(model: &Model, size: Vec3, at: Vec3) -> Mat4 {
    let (min, max) = (model.bounds_min(), model.bounds_max());
    let extent = (max - min).max(Vec3::splat(1e-3));
    let scale = size / extent;
    let center = (min + max) * 0.5;
    Mat4::from_translation(at) * Mat4::from_scale(scale) * Mat4::from_translation(Vec3::new(-center.x, -min.y, -center.z))
}

fn local(scale: f32, yaw: f32, at: Vec3) -> Mat4 {
    Mat4::from_scale_rotation_translation(Vec3::splat(scale), Quat::from_rotation_y(yaw), at)
}

/// A short description of a state.
fn describe(state: LodState) -> String {
    match state {
        LodState::NotDrawn => "not drawn yet".to_string(),
        LodState::Culled => "culled".to_string(),
        LodState::Level(level) => format!("level {level}"),
        LodState::Fading { from, to: Some(to), progress } => format!("fading {from} -> {to} at {progress:.3}"),
        LodState::Fading { from, to: None, progress } => format!("fading {from} -> nothing at {progress:.3}"),
        LodState::BeyondCutOff => "beyond the cut-off".to_string(),
    }
}

impl Demo for LodDemo {
    fn build(&mut self, kit: &mut Kit<'_>) -> Result<Framing, String> {
        let ground = kit.ground()?;
        // The ground model reaches 90 m either way; the field reaches about 150 m.
        self.ground = Some(ground.create_instance_at(Mat4::from_scale(Vec3::new(2.0, 1.0, 2.0))));
        let trees = [kit.model("tree")?, kit.model("tree-tall")?, kit.model("tree-autumn")?];
        let rock = kit.model("rock-a")?;
        let canopy = kit.model("rock-b")?;
        let far = kit.model("rock-c")?;
        let fox = kit.model("fox")?;
        let half = (GRID / 2) as f32;
        for row in 0..GRID {
            for column in 0..GRID {
                let k = (row * 7 + column * 3) as usize;
                let at = Vec3::new((column as f32 - half) * SPACING, 0.0, (row as f32 - half) * SPACING) + Vec3::new(((k * 13) % 7) as f32 * 0.6 - 1.8, 0.0, ((k * 5) % 7) as f32 * 0.6 - 1.8);
                let place = Mat4::from_rotation_translation(Quat::from_rotation_y(k as f32 * 0.9), at);
                if row == GRID / 2 && column == GRID / 2 {
                    self.marked = self.groves.len();
                }
                // Level 0: a SET of models.
                let mut set = vec![
                    trees[0].create_instance_at(local(2.6, 0.3, Vec3::new(-1.8, 0.0, -1.0))),
                    trees[1].create_instance_at(local(2.6, 1.1, Vec3::new(1.9, 0.0, 0.4))),
                    trees[2].create_instance_at(local(2.8, 2.0, Vec3::new(-0.2, 0.0, 2.1))),
                    rock.create_instance_at(local(1.6, 0.7, Vec3::new(2.2, 0.0, -2.2))),
                ];
                let mut walker = fox.create_instance_at(local(0.55, k as f32, Vec3::new(0.4, 0.0, -0.6)));
                walker.animation().play_named("walk", PlayOptions { start_time: (k % 11) as f32 * 0.13, ..PlayOptions::default() }).map_err(|e| e.to_string())?;
                set.push(walker);
                let levels = vec![
                    LodLevel::new(set, LEVEL_DISTANCES[0]),
                    LodLevel::single(canopy.create_instance_at(stretched(&canopy, Vec3::new(7.5, 6.5, 7.5), Vec3::ZERO)), LEVEL_DISTANCES[1]),
                    LodLevel::single(far.create_instance_at(stretched(&far, Vec3::new(7.0, 3.5, 7.0), Vec3::ZERO)), LEVEL_DISTANCES[2]),
                ];
                self.groves.push(LodInstance::new(place, levels).map_err(|e| e.to_string())?);
                // The test set: the same model on every level (only the tint differs).
                let same = |distance: f32| LodLevel::single(trees[1].create_instance_at(local(2.6, 0.0, Vec3::ZERO)), distance);
                self.tests.push(LodInstance::new(place, vec![same(LEVEL_DISTANCES[0]), same(LEVEL_DISTANCES[1]), same(LEVEL_DISTANCES[2])]).map_err(|e| e.to_string())?);
            }
        }
        self.renderer = Some(kit.graphics.create_model_renderer().map_err(|e| e.to_string())?);
        self.samples = 1;
        let reach = half * SPACING + 6.0;
        Ok(Framing { min: Vec3::new(-reach, 0.0, -reach), max: Vec3::new(reach, 8.0, reach), yaw: HEADING, pitch: 0.35, distance_scale: 0.6, auto_orbit: true })
    }

    fn place_camera(&mut self, time: f64, settings: &Settings, camera: &mut Camera) {
        let mode = settings.get("camera");
        self.fly_distance = fly_distance(time);
        if mode >= 1.5 {
            return;
        }
        let distance = if mode >= 0.5 { settings.get("distance") } else { self.fly_distance };
        let position = self.groves.get(self.marked).map_or(Vec3::ZERO, |g| g.position());
        let direction = Vec3::new(ELEVATION.cos() * HEADING.sin(), ELEVATION.sin(), ELEVATION.cos() * HEADING.cos());
        camera.position = position + direction * distance;
        camera.target = position + Vec3::new(0.0, 3.0, 0.0);
        camera.up = Vec3::Y;
    }

    fn update(&mut self, frame: &DemoFrame<'_>, settings: &Settings) -> Result<(), String> {
        self.samples = if settings.get("aa") >= 2.0 { 4 } else { 1 };
        self.test = settings.get("objects") >= 0.5;
        self.isolate = settings.get("isolate") >= 0.5;
        self.on_black = settings.get("isolate") >= 1.5;
        self.shown = settings.on("shown");
        let (tint, cutoff) = (settings.on("tint"), settings.on("cutoff"));
        let band = settings.get("band");
        let hysteresis = settings.get("hysteresis");
        if self.applied != Some((tint, cutoff, self.test)) {
            let tints = if self.test { TINTS[2] } else if tint { TINTS[0] } else { TINTS[1] };
            let cut = if cutoff { LEVEL_DISTANCES[2] } else { f32::INFINITY };
            for lod in self.groves.iter_mut().chain(self.tests.iter_mut()) {
                for (level, tint) in tints.iter().enumerate() {
                    if let Some(level) = lod.level_mut(level) {
                        for instance in level.instances.iter_mut() {
                            instance.tint = *tint;
                        }
                    }
                }
                lod.set_max_distance(2, cut).map_err(|e| e.to_string())?;
            }
            self.applied = Some((tint, cutoff, self.test));
        }
        for lod in self.groves.iter_mut().chain(self.tests.iter_mut()) {
            lod.fade_band = band;
            lod.hysteresis = hysteresis;
        }
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.set_lod_distance_scale(settings.get("quality"));
            renderer.set_frustum_culling(settings.on("culling"));
        }
        let marked = self.groves.get(self.marked).map_or(Vec3::ZERO, |g| g.position());
        self.camera_distance = (frame.camera.position - marked).length();
        // The sun from the side of the camera's line, so the shadows fall sideways in view (not
        // hidden behind their groves).
        let mut lighting = dim_lighting(frame.environment, 0.9, 2.6, Vec3::new(0.75, -1.0, -0.45));
        // The sun's shadow covers the marked grove's neighbourhood (where the camera looks).
        let reach = Vec3::new(45.0, 0.0, 45.0);
        lighting.sun_shadows = SunShadowSettings { enabled: settings.on("shadows") && !self.on_black, ..SunShadowSettings::covering(marked - reach, marked + reach + Vec3::new(0.0, 12.0, 0.0)) };
        self.lighting = lighting;
        Ok(())
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, context: &mut DrawContext<'_>) -> Result<(), GraphicsException> {
        let Some(renderer) = self.renderer.as_mut() else { return Ok(()) };
        renderer.begin(pass, context.camera, Some(&self.lighting))?;
        if !self.on_black {
            if let Some(ground) = self.ground.as_ref() {
                renderer.draw(ground)?;
            }
        }
        let marked = self.marked;
        let lods = if self.test { &mut self.tests } else { &mut self.groves };
        if self.shown {
            for (index, lod) in lods.iter_mut().enumerate() {
                if self.isolate && index != marked {
                    continue;
                }
                // Choose first, animate only what is shown, then draw (draw_lod chooses the same).
                if renderer.select_lod(lod)? != LodState::Culled {
                    for level in lod.shown_levels_mut() {
                        AnimationPlayer::update_all_to(&mut level.instances, context.time);
                    }
                }
                renderer.draw_lod(lod)?;
            }
        }
        renderer.end_opaque(pass)?;
        if !self.on_black {
            context.skybox.draw(pass, context.camera, Some(&self.lighting))?;
        }
        renderer.end_blended(pass)
    }

    fn sample_count(&self, settings: &Settings) -> u32 {
        if settings.get("aa") >= 2.0 { 4 } else { 1 }
    }

    fn stats(&self) -> ModelRenderStats {
        self.renderer.as_ref().map(|r| r.frame_stats()).unwrap_or_default()
    }

    fn readouts(&self, out: &mut Vec<Readout>) {
        let lods = if self.test { &self.tests } else { &self.groves };
        let mut at = [0usize; 3];
        let (mut fading, mut culled, mut beyond, mut undrawn) = (0, 0, 0, 0);
        for lod in lods {
            match lod.state() {
                LodState::Level(level) => at[level.min(2)] += 1,
                LodState::Fading { .. } => fading += 1,
                LodState::Culled => culled += 1,
                LodState::BeyondCutOff => beyond += 1,
                LodState::NotDrawn => undrawn += 1,
            }
        }
        let stats = self.stats();
        out.push(Readout::new("groves", "Groves", lods.len()));
        out.push(Readout::new("level-0", "At level 0 (models)", at[0]));
        out.push(Readout::new("level-1", "At level 1 (one surface)", at[1]));
        out.push(Readout::new("level-2", "At level 2 (far shape)", at[2]));
        out.push(Readout::new("fading", "Fading (inside a band)", fading));
        out.push(Readout::new("culled", "Culled (outside the view and the shadows)", culled));
        out.push(Readout::new("beyond", "Beyond the cut-off", beyond));
        out.push(Readout::new("not-drawn", "Never drawn yet", undrawn));
        out.push(Readout::new("renderer-fading", "The renderer's fading_lods", stats.fading_lods));
        out.push(Readout::new("renderer-culled", "The renderer's culled_lods", stats.culled_lods));
        let marked = lods.get(self.marked).map(|lod| lod.state()).unwrap_or(LodState::NotDrawn);
        out.push(Readout::new("marked-distance", "The marked grove's distance (m)", format!("{:.2}", self.camera_distance)));
        out.push(Readout::new("marked-state", "The marked grove", describe(marked)));
        out.push(Readout::new("marked-coverage", "Its coverage of levels 0 / 1 / 2", format!("{:.3} / {:.3} / {:.3}", marked.coverage(0), marked.coverage(1), marked.coverage(2))));
        out.push(Readout::new("fly-distance", "Fly path distance now (m)", format!("{:.2}", self.fly_distance)));
        out.push(Readout::new("draw-calls", "Draw calls", stats.draw_calls));
        out.push(Readout::new("instances", "Primitive instances drawn", stats.instances));
        out.push(Readout::new("shadow-passes", "Shadow passes", stats.shadow_passes));
        out.push(Readout::new("pipelines", "Pipelines", self.renderer.as_ref().map_or(0, |r| r.pipeline_count())));
    }
}
