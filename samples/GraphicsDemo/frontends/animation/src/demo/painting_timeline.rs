//! "painting-timeline" (painterly P2 planning, drawn with P1's GPU stroke set; Not in C#): the
//! iris picture PAINTED AS THE CLOCK RUNS, in four stages of a `PaintingTimeline` - a broad wash of
//! big horizontal strokes, the block-in from the centre out (`StrokeOrder::Positional` with a
//! centre-out order), detail where the detail map is high from dark to light (`DarkToLight`), and
//! highlights from light to dark (`LightToDark`). The schedule makes ONE stroke set sorted by start;
//! the frame at time t is every stroke at its progress at t, so the page's scene-time scrubber can
//! drag back to any moment and shows exactly the picture that playing to that moment shows. The
//! stage starts (from `PaintingTimeline::add_cues`) are the scene's named points: a button each.

use codebrix_webgpu_animation::animation::AnimationTimeline;
use codebrix_webgpu_animation::formations::{CenterOutArrivalOrder, FormationArea};
use codebrix_webgpu_animation::painting::{DetailMap, DirectionField, DirectionGuide, PaintingStage, PaintingTimeline, PictureFieldOptions, StrokeLayout, StrokeLayoutOptions, StrokeOrder, StrokeSize};
use codebrix_webgpu_animation::paths::PathGeometry;
use codebrix_webgpu_animation::planes::DrawingPlaneImage;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape};
use codebrix_webgpu_animation::strokes::{BrushStrokeRecord, BrushStrokeSet, BrushTips};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::kit::fill_rect;
use crate::demo::painted::{bristle_tips, pick, picture};
use crate::demo::{create_frame_session, create_session, label, Control, DemoScene, Readout, Settings, DIM, INK};

const TITLE: &str = "A painting painted as the clock runs";
const DESCRIPTION: &str = "PaintingTimeline: wash, block-in (centre out), detail (dark to light), highlights (light to dark) - one stroke set; scrub the scene time to any moment";
/// Where the painting is laid (frame units).
pub const AREA: RectangleF = RectangleF { x: 32.0, y: 110.0, width: 960.0, height: 540.0 };
/// The stages: name, first and last start time, stroke seconds.
pub const STAGES: [(&str, f64, f64, (f64, f64)); 4] = [("wash", 0.0, 12.0, (0.6, 1.2)), ("block-in", 8.0, 30.0, (0.4, 0.9)), ("detail", 25.0, 50.0, (0.2, 0.5)), ("highlights", 48.0, 58.0, (0.2, 0.4))];

const CONTROLS: &[Control] = &[Control::Toggle { key: "canvas", label: "A canvas-coloured ground under the painting", default: true }];

const LEGEND: &[&str] = &[
    "A broad wash first, then regions filling from the centre out, then dark details before light ones, highlights last.",
    "Strokes visibly paint on from one tip to the other.",
    "Drag the scene-time scrubber back: the picture is exactly the earlier state (no replay artefacts) - the frame is a function of the time alone.",
    "The stage buttons beside the scrubber seek to each stage's start (the cues of PaintingTimeline::add_cues).",
];

fn luminance(c: Color) -> f32 {
    (0.2126 * c.r as f32 + 0.7152 * c.g as f32 + 0.0722 * c.b as f32) / 255.0
}

/// The planned painting: the stroke set and the cues of its stages.
pub fn plan(image: &DrawingPlaneImage) -> (BrushStrokeSet, AnimationTimeline) {
    let area = FormationArea::from_geometry(&PathGeometry::rectangle(AREA), None).unwrap();
    let colour = |x: f32, y: f32| pick(image, AREA, x, y);
    // WASH: big horizontal strokes.
    let flat = DirectionField::from_guides((8, 6), AREA, &[DirectionGuide::Fixed { angle: 0.0, center: PointF::new(AREA.x, AREA.y), reach: f32::INFINITY, weight: 1.0 }]).unwrap();
    // The wash's big strokes are centred a little inside the painting so they do not spill far past it.
    let inner = FormationArea::from_geometry(&PathGeometry::rectangle(RectangleF::new(AREA.x + 28.0, AREA.y + 10.0, AREA.width - 56.0, AREA.height - 20.0)), None).unwrap();
    let wash = StrokeLayout::fill(&inner, &flat, None, &colour, &StrokeLayoutOptions { sizes: vec![StrokeSize::new(70.0, 24.0, 26.0)], angle_jitter: 4.0, seed: 1, ..Default::default() }).unwrap().remove(0);
    // BLOCK-IN: medium strokes along the picture's structure.
    let structure = DirectionField::from_picture(image, AREA, PictureFieldOptions { field_size: Some((120, 68)), smoothing: 3.0 }).unwrap();
    let block_in = StrokeLayout::fill(&area, &structure, None, &colour, &StrokeLayoutOptions { sizes: vec![StrokeSize::new(26.0, 9.0, 11.0)], seed: 2, ..Default::default() }).unwrap().remove(0);
    // DETAIL: small strokes where the detail map is high.
    let detail_map = DetailMap::from_picture_edges(image, 1.5).unwrap();
    let detail = StrokeLayout::fill(&area, &structure, Some((&detail_map, AREA)), &colour, &StrokeLayoutOptions { sizes: vec![StrokeSize { min_detail: 0.25, variation: 0.2, ..StrokeSize::new(10.0, 3.2, 4.5) }], seed: 3, ..Default::default() }).unwrap().remove(0);
    // HIGHLIGHTS: small strokes where the picture is light.
    let highlights: Vec<BrushStrokeRecord> = StrokeLayout::fill(&area, &structure, None, &colour, &StrokeLayoutOptions { sizes: vec![StrokeSize::new(9.0, 3.0, 6.0)], seed: 4, ..Default::default() })
        .unwrap()
        .remove(0)
        .into_iter()
        .filter(|r| luminance(r.color) > 0.62)
        .collect();
    let mut painting = PaintingTimeline::new();
    for (name, first, last, seconds) in STAGES {
        let stage = PaintingStage::new(name, first, last, seconds);
        let stage = match name {
            "block-in" => stage.with_order(StrokeOrder::Positional(Box::new(CenterOutArrivalOrder::new(false)))),
            "detail" => stage.with_order(StrokeOrder::DarkToLight),
            "highlights" => stage.with_order(StrokeOrder::LightToDark),
            _ => stage,
        };
        painting.add_stage(stage).unwrap();
    }
    let tipped = |mut list: Vec<BrushStrokeRecord>| {
        for r in list.iter_mut() {
            r.tip = 1 + (r.seed % 4) as u16;
        }
        list
    };
    let set = painting.schedule(vec![tipped(wash), tipped(block_in), tipped(detail), tipped(highlights)], 11).unwrap();
    let mut cues = AnimationTimeline::new(62.0, 60.0).unwrap();
    painting.add_cues(&mut cues).unwrap();
    (set, cues)
}

