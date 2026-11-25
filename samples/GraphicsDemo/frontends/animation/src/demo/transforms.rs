//! "transforms" (AnimationDemoApp's TransformScene): element transforms bound to tracks - a bar
//! turning about its left end, a square scaling and rocking about its centre, an arrow fading -
//! with their TRANSFORMED BOUNDS outlined every frame; a layer transform turning and breathing a
//! whole "orbit" layer; and, in a session of its own drawn without its background over a
//! checkerboard, two discs faded one by one (their overlap shows) beside the same two in a GROUP
//! faded as one unit (it does not).

use codebrix_webgpu_animation::animation::{EasingKind, TrackLoopMode};
use codebrix_webgpu_animation::models::{AnimatedProperty, BindingTarget, ElementId};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{DrawingGroup, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, SessionRenderOptions};

use crate::demo::kit::{checkerboard, disc, floats, outline_rect, part, points, L};
use crate::demo::{create_frame_session, create_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Transforms, opacity and groups";
const DESCRIPTION: &str = "DrawingElement.Transform/Opacity, DrawingGroup (fades as one unit), DrawingLayer.Transform; GetTransformedBounds";
const PANEL_WIDTH: f32 = 360.0;
const PANEL_HEIGHT: f32 = 520.0;
const PANEL_X: f32 = 460.0;
const PANEL_Y: f32 = 150.0;

pub struct TransformScene {
    frame: DrawingSession,
    panel: DrawingSession,
    outlined: Vec<ElementId>,
}

impl TransformScene {
    pub fn new(fonts: &FontRegistry) -> TransformScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = frame.add_layer("Notes", DIM);
        label(notes, fonts, "element transforms (bounds outlined)", 40.0, 140.0, 14.0, DIM, TextAlign::Left);
        label(notes, fonts, "faded elements | faded group (over transparency)", PANEL_X, 140.0, 14.0, DIM, TextAlign::Left);
        label(notes, fonts, "a layer transform (the whole layer)", 880.0, 140.0, 14.0, DIM, TextAlign::Left);
        checkerboard(notes, PANEL_X, PANEL_Y, PANEL_WIDTH, PANEL_HEIGHT);
        frame.add_layer("Shapes", ACCENT);
        let bar = frame.draw_rectangle(200.0, 281.0, 110.0, 18.0, None, Some(Color::from_rgb(120, 170, 255)), true, 0.0).unwrap();
        let square = frame.draw_rectangle(170.0, 430.0, 80.0, 80.0, None, Some(Color::from_rgb(140, 230, 160)), true, 10.0).unwrap();
        let arrow = frame.draw_arrow(80.0, 660.0, 330.0, 620.0, Some(16.0), Some(Color::from_rgba(255, 120, 150, 255)), Some(60.0)).unwrap();
        let mut a = frame.animations_mut();
        a.bind(bar, AnimatedProperty::Anchor, points(&[(0.0, 200.0, 290.0, L)])).unwrap();
        a.bind(bar, AnimatedProperty::Rotation, floats(&[(0.0, 0.0, L), (4.0, 360.0, L)]).with_loop_mode(TrackLoopMode::Loop)).unwrap();
        a.bind(square, AnimatedProperty::Anchor, points(&[(0.0, 210.0, 470.0, L)])).unwrap();
        a.bind(square, AnimatedProperty::Scale, floats(&[(0.0, 0.5, L), (2.0, 1.4, EasingKind::SineInOut)]).with_loop_mode(TrackLoopMode::PingPong)).unwrap();
        a.bind(square, AnimatedProperty::Rotation, floats(&[(0.0, -20.0, L), (2.0, 20.0, EasingKind::SineInOut)]).with_loop_mode(TrackLoopMode::PingPong)).unwrap();
        a.bind(arrow, AnimatedProperty::Opacity, floats(&[(0.0, 1.0, L), (2.0, 0.25, EasingKind::SineInOut)]).with_loop_mode(TrackLoopMode::PingPong)).unwrap();
        frame.add_layer("Orbit", Color::from_rgb(255, 210, 120));
        frame.draw_circle(1050.0, 410.0, 26.0, None, None, true).unwrap();
        for i in 0..6 {
            let angle = i as f64 * std::f64::consts::PI / 3.0;
            frame.draw_circle(1050.0 + (130.0 * angle.cos() as f32), 410.0 + (130.0 * angle.sin() as f32), 14.0, None, Some(Color::from_rgb(120 + (i * 20) as u8, 170, 255)), true).unwrap();
        }
        frame.draw_text("layer", 1050.0, 560.0, 18.0, TextShapeOptions { color: Some(INK), align: TextAlign::Center, ..TextShapeOptions::default() }).unwrap();
        let mut a = frame.animations_mut();
        a.bind(BindingTarget::layer("Orbit"), AnimatedProperty::Anchor, points(&[(0.0, 1050.0, 410.0, L)])).unwrap();
        a.bind(BindingTarget::layer("Orbit"), AnimatedProperty::Rotation, floats(&[(0.0, 0.0, L), (8.0, 360.0, L)]).with_loop_mode(TrackLoopMode::Loop)).unwrap();
        a.bind(BindingTarget::layer("Orbit"), AnimatedProperty::Scale, floats(&[(0.0, 0.8, L), (2.0, 1.1, EasingKind::SineInOut)]).with_loop_mode(TrackLoopMode::PingPong)).unwrap();
        frame.add_layer("Bounds", Color::from_rgba(236, 236, 244, 120));

        // The faded discs, in a session of their own drawn over transparency.
        let mut panel = create_session(PANEL_WIDTH as u32, PANEL_HEIGHT as u32, Color::TRANSPARENT, fonts);
        panel.add_layer("Discs", Color::from_rgb(255, 176, 72));
        let left = panel.draw_circle(130.0, 150.0, 80.0, None, None, true).unwrap();
        let right = panel.draw_circle(230.0, 150.0, 80.0, None, Some(Color::from_rgb(120, 170, 255)), true).unwrap();
        let mut pair = DrawingGroup::new(None);
        pair.add(disc(130.0, 380.0, 80.0, None));
        pair.add(disc(230.0, 380.0, 80.0, Some(Color::from_rgb(120, 170, 255))));
        let pair = panel.draw_shape(pair);
        let fade = floats(&[(0.0, 1.0, L), (2.0, 0.35, EasingKind::SineInOut)]).with_loop_mode(TrackLoopMode::PingPong);
        for id in [left, right, pair] {
            panel.animations_mut().bind(id, AnimatedProperty::Opacity, fade.clone()).unwrap();
        }
        TransformScene { frame, panel, outlined: vec![bar, square, arrow] }
    }
}

impl DemoScene for TransformScene {
    fn name(&self) -> &'static str {
        "transforms"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.frame.set_time(seconds);
        let boxes: Vec<_> = self.outlined.iter().filter_map(|id| self.frame.find_element(*id).and_then(|e| e.get_transformed_bounds())).collect();
        let bounds = self.frame.get_layer_mut("Bounds").unwrap();
        bounds.clear();
        for b in boxes {
            bounds.add_shape(outline_rect(b.x, b.y, b.width, b.height, 1.5, None, 0.0));
        }
        self.frame.render_with_background(scene, width, height);
        self.panel.set_time(seconds);
        let target = part(&self.frame, PANEL_X, PANEL_Y, PANEL_WIDTH, PANEL_HEIGHT, width, height);
        self.panel.render_with(scene, width, height, &SessionRenderOptions { include_background: false, target: Some(target), ..Default::default() });
    }
}
