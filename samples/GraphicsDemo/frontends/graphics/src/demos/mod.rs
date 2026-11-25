//! The FEATURE DEMOS of the graphics crate's newer capabilities, one scene each - "Lights" (point
//! lights and spotlights), "Shadows" (the sun's and the spotlights'), "Culling" (frustum culling),
//! "Translucency" (two-pass double-sided blended parts, per-instance alpha / tint / emissive
//! strength, node visibility), "Glow and bloom" (the glow layer and the bloom renderer on the HDR
//! path), "Particles" (the GPU particle system), "Picking and labels" (picking with a ray,
//! page-text labels placed with world_to_canvas), "Anti-aliasing" (1 or 4 samples per pixel,
//! switched while the page runs, beside a second canvas left at 1), "Level of detail" (objects
//! that dissolve from one level to the next through a fade band), "Water pond" (a simulated wave
//! field over the owner's mosaic rosette pool floor, stirred by the pointer and by rain, drawn by
//! the water surface) and "Water lake" (open-water waves on a large lake with a sloping bed,
//! islands, a floating barrel and a swimmer's wake) - all on ONE shell (this file):
//!
//! - the models come from the backend as the kits' .glb files (and the ground's blob), fetched
//!   once for the page (kit.rs); every scene builds its own GPU models from those bytes;
//! - every scene draws through the HDR path (an RGBA16Float target, then the tone mapper into the
//!   canvas) under an orbit camera (drag orbits, right / middle / Shift drag pans, the wheel zooms)
//!   on its own canvas clock (pause, half and quarter speed); a demo may ask for 4 samples per
//!   pixel (`Demo::sample_count`: the HDR target's `set_sample_count`), for the DIRECT path
//!   instead (`Demo::direct_to_canvas`: no tone mapping, the scene drawn straight into the canvas
//!   through a `CanvasFrameTarget` of the layer's own, over the layers below), and may place the
//!   camera itself (`Demo::place_camera`); a demo kind may have a SECOND CANVAS of the same scene
//!   with some settings held (`DemoKind::comparison`) and a 2D layer BELOW its own
//!   (`DemoKind::backdrop`); a WATER demo has its HDR target made with `Depth32Float` depth
//!   (`DemoKind::depth_format`, what the water renderer's scene capture reads) and a SECOND SCENE
//!   PASS (`Demo::water_pass`): after the opaque parts and the sky, the demo captures the scene
//!   and draws the water and the blended parts in a pass that loads colour and depth;
//! - its settings are declared as a list of CONTROLS (toggles, sliders, choices, buttons) the
//!   page renders and the layer reads every frame; its READ-OUTS and its page LABELS are what the
//!   layer publishes each frame; a short LEGEND says what to look at;
//! - the FRAME TIMINGS read-out (timings.rs, the crate's FrameTimer) is under every one of them.
//!
//! Every element a browser check reads has an id `<slug>-<key>` (`lights-shaded`, ...).

pub mod kit;
mod antialiasing;
mod culling;
mod glow;
mod lights;
mod lod;
mod particles;
mod picking;
mod shadows;
pub mod timings;
mod translucency;
mod water_lake;
pub mod water_pond;

use std::cell::RefCell;
use std::rc::Rc;

use codebrix_webgpu::glam::{Mat4, Quat, Vec3};
use codebrix_webgpu::{wgpu, CanvasClock, CanvasFrame, CanvasInput, CanvasLayers, CanvasRenderer, CanvasTarget, GpuContext, RendererHandle, WebGpuCanvas};
use codebrix_webgpu_graphics::{Camera, CanvasFrameTarget, CanvasFrameTargetOptions, Color4, EnvironmentMap, FrameTimer, FrameTimerOptions, GpuTexture, GraphicsContext, GraphicsException, GraphicsFrame, HeadlessRenderTarget, HeadlessTargetOptions, Model, ModelInstance, ModelRenderStats, ModelRenderer, OrbitCamera, OrbitController, RenderPass, RenderPassOptions, SceneLighting, SkyboxRenderer, ToneMapOperator, ToneMapper};
use dioxus::prelude::*;

use crate::demos::kit::{Kit, KitBytes};
use crate::demos::timings::{Timings, TimingsView};

/// The feature demos, one scene each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemoKind {
    /// Point lights and spotlights at dusk.
    Lights,
    /// The sun's shadow and the spotlights' shadows.
    Shadows,
    /// Frustum culling over a field of many instances.
    Culling,
    /// Two-pass double-sided translucency, per-instance alpha / tint / emissive, node visibility.
    Translucency,
    /// The glow layer and bloom on the HDR path.
    Glow,
    /// The GPU particle system.
    Particles,
    /// Picking with a ray and page-text labels.
    Picking,
    /// Anti-aliasing (MSAA): 1 or 4 samples per pixel, beside a second canvas left at 1.
    AntiAliasing,
    /// Level of detail with a fade band.
    Lod,
    /// A pool of simulated water over the mosaic rosette floor, stirred by the pointer and rain.
    WaterPond,
    /// A lake of open-water waves with a sloping bed, islands, a floating barrel and a wake.
    WaterLake,
}

impl DemoKind {
    /// Every demo, in the order of the page's buttons.
    pub const ALL: [DemoKind; 11] = [DemoKind::Lights, DemoKind::Shadows, DemoKind::Culling, DemoKind::Translucency, DemoKind::Glow, DemoKind::Particles, DemoKind::Picking, DemoKind::AntiAliasing, DemoKind::Lod, DemoKind::WaterPond, DemoKind::WaterLake];

