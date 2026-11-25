//! "Lights": point lights and spotlights at dusk (the graphics crate's `SceneLighting::
//! local_lights`). The sky and the key light are turned almost off, so the local lights carry the
//! picture: up to 40 coloured lamps over two rows of barrels (each a point light whose light ends
//! smoothly at its range), a spotlight high above the middle pointing down (full brightness inside
//! its inner cone, eased to nothing at its outer cone), and one lamp CARRIED by the walking archer -
//! a point light placed every frame at the archer's right arm (the instance's node world matrix).
//! More lamps than the renderer's ceiling of 16: the renderer shades the 16 that matter most to the
//! view (brightest for their distance from the camera, among those whose reach is in view); the page
//! works out the same choice to show how many are lit.

use codebrix_webgpu::glam::Vec3;
use codebrix_webgpu_graphics::{Color4, CullVolume, GraphicsException, LocalLight, ModelInstance, ModelRenderStats, ModelRenderer, PlayOptions, PointLight, RenderPass, SceneLighting, SpotLight};

use crate::demos::kit::Kit;
use crate::demos::{aspect, dim_lighting, draw_models, on_circle, placed, world, Control, Demo, DemoFrame, DrawContext, Framing, Readout, Settings};

/// The controls.
pub const CONTROLS: &[Control] = &[
    Control::Slider { key: "lamps", label: "Lamps", min: 0.0, max: 40.0, step: 1.0, default: 8.0 },
    Control::Slider { key: "lamp-intensity", label: "Lamp intensity", min: 1.0, max: 20.0, step: 0.5, default: 6.0 },
    Control::Slider { key: "lamp-range", label: "Lamp range (m)", min: 2.0, max: 20.0, step: 0.5, default: 8.0 },
    Control::Toggle { key: "moving", label: "Moving lamps", default: true },
    Control::Toggle { key: "spot", label: "Spotlight", default: true },
    Control::Slider { key: "spot-inner", label: "Spot inner angle", min: 0.0, max: 60.0, step: 1.0, default: 15.0 },
    Control::Slider { key: "spot-outer", label: "Spot outer angle", min: 1.0, max: 80.0, step: 1.0, default: 25.0 },
    Control::Toggle { key: "hand-lamp", label: "Lamp in the archer's hand", default: true },
];

/// What to look at.
pub const LEGEND: &[&str] = &[
    "Each lamp makes a soft pool that ends smoothly at its range: no ring where the light stops.",
    "The spotlight: a bright core inside the inner angle, a soft rim out to the outer angle.",
    "The archer carries a cyan lamp on its arm (the arm node's world matrix).",
    "Above 16 lights only the 16 that matter most are shaded: far lamps go dark, never one next to the camera.",
];

/// The lamp colours, in turn.
const LAMP_COLORS: [(f32, f32, f32); 5] = [(1.0, 0.72, 0.42), (1.0, 0.5, 0.25), (1.0, 0.42, 0.55), (0.55, 0.7, 1.0), (1.0, 0.9, 0.6)];

/// The lights demo.
#[derive(Default)]
pub struct LightsDemo {
    renderer: Option<ModelRenderer>,
    instances: Vec<ModelInstance>,
    archer: usize,
    posts: Vec<Vec3>,
    lighting: SceneLighting,
    given: usize,
    reaching: usize,
    hand: Option<Vec3>,
}

impl LightsDemo {
    /// The 40 lamp places, nearest the middle first: two rows of 20 along X.
    fn post_places() -> Vec<Vec3> {
        let mut places: Vec<Vec3> = (0..20).flat_map(|i| [-1.0f32, 1.0].map(|side| Vec3::new(-19.0 + 2.0 * i as f32, 0.0, side * 3.5))).collect();
        places.sort_by(|a, b| a.x.abs().total_cmp(&b.x.abs()).then(a.z.total_cmp(&b.z)));
        places
    }
}

impl Demo for LightsDemo {
    fn build(&mut self, kit: &mut Kit<'_>) -> Result<Framing, String> {
        let ground = kit.ground()?;
        let barrel = kit.model("barrel")?;
        let archer = kit.model("archer")?;
        let mut instances = vec![ground.create_instance()];
        self.posts = LightsDemo::post_places();
        for post in &self.posts {
            instances.push(placed(&barrel, 2.2, 0.3 * post.x, *post));
        }
        for (name, scale, at) in [("tree", 2.6, Vec3::new(-6.0, 0.0, -8.0)), ("tree-tall", 2.6, Vec3::new(2.0, 0.0, -9.0)), ("tree-autumn", 2.8, Vec3::new(8.0, 0.0, -7.5)), ("rock-a", 2.2, Vec3::new(-3.0, 0.0, 6.5)), ("rock-b", 2.0, Vec3::new(4.5, 0.0, 6.0)), ("chest", 3.2, Vec3::new(0.0, 0.0, 0.0)), ("fox", 0.55, Vec3::new(-1.5, 0.0, 1.2))] {
            instances.push(placed(&kit.model(name)?, scale, 0.4, at));
        }
        let mut walker = archer.create_instance();
        walker.animation().play_named("walk", PlayOptions::default()).map_err(|e| e.to_string())?;
        self.archer = instances.len();
        instances.push(walker);
        self.instances = instances;
        self.renderer = Some(kit.graphics.create_model_renderer().map_err(|e| e.to_string())?);
        Ok(Framing { min: Vec3::new(-20.0, 0.0, -9.0), max: Vec3::new(20.0, 3.0, 9.0), yaw: 0.35, pitch: 0.42, distance_scale: 0.55, auto_orbit: true })
    }

