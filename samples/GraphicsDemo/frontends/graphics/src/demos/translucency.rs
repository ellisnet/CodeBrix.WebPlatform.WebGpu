//! "Translucency": blended parts and the per-instance look.
//!
//! - TWO-PASS DOUBLE-SIDED TRANSLUCENCY side by side: two glass models - the kit's barrel, its tall
//!   tree or the Avian, chosen on a control - (copies whose one material was made BLEND with 40 % opacity, double-sided, right after loading
//!   while the page held its only handle - `Model::material_mut`), turning, each in front of a fox; the left one drawn by a ModelRenderer with `set_two_pass_double_sided(true)` (the default: back
//!   faces first, then front faces), the right one by a second renderer with it off (one draw of
//!   both faces in triangle order) - each switch on a control.
//! - PER-INSTANCE LOOK on a row of five foxes of ONE model (a copy whose emissive was set to its own
//!   colour map): plain, tinted (`tint`), tinted blue, FADED (`alpha` below 1: every part drawn among
//!   the blended ones) and GLOWING (`emissive_strength`); the others have emissive strength 0.
//! - NODE VISIBILITY per instance: the chest's lid, the foxes' tails (`set_node_visible_named`).
//! - Emissive times vertex colour (`ModelMaterial::emissive_vertex_color`) is NOT shown: no model
//!   the sample has carries vertex colours (every kit model and both models built in code have
//!   white or no COLOR_0), so it would look exactly like plain emissive.

use codebrix_webgpu::glam::Vec3;
use codebrix_webgpu_graphics::{GraphicsException, MaterialAlphaMode, ModelInstance, ModelRenderStats, ModelRenderer, RenderPass, SceneLighting};

use crate::demos::kit::Kit;
use crate::demos::{dim_lighting, placed, world, Control, Demo, DemoFrame, DrawContext, Framing, Readout, Settings};

/// The controls.
pub const CONTROLS: &[Control] = &[
    Control::Toggle { key: "two-pass-left", label: "Two-pass, left barrel", default: true },
    Control::Toggle { key: "two-pass-right", label: "Two-pass, right barrel", default: false },
    Control::Choice { key: "glass", label: "Glass model", options: &[("barrel", 0.0), ("tall tree", 1.0), ("Avian", 2.0)], default: 0.0 },
    Control::Toggle { key: "turn", label: "Turn the glass models", default: true },
    Control::Slider { key: "fade", label: "Fade (fourth fox)", min: 0.0, max: 1.0, step: 0.05, default: 0.5 },
    Control::Toggle { key: "fade-clock", label: "Fade with the clock", default: false },
    Control::Slider { key: "emissive", label: "Emissive strength (fifth fox)", min: 0.0, max: 4.0, step: 0.1, default: 1.5 },
    Control::Choice { key: "tint", label: "Tint (second fox)", options: &[("red", 0.0), ("green", 1.0), ("gold", 2.0), ("none", 3.0)], default: 0.0 },
    Control::Toggle { key: "lid", label: "Chest lid shown", default: true },
    Control::Toggle { key: "tails", label: "Fox tails shown", default: true },
];

/// What to look at.
pub const LEGEND: &[&str] = &[
    "Left glass model (two-pass): its far side shows evenly through its near side from every angle.",
    "Right one (one pass, triangle order): patches of the far side drawn over the near side.",
    "Foxes: plain, tinted, blue, faded (alpha), glowing (emissive strength) - one model.",
    "Node visibility: the chest's lid and the foxes' tails switch per instance.",
    "Emissive x vertex colour is not shown: no model here has vertex colours.",
];

/// The glass models: (model, scale, lift above the ground).
const GLASS: [(&str, f32, f32); 3] = [("barrel", 7.0, 0.0), ("tree-tall", 1.4, 0.0), ("avian", 1.1, 1.1)];

