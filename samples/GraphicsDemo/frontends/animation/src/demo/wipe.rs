//! "wipe" (a feature of the second GO, Not in C#): a soft edge between two renders
//! (`SessionWipe`) - ONE session wiped from its line drawing to its authored colours (a drawing
//! coloured in as the edge passes), two different sessions (day to night), and the night drawn
//! MULTIPLIED over the day through `SessionWipe::render_sessions` (the second session's blend moves
//! onto the wipe's group). A straight edge at any angle or a circle growing from a point (iris);
//! softness and progress are controls, and "animate" runs the progress from the scene's clock
//! (eased; pause freezes it, a seek shows the same frame). "Show" can draw either render alone, so
//! a check can compare the wipe at progress 0 and 1 with them.

use codebrix_webgpu_animation::animation::{Easing, EasingKind};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{DrawingSession, PointF, RectangleF, SessionBlendMode, SessionPaint, SessionRenderOptions, SessionWipe, WipeEdge};

use crate::demo::kit::part;
use crate::demo::paint_override::{ink, rich_session};
use crate::demo::{create_frame_session, label, Control, DemoScene, Readout, Settings, DIM};

const TITLE: &str = "Wipes between two renders";
const DESCRIPTION: &str = "SessionWipe: a straight edge at any angle or an iris, with a soft band - between one session's two looks, two sessions, or a multiplied session";
/// Where the renders are drawn (frame units).
pub const AREA: RectangleF = RectangleF { x: 160.0, y: 112.0, width: 960.0, height: 540.0 };

const CONTROLS: &[Control] = &[
    Control::Choice { key: "pair", label: "Renders", options: &[("line -> as authored (one session)", 0.0), ("day -> night (two sessions)", 1.0), ("night MULTIPLIED over day", 2.0)], default: 0.0 },
    Control::Choice { key: "edge", label: "Edge", options: &[("straight", 0.0), ("iris (circle)", 1.0)], default: 0.0 },
    Control::Slider { key: "angle", label: "Angle (degrees)", min: 0.0, max: 360.0, step: 5.0, default: 30.0 },
    Control::Slider { key: "iris-x", label: "Iris centre x", min: 0.0, max: 1.0, step: 0.05, default: 0.5 },
    Control::Slider { key: "iris-y", label: "Iris centre y", min: 0.0, max: 1.0, step: 0.05, default: 0.5 },
    Control::Slider { key: "softness", label: "Softness (canvas px)", min: 0.0, max: 200.0, step: 5.0, default: 60.0 },
    Control::Toggle { key: "animate", label: "Animate the progress", default: true },
    Control::Slider { key: "progress", label: "Progress (when not animated)", min: 0.0, max: 1.0, step: 0.01, default: 0.5 },
    Control::Choice { key: "show", label: "Show", options: &[("the wipe", 0.0), ("the first render alone", 1.0), ("the second render alone", 2.0)], default: 0.0 },
];

const LEGEND: &[&str] = &[
    "At progress 0 and 1 the picture is exactly one render; in between a straight (or circular) edge with a soft band of the chosen width moves at the chosen angle.",
    "Over opaque content the band shows no dark or light seam. Softness 0 gives a crisp edge.",
    "Pausing the clock freezes the wipe; a seek shows the same frame every time. The iris covers every corner exactly at progress 1.",
    "\"Multiplied\": the night darkens the day only where it has arrived. The wipe's group carries the blend, so the night multiplies with what is UNDER the wipe - the day is drawn once underneath; the edge spans the whole canvas there (render_sessions).",
];

pub struct WipeScene {
    frame: DrawingSession,
    day: DrawingSession,
    night: DrawingSession,
    settings: Settings,
    progress: f32,
}

impl WipeScene {
    pub fn new(fonts: &FontRegistry) -> WipeScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let labels = frame.add_layer("Labels", DIM);
        label(labels, fonts, "progress 0 = the first render, 1 = the second", AREA.x, AREA.y + AREA.height + 26.0, 14.0, DIM, TextAlign::Left);
        WipeScene { frame, day: rich_session(fonts, false), night: rich_session(fonts, true), settings: Settings::from_controls(CONTROLS), progress: 0.0 }
    }
}

impl DemoScene for WipeScene {
    fn name(&self) -> &'static str {
        "wipe"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        15.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let s = &self.settings;
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.day.set_time(seconds);
        self.night.set_time(seconds);
        let target = part(&self.frame, AREA.x, AREA.y, AREA.width, AREA.height, width, height);
        self.progress = if s.on("animate") { Easing::evaluate(EasingKind::CubicInOut, ((seconds % 5.0) / 4.0).min(1.0)) as f32 } else { s.get("progress") };
        let edge = if s.get("edge") as usize == 1 {
            WipeEdge::Circle { center: PointF::new(target.x + s.get("iris-x") * target.width, target.y + s.get("iris-y") * target.height) }
        } else {
            WipeEdge::Line { angle_degrees: s.get("angle") }
        };
        let wipe = SessionWipe::new(self.progress, edge, s.get("softness"));
        let at = |options: SessionRenderOptions| SessionRenderOptions { target: Some(target), ..options };
        let pair = s.get("pair") as usize;
        let (a, a_options, b, b_options) = match pair {
            0 => (&self.day, at(SessionRenderOptions { paint: SessionPaint::Line(ink(1.5, true, true)), ..Default::default() }), &self.day, at(SessionRenderOptions::default())),
            1 => (&self.day, at(SessionRenderOptions::default()), &self.night, at(SessionRenderOptions::default())),
            _ => (&self.day, at(SessionRenderOptions::default()), &self.night, at(SessionRenderOptions { blend: SessionBlendMode::Multiply, ..Default::default() })),
        };
        match s.get("show") as usize {
            1 => a.render_with(scene, width, height, &a_options),
            2 => {
                if pair == 2 {
                    a.render_with(scene, width, height, &a_options);
                }
                b.render_with(scene, width, height, &b_options)
            }
            // The multiplied night blends with what is UNDER the wipe (the wipe's group carries the
            // blend): the day is drawn first underneath, so the night darkens the day where the
            // edge has passed.
            _ if pair == 2 => {
                a.render_with(scene, width, height, &a_options);
                wipe.render_sessions(scene, width, height, (a, &a_options), (b, &b_options))
            }
            _ => wipe.render(scene, target, |scene| a.render_with(scene, width, height, &a_options), |scene| b.render_with(scene, width, height, &b_options)),
        }
    }

    fn controls(&self) -> &'static [Control] {
        CONTROLS
    }

    fn apply_settings(&mut self, settings: &Settings) {
        self.settings = settings.clone();
    }

    fn legend(&self) -> &'static [&'static str] {
        LEGEND
    }

    fn readouts(&self) -> Vec<Readout> {
        vec![Readout::new("progress-now", "Progress now", format!("{:.3}", self.progress)), Readout::new("area", "Area (frame units)", format!("{},{},{},{}", AREA.x, AREA.y, AREA.width, AREA.height))]
    }
}
