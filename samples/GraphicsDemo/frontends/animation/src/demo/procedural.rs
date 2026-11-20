//! "procedural" (AnimationDemoApp's ProceduralScene): a sky, a sun with its glow and rippling water
//! from ONE formula per pixel (DrawingPlaneImage::fill_with), values past 1.0 tone-mapped with
//! apply_tone_curve - CPU calculation with NoSkia's formula, here at HALF the panel's size
//! (608 x 260) and stretched by the GPU, the planes' advice for a real-time scene (C# fills the
//! whole 1216 x 520 panel; on one thread in a browser that costs more than a frame).

use codebrix_webgpu_animation::animation::Easing;
use codebrix_webgpu_animation::images::{ImageSampling, ImageSource};
use codebrix_webgpu_animation::planes::DrawingPlaneImage;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{DrawingSession, RectangleF};

use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM};

const TITLE: &str = "Per-pixel procedural fill";
const DESCRIPTION: &str = "Sky, sun, glow and rippling water from one formula; values past 1.0 tone-mapped with apply_tone_curve";
const PANEL_X: f32 = 32.0;
const PANEL_Y: f32 = 112.0;
const PANEL_WIDTH: f32 = 1216.0;
const PANEL_HEIGHT: f32 = 520.0;
const HORIZON: f32 = 330.0;
/// The plane is half the panel's size; the formula runs in the panel's coordinates.
const SCALE: u32 = 2;

pub struct ProceduralScene {
    frame: DrawingSession,
    fonts: FontRegistry,
}

/// The scene's colour at one panel point (C#'s Sky).
fn sky(x: f32, y: f32, sun_x: f32, sun_y: f32, time: f32) -> [f32; 4] {
    let (mut sample_y, mut shade) = (y, 1.0f32);
    if y >= HORIZON {
        let depth = (y - HORIZON) / (PANEL_HEIGHT - HORIZON);
        let ripple = (x * 0.045 + y * 0.6 - time * 3.1).sin() * (2.0 + 14.0 * depth);
        sample_y = (HORIZON - (y - HORIZON) - ripple).clamp(0.0, HORIZON - 1.0);
        shade = 0.72 - 0.35 * depth;
    }
    let h = sample_y / HORIZON;
    let mut c = [0.10 + 0.85 * h * h, 0.08 + 0.38 * h, 0.32 - 0.12 * h];
    let (dx, dy) = (x - sun_x, sample_y - sun_y);
    let distance = (dx * dx + dy * dy).sqrt();
    if distance < 46.0 {
        c = [2.4, 2.0, 1.3];
    } else {
        let g = (-(distance - 46.0) / 70.0).exp();
        c = [c[0] + 1.0 * g, c[1] + 0.55 * g, c[2] + 0.25 * g];
    }
    [c[0] * shade, c[1] * shade, c[2] * shade, 1.0]
}

impl ProceduralScene {
    pub fn new(fonts: &FontRegistry) -> ProceduralScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let captions = frame.add_layer("Captions", DIM);
        label(captions, fonts, "DrawingPlaneImage::new(w, h).fill_with(|x, y| colour) - one pure function per pixel, one thread; here at half size, stretched", PANEL_X, PANEL_Y + PANEL_HEIGHT + 26.0, 14.0, DIM, TextAlign::Left);
        frame.add_layer("Panel", DIM);
        frame.add_layer("Notes", ACCENT);
        ProceduralScene { frame, fonts: fonts.clone() }
    }
}

impl DemoScene for ProceduralScene {
    fn name(&self) -> &'static str {
        "procedural"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let t = seconds / self.duration_seconds();
        let sun_x = (260.0 + (960.0 - 260.0) * Easing::sine_in_out(t)) as f32;
        let sun_y = (150.0 + (300.0 - 150.0) * t) as f32;
        let time = seconds as f32;
        let (w, h) = (PANEL_WIDTH as u32 / SCALE, PANEL_HEIGHT as u32 / SCALE);
        let mut panel = DrawingPlaneImage::new(w, h).unwrap();
        let s = SCALE as f32;
        panel.fill_with(|x, y| sky(x as f32 * s, y as f32 * s, sun_x, sun_y, time));
        panel.apply_tone_curve(0.15, 1.08);
        let picture = ImageSource::from_plane_image(&panel).unwrap();
        self.frame.set_time(seconds);
        self.frame.get_layer_mut("Panel").unwrap().clear();
        self.frame.set_active_layer("Panel");
        self.frame.draw_image(&picture, RectangleF::new(PANEL_X, PANEL_Y, PANEL_WIDTH, PANEL_HEIGHT), ImageSampling::Linear).unwrap();
        let fonts = self.fonts.clone();
        let notes = self.frame.get_layer_mut("Notes").unwrap();
        notes.clear();
        label(notes, &fonts, &format!("sun at ({sun_x:.0}, {sun_y:.0}); sun core 2.4, glow added on top; apply_tone_curve(0.15, 1.08)"), PANEL_X, PANEL_Y + PANEL_HEIGHT + 50.0, 15.0, ACCENT, TextAlign::Left);
        self.frame.render_with_background(scene, width, height);
    }
}