/// The translucency demo.
#[derive(Default)]
pub struct TranslucencyDemo {
    left: Option<ModelRenderer>,
    right: Option<ModelRenderer>,
    instances: Vec<ModelInstance>,
    right_instances: Vec<ModelInstance>,
    left_glass: Vec<usize>,
    foxes: Vec<usize>,
    chest: usize,
    lighting: SceneLighting,
    fade: f32,
    lid_found: bool,
    tails_found: usize,
}

impl Demo for TranslucencyDemo {
    fn build(&mut self, kit: &mut Kit<'_>) -> Result<Framing, String> {
        let glass = |name: &str, kit: &mut Kit<'_>| {
            kit.changed(name, |model| {
                let material = model.material_mut(Some(0))?;
                material.alpha_mode = MaterialAlphaMode::Blend;
                material.base_color_factor.w = 0.4;
                material.double_sided = true;
                Ok(())
            })
        };
        let mut left_glass = Vec::new();
        let mut right_glass = Vec::new();
        for (name, scale, lift) in GLASS {
            left_glass.push(placed(&glass(name, kit)?, scale, 0.0, Vec3::new(-2.2, lift, -0.5)));
            right_glass.push(placed(&glass(name, kit)?, scale, 0.0, Vec3::new(2.2, lift, -0.5)));
        }
        let glowing_fox = kit.changed("fox", |model| {
            let material = model.material_mut(Some(0))?;
            material.emissive_texture = material.base_color_texture.clone();
            material.emissive_factor = Vec3::ONE;
            Ok(())
        })?;
        let fox = kit.model("fox")?;
        let ground = kit.ground()?;
        let mut instances = vec![ground.create_instance()];
        for instance in left_glass {
            self.left_glass.push(instances.len());
            instances.push(instance);
        }
        instances.push(placed(&fox, 0.55, 0.3, Vec3::new(-2.2, 0.0, -3.2)));
        instances.push(placed(&kit.model("tree-tall")?, 2.6, 0.0, Vec3::new(-4.5, 0.0, -6.0)));
        instances.push(placed(&kit.model("tree")?, 2.6, 0.0, Vec3::new(4.5, 0.0, -6.0)));
        for (k, x) in [-4.0f32, -2.0, 0.0, 2.0, 4.0].iter().enumerate() {
            let mut look = placed(&glowing_fox, 0.5, std::f32::consts::PI * 0.15, Vec3::new(*x, 0.0, 3.2));
            look.emissive_strength = 0.0;
            if k == 2 {
                look.tint = Vec3::new(0.35, 0.5, 1.6);
            }
            self.foxes.push(instances.len());
            instances.push(look);
        }
        self.chest = instances.len();
        instances.push(placed(&kit.model("chest")?, 3.2, -0.4, Vec3::new(0.0, 0.0, -2.5)));
        self.instances = instances;
        right_glass.push(placed(&fox, 0.55, -0.3, Vec3::new(2.2, 0.0, -3.2)));
        self.right_instances = right_glass;
        self.left = Some(kit.graphics.create_model_renderer().map_err(|e| e.to_string())?);
        self.right = Some(kit.graphics.create_model_renderer().map_err(|e| e.to_string())?);
        Ok(Framing { min: Vec3::new(-5.0, 0.0, -4.0), max: Vec3::new(5.0, 3.0, 4.0), yaw: 0.0, pitch: 0.3, distance_scale: 0.75, auto_orbit: false })
    }

    fn update(&mut self, frame: &DemoFrame<'_>, settings: &Settings) -> Result<(), String> {
        let t = frame.time as f32;
        let turn = if settings.on("turn") { 0.4 * t } else { 0.0 };
        let chosen = settings.get("glass") as usize;
        for (k, (_, scale, lift)) in GLASS.iter().enumerate() {
            let left = &mut self.instances[self.left_glass[k]];
            left.world = world(*scale, turn, Vec3::new(-2.2, *lift, -0.5));
            left.visible = k == chosen;
            let right = &mut self.right_instances[k];
            right.world = world(*scale, turn, Vec3::new(2.2, *lift, -0.5));
            right.visible = k == chosen;
        }
        self.fade = if settings.on("fade-clock") { 0.5 + 0.5 * (t * 1.2).sin() } else { settings.get("fade") };
        let tint = match settings.get("tint") as u32 {
            0 => Vec3::new(1.6, 0.35, 0.3),
            1 => Vec3::new(0.4, 1.5, 0.4),
            2 => Vec3::new(1.5, 1.2, 0.35),
            _ => Vec3::ONE,
        };
        let (second, fourth, fifth) = (self.foxes[1], self.foxes[3], self.foxes[4]);
        self.instances[second].tint = tint;
        self.instances[fourth].alpha = self.fade;
        self.instances[fifth].emissive_strength = settings.get("emissive");
        self.lid_found = self.instances[self.chest].set_node_visible_named("lid", settings.on("lid"));
        let tails = settings.on("tails");
        self.tails_found = 0;
        for &index in &self.foxes {
            if self.instances[index].set_node_visible_named("tail", tails) {
                self.tails_found += 1;
            }
        }
        for instance in self.right_instances.iter_mut().skip(GLASS.len()) {
            instance.set_node_visible_named("tail", tails);
        }
        if let Some(left) = self.left.as_mut() {
            left.set_two_pass_double_sided(settings.on("two-pass-left"));
        }
        if let Some(right) = self.right.as_mut() {
            right.set_two_pass_double_sided(settings.on("two-pass-right"));
        }
        self.lighting = dim_lighting(frame.environment, 1.0, 2.0, Vec3::new(-0.5, -1.0, -0.6));
        Ok(())
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, context: &mut DrawContext<'_>) -> Result<(), GraphicsException> {
        let (Some(left), Some(right)) = (self.left.as_mut(), self.right.as_mut()) else { return Ok(()) };
        // Two renderers in one pass: both queue, both draw their opaque parts, the sky, then both
        // draw their blended parts (the two glass barrels do not overlap on screen).
        left.begin(pass, context.camera, Some(&self.lighting))?;
        for instance in &self.instances {
            left.draw(instance)?;
        }
        right.begin(pass, context.camera, Some(&self.lighting))?;
        for instance in &self.right_instances {
            right.draw(instance)?;
        }
        left.end_opaque(pass)?;
        right.end_opaque(pass)?;
        context.skybox.draw(pass, context.camera, Some(&self.lighting))?;
        left.end_blended(pass)?;
        right.end_blended(pass)
    }

    fn stats(&self) -> ModelRenderStats {
        let mut stats = self.left.as_ref().map(|r| r.frame_stats()).unwrap_or_default();
        if let Some(right) = self.right.as_ref() {
            let r = right.frame_stats();
            stats.draw_calls += r.draw_calls;
            stats.instances += r.instances;
            stats.culled_instances += r.culled_instances;
        }
        stats
    }

    fn readouts(&self, out: &mut Vec<Readout>) {
        let on = |r: &Option<ModelRenderer>| r.as_ref().is_some_and(|r| r.two_pass_double_sided());
        out.push(Readout::new("left-two-pass", "Left renderer two-pass", on(&self.left)));
        out.push(Readout::new("right-two-pass", "Right renderer two-pass", on(&self.right)));
        out.push(Readout::new("left-draw-calls", "Left renderer draw calls", self.left.as_ref().map_or(0, |r| r.last_draw_call_count())));
        out.push(Readout::new("right-draw-calls", "Right renderer draw calls", self.right.as_ref().map_or(0, |r| r.last_draw_call_count())));
        out.push(Readout::new("fade-alpha", "Fourth fox's alpha", format!("{:.2}", self.fade)));
        out.push(Readout::new("lid-found", "Chest has a node 'lid'", self.lid_found));
        out.push(Readout::new("tails-found", "Foxes with a node 'tail'", self.tails_found));
    }
}
