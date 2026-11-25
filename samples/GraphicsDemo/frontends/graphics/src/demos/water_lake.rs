//! "Water lake": a large lake of OPEN-WATER WAVES (`OpenWaterWaves::from_wind`, a seeded wind sea
//! of Gerstner waves) drawn by the graphics crate's water surface over a lake bed made in code (a
//! MeshRenderer terrain: deep in the middle, sloping up to a grassy bank on the near side, two
//! islands, hills round the far edge), so the depth tint runs from the sandy shallows to blue-green
//! deep water and the caustics show in the shallows. The kits' trees, rocks, a walking archer and
//! foxes stand on the bank and the islands; the sun's shadow falls on the water (the model
//! renderer's sun shadow map handed to the water); two lamps by the shore leave highlights on the
//! waves; the environment and the screen-space reflections (switchable) mirror the sky and the
//! trees. A barrel FLOATS on the swell (`WaterSurface::height_at` / `normal_at`, the same function
//! of time as the waves), and a dog SWIMS in a circle near the shore drawing a WAKE into a small
//! wave field on top of the waves (a second `WaterSurface` over the same capture, its grid aligned
//! with the lake's so the two surfaces meet exactly; the pointer stirs it too).

use codebrix_webgpu::glam::{Mat4, Quat, Vec2, Vec3};
use codebrix_webgpu::{CanvasInput, PointerButton};
use codebrix_webgpu_graphics::{
    BloomRenderer, Camera, Color4, GpuTexture, GraphicsContext, GraphicsException, GraphicsFrame, HeadlessRenderTarget, LitMaterial, MaterialOptions, Mesh, MeshRenderer, MeshVertex, ModelInstance, ModelRenderStats, ModelRenderer, OpenWaterWaves, PlayOptions, PointLight, RenderPass, RenderPassOptions, SceneLighting, SunShadowSettings, WaterCaustics, WaterField, WaterFrame, WaterLook,
    WaterRenderer, WaterSurface, WaveEdge, WaveField, WaveFieldOptions, WaveStir, WaveWake,
};

use crate::demos::kit::Kit;
use crate::demos::{on_circle, placed, world, Control, Demo, DemoFrame, DrawContext, Framing, Readout, Settings};

/// The lake's water surface: its size (square), grid and the cell that follows.
const LAKE_SIZE: f32 = 240.0;
const LAKE_GRID: u32 = 256;
/// The wake field's surface: as many lake cells across, aligned with the lake's grid.
const WAKE_CELLS: u32 = 32;
/// Its centre (x, z): on a lake grid line plus half its size.
const WAKE_CENTER: (f32, f32) = (0.0, 8.4375);
/// The swimmer's circle round a point in the wake field (x, z, radius).
const SWIM: (f32, f32, f32) = (0.0, 8.0, 6.0);
/// The swimmer's size.
const SWIMMER_SCALE: f32 = 1.0;
/// The barrel's size.
const BARREL_SCALE: f32 = 4.0;
/// Where the barrel floats (x, z).
const BARREL_AT: (f32, f32) = (19.0, 10.0);