    fn update(&mut self, frame: &DemoFrame<'_>, settings: &Settings) -> Result<(), String> {
        let t = frame.time as f32;
        // The archer walks a circle round the chest, its clip on the frame's time.
        let (at, yaw) = on_circle(Vec3::ZERO, 3.0, 0.35 * t);
        let archer = &mut self.instances[self.archer];
        archer.world = world(2.0, yaw, at);
        archer.animation().update_to(frame.time);
        self.hand = archer.get_node_world_matrix_named("arm-right").map(|m| m.transform_point3(Vec3::ZERO) + Vec3::new(0.0, 0.25, 0.0));
        let mut lighting = dim_lighting(frame.environment, 0.12, 0.05, Vec3::new(-0.3, -1.0, -0.2));
        let count = (settings.get("lamps").round() as usize).min(self.posts.len());
        let intensity = settings.get("lamp-intensity");
        let range = settings.get("lamp-range");
        for (i, post) in self.posts.iter().take(count).enumerate() {
            let wobble = if settings.on("moving") && i % 4 == 0 { Vec3::new((t * 0.7 + i as f32).sin(), 0.0, (t * 0.7 + i as f32).cos()) } else { Vec3::ZERO };
            let (r, g, b) = LAMP_COLORS[i % LAMP_COLORS.len()];
            lighting.local_lights.push(PointLight::new(*post + Vec3::new(0.0, 1.6, 0.0) + wobble, Color4::new(r, g, b, 1.0), intensity, range).into());
        }
        if settings.on("spot") {
            let outer = settings.get("spot-outer").to_radians();
            let inner = settings.get("spot-inner").to_radians().min(outer);
            lighting.local_lights.push(SpotLight::new(Vec3::new(0.0, 7.0, 0.0), Vec3::NEG_Y, Color4::new(1.0, 0.95, 0.85, 1.0), 120.0, 12.0, inner, outer).into());
        }
        if settings.on("hand-lamp") {
            if let Some(hand) = self.hand {
                lighting.local_lights.push(PointLight::new(hand, Color4::new(0.3, 0.9, 1.0, 1.0), 4.0, 5.0).into());
            }
        }
        // The page's own count of the renderer's choice: lit lights whose reach is in view.
        let view = CullVolume::from_camera(frame.camera, aspect(frame.css));
        self.given = lighting.local_lights.len();
        self.reaching = lighting.local_lights.iter().filter(|light| light.is_lit() && reaches(&view, light)).count();
        self.lighting = lighting;
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
        // The renderer's own count of the local lights it shaded in this frame (its choice).
        let shaded = self.stats().local_lights;
        out.push(Readout::new("given", "Local lights given (lamps, spot, hand lamp)", self.given));
        out.push(Readout::new("reaching", "Lit and reaching the view", self.reaching));
        out.push(Readout::new("shaded", "Shaded this frame (the renderer's own count, at most 16)", shaded));
        out.push(Readout::new("skipped", "Skipped this frame", self.given - shaded));
        out.push(Readout::new("ceiling", "The renderer's ceiling", SceneLighting::MAX_LOCAL_LIGHTS));
        out.push(Readout::new("hand", "The archer's lamp at", self.hand.map_or("-".to_string(), |p| format!("({:.2}, {:.2}, {:.2})", p.x, p.y, p.z))));
        let stats = self.stats();
        out.push(Readout::new("draw-calls", "Draw calls", stats.draw_calls));
        out.push(Readout::new("pipelines", "Pipelines", self.renderer.as_ref().map_or(0, |r| r.pipeline_count())));
    }
}

/// Whether a light's reach (the sphere of its range) is not wholly outside `view` - the
/// renderer's own test.
fn reaches(view: &CullVolume, light: &LocalLight) -> bool {
    let center = light.position();
    let radius = light.range();
    view.planes().iter().all(|plane| plane.truncate().dot(center) + plane.w >= -radius)
}
