//! "timeline" (AnimationDemoApp's TimelineScene): an AnimationTimeline with four eased cues and
//! four markers drawn as a ruler; a playhead; for each cue its linear progress (upper bar) and
//! eased progress (lower bar, with the number); a puck that the cues drive - it travels, spins
//! (overshooting a little) and fades in and out - a DrawingGroup with a transform and an opacity;
//! markers ringed in the frame they are active; the timecode, frame and active cues.

use codebrix_webgpu_animation::animation::{AnimationTimeline, Easing, EasingKind, TimelineCue};
use codebrix_webgpu_animation::models::{DrawingTransform, DrawingTransformOptions};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{DrawingGroup, DrawingShape};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::kit::{disc, fill_rect, line, polygon, ring};
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, BACKDROP, DIM, INK};

const TITLE: &str = "Timeline and cues";
const DESCRIPTION: &str = "AnimationTimeline: frames and timecode; TimelineCue: named stretches with eased progress; markers active in their frame";
const RULER_LEFT: f32 = 120.0;
const RULER_RIGHT: f32 = 1220.0;
const RULER_TOP: f32 = 130.0;
const ROW_HEIGHT: f32 = 30.0;
const BARS_TOP: f32 = 330.0;
const CUE_COLORS: [Color; 4] = [Color::from_rgb(120, 170, 255), Color::from_rgb(255, 176, 72), Color::from_rgb(140, 230, 160), Color::from_rgb(240, 120, 190)];

fn x(seconds: f64) -> f32 {
    Easing::lerp_f32(RULER_LEFT, RULER_RIGHT, seconds / 8.0)
}

pub struct TimelineScene {
    timeline: AnimationTimeline,
    cues: Vec<TimelineCue>,
    session: DrawingSession,
    fonts: FontRegistry,
}

impl TimelineScene {
    pub fn new(fonts: &FontRegistry) -> TimelineScene {
        let mut timeline = AnimationTimeline::new(8.0, 30.0).unwrap();
        let mut cues = Vec::new();
        for (name, start, end, easing) in [("intro", 0.0, 1.5, EasingKind::SmoothStep), ("travel", 1.0, 6.0, EasingKind::CubicInOut), ("spin", 2.0, 5.0, EasingKind::BackOut), ("outro", 6.5, 8.0, EasingKind::SineIn)] {
            cues.push(timeline.add_cue(name, start, end, easing).unwrap().clone());
        }
        let mut markers = Vec::new();
        for i in 1..=4 {
            markers.push(timeline.add_marker(&format!("beat {i}"), 1.5 + i as f64).unwrap().clone());
        }
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let sheet = session.add_layer("Sheet", DIM);
        for second in 0..=8 {
            let sx = x(second as f64);
            sheet.add_shape(line(sx, RULER_TOP - 8.0, sx, RULER_TOP + (cues.len() as f32 * ROW_HEIGHT) + 30.0, 1.0, Some(Color::from_rgba(132, 138, 164, 60))));
            label(sheet, fonts, &format!("{second} s"), sx, RULER_TOP - 14.0, 11.0, DIM, TextAlign::Center);
        }
        for (i, cue) in cues.iter().enumerate() {
            let y = RULER_TOP + (i as f32 * ROW_HEIGHT);
            sheet.add_shape(fill_rect(x(cue.start_time()), y, x(cue.end_time()) - x(cue.start_time()), ROW_HEIGHT - 8.0, Some(CUE_COLORS[i]), 4.0));
            label(sheet, fonts, &format!("{}  ({})", cue.name(), cue.easing().name()), x(cue.start_time()) + 6.0, y + 16.0, 12.0, BACKDROP, TextAlign::Left);
            label(sheet, fonts, cue.name(), 32.0, BARS_TOP + (i as f32 * 52.0) + 4.0, 15.0, INK, TextAlign::Left);
            sheet.add_shape(fill_rect(RULER_LEFT, BARS_TOP + (i as f32 * 52.0) - 10.0, RULER_RIGHT - RULER_LEFT, 12.0, Some(Color::from_rgba(132, 138, 164, 50)), 0.0));
            sheet.add_shape(fill_rect(RULER_LEFT, BARS_TOP + (i as f32 * 52.0) + 6.0, RULER_RIGHT - RULER_LEFT, 12.0, Some(Color::from_rgba(132, 138, 164, 50)), 0.0));
        }
        let marker_y = RULER_TOP + (cues.len() as f32 * ROW_HEIGHT) + 14.0;
        for marker in &markers {
            let mx = x(marker.start_time());
            sheet.add_shape(polygon(&[PointF::new(mx - 6.0, marker_y - 8.0), PointF::new(mx + 6.0, marker_y - 8.0), PointF::new(mx, marker_y)], Some(DIM)));
        }
        label(sheet, fonts, "linear progress (upper bar) and eased progress (lower bar) of each cue at the playhead", RULER_LEFT, BARS_TOP - 26.0, 12.0, DIM, TextAlign::Left);
        session.add_layer("Now", ACCENT);
        TimelineScene { timeline, cues, session, fonts: fonts.clone() }
    }