    /// The demo's short name: the prefix of its element ids and its `?scene=` value.
    pub fn slug(self) -> &'static str {
        match self {
            DemoKind::Lights => "lights",
            DemoKind::Shadows => "shadows",
            DemoKind::Culling => "culling",
            DemoKind::Translucency => "translucency",
            DemoKind::Glow => "glow",
            DemoKind::Particles => "particles",
            DemoKind::Picking => "picking",
            DemoKind::AntiAliasing => "antialiasing",
            DemoKind::Lod => "lod",
            DemoKind::WaterPond => "water-pond",
            DemoKind::WaterLake => "water-lake",
        }
    }

    /// The demo's button text.
    pub fn title(self) -> &'static str {
        match self {
            DemoKind::Lights => "Lights",
            DemoKind::Shadows => "Shadows",
            DemoKind::Culling => "Culling",
            DemoKind::Translucency => "Translucency",
            DemoKind::Glow => "Glow and bloom",
            DemoKind::Particles => "Particles",
            DemoKind::Picking => "Picking and labels",
            DemoKind::AntiAliasing => "Anti-aliasing",
            DemoKind::Lod => "Level of detail",
            DemoKind::WaterPond => "Water pond",
            DemoKind::WaterLake => "Water lake",
        }
    }

    /// The demo in one sentence (the page without WebGPU lists it).
    pub fn summary(self) -> &'static str {
        match self {
            DemoKind::Lights => "coloured point lights and a spotlight at dusk, one lamp carried by a walking archer, more lamps than the renderer's 16",
            DemoKind::Shadows => "the sun's shadow and up to six shadow-casting spotlights over animated and skinned models",
            DemoKind::Culling => "a field of hundreds of models where everything outside the camera's view is skipped, with an overview",
            DemoKind::Translucency => "glass barrels drawn with and without two-pass translucency, fading, tinted and glowing foxes, hidden parts",
            DemoKind::Glow => "pulsing glows, glows riding on a bird's wings, hundreds of fireflies in one draw, and bloom",
            DemoKind::Particles => "sparks from a moving emitter, a mist and bursts where you click, on the canvas clock",
            DemoKind::Picking => "click a model to pick it; page-text labels follow the models",
            DemoKind::AntiAliasing => "thin fences and far trees against a bright sky with anti-aliasing Off or 4x, switched while it runs, beside the same scene left Off",
            DemoKind::Lod => "a field of groves that dissolve from many models to one simple surface to a far shape and out, with tinted levels, a fade band and counts",
            DemoKind::WaterPond => "a pool over a pastel glass mosaic of an eight-petalled rosette, its simulated water stirred by the pointer and by rain, refracting the floor, with caustics, sky reflection, sun glints and a thin-film sheen",
            DemoKind::WaterLake => "a lake of open-water waves with islands and a sloping bed, trees on the bank, a barrel riding the swell, a swimmer's wake, lamps and the sun's shadow on the water",
        }
    }

    fn create(self) -> Box<dyn Demo> {
        match self {
            DemoKind::Lights => Box::new(lights::LightsDemo::default()),
            DemoKind::Shadows => Box::new(shadows::ShadowsDemo::default()),
            DemoKind::Culling => Box::new(culling::CullingDemo::default()),
            DemoKind::Translucency => Box::new(translucency::TranslucencyDemo::default()),
            DemoKind::Glow => Box::new(glow::GlowDemo::default()),
            DemoKind::Particles => Box::new(particles::ParticlesDemo::default()),
            DemoKind::Picking => Box::new(picking::PickingDemo::default()),
            DemoKind::AntiAliasing => Box::new(antialiasing::AntiAliasingDemo::default()),
            DemoKind::Lod => Box::new(lod::LodDemo::default()),
            DemoKind::WaterPond => Box::new(water_pond::WaterPondDemo::default()),
            DemoKind::WaterLake => Box::new(water_lake::WaterLakeDemo::default()),
        }
    }

    fn controls(self) -> &'static [Control] {
        match self {
            DemoKind::Lights => lights::CONTROLS,
            DemoKind::Shadows => shadows::CONTROLS,
            DemoKind::Culling => culling::CONTROLS,
            DemoKind::Translucency => translucency::CONTROLS,
            DemoKind::Glow => glow::CONTROLS,
            DemoKind::Particles => particles::CONTROLS,
            DemoKind::Picking => picking::CONTROLS,
            DemoKind::AntiAliasing => antialiasing::CONTROLS,
            DemoKind::Lod => lod::CONTROLS,
            DemoKind::WaterPond => water_pond::CONTROLS,
            DemoKind::WaterLake => water_lake::CONTROLS,
        }
    }

    fn legend(self) -> &'static [&'static str] {
        match self {
            DemoKind::Lights => lights::LEGEND,
            DemoKind::Shadows => shadows::LEGEND,
            DemoKind::Culling => culling::LEGEND,
            DemoKind::Translucency => translucency::LEGEND,
            DemoKind::Glow => glow::LEGEND,
            DemoKind::Particles => particles::LEGEND,
            DemoKind::Picking => picking::LEGEND,
            DemoKind::AntiAliasing => antialiasing::LEGEND,
            DemoKind::Lod => lod::LEGEND,
            DemoKind::WaterPond => water_pond::LEGEND,
            DemoKind::WaterLake => water_lake::LEGEND,
        }
    }

    /// The settings a SECOND canvas of the same scene holds (beside the first, on the same clock,
    /// its camera following the first's), or None for a demo with one canvas.
    fn comparison(self) -> Option<&'static [(&'static str, f32)]> {
        match self {
            DemoKind::AntiAliasing => Some(antialiasing::COMPARISON),
            _ => None,
        }
    }

    /// Whether the demo's canvas has a 2D layer BELOW the demo's own (seen where the direct path
    /// leaves the canvas uncovered).
    fn backdrop(self) -> bool {
        matches!(self, DemoKind::AntiAliasing)
    }

    /// Whether the shell adds its multisampling read-outs (samples per pixel, the path, the buffer
    /// sets made) to the demo's.
    fn multisampled(self) -> bool {
        matches!(self, DemoKind::AntiAliasing | DemoKind::Lod | DemoKind::WaterPond | DemoKind::WaterLake)
    }

    /// The depth format of the demo's HDR target: `Depth32Float` for the water demos (the water
    /// renderer's scene capture reads only that), the shell's [`DEPTH_FORMAT`] for the others.
    fn depth_format(self) -> wgpu::TextureFormat {
        match self {
            DemoKind::WaterPond | DemoKind::WaterLake => wgpu::TextureFormat::Depth32Float,
            _ => DEPTH_FORMAT,
        }
    }

    /// The demo whose slug `scene=` names in the page's query string, if any.
    pub fn from_query() -> Option<DemoKind> {
        let search = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
        search.trim_start_matches('?').split('&').find_map(|pair| pair.strip_prefix("scene=")).and_then(|name| DemoKind::ALL.into_iter().find(|kind| kind.slug() == name))
    }
}

