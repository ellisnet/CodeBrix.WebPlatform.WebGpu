//! The shared checks and the eased fraction of the built-in arrival styles (internal; C#'s
//! `ArrivalMath`).

use crate::animation::{Easing, EasingKind};

pub(crate) fn check_times(start_time: f64, arrival_time: f64) -> Result<(), String> {
    if !start_time.is_finite() {
        return Err("The start time must be a finite number of seconds.".to_string());
    }
    if !arrival_time.is_finite() || arrival_time <= start_time {
        return Err("The arrival must be finite and after the start.".to_string());
    }
    Ok(())
}

/// Linear progress from the start to the arrival, clamped to 0..1, then eased.
pub(crate) fn eased(seconds: f64, start_time: f64, arrival_time: f64, easing: EasingKind) -> f64 {
    let u = ((seconds - start_time) / (arrival_time - start_time)).clamp(0.0, 1.0);
    Easing::evaluate(easing, u)
}

pub(crate) fn lerp(from: f32, to: f32, fraction: f64) -> f32 {
    (from as f64 + ((to as f64 - from as f64) * fraction)) as f32
}
