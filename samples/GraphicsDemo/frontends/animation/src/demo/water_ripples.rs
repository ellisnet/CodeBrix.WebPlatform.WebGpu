//! "water-ripples" (water A1 + A2, Not in C#): a pool floor MADE IN CODE (grout, two blues of
//! tiles, a dark lane line, a ring of cream discs and a small rosette of coloured petals in the
//! middle) seen THROUGH WATER: the floor's layer carries a `WaterRefraction` whose source is a
//! `RippleField` over the pool - three seeded opening drops, seeded RAIN, a gentle SWELL and WALLS
//! (each ring bounces once off the pool's edges) - with the water's light (shading), sun GLINTS on
//! the ring slopes and CAUSTIC light gathered under the crests.
//!
//! A CLICK on the water (the page hands a primary-button press on the canvas to the scene shown)
//! drops a stone THERE, at the scene time of the click; "Drop a stone" does the same at the x / y
//! sliders. Every drop is part of the field's value, and the field is a pure function of time: drag
//! the scene time back before a stone and its ring is gone, forward again and the SAME ring is
//! there, pixel for pixel - the point of the animation-side water. A leaf floats on the water (not
//! refracted: it lies on top), lifted and pushed by the field's CPU twin (`height_at` /
//! `slope_at`), which the GPU evaluates per pixel with the same arithmetic. The read-outs show the
//! live drops, the stones, and the ripples the renderer evaluated and skipped in the last frame.

use codebrix_webgpu_animation::rendering::{RenderStats, VectorScene};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::water::{CausticLight, RippleDrop, RippleField, RippleOptions, RippleRain, RippleSwell, WaterGlints, WaterRefraction, WaterSource};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::controls::trim;
use crate::demo::kit::{disc, fill_rect, polygon};
use crate::demo::{create_frame_session, create_session, label, Control, DemoScene, Readout, Settings, DIM, FRAME_HEIGHT, FRAME_WIDTH};

const TITLE: &str = "Water ripples";
const DESCRIPTION: &str = "A pool floor made in code seen through a ripple field - rain, a swell, walls and your stones - exact under seeking";
/// The pool (frame units): the water's place, its walls, and the floor.
pub const POOL: RectangleF = RectangleF { x: 32.0, y: 104.0, width: 1216.0, height: 560.0 };
/// Where the leaf floats (frame units).
const LEAF: PointF = PointF { x: 940.0, y: 520.0 };

const CONTROLS: &[Control] = &[
    Control::Toggle { key: "water", label: "Water (refraction)", default: true },
    Control::Slider { key: "strength", label: "Refraction strength", min: 0.0, max: 30.0, step: 0.5, default: 12.0 },
    Control::Slider { key: "shading", label: "Shading", min: 0.0, max: 1.0, step: 0.05, default: 0.3 },
    Control::Slider { key: "light", label: "Light from (degrees)", min: 0.0, max: 360.0, step: 5.0, default: 225.0 },
    Control::Toggle { key: "glints", label: "Glints", default: true },
    Control::Slider { key: "glint-amount", label: "Glint amount", min: 0.0, max: 2.0, step: 0.05, default: 0.8 },
    Control::Slider { key: "sharpness", label: "Glint sharpness", min: 20.0, max: 200.0, step: 5.0, default: 80.0 },
    Control::Toggle { key: "caustics", label: "Caustic light", default: true },
    Control::Slider { key: "caustic-strength", label: "Caustic strength", min: 0.0, max: 1.5, step: 0.05, default: 0.5 },
    Control::Slider { key: "caustic-frequency", label: "Caustic frequency", min: 0.005, max: 0.05, step: 0.001, default: 0.02 },
    Control::Slider { key: "focus", label: "Caustic focus", min: 0.0, max: 2.0, step: 0.05, default: 0.5 },
    Control::Slider { key: "rain", label: "Rain (drops / s)", min: 0.0, max: 20.0, step: 0.5, default: 2.0 },
    Control::Toggle { key: "swell", label: "Swell", default: true },
    Control::Toggle { key: "walls", label: "Walls (rings bounce once)", default: true },
    Control::Slider { key: "speed", label: "Ring speed (units / s)", min: 30.0, max: 200.0, step: 5.0, default: 90.0 },
    Control::Slider { key: "decay", label: "Ring decay (s)", min: 0.5, max: 5.0, step: 0.1, default: 1.8 },
    Control::Slider { key: "wavelength", label: "Ring wavelength", min: 8.0, max: 40.0, step: 1.0, default: 16.0 },
    Control::Slider { key: "height", label: "Ring height", min: 0.25, max: 3.0, step: 0.05, default: 1.5 },
    Control::Slider { key: "drop-x", label: "Stone x (across the pool)", min: 0.0, max: 1.0, step: 0.01, default: 0.3 },
    Control::Slider { key: "drop-y", label: "Stone y (down the pool)", min: 0.0, max: 1.0, step: 0.01, default: 0.6 },
    Control::Button { key: "drop", label: "Drop a stone at x / y" },
    Control::Button { key: "clear", label: "Clear the stones" },
];

