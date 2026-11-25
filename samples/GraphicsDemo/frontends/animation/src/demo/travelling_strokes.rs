//! "travelling-strokes" (painterly P1 + P2, Not in C#): a figure built in code from vector parts
//! - a robe, a head, an arm swinging on a bound rotation, a fluttering hem - painted by one stroke
//! set of several thousand strokes placed per part (`element_area` + `StrokeLayout::fill`) and
//! ANCHORED to the moving parts (`StrokeAnchor::FollowElement`); the hem's strokes blend between
//! the robe and the hem (`anchors = [robe, hem]`, `anchor_blend` by distance from the hem's top), so
//! they stretch. Three modes: "regenerated each frame" (a new random layout every frame - the fizz),
//! "anchored" (calm) and "anchored + boil" (`StrokeBoil`, a lively hand-made look); flow
//! (`StrokeFlow`, the GPU noise turning neighbouring strokes together) is a switch.

use codebrix_webgpu_animation::animation::{EasingKind, FloatTrack, PointTrack, TrackLoopMode};
use codebrix_webgpu_animation::formations::FormationArea;
use codebrix_webgpu_animation::models::{AnimatedProperty, DrawingTransform, ElementId};
use codebrix_webgpu_animation::painting::{element_area, DirectionField, NoiseFieldOptions, StrokeLayout, StrokeLayoutOptions, StrokeSize};
use codebrix_webgpu_animation::planes::DrawingNoiseKind;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape, DrawingShape};
use codebrix_webgpu_animation::strokes::{boil_epoch, BrushStrokeRecord, BrushStrokeSet, BrushTips, StrokeAnchor, StrokeBoil, StrokeFlow};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::kit::{disc, fill_rect, polygon};
use crate::demo::painted::bristle_tips;
use crate::demo::{create_frame_session, label, Control, DemoScene, Readout, Settings, DIM, INK};

const TITLE: &str = "Strokes that travel with what they depict";
const DESCRIPTION: &str = "One stroke set anchored to moving parts (FollowElement), hem strokes blended between two anchors; boil and flow on the GPU";
const FIGURE: RectangleF = RectangleF { x: 420.0, y: 180.0, width: 460.0, height: 500.0 };

const CONTROLS: &[Control] = &[
    Control::Choice { key: "mode", label: "Mode", options: &[("regenerated each frame", 0.0), ("anchored", 1.0), ("anchored + boil", 2.0)], default: 2.0 },
    Control::Choice { key: "rate", label: "Boil rate", options: &[("0", 0.0), ("8", 8.0), ("10", 10.0), ("12", 12.0), ("24", 24.0)], default: 10.0 },
    Control::Slider { key: "offset", label: "Boil offset", min: 0.0, max: 3.0, step: 0.25, default: 1.0 },
    Control::Slider { key: "turn", label: "Boil turn", min: 0.0, max: 10.0, step: 0.5, default: 3.0 },
    Control::Toggle { key: "flow", label: "Flow", default: false },
    Control::Slider { key: "flow-turn", label: "Flow turn", min: 0.0, max: 40.0, step: 1.0, default: 30.0 },
    Control::Slider { key: "flow-speed", label: "Flow speed", min: 0.0, max: 1.0, step: 0.05, default: 0.2 },
    Control::Toggle { key: "parts", label: "Show the vector parts under the strokes", default: false },
];

const LEGEND: &[&str] = &[
    "Anchored: the strokes move rigidly with their part - no fizz. Regenerated: a new layout every frame, the fizz.",
    "The hem's strokes stretch between the robe and the fluttering hem (two anchors blended).",
    "Boil at 8-12 a second looks hand-drawn and FREEZES when the clock is paused; a seek shows the same jitter.",
    "Flow turns neighbouring strokes together, slowly.",
];

/// The parts: name, colour, the element's shape, the anchor of its rotation.
struct Parts {
    robe: ElementId,
    head: ElementId,
    arm: ElementId,
    hem: ElementId,
}

fn sway() -> PointTrack {
    let keys = [(0.0, PointF::new(-60.0, 0.0), EasingKind::SineInOut), (3.0, PointF::new(60.0, -6.0), EasingKind::SineInOut), (6.0, PointF::new(-60.0, 0.0), EasingKind::SineInOut)];
    PointTrack::from_keys(&keys).unwrap().with_loop_mode(TrackLoopMode::Loop)
}

fn swing(keys: &[(f64, f32)]) -> FloatTrack {
    let keys: Vec<(f64, f32, EasingKind)> = keys.iter().map(|(t, v)| (*t, *v, EasingKind::SineInOut)).collect();
    FloatTrack::from_keys(&keys).unwrap().with_loop_mode(TrackLoopMode::Loop)
}

