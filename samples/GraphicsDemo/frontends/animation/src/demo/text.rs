//! "text" (AnimationDemoApp's TextScene): text from a registered font - left, centre and right
//! alignment about one anchor line, letter spacing (tracking) easing in and out, the measured
//! width drawn as a bracket under each line, the outline bounds drawn as a box, and a row of
//! sizes laid end to end by the widths the registry measures.

use codebrix_webgpu_animation::animation::Easing;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{LineShape, LineShapeOptions, RectangleShape, RectangleShapeOptions, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, MeasureTextOptions, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession};

use crate::demo::{create_frame_session, label, DemoScene, ACCENT, CONTENT_TOP, DIM, INK};

const TITLE: &str = "Text";
const DESCRIPTION: &str = "TextShape from a registered font: alignment, tracking, measured width (bracket) and outline bounds (box)";
const ANCHOR: f32 = 640.0;
const SAMPLE: &str = "Animate";

pub struct TextScene {
    session: DrawingSession,
    fonts: FontRegistry,
}

fn line(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) -> LineShape {
    LineShape::new(x1, y1, x2, y2, LineShapeOptions { stroke_thickness: thickness, color: Some(color) }).unwrap()
}

impl TextScene {
    pub fn new(fonts: &FontRegistry) -> TextScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let guides = session.add_layer("Guides", DIM);
        guides.add_shape(line(ANCHOR, CONTENT_TOP + 16.0, ANCHOR, 470.0, 1.0, Color::from_rgba(255, 176, 72, 160)));
        label(guides, fonts, "anchor x", ANCHOR + 6.0, CONTENT_TOP + 30.0, 12.0, ACCENT, TextAlign::Left);
        label(guides, fonts, "TextAlign.Left", 40.0, 190.0, 13.0, DIM, TextAlign::Left);
        label(guides, fonts, "TextAlign.Center", 40.0, 300.0, 13.0, DIM, TextAlign::Left);
        label(guides, fonts, "TextAlign.Right", 40.0, 410.0, 13.0, DIM, TextAlign::Left);
        label(guides, fonts, "sizes, all measured before drawing (FontRegistry.MeasureText):", 40.0, 520.0, 13.0, DIM, TextAlign::Left);
        session.add_layer("Text", INK);
        TextScene { session, fonts: fonts.clone() }
    }
}

impl DemoScene for TextScene {
    fn name(&self) -> &'static str {
        "text"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let fonts = self.fonts.clone();
        let moving = self.session.get_layer_mut("Text").unwrap();
        moving.clear();
        let tracking = (Easing::sine_in_out(0.5 - (0.5 * (2.0 * std::f64::consts::PI * seconds / 4.0).cos())) * 0.45) as f32;
        for (i, align) in [TextAlign::Left, TextAlign::Center, TextAlign::Right].into_iter().enumerate() {
            let baseline = 190.0 + (i as f32 * 110.0);
            let text = TextShape::new(SAMPLE, ANCHOR, baseline, 60.0, &fonts, TextShapeOptions { color: Some(INK), align, tracking, ..TextShapeOptions::default() }).unwrap();
            moving.add_shape(text.clone());

            // The measured advance width as a bracket, and the outline bounds as a box.
            let under = baseline + 16.0;
            let (left, w) = (text.left(), text.width());
            moving.add_shape(line(left, under, left + w, under, 2.0, ACCENT));
            moving.add_shape(line(left, under - 6.0, left, under, 2.0, ACCENT));
            moving.add_shape(line(left + w, under - 6.0, left + w, under, 2.0, ACCENT));
            let b = text.get_bounds();
            moving.add_shape(RectangleShape::new(b.x, b.y, b.width, b.height, RectangleShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgba(120, 170, 255, 150)), ..Default::default() }).unwrap());
            label(moving, &fonts, &format!("width {w:.1}"), 1240.0, baseline, 13.0, DIM, TextAlign::Right);
        }
        label(moving, &fonts, &format!("tracking {tracking:.2} em"), 1240.0, 140.0, 15.0, ACCENT, TextAlign::Right);

        // A row of sizes laid end to end: each placed with the width the registry measured for the one before.
        let mut x = 40.0;
        for size in [14.0f32, 22.0, 34.0, 52.0, 80.0] {
            let word = format!("{size:.0} px");
            label(moving, &fonts, &word, x, 640.0, size, INK, TextAlign::Left);
            x += fonts.measure_text(&word, size, &MeasureTextOptions::default()).unwrap() + 28.0;
        }

        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