const LEGEND: &[&str] = &[
    "CLICK THE WATER to drop a stone there (or use \"Drop a stone at x / y\"): a ring opens and spreads, the tiles bend under it, it bounces once off the pool's edges (walls) and fades in about eight seconds.",
    "Rain keeps rings coming; the swell sways the whole floor gently; glints sparkle on ring slopes facing the light; a pale caustic net drifts over the tiles, brighter under crests.",
    "DRAG THE SCENE TIME BACK before a stone: its ring is gone. Forward again: the SAME ring, pixel for pixel - the water is a function of time. Pause freezes everything; a stone dropped while paused rings when the clock runs.",
    "The leaf lies on top of the water (not refracted) and rides it through the field's CPU twin. Refraction strength 0 with shading 0 and no glints or caustics draws the floor as without water (a few anti-aliased edge pixels apart).",
    "For the owner's eye (library defaults): strength 12, shading 0.3, light 225 / 45, glints 0.8 / 80, caustics 0.5 / 0.02 / focus 0.5, rings speed 90 / wavelength 16 / 3 rings / decay 1.8, rain 2 / s; the ring height is 1.5 here (library 1), so the rings read.",
];

/// The opening drops (so the pool moves from the first second): place across / down the pool,
/// time.
const OPENING: [(f32, f32, f64); 3] = [(0.22, 0.30, 0.4), (0.68, 0.55, 1.3), (0.45, 0.20, 2.4)];

/// The pool floor, made in code, on a layer: grout, tiles in two blues, a dark lane line along the
/// middle row, a ring of cream discs and a small rosette of eight coloured petals round a cream
/// heart.
pub fn add_pool_floor(layer: &mut codebrix_webgpu_animation::models::DrawingLayer, area: RectangleF) {
    layer.add_shape(fill_rect(area.x, area.y, area.width, area.height, Some(Color::from_rgb(40, 70, 95)), 0.0));
    let tile = 40.0;
    let (cols, rows) = ((area.width / tile) as i32, (area.height / tile) as i32);
    let (ox, oy) = (area.x + (area.width - cols as f32 * tile) * 0.5, area.y + (area.height - rows as f32 * tile) * 0.5);
    for j in 0..rows {
        for i in 0..cols {
            let lane = j == rows / 2 && i >= 2 && i < cols - 2;
            let c = if lane {
                Color::from_rgb(28, 52, 98)
            } else if (i + j) % 2 == 0 {
                Color::from_rgb(90, 170, 215)
            } else {
                Color::from_rgb(70, 140, 200)
            };
            layer.add_shape(fill_rect(ox + i as f32 * tile + 2.0, oy + j as f32 * tile + 2.0, tile - 4.0, tile - 4.0, Some(c), 6.0));
        }
    }
    let (cx, cy) = (area.x + area.width * 0.5, area.y + area.height * 0.5);
    for k in 0..16 {
        let a = k as f32 / 16.0 * std::f32::consts::TAU;
        layer.add_shape(disc(cx + 220.0 * a.cos(), cy + 220.0 * a.sin() * 0.95, 16.0, Some(Color::from_rgb(240, 230, 190))));
    }
    // The rosette: a gold ring, eight petals in four colours, a cream heart.
    layer.add_shape(disc(cx, cy, 118.0, Some(Color::from_rgb(204, 148, 46))));
    layer.add_shape(disc(cx, cy, 110.0, Some(Color::from_rgb(120, 175, 215))));
    let petals = [Color::from_rgb(232, 128, 160), Color::from_rgb(246, 164, 96), Color::from_rgb(154, 122, 228), Color::from_rgb(102, 204, 168)];
    for k in 0..8 {
        let a = (k as f32 + 0.5) / 8.0 * std::f32::consts::TAU;
        let (dx, dy) = (a.cos(), a.sin());
        let points: Vec<PointF> = (0..24)
            .map(|i| {
                let t = i as f32 / 24.0 * std::f32::consts::TAU;
                let (along, across) = (60.0 + 44.0 * t.cos(), 22.0 * t.sin());
                PointF::new(cx + dx * along - dy * across, cy + dy * along + dx * across)
            })
            .collect();
        layer.add_shape(polygon(&points, Some(petals[k % 4])));
    }
    layer.add_shape(disc(cx, cy, 20.0, Some(Color::from_rgb(250, 224, 168))));
}