/// The controls.
pub const CONTROLS: &[Control] = &[
    Control::Slider { key: "wind", label: "Wind speed (m/s)", min: 1.0, max: 8.0, step: 0.5, default: 4.0 },
    Control::Slider { key: "direction", label: "Wind direction (degrees)", min: 0.0, max: 360.0, step: 5.0, default: 100.0 },
    Control::Slider { key: "waves", label: "Waves", min: 4.0, max: 16.0, step: 1.0, default: 8.0 },
    Control::Slider { key: "seed", label: "Sea seed", min: 1.0, max: 9.0, step: 1.0, default: 1.0 },
    Control::Toggle { key: "ssr", label: "Screen-space reflections", default: true },
    Control::Slider { key: "reflectivity", label: "Reflection strength", min: 0.0, max: 1.5, step: 0.05, default: 1.0 },
    Control::Choice { key: "water", label: "Water (absorption, depth tint)", options: &[("clear lake", 0.0), ("library default", 1.0), ("murky", 2.0)], default: 1.0 },
    Control::Toggle { key: "caustics", label: "Caustics", default: true },
    Control::Slider { key: "sun", label: "Sun height (degrees)", min: 4.0, max: 60.0, step: 1.0, default: 32.0 },
    Control::Toggle { key: "shadows", label: "Sun shadows", default: true },
    Control::Toggle { key: "lamps", label: "Lamps by the shore", default: true },
    Control::Choice { key: "wake", label: "Water", options: &[("waves + a wake field by the shore", 1.0), ("waves only", 0.0)], default: 1.0 },
    Control::Toggle { key: "bloom", label: "Bloom", default: true },
    Control::Choice { key: "aa", label: "Anti-aliasing", options: &[("Off", 1.0), ("4x", 4.0)], default: 1.0 },
];

/// What to look at.
pub const LEGEND: &[&str] = &[
    "The shallows by the islands and the bank show the sandy bed and its caustics; the deep middle turns blue-green.",
    "The barrel rides the crests without sinking into them or floating above them; pause or seek: the waves (and the barrel) are an exact function of time.",
    "The sky and the near trees show in the water (screen-space reflections: switch them off to compare); glints break up on the waves and bloom; the lamps' highlights trail toward you.",
    "The trees on the islands lay their shadows on the water. The swimming dog leaves a wake; drag on the water near it to stir the wake field too. Stronger wind: longer, higher waves.",
];

/// The lake bed's height at (x, z): deep in the middle, a bank rising on the near side (+z), two
/// islands, hills round the far edge.
fn bed(x: f32, z: f32) -> f32 {
    let deep = -4.2 + 1.2 * (x * 0.05).sin() * (z * 0.04).cos();
    let t = ((z - 14.0) / 20.0).clamp(0.0, 1.0);
    let bank = t * t * (3.0 - 2.0 * t);
    let land = 1.6 + 0.3 * (x * 0.15).sin();
    let mut y = deep + (land - deep) * bank;
    for (cx, cz, r, h) in [(-22.0f32, -6.0f32, 9.0f32, 5.6f32), (24.0, -22.0, 6.0, 5.0)] {
        let d2 = ((x - cx).powi(2) + (z - cz).powi(2)) / (r * r);
        y += h * (-d2).exp();
    }
    let rim = ((x.abs().max(z.abs()) - 90.0) / 35.0).clamp(0.0, 1.0);
    y + 7.0 * rim * rim
}

/// The terrain mesh: a grid over the lake, vertex colours by height (silt, sand, grass).
fn terrain(graphics: &GraphicsContext) -> Result<Mesh, GraphicsException> {
    let (n, extent) = (161u32, 260.0f32);
    let cell = extent / (n - 1) as f32;
    let mut vertices = Vec::with_capacity((n * n) as usize);
    for j in 0..n {
        for i in 0..n {
            let (x, z) = (-extent * 0.5 + i as f32 * cell, -extent * 0.5 + j as f32 * cell);
            let y = bed(x, z);
            let (dx, dz) = (bed(x + 0.5, z) - bed(x - 0.5, z), bed(x, z + 0.5) - bed(x, z - 0.5));
            let normal = Vec3::new(-dx, 1.0, -dz).normalize();
            let silt = Vec3::new(0.34, 0.32, 0.26);
            let sand = Vec3::new(0.72, 0.66, 0.48);
            let grass = Vec3::new(0.17, 0.30, 0.11);
            let c = if y < -2.5 { silt.lerp(sand, ((y + 4.5) / 2.0).clamp(0.0, 1.0)) } else { sand.lerp(grass, ((y - 0.3) / 0.6).clamp(0.0, 1.0)) };
            vertices.push(MeshVertex::new(Vec3::new(x, y, z), normal, Vec2::new(i as f32, j as f32), Color4::new(c.x, c.y, c.z, 1.0)));
        }
    }
    let mut indices = Vec::with_capacity(((n - 1) * (n - 1) * 6) as usize);
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let a = j * n + i;
            indices.extend_from_slice(&[a, a + n, a + 1, a + 1, a + n, a + n + 1]);
        }
    }
    graphics.create_mesh_from(&vertices, &indices, Some("lake bed"))
}

