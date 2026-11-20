//! [`AnimationState`]: one clip playing on an [`AnimationPlayer`](crate::AnimationPlayer) (C#:
//! `AnimationState`). CPU only, tested on the host.

use crate::animation::AnimationLoopMode;
use crate::models::ModelAnimation;

/// One clip playing on an [`AnimationPlayer`](crate::AnimationPlayer) (C#: `AnimationState`): its
/// playhead, speed, loop mode, blend weight (with an optional fade) and layer, plus "what happened
/// in the last update" flags. The player's `play`, `cross_fade` and `blend` hand it out; reach it
/// again with the player's `get_state` / `find_state`.
///
/// THE DIFFERENCE FROM C#: a state belongs to its player, so it cannot stop itself at once.
/// [`AnimationState::stop`] and a [`AnimationState::fade_to`] of weight 0 over no time with
/// `stop_when_zero` MARK the state; the player removes it (and returns what it animated to the
/// rest pose) at the start of its next call.
#[derive(Clone, Debug)]
pub struct AnimationState {
    clip: usize,
    name: Option<String>,
    duration: f32,
    /// How playback continues at the clip's end; [`AnimationLoopMode::Once`] holds the last
    /// frame (C#: `LoopMode`).
    pub loop_mode: AnimationLoopMode,
    /// The playback rate: 1 normal, 0.5 half speed, negative backwards; multiplied by the
    /// player's speed (C#: `Speed`).
    pub speed: f32,
    /// True to hold the playhead; the clip still contributes to the pose (C#: `Paused`).
    pub paused: bool,
    layer: i32,
    weight: f32,
    fade_rate: f32,
    fade_target: f32,
    stop_when_faded: bool,
    stop_requested: bool,
    phase: f32,
    playhead: f32,
    loop_count: u32,
    is_finished: bool,
    finished_this_update: bool,
    looped_this_update: bool,
    /// The key cursor of each channel of the clip.
    pub(crate) cursors: Vec<usize>,
}

impl AnimationState {
    /// A state playing `clip` (C#: the pooled state's `Start`).
    pub(crate) fn start(index: usize, clip: &ModelAnimation, loop_mode: AnimationLoopMode, speed: f32, start_time: f32, weight: f32, layer: i32) -> AnimationState {
        let mut state = AnimationState {
            clip: index,
            name: clip.name.clone(),
            duration: clip.duration(),
            loop_mode,
            speed,
            paused: false,
            layer,
            weight: weight.max(0.0),
            fade_rate: 0.0,
            fade_target: 0.0,
            stop_when_faded: false,
            stop_requested: false,
            phase: 0.0,
            playhead: 0.0,
            loop_count: 0,
            is_finished: false,
            finished_this_update: false,
            looped_this_update: false,
            cursors: vec![0; clip.channels.len().max(1)],
        };
        state.seek(start_time);
        state
    }

    /// The clip's index in the model's animations (C#: `ClipIndex`).
    pub fn clip_index(&self) -> usize {
        self.clip
    }

    /// The clip's name, or None (C#: `Name`).
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The clip's length in seconds (C#: `Duration`).
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// The blend layer: higher layers override lower ones (by their weight) on the properties
    /// they animate (C#: `Layer`).
    pub fn layer(&self) -> i32 {
        self.layer
    }

    /// False once the state was stopped and waits to leave its player (C#: `IsActive`).
    pub fn is_active(&self) -> bool {
        !self.stop_requested
    }

    /// The blend weight: 0..1 contributes that share; states of one layer whose weights add up
    /// past 1 are normalised (C#: `Weight`).
    pub fn weight(&self) -> f32 {
        self.weight
    }

