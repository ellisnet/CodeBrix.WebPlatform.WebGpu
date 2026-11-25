//! "fade" (AnimationDemoApp's FadeScene): one composition faded in and out with the session's
//! opacity (eased), shown twice - drawn over its opaque black background (a fade from and to
//! black), and drawn WITHOUT its background over a checkerboard, so the fade shows as
//! transparency. Its caption layer also blinks on its own through the layer's opacity. The C#
//! scene exports the composition's pixels into the frame; here the composition is drawn into its
//! two boxes of the canvas directly (SessionRenderOptions: a target part, with or without the
//! background) - the same picture, on the GPU, with no readback.

use codebrix_webgpu_animation::animation::{Easing, StepPosition};
use codebrix_webgpu_animation::fills::{ColorStop, GradientSpread, LinearGradientFill, RadialGradientFill};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{CircleShape, RectangleShape, RectangleShapeOptions, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF, SessionRenderOptions};

use crate::demo::{create_frame_session, create_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Whole-frame fade";
const DESCRIPTION: &str = "DrawingSession.Opacity fades every layer as one group; DrawingLayer.Opacity fades one layer; without the background the fade is transparency";
const BOX_WIDTH: f32 = 560.0;
const BOX_HEIGHT: f32 = 500.0;
const LEFT_X: f32 = 40.0;
const RIGHT_X: f32 = 680.0;
const BOX_Y: f32 = 150.0;

pub struct FadeScene {
    frame: DrawingSession,
    composition: DrawingSession,
    fonts: FontRegistry,
}

fn filled(x: f32, y: f32, w: f32, h: f32, color: Color) -> RectangleShape {
    RectangleShape::new(x, y, w, h, RectangleShapeOptions { stroke_thickness: 1.0, color: Some(color), is_filled: true, ..Default::default() }).unwrap()
}

impl FadeScene {
    pub fn new(fonts: &FontRegistry) -> FadeScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let backdrop = frame.add_layer("Backdrop", DIM);
        label(backdrop, fonts, "Background included: fades from and to black", LEFT_X, BOX_Y - 14.0, 13.0, DIM, TextAlign::Left);
        label(backdrop, fonts, "include_background: false - the fade is transparency", RIGHT_X, BOX_Y - 14.0, 13.0, DIM, TextAlign::Left);
        backdrop.add_shape(filled(RIGHT_X, BOX_Y, BOX_WIDTH, BOX_HEIGHT, Color::from_rgb(70, 72, 84)));
        let mut row = 0.0;
        while row < BOX_HEIGHT {
            let mut col = ((row / 20.0) as i32 % 2) as f32 * 20.0;
            while col < BOX_WIDTH {
                backdrop.add_shape(filled(RIGHT_X + col, BOX_Y + row, 20.0, 20.0, Color::from_rgb(110, 112, 124)));
                col += 40.0;
            }
            row += 20.0;
        }
        frame.add_layer("Notes", INK);

        let mut composition = create_session(BOX_WIDTH as u32, BOX_HEIGHT as u32, Color::BLACK, fonts);
        composition.add_layer("Sky", Color::BLACK);
        let sky = LinearGradientFill::new(
            PointF::new(0.0, 0.0),
            PointF::new(0.0, BOX_HEIGHT),
            &[ColorStop::new(0.0, Color::from_rgb(30, 40, 110)).unwrap(), ColorStop::new(0.7, Color::from_rgb(200, 90, 120)).unwrap(), ColorStop::new(1.0, Color::from_rgb(255, 190, 120)).unwrap()],
            GradientSpread::Pad,
        )
        .unwrap();
        composition.draw_shape(RectangleShape::with_fill(0.0, 0.0, BOX_WIDTH, BOX_HEIGHT, sky.into(), 0.0).unwrap());
        composition.add_layer("Shapes", Color::WHITE);
        let sun = RadialGradientFill::from_colors(PointF::new(250.0, 270.0), 150.0, Color::from_rgb(255, 250, 220), Color::from_rgb(255, 140, 60)).unwrap();
        composition.draw_shape(CircleShape::with_fill(280.0, 300.0, 120.0, sun.into()).unwrap());
        composition.draw_circle(120.0, 120.0, 40.0, None, Some(Color::from_rgb(240, 240, 255)), true).unwrap();
        composition.draw_rectangle(380.0, 380.0, 140.0, 80.0, None, Some(Color::from_rgb(40, 30, 60)), true, 12.0).unwrap();
        composition.add_layer("Caption", Color::WHITE);
        composition.draw_text("Fade in, hold, fade out", BOX_WIDTH / 2.0, 470.0, 30.0, TextShapeOptions { color: Some(Color::WHITE), align: TextAlign::Center, ..TextShapeOptions::default() }).unwrap();
        FadeScene { frame, composition, fonts: fonts.clone() }
    }
}

impl DemoScene for FadeScene {
    fn name(&self) -> &'static str {
        "fade"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let opacity = if seconds < 2.0 {
            Easing::cubic_out(seconds / 2.0)
        } else if seconds < 5.0 {
            1.0
        } else {
            Easing::cubic_in(1.0 - ((seconds - 5.0) / 2.0))
        };
        self.composition.set_opacity(opacity as f32);
        // A two-step blink of the caption layer alone.
        let blink = 1.0 - (0.75 * Easing::steps((seconds % 1.2) / 1.2, 2, StepPosition::End).unwrap_or(0.0) as f32);
        self.composition.get_layer_mut("Caption").unwrap().set_opacity(blink);

        let fonts = self.fonts.clone();
        let notes = self.frame.get_layer_mut("Notes").unwrap();
        notes.clear();
        label(notes, &fonts, &format!("session.opacity = {opacity:.2}"), LEFT_X, BOX_Y + BOX_HEIGHT + 34.0, 16.0, ACCENT, TextAlign::Left);
        label(notes, &fonts, &format!("caption layer opacity = {blink:.2}"), RIGHT_X, BOX_Y + BOX_HEIGHT + 34.0, 16.0, ACCENT, TextAlign::Left);
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);

        // The composition in its two boxes, in canvas pixels.
        let to_canvas = self.frame.calibration_transform(width, height);
        let part = |x: f32| {
            let a = to_canvas * kurbo_point(x, BOX_Y);
            let b = to_canvas * kurbo_point(x + BOX_WIDTH, BOX_Y + BOX_HEIGHT);
            RectangleF::new(a.x as f32, a.y as f32, (b.x - a.x) as f32, (b.y - a.y) as f32)
        };
        self.composition.set_time(seconds);
        self.composition.render_with(scene, width, height, &SessionRenderOptions { include_background: true, target: Some(part(LEFT_X)), ..Default::default() });
        self.composition.render_with(scene, width, height, &SessionRenderOptions { include_background: false, target: Some(part(RIGHT_X)), ..Default::default() });
    }
}

fn kurbo_point(x: f32, y: f32) -> codebrix_webgpu_animation::kurbo::Point {
    codebrix_webgpu_animation::kurbo::Point::new(x as f64, y as f64)
}
