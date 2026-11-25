//! "Water pond": a pool of simulated water (the graphics crate's `WaveField`, drawn by its
//! `WaterRenderer` as a `WaterSurface`) over the pool floor of the repository owner's palm
//! visualizer sample - the pastel glass MOSAIC of an eight-petalled ROSETTE (pool_mosaic.wgsl:
//! his `hash2`, `petal`, `rosette` and `tesserae`, combined with the grout as his pool's shader
//! does), drawn ONCE into a mipmapped sRGB texture and laid, unlit, on the pool's floor.
//!
//! The water is stirred by the POINTER (a left drag that starts on the water: `WaveField::stir`
//! through `WaterSurface::field_point` and the camera's ray) and by seeded RAIN; its walls reflect
//! the rings (the field's Reflect edge) and an island rock stands in it (a mask disc). The water
//! surface refracts the mosaic, lays the caustic net on it, tints it by the depth of a floor that
//! slopes from shallow to deep, reflects the sky (and, on the screen, the trees on the deck), shows
//! the sun's glints and the thin-film SHEEN (on in this scene, strength 0.4). THE DEFAULTS of the
//! controls are the palm pool's own look, derived value by value from his shader (THE PALM POOL'S
//! LOOK below): the camera straight down, an even floor, his refraction, rain, wave speed and
//! damping, absorption, caustic net, glints, reflection, sheen and swell, bloom off. The
//! frame order is the guide's: the floor, walls, deck and models with the sky (pass A), the
//! sun's shadow map, the scene capture, then the water and the blended parts (pass B), bloom,
//! tone mapping.
//!
//! "Water effects: off" turns every look effect off (no refraction, absorption, caustics,
//! reflection, glints, sheen, ripples or shore fade): the water then shows the floor exactly as it
//! is, which is how the mosaic is checked against a CPU rendering of the same rosette functions at
//! the probe points the read-outs give (canvas position, the palm colour, the colour expected on
//! screen after tone mapping). The GPU-vs-CPU panel under the scene steps a small probe field and
//! its CPU twin (`CpuWaveField`) with the same calls for 200 steps and reads the difference.

use codebrix_webgpu::glam::{Mat4, Vec2, Vec3};
use codebrix_webgpu::{wgpu, CanvasInput, GpuContext, PointerButton};
use codebrix_webgpu_graphics::{
    BloomRenderer, Camera, Color4, ColorTargetOptions, CpuWaveField, DrawOptions, GpuSampler, GpuTexture, GraphicsContext, GraphicsException, GraphicsFrame, HeadlessRenderTarget, LitMaterial, MaterialOptions, Mesh, MeshRenderer, MeshShapes, MeshVertex, ModelInstance, ModelRenderStats, ModelRenderer, RenderPass, RenderPassOptions, RenderTargetView, RgbaTextureOptions, SamplerDescription,
    SceneLighting, SunShadowSettings, Texture2DOptions, ToneMapOperator, ToneMapper, WaterCaustics, WaterDetail, WaterField, WaterFrame, WaterLook, WaterRenderer, WaterSheen, WaterSurface, WaveDrop, WaveEdge, WaveField, WaveFieldOptions, WaveMask, WaveRain, WaveStir, WaveWake,
};
use dioxus::prelude::*;

use crate::demos::kit::Kit;
use crate::demos::{placed, Control, Demo, DemoFrame, DrawContext, Framing, Readout, Settings};

/// The pool's inside, scene units (x across, z down the top-down view): the palm pool's shape,
/// 1.75 wide for 1 high, 8 units high.
pub const POOL_SIZE: Vec2 = Vec2::new(14.0, 8.0);
/// The floor texture's size in texels (pool_mosaic.wgsl's RESOLUTION).
pub const FLOOR_TEXTURE: (u32, u32) = (2800, 1600);
/// The pool's coping and deck: their top, above the water level (y = 0).
const DECK_TOP: f32 = 0.12;
/// The wave field's grid (square cells of 1/32 unit, the palm pool's 448 x 256).
const FIELD_GRID: (u32, u32) = (448, 256);
/// The field's steps per clock second (wave speeds up to 3.75 units per second hold).
const STEPS_PER_SECOND: f32 = 240.0;
/// Where the island rock stands (x, z) and the radius of the water it takes.
const ISLAND: (f32, f32, f32) = (5.6, -2.4, 0.55);
/// The palm pool's sun, as the direction its light travels in this scene (its `toSun`
/// (-0.35, -0.55, 0.76) in the view's x right, y down, z up is (-0.35, 0.76, -0.55) here).
const SUN_DIRECTION: Vec3 = Vec3::new(0.35, -0.76, 0.55);

