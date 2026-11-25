//! "Picking and labels": a click on the canvas PICKS the model under the pointer - the camera's
//! ray through the canvas point (`Camera::canvas_to_ray`) against every instance's box
//! (`Model::culling_bounds()`, which holds every pose of its clips, or - a control - the rest
//! pose's `bounds_min()` / `bounds_max()`, placed by the instance's world matrix:
//! `Ray::nearest_box_hit`) - and the picked one is highlighted with a gold `tint`. Every
//! model has a LABEL in the page's own text, placed over the top of its box with
//! `Camera::world_to_canvas` each frame, so the labels follow the walking and flying models and
//! hide when their model is behind the camera or outside the canvas.

use codebrix_webgpu::glam::Vec3;
use codebrix_webgpu::{CanvasInput, PointerButton};
use codebrix_webgpu_graphics::{Camera, GraphicsException, ModelInstance, ModelRenderStats, ModelRenderer, PlayOptions, RenderPass, SceneLighting};

use crate::demos::kit::Kit;
use crate::demos::{dim_lighting, draw_models, on_circle, placed, world, Control, Demo, DemoFrame, DrawContext, Framing, PageLabel, Readout, Settings};

/// The controls.
pub const CONTROLS: &[Control] = &[
    Control::Toggle { key: "labels", label: "Labels", default: true },
    Control::Choice { key: "boxes", label: "Pick against", options: &[("pose-covering boxes (culling_bounds)", 0.0), ("rest-pose boxes (bounds)", 1.0)], default: 0.0 },
    Control::Button { key: "clear", label: "Clear the pick" },
];

/// What to look at.
pub const LEGEND: &[&str] = &[
    "Click a model: it turns gold and its label is highlighted; click the ground to clear.",
    "The labels are page text: they follow the models and hide when a model is behind the camera.",
    "Pose-covering boxes hold every pose of EVERY clip (the Avian's take-off and landing too): big for some models.",
    "Dragging orbits as usual (a drag is not a click).",
];

/// The gold of the picked instance.
const PICKED_TINT: Vec3 = Vec3::new(1.7, 1.35, 0.35);

/// The picking demo.
#[derive(Default)]
pub struct PickingDemo {
    renderer: Option<ModelRenderer>,
    instances: Vec<ModelInstance>,
    names: Vec<String>,
    walkers: Vec<(usize, f32, f32, f32)>,
    avian: usize,
    picked: Option<(usize, f32)>,
    picks: u32,
    misses: u32,
    pressed_at: Option<(f64, f64)>,
    last_click: Option<(f32, f32)>,
    labels_on: bool,
    rest_boxes: bool,
    clear_presses: f32,
    lighting: SceneLighting,
}

impl PickingDemo {
    /// The box an instance is picked by (model space): the one covering every pose of its clips,
    /// or its rest pose's.
    fn pick_box(&self, instance: &ModelInstance) -> (Vec3, Vec3) {
        let model = instance.model();
        if self.rest_boxes { (model.bounds_min(), model.bounds_max()) } else { model.culling_bounds() }
    }
}

impl Demo for PickingDemo {
    fn build(&mut self, kit: &mut Kit<'_>) -> Result<Framing, String> {
        let ground = kit.ground()?;
        let mut instances = vec![ground.create_instance()];
        let mut names = vec![String::new()];
        for (label, name, scale, yaw, at) in [
            ("chest", "chest", 3.2, 0.5, Vec3::new(0.0, 0.0, 0.0)),
            ("barrel", "barrel", 2.2, 0.0, Vec3::new(2.2, 0.0, -1.0)),
            ("second barrel", "barrel", 2.2, 1.0, Vec3::new(2.7, 0.0, -0.4)),
            ("tree", "tree", 2.6, 0.0, Vec3::new(-4.0, 0.0, -3.0)),
            ("tall tree", "tree-tall", 2.6, 0.0, Vec3::new(3.5, 0.0, -4.0)),
            ("autumn tree", "tree-autumn", 2.8, 0.0, Vec3::new(-5.0, 0.0, 2.5)),
            ("rock", "rock-a", 2.2, 0.4, Vec3::new(-1.8, 0.0, 2.6)),
            ("cat", "cat", 0.55, 2.4, Vec3::new(1.5, 0.0, 2.2)),
            ("cow", "cow", 0.6, -0.6, Vec3::new(-2.6, 0.0, -1.0)),
        ] {
            let mut instance = placed(&kit.model(name)?, scale, yaw, at);
            if name == "cat" || name == "cow" {
                instance.animation().play_named("idle", PlayOptions::default()).map_err(|e| e.to_string())?;
            }
            instances.push(instance);
            names.push(label.to_string());
        }
        for (label, name, scale, radius, speed) in [("fox", "fox", 0.55, 5.0, 0.25), ("dog", "dog", 0.55, 6.0, -0.2), ("archer", "archer", 2.0, 3.6, 0.3)] {
            let mut walker = kit.model(name)?.create_instance();
            walker.animation().play_named("walk", PlayOptions::default()).map_err(|e| e.to_string())?;
            self.walkers.push((instances.len(), scale, radius, speed));
            instances.push(walker);
            names.push(label.to_string());
        }
        let mut bird = kit.model("avian")?.create_instance();
        bird.animation().play_named("fly", PlayOptions::default()).map_err(|e| e.to_string())?;
        self.avian = instances.len();
        instances.push(bird);
        names.push("Avian".to_string());
        self.instances = instances;
        self.names = names;
        self.renderer = Some(kit.graphics.create_model_renderer().map_err(|e| e.to_string())?);
        Ok(Framing { min: Vec3::new(-6.0, 0.0, -5.0), max: Vec3::new(6.0, 3.5, 5.0), yaw: 0.3, pitch: 0.35, distance_scale: 0.8, auto_orbit: false })
    }

