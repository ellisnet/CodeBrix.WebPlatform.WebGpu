//! "rolling-text" (AnimationDemoApp's RollingTextScene): rolling along paths. Top, the letters of a
//! word roll in along a swooping route as a train (bind_text_along_path): the last letter leads
//! (StaggerOrder::Reverse), each rolls on its own baseline (roll_on_baseline), the gaps close as
//! they arrive (spacing_scale), and the progress track runs past 1 (to 1.025) and eases back
//! (extend_beyond_ends) - an overshoot that settles every letter upright, exactly where the text
//! block draws it (its ghost is shown). Bottom, a wheel rolls over hills (bind_along_path with
//! orient_to_path and roll_radius): it tips with the slope and turns with the distance, the two
//! added; a read-out shows both parts.

use codebrix_webgpu_animation::animation::{EasingKind, FloatTrack};
use codebrix_webgpu_animation::models::{ElementId, PathTrainOptions, PathTravelOptions, StaggerOrder};
use codebrix_webgpu_animation::paths::{PathGeometry, PathGeometryBuilder, PathStrokeStyle, PathStrokeStyleOptions};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{CircleShape, DrawingGroup, PathShape, PolylineShape, PolylineShapeOptions, ShapeOptions, TextBlockShape, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign, TextUnit};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::kit::{disc, floats, line, L};
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, FRAME_HEIGHT, FRAME_WIDTH, INK};

const TITLE: &str = "Rolling along a path: letters and a wheel";
const DESCRIPTION: &str = "BindTextAlongPath (a train of letters: stagger, spacing scale, roll on baseline, overshoot), PathTravelOptions (facing + roll add)";
const WORD: &str = "ROLLING";
const WHEEL_RADIUS: f32 = 30.0;
const UPRIGHT: f32 = 1.0;

pub struct RollingTextScene {
    session: DrawingSession,
    hills: PathGeometry,
    wheel_progress: FloatTrack,
    wheel: ElementId,
    fonts: FontRegistry,
}

fn dashed(width: f32, dash: f32, gap: f32) -> PathStrokeStyle {
    PathStrokeStyle::new(width, PathStrokeStyleOptions { dash_pattern: Some(vec![dash, gap]), ..PathStrokeStyleOptions::default() }).unwrap()
}

