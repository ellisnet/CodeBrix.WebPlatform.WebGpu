//! "gradients" (AnimationDemoApp's GradientScene): gradient fills on filled shapes - a
//! LinearGradientFill whose direction turns, the three GradientSpread modes, RadialGradientFill
//! glows on a circle and an ellipse, a star polygon and a line of text painted by gradients.
//! Fills are immutable, so every moving gradient is a new fill on a new shape each frame.

use codebrix_webgpu_animation::animation::Easing;
use codebrix_webgpu_animation::fills::{ColorStop, GradientSpread, LinearGradientFill, RadialGradientFill};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{CircleShape, EllipseShape, PolylineShape, RectangleShape, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::{create_frame_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Gradient fills";
const DESCRIPTION: &str = "Linear and radial gradients with color stops on rectangles, circles, ellipses, polygons and text";

fn stop(position: f32, color: Color) -> ColorStop {
    ColorStop::new(position, color).unwrap()
}

fn aurora() -> [ColorStop; 3] {
    [stop(0.0, Color::from_rgb(40, 220, 170)), stop(0.5, Color::from_rgb(70, 110, 255)), stop(1.0, Color::from_rgb(230, 80, 200))]
}

pub struct GradientScene {
    session: DrawingSession,
    fonts: FontRegistry,
}

impl GradientScene {
    pub fn new(fonts: &FontRegistry) -> GradientScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let labels = session.add_layer("Labels", DIM);
        label(labels, fonts, "LinearGradientFill, direction turning", 50.0, 128.0, 14.0, DIM, TextAlign::Left);
        label(labels, fonts, "GradientSpread.Pad", 50.0, 420.0, 13.0, DIM, TextAlign::Left);
        label(labels, fonts, "GradientSpread.Repeat", 50.0, 500.0, 13.0, DIM, TextAlign::Left);
        label(labels, fonts, "GradientSpread.Reflect", 50.0, 580.0, 13.0, DIM, TextAlign::Left);
        label(labels, fonts, "RadialGradientFill: a glow with a moving highlight", 470.0, 128.0, 14.0, DIM, TextAlign::Left);
        label(labels, fonts, "radial fill on an ellipse", 470.0, 540.0, 13.0, DIM, TextAlign::Left);
        label(labels, fonts, "polygon and text with linear fills", 880.0, 128.0, 14.0, DIM, TextAlign::Left);
        session.add_layer("Shapes", INK);
        GradientScene { session, fonts: fonts.clone() }
    }
}

impl DemoScene for GradientScene {
    fn name(&self) -> &'static str {
        "gradients"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let fonts = self.fonts.clone();
        let moving = self.session.get_layer_mut("Shapes").unwrap();
        moving.clear();

        // A rounded rectangle whose gradient direction turns.
        let angle = seconds * 0.9;
        let center = PointF::new(230.0, 265.0);
        let direction = PointF::new(angle.cos() as f32 * 170.0, angle.sin() as f32 * 170.0);
        let fill = LinearGradientFill::new(PointF::new(center.x - direction.x, center.y - direction.y), PointF::new(center.x + direction.x, center.y + direction.y), &aurora(), GradientSpread::Pad).unwrap();
        moving.add_shape(RectangleShape::with_fill(50.0, 145.0, 360.0, 240.0, fill.into(), 24.0).unwrap());

        // The three spread modes, with a short gradient sliding along each bar.
        let offset = (Easing::sine_in_out((seconds % 4.0) / 4.0) * 120.0) as f32;
        for (i, spread) in [GradientSpread::Pad, GradientSpread::Repeat, GradientSpread::Reflect].into_iter().enumerate() {
            let y = 430.0 + (i as f32 * 80.0);
            let fill = LinearGradientFill::new(PointF::new(150.0 + offset, 0.0), PointF::new(210.0 + offset, 0.0), &[stop(0.0, Color::from_rgb(255, 190, 70)), stop(1.0, Color::from_rgb(120, 40, 160))], spread).unwrap();
            moving.add_shape(RectangleShape::with_fill(50.0, y, 360.0, 44.0, fill.into(), 6.0).unwrap());
        }

        // A glowing orb: a radial gradient whose highlight circles the centre, inside a pulsing halo.
        let pulse = Easing::sine_in_out(0.5 + (0.5 * (seconds * 2.2).sin())) as f32;
        let orb = PointF::new(645.0, 320.0);
        let halo = 150.0 + (40.0 * pulse);
        let halo_fill = RadialGradientFill::new(orb, halo, &[stop(0.55, Color::from_rgba(255, 200, 120, 150)), stop(1.0, Color::from_rgba(255, 200, 120, 0))], GradientSpread::Pad).unwrap();
        moving.add_shape(CircleShape::with_fill(orb.x, orb.y, halo, halo_fill.into()).unwrap());
        let highlight = PointF::new(orb.x + ((seconds * 1.4).cos() as f32 * 45.0), orb.y + ((seconds * 1.4).sin() as f32 * 45.0));
        let orb_fill = RadialGradientFill::new(highlight, 150.0, &[stop(0.0, Color::from_rgb(255, 250, 230)), stop(0.35, Color::from_rgb(255, 170, 70)), stop(1.0, Color::from_rgb(150, 40, 30))], GradientSpread::Pad).unwrap();
        moving.add_shape(CircleShape::with_fill(orb.x, orb.y, 90.0, orb_fill.into()).unwrap());

        // An ellipse lit by a radial "spotlight" that sweeps along it.
        let spot = PointF::new(645.0 + ((seconds * 1.1).sin() as f32 * 130.0), 610.0);
        let spot_fill = RadialGradientFill::from_colors(spot, 120.0, Color::from_rgb(230, 240, 255), Color::from_rgb(30, 50, 110)).unwrap();
        moving.add_shape(EllipseShape::with_fill(645.0, 610.0, 165.0, 55.0, spot_fill.into()).unwrap());

        // A turning star with a linear fill, and text painted by a gradient that slides across it.
        let star: Vec<PointF> = (0..10)
            .map(|i| {
                let a = (seconds * 0.6) + (i as f64 * std::f64::consts::PI / 5.0) - std::f64::consts::FRAC_PI_2;
                let radius = if i % 2 == 0 { 140.0 } else { 60.0 };
                PointF::new(1060.0 + (radius * a.cos() as f32), 310.0 + (radius * a.sin() as f32))
            })
            .collect();
        let star_fill = LinearGradientFill::new(PointF::new(920.0, 170.0), PointF::new(1200.0, 450.0), &aurora(), GradientSpread::Pad).unwrap();
        moving.add_shape(PolylineShape::with_fill(&star, star_fill.into()).unwrap());
        let sweep = ((seconds % 3.0) / 3.0 * 600.0) as f32 - 100.0;
        let text_fill = LinearGradientFill::new(
            PointF::new(860.0 + sweep, 0.0),
            PointF::new(1010.0 + sweep, 0.0),
            &[stop(0.0, Color::from_rgb(120, 170, 255)), stop(0.5, Color::from_rgb(255, 255, 255)), stop(1.0, Color::from_rgb(120, 170, 255))],
            GradientSpread::Pad,
        )
        .unwrap();
        moving.add_shape(TextShape::with_fill("GRADIENT", 1060.0, 600.0, 78.0, &fonts, text_fill.into(), TextShapeOptions { align: TextAlign::Center, tracking: 0.02, ..TextShapeOptions::default() }).unwrap());

        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