/// One control of a demo; its value is a number in [`Settings`] under `key` (toggles 0 / 1,
/// buttons count their presses).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Control {
    /// On or off.
    Toggle { key: &'static str, label: &'static str, default: bool },
    /// A number between `min` and `max`.
    Slider { key: &'static str, label: &'static str, min: f32, max: f32, step: f32, default: f32 },
    /// One of a few named numbers.
    Choice { key: &'static str, label: &'static str, options: &'static [(&'static str, f32)], default: f32 },
    /// A button: the value counts the presses.
    Button { key: &'static str, label: &'static str },
}

impl Control {
    fn key(&self) -> &'static str {
        match self {
            Control::Toggle { key, .. } | Control::Slider { key, .. } | Control::Choice { key, .. } | Control::Button { key, .. } => key,
        }
    }

    fn default_value(&self) -> f32 {
        match self {
            Control::Toggle { default, .. } => f32::from(u8::from(*default)),
            Control::Slider { default, .. } | Control::Choice { default, .. } => *default,
            Control::Button { .. } => 0.0,
        }
    }
}

/// The values of a demo's controls, by key.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    values: Vec<(&'static str, f32)>,
}

impl Settings {
    fn from_controls(controls: &[Control]) -> Settings {
        Settings { values: controls.iter().map(|c| (c.key(), c.default_value())).collect() }
    }

    /// The value of `key` (0 for a key no control has).
    pub fn get(&self, key: &str) -> f32 {
        self.values.iter().find(|(k, _)| *k == key).map_or(0.0, |(_, v)| *v)
    }

    /// Whether the toggle `key` is on.
    pub fn on(&self, key: &str) -> bool {
        self.get(key) >= 0.5
    }

    pub fn set(&mut self, key: &str, value: f32) {
        if let Some(entry) = self.values.iter_mut().find(|(k, _)| *k == key) {
            entry.1 = value;
        }
    }
}

/// One read-out line of a demo: shown as "label: value", the value in the element `<slug>-<key>`.
#[derive(Clone, Debug, PartialEq)]
pub struct Readout {
    /// The id suffix.
    pub key: &'static str,
    /// What it is.
    pub label: &'static str,
    /// Its value now.
    pub value: String,
}

impl Readout {
    /// A read-out.
    pub fn new(key: &'static str, label: &'static str, value: impl ToString) -> Readout {
        Readout { key, label, value: value.to_string() }
    }
}

/// A text label on the page over the canvas, at CSS pixels from the canvas's top-left corner.
#[derive(Clone, Debug, PartialEq)]
pub struct PageLabel {
    /// The id suffix (`<slug>-label-<key>`).
    pub key: String,
    /// The text.
    pub text: String,
    /// Left, in CSS pixels.
    pub x: f32,
    /// Top, in CSS pixels.
    pub y: f32,
    /// False: hidden (behind the camera or outside the canvas).
    pub shown: bool,
    /// True: drawn highlighted (the picked model's label).
    pub highlighted: bool,
}

/// What the layer publishes for the page each frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DemoView {
    /// The frame number on the clock.
    pub frame: u64,
    /// The frame's clock time, in seconds.
    pub time: f64,
    /// The demo's read-outs.
    pub readouts: Vec<Readout>,
    /// The demo's page labels.
    pub labels: Vec<PageLabel>,
}

/// What a demo gets at the start of a frame.
pub struct DemoFrame<'a> {
    /// The frame's clock time, in seconds (pause, speed and seeks apply).
    pub time: f64,
    /// The clock seconds since the last frame (0 while paused).
    pub delta: f64,
    /// The camera of this frame (already placed by the orbit controller).
    pub camera: &'a Camera,
    /// The canvas size in CSS pixels.
    pub css: (f32, f32),
    /// The procedural sky environment (the context's default).
    pub environment: &'a EnvironmentMap,
}

/// What a demo draws with in the scene pass.
pub struct DrawContext<'a> {
    /// The camera.
    pub camera: &'a Camera,
    /// The skybox renderer (draw it between the opaque and the blended parts).
    pub skybox: &'a mut SkyboxRenderer,
    /// The frame's clock time.
    pub time: f64,
}

