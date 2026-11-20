//! "camera" (AnimationDemoApp's CameraScene): a night landscape of layers - stars, far hills,
//! near hills, the ground with trees - under a DrawingCamera whose pan, zoom and roll are bound
//! to tracks; each layer follows it by its PARALLAX (stars 0.15, far 0.4, near 0.75, ground 1;
//! the header and the HUD 0, fixed to the frame). The stars come from the crate's DrawingRandom
//! with the C# scene's seed, so they land where the C# scene puts them.

use codebrix_webgpu_animation::animation::EasingKind;
use codebrix_webgpu_animation::models::{AnimatedProperty, BindingTarget, DrawingCamera};
use codebrix_webgpu_animation::planes::DrawingRandom;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::kit::{disc, fill_rect, floats, line, points, polygon, L};
use crate::demo::{add_header, create_session, label, DemoScene, ACCENT, FRAME_HEIGHT, FRAME_WIDTH, INK};

const TITLE: &str = "Camera and parallax";
const DESCRIPTION: &str = "DrawingSession.Camera (pan, zoom, roll; animated by tracks) and DrawingLayer.Parallax: 0 = HUD, 1 = moves with the scene";

pub struct CameraScene {
    session: DrawingSession,
    fonts: FontRegistry,
}

fn add_hills(session: &mut DrawingSession, name: &str, parallax: f32, baseline: f32, height: f32, frequency: f64, color: Color) {
    let layer = session.add_layer(name, color);
    layer.set_parallax(parallax).unwrap();
    let mut pts = vec![PointF::new(-1400.0, 760.0)];
    let mut x = -1400.0f32;
    while x <= 2700.0 {
        let y = baseline as f64 - (height as f64 * (0.6 + (0.4 * (x as f64 * frequency).sin()) + (0.25 * ((x as f64 * frequency * 2.7) + 1.3).sin())));
        pts.push(PointF::new(x, y as f32));
        x += 20.0;
    }
    pts.push(PointF::new(2700.0, 760.0));
    layer.add_shape(polygon(&pts, None));
}

impl CameraScene {
    pub fn new(fonts: &FontRegistry) -> CameraScene {
        let mut session = create_session(FRAME_WIDTH, FRAME_HEIGHT, Color::from_rgb(12, 14, 34), fonts);
        let sky = session.add_layer("Stars", INK);
        sky.set_parallax(0.15).unwrap();
        let mut random = DrawingRandom::new(77);
        for _ in 0..260 {
            let x = random.uniform(-500.0, 1900.0) as f32;
            let y = random.uniform(100.0, 430.0) as f32;
            let r = random.uniform(0.8, 2.2) as f32;
            let a = random.next_int_between(80, 230).unwrap() as u8;
            sky.add_shape(disc(x, y, r, Some(Color::from_rgba(230, 230, 255, a))));
        }
        add_hills(&mut session, "Far hills", 0.4, 470.0, 70.0, 0.006, Color::from_rgb(40, 44, 92));
        add_hills(&mut session, "Near hills", 0.75, 540.0, 60.0, 0.011, Color::from_rgb(56, 72, 120));
        let ground = session.add_layer("Ground", Color::from_rgb(30, 34, 50));
        ground.add_shape(fill_rect(-1200.0, 600.0, 3800.0, 400.0, None, 0.0));
        for i in -10..30 {
            let x = i as f32 * 110.0;
            ground.add_shape(fill_rect(x - 4.0, 520.0, 8.0, 84.0, Some(Color::from_rgb(120, 100, 80)), 0.0));
            ground.add_shape(disc(x, 512.0, 22.0, Some(Color::from_rgb((80 + ((i * 37) & 63)) as u8, 170, 110))));
        }
        add_header(&mut session, TITLE, DESCRIPTION, true);
        session.get_layer_mut("Header").unwrap().set_parallax(0.0).unwrap();
        let hud = session.add_layer("Hud", ACCENT);
        hud.set_parallax(0.0).unwrap();
        hud.add_shape(fill_rect(0.0, 664.0, FRAME_WIDTH as f32, 56.0, Some(Color::from_rgba(12, 14, 34, 200)), 0.0));
        hud.add_shape(line(630.0, 360.0, 650.0, 360.0, 1.5, Some(Color::from_rgba(255, 176, 72, 160))));
        hud.add_shape(line(640.0, 350.0, 640.0, 370.0, 1.5, Some(Color::from_rgba(255, 176, 72, 160))));
        session.add_layer("Readout", INK).set_parallax(0.0).unwrap();
        session.set_camera(Some(DrawingCamera::new()));
        let mut a = session.animations_mut();
        a.bind(BindingTarget::Camera, AnimatedProperty::Position, points(&[(0.0, -300.0, 0.0, L), (4.5, 500.0, 0.0, EasingKind::SineInOut), (8.0, -100.0, 0.0, EasingKind::SineInOut)])).unwrap();
        a.bind(BindingTarget::Camera, AnimatedProperty::Scale, floats(&[(0.0, 1.0, L), (3.0, 1.0, L), (5.0, 1.7, EasingKind::CubicInOut), (7.5, 1.0, EasingKind::CubicInOut)])).unwrap();
        a.bind(BindingTarget::Camera, AnimatedProperty::Rotation, floats(&[(0.0, 0.0, L), (4.0, 6.0, EasingKind::SineInOut), (8.0, -2.0, EasingKind::SineInOut)])).unwrap();
        CameraScene { session, fonts: fonts.clone() }
    }
}

impl DemoScene for CameraScene {
    fn name(&self) -> &'static str {
        "camera"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        let camera = *self.session.camera().unwrap();
        let text = format!("camera pan {:.0} px, zoom {:.2}x, roll {:.1} deg   parallax: stars 0.15, far 0.4, near 0.75, ground 1, HUD 0", camera.pan().x, camera.zoom(), camera.rotation());
        let fonts = self.fonts.clone();
        let readout = self.session.get_layer_mut("Readout").unwrap();
        readout.clear();
        label(readout, &fonts, &text, 32.0, 690.0, 16.0, INK, TextAlign::Left);
        self.session.render_with_background(scene, width, height);
    }
}