/// Where the bed crosses `level` going from (x0, z0) toward (x1, z1) (by halving).
fn crossing(x0: f32, z0: f32, x1: f32, z1: f32, level: f32) -> (f32, f32) {
    let (mut a, mut b) = (0.0f32, 1.0f32);
    for _ in 0..40 {
        let m = (a + b) * 0.5;
        let below = bed(x0 + (x1 - x0) * m, z0 + (z1 - z0) * m) < level;
        if below == (bed(x0, z0) < level) {
            a = m;
        } else {
            b = m;
        }
    }
    (x0 + (x1 - x0) * a, z0 + (z1 - z0) * a)
}

/// The lake demo.
#[derive(Default)]
pub struct WaterLakeDemo {
    models: Option<ModelRenderer>,
    meshes: Option<MeshRenderer>,
    water: Option<WaterRenderer>,
    bloom: Option<BloomRenderer>,
    bed: Option<Mesh>,
    bed_material: Option<LitMaterial>,
    field: Option<WaveField>,
    lake: WaterSurface,
    wake: WaterSurface,
    instances: Vec<ModelInstance>,
    archer: usize,
    barrel: usize,
    swimmer: usize,
    lamps: Vec<Vec3>,
    lighting: SceneLighting,
    settings: Settings,
    sea_key: (f32, f32, f32, f32),
    time: f64,
    barrel_y: f32,
    barrel_tilt: f32,
    stirring: bool,
    /// How far below the water line the barrel's and the swimmer's origins sit (from their bounds).
    barrel_sink: f32,
    swimmer_sink: f32,
    probes: Vec<Readout>,
    waterline: f32,
}

impl WaterLakeDemo {
    fn wake_on(&self) -> bool {
        self.settings.get("wake") >= 0.5
    }
}

