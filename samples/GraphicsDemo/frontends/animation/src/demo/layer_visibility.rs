//! "layer-visibility" (AnimationDemoApp's LayerVisibilityScene): layers switched on and off
//! (IsVisible bound to bool tracks): a hidden layer is skipped entirely and switches exactly at its
//! keys' times. Left, five card layers revealed one after another and taken away in reverse;
//! middle, a blinking "on air" layer (a looping track); right, a matte layer that stays hidden yet
//! still masks the striped layer (DrawingMask::from_layer), shown on its own for a moment near the
//! end. Below, every layer's state read back from the session.

use codebrix_webgpu_animation::animation::{BoolTrack, EasingKind, TrackLoopMode};
use codebrix_webgpu_animation::effects::DrawingMask;
use codebrix_webgpu_animation::fills::LinearGradientFill;
use codebrix_webgpu_animation::models::{AnimatedProperty, BindingTarget};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{DrawingGroup, RectangleShape, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::kit::{disc, fill_rect, line, outline_rect, points, L};
use crate::demo::{create_frame_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Layer visibility: switching layers on and off";
const DESCRIPTION: &str = "DrawingLayer.IsVisible bound to a BoolTrack (AnimatedProperty.IsVisible): exact steps, hidden layers skipped, a hidden matte still masks";
const CARD_COLORS: [Color; 5] = [Color::from_rgb(240, 84, 110), Color::from_rgb(255, 128, 72), Color::from_rgb(255, 176, 72), Color::from_rgb(120, 200, 140), Color::from_rgb(120, 160, 255)];

pub struct LayerVisibilityScene {
    session: DrawingSession,
    watched: Vec<String>,
    fonts: FontRegistry,
}

fn bools(keys: &[(f64, bool)]) -> BoolTrack {
    let keys: Vec<(f64, bool, EasingKind)> = keys.iter().map(|(t, v)| (*t, *v, L)).collect();
    BoolTrack::from_keys(&keys).unwrap()
}

fn text(s: &str, x: f32, y: f32, size: f32, fonts: &FontRegistry, color: Color, align: TextAlign) -> TextShape {
    TextShape::new(s, x, y, size, fonts, TextShapeOptions { color: Some(color), align, ..TextShapeOptions::default() }).unwrap()
}

impl LayerVisibilityScene {
    pub fn new(fonts: &FontRegistry) -> LayerVisibilityScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = session.add_layer("Notes", DIM);
        label(notes, fonts, "revealed in sequence, removed in reverse", 40.0, 128.0, 14.0, INK, TextAlign::Left);
        label(notes, fonts, "a looping BoolTrack: 0.5 s on, 0.3 s off", 470.0, 128.0, 14.0, INK, TextAlign::Left);
        label(notes, fonts, "a hidden matte layer still masks", 860.0, 128.0, 14.0, INK, TextAlign::Left);
        notes.add_shape(outline_rect(470.0, 160.0, 340.0, 260.0, 1.5, Some(Color::from_rgba(132, 138, 164, 90)), 12.0));
        notes.add_shape(outline_rect(860.0, 160.0, 380.0, 260.0, 1.5, Some(Color::from_rgba(132, 138, 164, 90)), 12.0));
        label(notes, fonts, "the Matte layer (IsVisible = false) masks the Stripes layer;", 860.0, 442.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "it is shown itself from 6.4 s to 7.4 s", 860.0, 462.0, 13.0, DIM, TextAlign::Left);
        let mut watched = Vec::new();
        // Five cards, each on its own layer, fanned out.
        for (i, color) in CARD_COLORS.iter().enumerate() {
            let name = format!("Card {}", i + 1);
            let card = session.add_layer(&name, INK);
            let (x, y) = (50.0 + i as f32 * 44.0, 160.0 + i as f32 * 34.0);
            card.add_shape(fill_rect(x, y, 200.0, 130.0, Some(*color), 12.0));
            card.add_shape(outline_rect(x, y, 200.0, 130.0, 2.0, Some(Color::from_rgba(18, 20, 30, 200)), 12.0));
            card.add_shape(text(&format!("layer {}", i + 1), x + 16.0, y + 34.0, 20.0, fonts, Color::from_rgb(24, 24, 36), TextAlign::Left));
            session.animations_mut().bind(BindingTarget::layer(&name), AnimatedProperty::IsVisible, bools(&[(0.0, false), (0.5 + i as f64 * 0.6, true), (5.6 + (4 - i) as f64 * 0.3, false)])).unwrap();
            watched.push(name);
        }
        // The blinking sign.
        let on_air = session.add_layer("On air", INK);
        on_air.add_shape(fill_rect(530.0, 250.0, 220.0, 80.0, Some(Color::from_rgb(60, 20, 30)), 14.0));
        on_air.add_shape(outline_rect(530.0, 250.0, 220.0, 80.0, 3.0, Some(Color::from_rgb(240, 84, 110)), 14.0));
        on_air.add_shape(disc(570.0, 290.0, 14.0, Some(Color::from_rgb(255, 60, 70))));
        on_air.add_shape(text("ON AIR", 596.0, 302.0, 32.0, fonts, Color::from_rgb(255, 220, 220), TextAlign::Left));
        let blink = bools(&[(0.0, true), (0.5, false), (0.8, true)]).with_loop_mode(TrackLoopMode::Loop);
        session.animations_mut().bind(BindingTarget::layer("On air"), AnimatedProperty::IsVisible, blink).unwrap();
        watched.push("On air".to_string());
        // The matte: a word sliding across, on a layer that is hidden except for a moment near the end.
        let word = session.add_layer("Matte", Color::WHITE).add_element(text("MATTE", 1050.0, 320.0, 92.0, fonts, Color::WHITE, TextAlign::Center));
        session.animations_mut().bind(word, AnimatedProperty::Position, points(&[(0.0, -24.0, 0.0, L), (4.0, 24.0, 0.0, EasingKind::SineInOut), (8.0, -24.0, 0.0, EasingKind::SineInOut)])).unwrap();
        session.animations_mut().bind(BindingTarget::layer("Matte"), AnimatedProperty::IsVisible, bools(&[(0.0, false), (6.4, true), (7.4, false)])).unwrap();
        watched.push("Matte".to_string());
        let stripes = session.add_layer("Stripes", INK);
        let gradient = LinearGradientFill::from_colors(PointF::new(870.0, 0.0), PointF::new(1230.0, 0.0), Color::from_rgb(255, 120, 80), Color::from_rgb(120, 120, 255)).unwrap();
        stripes.add_shape(RectangleShape::with_fill(870.0, 170.0, 360.0, 240.0, gradient.into(), 0.0).unwrap());
        let mut moving = DrawingGroup::new(Some(Color::from_rgba(255, 255, 255, 120)));
        for i in 0..16 {
            moving.add(line(760.0 + i as f32 * 36.0, 420.0, 860.0 + i as f32 * 36.0, 160.0, 10.0, None));
        }
        let moving = stripes.add_element(moving);
        stripes.set_mask(Some(DrawingMask::from_layer("Matte", false)));
        session.animations_mut().bind(moving, AnimatedProperty::Position, points(&[(0.0, 0.0, 0.0, L), (8.0, 144.0, 0.0, L)])).unwrap();
        watched.push("Stripes".to_string());
        session.add_layer("Now", INK);
        LayerVisibilityScene { session, watched, fonts: fonts.clone() }
    }
}

impl DemoScene for LayerVisibilityScene {
    fn name(&self) -> &'static str {
        "layer-visibility"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        // Every watched layer's state, read back from the session.
        let states: Vec<(String, bool)> = self.watched.iter().map(|n| (n.clone(), self.session.get_layer(n).is_some_and(|l| l.is_visible()))).collect();
        let fonts = self.fonts.clone();
        let now = self.session.get_layer_mut("Now").unwrap();
        now.clear();
        label(now, &fonts, "DrawingLayer.IsVisible now:", 40.0, 540.0, 15.0, INK, TextAlign::Left);
        for (i, (name, on)) in states.iter().enumerate() {
            let (x, y) = (40.0 + (i % 5) as f32 * 240.0, 580.0 + (i / 5) as f32 * 44.0);
            now.add_shape(fill_rect(x, y - 22.0, 224.0, 32.0, Some(if *on { Color::from_rgb(40, 70, 52) } else { Color::from_rgb(36, 38, 52) }), 8.0));
            now.add_shape(disc(x + 18.0, y - 6.0, 7.0, Some(if *on { Color::from_rgb(110, 220, 140) } else { Color::from_rgb(90, 94, 116) })));
            now.add_shape(text(&format!("{name}{}", if *on { "   on" } else { "   off" }), x + 34.0, y, 16.0, &fonts, if *on { INK } else { DIM }, TextAlign::Left));
        }
        self.session.render_with_background(scene, width, height);
    }
}
