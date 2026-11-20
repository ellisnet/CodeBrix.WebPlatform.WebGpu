//! "playback" (AnimationDemoApp's PlaybackScene): a PlaybackController driven by a SCRIPT of calls
//! (play, pause, speed 2, seek back, backwards, step, loop, a stall capped at 0.1 s), replayed
//! from the start at 30 ticks a second up to the scene's time - the controller is a pure state
//! machine - and a small clip it plays: a box (a group with a transform) that rises and spins on
//! the clip's cues, and a flash in the frame of its marker, drawn with a DebugOverlay (timecode,
//! frame, cues), into a part of the frame. The controller's state is read out below.

use std::rc::Rc;

use codebrix_webgpu_animation::animation::{AnimationTimeline, Easing, EasingKind, PlaybackController};
use codebrix_webgpu_animation::models::{DrawingTransform, DrawingTransformOptions};
use codebrix_webgpu_animation::rendering::{DebugOverlay, DebugOverlayCorner, VectorScene};
use codebrix_webgpu_animation::shapes::{AnimatedShape, DrawingGroup, DrawingShape};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, SessionRenderOptions};

use crate::demo::kit::{disc, fill_rect, outline_rect, part, ring};
use crate::demo::{create_frame_session, create_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Playback controller and debug overlay";
const DESCRIPTION: &str = "PlaybackController (play, pause, seek, step, speed, loop, capped advance) and DebugOverlay (timecode, frame, cues) in an export";
const CLIP_WIDTH: f32 = 640.0;
const CLIP_HEIGHT: f32 = 360.0;
const STEP: f64 = 1.0 / 30.0;
type Act = fn(&mut PlaybackController);
const SCRIPT: [(f64, &str, Option<Act>); 8] = [
    (0.0, "Play()", Some(|p| p.play())),
    (1.4, "Pause()", Some(|p| p.pause())),
    (2.0, "Speed = 2; Play()", Some(|p| {
        p.set_speed(2.0);
        p.play();
    })),
    (3.0, "SeekBy(-2)", Some(|p| p.seek_by(-2.0))),
    (3.8, "Speed = -1 (backwards)", Some(|p| {
        p.set_speed(-1.0);
    })),
    (4.8, "Step(3) - pauses", Some(|p| p.step(3))),
    (5.4, "IsLooping = true; Speed = 1.5; Play()", Some(|p| {
        p.set_is_looping(true);
        p.set_speed(1.5);
        p.play();
    })),
    (6.6, "a 0.5 s stall -> Advance(0.5): capped at 0.1 s", None),
];

pub struct PlaybackScene {
    timeline: AnimationTimeline,
    frame: DrawingSession,
    clip: DrawingSession,
    fonts: FontRegistry,
}

impl PlaybackScene {
    pub fn new(fonts: &FontRegistry) -> PlaybackScene {
        let mut timeline = AnimationTimeline::new(5.0, 30.0).unwrap();
        timeline.add_cue("rise", 0.0, 2.0, EasingKind::CubicOut).unwrap();
        timeline.add_cue("spin", 1.5, 4.0, EasingKind::SineInOut).unwrap();
        timeline.add_marker("flash", 2.5).unwrap();
        let shared = Rc::new(timeline.clone());
        let mut clip = create_session(CLIP_WIDTH as u32, CLIP_HEIGHT as u32, Color::from_rgb(24, 34, 60), fonts);
        clip.add_layer("Shapes", Color::from_rgb(255, 176, 72));
        let t1 = shared.clone();
        clip.draw_shape(AnimatedShape::new(
            move |t| {
                let y = Easing::lerp_f32(300.0, 180.0, t1.progress("rise", t).unwrap_or(0.0));
                let mut b = DrawingGroup::new(Some(Color::from_rgb(255, 176, 72)));
                b.add(fill_rect(-50.0, -50.0, 100.0, 100.0, None, 12.0));
                b.add(disc(0.0, -28.0, 10.0, Some(Color::from_rgb(24, 34, 60))));
                let transform = DrawingTransform::new(PointF::new(320.0, y), DrawingTransformOptions { rotation: (t1.progress("spin", t).unwrap_or(0.0) * 360.0) as f32, ..DrawingTransformOptions::default() }).unwrap();
                Some(DrawingShape::from(b).with_transform(transform))
            },
            0.0,
        ));
        let t2 = shared.clone();
        clip.draw_shape(AnimatedShape::new(move |t| if t2.frame_at(t) == t2.frame_at(2.5) { Some(ring(320.0, 180.0, 150.0, 6.0, Some(Color::from_rgb(255, 240, 200))).into()) } else { None }, 0.0));
        let mut overlay = DebugOverlay::new(Some(fonts.clone()), Some(timeline.clone()));
        overlay.show_cues = true;
        overlay.corner = DebugOverlayCorner::BottomLeft;
        clip.set_debug_overlay(Some(overlay));
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let script = frame.add_layer("Script", DIM);
        label(script, fonts, "the clip, drawn with a DebugOverlay", 48.0, 130.0, 13.0, DIM, TextAlign::Left);
        label(script, fonts, "scripted input (scene time: call)", 730.0, 130.0, 13.0, DIM, TextAlign::Left);
        for (i, (at, text, _)) in SCRIPT.iter().enumerate() {
            label(script, fonts, &format!("{at:.1} s  {text}"), 730.0, 160.0 + (i as f32 * 28.0), 14.0, INK, TextAlign::Left);
        }
        frame.add_layer("State", ACCENT);
        PlaybackScene { timeline, frame, clip, fonts: fonts.clone() }
    }
}

impl DemoScene for PlaybackScene {
    fn name(&self) -> &'static str {
        "playback"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        // Replay the script up to now, one 30 fps tick at a time: the controller is a pure state machine.
        let mut playback = PlaybackController::new(self.timeline.clone());
        let ticks = ((seconds / STEP) + 1e-9).floor() as i64;
        let mut next = 0;
        let mut current: Option<usize> = None;
        for k in 0..=ticks {
            let now = k as f64 * STEP;
            let mut elapsed = STEP;
            while next < SCRIPT.len() && SCRIPT[next].0 <= now + 1e-9 {
                match SCRIPT[next].2 {
                    None => elapsed = 0.5,
                    Some(act) => act(&mut playback),
                }
                current = Some(next);
                next += 1;
            }
            if k > 0 {
                playback.advance(elapsed);
            }
        }
        let fonts = self.fonts.clone();
        let state = self.frame.get_layer_mut("State").unwrap();
        state.clear();
        if let Some(c) = current {
            state.add_shape(outline_rect(722.0, 140.0 + (c as f32 * 28.0), 530.0, 26.0, 2.0, Some(ACCENT), 4.0));
        }
        let lines = [
            format!("Time {:.3} s   Frame {}", playback.time(), playback.frame()),
            format!("IsPlaying {}   Speed {}   IsLooping {}   IsAtEnd {}", playback.is_playing(), playback.speed(), playback.is_looping(), playback.is_at_end()),
        ];
        for (i, text) in lines.iter().enumerate() {
            label(state, &fonts, text, 48.0, 560.0 + (i as f32 * 30.0), 17.0, INK, TextAlign::Left);
        }
        label(state, &fonts, "Live: Space, Left/Right, Home go through the page's controller on the host clock; D toggles the overlay.", 48.0, 690.0, 14.0, DIM, TextAlign::Left);
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.clip.set_time(playback.time());
        let target = part(&self.frame, 48.0, 145.0, CLIP_WIDTH, CLIP_HEIGHT, width, height);
        self.clip.render_with(scene, width, height, &SessionRenderOptions { include_background: true, target: Some(target) });
    }
}