pub struct WaterRipplesScene {
    frame: DrawingSession,
    pool: DrawingSession,
    settings: Settings,
    /// The stones dropped by a click or the button (each at the scene time it was dropped).
    stones: Vec<RippleDrop>,
    field: Option<RippleField>,
    dirty: bool,
    /// The scene time of the last frame (a click's stone falls at it).
    now: f64,
    drop_presses: f32,
    clear_presses: f32,
    stats: RenderStats,
    error: Option<String>,
}

impl WaterRipplesScene {
    pub fn new(fonts: &FontRegistry) -> WaterRipplesScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = frame.add_layer("Notes", DIM);
        label(notes, fonts, "Click the water to drop a stone. Drag the scene time back and forth: the same ripples come back.", POOL.x, POOL.bottom() + 26.0, 14.0, DIM, TextAlign::Left);
        let mut pool = create_session(FRAME_WIDTH, FRAME_HEIGHT, Color::TRANSPARENT, fonts);
        add_pool_floor(pool.add_layer("Floor", Color::WHITE), POOL);
        pool.add_layer("Surface", Color::WHITE);
        WaterRipplesScene { frame, pool, settings: Settings::from_controls(CONTROLS), stones: Vec::new(), field: None, dirty: true, now: 0.0, drop_presses: 0.0, clear_presses: 0.0, stats: RenderStats::default(), error: None }
    }

    /// The field the controls and the stones give.
    fn build_field(&self) -> Result<RippleField, String> {
        let s = &self.settings;
        let options = RippleOptions { speed: s.get("speed"), wavelength: s.get("wavelength"), decay: s.get("decay"), height: s.get("height"), walls: s.on("walls"), ..RippleOptions::default() };
        let opening = OPENING.iter().map(|&(u, v, t)| RippleDrop { center: PointF::new(POOL.x + u * POOL.width, POOL.y + v * POOL.height), time: t, strength: 1.0, size: 1.2 }).collect();
        let mut field = RippleField::new(POOL, options)?.with_drops(opening)?.with_drops(self.stones.clone())?;
        if s.get("rain") > 0.0 {
            field = field.with_rain(RippleRain::new(s.get("rain"), 1))?;
        }
        if s.on("swell") {
            field = field.with_swell(RippleSwell::gentle(4, 160.0, 0.6))?;
        }
        Ok(field)
    }

    /// The refraction the controls give, over `field`.
    fn refraction(&self, field: &RippleField) -> WaterRefraction {
        let s = &self.settings;
        WaterRefraction {
            strength: s.get("strength"),
            shading: s.get("shading"),
            light_angle: s.get("light"),
            glints: s.on("glints").then(|| WaterGlints { amount: s.get("glint-amount"), sharpness: s.get("sharpness"), ..WaterGlints::default() }),
            caustics: s.on("caustics").then(|| CausticLight { strength: s.get("caustic-strength"), frequency: s.get("caustic-frequency"), focus: s.get("focus"), ..CausticLight::default() }),
            ..WaterRefraction::new(WaterSource::Ripples(field.clone()))
        }
    }

    fn drop_stone(&mut self, x: f32, y: f32) {
        self.stones.push(RippleDrop { center: PointF::new(x, y), time: self.now, strength: 1.2, size: 1.2 });
        self.dirty = true;
    }
}

