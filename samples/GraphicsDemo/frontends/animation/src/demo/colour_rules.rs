//! "colour-rules" (painterly P2 colour, Not in C#): one gradient landscape made in code, picked
//! up by strokes FOUR ways side by side - as picked, through a limited palette of six pigments
//! (`PaintPalette`, Nearest or MixOfTwo, with a snap), with BROKEN COLOUR (`BrokenColor`: hue,
//! chroma and lightness varied stroke by stroke) and both (`ColorRules::apply` per stroke seed).
//! Below: MIXING SPACES - three bars going blue -> yellow over time, each a layer colour bound to a
//! colour as a function of the time (`BoundTrack::color_fn` with `ColorMix::mix` in sRGB, Oklab
//! and Oklch), three gradients made with `LinearGradientFill::with_interpolation` in the same
//! spaces with a `ColorRamp::with_interpolation` swatch strip under each (which must match it),
//! and the palette's pigments.

use codebrix_webgpu_animation::animation::ColorRamp;
use codebrix_webgpu_animation::fills::LinearGradientFill;
use codebrix_webgpu_animation::models::{AnimatedProperty, BindingTarget, BoundTrack};
use codebrix_webgpu_animation::painting::{BrokenColor, ColorInterpolation, ColorMix, ColorRules, PaintPalette, PaletteMode};
use codebrix_webgpu_animation::planes::DrawingPlaneImage;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape, RectangleShape};
use codebrix_webgpu_animation::strokes::{BrushStrokeSet, BrushTips};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::kit::fill_rect;
use crate::demo::painted::{bristle_tips, jittered_strokes};
use crate::demo::{create_frame_session, create_session, label, Control, DemoScene, Readout, Settings, DIM, INK};

const TITLE: &str = "Colour rules and perceptual mixing";
const DESCRIPTION: &str = "Strokes picking up a landscape as picked, through a six-pigment palette, with broken colour, and both; sRGB vs Oklab vs Oklch on bars, gradients and ramps";
/// The four stroke panels (frame units).
pub fn panel(i: usize) -> RectangleF {
    RectangleF::new(40.0 + i as f32 * 302.0, 122.0, 288.0, 162.0)
}
/// The six pigments of the palette.
pub const PIGMENTS: [Color; 6] = [Color::from_rgb(28, 36, 96), Color::from_rgb(64, 120, 200), Color::from_rgb(236, 214, 160), Color::from_rgb(214, 120, 56), Color::from_rgb(60, 110, 50), Color::from_rgb(150, 170, 70)];
const BLUE: Color = Color::from_rgb(30, 60, 230);
const YELLOW: Color = Color::from_rgb(250, 220, 30);
const SPACES: [(ColorInterpolation, &str); 3] = [(ColorInterpolation::Srgb, "sRGB"), (ColorInterpolation::Oklab, "Oklab"), (ColorInterpolation::Oklch, "Oklch")];

const CONTROLS: &[Control] = &[
    Control::Slider { key: "snap", label: "Palette snap", min: 0.0, max: 1.0, step: 0.05, default: 1.0 },
    Control::Choice { key: "mode", label: "Palette mode", options: &[("MixOfTwo", 1.0), ("Nearest", 0.0)], default: 1.0 },
    Control::Slider { key: "hue", label: "Broken hue (degrees)", min: 0.0, max: 30.0, step: 1.0, default: 12.0 },
    Control::Slider { key: "chroma", label: "Broken chroma", min: 0.0, max: 0.3, step: 0.01, default: 0.15 },
    Control::Slider { key: "lightness", label: "Broken lightness", min: 0.0, max: 0.1, step: 0.005, default: 0.03 },
];

const LEGEND: &[&str] = &[
    "The palette panel reads as painted with a limited palette: MixOfTwo keeps gradations, Nearest posterises.",
    "Broken colour vibrates stroke to stroke without changing the overall value.",
    "Bars: the sRGB bar passes through grey half way; the Oklab bar stays colourful; Oklch goes round the hue circle (through green by the shorter arc).",
    "Each ColorRamp swatch strip matches the gradient above it.",
];

