//! "draw-on" (AnimationDemoApp's DrawOnScene): a signature flourish (SVG path data) WRITTEN by a
//! TrimEnd track while a nib follows it along the path (BindAlongPath on the same track), then
//! underlined; a loading ring whose TrimStart and TrimEnd chase each other while its TrimOffset
//! turns; a fixed 30 % piece running round a star (TrimOffset); marching ants (a dashed rounded
//! rectangle whose dash offset moves, an AnimatedShape). The nib glows, as in C# (DrawingEffects::glow).

use codebrix_webgpu_animation::animation::{EasingKind, TrackLoopMode};
use codebrix_webgpu_animation::effects::DrawingEffects;
use codebrix_webgpu_animation::models::{AnimatedProperty, PathTravelOptions};
use codebrix_webgpu_animation::paths::{PathGeometry, PathStrokeCap, PathStrokeJoin, PathStrokeStyle, PathStrokeStyleOptions};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{AnimatedShape, DrawingGroup, PathShape};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::kit::{disc, floats, L};
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Draw-on: trimming a path";
const DESCRIPTION: &str = "PathGeometry.Parse (SVG path data), PathShape.TrimStart/TrimEnd/TrimOffset bound to tracks, BindAlongPath, dashes";
const FLOURISH: &str = "M 150 330 C 190 200, 280 190, 260 280 C 245 350, 170 380, 190 300 C 210 230, 330 220, 320 300 C 315 345, 360 350, 385 300 S 440 220, 455 290 C 465 335, 510 345, 530 300 C 545 265, 590 262, 592 300 C 594 340, 640 348, 670 300 Q 720 220 770 300 T 870 300 C 895 260, 935 250, 960 250 C 1000 250, 990 330, 1040 320 C 1080 312, 1100 270, 1130 240";

fn style(width: f32) -> PathStrokeStyle {
    PathStrokeStyle::new(width, PathStrokeStyleOptions::default()).unwrap()
}

pub struct DrawOnScene {
    session: DrawingSession,
}

impl DrawOnScene {
    pub fn new(fonts: &FontRegistry) -> DrawOnScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = session.add_layer("Notes", DIM);
        label(notes, fonts, "TrimEnd 0 -> 1 writes the path; the nib follows GetPointAt(progress)", 40.0, 140.0, 14.0, DIM, TextAlign::Left);
        label(notes, fonts, "loading ring: TrimStart / TrimEnd / TrimOffset", 180.0, 470.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "a 30% piece running round (wraps the start)", 640.0, 470.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "marching ants: a moving dash offset", 1110.0, 470.0, 14.0, DIM, TextAlign::Center);
        session.add_layer("Ink", INK);
        let flourish = PathGeometry::parse(FLOURISH).unwrap();
        session.draw_path(PathShape::with_style(flourish.clone(), style(1.5), Some(Color::from_rgba(132, 138, 164, 60)), None, None));
        let mut signature = PathShape::with_style(flourish.clone(), style(7.0), Some(Color::from_rgb(120, 180, 255)), None, None);
        signature.set_trim_end(0.0).unwrap();
        let signature = session.draw_path(signature);
        let writing = floats(&[(0.3, 0.0, L), (5.0, 1.0, EasingKind::SineInOut)]);
        session.animations_mut().bind(signature, AnimatedProperty::TrimEnd, writing.clone()).unwrap();
        let mut underline = PathShape::with_style(PathGeometry::parse("M 170 400 C 450 380, 800 390, 1110 370").unwrap(), style(4.0), Some(ACCENT), None, None);
        underline.set_trim_end(0.0).unwrap();
        let underline = session.draw_path(underline);
        session.animations_mut().bind(underline, AnimatedProperty::TrimEnd, floats(&[(5.0, 0.0, L), (6.0, 1.0, EasingKind::CubicOut)])).unwrap();
        let mut nib = DrawingGroup::new(None);
        nib.add(disc(0.0, 0.0, 7.0, Some(Color::WHITE)));
        let nib = session.draw_shape(nib);
        session.find_element_mut(nib).unwrap().set_effects(Some(DrawingEffects::glow(18.0, Color::from_rgba(120, 180, 255, 255), 1.6).unwrap()));
        session.animations_mut().bind_along_path(nib, &flourish, writing, PathTravelOptions::default()).unwrap();
        session.animations_mut().bind(nib, AnimatedProperty::Opacity, floats(&[(0.2, 0.0, L), (0.4, 1.0, L), (4.9, 1.0, L), (5.3, 0.0, L)])).unwrap();
        // The loading ring: the ends chase each other while the whole piece turns.
        session.draw_circle(180.0, 590.0, 60.0, Some(10.0), Some(Color::from_rgba(255, 176, 72, 40)), false).unwrap();
        let ring = session.draw_path(PathShape::with_style(PathGeometry::circle(PointF::new(180.0, 590.0), 60.0), style(10.0), Some(ACCENT), None, None));
        let head = floats(&[(0.0, 0.05, L), (0.9, 0.75, EasingKind::CubicInOut), (1.8, 0.8, L)]).with_loop_mode(TrackLoopMode::Loop);
        let tail = floats(&[(0.0, 0.0, L), (0.9, 0.05, L), (1.8, 0.75, EasingKind::CubicInOut)]).with_loop_mode(TrackLoopMode::Loop);
        let mut a = session.animations_mut();
        a.bind(ring, AnimatedProperty::TrimEnd, head).unwrap();
        a.bind(ring, AnimatedProperty::TrimStart, tail).unwrap();
        a.bind(ring, AnimatedProperty::TrimOffset, floats(&[(0.0, 0.0, L), (8.0, 3.0, L)])).unwrap();
        // A fixed piece running round a star: start 0, end 0.3, the offset turning.
        let star = PathGeometry::star(PointF::new(640.0, 600.0), 95.0, 42.0, 5, 0.0).unwrap();
        session.draw_path(PathShape::with_style(star.clone(), style(2.0), Some(Color::from_rgba(132, 138, 164, 90)), None, None));
        let mut runner = PathShape::with_style(star, PathStrokeStyle::new(8.0, PathStrokeStyleOptions { cap: PathStrokeCap::Round, join: PathStrokeJoin::Round, ..PathStrokeStyleOptions::default() }).unwrap(), Some(Color::from_rgb(140, 230, 160)), None, None);
        runner.set_trim_end(0.3).unwrap();
        let runner = session.draw_path(runner);
        session.animations_mut().bind(runner, AnimatedProperty::TrimOffset, floats(&[(0.0, 0.0, L), (8.0, 2.0, L)])).unwrap();
        // Marching ants: a dashed rounded rectangle rebuilt for each moment with a moving dash offset.
        let rounded = PathGeometry::rounded_rectangle(RectangleF::new(1010.0, 520.0, 200.0, 140.0), 18.0);
        let dashes = PathStrokeStyle::new(4.0, PathStrokeStyleOptions { cap: PathStrokeCap::Butt, join: PathStrokeJoin::Round, miter_limit: 4.0, dash_pattern: Some(vec![14.0, 9.0]), dash_offset: 0.0, ..Default::default() }).unwrap();
        session.draw_shape(AnimatedShape::new(move |t| Some(PathShape::with_style(rounded.clone(), dashes.with_dash_offset(((-t * 46.0) % 23.0) as f32), Some(INK), None, None).into()), 0.0));
        DrawOnScene { session }
    }
}

impl DemoScene for DrawOnScene {
    fn name(&self) -> &'static str {
        "draw-on"
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