// THE PALM POOL'S LOOK (the defaults of this scene; the owner called his original pool
// beautiful). Each value is derived from his RippleWebGpuRenderer.cs / RippleRain.cs (palm
// visualizer sample), whose view is 1 view height = 8 units of this pool (14 x 8, his 1.75 : 1):
// - refraction (control, 0.95): he moves the floor by `0.07 x slope` view heights = 0.56 units per
//   unit of slope, with no depth; the library moves it by `refraction x path x slope`; over the
//   even 0.6-deep floor (control default) that is 0.56 / 0.6 = 0.93 -> 0.95.
// - rain: 1.6 drops / s (his calm rate), radius 0.008-0.016 view heights = 0.064-0.128 units; his
//   drop presses `depth 0.18-0.38` into BOTH steps of its frame, and his surface is `0.010 x` the
//   simulated height in view heights, so the dent is 2 x 0.18-0.38 x 0.010 x 8 = 0.029-0.061 units;
//   with the wave height (the field's height scale, control) at 2 that is a drop depth of
//   0.0144-0.0304.
// - wave speed 1.9 (control): his weights 0.25 / 0.25 move a wave 0.5 cell (1/256 view height) per
//   step at 120 steps / s = 1.875 units / s; damping 0.66 (control): 0.9965 per step, 120 steps /
//   s = 0.656 kept per second.
// - water (control "as the palm pool"): his floor is multiplied by (0.74, 0.81, 0.90) and mixed
//   92 : 8 with (0.08, 0.26, 0.46); at the floor's 0.6 depth that is absorption -ln(0.92 x
//   (0.74, 0.81, 0.90)) / 0.6 = (0.64, 0.49, 0.315) per unit, and turbidity 0.14 / scattering
//   colour (0.032, 0.104, 0.184) give his 8 % scatter under this scene's sun (key light 3 x its
//   elevation + the sky, about 2.5).
// - caustics: his net (`pow(1 - |sin x sin y|, 7)` of `p x 6` view heights) has a cell about 4.2
//   units across -> scale 0.25 cells / unit (control); he adds `0.18 x light` to colours about 0.6
//   (+30 %), the library multiplies by `1 + strength x net x exp(-depth / 4) x facing (0.76)` ->
//   strength 0.45 (control); his warp turns at 0.45-0.55 rad / s -> speed 0.08 cycles / s.
// - glints: his `pow(R.V, 140) x 1.4` Phong lobe is a Blinn lobe of exponent 560 = GGX alpha
//   sqrt(2 / 562) = 0.06 -> roughness sqrt(0.06) = 0.24 (control); its peak (F 0.02 x D x pi / 4
//   x key light 3 = 4.2 x sun_glint) equals his 1.4 at sun_glint 0.35.
// - reflection: his Fresnel is 0.05 straight down (+ 1.6 x steepness^2, at most 0.45); the
//   library's Schlick is 0.02 x reflectivity -> reflection strength 2.5 (control).
// - sheen: his `(sheen - 0.45) x (0.10 + 0.9 steep) x 0.5` is the library's formula at strength 1
//   times the light (about 2.5 here) -> strength 0.4 (control); his `0.6 x` simulated height is 7.5
//   per unit of surface height; his band `sin(p . (2.2, 1.7))` with p in view heights is scale
//   1 / 8 = 0.125.
// - fine ripples: his swell adds four travelling slopes of 0.25 x 0.045 each (0.045 in all) of
//   wavelengths 2 pi / (23, 31, 41, 57) view heights = 2.19, 1.62, 1.23, 0.88 units (the library's
//   detail has the ratios 1, 0.71, 0.53, 0.37 of its wavelength) at about 0.4 units / s.
// - camera straight down and bloom off (controls), as his view (no bloom; glints clip at 1).
/// "as the palm pool": absorption per unit.
const PALM_ABSORPTION: Vec3 = Vec3::new(0.64, 0.49, 0.315);
/// "as the palm pool": turbidity.
const PALM_TURBIDITY: f32 = 0.14;
/// "as the palm pool": scattering colour.
const PALM_SCATTERING: Vec3 = Vec3::new(0.032, 0.104, 0.184);
/// His caustic net's drift (cycles per second).
const PALM_CAUSTICS_SPEED: f32 = 0.08;
/// His glints' strength (with roughness 0.24).
const PALM_SUN_GLINT: f32 = 0.35;
/// His sheen's band scale and height gain.
const PALM_SHEEN_SCALE: f32 = 0.125;
/// His sheen's film change per unit of surface height.
const PALM_SHEEN_HEIGHT_GAIN: f32 = 7.5;
/// His swell, as the fine ripples ("Fine ripples" on).
const PALM_DETAIL: WaterDetail = WaterDetail { strength: 0.045, wavelength: 2.19, speed: 0.4, seed: 1 };
/// His rain's drop radius (units).
const PALM_RAIN_RADIUS: (f32, f32) = (0.064, 0.128);
/// His rain's dent (units of the field; x the wave height 2).
const PALM_RAIN_DEPTH: (f32, f32) = (0.0144, 0.0304);

/// The pointer's stirring: twice the library's default drop and wake (the palm pool's hands
/// rocked its water hard).
const STIR: WaveStir = WaveStir { drop: WaveDrop { position: Vec2::ZERO, radius: 0.14, depth: 0.08 }, wake: WaveWake { position: Vec2::ZERO, radius: 0.15, strength: 0.16, bob: 0.0, bob_rate: 1.2 } };