/// A demo scene on the shell: builds its instances, moves them each frame, draws them.
pub trait Demo {
    /// Makes the demo's models, instances and renderers; says how to frame the camera.
    fn build(&mut self, kit: &mut Kit<'_>) -> Result<Framing, String>;
    /// Called when the canvas is resized (in device pixels).
    fn resize(&mut self, _graphics: &GraphicsContext, _width: u32, _height: u32) -> Result<(), String> {
        Ok(())
    }
    /// Moves the scene to the frame's time and reads the settings.
    fn update(&mut self, frame: &DemoFrame<'_>, settings: &Settings) -> Result<(), String>;
    /// Work recorded before the scene pass (compute steps).
    fn before_scene(&mut self, _frame: &mut GraphicsFrame<'_>, _delta: f64, _settings: &Settings) -> Result<(), GraphicsException> {
        Ok(())
    }
    /// Draws the scene into the HDR target's pass (opaque, skybox, blended).
    fn draw(&mut self, pass: &mut RenderPass<'_>, context: &mut DrawContext<'_>) -> Result<(), GraphicsException>;
    /// True: the demo draws a SECOND scene pass ([`Demo::water_pass`]) after its `draw`.
    fn has_water_pass(&self) -> bool {
        false
    }
    /// The second scene pass of a water demo, after the pass of `draw` ended (opaque parts and the
    /// sky): the demo captures the HDR target (`WaterRenderer::capture_scene`) and draws the water
    /// and its blended parts in a pass it begins on `hdr` with `options`, loading colour and depth.
    fn water_pass(&mut self, _frame: &mut GraphicsFrame<'_>, _hdr: &HeadlessRenderTarget, _options: &RenderPassOptions, _context: &mut DrawContext<'_>) -> Result<(), GraphicsException> {
        Ok(())
    }
    /// Work after the scene pass, on the HDR texture (bloom).
    fn after_scene(&mut self, _frame: &mut GraphicsFrame<'_>, _scene: &GpuTexture, _settings: &Settings) -> Result<(), GraphicsException> {
        Ok(())
    }
    /// Work before the tone-map pass (another view of the scene into a target of its own).
    fn before_tone_map(&mut self, _frame: &mut GraphicsFrame<'_>, _camera: &Camera, _settings: &Settings) -> Result<(), GraphicsException> {
        Ok(())
    }
    /// Draws over the tone-mapped scene in the canvas pass (an inset), the canvas being `size`.
    fn in_tone_map(&mut self, _pass: &mut RenderPass<'_>, _tone_mapper: &mut ToneMapper, _size: (u32, u32), _settings: &Settings) -> Result<(), GraphicsException> {
        Ok(())
    }
    /// The model renderer's counts of this frame (for the timings' counts).
    fn stats(&self) -> ModelRenderStats;
    /// The read-outs of this frame.
    fn readouts(&self, out: &mut Vec<Readout>);
    /// The page labels of this frame.
    fn labels(&self, _camera: &Camera, _css: (f32, f32)) -> Vec<PageLabel> {
        Vec::new()
    }
    /// Input on the canvas; true when the demo used it (the orbit controller then does not).
    fn input(&mut self, _input: &CanvasInput, _camera: &Camera, _css: (f32, f32)) -> bool {
        false
    }
    /// The samples per pixel the scene is drawn with: 1 (every demo so far) or 4.
    fn sample_count(&self, _settings: &Settings) -> u32 {
        1
    }
    /// True: the DIRECT path - the scene drawn straight into the canvas (no HDR target, no tone
    /// mapping) over what the layers below drew, the demo leaving out the skybox; only `draw` is
    /// called (no compute step, bloom or inset).
    fn direct_to_canvas(&self, _settings: &Settings) -> bool {
        false
    }
    /// Places the camera after the orbit controller did, for a demo that moves the camera itself
    /// (on the frame's time).
    fn place_camera(&mut self, _time: f64, _settings: &Settings, _camera: &mut Camera) {}
}

/// The camera of a demo's first canvas, which its second canvas follows.
type SharedCamera = Rc<RefCell<Option<Camera>>>;

/// Which canvas of a demo a layer draws: the first (it publishes its camera) or the second (it
/// follows the first's camera and holds some settings).
#[derive(Clone)]
enum CanvasRole {
    First(SharedCamera),
    Second(SharedCamera, &'static [(&'static str, f32)]),
}

/// How the shell frames the camera for a demo.
#[derive(Clone, Copy, Debug)]
pub struct Framing {
    /// The box to frame (OrbitController::frame_box).
    pub min: Vec3,
    /// The box's other corner.
    pub max: Vec3,
    /// The start yaw, in radians.
    pub yaw: f32,
    /// The pitch, in radians.
    pub pitch: f32,
    /// The distance as a multiple of the framed one (below 1: closer).
    pub distance_scale: f32,
    /// Whether the camera turns by itself until the first drag.
    pub auto_orbit: bool,
}

/// The loading line, the failure message, or the demo's canvas.
#[component]
pub fn DemoScene(context: GpuContext, kind: DemoKind) -> Element {
    let loaded = use_resource(kit::load);
    let slug = kind.slug();
    let state = loaded.read();
    match &*state {
        None => rsx! { p { id: "{slug}-loading", role: "status", class: "rounded-lg bg-slate-900 p-4 text-sm text-slate-300 ring-1 ring-white/10", "Loading the models from the backend..." } },
        Some(Err(message)) => rsx! {
            div { id: "{slug}-failed", role: "alert", class: "rounded-lg border border-red-400/40 bg-red-400/10 p-5 text-red-100",
                p { class: "text-lg font-semibold text-red-200", "The models could not be loaded" }
                p { class: "mt-2 text-sm", "{message}" }
            }
        },
        Some(Ok(bytes)) => rsx! { DemoCanvas { key: "{slug}", context: context.clone(), kind, bytes: bytes.clone() } },
    }
}

