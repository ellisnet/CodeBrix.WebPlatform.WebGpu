//! "effects" (AnimationDemoApp's EffectsScene): shadow, glow and blur, made on the GPU. Top: a
//! card (a group) lifting off the page - its position, shadow offset, softness and darkness all
//! bound to tracks; a star whose glow pulses and changes colour; a focus pull between two shapes
//! (their blur radii bound to opposite tracks). Bottom: a whole layer with one shadow under
//! everything on it, and the same effects over a checkerboard - the shadow and glow are true
//! alpha. (C# exports that panel from a second session; here its shapes are drawn over the
//! checkerboard directly.)

use codebrix_webgpu_animation::animation::{ColorTrack, EasingKind, TrackLoopMode};
use codebrix_webgpu_animation::effects::DrawingEffects;
use codebrix_webgpu_animation::fills::RadialGradientFill;
use codebrix_webgpu_animation::models::{AnimatedProperty, BindingTarget};
use codebrix_webgpu_animation::paths::{PathGeometry, PathStrokeStyle, PathStrokeStyleOptions};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{DrawingGroup, DrawingShape, PathShape, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::kit::{checkerboard, disc, fill_rect, floats, points, L};
use crate::demo::{add_header, create_session, label, DemoScene, FRAME_HEIGHT, FRAME_WIDTH, INK};

const TITLE: &str = "Shadow, glow and blur";
const DESCRIPTION: &str = "DrawingEffects on elements, groups and layers: DropShadow, Glow, Blur - GPU passes; parameters bound to tracks";
const PANEL_X: f32 = 680.0;
const PANEL_Y: f32 = 440.0;

pub struct EffectsScene {
    session: DrawingSession,
}

fn loop_track(keys: &[(f64, f32, f32, EasingKind)]) -> codebrix_webgpu_animation::animation::PointTrack {
    points(keys).with_loop_mode(TrackLoopMode::Loop)
}

impl EffectsScene {
    pub fn new(fonts: &FontRegistry) -> EffectsScene {
        let note = Color::from_rgb(90, 90, 110);
        let mut session = create_session(FRAME_WIDTH, FRAME_HEIGHT, Color::from_rgb(214, 210, 200), fonts);
        add_header(&mut session, TITLE, DESCRIPTION, true);
        let notes = session.add_layer("Notes", note);
        label(notes, fonts, "a group's shadow: offset, softness and alpha bound to tracks", 230.0, 420.0, 14.0, note, TextAlign::Center);
        label(notes, fonts, "glow radius and colour bound to tracks", 640.0, 420.0, 14.0, note, TextAlign::Center);
        label(notes, fonts, "a focus pull: BlurRadius bound to tracks", 1050.0, 420.0, 14.0, note, TextAlign::Center);
        label(notes, fonts, "one shadow for a whole layer (DrawingLayer effects)", 340.0, 700.0, 14.0, note, TextAlign::Center);
        label(notes, fonts, "over transparency, on a checkerboard", PANEL_X + 280.0, 700.0, 14.0, note, TextAlign::Center);
        checkerboard(notes, PANEL_X, PANEL_Y, 560.0, 230.0);
        session.add_layer("Things", INK);

        // The lifting card.
        let mut card = DrawingGroup::new(None);
        card.add(fill_rect(100.0, 170.0, 260.0, 180.0, Some(Color::from_rgb(250, 248, 244)), 14.0));
        card.add(disc(150.0, 220.0, 26.0, Some(Color::from_rgb(255, 176, 72))));
        card.add(TextShape::new("Lift", 192.0, 232.0, 34.0, fonts, TextShapeOptions { color: Some(Color::from_rgb(40, 40, 60)), ..TextShapeOptions::default() }).unwrap());
        card.add(fill_rect(126.0, 280.0, 208.0, 10.0, Some(Color::from_rgb(210, 210, 220)), 5.0));
        card.add(fill_rect(126.0, 302.0, 150.0, 10.0, Some(Color::from_rgb(210, 210, 220)), 5.0));
        let card = session.draw_shape(DrawingShape::from(card).with_effects(DrawingEffects::drop_shadow(PointF::new(0.0, 4.0), 6.0, Color::from_rgba(0, 0, 0, 120)).unwrap()));
        let mut a = session.animations_mut();
        a.bind(card, AnimatedProperty::Position, loop_track(&[(0.0, 0.0, 10.0, L), (2.0, 0.0, -14.0, EasingKind::SineInOut), (4.0, 0.0, 10.0, EasingKind::SineInOut)])).unwrap();
        a.bind(card, AnimatedProperty::ShadowOffset, loop_track(&[(0.0, 0.0, 4.0, L), (2.0, 6.0, 30.0, EasingKind::SineInOut), (4.0, 0.0, 4.0, EasingKind::SineInOut)])).unwrap();
        a.bind(card, AnimatedProperty::ShadowBlurRadius, floats(&[(0.0, 6.0, L), (2.0, 34.0, EasingKind::SineInOut), (4.0, 6.0, EasingKind::SineInOut)]).with_loop_mode(TrackLoopMode::Loop)).unwrap();
        a.bind(card, AnimatedProperty::ShadowColor, ColorTrack::from_keys(&[(0.0, Color::from_rgba(0, 0, 0, 130), L), (2.0, Color::from_rgba(0, 0, 0, 80), EasingKind::SineInOut), (4.0, Color::from_rgba(0, 0, 0, 130), EasingKind::SineInOut)]).unwrap().with_loop_mode(TrackLoopMode::Loop)).unwrap();

        // The glowing star.
        let star = PathShape::with_fill(PathGeometry::star(PointF::new(640.0, 265.0), 95.0, 42.0, 5, 0.0).unwrap(), RadialGradientFill::from_colors(PointF::new(640.0, 255.0), 100.0, Color::from_rgb(255, 250, 220), Color::from_rgb(255, 190, 60)).unwrap().into());
        let star = session.draw_shape(DrawingShape::from(star).with_effects(DrawingEffects::glow(16.0, Color::from_rgb(255, 140, 40), 1.5).unwrap()));
        let mut a = session.animations_mut();
        a.bind(star, AnimatedProperty::GlowRadius, floats(&[(0.0, 10.0, L), (1.0, 44.0, EasingKind::SineInOut), (2.0, 10.0, EasingKind::SineInOut)]).with_loop_mode(TrackLoopMode::Loop)).unwrap();
        a.bind(star, AnimatedProperty::GlowColor, ColorTrack::from_keys(&[(0.0, Color::from_rgb(255, 140, 40), L), (4.0, Color::from_rgb(255, 60, 160), EasingKind::SineInOut), (8.0, Color::from_rgb(255, 140, 40), EasingKind::SineInOut)]).unwrap()).unwrap();

        // The focus pull.
        let back = session.draw_circle(1000.0, 250.0, 80.0, None, Some(Color::from_rgb(90, 150, 230)), true).unwrap();
        let front = session.draw_rectangle(1030.0, 230.0, 150.0, 120.0, None, Some(Color::from_rgb(240, 110, 90)), true, 12.0).unwrap();
        let mut a = session.animations_mut();
        a.bind(back, AnimatedProperty::BlurRadius, floats(&[(0.0, 0.0, L), (1.5, 0.0, L), (3.0, 22.0, EasingKind::SineInOut), (5.5, 22.0, L), (7.0, 0.0, EasingKind::SineInOut)])).unwrap();
        a.bind(front, AnimatedProperty::BlurRadius, floats(&[(0.0, 22.0, L), (1.5, 22.0, L), (3.0, 0.0, EasingKind::SineInOut), (5.5, 0.0, L), (7.0, 22.0, EasingKind::SineInOut)])).unwrap();

        // One shadow for a whole layer.
        let blue = Color::from_rgb(70, 110, 200);
        session.add_layer("Titled", blue);
        session.draw_text("Shadows", 70.0, 560.0, 84.0, TextShapeOptions { color: Some(blue), ..TextShapeOptions::default() }).unwrap();
        session.draw_circle(505.0, 530.0, 40.0, None, Some(Color::from_rgb(255, 176, 72)), true).unwrap();
        session.draw_rectangle(565.0, 495.0, 70.0, 70.0, None, Some(Color::from_rgb(140, 210, 160)), true, 8.0).unwrap();
        session.draw_line(80.0, 610.0, 630.0, 610.0, Some(8.0), Some(blue)).unwrap();
        session.get_layer_mut("Titled").unwrap().set_effects(Some(DrawingEffects::drop_shadow(PointF::new(8.0, 10.0), 8.0, Color::from_rgba(30, 20, 60, 150)).unwrap()));
        session.animations_mut().bind(BindingTarget::layer("Titled"), AnimatedProperty::ShadowOffset, points(&[(0.0, -10.0, 10.0, L), (8.0, 14.0, 10.0, EasingKind::SineInOut)])).unwrap();

        // The same effects over transparency (the panel's own coordinates moved onto the frame).
        session.add_layer("Panel", INK);
        let at = |x: f32, y: f32| (PANEL_X + x, PANEL_Y + y);
        let (x, y) = at(110.0, 115.0);
        let sun = session.draw_circle(x, y, 60.0, None, Some(Color::from_rgb(255, 210, 90)), true).unwrap();
        session.find_element_mut(sun).unwrap().set_effects(Some(DrawingEffects::glow(30.0, Color::from_rgb(255, 120, 40), 1.6).unwrap().with_shadow(PointF::new(14.0, 18.0), 14.0, Color::from_rgba(0, 0, 0, 200)).unwrap()));
        let wave = PathGeometry::parse(&format!("M {} {} C {} {}, {} {}, {} {} S {} {}, {} {}", at(230.0, 170.0).0, at(230.0, 170.0).1, at(260.0, 40.0).0, at(260.0, 40.0).1, at(330.0, 40.0).0, at(330.0, 40.0).1, at(350.0, 120.0).0, at(350.0, 120.0).1, at(440.0, 200.0).0, at(440.0, 200.0).1, at(480.0, 60.0).0, at(480.0, 60.0).1)).unwrap();
        let wave = PathShape::with_style(wave, PathStrokeStyle::new(16.0, PathStrokeStyleOptions::default()).unwrap(), Some(Color::from_rgb(120, 200, 255)), None, None);
        session.draw_shape(DrawingShape::from(wave).with_effects(DrawingEffects::drop_shadow(PointF::new(10.0, 12.0), 10.0, Color::from_rgba(0, 0, 40, 180)).unwrap()));
        let (x, y) = at(430.0, 140.0);
        let blurred = session.draw_rectangle(x, y, 100.0, 70.0, None, Some(Color::from_rgb(240, 110, 90)), true, 10.0).unwrap();
        session.animations_mut().bind(blurred, AnimatedProperty::BlurRadius, floats(&[(0.0, 0.0, L), (4.0, 20.0, EasingKind::SineInOut), (8.0, 0.0, EasingKind::SineInOut)])).unwrap();
        EffectsScene { session }
    }
}

impl DemoScene for EffectsScene {
    fn name(&self) -> &'static str {
        "effects"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