impl DemoScene for WaterRipplesScene {
    fn name(&self) -> &'static str {
        "water-ripples"
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
        self.now = seconds;
        if self.dirty {
            self.dirty = false;
            self.error = None;
            match self.build_field() {
                Ok(field) => {
                    let refraction = self.settings.on("water").then(|| self.refraction(&field));
                    if let Err(error) = self.pool.get_layer_mut("Floor").expect("made in new").set_refraction(refraction) {
                        self.error = Some(error);
                    }
                    self.field = Some(field);
                }
                Err(error) => self.error = Some(error),
            }
        }
        // The leaf rides the water (the field's CPU twin, the same arithmetic as the GPU's).
        let (lift, (sx, sy)) = self.field.as_ref().map_or((0.0, (0.0, 0.0)), |f| (f.height_at(LEAF.x, LEAF.y, seconds), f.slope_at(LEAF.x, LEAF.y, seconds)));
        let (x, y) = (LEAF.x + sx * 40.0, LEAF.y + sy * 40.0 - lift * 3.0);
        let turn = (sx * 3.0).clamp(-0.6, 0.6);
        let leaf: Vec<PointF> = (0..20)
            .map(|i| {
                let t = i as f32 / 20.0 * std::f32::consts::TAU;
                let (u, v) = (26.0 * t.cos(), 11.0 * t.sin() * (1.0 - 0.4 * t.cos()));
                PointF::new(x + u * turn.cos() - v * turn.sin(), y + u * turn.sin() + v * turn.cos())
            })
            .collect();
        let surface = self.pool.get_layer_mut("Surface").expect("made in new");
        surface.clear();
        surface.add_shape(polygon(&leaf.iter().map(|p| PointF::new(p.x + 5.0, p.y + 7.0)).collect::<Vec<_>>(), Some(Color::from_rgba(10, 30, 50, 90))));
        surface.add_shape(polygon(&leaf, Some(Color::from_rgb(96, 150, 60))));
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.pool.set_time(seconds);
        self.pool.render(scene, width, height);
    }

    fn controls(&self) -> &'static [Control] {
        CONTROLS
    }

    fn apply_settings(&mut self, settings: &Settings) {
        if settings.get("drop") != self.drop_presses {
            self.drop_presses = settings.get("drop");
            let (x, y) = (POOL.x + settings.get("drop-x") * POOL.width, POOL.y + settings.get("drop-y") * POOL.height);
            self.drop_stone(x, y);
        }
        if settings.get("clear") != self.clear_presses {
            self.clear_presses = settings.get("clear");
            self.stones.clear();
        }
        self.settings = settings.clone();
        self.dirty = true;
    }

    fn legend(&self) -> &'static [&'static str] {
        LEGEND
    }

    fn pointer_down(&mut self, x: f32, y: f32) -> bool {
        let inside = x >= POOL.x && x <= POOL.right() && y >= POOL.y && y <= POOL.bottom();
        if inside {
            self.drop_stone(x, y);
        }
        inside
    }

    fn see_stats(&mut self, stats: &RenderStats) {
        self.stats = *stats;
    }

    fn readouts(&self) -> Vec<Readout> {
        let live = self.field.as_ref().map_or(0, |f| f.live_drops(self.now).len());
        let (lift, (sx, sy)) = self.field.as_ref().map_or((0.0, (0.0, 0.0)), |f| (f.height_at(LEAF.x, LEAF.y, self.now), f.slope_at(LEAF.x, LEAF.y, self.now)));
        let stones = if self.stones.is_empty() {
            "none".to_string()
        } else {
            self.stones.iter().map(|d| format!("({:.0}, {:.0}) at {:.2} s", d.center.x, d.center.y, d.time)).collect::<Vec<_>>().join(", ")
        };
        vec![
            Readout::new("time-exact", "Scene time (exact)", self.now),
            Readout::new("live-drops", "Live drops now (RippleField::live_drops)", live),
            Readout::new("ripples-now", "Ripples evaluated / skipped by the renderer in the last frame", format!("{} / {}", self.stats.ripples_evaluated, self.stats.ripples_skipped)),
            Readout::new("stones", "Stones dropped (click or button)", stones),
            Readout::new("leaf", "The leaf's water (CPU twin): height, slope", format!("{lift:.4}, ({sx:.4}, {sy:.4})")),
            Readout::new("drop-life", "A ring lasts (s)", self.field.as_ref().map_or("-".to_string(), |f| trim(f.drop_life() as f32))),
            Readout::new("water-errors", "Water refused", self.error.clone().unwrap_or_else(|| "no".to_string())),
        ]
    }
}
