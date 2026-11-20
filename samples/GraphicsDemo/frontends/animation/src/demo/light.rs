//! "light" (AnimationDemoApp's LightScene): one night scene - a deep-blue sky, a moon, a dark
//! skyline and its lit windows twinkling from a seeded DrawingRandom per window - shown four ways,
//! each a session of its own drawn into its panel: as it is; blurred (an effect whose sigma
//! animates); with a BLOOM of its light; and the finished frame with bloom, VIGNETTE and TONE CURVE.
//! The planes' whole-frame operations run here as GPU PASSES over the layer
//! (DrawingFrameOperations), with C#'s parameters; C# computes them on float plane images on the
//! CPU (its lights at 3.0 bloom harder than the 8-bit layers here can).

use codebrix_webgpu_animation::effects::{DrawingEffects, DrawingFrameOperations, FrameVignette};
use codebrix_webgpu_animation::fills::LinearGradientFill;
use codebrix_webgpu_animation::planes::DrawingRandom;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::RectangleShape;
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, SessionRenderOptions};

use crate::demo::kit::{disc, fill_rect, part, polygon};
use crate::demo::pictures::ping_pong;
use crate::demo::{create_frame_session, create_session, label, DemoScene, ACCENT, DIM};

const TITLE: &str = "Blur, bloom, vignette and tone curve";
const DESCRIPTION: &str = "The planes' whole-frame filters on one night scene - here as GPU passes over a layer (DrawingFrameOperations)";
const PW: u32 = 592;
const PH: u32 = 250;
const LEFT_X: f32 = 32.0;
const RIGHT_X: f32 = 656.0;
const TOP_Y: f32 = 108.0;
const BOTTOM_Y: f32 = 400.0;
const WINDOWS: usize = 70;

pub struct LightScene {
    frame: DrawingSession,
    panels: Vec<DrawingSession>,
    windows: Vec<(f32, f32, f32, f32, f64)>,
    fonts: FontRegistry,
}

impl LightScene {
    pub fn new(fonts: &FontRegistry) -> LightScene {
        let mut random = DrawingRandom::new(2026);
        let windows = (0..WINDOWS)
            .map(|_| {
                let x = random.next_int_between(20, PW as i32 - 30).unwrap() as f32;
                let y = random.next_int_between(130, PH as i32 - 20).unwrap() as f32;
                let w = random.next_int_between(5, 11).unwrap() as f32;
                let h = random.next_int_between(6, 13).unwrap() as f32;
                (x, y, w, h, random.next_double())
            })
            .collect();
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let captions = frame.add_layer("Captions", DIM);
        label(captions, fonts, "1  the scene as it is drawn", LEFT_X, TOP_Y + PH as f32 + 20.0, 13.0, DIM, TextAlign::Left);
        label(captions, fonts, "3  + a bloom of its light: with_bloom(sigma 8, gain 0.9) - a GPU pass", LEFT_X, BOTTOM_Y + PH as f32 + 20.0, 13.0, DIM, TextAlign::Left);
        label(captions, fonts, "4  + with_vignette(...) + with_tone_curve(0.15, 1.08): the finished frame", RIGHT_X, BOTTOM_Y + PH as f32 + 20.0, 13.0, DIM, TextAlign::Left);
        frame.add_layer("Notes", ACCENT);
        let mut panels = Vec::new();
        for i in 0..4 {
            let mut panel = create_session(PW, PH, Color::BLACK, fonts);
            panel.add_layer("Night", Color::WHITE);
            let ops = match i {
                2 => DrawingFrameOperations::NONE.with_bloom(8.0, 0.9).unwrap(),
                3 => DrawingFrameOperations::NONE
                    .with_bloom(8.0, 0.9)
                    .unwrap()
                    .with_vignette(FrameVignette { center_x: PW as f32 / 2.0, center_y: PH as f32 * 0.48, radius_x: PW as f32 * 0.75, radius_y: PH as f32 * 0.62, strength: 0.38, minimum: 0.4 })
                    .unwrap()
                    .with_tone_curve(0.15, 1.08),
                _ => DrawingFrameOperations::NONE,
            };
            panel.get_layer_mut("Night").unwrap().set_frame_operations(Some(ops));
            panels.push(panel);
        }
        LightScene { frame, panels, windows, fonts: fonts.clone() }
    }

    /// The night: sky, moon, skyline, lit windows.
    fn paint_night(&mut self, seconds: f64, panel: usize) {
        let sky = LinearGradientFill::from_colors(PointF::new(0.0, 0.0), PointF::new(0.0, PH as f32), Color::from_rgb(8, 13, 41), Color::from_rgb(23, 33, 71)).unwrap();
        let night = self.panels[panel].get_layer_mut("Night").unwrap();
        night.clear();
        night.add_shape(RectangleShape::with_fill(0.0, 0.0, PW as f32, PH as f32, sky.into(), 0.0).unwrap());
        night.add_shape(disc(470.0, 52.0, 22.0, Some(Color::from_rgb(255, 255, 235))));
        let mut skyline = vec![PointF::new(0.0, PH as f32)];
        let mut x = 0.0f32;
        while x <= PW as f32 {
            let top = 120.0 + (30.0 * (x * 0.021).sin().abs()).floor() + 25.0 * ((x as i32 / 70) % 3) as f32;
            skyline.push(PointF::new(x, top));
            x += 6.0;
        }
        skyline.push(PointF::new(PW as f32, PH as f32));
        night.add_shape(polygon(&skyline, Some(Color::from_rgb(10, 12, 18))));
        for (x, y, w, h, phase) in &self.windows {
            if (seconds * 0.35 + phase) % 1.0 < 0.7 {
                night.add_shape(fill_rect(*x, *y, *w, *h, Some(Color::from_rgb(255, 225, 150)), 0.0));
            }
        }
    }
}

impl DemoScene for LightScene {
    fn name(&self) -> &'static str {
        "light"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let sigma = (0.5 + 5.5 * ping_pong(seconds, 4.0)) as f32;
        self.frame.set_time(seconds);
        let fonts = self.fonts.clone();
        let notes = self.frame.get_layer_mut("Notes").unwrap();
        notes.clear();
        label(notes, &fonts, &format!("2  DrawingEffects::blur({:.1}) - the effects' GPU blur", sigma * 2.0), RIGHT_X, TOP_Y + PH as f32 + 20.0, 13.0, ACCENT, TextAlign::Left);
        self.frame.render_with_background(scene, width, height);
        let places = [(LEFT_X, TOP_Y), (RIGHT_X, TOP_Y), (LEFT_X, BOTTOM_Y), (RIGHT_X, BOTTOM_Y)];
        for (i, (x, y)) in places.iter().enumerate() {
            self.paint_night(seconds, i);
            if i == 1 {
                // A blur radius is twice the Gaussian's sigma (C#'s rule).
                self.panels[i].get_layer_mut("Night").unwrap().set_effects(Some(DrawingEffects::blur(sigma * 2.0).unwrap()));
            }
            self.panels[i].set_time(seconds);
            let target = part(&self.frame, *x, *y, PW as f32, PH as f32, width, height);
            self.panels[i].render_with(scene, width, height, &SessionRenderOptions { include_background: true, target: Some(target) });
        }
    }
}