/// The controls.
pub const CONTROLS: &[Control] = &[
    Control::Choice { key: "view", label: "Camera", options: &[("above the pool, tilted (orbit freely)", 0.0), ("straight down, as the palm pool", 1.0)], default: 1.0 },
    Control::Choice { key: "drag", label: "Left drag", options: &[("stirs the water (when it starts on the water)", 0.0), ("orbits the camera", 1.0)], default: 0.0 },
    Control::Slider { key: "rain", label: "Rain (drops / s)", min: 0.0, max: 20.0, step: 0.2, default: 1.6 },
    Control::Slider { key: "wave-speed", label: "Wave speed", min: 0.3, max: 3.5, step: 0.1, default: 1.9 },
    Control::Slider { key: "damping", label: "Damping (height kept per second)", min: 0.2, max: 1.0, step: 0.02, default: 0.66 },
    Control::Slider { key: "wave-height", label: "Wave height (the field's height scale)", min: 0.5, max: 4.0, step: 0.1, default: 2.0 },
    Control::Slider { key: "refraction", label: "Refraction", min: 0.0, max: 1.0, step: 0.05, default: 0.95 },
    Control::Choice { key: "floor", label: "Floor", options: &[("sloping 0.3 to 1.2 deep", 0.0), ("even, 0.6 deep", 1.0), ("sloping 0.3 to 2 deep", 2.0)], default: 1.0 },
    Control::Choice { key: "water", label: "Water (absorption, depth tint)", options: &[("clear pool", 0.0), ("library default", 1.0), ("murky", 2.0), ("as the palm pool", 3.0)], default: 3.0 },
    Control::Slider { key: "caustics", label: "Caustics strength", min: 0.0, max: 1.5, step: 0.05, default: 0.45 },
    Control::Slider { key: "caustics-scale", label: "Caustics scale", min: 0.2, max: 4.0, step: 0.05, default: 0.25 },
    Control::Slider { key: "roughness", label: "Glint sharpness (roughness, lower = sharper)", min: 0.02, max: 0.3, step: 0.01, default: 0.24 },
    Control::Slider { key: "reflectivity", label: "Reflection strength", min: 0.0, max: 3.0, step: 0.05, default: 2.5 },
    Control::Slider { key: "sheen", label: "Sheen strength", min: 0.0, max: 1.0, step: 0.05, default: 0.4 },
    Control::Toggle { key: "ssr", label: "Screen-space reflections", default: true },
    Control::Toggle { key: "detail", label: "Fine ripples", default: true },
    Control::Toggle { key: "effects", label: "Water effects (off: the floor as it is, to check the mosaic)", default: true },
    Control::Toggle { key: "island", label: "Island rock", default: true },
    Control::Toggle { key: "bloom", label: "Bloom", default: false },
    Control::Choice { key: "aa", label: "Anti-aliasing", options: &[("Off", 1.0), ("4x", 4.0)], default: 4.0 },
    Control::Button { key: "calm", label: "Calm the water" },
];

/// What to look at.
pub const LEGEND: &[&str] = &[
    "The owner's mosaic rosette (eight pastel glass petals, gold outlines and ring, cream heart, sea glass) seen through the water: crisp in the shallows, deeper in colour toward the deep end.",
    "Drag across the water: rings spread at a steady speed, bounce off the walls and the island, and fade; the tiles wobble under them and not where the water is still. Light rain keeps it alive.",
    "A caustic net drifts on the floor and gathers under the crests; the sky (and, tilted, the trees) show in the water; sun glints sparkle (bloom is a switch, off as in the palm pool); the thin-film sheen shimmers on the slopes.",
    "The defaults are the palm pool's look, worked out from its shader: straight down, refraction 0.95 over an even floor, its rain, waves, colour, caustics, glints, reflection and sheen.",
    "Pause freezes everything exactly. \"Calm the water\" flattens the field. 4x and Off look the same apart from smoother edges.",
];

/// The probe points of the mosaic check: the read-out's key and the point relative to the
/// rosette's centre, in the palm pool's units (the floor's height is 1): the heart, the middle of
/// one petal of each colour, the sea glass outside the ring, and the gold ring at eight angles (a
/// grout line can run along the ring at any one of them).
pub const PROBES: [(&str, Vec2); 14] = [
    ("probe-heart", Vec2::new(0.0, 0.0)),
    ("probe-petal-pink", Vec2::new(0.23097, 0.09567)),
    ("probe-petal-orange", Vec2::new(0.09567, 0.23097)),
    ("probe-petal-violet", Vec2::new(-0.09567, 0.23097)),
    ("probe-petal-green", Vec2::new(-0.23097, 0.09567)),
    ("probe-sea-glass", Vec2::new(-0.70, 0.0)),
    ("probe-gold-ring-0", Vec2::new(0.47, 0.0)),
    ("probe-gold-ring-1", Vec2::new(0.33234, 0.33234)),
    ("probe-gold-ring-2", Vec2::new(0.0, 0.47)),
    ("probe-gold-ring-3", Vec2::new(-0.33234, 0.33234)),
    ("probe-gold-ring-4", Vec2::new(-0.47, 0.0)),
    ("probe-gold-ring-5", Vec2::new(-0.33234, -0.33234)),
    ("probe-gold-ring-6", Vec2::new(0.0, -0.47)),
    ("probe-gold-ring-7", Vec2::new(0.33234, -0.33234)),
];

/// The water pond demo.
#[derive(Default)]
pub struct WaterPondDemo {
    graphics: Option<GraphicsContext>,
    models: Option<ModelRenderer>,
    meshes: Option<MeshRenderer>,
    water: Option<WaterRenderer>,
    bloom: Option<BloomRenderer>,
    field: Option<WaveField>,
    floor_texture: Option<GpuTexture>,
    floor_sampler: Option<GpuSampler>,
    floor_material: Option<LitMaterial>,
    wall_material: Option<LitMaterial>,
    deck_material: Option<LitMaterial>,
    floor: Option<Mesh>,
    walls: Option<Mesh>,
    deck: Vec<(Mesh, Mat4)>,
    instances: Vec<ModelInstance>,
    island: Option<ModelInstance>,
    surface: WaterSurface,
    lighting: SceneLighting,
    settings: Settings,
    floor_choice: f32,
    island_on: Option<bool>,
    island_y: f32,
    calm_presses: f32,
    calms: u32,
    stirring: bool,
    speed_message: String,
    probes: Vec<Readout>,
    time: f64,
}