/// The canvas, the clock buttons, the controls, the read-outs, the labels and the timings.
#[component]
fn DemoCanvas(context: GpuContext, kind: DemoKind, bytes: KitBytes) -> Element {
    let slug = kind.slug();
    let graphics = use_hook(|| GraphicsContext::wrap_gpu_context(&context));
    let mut settings = use_signal(|| Settings::from_controls(kind.controls()));
    let view = use_signal(DemoView::default);
    let timings = use_signal(TimingsView::default);
    let errors = use_signal(Vec::<String>::new);
    let mut paused = use_signal(|| false);
    let mut speed = use_signal(|| 1.0f64);
    let mut timings_shown = use_signal(|| true);
    let clock = use_hook(CanvasClock::new);
    let shared_camera: SharedCamera = use_hook(|| Rc::new(RefCell::new(None)));
    // The second canvas (a demo with a comparison): the same scene on the same clock, its camera
    // following the first canvas's, some settings held.
    let second_view = use_signal(DemoView::default);
    let second_timings = use_signal(TimingsView::default);
    let second_errors = use_signal(Vec::<String>::new);
    let second_layers = {
        let (graphics, bytes, shared_camera) = (graphics.clone(), bytes.clone(), shared_camera.clone());
        use_hook(move || {
            kind.comparison().map(|held| {
                let layer = RendererHandle::new(DemoLayer::new(graphics.clone(), kind, bytes, settings, second_view, second_timings, second_errors, CanvasRole::Second(shared_camera, held)));
                if kind.backdrop() {
                    CanvasLayers::new(vec![RendererHandle::new(antialiasing::Backdrop::new(graphics)), layer])
                } else {
                    CanvasLayers::single(layer)
                }
            })
        })
    };
    let layers = use_hook(move || {
        let layer = RendererHandle::new(DemoLayer::new(graphics.clone(), kind, bytes, settings, view, timings, errors, CanvasRole::First(shared_camera)));
        if kind.backdrop() {
            CanvasLayers::new(vec![RendererHandle::new(antialiasing::Backdrop::new(graphics.clone())), layer])
        } else {
            CanvasLayers::single(layer)
        }
    });
    let toggle_clock = {
        let clock = clock.clone();
        move |_| paused.set(clock.toggle())
    };
    let change_speed = {
        let clock = clock.clone();
        move |_| {
            let next = if speed() == 1.0 { 0.5 } else if speed() == 0.5 { 0.25 } else { 1.0 };
            clock.set_speed(next);
            speed.set(next);
        }
    };
    let button = "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700";
    let field = "rounded-md bg-slate-800 px-2 py-1 text-sm text-slate-100 ring-1 ring-white/10";
    let shown = view();
    let second_shown = second_view();
    let state = if paused() { "paused" } else { "running" };
    rsx! {
        div { class: if second_layers.is_some() { "grid grid-cols-1 gap-3 lg:grid-cols-2" } else { "contents" },
        div { class: "relative",
            WebGpuCanvas {
                context: context.clone(),
                layers,
                clock: clock.clone(),
                clear_color: wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
                id: "{slug}-canvas",
                label: "{kind.title()}: {kind.summary()}. Drag to orbit, drag with the right or middle button or with Shift to pan, use the wheel to zoom.",
                class: "aspect-video w-full touch-none select-none rounded-lg ring-1 ring-white/10 focus-visible:outline-2 focus-visible:outline-sky-400",
            }
            div { id: "{slug}-label-layer", class: "pointer-events-none absolute inset-0 overflow-hidden",
                for label in shown.labels.iter() {
                    span {
                        key: "{label.key}",
                        id: "{slug}-label-{label.key}",
                        class: if !label.shown { "hidden" } else if label.highlighted { "absolute -translate-x-1/2 -translate-y-full whitespace-nowrap rounded bg-amber-300 px-1.5 py-0.5 text-xs font-semibold text-slate-950" } else { "absolute -translate-x-1/2 -translate-y-full whitespace-nowrap rounded bg-slate-950/75 px-1.5 py-0.5 text-xs text-slate-100 ring-1 ring-white/20" },
                        left: "{label.x:.1}px",
                        top: "{label.y:.1}px",
                        "{label.text}"
                    }
                }
            }
            div { id: "{slug}-legend", class: "pointer-events-none absolute left-3 top-3 max-w-sm rounded-md bg-slate-950/75 px-3 py-2 text-xs text-slate-200 ring-1 ring-white/10",
                p { class: "font-semibold text-white", "{kind.title()}: what to look at" }
                for line in kind.legend() {
                    p { "{line}" }
                }
                p { class: "mt-1 text-slate-400", "Drag: orbit. Right / middle / Shift drag: pan. Wheel: zoom." }
            }
        }
        if let Some(second) = second_layers.clone() {
            div { class: "flex flex-col gap-2",
                WebGpuCanvas {
                    context: context.clone(),
                    layers: second,
                    clock: clock.clone(),
                    clear_color: wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
                    id: "{slug}-compare-canvas",
                    label: "{kind.title()}: the same scene for comparison, with some settings held; its camera follows the first canvas.",
                    class: "aspect-video w-full touch-none select-none rounded-lg ring-1 ring-white/10",
                }
                div { id: "{slug}-compare-panel", class: "rounded-lg bg-slate-900 p-3 text-xs text-slate-300 ring-1 ring-white/10",
                    p { class: "font-semibold text-white", "The comparison canvas (its settings held; the camera follows the first canvas)" }
                    for readout in second_shown.readouts.iter().filter(|r| SHELL_READOUTS.contains(&r.key)) {
                        p { key: "{readout.key}",
                            "{readout.label}: " span { id: "{slug}-compare-{readout.key}", class: "font-mono text-white", "{readout.value}" }
                        }
                    }
                    p { "Frame " span { id: "{slug}-compare-frame", class: "font-mono text-white", "{second_shown.frame}" } "; errors while drawing: " span { id: "{slug}-compare-error-count", class: "font-mono text-white", "{second_errors.read().len()}" } }
                    for error in second_errors.read().iter() {
                        p { class: "font-mono text-red-300", "{error}" }
                    }
                }
            }
        }
        }
        div { class: "flex flex-wrap items-center gap-3",
            button { id: "{slug}-clock-toggle", class: "{button}", onclick: toggle_clock, if paused() { "Resume the clock" } else { "Pause the clock" } }
            button { id: "{slug}-speed", class: "{button}", onclick: change_speed, "Clock speed x{speed()}" }
            button { id: "{slug}-timings-toggle", class: "{button}", "aria-pressed": "{timings_shown()}", onclick: move |_| timings_shown.set(!timings_shown()), if timings_shown() { "Hide the frame timings" } else { "Show the frame timings" } }
        }
        div { id: "{slug}-controls", class: "flex flex-wrap items-center gap-x-5 gap-y-3 rounded-lg bg-slate-900 p-3 text-sm text-slate-300 ring-1 ring-white/10",
            for control in kind.controls().iter().copied() {
                match control {
                    Control::Toggle { key, label, .. } => {
                        let on = settings.read().on(key);
                        rsx! {
                            button {
                                key: "{key}",
                                id: "{slug}-{key}",
                                class: if on { "rounded-md bg-sky-700 px-3 py-1.5 text-sm text-white ring-1 ring-sky-300/40" } else { "{button}" },
                                "aria-pressed": "{on}",
                                onclick: move |_| { let value = if settings.peek().on(key) { 0.0 } else { 1.0 }; settings.write().set(key, value); },
                                "{label}: " if on { "on" } else { "off" }
                            }
                        }
                    }
                    Control::Slider { key, label, min, max, step, .. } => {
                        let value = settings.read().get(key);
                        rsx! {
                            label { key: "{key}", class: "flex items-center gap-2",
                                "{label}"
                                input {
                                    id: "{slug}-{key}",
                                    class: "w-32 accent-sky-400",
                                    r#type: "range",
                                    min: "{min}",
                                    max: "{max}",
                                    step: "{step}",
                                    value: "{value}",
                                    oninput: move |event: Event<FormData>| {
                                        if let Ok(v) = event.value().parse::<f32>() {
                                            settings.write().set(key, v.clamp(min, max));
                                        }
                                    },
                                }
                                span { id: "{slug}-{key}-value", class: "w-12 font-mono text-white", "{trim(value)}" }
                            }
                        }
                    }
                    Control::Choice { key, label, options, .. } => {
                        let value = settings.read().get(key);
                        rsx! {
                            label { key: "{key}", class: "flex items-center gap-2",
                                "{label}"
                                select {
                                    id: "{slug}-{key}",
                                    class: "{field}",
                                    onchange: move |event: Event<FormData>| {
                                        if let Some((_, v)) = options.iter().find(|(name, _)| *name == event.value()) {
                                            settings.write().set(key, *v);
                                        }
                                    },
                                    for (name, v) in options.iter() {
                                        option { value: "{name}", selected: *v == value, "{name}" }
                                    }
                                }
                            }
                        }
                    }
                    Control::Button { key, label } => rsx! {
                        button { key: "{key}", id: "{slug}-{key}", class: "{button}", onclick: move |_| { let value = settings.peek().get(key) + 1.0; settings.write().set(key, value); }, "{label}" }
                    },
                }
            }
        }
        div { class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
            p { class: "text-slate-300",
                "Frame " span { id: "{slug}-frame", class: "font-mono text-white", "{shown.frame}" }
                ", clock " span { id: "{slug}-clock", class: "font-mono text-white", "{shown.time:.3}" } " s, "
                span { id: "{slug}-clock-state", "{state}" }
            }
            for readout in shown.readouts.iter() {
                p { key: "{readout.key}", class: "text-slate-300",
                    "{readout.label}: " span { id: "{slug}-{readout.key}", class: "font-mono text-white", "{readout.value}" }
                }
            }
            p { class: "text-slate-300", "Errors while drawing: " span { id: "{slug}-error-count", class: "font-mono text-white", "{errors.read().len()}" } }
            for error in errors.read().iter() {
                p { class: "font-mono text-red-300", "{error}" }
            }
        }
        if kind == DemoKind::WaterPond {
            water_pond::ProbePanel { context: context.clone() }
        }
        if timings_shown() {
            Timings { slug, view: timings }
        }
    }
}