    /// Sets the blend weight (negative counts as 0) and cancels a running fade (C#: the `Weight`
    /// setter).
    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight.max(0.0);
        self.fade_rate = 0.0;
        self.stop_when_faded = false;
    }

    /// True while a fade ([`AnimationState::fade_to`], a cross-fade) changes the weight (C#:
    /// `IsFading`).
    pub fn is_fading(&self) -> bool {
        self.fade_rate != 0.0
    }

    /// The playhead in clip seconds, 0..duration; ping-pong reports the folded position (C#:
    /// `Time`).
    pub fn time(&self) -> f32 {
        self.playhead
    }

    /// Seeks: values outside the clip wrap (loop, ping-pong) or clamp (once); a once-clip seeked
    /// before its end plays again (C#: the `Time` setter).
    pub fn set_time(&mut self, time: f32) {
        self.seek(time);
    }

    /// The playhead as a fraction of the duration, 0..1; 0 for an empty clip (C#:
    /// `NormalizedTime`).
    pub fn normalized_time(&self) -> f32 {
        if self.duration > 0.0 {
            self.playhead / self.duration
        } else {
            0.0
        }
    }

    /// Seeks to a fraction of the duration (C#: the `NormalizedTime` setter).
    pub fn set_normalized_time(&mut self, fraction: f32) {
        self.seek(fraction * self.duration);
    }

    /// Completed cycles: loop wraps, ping-pong full forward-and-back trips (C#: `LoopCount`).
    pub fn loop_count(&self) -> u32 {
        self.loop_count
    }

    /// True when a once-clip has reached its end; it holds the last frame (C#: `IsFinished`).
    pub fn is_finished(&self) -> bool {
        self.is_finished
    }

    /// True when the player's last update finished the clip, once mode (C#:
    /// `FinishedThisUpdate`).
    pub fn finished_this_update(&self) -> bool {
        self.finished_this_update
    }

    /// True when the player's last update wrapped the clip, loop and ping-pong modes (C#:
    /// `LoopedThisUpdate`).
    pub fn looped_this_update(&self) -> bool {
        self.looped_this_update
    }

    /// Fades the weight linearly to `weight` over `duration` seconds of player time; 0 or less
    /// sets it at once. With `stop_when_zero` the state stops when it reaches weight 0 (C#:
    /// `FadeTo(weight, duration, stopWhenZero)`).
    pub fn fade_to(&mut self, weight: f32, duration: f32, stop_when_zero: bool) {
        let weight = weight.max(0.0);
        if duration <= 0.0 || weight == self.weight {
            self.weight = weight;
            self.fade_rate = 0.0;
            self.stop_when_faded = stop_when_zero && weight == 0.0;
            if self.stop_when_faded {
                self.stop_requested = true;
            }
            return;
        }
        self.fade_target = weight;
        self.fade_rate = (weight - self.weight) / duration;
        self.stop_when_faded = stop_when_zero;
    }

    /// Stops the clip: the player removes it at the start of its next call, and the nodes it
    /// animated return to the rest pose unless other states animate them (C#: `Stop`).
    pub fn stop(&mut self) {
        self.stop_requested = true;
    }

    /// True when the state should be removed once its fade reaches zero.
    pub(crate) fn stops_when_faded(&self) -> bool {
        self.stop_when_faded
    }

    /// True when the state was stopped and waits for its player to remove it.
    pub(crate) fn stop_requested(&self) -> bool {
        self.stop_requested
    }

    /// Clears the per-update flags.
    pub(crate) fn clear_flags(&mut self) {
        self.finished_this_update = false;
        self.looped_this_update = false;
    }

    /// Moves the playhead by clip seconds (already scaled by both speeds).
    pub(crate) fn advance(&mut self, seconds: f32) {
        if self.paused || seconds == 0.0 || seconds.is_nan() {
            return;
        }
        let duration = self.duration;
        if duration <= 0.0 {
            self.phase = 0.0;
            self.playhead = 0.0;
            if self.loop_mode == AnimationLoopMode::Once && !self.is_finished {
                self.is_finished = true;
                self.finished_this_update = true;
            }
            return;
        }
        match self.loop_mode {
            AnimationLoopMode::Once => {
                if self.is_finished {
                    return;
                }
                self.phase += seconds;
                if self.phase >= duration || (seconds < 0.0 && self.phase <= 0.0) {
                    self.phase = if self.phase >= duration { duration } else { 0.0 };
                    self.is_finished = true;
                    self.finished_this_update = true;
                }
                self.playhead = self.phase;
            }
            AnimationLoopMode::PingPong => {
                let period = 2.0 * duration;
                self.phase += seconds;
                if self.phase >= period || self.phase < 0.0 {
                    self.loop_count = self.loop_count.saturating_add(wrap(&mut self.phase, period));
                    self.looped_this_update = true;
                }
                self.playhead = if self.phase > duration { period - self.phase } else { self.phase };
            }
            AnimationLoopMode::Loop => {
                self.phase += seconds;
                if self.phase >= duration || self.phase < 0.0 {
                    self.loop_count = self.loop_count.saturating_add(wrap(&mut self.phase, duration));
                    self.looped_this_update = true;
                }
                self.playhead = self.phase;
            }
        }
    }

    /// Moves the weight along its fade by `seconds` of player time; true when the state faded
    /// out and should stop.
    pub(crate) fn advance_fade(&mut self, seconds: f32) -> bool {
        if self.fade_rate == 0.0 || seconds <= 0.0 {
            return false;
        }
        let next = self.weight + self.fade_rate * seconds;
        if (self.fade_rate > 0.0 && next >= self.fade_target) || (self.fade_rate < 0.0 && next <= self.fade_target) {
            self.weight = self.fade_target;
            self.fade_rate = 0.0;
            return self.stop_when_faded && self.fade_target == 0.0;
        }
        self.weight = next;
        false
    }

    fn seek(&mut self, time: f32) {
        let duration = self.duration;
        self.is_finished = false;
        if duration <= 0.0 || time.is_nan() {
            self.phase = 0.0;
            self.playhead = 0.0;
            return;
        }
        match self.loop_mode {
            AnimationLoopMode::Once => {
                self.phase = time.clamp(0.0, duration);
                self.is_finished = time >= duration;
                self.playhead = self.phase;
            }
            AnimationLoopMode::PingPong => {
                let period = 2.0 * duration;
                self.phase = time;
                if self.phase >= period || self.phase < 0.0 {
                    wrap(&mut self.phase, period);
                }
                self.playhead = if self.phase > duration { period - self.phase } else { self.phase };
            }
            AnimationLoopMode::Loop => {
                self.phase = time;
                if self.phase >= duration || self.phase < 0.0 {
                    wrap(&mut self.phase, duration);
                }
                self.playhead = self.phase;
            }
        }
    }
}

impl std::fmt::Display for AnimationState {
    /// For example `AnimationState 'walk' 0.25/0.667 s weight 1` (C#: `ToString`).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name.clone().unwrap_or_else(|| self.clip.to_string());
        write!(f, "AnimationState '{}' {}/{} s weight {}", name, short(self.playhead), short(self.duration), short(self.weight))
    }
}

/// A number with at most three decimals and no trailing zeros (C#'s "0.###").
fn short(value: f32) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() || text == "-" {
        "0".to_string()
    } else {
        text.to_string()
    }
}

/// Wraps a value into [0, period) and returns how many periods were crossed.
fn wrap(value: &mut f32, period: f32) -> u32 {
    let cycles = (*value / period).floor();
    *value -= cycles * period;
    if *value >= period || *value < 0.0 {
        *value = 0.0;
    }
    cycles.abs().min(u32::MAX as f32) as u32
}
