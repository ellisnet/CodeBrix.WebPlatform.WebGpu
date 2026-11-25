//! "checkpoints" (painterly P4, Not in C#): THE CHECKPOINT CACHE OF A STILL PAINTING AT WORK. A
//! LARGE stroke set (10,000 to 60,000 strokes with bristle tips, the "stroke-sets" scene's records
//! of the iris picture, painting on from left to right over about 20 s and then holding still)
//! drawn by ONE `BrushStrokesShape` whose checkpoint cache is switched on or off
//! (`with_checkpoints(Some(StrokeCheckpoints { spacing, capacity }))` or `None`). With it on, a
//! frame restores the latest checkpoint at or below its time and draws only the strokes after it,
//! so the strokes drawn per frame fall from tens of thousands to a few thousand while painting on
//! and to none once the painting holds still; seeking back restores an earlier checkpoint. The
//! picture is IDENTICAL either way (the cache's rule). The page's third stats line
//! (`demo-stats-3`) shows the RenderStats counters (strokes drawn, strokes restored, checkpoint
//! hits / bakes / misses, checkpoints held and their memory) and the smoothed frame interval.
//!
//! The scene keeps P4's rule "draw nothing that changes below a cached painting": the frame
//! (header, labels, background) is a still session drawn first, the painting second, and the
//! read-out text that changes every frame is a third session drawn ABOVE the painting. "Move the
//! painting" lays the painting into a slowly drifting rectangle: the cache then switches itself
//! off (a miss every frame, nothing baked) and comes back when it stops.

use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape};
use codebrix_webgpu_animation::strokes::{BrushStrokeSet, BrushTips, StrokeCheckpoints};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, RectangleF, SessionRenderOptions};

use crate::demo::painted::bristle_tips;
use crate::demo::stroke_sets::{records, AREA};
use crate::demo::{create_frame_session, create_session, label, Control, DemoScene, Readout, Settings, DIM, INK};

const TITLE: &str = "A still painting from its checkpoints";
const DESCRIPTION: &str = "StrokeCheckpoints: the same frame, pixel for pixel, with far fewer strokes drawn - see demo-stats-3 under the canvas";

const CONTROLS: &[Control] = &[
    Control::Toggle { key: "cache", label: "Checkpoints", default: true },
    Control::Choice { key: "count", label: "Strokes", options: &[("10,000", 10000.0), ("30,000", 30000.0), ("60,000", 60000.0)], default: 30000.0 },
    Control::Choice { key: "spacing", label: "Spacing (whole strokes between checkpoints)", options: &[("500", 500.0), ("2,000 (default)", 2000.0), ("5,000", 5000.0)], default: 2000.0 },
    Control::Choice { key: "capacity", label: "Capacity (checkpoints kept)", options: &[("1", 1.0), ("4 (default)", 4.0), ("8", 8.0)], default: 4.0 },
    Control::Toggle { key: "move", label: "Move the painting (the cache switches itself off)", default: false },
];

const LEGEND: &[&str] = &[
    "The picture is exactly the same with checkpoints on and off - compare at any time, playing or after a seek.",
    "demo-stats-3: with checkpoints on, strokes drawn per frame fall to a few thousand while painting on and to 0 once finished (strokes_restored takes the rest); checkpoint_bakes ticks about every 'spacing' whole strokes.",
    "Drag the scene-time scrubber back: checkpoint_hits with thousands of strokes restored; a drag far back costs one full frame, then cheap again.",
    "Move the painting: a miss every frame and nothing baked, the same look; the gain shows most at a device pixel ratio of 2 (frame_ms_smoothed).",
];

/// The checkpoint scene.
pub struct CheckpointsScene {
    frame: DrawingSession,
    painting: DrawingSession,
    now: DrawingSession,
    tips: BrushTips,
    settings: Settings,
    built: Option<Settings>,
    set: Option<(usize, BrushStrokeSet)>,
    refusal: Option<String>,
    seconds: f64,
    size: (u32, u32),
}

