//! "effects-follow" (AnimationDemoApp's EffectsFollowScene): effects follow scale and camera. A
//! tabletop under a camera that zooms in and out and rolls a little: every shadow and glow grows
//! and shrinks with it, in proportion to the objects. Two cards spin: the left one's shadow is
//! ShadowDirection::FixedInScene (the light stays put, the shadow always falls the same way and
//! turns only with the camera's roll), the right one's RotatesWithElement (carried round with
//! the card). Between them a pulsing star shows glow and shadow scaling with the element's own
//! scale. The header, the read-out and the captions are on layers with parallax 0.

use codebrix_webgpu_animation::animation::{EasingKind, FloatTrack, TrackLoopMode};
use codebrix_webgpu_animation::effects::{DrawingEffects, ShadowDirection};
use codebrix_webgpu_animation::fills::RadialGradientFill;
use codebrix_webgpu_animation::models::{AnimatedProperty, BindingTarget, DrawingCamera, DrawingLayer};
use codebrix_webgpu_animation::paths::PathGeometry;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{ArrowShape, ArrowShapeOptions, DrawingGroup, DrawingShape, PathShape, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::kit::{disc, fill_rect, floats, line, points, L};
use crate::demo::{add_header, create_session, label, DemoScene, ACCENT, BACKDROP, DIM, FRAME_HEIGHT, FRAME_WIDTH, INK};

const TITLE: &str = "Effects follow scale and camera";
const DESCRIPTION: &str = "Shadows, glows and blurs scale with the element, its groups, its layer and the camera; ShadowDirection: fixed in the scene or turning with the element";

pub struct EffectsFollowScene {
    session: DrawingSession,
    fonts: FontRegistry,
    zoom: FloatTrack,
    roll: FloatTrack,
}

fn add_card(session: &mut DrawingSession, fonts: &FontRegistry, cx: f32, cy: f32, word: &str, color: Color, direction: ShadowDirection) {
    let mut card = DrawingGroup::new(None);
    card.add(fill_rect(cx - 110.0, cy - 70.0, 220.0, 140.0, Some(Color::from_rgb(250, 248, 244)), 16.0));
    card.add(fill_rect(cx - 110.0, cy - 70.0, 220.0, 34.0, Some(color), 16.0));
    card.add(fill_rect(cx - 110.0, cy - 50.0, 220.0, 14.0, Some(color), 0.0));
    card.add(TextShape::new(word, cx, cy + 34.0, 40.0, fonts, TextShapeOptions { color: Some(Color::from_rgb(40, 40, 60)), align: TextAlign::Center, ..TextShapeOptions::default() }).unwrap());
    let effects = DrawingEffects::drop_shadow_with_direction(PointF::new(22.0, 22.0), 10.0, Color::from_rgba(30, 24, 20, 150), direction).unwrap();
    let card = session.draw_shape(DrawingShape::from(card).with_effects(effects));
    let mut a = session.animations_mut();
    a.bind(card, AnimatedProperty::Anchor, points(&[(0.0, cx, cy, L)])).unwrap();
    a.bind(card, AnimatedProperty::Rotation, floats(&[(0.0, 0.0, L), (8.0, 360.0, L)])).unwrap();
}

impl EffectsFollowScene {
    pub fn new(fonts: &FontRegistry) -> EffectsFollowScene {
        let table_color = Color::from_rgb(222, 216, 204);
        let mut session = create_session(FRAME_WIDTH, FRAME_HEIGHT, BACKDROP, fonts);
        let table: &mut DrawingLayer = session.add_layer("Table", table_color);
        table.add_shape(fill_rect(40.0, 120.0, 1200.0, 520.0, Some(table_color), 24.0));
        // The light, fixed in the scene: a sun and an arrow the way its light falls.
        let sun = Color::from_rgb(255, 196, 70);
        table.add_shape(disc(230.0, 210.0, 26.0, Some(sun)));
        for i in 0..8 {
            let a = i as f64 * std::f64::consts::PI / 4.0;
            table.add_shape(line((230.0 + 34.0 * a.cos()) as f32, (210.0 + 34.0 * a.sin()) as f32, (230.0 + 44.0 * a.cos()) as f32, (210.0 + 44.0 * a.sin()) as f32, 4.0, Some(sun)));
        }
        table.add_shape(ArrowShape::new(270.0, 250.0, 325.0, 305.0, ArrowShapeOptions { stroke_thickness: 4.0, color: Some(Color::from_rgb(200, 150, 60)), head_length: Some(16.0) }).unwrap());
        label(table, fonts, "light", 206.0, 276.0, 16.0, Color::from_rgb(160, 120, 50), TextAlign::Left);
        session.add_layer("Things", INK);
        add_card(&mut session, fonts, 340.0, 390.0, "fixed", Color::from_rgb(70, 120, 220), ShadowDirection::FixedInScene);
        add_card(&mut session, fonts, 940.0, 390.0, "turns", Color::from_rgb(220, 90, 80), ShadowDirection::RotatesWithElement);
        let star = PathShape::with_fill(PathGeometry::star(PointF::new(640.0, 390.0), 70.0, 30.0, 5, 0.0).unwrap(), RadialGradientFill::from_colors(PointF::new(640.0, 380.0), 80.0, Color::from_rgb(255, 250, 220), Color::from_rgb(255, 176, 60)).unwrap().into());
        let effects = DrawingEffects::glow(14.0, Color::from_rgb(255, 120, 40), 1.4).unwrap().with_shadow(PointF::new(14.0, 16.0), 8.0, Color::from_rgba(40, 30, 20, 140)).unwrap();
        let star = session.draw_shape(DrawingShape::from(star).with_effects(effects));
        let mut a = session.animations_mut();
        a.bind(star, AnimatedProperty::Anchor, points(&[(0.0, 640.0, 390.0, L)])).unwrap();
        a.bind(star, AnimatedProperty::Scale, floats(&[(0.0, 0.6, L), (2.0, 1.25, EasingKind::SineInOut), (4.0, 0.6, EasingKind::SineInOut)]).with_loop_mode(TrackLoopMode::Loop)).unwrap();
        add_header(&mut session, TITLE, DESCRIPTION, true);
        session.get_layer_mut("Header").unwrap().set_parallax(0.0).unwrap();
        let hud = session.add_layer("Hud", INK);
        hud.set_parallax(0.0).unwrap();
        hud.add_shape(fill_rect(0.0, 652.0, FRAME_WIDTH as f32, 68.0, Some(BACKDROP), 0.0));
        label(hud, fonts, "ShadowDirection::FixedInScene - the light stays put", 340.0, 676.0, 15.0, INK, TextAlign::Center);
        label(hud, fonts, "the star: glow and shadow scale with the element", 640.0, 700.0, 13.0, DIM, TextAlign::Center);
        label(hud, fonts, "ShadowDirection::RotatesWithElement - turns with it", 940.0, 676.0, 15.0, INK, TextAlign::Center);
        session.add_layer("Readout", ACCENT).set_parallax(0.0).unwrap();
        let zoom = floats(&[(0.0, 0.85, L), (2.5, 1.45, EasingKind::SineInOut), (5.5, 0.85, EasingKind::SineInOut), (8.0, 0.85, L)]);
        let roll = floats(&[(0.0, 0.0, L), (4.0, 0.0, L), (5.5, 14.0, EasingKind::SineInOut), (7.5, 0.0, EasingKind::SineInOut)]);
        session.set_camera(Some(DrawingCamera::new()));
        let mut a = session.animations_mut();
        a.bind(BindingTarget::Camera, AnimatedProperty::Scale, zoom.clone()).unwrap();
        a.bind(BindingTarget::Camera, AnimatedProperty::Rotation, roll.clone()).unwrap();
        EffectsFollowScene { session, fonts: fonts.clone(), zoom, roll }
    }
}

impl DemoScene for EffectsFollowScene {
    fn name(&self) -> &'static str {
        "effects-follow"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        let text = format!("camera zoom {:.2}x  roll {:.0} deg", self.zoom.evaluate(seconds).unwrap_or(1.0), self.roll.evaluate(seconds).unwrap_or(0.0));
        let fonts = self.fonts.clone();
        let readout = self.session.get_layer_mut("Readout").unwrap();
        readout.clear();
        label(readout, &fonts, &text, 1248.0, 46.0, 16.0, ACCENT, TextAlign::Right);
        self.session.render_with_background(scene, width, height);
    }
}
