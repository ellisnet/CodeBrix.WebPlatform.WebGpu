//! "simulated-pond-2d" (water A2 + the graphics crate's wave field, ruling O7; Not in C#): the
//! pool floor of "water-ripples" (made in code) seen through water that is NOT a function of time
//! but a SIMULATION - the graphics crate's `WaveField` (a wave equation stepped on the GPU, walls
//! that reflect, seeded rain), stirred by the POINTER (a press drops, a drag draws a wake:
//! `WaveField::stir`). Its picture (R height, G/B slopes) is handed to the animation crate's
//! refraction as an application texture: `WaterSource::Texture { picture: ExternalPicture { view:
//! field.surface().view(), .. }, .. }`.
//!
//! The field is stepped by a canvas layer BELOW the catalogue's layer (so the refraction reads THIS
//! frame's water), only while this scene is shown, by the clock's delta: pausing stops it, but
//! seeking does NOT take it back - THIS SCENE CANNOT BE REWOUND (it is a simulation; the legend
//! says so plainly). The ripple field of "water-ripples" is the exact, scrubbable kind.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use codebrix_webgpu::glam::Vec2;
use codebrix_webgpu::{CanvasFrame, CanvasInput, CanvasRenderer, CanvasTarget, GpuContext, RendererHandle};
use codebrix_webgpu_animation::strokes::{ExternalPicture, PictureEncoding};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::water::{CausticLight, WaterGlints, WaterRefraction, WaterSource};
use codebrix_webgpu_animation::{Color, DrawingSession};
use codebrix_webgpu_graphics::{GraphicsContext, WaveEdge, WaveField, WaveFieldOptions, WaveRain, WaveStir};

use crate::demo::water_ripples::{add_pool_floor, POOL};
use crate::demo::{create_frame_session, create_session, label, Control, DemoScene, Readout, Settings, DIM, FRAME_HEIGHT, FRAME_WIDTH};

const TITLE: &str = "Simulated pond (cannot be rewound)";
const DESCRIPTION: &str = "The graphics crate's simulated wave field, stirred by the pointer, refracting a pool floor through the animation crate's water";
/// Frame units per field unit (the field is 12.16 x 5.6 units over the 1216 x 560 pool).
const UNITS: f32 = 100.0;
/// The field's grid.
const GRID: (u32, u32) = (304, 140);

const CONTROLS: &[Control] = &[
    Control::Slider { key: "rain", label: "Rain (drops / simulated s)", min: 0.0, max: 10.0, step: 0.25, default: 1.5 },
    Control::Slider { key: "strength", label: "Refraction strength", min: 0.0, max: 30.0, step: 0.5, default: 12.0 },
    Control::Slider { key: "height", label: "Height (water units per field unit)", min: 20.0, max: 300.0, step: 10.0, default: 100.0 },
    Control::Toggle { key: "glints", label: "Glints", default: true },
    Control::Toggle { key: "caustics", label: "Caustic light", default: true },
];

const LEGEND: &[&str] = &[
    "THIS ONE CANNOT BE SCRUBBED BACK: the water is a SIMULATION (the graphics crate's WaveField). Dragging the scene time back does not bring earlier water; pausing stops it.",
    "Press on the water to drop, drag to draw a wake: rings spread at the field's wave speed, bounce off the pool's edges again and again (the walls reflect) and die down; light rain keeps it alive.",
    "The floor is drawn by the animation crate and bent by the field's picture handed over as an application texture (WaterSource::Texture). Compare \"water-ripples\": exact in time, scrubbable.",
];

/// What the field's layer (below the catalogue's) and the scene share.
struct Shared {
    want: Cell<bool>,
    graphics: RefCell<Option<GraphicsContext>>,
    field: RefCell<Option<WaveField>>,
    picture: RefCell<Option<ExternalPicture>>,
    events: RefCell<Vec<(CanvasInput, Option<Vec2>)>>,
    rain: Cell<f32>,
    last_time: Cell<Option<f64>>,
    errors: RefCell<Vec<String>>,
}

thread_local! {
    static SHARED: Rc<Shared> = Rc::new(Shared {
        want: Cell::new(false),
        graphics: RefCell::new(None),
        field: RefCell::new(None),
        picture: RefCell::new(None),
        events: RefCell::new(Vec::new()),
        rain: Cell::new(1.5),
        last_time: Cell::new(None),
        errors: RefCell::new(Vec::new()),
    });
}

fn shared() -> Rc<Shared> {
    SHARED.with(|s| s.clone())
}

/// The canvas layer that steps the field (below the catalogue's layer); it draws nothing on the
/// canvas.
pub fn source_layer() -> RendererHandle {
    RendererHandle::new(FieldLayer { shared: shared() })
}

struct FieldLayer {
    shared: Rc<Shared>,
}

