//! "stroke-sets" (painterly P1, Not in C#): the iris picture (made in code) painted by a LARGE
//! stroke set - 2,000 to 100,000 records drawn by ONE instanced GPU draw (`BrushStrokesShape`,
//! `DrawingSession::draw_brush_strokes`) with brush tips made in code (bristle ridges) - painting
//! itself on from left to right over 18 s. A switch draws the same strokes the OLD way (one
//! `BrushStrokeShape` per record, `BrushStrokeRecord::to_shape` each frame) at a count the old path
//! can bear (2,000), and the page's RenderStats line (cpu_ms, draw_calls, stroke_draws, strokes)
//! shows the gain. Head softness, tip mirroring, the tips and the look are controls.

use codebrix_webgpu_animation::images::ImageSource;
use codebrix_webgpu_animation::planes::DrawingRandom;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape, ImageShape, ImageShapeOptions};
use codebrix_webgpu_animation::strokes::{BrushStrokeRecord, BrushStrokeSet, BrushTips};
use codebrix_webgpu_animation::text::FontRegistry;
use codebrix_webgpu_animation::{Color, DrawingSession, RectangleF, SessionPaint, SessionRenderOptions};

use crate::demo::paint_override::ink;
use crate::demo::painted::{bristle_tips, jittered_strokes, picture};
use crate::demo::{create_frame_session, Control, DemoScene, Readout, Settings, DIM};

const TITLE: &str = "GPU stroke sets";
const DESCRIPTION: &str = "Tens of thousands of brushstrokes as records drawn by ONE instanced draw, painting on from left to right; switch to the old path to compare";
/// Where the painting is laid (frame units; the picture's 16:9).
pub const AREA: RectangleF = RectangleF { x: 107.0, y: 104.0, width: 1066.0, height: 600.0 };
/// How many strokes the old path draws at most.
pub const OLD_PATH_COUNT: usize = 2000;

const CONTROLS: &[Control] = &[
    Control::Choice { key: "count", label: "Strokes", options: &[("2,000", 2000.0), ("10,000", 10000.0), ("30,000", 30000.0), ("100,000", 100000.0)], default: 30000.0 },
    Control::Toggle { key: "old-path", label: "Old path (one BrushStrokeShape each, 2,000 at most)", default: false },
    Control::Toggle { key: "tips", label: "Brush tips", default: true },
    Control::Slider { key: "head", label: "Head softness", min: 0.0, max: 0.5, step: 0.05, default: 0.1 },
    Control::Toggle { key: "mirroring", label: "Tip mirroring", default: true },
    Control::Choice { key: "look", label: "Look", options: &[("as authored", 0.0), ("flat", 1.0), ("line", 2.0)], default: 0.0 },
    Control::Toggle { key: "picture", label: "The picture underneath", default: false },
];

const LEGEND: &[&str] = &[
    "The picture builds up left to right, each stroke painting on from its first tip; seeking back shows exactly the earlier state.",
    "Tips show bristle ridges, mirrored for variety; edges are smooth. stroke_draws = 1 and draw_calls stay small at any count; cpu_ms stays near 0.1.",
    "Old path (2,000 strokes): one shape each - compare draw_calls and cpu_ms (smoothed) with the GPU set at 2,000.",
    "Flat drops the ridges; line mode leaves the painting out (only the frame is drawn).",
];

pub struct StrokeSetsScene {
    frame: DrawingSession,
    painting: DrawingSession,
    set: Option<(usize, BrushStrokeSet)>,
    tips: BrushTips,
    settings: Settings,
    started: usize,
    old_drawn: usize,
}