/// The floor's depth (below the water level) at x for a floor choice.
fn depth_at(choice: f32, x: f32) -> f32 {
    let t = ((x + POOL_SIZE.x * 0.5) / POOL_SIZE.x).clamp(0.0, 1.0);
    match choice as u32 {
        1 => 0.6,
        2 => 0.3 + 1.7 * t,
        _ => 0.3 + 0.9 * t,
    }
}

/// A point of the floor in the palm pool's units (0..1.75 across, 0..1 down) in the world.
fn floor_point(choice: f32, p: Vec2) -> Vec3 {
    let x = p.x / (POOL_SIZE.x / POOL_SIZE.y) * POOL_SIZE.x - POOL_SIZE.x * 0.5;
    let z = p.y * POOL_SIZE.y - POOL_SIZE.y * 0.5;
    Vec3::new(x, -depth_at(choice, x), z)
}

/// The pool floor (one sloping quad, its texture over the whole of it) and the four inner walls
/// (from the floor up to the coping, their tile texture in half-unit tiles).
fn pool_meshes(graphics: &GraphicsContext, choice: f32) -> Result<(Mesh, Mesh), GraphicsException> {
    let (hx, hz) = (POOL_SIZE.x * 0.5, POOL_SIZE.y * 0.5);
    let (left, right) = (-depth_at(choice, -hx), -depth_at(choice, hx));
    let slope = (left - right) / POOL_SIZE.x;
    let up = Vec3::new(slope, 1.0, 0.0).normalize();
    let floor = [
        MeshVertex::white(Vec3::new(-hx, left, -hz), up, Vec2::new(0.0, 0.0)),
        MeshVertex::white(Vec3::new(hx, right, -hz), up, Vec2::new(1.0, 0.0)),
        MeshVertex::white(Vec3::new(hx, right, hz), up, Vec2::new(1.0, 1.0)),
        MeshVertex::white(Vec3::new(-hx, left, hz), up, Vec2::new(0.0, 1.0)),
    ];
    let floor = graphics.create_mesh_from(&floor, &[0, 2, 1, 0, 3, 2], Some("pool floor"))?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut wall = |a: Vec3, b: Vec3, normal: Vec3| {
        // From a to b along the floor, up to the coping; tiles of half a unit.
        let base = vertices.len() as u32;
        let length = (b - a).length();
        let corners = [(a, a.y), (b, b.y), (Vec3::new(b.x, DECK_TOP, b.z), DECK_TOP), (Vec3::new(a.x, DECK_TOP, a.z), DECK_TOP)];
        for (i, (point, y)) in corners.into_iter().enumerate() {
            let u = if i == 0 || i == 3 { 0.0 } else { length / 0.5 };
            vertices.push(MeshVertex::white(point, normal, Vec2::new(u, -y / 0.5)));
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };
    wall(Vec3::new(hx, right, -hz), Vec3::new(-hx, left, -hz), Vec3::Z);
    wall(Vec3::new(-hx, left, hz), Vec3::new(hx, right, hz), Vec3::NEG_Z);
    wall(Vec3::new(-hx, left, -hz), Vec3::new(-hx, left, hz), Vec3::X);
    wall(Vec3::new(hx, right, hz), Vec3::new(hx, right, -hz), Vec3::NEG_X);
    let walls = graphics.create_mesh_from(&vertices, &indices, Some("pool walls"))?;
    Ok((floor, walls))
}

/// Draws the mosaic (pool_mosaic.wgsl) once into a mipmapped sRGB texture.
fn mosaic_texture(graphics: &GraphicsContext) -> Result<GpuTexture, GraphicsException> {
    let (width, height) = FLOOR_TEXTURE;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
    let texture = graphics.create_texture_2d(width, height, format, usage, Texture2DOptions { mip_level_count: GpuTexture::full_mip_chain_length(width, height), label: Some("pool mosaic".to_string()) })?;
    let shader = graphics.create_shader(include_str!("pool_mosaic.wgsl"), Some("pool mosaic"));
    let pipeline = graphics.create_render_pipeline(&shader).with_color_target(format, ColorTargetOptions::default()).with_label("pool mosaic").build()?;
    let level = texture.handle().create_view(&wgpu::TextureViewDescriptor { base_mip_level: 0, mip_level_count: Some(1), ..wgpu::TextureViewDescriptor::default() });
    let target = RenderTargetView::new(level, format, width, height, None);
    let mut frame = graphics.begin_frame();
    {
        let mut pass = frame.begin_render_pass_to(&target, Some(Color4::BLACK), None)?;
        pass.set_pipeline(&pipeline);
        pass.draw(3, DrawOptions::default());
    }
    frame.submit()?;
    graphics.generate_mipmaps(&texture)?;
    Ok(texture)
}

/// A small tile texture made in code for the pool's walls: pale aqua tiles with light grout.
fn tile_pixels(size: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let grout = x < 2 || y < 2;
            let shade = ((x * 7 + y * 3) % 11) as u8;
            let pixel = if grout { [214, 218, 214] } else { [150 + shade, 206 + shade / 2, 212] };
            pixels.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
        }
    }
    pixels
}