impl Demo for WaterLakeDemo {
    fn build(&mut self, kit: &mut Kit<'_>) -> Result<Framing, String> {
        let graphics = kit.graphics.clone();
        let e = |e: GraphicsException| e.to_string();
        let meshes = graphics.create_mesh_renderer().map_err(e)?;
        self.bed_material = Some(meshes.create_material(Color4::WHITE, MaterialOptions { label: Some("lake bed"), ..MaterialOptions::default() }).map_err(e)?);
        self.bed = Some(terrain(&graphics).map_err(e)?);
        self.meshes = Some(meshes);
        let on_bed = |x: f32, z: f32| Vec3::new(x, bed(x, z), z);
        let (tree, tall, autumn, rock_a, rock_c, fence, barrel, fox, dog, archer) = (kit.model("tree")?, kit.model("tree-tall")?, kit.model("tree-autumn")?, kit.model("rock-a")?, kit.model("rock-c")?, kit.model("fence")?, kit.model("barrel")?, kit.model("fox")?, kit.model("dog")?, kit.model("archer")?);
        let mut instances = Vec::new();
        // The bank: a row of trees, rocks, foxes.
        for (i, x) in [-34.0f32, -26.0, -17.0, -9.5, 13.0, 21.0, 29.0, 37.0].into_iter().enumerate() {
            let z = 38.0 + (i % 3) as f32 * 3.0;
            let model = [&tree, &tall, &autumn][i % 3];
            instances.push(placed(model, 3.2, i as f32 * 1.3, on_bed(x, z)));
        }
        instances.push(placed(&rock_a, 2.6, 0.4, on_bed(-4.0, 33.0)));
        instances.push(placed(&rock_c, 2.2, 1.9, on_bed(16.0, 31.0)));
        for (i, x) in [-12.0f32, 9.0].into_iter().enumerate() {
            let mut animal = placed(&fox, 1.1, 2.6 + i as f32, on_bed(x, 35.0));
            animal.animation().play_named("walk", PlayOptions::default()).ok();
            instances.push(animal);
        }
        // The islands.
        instances.push(placed(&tall, 5.4, 0.3, on_bed(-23.0, -7.0)));
        instances.push(placed(&tree, 5.0, 1.1, on_bed(-18.5, -4.0)));
        instances.push(placed(&autumn, 4.6, 2.0, on_bed(-25.5, -3.0)));
        instances.push(placed(&rock_a, 2.4, 0.8, on_bed(-18.5, -8.5)));
        instances.push(placed(&tall, 5.0, 1.7, on_bed(24.0, -22.0)));
        instances.push(placed(&rock_c, 2.6, 0.2, on_bed(26.5, -20.0)));
        // The shore: fences along the waterline, a lamp over two of them.
        let mut lamps = Vec::new();
        for (i, x) in [-14.0f32, -10.0, -6.0, 6.0, 10.0, 14.0].into_iter().enumerate() {
            let (_, z) = crossing(x, 0.0, x, 40.0, 0.25);
            instances.push(placed(&fence, 2.4, 0.0, on_bed(x, z + 1.0)));
            if i == 1 || i == 4 {
                lamps.push(Vec3::new(x, bed(x, z + 1.0) + 2.4, z + 1.0));
            }
        }
        self.waterline = crossing(0.0, 0.0, 0.0, 40.0, 0.0).1;
        self.lamps = lamps;
        // The walker on the bank, the barrel on the swell, the swimmer in the wake field.
        let mut walker = archer.create_instance();
        walker.animation().play_named("walk", PlayOptions::default()).map_err(|e| e.to_string())?;
        self.archer = instances.len();
        instances.push(walker);
        // The barrel floats with 45 % of its height under water, the dog with 60 %.
        self.barrel_sink = barrel.bounds_min().y * BARREL_SCALE + (barrel.bounds_max().y - barrel.bounds_min().y) * BARREL_SCALE * 0.45;
        self.swimmer_sink = dog.bounds_min().y * SWIMMER_SCALE + (dog.bounds_max().y - dog.bounds_min().y) * SWIMMER_SCALE * 0.6;
        self.barrel = instances.len();
        instances.push(barrel.create_instance());
        let mut swimmer = dog.create_instance();
        swimmer.animation().play_named("walk", PlayOptions::default()).ok();
        self.swimmer = instances.len();
        instances.push(swimmer);
        self.instances = instances;
        // The water: the lake, and the wake field's surface over the same capture.
        let cell = LAKE_SIZE / LAKE_GRID as f32;
        let wake_size = WAKE_CELLS as f32 * cell;
        let field = graphics.create_wave_field(&WaveFieldOptions { columns: 128, rows: 128, size: Vec2::splat(wake_size), wave_speed: 2.5, damping: 0.5, edge: WaveEdge::ABSORB, label: Some("wake".to_string()), ..WaveFieldOptions::default() }).map_err(e)?;
        self.lake = WaterSurface { center: Vec3::ZERO, size: Vec2::splat(LAKE_SIZE), grid: LAKE_GRID, ..WaterSurface::default() };
        // The wake surface's vertices are the lake's own (the same cell, on the lake's grid lines),
        // the field only shades its normals, and it lies 3 mm higher: it covers the lake exactly.
        self.wake = WaterSurface { center: Vec3::new(WAKE_CENTER.0, 0.003, WAKE_CENTER.1), size: Vec2::splat(wake_size), grid: WAKE_CELLS, field: Some(WaterField { displace: false, ..WaterField::of(&field) }), ..WaterSurface::default() };
        self.field = Some(field);
        self.water = Some(graphics.create_water_renderer().map_err(e)?);
        self.models = Some(graphics.create_model_renderer().map_err(e)?);
        self.bloom = Some(graphics.create_bloom_renderer().map_err(e)?);
        self.settings = Settings::from_controls(CONTROLS);
        self.sea_key = (-1.0, 0.0, 0.0, 0.0);
        Ok(Framing { min: Vec3::new(-34.0, -2.0, -34.0), max: Vec3::new(34.0, 3.0, 24.0), yaw: 0.0, pitch: 0.45, distance_scale: 0.6, auto_orbit: false })
    }