/// The records of the scene at a count: the picture's colours, a left-to-right start sweep over
/// 18 s, 0.6 to 1.5 s each.
pub fn records(count: usize) -> Vec<BrushStrokeRecord> {
    let image = picture();
    let spacing = (AREA.width * AREA.height / count as f32).sqrt();
    let horizon = AREA.y + AREA.height * 0.42;
    let mut list = jittered_strokes(&image, AREA, count, 7, spacing * 3.6, spacing * 1.6, &|x, y| if y < horizon { 8.0 * ((x - AREA.x) / 160.0).sin() } else { -80.0 + 20.0 * ((x - AREA.x) / 50.0).sin() });
    let mut random = DrawingRandom::new(99);
    for r in list.iter_mut() {
        r.start = (r.center.x - AREA.x) / AREA.width * 18.0 + random.uniform(-0.4, 0.4) as f32;
        r.end = r.start + random.uniform(0.6, 1.5) as f32;
    }
    // Sorted by start: the GPU draws only the records up to the last one started.
    list.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    list
}

impl StrokeSetsScene {
    pub fn new(fonts: &FontRegistry) -> StrokeSetsScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        frame.add_layer("Note", DIM);
        let mut painting = crate::demo::create_session(1280, 720, Color::TRANSPARENT, fonts);
        painting.add_layer("Picture", Color::WHITE);
        painting.add_layer("Paint", Color::WHITE);
        StrokeSetsScene { frame, painting, set: None, tips: bristle_tips(), settings: Settings::from_controls(CONTROLS), started: 0, old_drawn: 0 }
    }

    fn options(&self) -> BrushStrokesOptions {
        BrushStrokesOptions { head_softness: self.settings.get("head"), tip_mirroring: self.settings.on("mirroring"), ..Default::default() }
    }
}

impl DemoScene for StrokeSetsScene {
    fn name(&self) -> &'static str {
        "stroke-sets"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        24.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let count = self.settings.get("count") as usize;
        if self.set.as_ref().map(|(n, _)| *n) != Some(count) {
            self.set = Some((count, BrushStrokeSet::new(records(count)).unwrap()));
        }
        let set = self.set.as_ref().unwrap().1.clone();
        self.started = set.started_count(seconds);
        let picture_on = self.settings.on("picture");
        let picture_layer = self.painting.get_layer_mut("Picture").unwrap();
        picture_layer.clear();
        if picture_on {
            let image = ImageSource::from_plane_image(&picture()).unwrap();
            picture_layer.add_shape(ImageShape::new(image, AREA, ImageShapeOptions { tint: Some(Color::from_rgba(255, 255, 255, 110)), ..Default::default() }).unwrap());
        }
        let old_path = self.settings.on("old-path");
        let paint = self.painting.get_layer_mut("Paint").unwrap();
        paint.clear();
        self.old_drawn = 0;
        if old_path {
            // THE OLD PATH: one BrushStrokeShape per started record, every frame.
            let step = (set.len() / OLD_PATH_COUNT).max(1);
            for r in set.records().iter().step_by(step).take(OLD_PATH_COUNT).filter(|r| r.has_started(seconds)) {
                paint.add_shape(r.to_shape(seconds).unwrap());
                self.old_drawn += 1;
            }
        }
        if !old_path {
            let mut shape = BrushStrokesShape::new(set, self.options()).unwrap();
            if self.settings.on("tips") {
                shape = shape.with_tips(self.tips.clone());
            }
            self.painting.draw_brush_strokes(shape);
        }
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.painting.set_time(seconds);
        let paint = match self.settings.get("look") as usize {
            1 => SessionPaint::Flat,
            2 => SessionPaint::Line(ink(1.5, true, false)),
            _ => SessionPaint::AsAuthored,
        };
        self.painting.render_with(scene, width, height, &SessionRenderOptions { include_background: false, paint, ..Default::default() });
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
        let total = self.set.as_ref().map_or(0, |(_, s)| s.len());
        vec![
            Readout::new("records", "Records in the set", total),
            Readout::new("started", "Started now (set.started_count)", self.started),
            Readout::new("old-drawn", "Old path: shapes drawn this frame", self.old_drawn),
        ]
    }
}