    fn update(&mut self, frame: &DemoFrame<'_>, settings: &Settings) -> Result<(), String> {
        let t = frame.time as f32;
        for &(index, scale, radius, speed) in &self.walkers {
            let (at, yaw) = on_circle(Vec3::ZERO, radius, speed * t);
            let walker = &mut self.instances[index];
            walker.world = world(scale, if speed < 0.0 { yaw + std::f32::consts::PI } else { yaw }, at);
        }
        let (at, yaw) = on_circle(Vec3::new(0.0, 3.0, 0.0), 4.0, -0.4 * t);
        self.instances[self.avian].world = world(0.36, yaw + std::f32::consts::PI, at);
        for instance in self.instances.iter_mut() {
            if instance.has_animation_player() {
                instance.animation().update_to(frame.time);
            }
        }
        if settings.get("clear") != self.clear_presses {
            self.clear_presses = settings.get("clear");
            self.picked = None;
        }
        for (index, instance) in self.instances.iter_mut().enumerate() {
            instance.tint = if self.picked.is_some_and(|(p, _)| p == index) { PICKED_TINT } else { Vec3::ONE };
        }
        self.labels_on = settings.on("labels");
        self.rest_boxes = settings.get("boxes") >= 0.5;
        self.lighting = dim_lighting(frame.environment, 1.0, 2.5, Vec3::new(-0.4, -1.0, -0.3));
        Ok(())
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, context: &mut DrawContext<'_>) -> Result<(), GraphicsException> {
        match self.renderer.as_mut() {
            Some(renderer) => draw_models(renderer, &self.instances, pass, context, &self.lighting),
            None => Ok(()),
        }
    }

    fn stats(&self) -> ModelRenderStats {
        self.renderer.as_ref().map(|r| r.frame_stats()).unwrap_or_default()
    }

    fn readouts(&self, out: &mut Vec<Readout>) {
        out.push(Readout::new("picked", "Picked", self.picked.map_or("nothing".to_string(), |(i, _)| self.names[i].clone())));
        out.push(Readout::new("distance", "Distance to the hit (m)", self.picked.map_or("-".to_string(), |(_, d)| format!("{d:.2}"))));
        out.push(Readout::new("picked-box", "The picked model's box (model space, size)", self.picked.map_or("-".to_string(), |(i, _)| {
            let (min, max) = self.pick_box(&self.instances[i]);
            let size = max - min;
            format!("{:.2} x {:.2} x {:.2}", size.x, size.y, size.z)
        })));
        out.push(Readout::new("picks", "Picks", self.picks));
        out.push(Readout::new("misses", "Clicks that hit no model", self.misses));
        out.push(Readout::new("last-click", "Last click (CSS px)", self.last_click.map_or("-".to_string(), |(x, y)| format!("{x:.0}, {y:.0}"))));
    }

    fn labels(&self, camera: &Camera, css: (f32, f32)) -> Vec<PageLabel> {
        if !self.labels_on {
            return Vec::new();
        }
        self.instances.iter().enumerate().skip(1).map(|(index, instance)| {
            let (min, max) = self.pick_box(instance);
            let top = instance.world.transform_point3(Vec3::new((min.x + max.x) * 0.5, max.y, (min.z + max.z) * 0.5));
            let at = camera.world_to_canvas(top, css.0, css.1);
            PageLabel { key: format!("{index}"), text: self.names[index].clone(), x: at.x, y: at.y, shown: at.in_view, highlighted: self.picked.is_some_and(|(p, _)| p == index) }
        }).collect()
    }

    fn input(&mut self, input: &CanvasInput, camera: &Camera, css: (f32, f32)) -> bool {
        match input {
            CanvasInput::PointerDown { x, y, button: PointerButton::Primary, modifiers, .. } if !modifiers.shift => {
                self.pressed_at = Some((*x, *y));
                false
            }
            CanvasInput::PointerUp { x, y, .. } => {
                if let Some((x0, y0)) = self.pressed_at.take() {
                    if (x - x0).abs() < 4.0 && (y - y0).abs() < 4.0 {
                        let ray = camera.canvas_to_ray(*x as f32, *y as f32, css.0, css.1);
                        // Every instance but the ground (index 0), by its pose-covering box.
                        let hit = ray.nearest_box_hit(self.instances.iter().skip(1).map(|i| {
                            let (min, max) = self.pick_box(i);
                            (min, max, i.world)
                        }));
                        self.last_click = Some((*x as f32, *y as f32));
                        match hit {
                            Some((k, distance)) => {
                                self.picked = Some((k + 1, distance));
                                self.picks += 1;
                            }
                            None => {
                                self.picked = None;
                                self.misses += 1;
                            }
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }
}
