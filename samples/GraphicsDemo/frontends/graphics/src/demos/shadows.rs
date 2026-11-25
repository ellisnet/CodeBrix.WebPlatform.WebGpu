//! "Shadows": the sun's shadow (`SceneLighting::sun_shadows`) and the spotlights' shadows
//! (`SpotLight::casts_shadows`, `SceneLighting::spot_shadows`) over animated casters - the chest
//! opening and closing (node animation), three foxes walking (node animation) and the archer
//! walking (a skinned model, which casts in its current pose). The covered region is one box (the
//! one the camera is framed on, OrbitController::frame_box), outlined with small glows when asked.
//! A very tall tree stands BEHIND the camera at the start: it is culled for the camera, but still
//! casts its long shadow into view (each shadow pass culls against its own light's volume). Up to
//! six shadow-casting spotlights circle above the archer: the renderer gives shadows to at most
//! four, the others still light without shadows.
//!
//! The renderer's bias constants are private (not settings): a normal offset of 1.5 shadow-map
//! texels, a depth bias of 0.0002 for the sun and 0.00005 for spotlights, a slope-scaled bias of
//! 2.0 in the depth pipelines. The legend names them so a person can judge acne, peter-panning and
//! light leaks by eye.

use codebrix_webgpu::glam::Vec3;
use codebrix_webgpu_graphics::{Color4, CullVolume, Glow, GlowLayer, GraphicsException, ModelInstance, ModelRenderStats, ModelRenderer, PlayOptions, RenderPass, SceneLighting, ShadowMapSize, SpotLight, SpotShadowSettings, SunShadowSettings};

use crate::demos::kit::Kit;
use crate::demos::{aspect, dim_lighting, draw_models, on_circle, placed, world, Control, Demo, DemoFrame, DrawContext, Framing, Readout, Settings};

/// The controls.
pub const CONTROLS: &[Control] = &[
    Control::Toggle { key: "sun-shadows", label: "Sun shadows", default: false },
    Control::Choice { key: "map-size", label: "Sun map size", options: &[("1024", 1024.0), ("2048", 2048.0), ("4096", 4096.0)], default: 2048.0 },
    Control::Choice { key: "softness", label: "Softness (samples)", options: &[("1", 1.0), ("4", 4.0), ("9", 9.0), ("16", 16.0), ("25", 25.0)], default: 9.0 },
    Control::Slider { key: "strength", label: "Strength", min: 0.0, max: 1.0, step: 0.05, default: 1.0 },
    Control::Slider { key: "sun-azimuth", label: "Sun azimuth", min: 0.0, max: 360.0, step: 1.0, default: 34.0 },
    Control::Slider { key: "sun-elevation", label: "Sun elevation", min: 8.0, max: 85.0, step: 1.0, default: 25.0 },
    Control::Choice { key: "region", label: "Covered region", options: &[("the whole scene", 0.0), ("the middle only", 1.0)], default: 0.0 },
    Control::Toggle { key: "show-region", label: "Show the region", default: false },
    Control::Toggle { key: "tall-tree-casts", label: "Tall tree casts", default: true },
    Control::Toggle { key: "foxes-cast", label: "Foxes cast", default: true },
    Control::Toggle { key: "archer-receives", label: "Archer receives", default: true },
    Control::Toggle { key: "culling", label: "Frustum culling", default: true },
    Control::Slider { key: "spots", label: "Shadow-casting spotlights", min: 0.0, max: 6.0, step: 1.0, default: 0.0 },
    Control::Choice { key: "spot-map-size", label: "Spot map size", options: &[("1024", 1024.0), ("2048", 2048.0)], default: 1024.0 },
];