/// A number without trailing zeros.
fn trim(value: f32) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The depth format of the demos' HDR target (and of the direct path's canvas target).
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

/// The keys of the shell's own multisampling read-outs (also shown under a second canvas).
const SHELL_READOUTS: [&str; 4] = ["ms-samples", "ms-path", "ms-hdr-sets", "ms-canvas-sets"];

/// What the shell draws with, once prepared.
struct ShellGpu {
    demo: Box<dyn Demo>,
    skybox: SkyboxRenderer,
    tone_mapper: ToneMapper,
    timer: Option<FrameTimer>,
    environment: EnvironmentMap,
}

/// The layer of every demo: the HDR target, the tone mapper, the orbit camera, the timer and the
/// demo's own drawing.
pub struct DemoLayer {
    graphics: GraphicsContext,
    kind: DemoKind,
    bytes: KitBytes,
    settings: Signal<Settings>,
    view: Signal<DemoView>,
    timings: Signal<TimingsView>,
    errors: Signal<Vec<String>>,
    gpu: Option<ShellGpu>,
    hdr: Option<HeadlessRenderTarget>,
    controller: OrbitController,
    camera: Camera,
    css: (f32, f32),
    timings_shown_at: f64,
    role: CanvasRole,
    /// The direct path's canvas target (made the first time the direct path is used).
    canvas_target: Option<CanvasFrameTarget>,
    /// The page's count of the HDR target's buffer sets: one per target made (its single-sampled
    /// set) plus one per multisampled colour texture seen for the first time (HeadlessRenderTarget
    /// has no counter of its own).
    hdr_sets_made: u64,
    hdr_multisampled_seen: Option<wgpu::Texture>,
    /// What the last frame drew with: samples per pixel, direct path.
    drawn_with: (u32, bool),
}

impl DemoLayer {
    #[allow(clippy::too_many_arguments)]
    fn new(graphics: GraphicsContext, kind: DemoKind, bytes: KitBytes, settings: Signal<Settings>, view: Signal<DemoView>, timings: Signal<TimingsView>, errors: Signal<Vec<String>>, role: CanvasRole) -> DemoLayer {
        let mut camera = Camera::new();
        camera.field_of_view = 50f32.to_radians();
        DemoLayer { graphics, kind, bytes, settings, view, timings, errors, gpu: None, hdr: None, controller: OrbitController::new(OrbitCamera::new()), camera, css: (1.0, 1.0), timings_shown_at: f64::NEG_INFINITY, role, canvas_target: None, hdr_sets_made: 0, hdr_multisampled_seen: None, drawn_with: (1, false) }
    }

    fn report(&mut self, error: String) {
        let mut errors = self.errors.write();
        if errors.len() < 20 && !errors.contains(&error) {
            errors.push(error);
        }
    }

    fn create(&mut self) -> Result<ShellGpu, String> {
        let environment = self.graphics.default_environment().map_err(|e| e.to_string())?;
        let mut kit = Kit::new(&self.graphics, &self.bytes);
        let mut demo = self.kind.create();
        let framing = demo.build(&mut kit)?;
        let mut orbit = OrbitCamera::new();
        orbit.set_pitch(framing.pitch);
        orbit.yaw = framing.yaw;
        let mut controller = OrbitController::new(orbit);
        controller.frame_box(framing.min, framing.max, &self.camera, 16.0 / 9.0);
        let distance = controller.orbit.distance() * framing.distance_scale;
        controller.orbit.set_distance(distance);
        let radius = (framing.max - framing.min).length() * 0.5;
        self.camera.near_plane = (radius * 0.004).max(0.05);
        self.camera.far_plane = radius * 12.0 + distance;
        controller.auto_orbit_start_yaw = framing.yaw;
        controller.auto_orbit_speed = 0.12;
        controller.set_auto_orbit(framing.auto_orbit);
        self.controller = controller;
        let skybox = self.graphics.create_skybox_renderer().map_err(|e| e.to_string())?;
        let mut tone_mapper = self.graphics.create_tone_mapper().map_err(|e| e.to_string())?;
        tone_mapper.operator = ToneMapOperator::KhronosPbrNeutral;
        let timer = match self.graphics.create_frame_timer(FrameTimerOptions { label: Some(format!("{} timer", self.kind.slug())), ..FrameTimerOptions::default() }) {
            Ok(timer) => Some(timer),
            Err(error) => {
                self.report(format!("The frame timer could not be made: {error}"));
                None
            }
        };
        Ok(ShellGpu { demo, skybox, tone_mapper, timer, environment })
    }
}

