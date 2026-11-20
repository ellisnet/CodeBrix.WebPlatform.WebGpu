//! "brush-strokes" (no C# scene of its own: the painterly scenes' strokes, PLAYED): a seeded
//! layout of brushstrokes painting themselves ON one after another - BrushStrokeShape's
//! `progress` (no C# equivalent; at 1 it is exactly C#'s stroke), each stroke drawn by the
//! renderer as an anti-aliased filled polygon in a colour picked up from a gradient under it - then
//! holding, and painting on again.

use codebrix_webgpu_animation::animation::{ColorRamp, Easing};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokeShape, BrushStrokeShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::effects::DrawingMask;
use codebrix_webgpu_animation::paths::PathGeometry;
use codebrix_webgpu_animation::{Color, DrawingSession, RectangleF};

use crate::demo::brushwork::layout;
use crate::demo::kit::fill_rect;
use crate::demo::{create_frame_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Brushstrokes painting on";
const DESCRIPTION: &str = "BrushStrokeShape with a progress: each stroke painted from its first tip to its end, one after another";
const X: f32 = 140.0;
const Y: f32 = 120.0;
const W: u32 = 1000;
const H: u32 = 480;

pub struct BrushStrokesScene {
    session: DrawingSession,
    strokes: Vec<BrushStrokeShape>,
}

impl BrushStrokesScene {
    pub fn new(fonts: &FontRegistry) -> BrushStrokesScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = session.add_layer("Notes", DIM);
        notes.add_shape(fill_rect(X, Y, W as f32, H as f32, Some(Color::from_rgb(236, 226, 206)), 6.0));
        label(notes, fonts, "with_progress(p): the outline cut straight across the stroke's axis; p = 1 is C#'s whole stroke", X, Y + H as f32 + 34.0, 15.0, INK, TextAlign::Left);
        label(notes, fonts, "the strokes start one after another over six seconds, each painting on over 0.6 s; colours picked up from a ramp along the canvas", X, Y + H as f32 + 58.0, 13.0, DIM, TextAlign::Left);
        // The strokes run past the canvas's edges, as a painter's do: the layer is kept to it.
        session.add_layer("Strokes", INK).set_mask(Some(DrawingMask::inside(PathGeometry::rectangle(RectangleF::new(X, Y, W as f32, H as f32)))));
        let ramp = ColorRamp::from_colors(&[Color::from_rgb(32, 60, 120), Color::from_rgb(60, 130, 170), Color::from_rgb(240, 190, 90), Color::from_rgb(210, 80, 60)]).unwrap();
        let strokes = layout(W, H, 54.0, 7)
            .into_iter()
            .map(|s| {
                let t = (s.center_x() / W as f32).clamp(0.0, 1.0) as f64;
                let options = BrushStrokeShapeOptions { bend: s.bend(), profile: s.profile(), color: Some(ramp.at(t)) };
                BrushStrokeShape::new(X + s.center_x(), Y + s.center_y(), s.length(), s.width(), s.angle(), options).unwrap()
            })
            .collect();
        BrushStrokesScene { session, strokes }
    }
}

impl DemoScene for BrushStrokesScene {
    fn name(&self) -> &'static str {
        "brush-strokes"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        let layer = self.session.get_layer_mut("Strokes").unwrap();
        layer.clear();
        // The strokes start one after another over six seconds, each painting on over 0.6 s.
        let step = 6.0 / self.strokes.len().max(1) as f64;
        for (i, stroke) in self.strokes.iter().enumerate() {
            let p = Easing::cubic_out(((seconds - 0.3 - i as f64 * step) / 0.6).clamp(0.0, 1.0));
            if p <= 0.0 {
                continue;
            }
            layer.add_shape(stroke.clone().with_progress(p as f32));
        }
        self.session.render_with_background(scene, width, height);
    }
}