/// What to look at.
pub const LEGEND: &[&str] = &[
    "Shadows touch the feet of things (little peter-panning), no stripes on sunlit faces (no acne).",
    "The archer's shadow moves with its legs; the chest's lid opens in its shadow.",
    "The long shadow comes from a tall tree BEHIND the camera (culled for the camera, still casting).",
    "With 5 or 6 spotlights only 4 cast shadows; the others light through objects.",
    "Bias (fixed, private): normal offset 1.5 texels, depth bias 0.0002 sun / 0.00005 spot, slope 2.0.",
];

/// The region boxes: the whole scene and the middle only.
const SCENE_MIN: Vec3 = Vec3::new(-10.0, 0.0, -10.0);
const SCENE_MAX: Vec3 = Vec3::new(10.0, 6.0, 10.0);
const MIDDLE_MIN: Vec3 = Vec3::new(-4.0, 0.0, -4.0);
const MIDDLE_MAX: Vec3 = Vec3::new(4.0, 4.0, 4.0);
/// The camera's start yaw; the tall tree stands behind the camera along it.
const START_YAW: f32 = 0.6;
/// The spotlights' colours.
const SPOT_COLORS: [(f32, f32, f32); 6] = [(1.0, 0.35, 0.3), (0.35, 1.0, 0.4), (0.35, 0.5, 1.0), (1.0, 0.85, 0.3), (1.0, 0.4, 1.0), (0.3, 1.0, 1.0)];

fn map_size(value: f32) -> ShadowMapSize {
    if value >= 4096.0 {
        ShadowMapSize::Size4096
    } else if value >= 2048.0 {
        ShadowMapSize::Size2048
    } else {
        ShadowMapSize::Size1024
    }
}

/// The shadows demo.
#[derive(Default)]
pub struct ShadowsDemo {
    renderer: Option<ModelRenderer>,
    glows: Option<GlowLayer>,
    instances: Vec<ModelInstance>,
    archer: usize,
    chest: usize,
    foxes: Vec<usize>,
    tall_tree: usize,
    lighting: SceneLighting,
    region: (Vec3, Vec3),
    show_region: bool,
    tree_in_view: bool,
    spots: usize,
    map_bytes: u64,
}

impl Demo for ShadowsDemo {
    fn build(&mut self, kit: &mut Kit<'_>) -> Result<Framing, String> {
        let ground = kit.ground()?;
        let mut instances = vec![ground.create_instance()];
        // The ground receives, but casts nothing (it is under everything).
        instances[0].casts_shadows = false;
        for (name, scale, yaw, at) in [
            ("tree-tall", 2.6, 0.2, Vec3::new(-6.0, 0.0, -5.0)),
            ("tree", 2.6, 1.0, Vec3::new(5.5, 0.0, -6.0)),
            ("tree-autumn", 2.8, 2.0, Vec3::new(-7.0, 0.0, 4.5)),
            ("tree", 2.4, 0.5, Vec3::new(7.0, 0.0, 5.0)),
            ("rock-a", 2.2, 0.3, Vec3::new(-2.5, 0.0, -6.5)),
            ("rock-b", 2.0, 1.3, Vec3::new(3.0, 0.0, 6.5)),
            ("barrel", 2.2, 0.0, Vec3::new(1.6, 0.0, -1.2)),
            ("barrel", 2.2, 0.7, Vec3::new(2.1, 0.0, -0.7)),
            ("fence", 2.4, 0.0, Vec3::new(-1.0, 0.0, -8.0)),
            ("fence", 2.4, 0.0, Vec3::new(1.4, 0.0, -8.0)),
        ] {
            instances.push(placed(&kit.model(name)?, scale, yaw, at));
        }
        let mut chest = placed(&kit.model("chest")?, 3.2, 0.6, Vec3::new(-1.5, 0.0, 0.5));
        chest.animation().play_named("open-close", PlayOptions::default()).map_err(|e| e.to_string())?;
        self.chest = instances.len();
        instances.push(chest);
        let fox = kit.model("fox")?;
        for i in 0..3 {
            let mut walker = fox.create_instance();
            walker.animation().play_named("walk", PlayOptions { start_time: i as f32 * 0.3, ..PlayOptions::default() }).map_err(|e| e.to_string())?;
            self.foxes.push(instances.len());
            instances.push(walker);
        }
        let mut archer = kit.model("archer")?.create_instance();
        archer.animation().play_named("walk", PlayOptions::default()).map_err(|e| e.to_string())?;
        self.archer = instances.len();
        instances.push(archer);
        // The tall tree behind the camera's start place, along the start yaw.
        let behind = 34.0 * Vec3::new(START_YAW.sin(), 0.0, START_YAW.cos());
        self.tall_tree = instances.len();
        instances.push(placed(&kit.model("tree-tall")?, 9.0, 0.0, behind));
        self.instances = instances;
        self.renderer = Some(kit.graphics.create_model_renderer().map_err(|e| e.to_string())?);
        self.glows = Some(kit.graphics.create_glow_layer().map_err(|e| e.to_string())?);
        Ok(Framing { min: SCENE_MIN, max: SCENE_MAX, yaw: START_YAW, pitch: 0.45, distance_scale: 0.7, auto_orbit: false })
    }