    fn active_names(&self, seconds: f64) -> String {
        let names: Vec<&str> = self.timeline.get_active_cues(seconds).iter().map(|c| c.name()).collect();
        if names.is_empty() {
            "none".to_string()
        } else {
            names.join(", ")
        }
    }
}

impl DemoScene for TimelineScene {
    fn name(&self) -> &'static str {
        "timeline"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let fonts = self.fonts.clone();
        let travel = self.timeline.progress("travel", seconds).unwrap_or(0.0);
        let spin = self.timeline.progress("spin", seconds).unwrap_or(0.0);
        let fade = self.timeline.progress("intro", seconds).unwrap_or(0.0) * (1.0 - self.timeline.progress("outro", seconds).unwrap_or(0.0));
        let readout = format!("{}   frame {} / {}   active: {}", self.timeline.format_timecode(seconds), self.timeline.frame_at(seconds), self.timeline.frame_count(), self.active_names(seconds));
        let markers: Vec<f64> = self.timeline.get_active_cues(seconds).iter().filter(|c| c.is_marker()).map(|c| c.start_time()).collect();
        let now = self.session.get_layer_mut("Now").unwrap();
        now.clear();
        let playhead = x(seconds);
        now.add_shape(line(playhead, RULER_TOP - 10.0, playhead, RULER_TOP + (self.cues.len() as f32 * ROW_HEIGHT) + 20.0, 2.0, Some(ACCENT)));
        for (i, cue) in self.cues.iter().enumerate() {
            let w = RULER_RIGHT - RULER_LEFT;
            let y = BARS_TOP + (i as f32 * 52.0);
            let linear = cue.linear_progress(seconds) as f32;
            let eased = cue.progress(seconds) as f32;
            if linear > 0.0 {
                now.add_shape(fill_rect(RULER_LEFT, y - 10.0, w * linear, 12.0, Some(CUE_COLORS[i]), 0.0));
            }
            // An overshooting curve (BackOut) passes 1; the bar stops at the end and the number shows the overshoot.
            if eased > 0.0 {
                now.add_shape(fill_rect(RULER_LEFT, y + 6.0, (w * eased.min(1.0)).max(1.0), 12.0, Some(CUE_COLORS[i]), 0.0));
            }
            label(now, &fonts, &format!("{eased:.3}"), 112.0, y + 4.0, 12.0, INK, TextAlign::Right);
        }
        // What the cues drive: a puck (a group) that travels, spins and fades in and out.
        let mut puck = DrawingGroup::new(Some(Color::from_rgb(255, 220, 140)));
        puck.add(fill_rect(-26.0, -26.0, 52.0, 52.0, None, 8.0));
        puck.add(disc(0.0, -14.0, 6.0, Some(BACKDROP)));
        let transform = DrawingTransform::new(PointF::new(Easing::lerp_f32(RULER_LEFT + 40.0, RULER_RIGHT - 40.0, travel), 600.0), DrawingTransformOptions { rotation: (spin * 360.0) as f32, ..DrawingTransformOptions::default() }).unwrap();
        now.add_shape(DrawingShape::from(puck).with_transform(transform).with_opacity(fade.clamp(0.0, 1.0) as f32));
        let marker_y = RULER_TOP + (self.cues.len() as f32 * ROW_HEIGHT) + 14.0;
        for m in markers {
            now.add_shape(ring(x(m), marker_y - 4.0, 12.0, 3.0, Some(ACCENT)));
        }
        label(now, &fonts, &readout, 32.0, 690.0, 16.0, INK, TextAlign::Left);
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
