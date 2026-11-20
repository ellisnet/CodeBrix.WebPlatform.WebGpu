//! "transitions" (AnimationDemoApp's TransitionScene): the planes' DrawingTransitions between two
//! opaque "cards" (A and B: a gradient, a ring and a big letter), six at once while the fraction
//! eases from 0 to 1 and back with short holds - here ON THE GPU: each panel is a session of its own
//! with the two cards as LAYERS and `set_layer_transition`, drawn into its part of the frame (C#
//! mixes 8-bit frames on the CPU). Fraction 0 is the outgoing card and 1 the incoming card
//! exactly; slides and pushes move whole pixels.

use codebrix_webgpu_animation::fills::LinearGradientFill;
use codebrix_webgpu_animation::planes::{DrawingTransition, DrawingTransitionDirection};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{RectangleShape, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, SessionRenderOptions};

use crate::demo::kit::{part, ring};
use crate::demo::{create_frame_session, create_session, label, DemoScene, ACCENT, FRAME_WIDTH, INK};

const TITLE: &str = "Transitions";
const DESCRIPTION: &str = "Dissolve, wipe, slide, push and fade through a colour between two layers - drawn on the GPU, exact at 0 and 1";
const PANEL_WIDTH: u32 = 384;
const PANEL_HEIGHT: u32 = 216;
const PANEL_X: [f32; 3] = [32.0, 448.0, 864.0];
const PANEL_Y: [f32; 2] = [116.0, 400.0];

pub struct TransitionScene {
    frame: DrawingSession,
    panels: Vec<DrawingSession>,
    fonts: FontRegistry,
}

/// One opaque card on a layer of its own: a gradient, a ring and a big letter.
fn add_card(session: &mut DrawingSession, name: &str, letter: &str, top: Color, bottom: Color, ink: Color, fonts: &FontRegistry) {
    let gradient = LinearGradientFill::from_colors(PointF::new(0.0, 0.0), PointF::new(0.0, PANEL_HEIGHT as f32), top, bottom).unwrap();
    let card = session.add_layer(name, ink);
    card.add_shape(RectangleShape::with_fill(0.0, 0.0, PANEL_WIDTH as f32, PANEL_HEIGHT as f32, gradient.into(), 0.0).unwrap());
    card.add_shape(ring(110.0, 108.0, 64.0, 10.0, Some(ink)));
    card.add_shape(TextShape::new(letter, 270.0, 150.0, 120.0, fonts, TextShapeOptions { color: Some(ink), align: TextAlign::Center, ..TextShapeOptions::default() }).unwrap());
}

fn transitions() -> [(&'static str, DrawingTransition); 6] {
    [
        ("dissolve()", DrawingTransition::dissolve()),
        ("wipe(Right, softness 60)", DrawingTransition::wipe(DrawingTransitionDirection::Right, Some(60.0)).unwrap()),
        ("slide(Left)", DrawingTransition::slide(DrawingTransitionDirection::Left)),
        ("push(Up)", DrawingTransition::push(DrawingTransitionDirection::Up)),
        ("fade_through_color(Black)", DrawingTransition::fade_through_color(Color::BLACK)),
        ("wipe(Down) - hard edge", DrawingTransition::wipe(DrawingTransitionDirection::Down, None).unwrap()),
    ]
}

impl TransitionScene {
    pub fn new(fonts: &FontRegistry) -> TransitionScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let captions = frame.add_layer("Captions", INK);
        for (i, (caption, _)) in transitions().iter().enumerate() {
            label(captions, fonts, &format!("DrawingTransition::{caption}"), PANEL_X[i % 3], PANEL_Y[i / 3] + PANEL_HEIGHT as f32 + 22.0, 14.0, INK, TextAlign::Left);
        }
        frame.add_layer("Notes", ACCENT);
        let mut panels = Vec::new();
        for _ in 0..6 {
            let mut panel = create_session(PANEL_WIDTH, PANEL_HEIGHT, Color::BLACK, fonts);
            add_card(&mut panel, "A", "A", Color::from_rgb(240, 120, 60), Color::from_rgb(120, 30, 70), Color::from_rgb(255, 230, 160), fonts);
            add_card(&mut panel, "B", "B", Color::from_rgb(40, 150, 200), Color::from_rgb(20, 40, 100), Color::from_rgb(200, 255, 220), fonts);
            panels.push(panel);
        }
        TransitionScene { frame, panels, fonts: fonts.clone() }
    }
}

/// C#'s PingPong(seconds, period): 0 -> 1 -> 0 over twice the period.
fn ping_pong(seconds: f64, period: f64) -> f64 {
    let t = (seconds / period).rem_euclid(2.0);
    if t <= 1.0 {
        t
    } else {
        2.0 - t
    }
}

impl DemoScene for TransitionScene {
    fn name(&self) -> &'static str {
        "transitions"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        // Short holds at 0 and 1.
        let fraction = (ping_pong(seconds, 4.0) * 1.25 - 0.125).clamp(0.0, 1.0) as f32;
        self.frame.set_time(seconds);
        let fonts = self.fonts.clone();
        let notes = self.frame.get_layer_mut("Notes").unwrap();
        notes.clear();
        label(notes, &fonts, &format!("fraction = {fraction:.3}"), FRAME_WIDTH as f32 - 32.0, 74.0, 16.0, ACCENT, TextAlign::Right);
        self.frame.render_with_background(scene, width, height);
        for (i, (_, transition)) in transitions().into_iter().enumerate() {
            let panel = &mut self.panels[i];
            panel.set_time(seconds);
            panel.set_layer_transition("A", "B", transition, fraction).unwrap();
            let target = part(&self.frame, PANEL_X[i % 3], PANEL_Y[i / 3], PANEL_WIDTH as f32, PANEL_HEIGHT as f32, width, height);
            panel.render_with(scene, width, height, &SessionRenderOptions { include_background: true, target: Some(target) });
        }
    }
}