/// The rosette's colour at a point relative to its centre, on the CPU: the palm pool's `rosette`
/// function, line for line (for the mosaic check's expected colours).
pub fn rosette_cpu(q: Vec2) -> Vec3 {
    let palette = [Vec3::new(0.92, 0.50, 0.64), Vec3::new(0.98, 0.64, 0.36), Vec3::new(0.60, 0.48, 0.90), Vec3::new(0.40, 0.80, 0.66)];
    let gold = Vec3::new(0.80, 0.58, 0.18);
    let r = q.length();
    let mut col = Vec3::new(0.45, 0.74, 0.84).lerp(Vec3::new(0.62, 0.58, 0.88), 0.5 + 0.5 * (q.x * 4.0 + q.y * 3.0).sin());
    if r < 0.47 {
        col = col.lerp(Vec3::new(0.84, 0.88, 0.95), 0.15);
    }
    let petal = |q: Vec2, a: f32| -> f32 {
        let along = Vec2::new(a.cos(), a.sin());
        let p = Vec2::new(q.dot(Vec2::new(-along.y, along.x)), q.dot(along) - 0.25).abs();
        let (r, d, b) = (0.2305f32, 0.1305f32, 0.19f32);
        if (p.y - b) * d > p.x * b {
            return (p - Vec2::new(0.0, b)).length();
        }
        (p - Vec2::new(-d, 0.0)).length() - r
    };
    let mut fill = Vec3::ZERO;
    let mut count = 0.0;
    let mut edge = 1000.0f32;
    for k in 0..8 {
        let a = (k as f32 + 0.5) * std::f32::consts::PI / 4.0;
        let dk = petal(q, a);
        edge = edge.min(dk.abs());
        if dk < 0.0 {
            let tip = ((q.dot(Vec2::new(a.cos(), a.sin())) - 0.06) / 0.38).clamp(0.0, 1.0);
            let base = palette[k % 4];
            fill += (base * 0.92).lerp(base.lerp(Vec3::ONE, 0.15), tip);
            count += 1.0;
        }
    }
    if count > 0.0 {
        col = (fill / count).lerp(Vec3::ONE, 0.06 * (count - 1.0));
    }
    if edge < 0.0075 || (r - 0.47).abs() < 0.00975 {
        col = gold;
    }
    let t = ((r - 0.045) / (0.072 - 0.045)).clamp(0.0, 1.0);
    let smooth = t * t * (3.0 - 2.0 * t);
    col.lerp(Vec3::new(0.98, 0.88, 0.66), 1.0 - smooth)
}

