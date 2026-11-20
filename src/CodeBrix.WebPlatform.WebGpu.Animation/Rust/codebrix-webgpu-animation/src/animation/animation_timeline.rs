//! `AnimationTimeline`: a duration at a frame rate, with named cues and markers (C#
//! `AnimationTimeline`). Its time is whatever the caller passes - in a browser, the canvas
//! host's clock time from the frame (`CanvasFrame::info.time`); the timeline reads no clock.

use codebrix_webgpu::TimePoints;

use crate::animation::easing_kind::EasingKind;
use crate::animation::timeline_cue::TimelineCue;

// How close (in frames) a time may fall short of a frame boundary and still count as on it.
const FRAME_TOLERANCE: f64 = 1e-6;

/// A timeline of `duration` seconds at `frames_per_second`, with named cues (spans) and markers.
#[derive(Clone, Debug)]
pub struct AnimationTimeline {
    duration: f64,
    frames_per_second: f64,
    frame_count: i32,
    cues: Vec<TimelineCue>,
}

impl AnimationTimeline {
    /// A timeline. The error (C# ArgumentOutOfRangeException) says what to do: both values
    /// positive and finite. 30 frames per second is the C# default.
    pub fn new(duration: f64, frames_per_second: f64) -> Result<AnimationTimeline, String> {
        if !duration.is_finite() || duration <= 0.0 {
            return Err("The timeline's duration must be a positive, finite number of seconds.".to_string());
        }
        if !frames_per_second.is_finite() || frames_per_second <= 0.0 {
            return Err("The timeline's frame rate must be a positive, finite number of frames per second (30 is usual).".to_string());
        }
        let frames = ((duration * frames_per_second) - FRAME_TOLERANCE).ceil();
        Ok(AnimationTimeline { duration, frames_per_second, frame_count: frames.clamp(1.0, i32::MAX as f64) as i32, cues: Vec::new() })
    }

    /// The length, seconds.
    pub fn duration(&self) -> f64 {
        self.duration
    }

    /// The frame rate.
    pub fn frames_per_second(&self) -> f64 {
        self.frames_per_second
    }

    /// One frame's length, seconds.
    pub fn frame_duration(&self) -> f64 {
        1.0 / self.frames_per_second
    }

    /// The number of frames (at least 1).
    pub fn frame_count(&self) -> i32 {
        self.frame_count
    }

    /// The cues, in the order they were added.
    pub fn cues(&self) -> &[TimelineCue] {
        &self.cues
    }

    /// The time of a frame, seconds (None for a negative frame, where C# throws).
    pub fn time_of_frame(&self, frame: i32) -> Option<f64> {
        if frame < 0 {
            None
        } else {
            Some(frame as f64 / self.frames_per_second)
        }
    }

    /// The frame shown at a time (clamped to the timeline; NaN is frame 0).
    pub fn frame_at(&self, seconds: f64) -> i32 {
        if seconds.is_nan() {
            return 0;
        }
        let frame = ((seconds * self.frames_per_second) + FRAME_TOLERANCE).floor();
        frame.clamp(0.0, (self.frame_count - 1) as f64) as i32
    }

    /// A time clamped to 0..duration (NaN becomes 0).
    pub fn clamp(&self, seconds: f64) -> f64 {
        if seconds.is_nan() {
            0.0
        } else {
            seconds.clamp(0.0, self.duration)
        }
    }

    /// The time as a timecode "hh:mm:ss:ff" (C# `FormatTimecode`).
    pub fn format_timecode(&self, seconds: f64) -> String {
        let whole = if seconds.is_nan() || seconds < 0.0 { 0.0 } else { ((seconds * self.frames_per_second) + FRAME_TOLERANCE).floor() };
        let frames = whole.min((i64::MAX / 2) as f64) as i64;
        let nominal = (self.frames_per_second.round() as i64).max(1);
        let total_seconds = frames / nominal;
        let frame_in_second = frames % nominal;
        let frame_digits = (nominal - 1).to_string().len().max(2);
        format!("{:02}:{:02}:{:02}:{:0width$}", total_seconds / 3600, (total_seconds / 60) % 60, total_seconds % 60, frame_in_second, width = frame_digits)
    }

    /// Adds a cue eased by a named curve. The error says what to do (a name already used, or the
    /// cue's own errors).
    pub fn add_cue(&mut self, name: &str, start_time: f64, end_time: f64, easing: EasingKind) -> Result<&TimelineCue, String> {
        self.add(TimelineCue::new(name, start_time, end_time, easing)?)
    }

    /// Adds a marker (a cue with no length).
    pub fn add_marker(&mut self, name: &str, seconds: f64) -> Result<&TimelineCue, String> {
        self.add(TimelineCue::new(name, seconds, seconds, EasingKind::Linear)?)
    }

    /// Adds a marker at the time of one of the canvas clock's NAMED POINTS (so a point an
    /// application defines once on the host's clock is the same moment on this timeline). The
    /// error says what to do when the clock has no such point.
    pub fn add_marker_at_point(&mut self, name: &str, points: &TimePoints, point: &str) -> Result<&TimelineCue, String> {
        let seconds = points.time(point).ok_or_else(|| format!("The clock has no named point '{point}': define it with CanvasClock::define_point first."))?;
        self.add_marker(name, seconds)
    }

    /// Adds a cue (C# `AddCue(TimelineCue)`). The error says what to do when the name is used.
    pub fn add(&mut self, cue: TimelineCue) -> Result<&TimelineCue, String> {
        if self.cues.iter().any(|c| c.name() == cue.name()) {
            return Err(format!("A cue named '{}' already exists: give each cue its own name.", cue.name()));
        }
        self.cues.push(cue);
        Ok(self.cues.last().unwrap())
    }

    /// The cue with this name (trimmed), or None.
    pub fn get_cue(&self, name: &str) -> Option<&TimelineCue> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return None;
        }
        self.cues.iter().find(|c| c.name() == trimmed)
    }

    /// The eased progress of a named cue at a time; None for an unknown name (C# throws).
    pub fn progress(&self, cue_name: &str, seconds: f64) -> Option<f64> {
        self.get_cue(cue_name).map(|c| c.progress(seconds))
    }

    /// The cues active at a time (a marker is active in the frame it falls in).
    pub fn get_active_cues(&self, seconds: f64) -> Vec<&TimelineCue> {
        let frame = self.frame_at(seconds);
        self.cues.iter().filter(|c| if c.is_marker() { self.frame_at(c.start_time()) == frame } else { c.is_active_at(seconds) }).collect()
    }
}