impl CanvasRenderer for FieldLayer {
    fn prepare(&mut self, context: &GpuContext, _target: &CanvasTarget) {
        *self.shared.graphics.borrow_mut() = Some(GraphicsContext::wrap_gpu_context(context));
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        if !self.shared.want.replace(false) {
            self.shared.last_time.set(None);
            return;
        }
        let Some(graphics) = self.shared.graphics.borrow().clone() else { return };
        let mut field_slot = self.shared.field.borrow_mut();
        if field_slot.is_none() {
            let options = WaveFieldOptions { columns: GRID.0, rows: GRID.1, size: Vec2::new(POOL.width / UNITS, POOL.height / UNITS), wave_speed: 1.6, damping: 0.6, edge: WaveEdge::Reflect, steps_per_second: 240.0, max_steps_per_frame: 8, label: Some("simulated pond".to_string()) };
            match graphics.create_wave_field(&options) {
                Ok(field) => {
                    *self.shared.picture.borrow_mut() = Some(ExternalPicture { view: field.surface().view().clone(), size: GRID, encoding: PictureEncoding::Linear });
                    *field_slot = Some(field);
                }
                Err(error) => {
                    self.shared.errors.borrow_mut().push(error.to_string());
                    return;
                }
            }
        }
        let Some(field) = field_slot.as_mut() else { return };
        let rain = self.shared.rain.get();
        field.set_rain((rain > 0.0).then_some(WaveRain { rate: rain, seed: 7, ..WaveRain::default() }));
        for (input, point) in self.shared.events.borrow_mut().drain(..) {
            field.stir(&input, &WaveStir::default(), &|_, _| point);
        }
        // The clock's delta: 0 while paused; a seek back is no step back (a simulation cannot be
        // rewound), a seek forward at most the field's steps per frame.
        let time = frame.info.time;
        let delta = self.shared.last_time.replace(Some(time)).map_or(0.0, |last| (time - last).clamp(0.0, 0.1));
        let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
        if let Err(error) = field.update(&mut graphics_frame, delta) {
            let mut errors = self.shared.errors.borrow_mut();
            if errors.len() < 10 {
                errors.push(error.to_string());
            }
        }
    }
}

pub struct SimulatedPondScene {
    frame: DrawingSession,
    pool: DrawingSession,
    settings: Settings,
    applied: Option<(Settings, bool)>,
    shared: Rc<Shared>,
    error: Option<String>,
}

impl SimulatedPondScene {
    pub fn new(fonts: &FontRegistry) -> SimulatedPondScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = frame.add_layer("Notes", DIM);
        label(notes, fonts, "A simulation: press or drag on the water to stir it. It cannot be rewound - the scene time slider does not take it back.", POOL.x, POOL.bottom() + 26.0, 14.0, DIM, TextAlign::Left);
        let mut pool = create_session(FRAME_WIDTH, FRAME_HEIGHT, Color::TRANSPARENT, fonts);
        add_pool_floor(pool.add_layer("Floor", Color::WHITE), POOL);
        SimulatedPondScene { frame, pool, settings: Settings::from_controls(CONTROLS), applied: None, shared: shared(), error: None }
    }
}

impl DemoScene for SimulatedPondScene {
    fn name(&self) -> &'static str {
        "simulated-pond-2d"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        20.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        // The field's layer steps the field from the next frame on, while this scene is shown.
        self.shared.want.set(true);
        self.shared.rain.set(self.settings.get("rain"));
        let picture = self.shared.picture.borrow().clone();
        let state = (self.settings.clone(), picture.is_some());
        if self.applied.as_ref() != Some(&state) {
            let s = &self.settings;
            let refraction = picture.map(|picture| WaterRefraction {
                strength: s.get("strength"),
                glints: s.on("glints").then(WaterGlints::default),
                caustics: s.on("caustics").then(CausticLight::default),
                ..WaterRefraction::new(WaterSource::Texture { picture, place: POOL, height: s.get("height") })
            });
            self.error = self.pool.get_layer_mut("Floor").expect("made in new").set_refraction(refraction).err();
            self.applied = Some(state);
        }
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.pool.set_time(seconds);
        self.pool.render(scene, width, height);
    }

    fn controls(&self) -> &'static [Control] {
        CONTROLS
    }

    fn apply_settings(&mut self, settings: &Settings) {
        self.settings = settings.clone();
    }

    fn legend(&self) -> &'static [&'static str] {
        LEGEND
    }

    fn pointer_input(&mut self, input: &CanvasInput, to_frame: &dyn Fn(f64, f64) -> (f32, f32)) {
        let (x, y) = match input {
            CanvasInput::PointerDown { x, y, .. } | CanvasInput::PointerMove { x, y, .. } | CanvasInput::PointerUp { x, y, .. } => (*x, *y),
            _ => return,
        };
        let (fx, fy) = to_frame(x, y);
        let inside = fx >= POOL.x && fx <= POOL.right() && fy >= POOL.y && fy <= POOL.bottom();
        let point = inside.then(|| Vec2::new((fx - POOL.x) / UNITS, (fy - POOL.y) / UNITS));
        self.shared.events.borrow_mut().push((input.clone(), point));
    }

    fn readouts(&self) -> Vec<Readout> {
        let field = self.shared.field.borrow();
        let simulated = field.as_ref().map_or("not yet made".to_string(), |f| format!("{:.2} s", f.simulated_time()));
        let errors = self.shared.errors.borrow();
        let refused = match (&self.error, errors.is_empty()) {
            (Some(e), _) => e.clone(),
            (None, false) => errors.join(" | "),
            (None, true) => "no".to_string(),
        };
        vec![
            Readout::new("simulated", "Simulated time (the field's own; scrubbing does not move it back)", simulated),
            Readout::new("grid", "Field", format!("{} x {} cells over the pool, walls reflect", GRID.0, GRID.1)),
            Readout::new("sim-errors", "Refused", refused),
        ]
    }
}
