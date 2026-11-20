//! "svg-features": the SVG features the crate draws, one per cell of a document served by the
//! backend: a clip path of TWO circles (their union), an alpha mask and a luminance mask (applied
//! by its alpha - reported), multiply and screen blend modes, a focal radial gradient, the one
//! plain Gaussian blur beside the same circle sharp, dashes / joins / caps and markers, <use>
//! instances of one star, and SVG TEXT laid out by usvg in the fonts the backend serves (Roboto,
//! Open Sans; a family not supplied falls back to the application's font - reported). A frame
//! moves from cell to cell naming the feature (breathing, so the scene is never still); the pattern paint and the drop shadow at the bottom
//! right are not drawn and are reported. The document's report is listed under it.

use codebrix_webgpu_animation::animation::EasingKind;
use codebrix_webgpu_animation::models::AnimatedProperty;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{AnimatedShape, DrawingGroup, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::svg::DrawingSvgAspect;
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, RectangleF};

use crate::demo::kit::{fill_rect, floats, outline_rect, L};
use crate::demo::svg_files::load_svg;
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "SVG features";
const DESCRIPTION: &str = "DrawingSvg + draw_svg: clips (a union), masks, blend modes, gradients, the blur, strokes, markers, <use>, SVG text";

/// The cells of the document (x, y, width, height in document units) and what each shows.
const CELLS: [(f32, f32, f32, f32, &str); 8] = [
    (20.0, 40.0, 240.0, 140.0, "clip-path of two circles: their union"),
    (330.0, 40.0, 220.0, 220.0, "an alpha mask (top) and a luminance mask applied by alpha (below)"),
    (620.0, 40.0, 240.0, 140.0, "mix-blend-mode multiply (left) and screen (right)"),
    (950.0, 40.0, 140.0, 140.0, "a radial gradient with a focal point"),
    (45.0, 285.0, 190.0, 110.0, "feGaussianBlur (left) beside the same circle sharp"),
    (590.0, 240.0, 220.0, 150.0, "dashes, round joins and caps; markers"),
    (870.0, 270.0, 200.0, 70.0, "<use> instances of one star"),
    (60.0, 420.0, 1100.0, 100.0, "SVG text, laid out by usvg in the backend's fonts"),
];

pub struct SvgFeaturesScene {
    session: DrawingSession,
}

impl SvgFeaturesScene {
    pub fn new(fonts: &FontRegistry) -> SvgFeaturesScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let svg = load_svg("features").expect("the feature sheet loads");
        let notes = session.add_layer("Notes", DIM);
        let report = svg.report();
        let mut y = 560.0;
        label(notes, fonts, "report():", 40.0, y, 13.0, ACCENT, TextAlign::Left);
        for line in report.iter().take(5) {
            y += 18.0;
            let short: String = line.chars().take(170).collect();
            label(notes, fonts, &short, 40.0, y, 11.0, DIM, TextAlign::Left);
        }
        session.add_layer("Art", INK);
        // The document is 1200 x 540, fitted (Meet) above the report.
        let svg_id = session.draw_svg(&svg, RectangleF::new(40.0, 100.0, 1200.0, 540.0 * 0.82), DrawingSvgAspect::Meet);
        session.animations_mut().bind(svg_id, AnimatedProperty::Opacity, floats(&[(0.0, 0.0, L), (0.6, 1.0, EasingKind::CubicOut)])).unwrap();
        // The frame that moves from cell to cell (2 s each), with the cell's name.
        let fonts_for_frame = fonts.clone();
        let content = RectangleF::new(40.0, 100.0, 1200.0, 540.0 * 0.82);
        let scale = (content.width / 1200.0).min(content.height / 540.0);
        let left = content.x + (content.width - 1200.0 * scale) / 2.0;
        session.draw_shape(AnimatedShape::new(
            move |t| {
                let i = ((t / 2.0).floor().max(0.0) as usize) % CELLS.len();
                let (x, y, w, h, name) = CELLS[i];
                let mut g = DrawingGroup::new(None);
                // The frame and a faint wash over the cell breathe (their alpha follows the time), so the
                // scene is never still.
                let alpha = (170.0 + 85.0 * (t * 3.0).sin()) as u8;
                let frame = Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, alpha);
                g.add(fill_rect(left + (x - 8.0) * scale, content.y + (y - 8.0) * scale, (w + 16.0) * scale, (h + 16.0) * scale, Some(Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, alpha / 6)), 8.0));
                g.add(outline_rect(left + (x - 8.0) * scale, content.y + (y - 8.0) * scale, (w + 16.0) * scale, (h + 16.0) * scale, 2.0, Some(frame), 8.0));
                g.add(TextShape::new(name, left + (x - 8.0) * scale, content.y + (y - 14.0) * scale, 13.0, &fonts_for_frame, TextShapeOptions { color: Some(ACCENT), ..TextShapeOptions::default() }).ok()?);
                Some(g.into())
            },
            0.0,
        ));
        SvgFeaturesScene { session }
    }
}

impl DemoScene for SvgFeaturesScene {
    fn name(&self) -> &'static str {
        "svg-features"
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
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
