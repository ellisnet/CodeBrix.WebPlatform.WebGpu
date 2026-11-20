//! "motion-path" (AnimationDemoApp's MotionPathScene): a figure eight built with the
//! PathGeometryBuilder and a rounded track; a ball moved along the eight (BindAlongPath, not
//! turned), a dart moved along it and turned to face along it (orient_to_path), and a car - a
//! group - driving the track, eased; the dart's tangent drawn as an arrow from GetPointAt with
//! the distance and heading read out. The dart casts a drop shadow, as in C#.

use codebrix_webgpu_animation::animation::{EasingKind, FloatTrack, TrackLoopMode};
use codebrix_webgpu_animation::effects::DrawingEffects;
use codebrix_webgpu_animation::models::PathTravelOptions;
use codebrix_webgpu_animation::paths::{PathGeometry, PathGeometryBuilder, PathStrokeStyle, PathStrokeStyleOptions};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{ArrowShape, ArrowShapeOptions, DrawingGroup, PathShape};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::kit::{disc, fill_rect, floats, polygon, L};
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Motion along a path";
const DESCRIPTION: &str = "DrawingAnimations.BindAlongPath (position + optional tangent rotation), PathGeometry.GetPointAt, PathPoint";

pub struct MotionPathScene {
    session: DrawingSession,
    eight: PathGeometry,
    dart_progress: FloatTrack,
    fonts: FontRegistry,
}

fn p(x: f32, y: f32) -> PointF {
    PointF::new(x, y)
}

impl MotionPathScene {
    pub fn new(fonts: &FontRegistry) -> MotionPathScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = session.add_layer("Notes", DIM);
        label(notes, fonts, "orient_to_path: the dart turns with the path; the ball does not", 60.0, 140.0, 14.0, DIM, TextAlign::Left);
        label(notes, fonts, "a group driving a rounded track, eased", 760.0, 140.0, 14.0, DIM, TextAlign::Left);
        // A figure eight from two loops of cubic curves.
        let eight = PathGeometryBuilder::new()
            .move_to(360.0, 410.0)
            .cubic_to(p(460.0, 280.0), p(640.0, 300.0), p(640.0, 410.0))
            .cubic_to(p(640.0, 520.0), p(460.0, 540.0), p(360.0, 410.0))
            .cubic_to(p(260.0, 280.0), p(80.0, 300.0), p(80.0, 410.0))
            .cubic_to(p(80.0, 520.0), p(260.0, 540.0), p(360.0, 410.0))
            .close()
            .build();
        let track = PathGeometry::rounded_rectangle(RectangleF::new(780.0, 200.0, 420.0, 380.0), 120.0);
        session.add_layer("Paths", Color::from_rgba(132, 138, 164, 110));
        let dashed = |w: f32, pattern: Vec<f32>| PathStrokeStyle::new(w, PathStrokeStyleOptions { dash_pattern: Some(pattern), ..PathStrokeStyleOptions::default() }).unwrap();
        session.draw_path(PathShape::with_style(eight.clone(), dashed(2.0, vec![8.0, 8.0]), None, None, None));
        session.draw_path(PathShape::with_style(track.clone(), PathStrokeStyle::new(34.0, PathStrokeStyleOptions::default()).unwrap(), Some(Color::from_rgb(46, 50, 66)), None, None));
        session.draw_path(PathShape::with_style(track.clone(), dashed(2.0, vec![18.0, 14.0]), Some(Color::from_rgba(236, 236, 244, 120)), None, None));
        session.add_layer("Riders", ACCENT);
        let dart_progress = floats(&[(0.0, 0.0, L), (4.0, 1.0, L)]).with_loop_mode(TrackLoopMode::Loop);
        let ball = session.draw_circle(0.0, 0.0, 11.0, None, Some(Color::from_rgb(120, 180, 255)), true).unwrap();
        let mut ball_track = FloatTrack::new();
        ball_track.add(0.0, 0.5, L).unwrap();
        ball_track.add(2.0, 1.0, L).unwrap();
        ball_track.add(2.0, 0.0, L).unwrap();
        ball_track.add(4.0, 0.5, L).unwrap();
        session.animations_mut().bind_along_path(ball, &eight, ball_track.with_loop_mode(TrackLoopMode::Loop), PathTravelOptions::default()).unwrap();
        let dart = session.draw_shape(polygon(&[p(24.0, 0.0), p(-14.0, -13.0), p(-6.0, 0.0), p(-14.0, 13.0)], Some(ACCENT)));
        session.find_element_mut(dart).unwrap().set_effects(Some(DrawingEffects::drop_shadow(PointF::new(6.0, 8.0), 6.0, Color::from_rgba(0, 0, 0, 150)).unwrap()));
        session.animations_mut().bind_along_path(dart, &eight, dart_progress.clone(), PathTravelOptions { orient_to_path: true, ..PathTravelOptions::default() }).unwrap();
        let mut car = DrawingGroup::new(None);
        car.add(fill_rect(-30.0, -15.0, 60.0, 30.0, Some(Color::from_rgb(230, 80, 90)), 8.0));
        car.add(fill_rect(4.0, -11.0, 16.0, 22.0, Some(Color::from_rgb(160, 210, 255)), 4.0));
        car.add(disc(26.0, -9.0, 3.0, Some(Color::from_rgb(255, 240, 160))));
        car.add(disc(26.0, 9.0, 3.0, Some(Color::from_rgb(255, 240, 160))));
        let car = session.draw_shape(car);
        session.animations_mut().bind_along_path(car, &track, floats(&[(0.0, 0.0, L), (8.0, 1.0, EasingKind::SineInOut)]), PathTravelOptions { orient_to_path: true, ..PathTravelOptions::default() }).unwrap();
        session.add_layer("Now", INK);
        MotionPathScene { session, eight, dart_progress, fonts: fonts.clone() }
    }
}

impl DemoScene for MotionPathScene {
    fn name(&self) -> &'static str {
        "motion-path"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        // The tangent at the dart, read from the same path and progress the binding uses.
        let point = self.eight.get_point_at(self.dart_progress.evaluate(seconds).unwrap_or(0.0));
        let length = self.eight.get_length();
        let fonts = self.fonts.clone();
        let now = self.session.get_layer_mut("Now").unwrap();
        now.clear();
        let q = point.position;
        now.add_shape(ArrowShape::new(q.x, q.y, q.x + (point.tangent.x * 70.0), q.y + (point.tangent.y * 70.0), ArrowShapeOptions { stroke_thickness: 2.0, color: Some(Color::from_rgba(236, 236, 244, 170)), head_length: Some(12.0) }).unwrap());
        label(now, &fonts, &format!("distance {:6.1} of {:.1}    heading {:6.1} deg", point.distance, length, point.angle()), 60.0, 640.0, 16.0, INK, TextAlign::Left);
        self.session.render_with_background(scene, width, height);
    }
}