/// The landscape: a sky gradient, a low sun, two rolling hills.
pub fn landscape() -> DrawingPlaneImage {
    let (w, h) = (288u32, 162u32);
    let mut image = DrawingPlaneImage::new(w, h).unwrap();
    image.fill_with(|x, y| {
        let (u, v) = (x as f32 / w as f32, y as f32 / h as f32);
        let hill1 = 0.62 + 0.08 * (u * 5.0).sin();
        let hill2 = 0.78 + 0.06 * (u * 3.0 + 1.0).cos();
        let c = if v > hill2 {
            [0.20 + 0.1 * u, 0.42, 0.16]
        } else if v > hill1 {
            [0.46, 0.58 - 0.2 * (v - hill1), 0.26]
        } else {
            let sun = (1.0 - (((u - 0.7) * 1.8).powi(2) + (v - 0.45).powi(2)).sqrt() / 0.12).clamp(0.0, 1.0);
            let t = v / hill1;
            [0.18 + 0.75 * t + 0.1 * sun, 0.32 + 0.45 * t + 0.1 * sun, 0.75 - 0.2 * t]
        };
        [c[0].min(1.0), c[1].min(1.0), c[2].min(1.0), 1.0]
    });
    image
}

fn rules(settings: &Settings, which: usize) -> ColorRules {
    let palette = PaintPalette { pigments: PIGMENTS.to_vec(), snap: settings.get("snap"), mode: if settings.get("mode") >= 0.5 { PaletteMode::MixOfTwo } else { PaletteMode::Nearest } };
    let broken = BrokenColor { hue: settings.get("hue"), chroma: settings.get("chroma"), lightness: settings.get("lightness"), seed: 1 };
    match which {
        1 => ColorRules { palette: Some(palette), broken: None },
        2 => ColorRules { palette: None, broken: Some(broken) },
        3 => ColorRules { palette: Some(palette), broken: Some(broken) },
        _ => ColorRules::default(),
    }
}

pub struct ColourRulesScene {
    session: DrawingSession,
    painting: DrawingSession,
    picture: DrawingPlaneImage,
    tips: BrushTips,
    settings: Settings,
    dirty: bool,
}

