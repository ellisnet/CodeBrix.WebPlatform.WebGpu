//! "surface-finish" (painterly P3, Not in C#): the iris picture (made in code) painted by a stroke
//! set of 10,000 or 30,000 records whose bristle tips CARRY HEIGHT (`BrushTip::new(coverage,
//! Some(height))`), on a layer with a SURFACE FINISH (`DrawingLayer::set_surface_finish`): the
//! canvas weave and the thickness of the paint, lit from one direction - ridges catch the light on
//! the side facing it and shade on the other, with a small sheen. Beside it, on a second finished
//! layer with NO strokes, a flat rectangle and the same picture as an image: they show only the
//! weave (thickness comes from stroke sets only).
//!
//! Controls: the finish on / off, a preset (the sliders, thick impasto, a smooth glaze), the light's
//! direction (or circling on the scene's clock) and height, relief, impasto (thickness), gloss,
//! the weave on / off, its spacing, depth and irregularity, and the stroke count. The sliders start
//! at the library's defaults of the finish.

use codebrix_webgpu_animation::effects::{CanvasWeave, SurfaceFinish};
use codebrix_webgpu_animation::images::ImageSource;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape, ImageShape, ImageShapeOptions};
use codebrix_webgpu_animation::strokes::{BrushStrokeSet, BrushTips};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, RectangleF};

use crate::demo::kit::fill_rect;
use crate::demo::painted::{bristle_tips_with_height, jittered_strokes, picture};
use crate::demo::{create_frame_session, create_session, label, Control, DemoScene, Readout, Settings, DIM, FRAME_HEIGHT, FRAME_WIDTH, INK};

const TITLE: &str = "Surface finish";
const DESCRIPTION: &str = "Canvas weave and paint thickness, lit: brush tips that carry height stand up as impasto; beside it, shapes without strokes show only the weave";
/// The painting, frame units (16:9).
const PAINTING: RectangleF = RectangleF { x: 36.0, y: 110.0, width: 864.0, height: 486.0 };
/// The flat rectangle and the picture beside it (no strokes: weave only).
const FLAT: RectangleF = RectangleF { x: 924.0, y: 110.0, width: 320.0, height: 180.0 };
const IMAGE: RectangleF = RectangleF { x: 924.0, y: 330.0, width: 320.0, height: 180.0 };

const CONTROLS: &[Control] = &[
    Control::Toggle { key: "finish", label: "Surface finish", default: true },
    Control::Choice { key: "preset", label: "Preset", options: &[("the sliders", 0.0), ("thick impasto", 1.0), ("smooth glaze", 2.0)], default: 0.0 },
    Control::Toggle { key: "circle", label: "Light circles", default: true },
    Control::Slider { key: "angle", label: "Light from (degrees)", min: 0.0, max: 360.0, step: 5.0, default: 225.0 },
    Control::Slider { key: "elevation", label: "Light height (degrees)", min: 10.0, max: 80.0, step: 1.0, default: 35.0 },
    Control::Slider { key: "relief", label: "Relief", min: 0.0, max: 1.5, step: 0.05, default: 0.6 },
    Control::Slider { key: "impasto", label: "Impasto (thickness)", min: 0.0, max: 1.5, step: 0.05, default: 1.0 },
    Control::Slider { key: "gloss", label: "Gloss", min: 0.0, max: 0.5, step: 0.01, default: 0.15 },
    Control::Toggle { key: "weave", label: "Canvas weave", default: true },
    Control::Slider { key: "spacing", label: "Weave spacing", min: 2.0, max: 8.0, step: 0.5, default: 3.0 },
    Control::Slider { key: "depth", label: "Weave depth", min: 0.0, max: 0.8, step: 0.05, default: 0.35 },
    Control::Slider { key: "irregularity", label: "Weave irregularity", min: 0.0, max: 1.0, step: 0.05, default: 0.3 },
    Control::Choice { key: "count", label: "Strokes", options: &[("10,000", 10000.0), ("30,000", 30000.0)], default: 30000.0 },
];

const LEGEND: &[&str] = &[
    "Thick paint stands up where strokes overlap; ridges of the tips catch the light on the side facing it and shade on the other, with a small sheen.",
    "Impasto 0 (or \"smooth glaze\"): the paint lies flat and only the weave shows through thin paint; \"thick impasto\" stands it up.",
    "Right: a flat rectangle and a picture on a finished layer with no strokes - weave only, no thickness.",
    "The weave stays fixed to the painting; flat areas keep their colour; the finish off gives the plain painting.",
    "The sliders start at the library's defaults (light 225 / 35, relief 0.6, impasto 1, gloss 0.15, weave 3 / 0.35 / 0.3).",
];

pub struct SurfaceFinishScene {
    frame: DrawingSession,
    painting: DrawingSession,
    set: Option<(usize, BrushStrokeSet)>,
    tips: BrushTips,
    settings: Settings,
    applied: Option<SurfaceFinish>,
    errors: Vec<String>,
}

