//! "stroke-transition" (painterly P4, Not in C#; JUDGED BY EYE): a STROKE-LEVEL TRANSITION between
//! two paintings made in code from two different pictures - the iris field at noon and a shore at
//! dusk (painted.rs) - each laid out as a grid of brushstrokes taking its picture's colours, with
//! bristle tips. `BrushStrokeMorph::pair` pairs the strokes of the two sets (greedy, within a
//! search radius, colour weighed in); `BrushStrokesShape::morph` draws every slot in one draw at
//! its own fraction: paired strokes slide, turn (never more than 90 degrees) and change colour in
//! Oklab, unpaired ones paint themselves off or on; a SPREAD sends the change across the picture
//! as a wave (left to right, top to bottom or from the centre out).
//!
//! The fraction is on a slider, or runs on the scene's clock (`AnimatedProperty::MorphFraction`
//! bound to a track: 0 at 1 s, 1 at 7 s, held, back to 0 at 15 s) - a function of time, so the
//! scrubber plays it backwards exactly. "Show" draws either painting alone (fraction 0 and 1 must
//! equal them exactly) or a plain cross-dissolve of the two for comparison.

use codebrix_webgpu_animation::animation::{EasingKind, FloatTrack};
use codebrix_webgpu_animation::formations::{CenterOutArrivalOrder, DirectionalArrivalOrder, IArrivalOrder};
use codebrix_webgpu_animation::models::AnimatedProperty;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape};
use codebrix_webgpu_animation::strokes::{BrushStrokeMorph, BrushStrokeRecord, BrushStrokeSet, BrushTips, StrokePairingOptions};
use codebrix_webgpu_animation::text::FontRegistry;
use codebrix_webgpu_animation::{Color, DrawingSession, SessionRenderOptions};

use crate::demo::painted::{bristle_tips, dusk_picture, jittered_strokes, picture};
use crate::demo::stroke_sets::AREA;
use crate::demo::{create_frame_session, create_session, Control, DemoScene, Readout, Settings, DIM};

const TITLE: &str = "Stroke-level transition";
const DESCRIPTION: &str = "BrushStrokeMorph: the strokes of one painting slide, turn and change colour into another's; unpaired strokes paint off and on";

const CONTROLS: &[Control] = &[
    Control::Choice { key: "mode", label: "Fraction", options: &[("on the clock (0 at 1 s, 1 at 7 s, back at 15 s)", 0.0), ("on the slider", 1.0)], default: 0.0 },
    Control::Slider { key: "fraction", label: "Fraction (slider)", min: 0.0, max: 1.0, step: 0.01, default: 0.5 },
    Control::Choice { key: "show", label: "Show", options: &[("the transition", 0.0), ("the first painting alone", 1.0), ("the second painting alone", 2.0), ("a cross-dissolve, for comparison", 3.0)], default: 0.0 },
    Control::Choice { key: "spread", label: "Spread", options: &[("0 (all at once)", 0.0), ("0.4", 0.4), ("0.8", 0.8)], default: 0.4 },
    Control::Choice { key: "sweep", label: "Sweep", options: &[("left to right", 0.0), ("top to bottom", 1.0), ("from the centre out", 2.0)], default: 0.0 },
    Control::Choice { key: "radius", label: "Search radius", options: &[("20", 20.0), ("40 (default)", 40.0), ("80", 80.0)], default: 40.0 },
    Control::Choice { key: "colour-weight", label: "Colour weight", options: &[("0", 0.0), ("0.5 (default)", 0.5), ("2", 2.0)], default: 0.5 },
    Control::Choice { key: "counts", label: "Stroke counts", options: &[("8,000 and 8,000", 0.0), ("8,000 and 7,000", 1.0), ("8,000 and 2,500 (many paint off)", 2.0)], default: 1.0 },
    Control::Toggle { key: "tips", label: "Brush tips", default: true },
];

const LEGEND: &[&str] = &[
    "At 0 and 1 the canvas is exactly each painting (compare with 'Show: ... alone').",
    "Between, strokes slide, turn by at most 90 degrees and change colour without passing through grey; with a spread a wave of change sweeps across.",
    "Unpaired strokes paint themselves off and on from their first tip ('8,000 and 2,500' shows many).",
    "Scrubbing back plays it backwards exactly. The open question for the eye: does it read as painting, or as particles?",
];