impl ColourRulesScene {
    pub fn new(fonts: &FontRegistry) -> ColourRulesScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let labels = session.add_layer("Labels", DIM);
        for (i, text) in ["as picked", "palette: six pigments", "broken colour", "palette + broken colour"].iter().enumerate() {
            let p = panel(i);
            label(labels, fonts, text, p.x, p.y + p.height + 20.0, 14.0, INK, TextAlign::Left);
        }
        label(labels, fonts, "blue -> yellow over time: BoundTrack::color_fn(ColorMix::mix(.., space))", 40.0, 352.0, 14.0, INK, TextAlign::Left);
        label(labels, fonts, "LinearGradientFill::with_interpolation, ColorRamp strip under each", 660.0, 352.0, 14.0, INK, TextAlign::Left);
        label(labels, fonts, "the palette's pigments", 40.0, 590.0, 13.0, DIM, TextAlign::Left);
        for (k, (space, name)) in SPACES.iter().enumerate() {
            let y = 372.0 + k as f32 * 64.0;
            label(labels, fonts, name, 40.0, y + 26.0, 15.0, INK, TextAlign::Left);
            label(labels, fonts, name, 660.0, y + 22.0, 15.0, INK, TextAlign::Left);
            // The gradient and its ramp strip.
            let fill = LinearGradientFill::from_colors(PointF::new(730.0, 0.0), PointF::new(1230.0, 0.0), BLUE, YELLOW).unwrap().with_interpolation(*space);
            labels.add_shape(RectangleShape::with_fill(730.0, y, 500.0, 30.0, fill.into(), 0.0).unwrap());
            let ramp = ColorRamp::from_colors(&[BLUE, YELLOW]).unwrap().with_interpolation(*space);
            for i in 0..20 {
                labels.add_shape(fill_rect(730.0 + i as f32 * 25.0, y + 34.0, 25.0, 14.0, Some(ramp.at((i as f64 + 0.5) / 20.0)), 0.0));
            }
        }
        for (i, c) in PIGMENTS.iter().enumerate() {
            labels.add_shape(fill_rect(40.0 + i as f32 * 60.0, 600.0, 52.0, 40.0, Some(*c), 4.0));
        }
        // The bars: a layer each, its colour bound to a function of the time.
        for (k, (space, name)) in SPACES.iter().enumerate() {
            let layer_name = format!("Bar {name}");
            let bar = session.add_layer(&layer_name, Color::WHITE);
            bar.add_shape(fill_rect(110.0, 372.0 + k as f32 * 64.0, 500.0, 40.0, None, 6.0));
            let space = *space;
            session
                .animations_mut()
                .bind(BindingTarget::layer(&layer_name), AnimatedProperty::Color, BoundTrack::color_fn(move |t| {
                    let phase = (t / 6.0).rem_euclid(1.0) as f32;
                    let wave = 1.0 - (2.0 * phase - 1.0).abs();
                    ColorMix::mix(BLUE, YELLOW, wave, space)
                }))
                .unwrap();
        }
        let mut painting = create_session(1280, 720, Color::TRANSPARENT, fonts);
        painting.add_layer("Strokes", Color::WHITE);
        ColourRulesScene { session, painting, picture: landscape(), tips: bristle_tips(), settings: Settings::from_controls(CONTROLS), dirty: true }
    }

    fn refill(&mut self) {
        let layer = self.painting.get_layer_mut("Strokes").unwrap();
        layer.clear();
        for which in 0..4 {
            let p = panel(which);
            let rules = rules(&self.settings, which);
            let mut records = jittered_strokes(&self.picture, p, 2600, 5, 15.0, 5.5, &|_, y| if y < p.y + p.height * 0.6 { 4.0 } else { -12.0 });
            for r in records.iter_mut() {
                r.color = rules.apply(r.color, r.seed);
                r.load = 1.0;
            }
            layer.add_shape(fill_rect(p.x, p.y, p.width, p.height, Some(Color::from_rgb(40, 40, 46)), 0.0));
            layer.add_shape(BrushStrokesShape::new(BrushStrokeSet::new(records).unwrap(), BrushStrokesOptions { head_softness: 0.1, ..Default::default() }).unwrap().with_tips(self.tips.clone()));
        }
        self.dirty = false;
    }
}

impl DemoScene for ColourRulesScene {
    fn name(&self) -> &'static str {
        "colour-rules"
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
        if self.dirty {
            self.refill();
        }
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
        self.painting.set_time(seconds);
        self.painting.render_with(scene, width, height, &codebrix_webgpu_animation::SessionRenderOptions { include_background: false, ..Default::default() });
    }

    fn controls(&self) -> &'static [Control] {
        CONTROLS
    }

    fn apply_settings(&mut self, settings: &Settings) {
        self.settings = settings.clone();
        self.dirty = true;
    }

    fn legend(&self) -> &'static [&'static str] {
        LEGEND
    }

    fn readouts(&self) -> Vec<Readout> {
        let mid = |space| {
            let c = ColorMix::mix(BLUE, YELLOW, 0.5, space);
            format!("({},{},{})", c.r, c.g, c.b)
        };
        vec![Readout::new("midpoints", "Blue -> yellow at 0.5: sRGB / Oklab / Oklch", format!("{} {} {}", mid(ColorInterpolation::Srgb), mid(ColorInterpolation::Oklab), mid(ColorInterpolation::Oklch)))]
    }
}