/// The figure's parts as elements of the session's "Figure" layer (drawn at `opacity`), with
/// their bound movement.
fn add_parts(session: &mut DrawingSession, opacity: f32) -> Parts {
    let layer = session.get_layer_mut("Figure").unwrap();
    let robe_shape = polygon(&[PointF::new(580.0, 300.0), PointF::new(700.0, 300.0), PointF::new(760.0, 560.0), PointF::new(520.0, 560.0)], Some(Color::from_rgb(170, 40, 50)));
    let robe = layer.add_element(DrawingShape::from(robe_shape).with_opacity(opacity));
    let head = layer.add_element(DrawingShape::from(disc(640.0, 255.0, 42.0, Some(Color::from_rgb(230, 190, 150)))).with_opacity(opacity).with_transform(DrawingTransform::rotate(0.0, PointF::new(640.0, 295.0))));
    let arm = layer.add_element(DrawingShape::from(fill_rect(690.0, 305.0, 150.0, 34.0, Some(Color::from_rgb(190, 60, 60)), 16.0)).with_opacity(opacity).with_transform(DrawingTransform::rotate(0.0, PointF::new(700.0, 322.0))));
    let hem_shape = polygon(&[PointF::new(520.0, 560.0), PointF::new(760.0, 560.0), PointF::new(790.0, 640.0), PointF::new(490.0, 640.0)], Some(Color::from_rgb(220, 170, 60)));
    let hem = layer.add_element(DrawingShape::from(hem_shape).with_opacity(opacity).with_transform(DrawingTransform::rotate(0.0, PointF::new(640.0, 560.0))));
    let mut a = session.animations_mut();
    for id in [robe, head, arm, hem] {
        a.bind(id, AnimatedProperty::Position, sway()).unwrap();
    }
    a.bind(head, AnimatedProperty::Rotation, swing(&[(0.0, -6.0), (1.5, 6.0), (3.0, -6.0)])).unwrap();
    a.bind(arm, AnimatedProperty::Rotation, swing(&[(0.0, -70.0), (1.0, 15.0), (2.0, -70.0)])).unwrap();
    a.bind(hem, AnimatedProperty::Rotation, swing(&[(0.0, -7.0), (0.4, 6.0), (0.8, -7.0)])).unwrap();
    Parts { robe, head, arm, hem }
}

/// A part's colour with a little variation by place.
fn shade(base: Color, x: f32, y: f32) -> Color {
    let v = ((x * 0.11).sin() * (y * 0.07).cos() * 0.5 + 0.5) * 0.35 + 0.82;
    let c = |u: u8| ((u as f32 * v).min(255.0)) as u8;
    Color::from_rgb(c(base.r), c(base.g), c(base.b))
}

/// The strokes of every part for a layout seed: one list, each record anchored to its part.
fn layout(areas: &[(FormationArea, Color, u16)], seed: i64) -> Vec<BrushStrokeRecord> {
    let field = DirectionField::from_noise((46, 50), FIGURE, DrawingNoiseKind::Gradient, 5, 0.01, NoiseFieldOptions { turns: 0.35, base_angle: 90.0, ..Default::default() }).unwrap();
    let mut all = Vec::new();
    for (k, (area, color, anchor)) in areas.iter().enumerate() {
        let options = StrokeLayoutOptions { sizes: vec![StrokeSize::new(13.0, 4.5, 4.2)], seed: seed * 7 + k as i64, ..Default::default() };
        let base = *color;
        let bands = StrokeLayout::fill(area, &field, None, &|x, y| shade(base, x, y), &options).unwrap();
        for mut r in bands.into_iter().flatten() {
            r.tip = 1 + (r.seed % 4) as u16;
            if *anchor == 3 {
                // The hem: blended between the robe (0) and the hem (3) by the distance from its top.
                r.anchors = [0, 3];
                r.anchor_blend = ((r.center.y - 560.0) / 70.0).clamp(0.0, 1.0);
            } else {
                r.anchors = [*anchor, *anchor];
            }
            all.push(r);
        }
    }
    all
}

pub struct TravellingStrokesScene {
    frame: DrawingSession,
    fonts: FontRegistry,
    session: Option<DrawingSession>,
    areas: Vec<(FormationArea, Color, u16)>,
    tips: BrushTips,
    set: Option<BrushStrokeSet>,
    settings: Settings,
    built_epoch: i64,
    epoch: i64,
}