impl CanvasRenderer for DemoLayer {
    fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
        match self.create() {
            Ok(gpu) => self.gpu = Some(gpu),
            Err(error) => self.report(error),
        }
        for error in self.graphics.take_errors() {
            self.report(error.to_string());
        }
    }

    fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
        let options = HeadlessTargetOptions { format: wgpu::TextureFormat::Rgba16Float, with_depth: true, depth_format: self.kind.depth_format() };
        match self.graphics.create_headless_target(width, height, options) {
            Ok(hdr) => {
                self.hdr = Some(hdr);
                self.hdr_sets_made += 1;
            }
            Err(error) => self.report(error.to_string()),
        }
        let graphics = self.graphics.clone();
        let failure = self.gpu.as_mut().and_then(|gpu| gpu.demo.resize(&graphics, width, height).err());
        if let Some(error) = failure {
            self.report(error);
        }
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let time = frame.info.time;
        let delta = frame.info.delta;
        let number = frame.info.number;
        let size = (frame.info.width, frame.info.height);
        let ratio = frame.info.pixel_ratio as f32;
        self.css = (frame.info.width as f32 / ratio, frame.info.height as f32 / ratio);
        let mut settings = self.settings.peek().clone();
        match &self.role {
            CanvasRole::First(shared) => {
                self.controller.update(time);
                self.controller.set_viewport(&self.camera, self.css.1);
                self.controller.apply(&mut self.camera);
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.demo.place_camera(time, &settings, &mut self.camera);
                }
                *shared.borrow_mut() = Some(self.camera);
            }
            CanvasRole::Second(shared, held) => {
                if let Some(camera) = shared.borrow().as_ref() {
                    self.camera = *camera;
                }
                for (key, value) in held.iter() {
                    settings.set(key, *value);
                }
            }
        }
        let graphics = self.graphics.clone();
        let mut failures = Vec::new();
        let mut view = DemoView { frame: number, time: (time * 1000.0).round() / 1000.0, ..DemoView::default() };
        let mut timing_line = None;
        if let (Some(gpu), Some(hdr)) = (self.gpu.as_mut(), self.hdr.as_mut()) {
            if let Some(timer) = gpu.timer.as_mut() {
                timer.begin_frame();
            }
            let samples = gpu.demo.sample_count(&settings);
            let direct = gpu.demo.direct_to_canvas(&settings);
            if direct {
                if self.canvas_target.is_none() {
                    match graphics.create_canvas_frame_target(CanvasFrameTargetOptions { sample_count: samples, depth_format: Some(self.kind.depth_format()) }) {
                        Ok(target) => self.canvas_target = Some(target),
                        Err(error) => failures.push(error.to_string()),
                    }
                }
                if let Some(target) = self.canvas_target.as_mut() {
                    if let Err(error) = target.set_sample_count(samples) {
                        failures.push(error.to_string());
                    }
                }
            } else {
                if let Err(error) = hdr.set_sample_count(samples) {
                    failures.push(error.to_string());
                }
                // A multisampled colour texture not seen before is a new buffer set.
                if let Some(multi) = hdr.multisampled_color() {
                    if self.hdr_multisampled_seen.as_ref() != Some(multi.handle()) {
                        self.hdr_multisampled_seen = Some(multi.handle().clone());
                        self.hdr_sets_made += 1;
                    }
                }
            }
            self.drawn_with = (if direct { self.canvas_target.as_ref().map_or(1, |t| t.sample_count()) } else { hdr.sample_count() }, direct);
            let demo_frame = DemoFrame { time, delta, camera: &self.camera, css: self.css, environment: &gpu.environment };
            if let Err(error) = gpu.demo.update(&demo_frame, &settings) {
                failures.push(error);
            }
            let camera = &self.camera;
            let canvas_target = self.canvas_target.as_mut();
            let hdr: &HeadlessRenderTarget = hdr;
            let result: Result<(), GraphicsException> = (|| {
                if let (true, Some(target)) = (direct, canvas_target) {
                    // THE DIRECT PATH: the scene straight into the canvas (loading what the layers
                    // below drew, clearing the target's own depth), 1 or 4 samples per pixel.
                    let mut graphics_frame = graphics.begin_canvas_frame_on(frame, target)?;
                    {
                        let options = RenderPassOptions { timestamps: gpu.timer.as_mut().and_then(|t| t.pass("scene")), ..RenderPassOptions::default() };
                        let mut pass = graphics_frame.begin_render_pass_with_options(None, None, Some(1.0), &options)?;
                        let mut context = DrawContext { camera, skybox: &mut gpu.skybox, time };
                        gpu.demo.draw(&mut pass, &mut context)?;
                    }
                    if let Some(timer) = gpu.timer.as_mut() {
                        let stats = gpu.demo.stats();
                        timer.set_count("draw calls", stats.draw_calls as u64);
                        timer.set_count("instances", stats.instances as u64);
                        timer.set_count("culled", stats.culled_instances as u64);
                        timer.set_count("shadow passes", stats.shadow_passes as u64);
                        timer.end_frame(&mut graphics_frame);
                    }
                    return Ok(());
                }
                let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
                gpu.demo.before_scene(&mut graphics_frame, delta, &settings)?;
                {
                    let options = RenderPassOptions { timestamps: gpu.timer.as_mut().and_then(|t| t.pass("scene")), ..RenderPassOptions::default() };
                    let mut pass = graphics_frame.begin_render_pass_with_options(Some(&hdr.view()), Some(Color4::BLACK), Some(1.0), &options)?;
                    let mut context = DrawContext { camera, skybox: &mut gpu.skybox, time };
                    gpu.demo.draw(&mut pass, &mut context)?;
                }
                if gpu.demo.has_water_pass() {
                    let options = RenderPassOptions { timestamps: gpu.timer.as_mut().and_then(|t| t.pass("water")), ..RenderPassOptions::default() };
                    let mut context = DrawContext { camera, skybox: &mut gpu.skybox, time };
                    gpu.demo.water_pass(&mut graphics_frame, hdr, &options, &mut context)?;
                }
                gpu.demo.after_scene(&mut graphics_frame, hdr.color(), &settings)?;
                gpu.demo.before_tone_map(&mut graphics_frame, camera, &settings)?;
                {
                    let options = RenderPassOptions { timestamps: gpu.timer.as_mut().and_then(|t| t.pass("tone map")), ..RenderPassOptions::default() };
                    let mut pass = graphics_frame.begin_render_pass_with_options(None, None, None, &options)?;
                    gpu.tone_mapper.apply_in_pass(&mut pass, hdr.color())?;
                    gpu.demo.in_tone_map(&mut pass, &mut gpu.tone_mapper, size, &settings)?;
                }
                if let Some(timer) = gpu.timer.as_mut() {
                    let stats = gpu.demo.stats();
                    timer.set_count("draw calls", stats.draw_calls as u64);
                    timer.set_count("instances", stats.instances as u64);
                    timer.set_count("culled", stats.culled_instances as u64);
                    timer.set_count("shadow passes", stats.shadow_passes as u64);
                    timer.end_frame(&mut graphics_frame);
                }
                Ok(())
            })();
            if let Err(error) = result {
                failures.push(error.to_string());
            }
            gpu.demo.readouts(&mut view.readouts);
            if self.kind.multisampled() {
                let (samples, direct) = self.drawn_with;
                view.readouts.push(Readout::new("ms-samples", "Samples per pixel (this canvas)", samples));
                view.readouts.push(Readout::new("ms-path", "Drawn", if direct { "straight into the canvas (CanvasFrameTarget), over the layer below" } else { "into the HDR target, then tone mapped" }));
                view.readouts.push(Readout::new("ms-hdr-sets", "HDR target buffer sets made (the page's count)", self.hdr_sets_made));
                view.readouts.push(Readout::new("ms-canvas-sets", "Canvas target buffer sets made (buffer_sets_made)", self.canvas_target.as_ref().map_or(0, |t| t.buffer_sets_made())));
            }
            view.labels = gpu.demo.labels(&self.camera, self.css);
            // The timings read-out changes too fast to read every frame: twice a second.
            let now = kit::now_ms();
            if now - self.timings_shown_at >= 500.0 {
                if let Some(timer) = gpu.timer.as_ref() {
                    timing_line = Some(TimingsView::from_timings(timer.timings()));
                    self.timings_shown_at = now;
                }
            }
        }
        for error in failures {
            self.report(error);
        }
        for error in self.graphics.take_errors() {
            self.report(error.to_string());
        }
        if *self.view.peek() != view {
            self.view.set(view);
        }
        if let Some(line) = timing_line {
            if *self.timings.peek() != line {
                self.timings.set(line);
            }
        }
    }

    fn input(&mut self, input: &CanvasInput) {
        if matches!(self.role, CanvasRole::Second(..)) {
            return;
        }
        let used = match self.gpu.as_mut() {
            Some(gpu) => gpu.demo.input(input, &self.camera, self.css),
            None => false,
        };
        if !used {
            self.controller.input(input);
        }
    }
}

