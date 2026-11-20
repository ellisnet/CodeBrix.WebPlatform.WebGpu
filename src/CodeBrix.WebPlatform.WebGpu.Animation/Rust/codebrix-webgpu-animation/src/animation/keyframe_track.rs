//! `KeyframeTrack<T>`: keys in time order, evaluated at any time - with per-segment easing, hold
//! keys and a loop mode (C# `KeyframeTrack<T>` and its four kinds `FloatTrack`, `PointTrack`,
//! `ColorTrack`, `BoolTrack`, here type aliases).

use crate::animation::{EasingKind, Keyframe, TrackLoopMode, TrackValue};
use crate::color::Color;
use crate::geometry::PointF;

/// A track of `f32` keys: rotations, scales, opacities, trims, progress (C# `FloatTrack`).
pub type FloatTrack = KeyframeTrack<f32>;
/// A track of points: positions, anchors, two-axis scales (C# `PointTrack`).
pub type PointTrack = KeyframeTrack<PointF>;
/// A track of colours, channel by channel in straight sRGB (C# `ColorTrack`).
pub type ColorTrack = KeyframeTrack<Color>;
/// A track of on/off values, stepping at each key (C# `BoolTrack`).
pub type BoolTrack = KeyframeTrack<bool>;

/// Keys in time order and a [`TrackLoopMode`]; [`KeyframeTrack::evaluate`] gives the value at any
/// time - a PURE function of the time it is given (C# `KeyframeTrack<T>`).
#[derive(Clone, Debug)]
pub struct KeyframeTrack<T> {
    keys: Vec<Keyframe<T>>,
    /// What the track does outside its keys (default Hold; C# `LoopMode`).
    pub loop_mode: TrackLoopMode,
}

impl<T: TrackValue> Default for KeyframeTrack<T> {
    fn default() -> KeyframeTrack<T> {
        KeyframeTrack { keys: Vec::new(), loop_mode: TrackLoopMode::Hold }
    }
}

impl<T: TrackValue> KeyframeTrack<T> {
    /// An empty track (C# `new FloatTrack()` and the others).
    pub fn new() -> KeyframeTrack<T> {
        KeyframeTrack::default()
    }

    /// A track from `(time, value, easing)` keys (C#'s collection initialiser
    /// `{ { 0, a }, { 2, b, EasingKind.CubicInOut } }`). The error says which key is wrong.
    pub fn from_keys(keys: &[(f64, T, EasingKind)]) -> Result<KeyframeTrack<T>, String> {
        let mut track = KeyframeTrack::new();
        for (time, value, easing) in keys {
            track.add(*time, value.clone(), *easing)?;
        }
        Ok(track)
    }

    /// The same track with a loop mode (as C# sets `LoopMode` after building).
    pub fn with_loop_mode(mut self, mode: TrackLoopMode) -> KeyframeTrack<T> {
        self.loop_mode = mode;
        self
    }

    /// The number of keys (C# `Count`).
    pub fn count(&self) -> usize {
        self.keys.len()
    }

    /// The first key's time (0 when there is none).
    pub fn start_time(&self) -> f64 {
        self.keys.first().map_or(0.0, |k| k.time())
    }

    /// The last key's time (0 when there is none).
    pub fn end_time(&self) -> f64 {
        self.keys.last().map_or(0.0, |k| k.time())
    }

    /// From the first key to the last.
    pub fn duration(&self) -> f64 {
        self.end_time() - self.start_time()
    }

    /// Adds a key, kept in time order; a key at the same time as others goes after them (C#
    /// `Add(Keyframe<T>)`).
    pub fn add_key(&mut self, key: Keyframe<T>) -> &mut KeyframeTrack<T> {
        let mut index = self.keys.len();
        while index > 0 && self.keys[index - 1].time() > key.time() {
            index -= 1;
        }
        self.keys.insert(index, key);
        self
    }

    /// Adds a key arriving with a named easing (C# `Add(time, value, easing = Linear)`).
    pub fn add(&mut self, time: f64, value: T, easing: EasingKind) -> Result<&mut KeyframeTrack<T>, String> {
        let key = Keyframe::new(time, value, easing)?;
        Ok(self.add_key(key))
    }

    /// Adds a key arriving along a custom curve (C# `Add(time, value, Func<double, double>)`).
    pub fn add_with_curve(&mut self, time: f64, value: T, curve: impl Fn(f64) -> f64 + 'static) -> Result<&mut KeyframeTrack<T>, String> {
        let key = Keyframe::with_curve(time, value, curve)?;
        Ok(self.add_key(key))
    }

    /// Adds a hold key (C# `AddHold`).
    pub fn add_hold(&mut self, time: f64, value: T) -> Result<&mut KeyframeTrack<T>, String> {
        let key = Keyframe::hold(time, value)?;
        Ok(self.add_key(key))
    }

    /// Removes every key.
    pub fn clear(&mut self) {
        self.keys.clear();
    }

    /// The keys, in time order (C# `GetKeyframes`).
    pub fn get_keyframes(&self) -> &[Keyframe<T>] {
        &self.keys
    }

    /// The value at a time, seconds (C# `Evaluate`): before the first key the first value, after
    /// the last the last (or, with Loop / PingPong, the time mapped back into the keys first);
    /// between two keys the value interpolated by the eased progress of the segment, which the
    /// LATER key's easing decides; up to a hold key, the earlier value. NaN counts as the first
    /// key's time. None when the track has no keys (C# throws).
    pub fn evaluate(&self, seconds: f64) -> Option<T> {
        let keys = &self.keys;
        let first = keys.first()?;
        let last = keys.last()?;
        let (start, end) = (first.time(), last.time());
        let t = if seconds.is_nan() { start } else { self.map_time(seconds, start, end) };
        if t < start {
            return Some(first.value().clone());
        }
        if t >= end {
            return Some(last.value().clone());
        }
        // The last key at or before t starts the segment; its successor lies strictly after t.
        let (mut low, mut high) = (0usize, keys.len() - 1);
        while high - low > 1 {
            let middle = (low + high) / 2;
            if keys[middle].time() <= t {
                low = middle;
            } else {
                high = middle;
            }
        }
        let (a, b) = (&keys[low], &keys[high]);
        if b.is_hold() {
            return Some(a.value().clone());
        }
        let u = (t - a.time()) / (b.time() - a.time());
        Some(T::interpolate(a.value(), b.value(), b.ease(u)))
    }

    /// Maps a time into [start, end] by the loop mode (no change for Hold).
    fn map_time(&self, seconds: f64, start: f64, end: f64) -> f64 {
        let span = end - start;
        if self.loop_mode == TrackLoopMode::Hold || !(span > 0.0) || seconds.is_infinite() {
            return seconds;
        }
        if self.loop_mode == TrackLoopMode::Loop {
            let mut phase = (seconds - start) % span;
            if phase < 0.0 {
                phase += span;
            }
            return start + phase;
        }
        let mut doubled = (seconds - start) % (2.0 * span);
        if doubled < 0.0 {
            doubled += 2.0 * span;
        }
        start + if doubled <= span { doubled } else { (2.0 * span) - doubled }
    }
}
