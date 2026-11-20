//! "noise" (AnimationDemoApp's NoiseScene): value, gradient and fractal noise EVOLVING WITH TIME as
//! the third coordinate - here DRAWN ON THE GPU (NoiseShape: the planes' parameters and character,
//! not C#'s exact pattern for the seed) - and, below, a fixed, documented random sequence: dots
//! placed by DrawingRandom (seed 7), Normal(0.5, 0.15) above and Uniform below, EXACTLY where C#
//! places them (values that position things stay on the CPU and exact). On the right of the dots,
//! the CPU's exact noise fill of a small plane, stretched (DrawingNoise::fill, the slow route kept
//! for exact pixels).

use codebrix_webgpu_animation::animation::ColorRamp;
use codebrix_webgpu_animation::fills::ColorStop;
use codebrix_webgpu_animation::images::{ImageSampling, ImageSource};
use codebrix_webgpu_animation::planes::{DrawingNoise, DrawingNoiseFillOptions, DrawingNoiseKind, DrawingPlane, DrawingPlaneImage, DrawingRandom};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{NoiseShape, NoiseShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, RectangleF};

use crate::demo::kit::fill_rect;
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Noise and seeded randomness";
const DESCRIPTION: &str = "Value, gradient and fractal noise evolving with time as the third coordinate (drawn on the GPU); a fixed, documented random sequence";
const PANEL: f32 = 384.0;
const PANEL_HEIGHT: f32 = 340.0;
const PANEL_Y: f32 = 112.0;
const PANEL_X: [f32; 3] = [32.0, 448.0, 864.0];
const DOTS_Y: f32 = 520.0;
const DOTS_WIDTH: f32 = 900.0;
const DOTS_HEIGHT: f32 = 150.0;
/// The small plane the CPU fills exactly (stretched over 284 x 150).
const CPU_W: u32 = 96;
const CPU_H: u32 = 50;

pub struct NoiseScene {
    session: DrawingSession,
    terrain: Vec<ColorStop>,
    plane: DrawingPlane,
}

fn stops(colors: &[(f32, Color)]) -> Vec<ColorStop> {
    colors.iter().map(|(p, c)| ColorStop::new(*p, *c).unwrap()).collect()
}

impl NoiseScene {
    pub fn new(fonts: &FontRegistry) -> NoiseScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let captions = session.add_layer("Captions", DIM);
        label(captions, fonts, "NoiseShape: Value, seed 11, frequency 0.025, z = time", PANEL_X[0], PANEL_Y + PANEL_HEIGHT + 24.0, 13.0, INK, TextAlign::Left);
        label(captions, fonts, "NoiseShape: Gradient, seed 11, frequency 0.025, z = time", PANEL_X[1], PANEL_Y + PANEL_HEIGHT + 24.0, 13.0, INK, TextAlign::Left);
        label(captions, fonts, "Gradient, 5 octaves, through a colour ramp", PANEL_X[2], PANEL_Y + PANEL_HEIGHT + 24.0, 13.0, INK, TextAlign::Left);
        label(captions, fonts, "DrawingRandom(7): Normal(0.5, 0.15) above, Uniform below - the same sequence every run, as in C#", 32.0, DOTS_Y - 10.0, 13.0, DIM, TextAlign::Left);
        label(captions, fonts, "CPU: DrawingNoise::fill (exact, slow)", 956.0, DOTS_Y - 10.0, 13.0, DIM, TextAlign::Left);
        session.add_layer("Noise", INK);
        session.add_layer("Dots", ACCENT);
        // C#'s terrain ramp sampled at value * 0.7 + 0.5: the stops placed where that lands on the
        // GPU's value * 0.5 + 0.5.
        let ramp = ColorRamp::from_colors(&[Color::from_rgb(18, 40, 110), Color::from_rgb(40, 110, 190), Color::from_rgb(230, 210, 150), Color::from_rgb(70, 140, 60), Color::from_rgb(40, 80, 40), Color::from_rgb(240, 240, 250)]).unwrap();
        let terrain: Vec<ColorStop> = (0..=10).map(|i| {
            let u = i as f32 / 10.0;
            ColorStop::new(((u - 0.5) / 1.4 + 0.5).clamp(0.0, 1.0), ramp.at(u as f64)).unwrap()
        }).collect();
        NoiseScene { session, terrain, plane: DrawingPlane::new(CPU_W, CPU_H).unwrap() }
    }
}