/// An instance of `model` scaled by `scale`, turned by `yaw` about Y and placed at `at`.
pub fn placed(model: &Model, scale: f32, yaw: f32, at: Vec3) -> ModelInstance {
    model.create_instance_at(Mat4::from_scale_rotation_translation(Vec3::splat(scale), Quat::from_rotation_y(yaw), at))
}

/// The world matrix of a model scaled by `scale`, turned by `yaw` and placed at `at`.
pub fn world(scale: f32, yaw: f32, at: Vec3) -> Mat4 {
    Mat4::from_scale_rotation_translation(Vec3::splat(scale), Quat::from_rotation_y(yaw), at)
}

/// The usual drawing of a set of instances: begin, every instance, the opaque parts, the sky
/// behind them, then the blended parts.
pub fn draw_models(renderer: &mut ModelRenderer, instances: &[ModelInstance], pass: &mut RenderPass<'_>, context: &mut DrawContext<'_>, lighting: &SceneLighting) -> Result<(), GraphicsException> {
    renderer.begin(pass, context.camera, Some(lighting))?;
    for instance in instances {
        renderer.draw(instance)?;
    }
    renderer.end_opaque(pass)?;
    context.skybox.draw(pass, context.camera, Some(lighting))?;
    renderer.end_blended(pass)
}

/// The scene lighting of a dim scene: the procedural sky at `environment` strength and a weak
/// key light from `direction` at `sun`.
pub fn dim_lighting(environment: &EnvironmentMap, environment_intensity: f32, sun: f32, direction: Vec3) -> SceneLighting {
    SceneLighting { environment: Some(environment.clone()), environment_intensity, light_intensity: sun, light_direction: direction.normalize(), ..SceneLighting::default() }
}

/// The aspect ratio of a canvas size.
pub fn aspect(css: (f32, f32)) -> f32 {
    if css.1 > 0.0 { css.0 / css.1 } else { 16.0 / 9.0 }
}

/// A point on the ground plane (y = 0) under the canvas point (x, y), if the ray reaches it.
pub fn ground_point(camera: &Camera, x: f32, y: f32, css: (f32, f32)) -> Option<Vec3> {
    let ray = camera.canvas_to_ray(x, y, css.0, css.1);
    if ray.direction.y.abs() < 1e-5 {
        return None;
    }
    let distance = -ray.origin.y / ray.direction.y;
    (distance > 0.0).then(|| ray.at(distance))
}

/// A circling path: the position at `angle` on a circle of `radius` round `center`, and the
/// rotation about Y that faces along the path (for a model whose front is +Z).
pub fn on_circle(center: Vec3, radius: f32, angle: f32) -> (Vec3, f32) {
    (center + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius), -angle)
}
