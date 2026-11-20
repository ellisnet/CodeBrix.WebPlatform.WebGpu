//! `PlaybackController`: play, pause, seek, step and loop over a timeline (C#
//! `PlaybackController`). THE RULE OF THIS CRATE: it reads no clock. It moves only when it is
//! told how much time passed - in a browser, the canvas host's frame delta
//! (`CanvasFrame::info.delta`, which is 0 while the host's clock is paused).

use crate::animation::animation_timeline::AnimationTimeline;

/// Playback over an [`AnimationTimeline`]: a time, playing or paused, looping or not, a speed
/// (negative plays backwards) and the largest step one [`advance`](PlaybackController::advance)
/// may take (so a slow frame slows the animation rather than skipping it).
#[derive(Clone, Debug)]
pub struct PlaybackController {
    timeline: AnimationTimeline,
    time: f64,
    is_playing: bool,
    is_looping: bool,
    speed: f64,
    max_step: f64,
}

impl PlaybackController {
    /// The default largest step, seconds.
    pub const DEFAULT_MAX_STEP: f64 = 0.1;

    /// A controller over a new timeline (C# `PlaybackController(duration, framesPerSecond)`).
    pub fn with_duration(duration: f64, frames_per_second: f64) -> Result<PlaybackController, String> {
        Ok(PlaybackController::new(AnimationTimeline::new(duration, frames_per_second)?))
    }

    /// A controller over a timeline, paused at 0.
    pub fn new(timeline: AnimationTimeline) -> PlaybackController {
        PlaybackController { timeline, time: 0.0, is_playing: false, is_looping: false, speed: 1.0, max_step: PlaybackController::DEFAULT_MAX_STEP }
    }

    /// The timeline.
    pub fn timeline(&self) -> &AnimationTimeline {
        &self.timeline
    }

    /// The timeline's length, seconds.
    pub fn duration(&self) -> f64 {
        self.timeline.duration()
    }

    /// The current time, seconds.
    pub fn time(&self) -> f64 {
        self.time
    }

    /// The frame at the current time.
    pub fn frame(&self) -> i32 {
        self.timeline.frame_at(self.time)
    }

    /// True while playing.
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// True when playback wraps at the ends.
    pub fn is_looping(&self) -> bool {
        self.is_looping
    }

    /// Sets looping.
    pub fn set_is_looping(&mut self, value: bool) {
        self.is_looping = value;
    }

    /// The speed (1 is normal, negative plays backwards).
    pub fn speed(&self) -> f64 {
        self.speed
    }

    /// Sets the speed; false (and no change) for a value that is not finite, where C# throws.
    pub fn set_speed(&mut self, value: f64) -> bool {
        if !value.is_finite() {
            return false;
        }
        self.speed = value;
        true
    }

    /// The largest step one advance takes, seconds.
    pub fn max_step(&self) -> f64 {
        self.max_step
    }

    /// Sets the largest step; false (and no change) for a value that is not positive and finite.
    pub fn set_max_step(&mut self, value: f64) -> bool {
        if !value.is_finite() || value <= 0.0 {
            return false;
        }
        self.max_step = value;
        true
    }

    /// True at (or past) the end.
    pub fn is_at_end(&self) -> bool {
        self.time >= self.duration()
    }

    /// Plays (from the start again when it is at the end and not looping).
    pub fn play(&mut self) {
        if !self.is_looping {
            if self.speed >= 0.0 && self.is_at_end() {
                self.time = 0.0;
            } else if self.speed < 0.0 && self.time <= 0.0 {
                self.time = self.duration();
            }
        }
        self.is_playing = true;
    }

    /// Pauses.
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// Plays when paused, pauses when playing.
    pub fn toggle(&mut self) {
        if self.is_playing {
            self.pause();
        } else {
            self.play();
        }
    }

    /// Moves to a time (clamped to the timeline).
    pub fn seek(&mut self, seconds: f64) {
        self.time = self.timeline.clamp(seconds);
    }

    /// Moves by a number of seconds.
    pub fn seek_by(&mut self, delta: f64) {
        self.seek(self.time + delta);
    }

    /// Pauses and moves by whole frames (C# `Step(frames = 1)`).
    pub fn step(&mut self, frames: i32) {
        self.pause();
        let target = (self.frame() as i64 + frames as i64).clamp(0, self.timeline.frame_count() as i64 - 1);
        self.time = self.timeline.clamp(target as f64 / self.timeline.frames_per_second());
    }

    /// Back to 0, playing.
    pub fn restart(&mut self) {
        self.time = 0.0;
        self.is_playing = true;
    }

    /// Moves the time on by `elapsed` seconds (capped at the largest step, times the speed).
    /// Returns true when this step reached an end (or wrapped, when looping). Pass the frame's
    /// delta: `controller.advance(frame.info.delta)`.
    pub fn advance(&mut self, elapsed: f64) -> bool {
        if !self.is_playing || !(elapsed > 0.0) || self.speed == 0.0 {
            return false;
        }
        let step = elapsed.min(self.max_step) * self.speed;
        let duration = self.duration();
        let next = self.time + step;
        if next >= duration {
            if self.is_looping {
                self.time = next % duration;
                return true;
            }
            let arrived = self.time < duration;
            self.time = duration;
            self.is_playing = false;
            return arrived;
        }
        if next <= 0.0 && self.speed < 0.0 {
            if self.is_looping {
                let wrapped = next % duration;
                self.time = if wrapped < 0.0 { wrapped + duration } else { wrapped };
                return true;
            }
            let arrived = self.time > 0.0;
            self.time = 0.0;
            self.is_playing = false;
            return arrived;
        }
        self.time = next;
        false
    }
}
