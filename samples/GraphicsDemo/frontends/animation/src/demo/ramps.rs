//! "ramps" (AnimationDemoApp's ColorRampScene): multi-stop colour ramps, each drawn twice - once
//! sampled with ColorRamp::at into thin bars, once as a LinearGradientFill built from the ramp's
//! own stops - to show that both interpolate in the same space (straight sRGB). A marker sweeps
//! each ramp, with the colour it reads shown beside it.

use codebrix_webgpu_animation::animation::{ColorRamp, Easing};
use codebrix_webgpu_animation::fills::{ColorStop, GradientSpread, LinearGradientFill};
use codebrix_webgpu_animation::models::DrawingLayer;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{LineShape, LineShapeOptions, RectangleShape, RectangleShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::{create_frame_session, label, DemoScene, CONTENT_TOP, DIM, INK};

const TITLE: &str = "Color ramps";
const DESCRIPTION: &str = "ColorRamp.At(t) sampled into bars above a LinearGradientFill made from the same ColorStops - the same straight-sRGB interpolation";

const STRIP_LEFT: f32 = 230.0;
const STRIP_WIDTH: f32 = 640.0;
const STRIP_HEIGHT: f32 = 34.0;
const ROW_TOP: f32 = CONTENT_TOP + 28.0;
const ROW_HEIGHT: f32 = 148.0;
const SAMPLES: usize = 128;

pub struct ColorRampScene {
    ramps: Vec<(&'static str, ColorRamp)>,
    session: DrawingSession,
    fonts: FontRegistry,
}

fn filled(x: f32, y: f32, w: f32, h: f32, color: Color) -> RectangleShape {
    RectangleShape::new(x, y, w, h, RectangleShapeOptions { stroke_thickness: 1.0, color: Some(color), is_filled: true, ..Default::default() }).unwrap()
}

fn checkerboard(layer: &mut DrawingLayer, x: f32, y: f32, width: f32, height: f32) {
    const SQUARE: f32 = 10.0;
    layer.add_shape(filled(x, y, width, height, Color::from_rgb(70, 72, 84)));
    let mut row = 0.0;
    while row < height {
        let mut col = ((row / SQUARE) as i32 % 2) as f32 * SQUARE;
        while col < width {
            layer.add_shape(filled(x + col, y + row, SQUARE.min(width - col), SQUARE.min(height - row), Color::from_rgb(110, 112, 124)));
            col += SQUARE * 2.0;
        }
        row += SQUARE;
    }
}

impl ColorRampScene {
    pub fn new(fonts: &FontRegistry) -> ColorRampScene {
        let stop = |p: f32, c: Color| ColorStop::new(p, c).unwrap();
        let ramps = vec![
            ("Venus thread: rose, copper, violet", ColorRamp::from_colors(&[Color::from_rgb(246, 112, 168), Color::from_rgb(232, 146, 92), Color::from_rgb(156, 122, 248)]).unwrap()),
            ("Sunset: five evenly spaced stops", ColorRamp::from_colors(&[Color::from_rgb(20, 24, 82), Color::from_rgb(120, 40, 140), Color::from_rgb(232, 72, 96), Color::from_rgb(255, 160, 64), Color::from_rgb(255, 236, 160)]).unwrap()),
            ("Alpha: opaque teal to transparent", ColorRamp::from_colors(&[Color::from_rgba(40, 200, 190, 255), Color::from_rgba(40, 200, 190, 0)]).unwrap()),
            ("Hard edges: stops sharing a position", ColorRamp::new(&[
                stop(0.0, Color::from_rgb(230, 60, 60)), stop(0.33, Color::from_rgb(230, 60, 60)),
                stop(0.33, Color::from_rgb(240, 200, 40)), stop(0.66, Color::from_rgb(240, 200, 40)),
                stop(0.66, Color::from_rgb(60, 120, 230)), stop(1.0, Color::from_rgb(60, 120, 230)),
            ]).unwrap()),
        ];
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let strips = session.add_layer("Strips", INK);
        for (r, (name, ramp)) in ramps.iter().enumerate() {
            let top = ROW_TOP + (r as f32 * ROW_HEIGHT);
            label(strips, fonts, name, STRIP_LEFT, top + 14.0, 15.0, INK, TextAlign::Left);
            label(strips, fonts, "At(t), sampled", STRIP_LEFT - 14.0, top + 46.0, 12.0, DIM, TextAlign::Right);
            label(strips, fonts, "gradient fill", STRIP_LEFT - 14.0, top + 46.0 + STRIP_HEIGHT + 6.0, 12.0, DIM, TextAlign::Right);
            checkerboard(strips, STRIP_LEFT, top + 24.0, STRIP_WIDTH, (STRIP_HEIGHT * 2.0) + 6.0);
            let cell = STRIP_WIDTH / SAMPLES as f32;
            for i in 0..SAMPLES {
                let t = (i as f64 + 0.5) / SAMPLES as f64;
                strips.add_shape(filled(STRIP_LEFT + (i as f32 * cell), top + 24.0, cell + 0.5, STRIP_HEIGHT, ramp.at(t)));
            }
            let fill = LinearGradientFill::new(PointF::new(STRIP_LEFT, 0.0), PointF::new(STRIP_LEFT + STRIP_WIDTH, 0.0), &ramp.get_stops(), GradientSpread::Pad).unwrap();
            strips.add_shape(RectangleShape::with_fill(STRIP_LEFT, top + 30.0 + STRIP_HEIGHT, STRIP_WIDTH, STRIP_HEIGHT, fill.into(), 0.0).unwrap());
        }
        session.add_layer("Markers", INK);
        ColorRampScene { ramps, session, fonts: fonts.clone() }
    }
}

impl DemoScene for ColorRampScene {
    fn name(&self) -> &'static str {
        "ramps"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let t = Easing::sine_in_out(0.5 - (0.5 * (2.0 * std::f64::consts::PI * seconds / 4.0).cos()));
        let moving = self.session.get_layer_mut("Markers").unwrap();
        moving.clear();
        for (r, (_, ramp)) in self.ramps.iter().enumerate() {
            let top = ROW_TOP + (r as f32 * ROW_HEIGHT);
            let x = STRIP_LEFT + (t as f32 * STRIP_WIDTH);
            moving.add_shape(LineShape::new(x, top + 20.0, x, top + 34.0 + (STRIP_HEIGHT * 2.0), LineShapeOptions { stroke_thickness: 3.0, color: Some(INK) }).unwrap());
            checkerboard(moving, 920.0, top + 24.0, 74.0, 74.0);
            moving.add_shape(filled(920.0, top + 24.0, 74.0, 74.0, ramp.at(t)));
            let [red, g, b, a] = ramp.at(t).to_rgba32();
            label(moving, &self.fonts, &format!("At({t:.2})"), 1012.0, top + 50.0, 15.0, INK, TextAlign::Left);
            label(moving, &self.fonts, &format!("R {red}  G {g}  B {b}  A {a}"), 1012.0, top + 76.0, 13.0, DIM, TextAlign::Left);
        }
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