impl DemoScene for NoiseScene {
    fn name(&self) -> &'static str {
        "noise"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        let fill = |z: f64| DrawingNoiseFillOptions { z: Some(z), ..DrawingNoiseFillOptions::default() };
        let warm = NoiseShapeOptions { fill: fill(seconds), stops: stops(&[(0.0, Color::BLACK), (1.0, Color::from_rgb(255, 217, 153))]) };
        let cool = NoiseShapeOptions { fill: fill(seconds), stops: stops(&[(0.0, Color::BLACK), (1.0, Color::from_rgb(153, 217, 255))]) };
        let land = NoiseShapeOptions { fill: DrawingNoiseFillOptions { z: Some(seconds * 0.4), octaves: 5, offset_x: seconds * 6.0, ..DrawingNoiseFillOptions::default() }, stops: self.terrain.clone() };
        // The exact CPU fill of a small plane (the same parameters as the middle panel, scaled to
        // the plane), shown stretched as a picture.
        let scale = 284.0 / CPU_W as f64;
        DrawingNoise::fill(&mut self.plane, DrawingNoiseKind::Gradient, 11, 0.025 * scale, DrawingNoiseFillOptions { z: Some(seconds), ..DrawingNoiseFillOptions::default() }).unwrap();
        let mut cpu = self.plane.clone();
        cpu.multiply(0.5).add(0.5);
        let mut image = DrawingPlaneImage::new(CPU_W, CPU_H).unwrap();
        let values = cpu.data().to_vec();
        image.fill_with(|x, y| {
            let v = values[(y * CPU_W + x) as usize];
            [v * 0.6, v * 0.85, v, 1.0]
        });
        let picture = ImageSource::from_plane_image(&image).unwrap();
        let noise = self.session.get_layer_mut("Noise").unwrap();
        noise.clear();
        noise.add_shape(NoiseShape::new(PANEL_X[0], PANEL_Y, PANEL, PANEL_HEIGHT, DrawingNoiseKind::Value, 11, 0.025, warm).unwrap());
        noise.add_shape(NoiseShape::new(PANEL_X[1], PANEL_Y, PANEL, PANEL_HEIGHT, DrawingNoiseKind::Gradient, 11, 0.025, cool).unwrap());
        noise.add_shape(NoiseShape::new(PANEL_X[2], PANEL_Y, PANEL, PANEL_HEIGHT, DrawingNoiseKind::Gradient, 5, 0.006, land).unwrap());
        self.session.set_active_layer("Noise");
        self.session.draw_image(&picture, RectangleF::new(956.0, DOTS_Y, 284.0, DOTS_HEIGHT), ImageSampling::Linear).unwrap();
        // The dots: one seeded stream; more appear as the scene plays, and they never move.
        let dots = self.session.get_layer_mut("Dots").unwrap();
        dots.clear();
        dots.add_shape(fill_rect(32.0, DOTS_Y, DOTS_WIDTH, DOTS_HEIGHT, Some(Color::from_rgb(13, 15, 26)), 0.0));
        let mut random = DrawingRandom::new(7);
        let count = (40.0 + seconds * 45.0) as i32;
        for _ in 0..count {
            let x = random.next_int_between(4, DOTS_WIDTH as i32 - 8).unwrap() as f32;
            let normal = random.normal(0.5, 0.15).clamp(0.0, 1.0);
            let uniform = random.next_double();
            dots.add_shape(fill_rect(32.0 + x, DOTS_Y + (normal * 60.0) as i32 as f32 + 4.0, 4.0, 4.0, Some(ACCENT), 0.0));
            dots.add_shape(fill_rect(32.0 + x, DOTS_Y + 80.0 + (uniform * 60.0) as i32 as f32, 4.0, 4.0, Some(Color::from_rgb(120, 190, 255)), 0.0));
        }
        self.session.render_with_background(scene, width, height);
    }
}