    fn update(&mut self, frame: &DemoFrame<'_>, settings: &Settings) -> Result<(), String> {
        let t = frame.time as f32;
        let (at, yaw) = on_circle(Vec3::new(1.0, 0.0, 2.0), 2.5, 0.4 * t);
        let archer = &mut self.instances[self.archer];
        archer.world = world(2.0, yaw, at);
        archer.receives_shadows = settings.on("archer-receives");
        archer.animation().update_to(frame.time);
        for (k, &index) in self.foxes.iter().enumerate() {
            let (at, yaw) = on_circle(Vec3::ZERO, 6.5, 0.22 * t + k as f32 * 2.1);
            let fox = &mut self.instances[index];
            fox.world = world(0.55, yaw, at);
            fox.casts_shadows = settings.on("foxes-cast");
            fox.animation().update_to(frame.time);
        }
        self.instances[self.chest].animation().update_to(frame.time);
        self.instances[self.tall_tree].casts_shadows = settings.on("tall-tree-casts");
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.set_frustum_culling(settings.on("culling"));
        }
        // The sun from the azimuth and elevation: light_direction is the way the light travels.
        let (azimuth, elevation) = (settings.get("sun-azimuth").to_radians(), settings.get("sun-elevation").to_radians());
        let towards_sun = Vec3::new(azimuth.sin() * elevation.cos(), elevation.sin(), azimuth.cos() * elevation.cos());
        let mut lighting = dim_lighting(frame.environment, 0.45, 2.2, -towards_sun);
        self.region = if settings.get("region") >= 0.5 { (MIDDLE_MIN, MIDDLE_MAX) } else { (SCENE_MIN, SCENE_MAX) };
        let size = map_size(settings.get("map-size"));
        let softness = settings.get("softness") as u32;
        let strength = settings.get("strength");
        lighting.sun_shadows = SunShadowSettings { enabled: settings.on("sun-shadows"), map_size: size, softness, strength, ..SunShadowSettings::covering(self.region.0, self.region.1) };
        self.spots = settings.get("spots").round() as usize;
        let spot_size = map_size(settings.get("spot-map-size"));
        lighting.spot_shadows = SpotShadowSettings { map_size: spot_size, softness, strength };
        for i in 0..self.spots {
            let angle = i as f32 / 6.0 * std::f32::consts::TAU + 0.3 * t;
            let rig = Vec3::new(1.0, 0.0, 2.0) + Vec3::new(angle.cos() * 4.0, 6.0, angle.sin() * 4.0);
            let aim = Vec3::new(1.0, 0.0, 2.0) + 0.6 * Vec3::new((t * 0.5 + i as f32).sin(), 0.0, (t * 0.5 + i as f32).cos());
            let (r, g, b) = SPOT_COLORS[i];
            let spot = SpotLight::new(rig, aim - rig, Color4::new(r, g, b, 1.0), 30.0, 15.0, 12f32.to_radians(), 22f32.to_radians()).with_shadows(true);
            lighting.local_lights.push(spot.into());
        }
        self.show_region = settings.on("show-region");
        let tree = &self.instances[self.tall_tree];
        let (min, max) = tree.model().culling_bounds();
        self.tree_in_view = CullVolume::from_camera(frame.camera, aspect(frame.css)).intersects_transformed_box(min, max, tree.world);
        let sun_texels = if lighting.sun_shadows.enabled { u64::from(size.texels()) } else { 0 };
        let spot_texels = if self.spots > 0 { u64::from(spot_size.texels()) } else { 0 };
        self.map_bytes = 4 * (sun_texels * sun_texels + 4 * spot_texels * spot_texels);
        self.lighting = lighting;
        Ok(())
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, context: &mut DrawContext<'_>) -> Result<(), GraphicsException> {
        if let Some(renderer) = self.renderer.as_mut() {
            draw_models(renderer, &self.instances, pass, context, &self.lighting)?;
        }
        if self.show_region {
            if let Some(glows) = self.glows.as_mut() {
                glows.begin(pass, context.camera, context.time)?;
                let (min, max) = self.region;
                for glow in box_outline(min, max) {
                    glows.add(&glow);
                }
                glows.end(pass)?;
            }
        }
        Ok(())
    }

    fn stats(&self) -> ModelRenderStats {
        self.renderer.as_ref().map(|r| r.frame_stats()).unwrap_or_default()
    }

    fn readouts(&self, out: &mut Vec<Readout>) {
        let stats = self.stats();
        out.push(Readout::new("shadow-passes", "Shadow passes", stats.shadow_passes));
        out.push(Readout::new("shadow-draw-calls", "Shadow draw calls", stats.shadow_draw_calls));
        out.push(Readout::new("draw-calls", "Draw calls (camera)", stats.draw_calls));
        out.push(Readout::new("culled", "Instances culled for the camera", stats.culled_instances));
        out.push(Readout::new("tree-in-view", "The tall tree in the camera's view", if self.tree_in_view { "yes" } else { "no (culled for the camera)" }));
        let casting = self.spots.min(SceneLighting::MAX_SHADOWED_SPOTLIGHTS);
        out.push(Readout::new("spots-casting", "Spotlights with shadows", casting));
        out.push(Readout::new("spots-without", "Spotlights lighting without shadows", self.spots - casting));
        out.push(Readout::new("map-memory", "Shadow map memory (MB)", format!("{:.0}", self.map_bytes as f64 / (1024.0 * 1024.0))));
        out.push(Readout::new("depth-pipelines", "Shadow depth pipelines", self.renderer.as_ref().map_or(0, |r| r.shadow_pipeline_count())));
        let (min, max) = self.region;
        out.push(Readout::new("region", "Covered region", format!("({:.0}, {:.0}, {:.0}) to ({:.0}, {:.0}, {:.0})", min.x, min.y, min.z, max.x, max.y, max.z)));
    }
}

/// Small glows along the twelve edges of a box, one a metre.
fn box_outline(min: Vec3, max: Vec3) -> Vec<Glow> {
    let corners: Vec<Vec3> = (0..8).map(|i| Vec3::new(if i & 1 == 0 { min.x } else { max.x }, if i & 2 == 0 { min.y } else { max.y }, if i & 4 == 0 { min.z } else { max.z })).collect();
    let mut glows = Vec::new();
    for a in 0..8usize {
        for bit in [1usize, 2, 4] {
            if a & bit == 0 {
                let (from, to) = (corners[a], corners[a | bit]);
                let steps = (from.distance(to).ceil() as usize).max(1);
                for s in 0..=steps {
                    glows.push(Glow::new(from.lerp(to, s as f32 / steps as f32), 0.35, Vec3::new(1.0, 0.85, 0.2), 3.0));
                }
            }
        }
    }
    glows
}