impl TravellingStrokesScene {
    pub fn new(fonts: &FontRegistry) -> TravellingStrokesScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let labels = frame.add_layer("Labels", DIM);
        label(labels, fonts, "robe, head, arm, hem: ELEMENTS with bound", 40.0, 200.0, 14.0, INK, TextAlign::Left);
        label(labels, fonts, "Position / Rotation tracks; the strokes", 40.0, 220.0, 14.0, INK, TextAlign::Left);
        label(labels, fonts, "follow them (StrokeAnchor::FollowElement)", 40.0, 240.0, 14.0, INK, TextAlign::Left);
        label(labels, fonts, "hem strokes: anchors [robe, hem], blend 0 -> 1", 40.0, 280.0, 13.0, DIM, TextAlign::Left);
        TravellingStrokesScene { frame, fonts: fonts.clone(), session: None, areas: Vec::new(), tips: bristle_tips(), set: None, settings: Settings::from_controls(CONTROLS), built_epoch: -1, epoch: 0 }
    }

    /// The session with the parts and the stroke shape for the current settings and set.
    fn build(&self, set: &BrushStrokeSet) -> DrawingSession {
        let mut session = crate::demo::create_session(1280, 720, Color::TRANSPARENT, &self.fonts);
        session.add_layer("Figure", Color::WHITE);
        let parts = add_parts(&mut session, if self.settings.on("parts") { 0.45 } else { 0.0 });
        let s = &self.settings;
        let boil = (s.get("mode") as usize == 2).then(|| StrokeBoil { rate: s.get("rate"), offset: s.get("offset"), turn: s.get("turn"), ..StrokeBoil::default() });
        let flow = s.on("flow").then(|| StrokeFlow { turn: s.get("flow-turn"), speed: s.get("flow-speed"), ..StrokeFlow::default() });
        let anchors = vec![StrokeAnchor::FollowElement(parts.robe), StrokeAnchor::FollowElement(parts.head), StrokeAnchor::FollowElement(parts.arm), StrokeAnchor::FollowElement(parts.hem)];
        let shape = BrushStrokesShape::new(set.clone(), BrushStrokesOptions { head_softness: 0.1, boil, flow, ..Default::default() }).unwrap().with_tips(self.tips.clone()).with_anchors(anchors).unwrap();
        session.draw_brush_strokes(shape);
        session
    }
}

impl DemoScene for TravellingStrokesScene {
    fn name(&self) -> &'static str {
        "travelling-strokes"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        16.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        if self.areas.is_empty() {
            // The parts' areas in their OWN coordinates, once (element_area of a template session).
            let mut template = crate::demo::create_session(1280, 720, Color::TRANSPARENT, &self.fonts);
            template.add_layer("Figure", Color::WHITE);
            let parts = add_parts(&mut template, 1.0);
            let list = [(parts.robe, Color::from_rgb(170, 40, 50), 0u16), (parts.head, Color::from_rgb(230, 190, 150), 1), (parts.arm, Color::from_rgb(190, 60, 60), 2), (parts.hem, Color::from_rgb(220, 170, 60), 3)];
            self.areas = list.iter().map(|(id, c, a)| (element_area(&template, *id, 0.0).unwrap(), *c, *a)).collect();
        }
        let regenerate = self.settings.get("mode") as usize == 0;
        // Regenerated: a new layout seed every 60th of a second of clock time (frozen when paused).
        self.epoch = if regenerate { boil_epoch(seconds, 60.0) } else { 0 };
        if self.set.is_none() || (regenerate && self.epoch != self.built_epoch) || self.session.is_none() {
            let set = BrushStrokeSet::new(layout(&self.areas, if regenerate { self.epoch + 1 } else { 0 })).unwrap();
            self.session = Some(self.build(&set));
            self.set = Some(set);
            self.built_epoch = self.epoch;
        }
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        let session = self.session.as_mut().unwrap();
        session.set_time(seconds);
        session.render_with(scene, width, height, &codebrix_webgpu_animation::SessionRenderOptions { include_background: false, ..Default::default() });
    }

    fn controls(&self) -> &'static [Control] {
        CONTROLS
    }

    fn apply_settings(&mut self, settings: &Settings) {
        let regenerate_changed = (settings.get("mode") as usize == 0) != (self.settings.get("mode") as usize == 0);
        self.settings = settings.clone();
        if regenerate_changed {
            self.set = None;
        }
        self.session = self.set.as_ref().map(|set| self.build(set));
    }

    fn legend(&self) -> &'static [&'static str] {
        LEGEND
    }

    fn readouts(&self) -> Vec<Readout> {
        vec![
            Readout::new("records", "Strokes in the set", self.set.as_ref().map_or(0, |s| s.len())),
            Readout::new("layout", "Layout number (changes every frame only when regenerated)", self.built_epoch),
        ]
    }
}