pub struct PaintingTimelineScene {
    frame: DrawingSession,
    painting: DrawingSession,
    tips: BrushTips,
    planned: Option<(BrushStrokeSet, AnimationTimeline)>,
    settings: Settings,
    seconds: f64,
}

impl PaintingTimelineScene {
    pub fn new(fonts: &FontRegistry) -> PaintingTimelineScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let labels = frame.add_layer("Labels", DIM);
        label(labels, fonts, "Stages (start of the first stroke - last start):", 1010.0, 140.0, 13.0, INK, TextAlign::Left);
        for (i, (name, first, last, seconds)) in STAGES.iter().enumerate() {
            label(labels, fonts, &format!("{name}: {first} - {last} s, strokes {}-{} s", seconds.0, seconds.1), 1010.0, 166.0 + i as f32 * 22.0, 13.0, DIM, TextAlign::Left);
        }
        frame.add_layer("Now", INK);
        let mut painting = create_session(1280, 720, Color::TRANSPARENT, fonts);
        painting.add_layer("Ground", Color::WHITE);
        painting.add_layer("Paint", Color::WHITE);
        PaintingTimelineScene { frame, painting, tips: bristle_tips(), planned: None, settings: Settings::from_controls(CONTROLS), seconds: 0.0 }
    }

    fn active(&self) -> String {
        let Some((_, cues)) = self.planned.as_ref() else { return String::new() };
        let names: Vec<&str> = cues.cues().iter().filter(|c| c.is_active_at(self.seconds)).map(|c| c.name()).collect();
        if names.is_empty() { "none".to_string() } else { names.join(", ") }
    }
}

impl DemoScene for PaintingTimelineScene {
    fn name(&self) -> &'static str {
        "painting-timeline"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        62.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        if self.planned.is_none() {
            let planned = plan(&picture());
            let shape = BrushStrokesShape::new(planned.0.clone(), BrushStrokesOptions { head_softness: 0.15, ..Default::default() }).unwrap().with_tips(self.tips.clone());
            let paint = self.painting.get_layer_mut("Paint").unwrap();
            paint.add_shape(shape);
            self.planned = Some(planned);
            self.apply_settings(&self.settings.clone());
        }
        self.seconds = seconds;
        let fonts = self.frame.fonts().clone();
        let active = self.active();
        let started = self.planned.as_ref().map_or(0, |(set, _)| set.started_count(seconds));
        let now = self.frame.get_layer_mut("Now").unwrap();
        now.clear();
        label(now, &fonts, &format!("t = {seconds:.2} s"), 1010.0, 290.0, 18.0, INK, TextAlign::Left);
        label(now, &fonts, &format!("stages painting: {active}"), 1010.0, 316.0, 14.0, INK, TextAlign::Left);
        label(now, &fonts, &format!("strokes started: {started}"), 1010.0, 338.0, 14.0, DIM, TextAlign::Left);
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.painting.set_time(seconds);
        self.painting.render_with(scene, width, height, &codebrix_webgpu_animation::SessionRenderOptions { include_background: false, ..Default::default() });
    }

    fn time_points(&self) -> Vec<(String, f64)> {
        STAGES.iter().map(|(name, first, _, _)| (name.to_string(), *first)).collect()
    }

    fn controls(&self) -> &'static [Control] {
        CONTROLS
    }

    fn apply_settings(&mut self, settings: &Settings) {
        self.settings = settings.clone();
        let ground = self.painting.get_layer_mut("Ground").unwrap();
        ground.clear();
        if settings.on("canvas") {
            ground.add_shape(fill_rect(AREA.x, AREA.y, AREA.width, AREA.height, Some(Color::from_rgb(226, 214, 190)), 0.0));
        }
    }

    fn legend(&self) -> &'static [&'static str] {
        LEGEND
    }

    fn readouts(&self) -> Vec<Readout> {
        let Some((set, cues)) = self.planned.as_ref() else { return Vec::new() };
        vec![
            Readout::new("time-exact", "Scene time (exact)", format!("{}", self.seconds)),
            Readout::new("records", "Strokes in the set", set.len()),
            Readout::new("started", "Started now", set.started_count(self.seconds)),
            Readout::new("active", "Stages painting now", self.active()),
            Readout::new("cues", "Cues (add_cues)", cues.cues().iter().map(|c| format!("{} {:.1}-{:.1}", c.name(), c.start_time(), c.end_time())).collect::<Vec<_>>().join("; ")),
        ]
    }
}
