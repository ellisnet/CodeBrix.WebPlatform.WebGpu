//! "keyframes" (AnimationDemoApp's KeyframeScene): balls on eight lanes, each bound to a
//! PointTrack - four keys arriving with linear, CubicInOut, BackOut, BounceOut and ElasticOut
//! easing, hold keys, a Loop track and a PingPong track; a star badge whose rotation, scale and
//! opacity are FloatTracks (about its centre, its anchor); a group of swatches whose content
//! colour is a ColorTrack. Every frame is only `session.set_time(t)`: the bindings do the rest.

use codebrix_webgpu_animation::animation::{ColorTrack, Easing, EasingKind, PointTrack, TrackLoopMode};
use codebrix_webgpu_animation::models::AnimatedProperty;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::DrawingGroup;
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::kit::{disc, fill_rect, floats, line, points, polygon, L};
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, BACKDROP, DIM, INK};

const TITLE: &str = "Keyframe tracks";
const DESCRIPTION: &str = "PointTrack, FloatTrack, ColorTrack with per-segment easing, hold keys, Loop and PingPong, bound through session.Animations";
const LANE_LEFT: f32 = 220.0;
const LANE_RIGHT: f32 = 860.0;
const LANE_TOP: f32 = 130.0;
const LANE_GAP: f32 = 62.0;

fn lane_y(lane: usize) -> f32 {
    LANE_TOP + (lane as f32 * LANE_GAP)
}

/// Four keys along the lane, each segment arriving with the lane's easing.
fn lane(i: usize, easing: EasingKind) -> PointTrack {
    let y = lane_y(i);
    points(&[(0.0, LANE_LEFT, y, L), (2.5, LANE_RIGHT, y, easing), (5.0, (LANE_LEFT + LANE_RIGHT) / 2.0, y, easing), (7.5, LANE_LEFT, y, easing)])
}

pub fn star(cx: f32, cy: f32, outer: f32, inner: f32) -> Vec<PointF> {
    (0..10)
        .map(|i| {
            let angle = (std::f64::consts::PI * i as f64 / 5.0) - (std::f64::consts::PI / 2.0);
            let r = if i % 2 == 0 { outer } else { inner };
            PointF::new(cx + (r * angle.cos() as f32), cy + (r * angle.sin() as f32))
        })
        .collect()
}

pub struct KeyframeScene {
    session: DrawingSession,
}

impl KeyframeScene {
    pub fn new(fonts: &FontRegistry) -> KeyframeScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        session.add_layer("Guides", DIM);
        session.add_layer("Moving", ACCENT);
        let mut lanes: Vec<(&str, PointTrack)> = vec![("linear", lane(0, EasingKind::Linear)), ("CubicInOut", lane(1, EasingKind::CubicInOut)), ("BackOut", lane(2, EasingKind::BackOut)), ("BounceOut", lane(3, EasingKind::BounceOut)), ("ElasticOut", lane(4, EasingKind::ElasticOut))];
        let mut hold = PointTrack::new();
        hold.add(0.0, PointF::new(LANE_LEFT, lane_y(5)), L).unwrap();
        for i in 1..=4 {
            hold.add_hold(i as f64 * 1.6, PointF::new(Easing::lerp_f32(LANE_LEFT, LANE_RIGHT, i as f64 / 4.0), lane_y(5))).unwrap();
        }
        lanes.push(("hold keys", hold));
        lanes.push(("loop", points(&[(0.0, LANE_LEFT, lane_y(6), L), (1.5, LANE_RIGHT, lane_y(6), EasingKind::QuadInOut)]).with_loop_mode(TrackLoopMode::Loop)));
        lanes.push(("ping-pong", points(&[(0.0, LANE_LEFT, lane_y(7), L), (1.5, LANE_RIGHT, lane_y(7), EasingKind::SineInOut)]).with_loop_mode(TrackLoopMode::PingPong)));
        for (i, (name, track)) in lanes.into_iter().enumerate() {
            let y = lane_y(i);
            let guides = session.get_layer_mut("Guides").unwrap();
            guides.add_shape(line(LANE_LEFT, y, LANE_RIGHT, y, 2.0, Some(Color::from_rgba(132, 138, 164, 70))));
            label(guides, fonts, name, LANE_LEFT - 24.0, y + 5.0, 15.0, INK, TextAlign::Right);
            let color = if i < 5 { ACCENT } else { Color::from_rgb(120, 200, 255) };
            let ball = session.get_layer_mut("Moving").unwrap().add_element(disc(0.0, 0.0, 11.0, Some(color)));
            session.animations_mut().bind(ball, AnimatedProperty::Position, track).unwrap();
        }
        // A badge: rotation, scale and opacity from float tracks (about the badge's centre).
        let mut badge = DrawingGroup::new(Some(Color::from_rgb(255, 210, 120)));
        badge.add(polygon(&star(1040.0, 300.0, 70.0, 32.0), None));
        badge.add(disc(1040.0, 300.0, 18.0, Some(BACKDROP)));
        let badge = session.get_layer_mut("Moving").unwrap().add_element(badge);
        let mut a = session.animations_mut();
        a.bind(badge, AnimatedProperty::Anchor, points(&[(0.0, 1040.0, 300.0, L)])).unwrap();
        a.bind(badge, AnimatedProperty::Rotation, floats(&[(0.0, 0.0, L), (4.0, 360.0, EasingKind::CubicInOut), (8.0, 0.0, EasingKind::CubicInOut)])).unwrap();
        a.bind(badge, AnimatedProperty::Scale, floats(&[(0.0, 0.4, L), (2.0, 1.2, EasingKind::ElasticOut), (6.0, 1.2, L), (8.0, 0.4, EasingKind::BackIn)])).unwrap();
        a.bind(badge, AnimatedProperty::Opacity, floats(&[(0.0, 0.0, L), (0.8, 1.0, L), (7.2, 1.0, L), (8.0, 0.0, L)])).unwrap();
        // A group whose children take the colour track's colour.
        let mut swatches = DrawingGroup::new(None);
        for i in 0..4 {
            swatches.add(fill_rect(960.0 + (i as f32 * 42.0), 520.0, 34.0, 80.0, None, 5.0));
        }
        let swatches = session.get_layer_mut("Moving").unwrap().add_element(swatches);
        let colors = ColorTrack::from_keys(&[(0.0, Color::from_rgb(255, 90, 120), L), (2.5, Color::from_rgb(255, 200, 80), EasingKind::SineInOut), (5.0, Color::from_rgb(90, 200, 255), EasingKind::SineInOut), (8.0, Color::from_rgb(255, 90, 120), EasingKind::SineInOut)]).unwrap();
        session.animations_mut().bind(swatches, AnimatedProperty::Color, colors).unwrap();
        let guides = session.get_layer_mut("Guides").unwrap();
        label(guides, fonts, "rotation, scale, opacity tracks", 1040.0, 420.0, 13.0, DIM, TextAlign::Center);
        label(guides, fonts, "color track -> group content color", 1040.0, 630.0, 13.0, DIM, TextAlign::Center);
        label(guides, fonts, "Every frame is only session.set_time(t): the bindings set positions, transforms, opacity and color.", 32.0, 690.0, 15.0, INK, TextAlign::Left);
        KeyframeScene { session }
    }
}

impl DemoScene for KeyframeScene {
    fn name(&self) -> &'static str {
        "keyframes"
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
