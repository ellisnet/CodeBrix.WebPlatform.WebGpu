//! "kerning" (AnimationDemoApp's KerningScene): the same words drawn with the font's kerning (the
//! default) and without it, one above the other, with each line's measured width; then a tracked
//! (letter-spaced) line whose tracking eases in and out - shaped as one run with the tracking
//! added between its glyphs, so it stays kerned - beside the same line unkerned.

use codebrix_webgpu_animation::models::DrawingLayer;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{LineShape, LineShapeOptions, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession};

use crate::demo::pictures::ping_pong;
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Kerning";
const DESCRIPTION: &str = "Text is kerned when the font carries kerning data; TextShape kerning: false switches it off; tracking keeps it";
const PAIRS: &str = "AVATAR  WAVE  Tower  Ty  To";
const TRACKED: &str = "WAVY TOWER";

pub struct KerningScene {
    session: DrawingSession,
    fonts: FontRegistry,
}

impl KerningScene {
    /// Builds the kerned and unkerned rows (drawn once; the tracked rows are rebuilt every frame).
    pub fn new(fonts: &FontRegistry) -> KerningScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        {
            let rows = session.add_layer("Rows", INK);
            label(rows, fonts, "kerning on (the default) - the font's own pair adjustments", 40.0, 140.0, 14.0, DIM, TextAlign::Left);
            label(rows, fonts, "kerning off - plain advances", 40.0, 300.0, 14.0, DIM, TextAlign::Left);
            let kerned = text(PAIRS, 40.0, 240.0, 76.0, fonts, INK, 0.0, true);
            let plain = text(PAIRS, 40.0, 400.0, 76.0, fonts, DIM, 0.0, false);
            let (k, p) = (extent(&kerned), extent(&plain));
            rows.add_shape(kerned);
            rows.add_shape(plain);
            bracket(rows, k, 256.0, ACCENT);
            bracket(rows, p, 416.0, DIM);
            label(rows, fonts, &width(k), 1240.0, 240.0, 15.0, ACCENT, TextAlign::Right);
            label(rows, fonts, &width(p), 1240.0, 400.0, 15.0, DIM, TextAlign::Right);
            label(rows, fonts, "tracked text stays kerned: one shaped run, tracking added between its glyphs", 40.0, 480.0, 14.0, DIM, TextAlign::Left);
        }
        session.add_layer("Tracked", INK);
        KerningScene { session, fonts: fonts.clone() }
    }
}

impl DemoScene for KerningScene {
    fn name(&self) -> &'static str {
        "kerning"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width_px: u32, height_px: u32) {
        let fonts = self.fonts.clone();
        let tracking = (ping_pong(seconds, 4.0) * 0.35) as f32;
        let kerned = text(TRACKED, 40.0, 570.0, 58.0, &fonts, INK, tracking, true);
        let plain = text(TRACKED, 40.0, 660.0, 58.0, &fonts, DIM, tracking, false);
        let (k, p) = (extent(&kerned), extent(&plain));
        let moving = self.session.get_layer_mut("Tracked").unwrap();
        moving.clear();
        moving.add_shape(kerned);
        moving.add_shape(plain);
        bracket(moving, k, 584.0, ACCENT);
        bracket(moving, p, 674.0, DIM);
        label(moving, &fonts, &format!("tracking {tracking:.2} em"), 1240.0, 520.0, 15.0, ACCENT, TextAlign::Right);
        label(moving, &fonts, &format!("kerned {}", width(k)), 1240.0, 570.0, 15.0, ACCENT, TextAlign::Right);
        label(moving, &fonts, &format!("unkerned {}", width(p)), 1240.0, 660.0, 15.0, DIM, TextAlign::Right);
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width_px, height_px);
    }
}

/// A left-aligned line of text with the given tracking and kerning.
#[allow(clippy::too_many_arguments)]
fn text(content: &str, x: f32, y: f32, size: f32, fonts: &FontRegistry, color: Color, tracking: f32, kerning: bool) -> TextShape {
    TextShape::new(content, x, y, size, fonts, TextShapeOptions { color: Some(color), align: TextAlign::Left, tracking, kerning, ..TextShapeOptions::default() }).unwrap()
}

/// Where a line starts after alignment, and its measured width.
fn extent(text: &TextShape) -> (f32, f32) {
    (text.left(), text.width())
}

/// The line's measured width, as the scene shows it.
fn width((_, measured): (f32, f32)) -> String {
    format!("width {measured:.1}")
}

/// A bracket under a line from its left end to its right end.
fn bracket(layer: &mut DrawingLayer, (left, measured): (f32, f32), y: f32, color: Color) {
    let right = left + measured;
    let options = || LineShapeOptions { stroke_thickness: 2.0, color: Some(color) };
    layer.add_shape(LineShape::new(left, y, right, y, options()).unwrap());
    layer.add_shape(LineShape::new(left, y - 6.0, left, y, options()).unwrap());
    layer.add_shape(LineShape::new(right, y - 6.0, right, y, options()).unwrap());
}