    fn update(&mut self, frame: &DemoFrame<'_>, settings: &Settings) -> Result<(), String> {
        self.settings = settings.clone();
        self.time = frame.time;
        let t = frame.time;
        // The sea: rebuilt when its settings change (the same arguments give the same sea).
        let key = (settings.get("wind"), settings.get("direction"), settings.get("waves"), settings.get("seed"));
        if key != self.sea_key {
            self.sea_key = key;
            let sea = OpenWaterWaves::from_wind(key.1.to_radians(), key.0, key.2 as u32, key.3 as u32);
            self.lake.open_waves = Some(sea.clone());
            self.wake.open_waves = Some(sea);
        }
        let mut look = WaterLook::default();
        match settings.get("water") as u32 {
            0 => look.absorption = Vec3::new(0.25, 0.06, 0.04),
            2 => {
                look.absorption = Vec3::new(0.6, 0.2, 0.12);
                look.turbidity = 0.4;
            }
            _ => {}
        }
        look.screen_reflections = settings.on("ssr");
        look.reflectivity = settings.get("reflectivity");
        look.caustics = settings.on("caustics").then(WaterCaustics::default);
        self.lake.look = look;
        self.wake.look = look;
        // The barrel: at the surface's height, tilted to its normal, every frame.
        let (bx, bz) = BARREL_AT;
        self.barrel_y = self.lake.height_at(bx, bz, t);
        let normal = self.lake.normal_at(bx, bz, t);
        self.barrel_tilt = normal.angle_between(Vec3::Y).to_degrees();
        // The barrel's water line at the surface: its origin sits `barrel_sink` below it (in the
        // barrel's own frame, so it tilts about the water line).
        self.instances[self.barrel].world = Mat4::from_scale_rotation_translation(Vec3::splat(BARREL_SCALE), Quat::from_rotation_arc(Vec3::Y, normal), Vec3::new(bx, self.barrel_y, bz)) * Mat4::from_translation(Vec3::new(0.0, -self.barrel_sink / BARREL_SCALE, 0.0));
        // The walker along the bank.
        let (at, yaw) = on_circle(Vec3::new(0.0, 0.0, 44.0), 8.0, 0.25 * t as f32);
        let archer = &mut self.instances[self.archer];
        archer.world = world(2.4, yaw, Vec3::new(at.x, bed(at.x, at.z), at.z));
        archer.animation().update_to(t);
        for instance in self.instances.iter_mut() {
            if instance.has_animation_player() {
                instance.animation().update_to(t);
            }
        }
        // The swimmer: on its circle, riding the waves, its wake pressed into the field.
        let (sx, sz, sr) = SWIM;
        let (at, yaw) = on_circle(Vec3::new(sx, 0.0, sz), sr, 0.35 * t as f32);
        let swim_y = self.lake.height_at(at.x, at.z, t);
        let wake_on = self.wake_on();
        let swimmer = &mut self.instances[self.swimmer];
        swimmer.visible = wake_on;
        swimmer.world = world(SWIMMER_SCALE, yaw, Vec3::new(at.x, swim_y - self.swimmer_sink, at.z));
        if let Some(field) = self.field.as_mut() {
            let wake = wake_on.then(|| WaveWake { position: self.wake.to_field(at), radius: 0.35, strength: 0.06, bob: 0.01, ..WaveWake::default() });
            field.set_wake(0, wake).map_err(|e| e.to_string())?;
        }
        // The light: the sun ahead and to the left (its glints off to the left of the middle), the
        // sky, the lamps.
        let elevation = settings.get("sun").to_radians();
        let direction = Vec3::new(0.75 * elevation.cos(), -elevation.sin(), 0.66 * elevation.cos()).normalize();
        let mut lighting = SceneLighting { environment: Some(frame.environment.clone()), environment_intensity: 0.8, light_intensity: 2.6, light_direction: direction, light_color: Color4::new(1.0, 0.9, 0.78, 1.0), ..SceneLighting::default() };
        if settings.on("shadows") {
            lighting.sun_shadows = SunShadowSettings::covering(Vec3::new(-45.0, -5.0, -40.0), Vec3::new(45.0, 14.0, 48.0));
        }
        if settings.on("lamps") {
            for lamp in &self.lamps {
                lighting.local_lights.push(PointLight::new(*lamp, Color4::new(1.0, 0.72, 0.4, 1.0), 9.0, 16.0).into());
            }
        }
        self.lighting = lighting;
        // Probes for the checks: a shallow point (0.4 deep, by the first island) and a deep one,
        // on the canvas.
        self.probes.clear();
        let (shallow_x, shallow_z) = crossing(-22.0, -6.0, 10.0, -6.0, -0.4);
        for (key, label, x, z) in [("probe-shallow", "Shallow probe (bed depth; canvas x,y)", shallow_x + 0.6, shallow_z), ("probe-deep", "Deep probe (bed depth; canvas x,y)", 4.0, -14.0), ("probe-barrel", "Barrel (canvas x,y of its water line)", bx, bz)] {
            let at = frame.camera.world_to_canvas(Vec3::new(x, if key == "probe-barrel" { self.barrel_y } else { 0.0 }, z), frame.css.0, frame.css.1);
            self.probes.push(Readout::new(key, label, format!("{:.2};{:.1},{:.1}", -bed(x, z), at.x, at.y)));
        }
        Ok(())
    }

