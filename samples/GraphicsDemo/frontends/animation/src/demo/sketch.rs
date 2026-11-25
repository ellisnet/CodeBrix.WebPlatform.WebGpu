//! "sketch" (a feature of the second GO, Not in C#): THE GRID of the sketch helper's settings,
//! beside its defaults. One drawing - a small house with straight walls, sharp corners, a roof apex,
//! a round window, a door with a rounded top and a wavy ground line - roughened with
//! `PathGeometry::sketch` at 4 x 4 settings: roughness 0.75 / 1.5 / 3 / 6 (columns) by wobble
//! length 10 / 20 / 40 / 80 (rows), or roughness by passes 1 - 4 (the second grid mode). Each cell
//! prints its values, so a cell can be named. The large cell on the right uses
//! `SketchOptions::default()` - the defaults. Passes, corner angle, seed and the boil
//! rate (`SketchOptions::boiled` with the scene's clock time) are controls; a faint unsketched
//! drawing can be shown underneath.

use codebrix_webgpu_animation::paths::{PathGeometry, PathStrokeStyle, PathStrokeStyleOptions, SketchOptions};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::PathShape;
use codebrix_webgpu_animation::strokes::boil_epoch;
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession};

use crate::demo::controls::trim;
use crate::demo::kit::{fill_rect, outline_rect};
use crate::demo::{create_frame_session, label, Control, DemoScene, Readout, Settings};

const TITLE: &str = "Sketch helper: a grid of settings";
const DESCRIPTION: &str = "PathGeometry::sketch at 4 x 4 settings - each cell names its values; the large cell is SketchOptions::default()";
/// The columns' roughness.
pub const ROUGHNESS: [f32; 4] = [0.75, 1.5, 3.0, 6.0];
/// The rows' wobble length (grid mode 1).
pub const WOBBLE: [f32; 4] = [10.0, 20.0, 40.0, 80.0];
/// The rows' passes (grid mode 2).
pub const PASSES: [u32; 4] = [1, 2, 3, 4];
const CELL_W: f32 = 245.0;
const CELL_H: f32 = 150.0;
const GRID_X: f32 = 28.0;
const GRID_Y: f32 = 104.0;
const PAPER: Color = Color::from_rgb(248, 246, 238);
const INK: Color = Color::from_rgb(34, 32, 44);
const NOTE: Color = Color::from_rgb(110, 104, 120);

const CONTROLS: &[Control] = &[
    Control::Choice { key: "mode", label: "Grid", options: &[("roughness x wobble length", 0.0), ("roughness x passes", 1.0)], default: 0.0 },
    Control::Choice { key: "passes", label: "Passes (grid 1)", options: &[("1", 1.0), ("2", 2.0), ("3", 3.0)], default: 2.0 },
    Control::Choice { key: "wobble", label: "Wobble length (grid 2)", options: &[("10", 10.0), ("20", 20.0), ("30", 30.0), ("40", 40.0), ("80", 80.0)], default: 30.0 },
    Control::Choice { key: "corner", label: "Corner angle", options: &[("15", 15.0), ("30", 30.0), ("45", 45.0), ("60", 60.0)], default: 30.0 },
    Control::Slider { key: "seed", label: "Seed", min: 1.0, max: 20.0, step: 1.0, default: 1.0 },
    Control::Choice { key: "boil", label: "Boil rate (per second)", options: &[("0 (off)", 0.0), ("6", 6.0), ("8", 8.0), ("10", 10.0), ("12", 12.0)], default: 0.0 },
    Control::Toggle { key: "under", label: "Unsketched drawing underneath", default: false },
];

const LEGEND: &[&str] = &[
    "Low roughness + long wobble = a calm, slightly hand-drawn line; high roughness + short wobble = a nervous scribble.",
    "Corners stay crisp points in every cell (the roof's apex, the walls' corners); curves (window, door, ground) stay round.",
    "Two passes read as a line gone over twice. Boiling at 8-12 a second looks like a drawn film; pausing the clock freezes it; a reload at the same ?t= shows the identical line.",
    "Each cell's values are printed under it; the large cell shows the defaults (roughness 2, wobble 30, passes 2, corner angle 30).",
];

/// The house drawing at a scale `k`, its top-left near (x, y): walls, roof, round window, door with
/// a rounded top and a wavy ground line, as one path of several contours.
pub fn house(x: f32, y: f32, k: f32) -> PathGeometry {
    let p = |px: f32, py: f32| format!("{} {}", x + px * k, y + py * k);
    let c = 0.5523 * 12.0;
    let (wx, wy) = (35.0, 75.0);
    let window = format!(
        "M {} C {} {} {} C {} {} {} C {} {} {} C {} {} {} Z",
        p(wx + 12.0, wy),
        p(wx + 12.0, wy + c), p(wx + c, wy + 12.0), p(wx, wy + 12.0),
        p(wx - c, wy + 12.0), p(wx - 12.0, wy + c), p(wx - 12.0, wy),
        p(wx - 12.0, wy - c), p(wx - c, wy - 12.0), p(wx, wy - 12.0),
        p(wx + c, wy - 12.0), p(wx + 12.0, wy - c), p(wx + 12.0, wy)
    );
    let text = format!(
        "M {} L {} L {} L {} Z M {} L {} L {} Z {} M {} L {} C {} {} {} L {} M {} C {} {} {} S {} {}",
        p(0.0, 45.0), p(120.0, 45.0), p(120.0, 125.0), p(0.0, 125.0),
        p(-12.0, 45.0), p(60.0, -5.0), p(132.0, 45.0),
        window,
        p(78.0, 125.0), p(78.0, 92.0), p(78.0, 76.0), p(102.0, 76.0), p(102.0, 92.0), p(102.0, 125.0),
        p(-20.0, 132.0), p(10.0, 124.0), p(40.0, 140.0), p(70.0, 132.0), p(130.0, 124.0), p(150.0, 132.0)
    );
    PathGeometry::parse(&text).unwrap()
}

