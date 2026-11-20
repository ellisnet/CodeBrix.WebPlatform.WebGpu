//! "time-keys" (AnimationDemoApp's TimeKeyScene): 480 threads added ONCE, each with a StartTime
//! (every seventh flashes and has an EndTime too); setting the session's time decides which exist,
//! so seeking back needs no rebuilding. The pen drawing them and the read-out are AnimatedShapes
//! (rebuilt for each moment by a function of the time).

use codebrix_webgpu_animation::animation::ColorRamp;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{AnimatedShape, DrawingGroup, DrawingShape, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::FontRegistry;
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::kit::{disc, line};
use crate::demo::{create_frame_session, DemoScene, ACCENT, INK};

const TITLE: &str = "Time-keyed elements";
const DESCRIPTION: &str = "DrawingElement.StartTime/EndTime: add everything once, set session.Time; seeking back needs no rebuild; appends stay incremental live";
const THREAD_COUNT: usize = 480;
const DRAW_SECONDS: f64 = 7.2;
const CENTER: PointF = PointF::new(640.0, 410.0);

/// The two ends of the thread drawn at a time: two points circling the centre at different speeds.
fn ends(seconds: f64) -> (PointF, PointF) {
    let a = seconds / DRAW_SECONDS * 2.0 * std::f64::consts::PI;
    let outer = PointF::new(CENTER.x + (270.0 * (a * 3.0).cos() as f32), CENTER.y + (270.0 * (a * 3.0).sin() as f32));
    let inner = PointF::new(CENTER.x + (110.0 * (-a * 7.0).cos() as f32), CENTER.y + (110.0 * (-a * 7.0).sin() as f32));
    (outer, inner)
}

pub struct TimeKeyScene {
    session: DrawingSession,
}

impl TimeKeyScene {
    pub fn new(fonts: &FontRegistry) -> TimeKeyScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let threads = session.add_layer("Threads", INK);
        let ramp = ColorRamp::from_colors(&[Color::from_rgb(246, 112, 168), Color::from_rgb(232, 146, 92), Color::from_rgb(156, 122, 248)]).unwrap();
        let mut windows = Vec::with_capacity(THREAD_COUNT);
        for k in 0..THREAD_COUNT {
            let appear = k as f64 * DRAW_SECONDS / THREAD_COUNT as f64;
            let (a, b) = ends(appear);
            let flash = k % 7 == 3;
            let end = if flash { Some(appear + 1.2) } else { None };
            let color = ramp.at_with_alpha(k as f64 / THREAD_COUNT as f64, if flash { 255 } else { 90 });
            threads.add_shape(DrawingShape::from(line(a.x, a.y, b.x, b.y, if flash { 3.0 } else { 1.2 }, Some(color))).with_time_window(Some(appear), end));
            windows.push((appear, end));
        }
        let now = session.add_layer("Now", ACCENT);
        now.add_shape(AnimatedShape::new(
            |t| {
                if t > DRAW_SECONDS {
                    return None;
                }
                let (a, b) = ends(t);
                let mut pen = DrawingGroup::new(Some(ACCENT));
                pen.add(line(a.x, a.y, b.x, b.y, 3.0, None));
                pen.add(disc(a.x, a.y, 7.0, None));
                pen.add(disc(b.x, b.y, 7.0, None));
                Some(pen.into())
            },
            0.0,
        ));
        let fonts = fonts.clone();
        now.add_shape(AnimatedShape::new(
            move |t| {
                let visible = windows.iter().filter(|(s, e)| t >= *s && e.map_or(true, |e| t < e)).count();
                let text = format!("{THREAD_COUNT} threads on the layer, {visible} existing at {t:.2} s");
                TextShape::new(&text, 32.0, 690.0, 16.0, &fonts, TextShapeOptions { color: Some(INK), ..TextShapeOptions::default() }).ok().map(DrawingShape::from)
            },
            0.0,
        ));
        TimeKeyScene { session }
    }
}

impl DemoScene for TimeKeyScene {
    fn name(&self) -> &'static str {
        "time-keys"
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