/// The two paintings' records at `count` strokes each.
fn noon(count: usize) -> Vec<BrushStrokeRecord> {
    let spacing = (AREA.width * AREA.height / count as f32).sqrt();
    let horizon = AREA.y + AREA.height * 0.42;
    jittered_strokes(&picture(), AREA, count, 7, spacing * 3.4, spacing * 1.5, &|x, y| if y < horizon { 8.0 * ((x - AREA.x) / 160.0).sin() } else { -80.0 + 20.0 * ((x - AREA.x) / 50.0).sin() })
}

fn dusk(count: usize) -> Vec<BrushStrokeRecord> {
    let spacing = (AREA.width * AREA.height / count as f32).sqrt();
    let horizon = AREA.y + AREA.height * 0.58;
    jittered_strokes(&dusk_picture(), AREA, count, 8, spacing * 3.4, spacing * 1.5, &|x, y| if x < AREA.x + AREA.width * 0.42 && y > horizon - 70.0 && y < horizon + 10.0 { -90.0 } else { 3.0 * ((y - AREA.y) / 90.0).sin() })
}

/// The stroke transition scene.
pub struct StrokeTransitionScene {
    frame: DrawingSession,
    painting: DrawingSession,
    second: DrawingSession,
    tips: BrushTips,
    settings: Settings,
    built: Option<Settings>,
    sets: Option<(f32, BrushStrokeSet, BrushStrokeSet)>,
    morph: Option<(Settings, BrushStrokeMorph)>,
    dissolve: f32,
}

impl StrokeTransitionScene {
    pub fn new(fonts: &FontRegistry) -> StrokeTransitionScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        frame.add_layer("Note", DIM);
        let mut painting = create_session(1280, 720, Color::TRANSPARENT, fonts);
        painting.add_layer("Paint", Color::WHITE);
        let mut second = create_session(1280, 720, Color::TRANSPARENT, fonts);
        second.add_layer("Paint", Color::WHITE);
        StrokeTransitionScene { frame, painting, second, tips: bristle_tips(), settings: Settings::from_controls(CONTROLS), built: None, sets: None, morph: None, dissolve: 0.0 }
    }

    fn options(&self) -> BrushStrokesOptions {
        BrushStrokesOptions { head_softness: 0.1, ..Default::default() }
    }

    fn tipped(&self, shape: BrushStrokesShape) -> BrushStrokesShape {
        if self.settings.on("tips") { shape.with_tips(self.tips.clone()) } else { shape }
    }

    /// The fraction track of the clock mode.
    fn track() -> FloatTrack {
        FloatTrack::from_keys(&[(1.0, 0.0, EasingKind::Linear), (7.0, 1.0, EasingKind::CubicInOut), (9.0, 1.0, EasingKind::Linear), (15.0, 0.0, EasingKind::CubicInOut)]).unwrap()
    }

    /// The fraction at a scene time, as the bound track gives it (for the cross-dissolve and the
    /// read-out; the transition itself takes it from its binding).
    fn fraction_at(&self, seconds: f64) -> f32 {
        if self.settings.get("mode") >= 0.5 {
            return self.settings.get("fraction");
        }
        let ease = |t: f64| if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 };
        let f = if seconds <= 1.0 {
            0.0
        } else if seconds < 7.0 {
            ease((seconds - 1.0) / 6.0)
        } else if seconds < 9.0 {
            1.0
        } else if seconds < 15.0 {
            1.0 - ease((seconds - 9.0) / 6.0)
        } else {
            0.0
        };
        f as f32
    }

    /// Rebuilds the drawn shape when a setting changed (pairs again only when the pairing's own
    /// settings changed).
    fn build(&mut self) {
        let mut key = self.settings.clone();
        if self.settings.get("mode") < 0.5 {
            key.set("fraction", 0.0);
        }
        if self.built.as_ref() == Some(&key) {
            return;
        }
        let counts = self.settings.get("counts");
        if self.sets.as_ref().map(|(c, _, _)| *c) != Some(counts) {
            let to = match counts as u32 {
                0 => 8000,
                2 => 2500,
                _ => 7000,
            };
            self.sets = Some((counts, BrushStrokeSet::new(noon(8000)).unwrap(), BrushStrokeSet::new(dusk(to)).unwrap()));
            self.morph = None;
        }
        let (_, from, to) = self.sets.clone().unwrap();
        let mut pairing = self.settings.clone();
        for k in ["mode", "fraction", "show", "spread", "sweep", "tips"] {
            pairing.set(k, 0.0);
        }
        if self.morph.as_ref().map(|(p, _)| p) != Some(&pairing) {
            let options = StrokePairingOptions { search_radius: self.settings.get("radius"), color_weight: self.settings.get("colour-weight"), seed: 3 };
            self.morph = Some((pairing, BrushStrokeMorph::pair(&from, &to, options).unwrap()));
        }
        let morph = self.morph.as_ref().unwrap().1.clone();
        let paint = self.painting.get_layer_mut("Paint").unwrap();
        paint.clear();
        let second = self.second.get_layer_mut("Paint").unwrap();
        second.clear();
        self.painting.animations_mut().clear();
        match self.settings.get("show") as u32 {
            1 => {
                let shape = self.tipped(BrushStrokesShape::new(from, self.options()).unwrap());
                self.painting.draw_brush_strokes(shape);
            }
            2 => {
                let shape = self.tipped(BrushStrokesShape::new(to, self.options()).unwrap());
                self.painting.draw_brush_strokes(shape);
            }
            3 => {
                let first = self.tipped(BrushStrokesShape::new(from, self.options()).unwrap());
                self.painting.draw_brush_strokes(first);
                let other = self.tipped(BrushStrokesShape::new(to, self.options()).unwrap());
                self.second.draw_brush_strokes(other);
            }
            _ => {
                let spread_amount = self.settings.get("spread");
                let spread: Option<(Box<dyn IArrivalOrder>, f32)> = if spread_amount > 0.0 {
                    let order: Box<dyn IArrivalOrder> = match self.settings.get("sweep") as u32 {
                        1 => Box::new(DirectionalArrivalOrder::new(90.0, 0.0).unwrap()),
                        2 => Box::new(CenterOutArrivalOrder::new(false)),
                        _ => Box::new(DirectionalArrivalOrder::new(0.0, 0.0).unwrap()),
                    };
                    Some((order, spread_amount))
                } else {
                    None
                };
                let mut shape = self.tipped(BrushStrokesShape::morph(morph, self.options(), spread).unwrap());
                let slider = self.settings.get("mode") >= 0.5;
                if slider {
                    shape = shape.with_morph_fraction(self.settings.get("fraction"));
                }
                let id = self.painting.draw_brush_strokes(shape);
                if !slider {
                    self.painting.animations_mut().bind(id, AnimatedProperty::MorphFraction, StrokeTransitionScene::track()).unwrap();
                }
            }
        }
        self.built = Some(key);
    }
}