/// The settings of a grid cell (column, row) for the scene's controls.
pub fn cell_options(settings: &Settings, column: usize, row: usize) -> SketchOptions {
    let (wobble, passes) = if settings.get("mode") as usize == 1 { (settings.get("wobble"), PASSES[row]) } else { (WOBBLE[row], settings.get("passes") as u32) };
    SketchOptions { roughness: ROUGHNESS[column], wobble_length: wobble, passes, seed: settings.get("seed") as u64, corner_angle_degrees: settings.get("corner") }
}

pub struct SketchScene {
    session: DrawingSession,
    fonts: FontRegistry,
    settings: Settings,
    built: Option<(Settings, i64)>,
}

impl SketchScene {
    pub fn new(fonts: &FontRegistry) -> SketchScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        session.add_layer("Paper", PAPER);
        session.add_layer("Cells", INK);
        SketchScene { session, fonts: fonts.clone(), settings: Settings::from_controls(CONTROLS), built: None }
    }

    fn rebuild(&mut self, seconds: f64) {
        let rate = self.settings.get("boil");
        let epoch = boil_epoch(seconds, rate);
        if self.built.as_ref().is_some_and(|(s, e)| *s == self.settings && *e == epoch) {
            return;
        }
        let settings = self.settings.clone();
        let fonts = self.fonts.clone();
        let paper = self.session.get_layer_mut("Paper").unwrap();
        paper.clear();
        paper.add_shape(fill_rect(16.0, 96.0, 1248.0, 616.0, Some(PAPER), 6.0));
        let pen = PathStrokeStyle::new(2.0, PathStrokeStyleOptions::default()).unwrap();
        let cells = self.session.get_layer_mut("Cells").unwrap();
        cells.clear();
        let mode = settings.get("mode") as usize;
        for row in 0..4 {
            for column in 0..4 {
                let (x, y) = (GRID_X + column as f32 * CELL_W, GRID_Y + row as f32 * CELL_H);
                let drawing = house(x + 64.0, y + 16.0, 0.8);
                if settings.on("under") {
                    cells.add_shape(PathShape::with_style(drawing.clone(), pen.clone(), Some(Color::from_rgba(120, 150, 220, 90)), None, None));
                }
                let options = cell_options(&settings, column, row).boiled(seconds, rate);
                cells.add_shape(PathShape::with_style(drawing.sketch(&options).unwrap(), pen.clone(), Some(INK), None, None));
                let o = cell_options(&settings, column, row);
                let text = if mode == 1 { format!("roughness {} - passes {}", trim(o.roughness), o.passes) } else { format!("roughness {} - wobble {}", trim(o.roughness), trim(o.wobble_length)) };
                label(cells, &fonts, &text, x + 122.0, y + CELL_H - 6.0, 13.0, NOTE, TextAlign::Center);
            }
        }
        // The defaults, larger.
        let defaults = SketchOptions::default();
        let (x, y) = (1012.0, 150.0);
        cells.add_shape(outline_rect(x - 8.0, y - 34.0, 252.0, 300.0, 1.5, Some(Color::from_rgb(200, 120, 40)), 8.0));
        label(cells, &fonts, "THE DEFAULTS", x + 118.0, y - 12.0, 14.0, Color::from_rgb(200, 120, 40), TextAlign::Center);
        let drawing = house(x + 40.0, y + 20.0, 1.25);
        if settings.on("under") {
            cells.add_shape(PathShape::with_style(drawing.clone(), pen.clone(), Some(Color::from_rgba(120, 150, 220, 90)), None, None));
        }
        cells.add_shape(PathShape::with_style(drawing.sketch(&defaults.boiled(seconds, rate)).unwrap(), pen.clone(), Some(INK), None, None));
        label(cells, &fonts, &format!("roughness {} - wobble {}", trim(defaults.roughness), trim(defaults.wobble_length)), x + 118.0, y + 220.0, 13.0, NOTE, TextAlign::Center);
        label(cells, &fonts, &format!("passes {} - seed {} - corner {}", defaults.passes, defaults.seed, trim(defaults.corner_angle_degrees)), x + 118.0, y + 240.0, 13.0, NOTE, TextAlign::Center);
        label(cells, &fonts, "(drawn 1.25 x the grid's size)", x + 118.0, y + 260.0, 12.0, NOTE, TextAlign::Center);
        let all = format!("grid cells: passes {}, seed {}, corner {}, boil {}/s", if mode == 1 { "by row".to_string() } else { trim(settings.get("passes")) }, settings.get("seed"), settings.get("corner"), settings.get("boil"));
        label(cells, &fonts, &all, x + 118.0, y + 330.0, 12.0, NOTE, TextAlign::Center);
        label(cells, &fonts, "pen: 2.0 units, one colour", x + 118.0, y + 350.0, 12.0, NOTE, TextAlign::Center);
        self.built = Some((settings, epoch));
    }
}

impl DemoScene for SketchScene {
    fn name(&self) -> &'static str {
        "sketch"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        12.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.rebuild(seconds);
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
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

    fn readouts(&self) -> Vec<Readout> {
        let epoch = self.built.as_ref().map_or(0, |(_, e)| *e);
        vec![
            Readout::new("epoch", "Boil step (seed added)", epoch),
            Readout::new("cells", "Cell size and grid origin (frame units)", format!("{CELL_W} x {CELL_H} from {GRID_X},{GRID_Y}")),
        ]
    }
}