    fn before_scene(&mut self, frame: &mut GraphicsFrame<'_>, delta: f64, _settings: &Settings) -> Result<(), GraphicsException> {
        match self.field.as_mut() {
            Some(field) => field.update(frame, delta),
            None => Ok(()),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, context: &mut DrawContext<'_>) -> Result<(), GraphicsException> {
        let (Some(meshes), Some(models)) = (self.meshes.as_mut(), self.models.as_mut()) else { return Ok(()) };
        meshes.begin(pass, context.camera, Some(&self.lighting))?;
        if let (Some(bed), Some(material)) = (self.bed.as_ref(), self.bed_material.as_ref()) {
            meshes.draw(bed, material, Mat4::IDENTITY)?;
        }
        meshes.end(pass)?;
        models.begin(pass, context.camera, Some(&self.lighting))?;
        for instance in &self.instances {
            models.draw(instance)?;
        }
        models.end_opaque(pass)?;
        context.skybox.draw(pass, context.camera, Some(&self.lighting))
    }

    fn has_water_pass(&self) -> bool {
        true
    }

    fn water_pass(&mut self, frame: &mut GraphicsFrame<'_>, hdr: &HeadlessRenderTarget, options: &RenderPassOptions, context: &mut DrawContext<'_>) -> Result<(), GraphicsException> {
        let (Some(water), Some(models)) = (self.water.as_mut(), self.models.as_mut()) else { return Ok(()) };
        let sun = models.sun_shadow_map();
        water.capture_scene(frame, hdr)?;
        let mut pass = frame.begin_render_pass_with_options(Some(&hdr.view()), None, None, options)?;
        let water_frame = WaterFrame { camera: context.camera, lighting: Some(&self.lighting), time: context.time, sun_shadow: sun.as_ref() };
        water.draw(&mut pass, &water_frame, &self.lake)?;
        if self.settings.get("wake") >= 0.5 {
            water.draw(&mut pass, &water_frame, &self.wake)?;
        }
        models.end_blended(&mut pass)
    }

    fn after_scene(&mut self, frame: &mut GraphicsFrame<'_>, scene: &GpuTexture, settings: &Settings) -> Result<(), GraphicsException> {
        match self.bloom.as_mut() {
            Some(bloom) if settings.on("bloom") => bloom.apply(frame, scene),
            _ => Ok(()),
        }
    }

    fn sample_count(&self, settings: &Settings) -> u32 {
        if settings.get("aa") >= 2.0 { 4 } else { 1 }
    }

    fn input(&mut self, input: &CanvasInput, camera: &Camera, css: (f32, f32)) -> bool {
        if !self.wake_on() {
            return false;
        }
        let (Some(field), surface) = (self.field.as_mut(), &self.wake) else { return false };
        let to_field = |x: f64, y: f64| surface.field_point(&camera.canvas_to_ray(x as f32, y as f32, css.0, css.1));
        let used = match input {
            CanvasInput::PointerDown { x, y, button: PointerButton::Primary, .. } => {
                self.stirring = to_field(*x, *y).is_some();
                self.stirring
            }
            CanvasInput::PointerMove { .. } => self.stirring,
            CanvasInput::PointerUp { .. } => std::mem::take(&mut self.stirring),
            _ => false,
        };
        if used {
            field.stir(input, &WaveStir::default(), &to_field);
        }
        used
    }

    fn stats(&self) -> ModelRenderStats {
        self.models.as_ref().map(|r| r.frame_stats()).unwrap_or_default()
    }

    fn readouts(&self, out: &mut Vec<Readout>) {
        let (bx, bz) = BARREL_AT;
        let surface = self.lake.height_at(bx, bz, self.time);
        out.push(Readout::new("barrel", "Barrel: water line y, surface height there, tilt", format!("{:.4}, {:.4}, {:.1} deg", self.barrel_y, surface, self.barrel_tilt)));
        if let Some(sea) = self.lake.open_waves.as_ref() {
            out.push(Readout::new("sea", "Sea: waves, highest crest above still water (sum of amplitudes)", format!("{}, {:.3} m", sea.waves().len(), sea.max_height())));
        }
        if let Some(water) = self.water.as_ref() {
            let stats = water.last_stats();
            out.push(Readout::new("water-stats", "Water (captures, surfaces, vertices, capture size)", format!("{}, {}, {}, {} x {}", stats.captures, stats.surfaces, stats.vertices, stats.capture_size.0, stats.capture_size.1)));
            out.push(Readout::new("water-lights", "Lamps on the water (local lights)", stats.local_lights));
        }
        let stats = self.stats();
        out.push(Readout::new("model-lights", "Lamps on the models (frame_stats().local_lights)", stats.local_lights));
        out.push(Readout::new("shadowed-spots", "Shadowed spotlights (last_shadowed_spotlight_count)", self.models.as_ref().map_or(0, |m| m.last_shadowed_spotlight_count())));
        out.push(Readout::new("sun-shadow", "Sun shadow map this frame", if self.models.as_ref().is_some_and(|m| m.sun_shadow_map().is_some()) { "yes" } else { "no" }));
        if let Some(field) = self.field.as_ref() {
            out.push(Readout::new("wake-steps", "Wake field: steps this frame, steps taken", format!("{}, {}", field.last_step_count(), field.steps_taken())));
        }
        out.push(Readout::new("waterline", "Shore: where the bank meets the still water (z)", format!("{:.1}", self.waterline)));
        out.extend(self.probes.iter().cloned());
    }
}