impl DemoScene for StrokeTransitionScene {
    fn name(&self) -> &'static str {
        "stroke-transition"
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
        self.build();
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.painting.set_time(seconds);
        self.painting.render_with(scene, width, height, &SessionRenderOptions { include_background: false, ..Default::default() });
        self.dissolve = self.fraction_at(seconds);
        if self.settings.get("show") as u32 == 3 {
            self.second.set_time(seconds);
            self.second.render_with(scene, width, height, &SessionRenderOptions { include_background: false, opacity: self.dissolve, ..Default::default() });
        }
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

    fn time_points(&self) -> Vec<(String, f64)> {
        vec![("first".to_string(), 0.5), ("halfway".to_string(), 4.0), ("second".to_string(), 8.0)]
    }

    fn readouts(&self) -> Vec<Readout> {
        let Some((_, morph)) = self.morph.as_ref() else { return Vec::new() };
        let from_only = morph.pairs().iter().filter(|p| p.to.is_none()).count();
        let to_only = morph.pairs().iter().filter(|p| p.from.is_none()).count();
        vec![
            Readout::new("strokes", "Strokes: first painting, second painting", format!("{}, {}", morph.from().len(), morph.to().len())),
            Readout::new("slots", "Slots drawn", morph.len()),
            Readout::new("paired", "Paired strokes", morph.paired_count()),
            Readout::new("unpaired-from", "Unpaired in the first (paint themselves off)", from_only),
            Readout::new("unpaired-to", "Unpaired in the second (paint themselves on)", to_only),
            Readout::new("fraction-now", "Fraction now", format!("{:.3}", self.dissolve)),
        ]
    }
}
