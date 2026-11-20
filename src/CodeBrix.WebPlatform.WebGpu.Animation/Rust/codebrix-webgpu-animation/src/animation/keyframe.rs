//! `Keyframe<T>`: one key of a track - a time, a value, and how the segment that ENDS at it is
//! eased (C# `Keyframe<T>`).

use std::fmt;
use std::rc::Rc;

use crate::animation::{Easing, EasingKind};

/// One key of a [`KeyframeTrack`](crate::animation::KeyframeTrack): its time (seconds), its
/// value, and the easing of the segment that arrives at it - a named curve, a custom curve, or a
/// HOLD (the previous value is kept until this key's time, then it jumps) (C# `Keyframe<T>`).
#[derive(Clone)]
pub struct Keyframe<T> {
    time: f64,
    value: T,
    easing: EasingKind,
    curve: Option<Rc<dyn Fn(f64) -> f64>>,
    is_hold: bool,
}

impl<T: Clone> Keyframe<T> {
    /// A key arriving with a named easing (C# `Keyframe(time, value, easing = Linear)`). The
    /// error says what to do when the time is not finite.
    pub fn new(time: f64, value: T, easing: EasingKind) -> Result<Keyframe<T>, String> {
        if !time.is_finite() {
            return Err("A keyframe's time must be a finite number of seconds.".to_string());
        }
        Ok(Keyframe { time, value, easing, curve: None, is_hold: false })
    }

    /// A key arriving along a custom curve of the segment's progress 0..1 (C#
    /// `Keyframe(time, value, Func<double, double>)`).
    pub fn with_curve(time: f64, value: T, curve: impl Fn(f64) -> f64 + 'static) -> Result<Keyframe<T>, String> {
        let mut key = Keyframe::new(time, value, EasingKind::Linear)?;
        key.curve = Some(Rc::new(curve));
        Ok(key)
    }

    /// A HOLD key: the previous key's value stays until this time, then this value (C#
    /// `Keyframe<T>.Hold`).
    pub fn hold(time: f64, value: T) -> Result<Keyframe<T>, String> {
        let mut key = Keyframe::new(time, value, EasingKind::Linear)?;
        key.is_hold = true;
        Ok(key)
    }

    /// The time, seconds.
    pub fn time(&self) -> f64 {
        self.time
    }

    /// The value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// The named easing of the segment arriving here (Linear for a custom curve or a hold).
    pub fn easing(&self) -> EasingKind {
        self.easing
    }

    /// True when the key has a custom curve.
    pub fn has_custom_curve(&self) -> bool {
        self.curve.is_some()
    }

    /// True for a hold key.
    pub fn is_hold(&self) -> bool {
        self.is_hold
    }

    /// The same key (time, easing or curve, hold) with another value (C# `WithValue`).
    pub fn with_value(&self, value: T) -> Keyframe<T> {
        Keyframe { time: self.time, value, easing: self.easing, curve: self.curve.clone(), is_hold: self.is_hold }
    }

    /// The eased progress through the segment that ends at this key.
    pub(crate) fn ease(&self, fraction: f64) -> f64 {
        match &self.curve {
            Some(curve) => curve(fraction),
            None => Easing::evaluate(self.easing, fraction),
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Keyframe<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_hold {
            write!(f, "{}: {:?} (hold)", self.time, self.value)
        } else if self.curve.is_some() {
            write!(f, "{}: {:?} (custom curve)", self.time, self.value)
        } else {
            write!(f, "{}: {:?} ({:?})", self.time, self.value, self.easing)
        }
    }
}