impl RollingTextScene {
    pub fn new(fonts: &FontRegistry) -> RollingTextScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        session.add_layer("Stage", DIM);
        // The word, laid out where it should end up; a ghost of it marks the resting places.
        let layout = session.layout_text(WORD, 120.0, None, None).unwrap();
        let (left, top) = ((FRAME_WIDTH as f32 - layout.width()) / 2.0, 130.0);
        session.draw_shape(TextBlockShape::new(layout.clone(), left, top, Some(Color::from_rgba(132, 138, 164, 40))).unwrap());
        let (cx, cy) = (left + layout.width() / 2.0, top + layout.height() / 2.0);
        // The route: from off the left edge, a swoop down and up, ending heading right (so the letters arrive upright).
        let p = PointF::new;
        let route = PathGeometryBuilder::new()
            .move_to(cx - 1150.0, cy + 40.0)
            .cubic_to(p(cx - 900.0, cy + 190.0), p(cx - 700.0, cy - 120.0), p(cx - 460.0, cy + 20.0))
            .cubic_to(p(cx - 280.0, cy + 120.0), p(cx - 160.0, cy), p(cx, cy))
            .build();
        session.draw_shape(PathShape::with_style(route.clone(), dashed(1.5, 7.0, 7.0), Some(Color::from_rgba(132, 138, 164, 110)), None, None));
        {
            let stage = session.get_layer_mut("Stage").unwrap();
            label(stage, fonts, "the route (each letter travels a copy of it ending at its own place)", 40.0, 372.0, 13.0, DIM, TextAlign::Left);
            label(stage, fonts, "BindTextAlongPath: Reverse order, Stagger 0.07 s, Spacing 80 x SpacingScale 1 -> 0, RollOnBaseline, progress 0 -> 1.025 -> 1", 40.0, 394.0, 13.0, DIM, TextAlign::Left);
        }
        let progress = floats(&[(0.3, 0.0, L), (3.6, 1.025, EasingKind::SineOut), (4.6, 1.0, EasingKind::SineInOut)]);
        let block = TextBlockShape::new(layout, left, top, Some(ACCENT)).unwrap();
        session.add_layer("Letters", ACCENT);
        let options = PathTrainOptions {
            order: StaggerOrder::Reverse,
            stagger: 0.07,
            spacing: 80.0,
            spacing_scale: Some(floats(&[(0.0, 1.0, L), (2.6, 1.0, L), (4.8, 0.0, EasingKind::SineInOut)])),
            roll_on_baseline: true,
            extend_beyond_ends: true,
            ..PathTrainOptions::default()
        };
        session.animations_mut().bind_text_along_path(&block, TextUnit::Character, "Letters", &route, progress, options).unwrap();
        // The hills: the path of the wheel's hub; the ground lies one radius below it, square to the path.
        let hills = PathGeometryBuilder::new()
            .move_to(40.0, 560.0)
            .cubic_to(p(190.0, 560.0), p(250.0, 470.0), p(380.0, 480.0))
            .cubic_to(p(500.0, 490.0), p(540.0, 610.0), p(680.0, 610.0))
            .cubic_to(p(820.0, 610.0), p(860.0, 500.0), p(980.0, 505.0))
            .cubic_to(p(1090.0, 510.0), p(1140.0, 580.0), p(1240.0, 580.0))
            .build();
        let mut ground: Vec<PointF> = (0..=300).map(|i| {
            let q = hills.get_point_at(i as f32 / 300.0);
            p(q.position.x - q.tangent.y * WHEEL_RADIUS, q.position.y + q.tangent.x * WHEEL_RADIUS)
        }).collect();
        let edge = ground.clone();
        ground.push(p(ground[ground.len() - 1].x, FRAME_HEIGHT as f32));
        ground.push(p(ground[0].x, FRAME_HEIGHT as f32));
        session.set_active_layer("Stage");
        session.draw_shape(PolylineShape::new(&ground, PolylineShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgb(44, 52, 46)), is_closed: true, is_filled: true }).unwrap());
        session.draw_shape(PolylineShape::new(&edge, PolylineShapeOptions { stroke_thickness: 3.0, color: Some(Color::from_rgb(96, 140, 96)), is_closed: false, is_filled: false }).unwrap());
        session.draw_shape(PathShape::with_style(hills.clone(), dashed(1.0, 4.0, 6.0), Some(Color::from_rgba(132, 138, 164, 100)), None, None));
        // The wheel, drawn round its hub at (0, 0): a rim, spokes and one red spoke to follow.
        let mut wheel = DrawingGroup::new(None);
        wheel.add(CircleShape::new(0.0, 0.0, WHEEL_RADIUS - 2.0, ShapeOptions { stroke_thickness: 4.0, color: Some(INK), is_filled: false }).unwrap());
        for i in 0..6 {
            let a = i as f64 * std::f64::consts::PI / 3.0;
            wheel.add(line(0.0, 0.0, (WHEEL_RADIUS as f64 * a.cos()) as f32, (WHEEL_RADIUS as f64 * a.sin()) as f32, 2.0, Some(DIM)));
        }
        wheel.add(line(0.0, 0.0, 0.0, -(WHEEL_RADIUS - 3.0), 4.0, Some(Color::from_rgb(240, 84, 110))));
        wheel.add(disc(0.0, 0.0, 5.0, Some(ACCENT)));
        let wheel = session.add_layer("Riders", INK).add_element(wheel);
        let wheel_progress = floats(&[(0.4, 0.0, L), (7.4, 1.0, EasingKind::SineInOut)]);
        session.animations_mut().bind_along_path(wheel, &hills, wheel_progress.clone(), PathTravelOptions { orient_to_path: true, roll_radius: WHEEL_RADIUS, upright_at_progress: UPRIGHT, ..PathTravelOptions::default() }).unwrap();
        session.add_layer("Now", INK);
        RollingTextScene { session, hills, wheel_progress, wheel, fonts: fonts.clone() }
    }
}

impl DemoScene for RollingTextScene {
    fn name(&self) -> &'static str {
        "rolling-text"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        // The wheel's rotation is its heading plus its roll (distance from where it is upright / radius).
        let point = self.hills.get_point_at(self.wheel_progress.evaluate(seconds).unwrap_or(0.0));
        let roll = (point.distance - UPRIGHT * self.hills.get_length()) as f64 / WHEEL_RADIUS as f64 * 180.0 / std::f64::consts::PI;
        let rotation = self.session.find_element(self.wheel).map_or(0.0, |w| w.transform().rotation());
        let readout = format!("wheel rotation {rotation:>7.1} deg  =  heading {:>6.1} deg  +  roll {roll:>7.1} deg  (distance / radius)", point.angle());
        let fonts = self.fonts.clone();
        let now = self.session.get_layer_mut("Now").unwrap();
        now.clear();
        now.add_shape(TextShape::new(&readout, 40.0, 700.0, 15.0, &fonts, TextShapeOptions { color: Some(INK), ..TextShapeOptions::default() }).unwrap());
        self.session.render_with_background(scene, width, height);
    }
}