fn decode(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn encode(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}

/// A colour 0..1 as "r,g,b" bytes.
fn bytes(c: Vec3) -> String {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("{},{},{}", b(c.x), b(c.y), b(c.z))
}

impl WaterPondDemo {
    fn look(&self) -> (WaterLook, WaterDetail) {
        let s = &self.settings;
        let mut look = WaterLook::default();
        match s.get("water") as u32 {
            0 => look.absorption = Vec3::new(0.25, 0.06, 0.04),
            2 => {
                look.absorption = Vec3::new(0.6, 0.2, 0.12);
                look.turbidity = 0.4;
            }
            3 => {
                look.absorption = PALM_ABSORPTION;
                look.turbidity = PALM_TURBIDITY;
                look.scattering_color = PALM_SCATTERING;
            }
            _ => {}
        }
        look.caustics = (s.get("caustics") > 0.0).then(|| WaterCaustics { strength: s.get("caustics"), scale: s.get("caustics-scale"), speed: PALM_CAUSTICS_SPEED, ..WaterCaustics::default() });
        look.roughness = s.get("roughness");
        look.refraction = s.get("refraction");
        look.reflectivity = s.get("reflectivity");
        look.sun_glint = PALM_SUN_GLINT;
        look.screen_reflections = s.on("ssr");
        look.sheen = WaterSheen { strength: s.get("sheen"), scale: PALM_SHEEN_SCALE, height_gain: PALM_SHEEN_HEIGHT_GAIN, ..WaterSheen::default() };
        let mut detail = PALM_DETAIL;
        if !s.on("detail") {
            detail.strength = 0.0;
        }
        if !s.on("effects") {
            look = WaterLook { absorption: Vec3::ZERO, turbidity: 0.0, refraction: 0.0, reflectivity: 0.0, sun_glint: 0.0, caustics: None, screen_reflections: false, local_light_highlights: false, soft_shore: 0.0, sheen: WaterSheen { strength: 0.0, ..WaterSheen::default() }, ..look };
            detail.strength = 0.0;
        }
        (look, detail)
    }

    fn rebuild_floor(&mut self, choice: f32) -> Result<(), String> {
        let graphics = self.graphics.clone().ok_or("no context")?;
        let (floor, walls) = pool_meshes(&graphics, choice).map_err(|e| e.to_string())?;
        self.floor = Some(floor);
        self.walls = Some(walls);
        self.floor_choice = choice;
        Ok(())
    }

    fn set_island(&mut self, on: bool) -> Result<(), String> {
        let Some(field) = self.field.as_mut() else { return Ok(()) };
        if on {
            let (cx, cz, radius) = ISLAND;
            let mask = WaveMask::from_fn(FIELD_GRID.0, FIELD_GRID.1, |u, v| {
                let (x, z) = (u * POOL_SIZE.x - POOL_SIZE.x * 0.5, v * POOL_SIZE.y - POOL_SIZE.y * 0.5);
                let d = ((x - cx).powi(2) + (z - cz).powi(2)).sqrt();
                // Solid inside the rock; a ramp of a few cells makes its edge a beach.
                ((radius + 0.08 - d) / 0.16).clamp(0.0, 1.0)
            });
            field.set_mask(Some(&mask)).map_err(|e| e.to_string())?;
        } else {
            field.set_mask(None).map_err(|e| e.to_string())?;
        }
        self.island_on = Some(on);
        Ok(())
    }
}

impl Demo for WaterPondDemo {
    fn build(&mut self, kit: &mut Kit<'_>) -> Result<Framing, String> {
        let graphics = kit.graphics.clone();
        self.graphics = Some(graphics.clone());
        let e = |e: GraphicsException| e.to_string();
        // THE FLOOR: the owner's mosaic, drawn once, unlit (the pool shows its glass colours).
        let texture = mosaic_texture(&graphics).map_err(e)?;
        let sampler = graphics.create_sampler(&SamplerDescription { mipmap_filter: wgpu::MipmapFilterMode::Linear, max_anisotropy: 8, ..SamplerDescription::linear_clamp() });
        let meshes = graphics.create_mesh_renderer().map_err(e)?;
        let floor_material = meshes.create_material(Color4::WHITE, MaterialOptions { base_color_texture: Some(&texture), sampler: Some(&sampler), label: Some("pool floor"), ..MaterialOptions::default() }).map_err(e)?;
        floor_material.set_unlit(true);
        let tiles = graphics.create_texture_from_rgba(32, 32, &tile_pixels(32), RgbaTextureOptions { mipmaps: true, srgb: true, label: Some("pool tiles".to_string()) }).map_err(e)?;
        let wall_material = meshes.create_material(Color4::WHITE, MaterialOptions { base_color_texture: Some(&tiles), label: Some("pool walls"), ..MaterialOptions::default() }).map_err(e)?;
        let deck_material = meshes.create_material(Color4::new(0.17, 0.15, 0.13, 1.0), MaterialOptions { label: Some("deck"), ..MaterialOptions::default() }).map_err(e)?;
        // THE DECK round the pool: four slabs whose top is the coping.
        let (hx, hz) = (POOL_SIZE.x * 0.5, POOL_SIZE.y * 0.5);
        let (ox, oz, thick) = (32.0f32, 26.0f32, 0.6f32);
        let slab = |x0: f32, x1: f32, z0: f32, z1: f32| -> Result<(Mesh, Mat4), String> {
            let size = Vec3::new(x1 - x0, thick, z1 - z0);
            let mesh = graphics.create_mesh(&MeshShapes::box_shape(size, None), Some("deck")).map_err(|e| e.to_string())?;
            Ok((mesh, Mat4::from_translation(Vec3::new((x0 + x1) * 0.5, DECK_TOP - thick * 0.5, (z0 + z1) * 0.5))))
        };
        self.deck = vec![slab(-ox, ox, -oz, -hz)?, slab(-ox, ox, hz, oz)?, slab(-ox, -hx, -hz, hz)?, slab(hx, ox, -hz, hz)?];
        // Trees and rocks on the deck (they cast the sun's shadow on the water and show in it).
        let (tree, tall, autumn, rock_a, rock_b) = (kit.model("tree")?, kit.model("tree-tall")?, kit.model("tree-autumn")?, kit.model("rock-a")?, kit.model("rock-b")?);
        let on_deck = |x: f32, z: f32| Vec3::new(x, DECK_TOP, z);
        self.instances = vec![
            placed(&tall, 2.6, 0.4, on_deck(-8.4, -5.3)),
            placed(&tree, 2.4, 1.3, on_deck(-10.5, -1.0)),
            placed(&autumn, 2.5, 2.1, on_deck(9.2, -6.0)),
            placed(&tree, 2.2, 0.2, on_deck(-11.5, 6.5)),
            placed(&tall, 2.4, 2.8, on_deck(12.5, 5.0)),
            placed(&rock_b, 1.8, 0.7, on_deck(8.4, 4.9)),
            placed(&rock_a, 1.6, 2.2, on_deck(-8.2, 4.6)),
        ];
        // THE ISLAND: a rock standing on the floor and out of the water.
        // As wide as the water it takes; its top 0.3 above the water, its foot hidden under the floor.
        let rock_width = (rock_a.bounds_max().x - rock_a.bounds_min().x).max(0.05);
        let scale = ISLAND.2 * 2.0 / rock_width;
        self.island_y = 0.3 - rock_a.bounds_max().y * scale;
        self.island = Some(placed(&rock_a, scale, 0.9, Vec3::new(ISLAND.0, self.island_y, ISLAND.1)));
        // THE WATER: a field over the whole pool, its picture on the surface.
        let options = WaveFieldOptions { columns: FIELD_GRID.0, rows: FIELD_GRID.1, size: POOL_SIZE, wave_speed: 1.9, damping: 0.66, edge: WaveEdge::Reflect, steps_per_second: STEPS_PER_SECOND, max_steps_per_frame: 8, label: Some("pond".to_string()) };
        let field = graphics.create_wave_field(&options).map_err(e)?;
        self.surface = WaterSurface { center: Vec3::ZERO, size: POOL_SIZE, grid: 192, field: Some(WaterField::of(&field)), ..WaterSurface::default() };
        self.field = Some(field);
        self.water = Some(graphics.create_water_renderer().map_err(e)?);
        self.models = Some(graphics.create_model_renderer().map_err(e)?);
        self.bloom = Some(graphics.create_bloom_renderer().map_err(e)?);
        self.meshes = Some(meshes);
        self.floor_texture = Some(texture);
        self.floor_sampler = Some(sampler);
        self.floor_material = Some(floor_material);
        self.wall_material = Some(wall_material);
        self.deck_material = Some(deck_material);
        self.settings = Settings::from_controls(CONTROLS);
        self.rebuild_floor(0.0)?;
        self.set_island(true)?;
        Ok(Framing { min: Vec3::new(-hx - 0.5, -1.0, -hz - 0.5), max: Vec3::new(hx + 0.5, 1.0, hz + 0.5), yaw: 0.0, pitch: 0.85, distance_scale: 0.82, auto_orbit: false })
    }

    fn update(&mut self, frame: &DemoFrame<'_>, settings: &Settings) -> Result<(), String> {
        self.settings = settings.clone();
        self.time = frame.time;
        if settings.get("floor") != self.floor_choice {
            self.rebuild_floor(settings.get("floor"))?;
        }
        if self.island_on != Some(settings.on("island")) {
            self.set_island(settings.on("island"))?;
        }
        if let Some(field) = self.field.as_mut() {
            let rate = settings.get("rain");
            field.set_rain((rate > 0.0).then_some(WaveRain { rate, radius: PALM_RAIN_RADIUS, depth: PALM_RAIN_DEPTH, area: None, seed: 1 }));
            self.speed_message = match field.set_wave_speed(settings.get("wave-speed")) {
                Ok(()) => "accepted".to_string(),
                Err(error) => error.to_string(),
            };
            field.set_damping(settings.get("damping")).map_err(|e| e.to_string())?;
        }
        let (look, detail) = self.look();
        if let Some(field) = self.surface.field.as_mut() {
            field.height_scale = settings.get("wave-height");
        }
        self.surface.look = look;
        self.surface.detail = detail;
        self.lighting = SceneLighting {
            environment: Some(frame.environment.clone()),
            environment_intensity: 1.0,
            light_intensity: 3.0,
            light_direction: SUN_DIRECTION.normalize(),
            sun_shadows: SunShadowSettings::covering(Vec3::new(-17.0, -2.5, -13.0), Vec3::new(17.0, 7.0, 13.0)),
            ..SceneLighting::default()
        };
        // The mosaic check's probe points: where each lies on the canvas, the palm colour there
        // and the colour expected on screen with the water's effects off (sRGB decoded by the
        // texture, the Khronos PBR Neutral curve at exposure 1, sRGB encoded).
        self.probes.clear();
        for (key, q) in PROBES {
            let p = q + Vec2::new(POOL_SIZE.x / POOL_SIZE.y * 0.5, 0.5);
            let world = floor_point(self.floor_choice, p);
            let at = frame.camera.world_to_canvas(world, frame.css.0, frame.css.1);
            let palm = rosette_cpu(q);
            let linear = Vec3::new(decode(palm.x), decode(palm.y), decode(palm.z));
            let shown = ToneMapper::map(linear, ToneMapOperator::KhronosPbrNeutral, 1.0);
            let shown = Vec3::new(encode(shown.x), encode(shown.y), encode(shown.z));
            self.probes.push(Readout::new(key, "Mosaic probe: canvas x,y (CSS px); palm colour; expected on screen with the effects off", format!("{:.1},{:.1};{};{}", at.x, at.y, bytes(palm), bytes(shown))));
        }
        Ok(())
    }

    fn before_scene(&mut self, frame: &mut GraphicsFrame<'_>, delta: f64, settings: &Settings) -> Result<(), GraphicsException> {
        let Some(field) = self.field.as_mut() else { return Ok(()) };
        let presses = settings.get("calm");
        if presses != self.calm_presses {
            self.calm_presses = presses;
            field.reset(frame)?;
            self.calms += 1;
        }
        field.update(frame, delta)
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, context: &mut DrawContext<'_>) -> Result<(), GraphicsException> {
        let (Some(meshes), Some(models)) = (self.meshes.as_mut(), self.models.as_mut()) else { return Ok(()) };
        // PASS A: the pool (floor, walls, deck), the models, the sky.
        meshes.begin(pass, context.camera, Some(&self.lighting))?;
        if let (Some(floor), Some(material)) = (self.floor.as_ref(), self.floor_material.as_ref()) {
            meshes.draw(floor, material, Mat4::IDENTITY)?;
        }
        if let (Some(walls), Some(material)) = (self.walls.as_ref(), self.wall_material.as_ref()) {
            meshes.draw(walls, material, Mat4::IDENTITY)?;
        }
        if let Some(material) = self.deck_material.as_ref() {
            for (mesh, world) in &self.deck {
                meshes.draw(mesh, material, *world)?;
            }
        }
        meshes.end(pass)?;
        models.begin(pass, context.camera, Some(&self.lighting))?;
        for instance in &self.instances {
            models.draw(instance)?;
        }
        if let (Some(island), Some(true)) = (self.island.as_ref(), self.island_on) {
            models.draw(island)?;
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
        // PASS B: loads colour and depth; the water, then the blended parts of the same pair.
        let mut pass = frame.begin_render_pass_with_options(Some(&hdr.view()), None, None, options)?;
        water.draw(&mut pass, &WaterFrame { camera: context.camera, lighting: Some(&self.lighting), time: context.time, sun_shadow: sun.as_ref() }, &self.surface)?;
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

    fn place_camera(&mut self, _time: f64, settings: &Settings, camera: &mut Camera) {
        if settings.get("view") >= 0.5 {
            // Straight down over the pool's middle, the screen's up toward -Z: the palm pool's
            // view (its picture's top is the floor texture's first row).
            camera.position = Vec3::new(0.0, 9.6, 0.0);
            camera.target = Vec3::ZERO;
            camera.up = Vec3::NEG_Z;
        } else {
            camera.up = Vec3::Y;
        }
    }

    fn input(&mut self, input: &CanvasInput, camera: &Camera, css: (f32, f32)) -> bool {
        if self.settings.get("drag") >= 0.5 {
            return false;
        }
        let (Some(field), surface) = (self.field.as_mut(), &self.surface) else { return false };
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
            field.stir(input, &STIR, &to_field);
        }
        used
    }

    fn stats(&self) -> ModelRenderStats {
        self.models.as_ref().map(|r| r.frame_stats()).unwrap_or_default()
    }

    fn readouts(&self, out: &mut Vec<Readout>) {
        if let Some(field) = self.field.as_ref() {
            out.push(Readout::new("steps", "Field steps this frame (last_step_count)", field.last_step_count()));
            out.push(Readout::new("steps-taken", "Field steps taken", field.steps_taken()));
            out.push(Readout::new("simulated", "Field's simulated time (s)", format!("{:.3}", field.simulated_time())));
            out.push(Readout::new("grid", "Field grid and cell", format!("{} x {}, cell {:.4}", FIELD_GRID.0, FIELD_GRID.1, field.cell_size().x)));
        }
        out.push(Readout::new("speed-check", "Wave speed (set_wave_speed)", &self.speed_message));
        out.push(Readout::new("calms", "Calmed (reset)", self.calms));
        if let Some(water) = self.water.as_ref() {
            let stats = water.last_stats();
            out.push(Readout::new("water-stats", "Water (captures, surfaces, vertices, capture size)", format!("{}, {}, {}, {} x {}", stats.captures, stats.surfaces, stats.vertices, stats.capture_size.0, stats.capture_size.1)));
            out.push(Readout::new("water-pipelines", "Water pipelines", water.pipeline_count()));
        }
        out.push(Readout::new("sheen-now", "Sheen strength", format!("{:.2}", self.surface.look.sheen.strength)));
        out.push(Readout::new("effects-now", "Water effects", if self.settings.on("effects") { "on" } else { "off" }));
        out.push(Readout::new("floor-depth", "Floor depth, shallow to deep end", format!("{:.2} to {:.2}", depth_at(self.floor_choice, -POOL_SIZE.x * 0.5), depth_at(self.floor_choice, POOL_SIZE.x * 0.5))));
        out.push(Readout::new("sun-shadow", "Sun shadow map this frame", if self.models.as_ref().is_some_and(|m| m.sun_shadow_map().is_some()) { "yes" } else { "no" }));
        out.extend(self.probes.iter().cloned());
    }
}

/// The GPU-vs-CPU check of the wave field (under the pond's canvas): a 64 x 64 probe field and
/// its CPU twin driven by the same calls and deltas for 200 steps, then the largest height
/// difference of the GPU picture read back.
#[component]
pub fn ProbePanel(context: GpuContext) -> Element {
    let graphics = use_hook(|| GraphicsContext::wrap_gpu_context(&context));
    let mut result = use_signal(|| "not run yet".to_string());
    let run = move |_| {
        let graphics = graphics.clone();
        result.set("running".to_string());
        spawn(async move {
            let text = match probe(&graphics).await {
                Ok(text) => text,
                Err(error) => format!("failed: {error}"),
            };
            result.set(text);
        });
    };
    rsx! {
        div { class: "flex flex-wrap items-center gap-3 rounded-lg bg-slate-900 p-3 text-sm text-slate-300 ring-1 ring-white/10",
            button { id: "water-pond-probe-run", class: "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700", onclick: run, "Compare the GPU wave field with its CPU twin" }
            p { "A 64 x 64 probe field, 200 steps with a drop and rain: " span { id: "water-pond-probe", class: "font-mono text-white", "{result}" } }
        }
    }
}

async fn probe(graphics: &GraphicsContext) -> Result<String, String> {
    let e = |e: GraphicsException| e.to_string();
    let options = WaveFieldOptions { columns: 64, rows: 64, size: Vec2::new(4.0, 4.0), label: Some("probe".to_string()), ..WaveFieldOptions::default() };
    let mut gpu = graphics.create_wave_field(&options).map_err(e)?;
    let mut cpu = CpuWaveField::new(&options).map_err(e)?;
    let rain = WaveRain { rate: 6.0, ..WaveRain::default() };
    gpu.set_rain(Some(rain));
    cpu.set_rain(Some(rain));
    let drop = WaveDrop::at(Vec2::new(1.3, 2.1));
    gpu.drop(drop);
    cpu.drop(drop);
    for _ in 0..100 {
        let mut frame = graphics.begin_frame();
        gpu.update(&mut frame, 1.0 / 60.0).map_err(e)?;
        frame.submit().map_err(e)?;
        cpu.update(1.0 / 60.0);
    }
    let picture = gpu.surface().read_pixels(0).await.map_err(e)?;
    let difference = cpu.height_difference(&picture).map_err(e)?;
    Ok(format!("{} GPU steps, {} CPU steps, largest height difference {:.2e}", gpu.steps_taken(), cpu.steps_taken(), difference))
}