impl CheckpointsScene {
    pub fn new(fonts: &FontRegistry) -> CheckpointsScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        frame.add_layer("Note", DIM);
        let mut painting = create_session(1280, 720, Color::TRANSPARENT, fonts);
        painting.add_layer("Paint", Color::WHITE);
        let mut now = create_session(1280, 720, Color::TRANSPARENT, fonts);
        now.add_layer("Now", INK);
        CheckpointsScene { frame, painting, now, tips: bristle_tips(), settings: Settings::from_controls(CONTROLS), built: None, set: None, refusal: None, seconds: 0.0, size: (0, 0) }
    }

    /// (Re)builds the painting's one shape when a setting that shapes it changed (never per frame:
    /// the cache keys on the shape staying the same).
    fn build(&mut self) {
        let mut key = self.settings.clone();
        key.set("move", 0.0);
        if self.built.as_ref() == Some(&key) {
            return;
        }
        let count = self.settings.get("count") as usize;
        if self.set.as_ref().map(|(n, _)| *n) != Some(count) {
            self.set = Some((count, BrushStrokeSet::new(records(count)).unwrap()));
        }
        let set = self.set.as_ref().unwrap().1.clone();
        let checkpoints = self.settings.on("cache").then(|| StrokeCheckpoints { spacing: self.settings.get("spacing") as usize, capacity: self.settings.get("capacity") as usize });
        let shape = BrushStrokesShape::new(set, BrushStrokesOptions { head_softness: 0.1, ..Default::default() }).unwrap().with_tips(self.tips.clone()).with_checkpoints(checkpoints).unwrap();
        self.refusal = shape.checkpoint_refusal();
        let paint = self.painting.get_layer_mut("Paint").unwrap();
        paint.clear();
        self.painting.draw_brush_strokes(shape);
        self.built = Some(key);
    }
}

impl DemoScene for CheckpointsScene {
    fn name(&self) -> &'static str {
        "checkpoints"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        30.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.build();
        self.seconds = seconds;
        self.size = (width, height);
        // 1. THE STILL FRAME (with its background): never changes, so it never changes the key.
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        // 2. THE PAINTING (checkpointed), in place or in a slowly drifting rectangle.
        let target = self.settings.on("move").then(|| {
            let drift = (seconds * 0.7).sin() as f32 * 24.0;
            RectangleF::new(drift, 0.0, 1280.0, 720.0)
        });
        self.painting.set_time(seconds);
        self.painting.render_with(scene, width, height, &SessionRenderOptions { include_background: false, target, ..Default::default() });
        // 3. WHAT CHANGES EVERY FRAME, drawn ABOVE the painting.
        let fonts = self.now.fonts().clone();
        let started = self.set.as_ref().map_or(0, |(_, s)| s.started_count(seconds));
        let now = self.now.get_layer_mut("Now").unwrap();
        now.clear();
        label(now, &fonts, &format!("t = {seconds:.2} s, strokes started {started}"), 1248.0, 46.0, 15.0, INK, TextAlign::Right);
        self.now.set_time(seconds);
        self.now.render_with(scene, width, height, &SessionRenderOptions { include_background: false, ..Default::default() });
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
        let started = self.set.as_ref().map_or(0, |(_, s)| s.started_count(self.seconds));
        let bytes = StrokeCheckpoints::bytes_at(self.size.0, self.size.1);
        vec![
            Readout::new("records", "Strokes in the set", total),
            Readout::new("started", "Started now", started),
            Readout::new("asked", "Checkpoints asked for", if self.settings.on("cache") { format!("spacing {}, capacity {}", self.settings.get("spacing"), self.settings.get("capacity")) } else { "none".to_string() }),
            Readout::new("refusal", "Refused (checkpoint_refusal)", self.refusal.clone().unwrap_or_else(|| "no".to_string())),
            Readout::new("checkpoint-mb", "One checkpoint at this canvas size", format!("{:.1} MB ({} x {})", bytes as f64 / 1_048_576.0, self.size.0, self.size.1)),
            Readout::new("area", "Painting area", format!("{} x {} at ({}, {})", AREA.width, AREA.height, AREA.x, AREA.y)),
        ]
    }
}