impl SurfaceFinishScene {
    pub fn new(fonts: &FontRegistry) -> SurfaceFinishScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = frame.add_layer("Notes", DIM);
        label(notes, fonts, "the painting: a stroke set whose tips carry height", PAINTING.x, PAINTING.y + PAINTING.height + 22.0, 14.0, INK, TextAlign::Left);
        label(notes, fonts, "a flat rectangle (no strokes): weave only", FLAT.x, FLAT.y + FLAT.height + 20.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "a picture (no strokes): weave only", IMAGE.x, IMAGE.y + IMAGE.height + 20.0, 13.0, DIM, TextAlign::Left);
        let mut painting = create_session(FRAME_WIDTH, FRAME_HEIGHT, Color::TRANSPARENT, fonts);
        // The painting's layer: the canvas ground and the strokes (added in render, once per count).
        painting.add_layer("Painting", Color::WHITE);
        let weave_only = painting.add_layer("Weave only", Color::WHITE);
        weave_only.add_shape(fill_rect(FLAT.x, FLAT.y, FLAT.width, FLAT.height, Some(Color::from_rgb(200, 150, 80)), 0.0));
        let image = ImageSource::from_plane_image(&picture()).expect("the iris picture");
        weave_only.add_shape(ImageShape::new(image, IMAGE, ImageShapeOptions::default()).expect("a valid area"));
        SurfaceFinishScene { frame, painting, set: None, tips: bristle_tips_with_height(), settings: Settings::from_controls(CONTROLS), applied: None, errors: Vec::new() }
    }

    /// The finish the controls give at `seconds` (the light circles on the scene's time).
    fn finish(&self, seconds: f64) -> SurfaceFinish {
        let s = &self.settings;
        let mut finish = SurfaceFinish {
            light_angle: if s.on("circle") { ((seconds * 20.0) % 360.0) as f32 } else { s.get("angle") },
            light_elevation: s.get("elevation"),
            relief: s.get("relief"),
            impasto: s.get("impasto"),
            gloss: s.get("gloss"),
            weave: s.on("weave").then(|| CanvasWeave { spacing: s.get("spacing"), depth: s.get("depth"), irregularity: s.get("irregularity"), ..CanvasWeave::default() }),
        };
        match s.get("preset") as usize {
            1 => {
                finish.impasto = 1.5;
                finish.relief = 1.1;
                finish.gloss = 0.3;
            }
            2 => {
                finish.impasto = 0.0;
                finish.relief = 0.35;
                finish.gloss = 0.05;
                if let Some(weave) = finish.weave.as_mut() {
                    weave.depth = 0.2;
                }
            }
            _ => {}
        }
        finish
    }
}

impl DemoScene for SurfaceFinishScene {
    fn name(&self) -> &'static str {
        "surface-finish"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        18.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let count = self.settings.get("count") as usize;
        if self.set.as_ref().map(|(n, _)| *n) != Some(count) {
            let image = picture();
            let spacing = (PAINTING.width * PAINTING.height / count as f32).sqrt();
            let horizon = PAINTING.y + PAINTING.height * 0.42;
            let records = jittered_strokes(&image, PAINTING, count, 23, spacing * 3.8, spacing * 1.7, &|x, y| if y < horizon { 10.0 * ((x - PAINTING.x) / 140.0).sin() } else { -76.0 + 22.0 * ((x - PAINTING.x) / 45.0).sin() });
            let set = BrushStrokeSet::new(records).expect("valid records");
            // The canvas ground under the paint, then the strokes, on ONE layer (a finished layer is
            // lit where it drew; its stroke sets give it thickness).
            let layer = self.painting.get_layer_mut("Painting").expect("made in new");
            layer.clear();
            layer.add_shape(fill_rect(PAINTING.x, PAINTING.y, PAINTING.width, PAINTING.height, Some(Color::from_rgb(222, 212, 190)), 0.0));
            self.painting.set_active_layer("Painting");
            self.painting.draw_brush_strokes(BrushStrokesShape::new(set.clone(), BrushStrokesOptions::default()).expect("a set").with_tips(self.tips.clone()));
            self.set = Some((count, set));
        }
        let finish = self.settings.on("finish").then(|| self.finish(seconds));
        let current = finish;
        if self.applied != current {
            self.errors.clear();
            for name in ["Painting", "Weave only"] {
                if let Err(error) = self.painting.get_layer_mut(name).expect("made in new").set_surface_finish(current) {
                    self.errors.push(error);
                }
            }
            self.applied = current;
        }
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.painting.set_time(seconds);
        self.painting.render(scene, width, height);
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
        let finish = match self.applied {
            Some(f) => format!(
                "light {:.0} deg at {:.0} deg, relief {:.2}, impasto {:.2}, gloss {:.2}, weave {}",
                f.light_angle,
                f.light_elevation,
                f.relief,
                f.impasto,
                f.gloss,
                f.weave.map_or("off".to_string(), |w| format!("spacing {:.1} depth {:.2} irregularity {:.2}", w.spacing, w.depth, w.irregularity))
            ),
            None => "off".to_string(),
        };
        vec![
            Readout::new("finish-now", "Finish on the layers", finish),
            Readout::new("strokes", "Strokes in the set", self.set.as_ref().map_or(0, |(_, s)| s.len())),
            Readout::new("finish-errors", "Finish refused", if self.errors.is_empty() { "no".to_string() } else { self.errors.join(" | ") }),
        ]
    }
}
