//! `TimelineCue`: a named span (or, with no length, a marker) on an animation timeline (C#
//! `TimelineCue`).

use std::rc::Rc;

use crate::animation::easing::Easing;
use crate::animation::easing_kind::EasingKind;

/// A named span of time from `start_time` to `end_time` (a MARKER when the two are equal), with
/// the easing its [`progress`](TimelineCue::progress) follows. Times are seconds.
#[derive(Clone)]
pub struct TimelineCue {
    name: String,
    start_time: f64,
    end_time: f64,
    easing: EasingKind,
    curve: Option<Rc<dyn Fn(f64) -> f64>>,
}

impl TimelineCue {
    /// A cue eased by a named curve. The error (C# ArgumentException /
    /// ArgumentOutOfRangeException) says what to do.
    pub fn new(name: &str, start_time: f64, end_time: f64, easing: EasingKind) -> Result<TimelineCue, String> {
        if name.trim().is_empty() {
            return Err("A cue name is required: give the cue a name that is not empty.".to_string());
        }
        if !start_time.is_finite() {
            return Err("The cue's start time must be a finite number of seconds.".to_string());
        }
        if !end_time.is_finite() || end_time < start_time {
            return Err("The cue's end time must be finite and not before its start time.".to_string());
        }
        Ok(TimelineCue { name: name.trim().to_string(), start_time, end_time, easing, curve: None })
    }

    /// A cue eased by a custom curve (C# `TimelineCue(name, start, end, Func<double, double>)`).
    pub fn with_curve(name: &str, start_time: f64, end_time: f64, curve: impl Fn(f64) -> f64 + 'static) -> Result<TimelineCue, String> {
        let mut cue = TimelineCue::new(name, start_time, end_time, EasingKind::Linear)?;
        cue.curve = Some(Rc::new(curve));
        Ok(cue)
    }

    /// The cue's name (trimmed).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where the cue starts, seconds.
    pub fn start_time(&self) -> f64 {
        self.start_time
    }

    /// Where the cue ends, seconds.
    pub fn end_time(&self) -> f64 {
        self.end_time
    }

    /// The cue's length, seconds.
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }

    /// True for a marker (no length).
    pub fn is_marker(&self) -> bool {
        self.end_time == self.start_time
    }

    /// The named easing (Linear when the cue has a custom curve).
    pub fn easing(&self) -> EasingKind {
        self.easing
    }

    /// True when the cue has a custom curve.
    pub fn has_custom_curve(&self) -> bool {
        self.curve.is_some()
    }

    /// True from the start time (inclusive) to the end time (exclusive).
    pub fn is_active_at(&self, seconds: f64) -> bool {
        seconds >= self.start_time && seconds < self.end_time
    }

    /// Progress without easing: 0 before the cue, 1 after it (a marker is 1 from its time on).
    pub fn linear_progress(&self, seconds: f64) -> f64 {
        if seconds.is_nan() || seconds <= self.start_time {
            return if self.is_marker() && seconds >= self.start_time { 1.0 } else { 0.0 };
        }
        if seconds >= self.end_time {
            return 1.0;
        }
        (seconds - self.start_time) / (self.end_time - self.start_time)
    }

    /// Eased progress at a time.
    pub fn progress(&self, seconds: f64) -> f64 {
        let linear = self.linear_progress(seconds);
        match &self.curve {
            Some(curve) => curve(linear),
            None => Easing::evaluate(self.easing, linear),
        }
    }

    /// Seconds since the cue's start (negative before it).
    pub fn local_time(&self, seconds: f64) -> f64 {
        seconds - self.start_time
    }
}

impl std::fmt::Debug for TimelineCue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::fmt::Display for TimelineCue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_marker() {
            write!(f, "{} @ {}", self.name, self.start_time)
        } else {
            write!(f, "{} [{}, {})", self.name, self.start_time, self.end_time)
        }
    }
}
