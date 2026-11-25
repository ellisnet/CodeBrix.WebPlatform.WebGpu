//! `RippleRain`: rain on a ripple field as a FUNCTION OF TIME (no C# equivalent) - the drops of
//! each one-second window of the session come from that window's own seeded sequence, so the
//! drops alive at any time are found without replaying anything before it.

use crate::planes::DrawingRandom;
use crate::water::RippleDrop;
use crate::{PointF, RectangleF};

/// Rain on a [`RippleField`](super::RippleField) as a FUNCTION OF TIME (no C# equivalent).
///
/// THE RULE. Session second `k` (the window `k <= t < k + 1`, k a whole number, negative too)
/// holds a Poisson(`rate`) number of drops, counted by Knuth's method on
/// `DrawingRandom::for_key(seed, k)`; then, from the same sequence and per drop in this order,
/// its time (uniform in the window), its x and y (uniform in `area`, or in the field's place
/// when `area` is `None`), its strength and its size (uniform in their ranges). A drop whose
/// time is before `start` or at or after `end` is left out (the others keep their places), so
/// changing `start` or `end` never moves the drops that remain. The drops alive at `t` come from
/// the windows `floor(t - life) ..= floor(t)` only (`life` is the field's
/// [`drop_life`](super::RippleField::drop_life)): seeking to any time shows exactly the rain
/// that playing to it shows, and frames may be drawn in any order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RippleRain {
    /// Drops per second on average, 0 to [`RippleRain::MAX_RATE`]. Default 2.
    pub rate: f32,
    /// Session seconds the rain starts. Default 0.
    pub start: f64,
    /// Session seconds it stops (drops already fallen still spread and fade); `None` = never.
    /// Default `None`.
    pub end: Option<f64>,
    /// The range a drop's strength is drawn from (1 = the field's height). Default (0.4, 1).
    pub strength: (f32, f32),
    /// The range a drop's size is drawn from (it scales the wavelength). Default (0.7, 1.3).
    pub size: (f32, f32),
    /// Where drops fall, calibrated units; `None` = the field's place. Default `None`.
    pub area: Option<RectangleF>,
    /// The seed. Default 0.
    pub seed: i64,
}

impl Default for RippleRain {
    fn default() -> RippleRain {
        RippleRain { rate: 2.0, start: 0.0, end: None, strength: (0.4, 1.0), size: (0.7, 1.3), area: None, seed: 0 }
    }
}

impl RippleRain {
    /// The highest rate (drops per second) rain may have.
    pub const MAX_RATE: f32 = 200.0;

    /// Rain of `rate` drops per second from `seed`, with the other defaults (no C# equivalent).
    pub fn new(rate: f32, seed: i64) -> RippleRain {
        RippleRain { rate, seed, ..RippleRain::default() }
    }

    /// Checks the rain; the error says what to change.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !(self.rate >= 0.0 && self.rate <= RippleRain::MAX_RATE) {
            return Err(format!("The rain's rate must be 0 to {} drops per second (it is {}).", RippleRain::MAX_RATE, self.rate));
        }
        if !self.start.is_finite() {
            return Err("The rain's start must be a finite number of seconds.".to_string());
        }
        if let Some(end) = self.end {
            if !(end.is_finite() && end >= self.start) {
                return Err(format!("The rain's end must be a finite time at or after its start ({}); it is {end}.", self.start));
            }
        }
        let (s0, s1) = self.strength;
        if !(s0.is_finite() && s1.is_finite() && s0 >= 0.0 && s0 <= s1) {
            return Err(format!("The rain's strength range must be finite, at least 0 and low to high (it is {s0} to {s1})."));
        }
        let (z0, z1) = self.size;
        if !(z0.is_finite() && z1.is_finite() && z0 > 0.0 && z0 <= z1) {
            return Err(format!("The rain's size range must be finite, above 0 and low to high (it is {z0} to {z1})."));
        }
        if let Some(area) = self.area {
            if area.is_empty() || !(area.x.is_finite() && area.y.is_finite() && area.width.is_finite() && area.height.is_finite()) {
                return Err("The rain's area must be a finite rectangle with an area (or None for the field's place).".to_string());
            }
        }
        Ok(())
    }

    /// The drops of window `k` (seconds `k..k + 1`), in the order they were drawn, `start` and
    /// `end` applied. `place` is used when the rain has no area of its own.
    pub(crate) fn window(&self, k: i64, place: RectangleF, out: &mut Vec<RippleDrop>) {
        let window_start = k as f64;
        if window_start + 1.0 <= self.start {
            return;
        }
        if let Some(end) = self.end {
            if window_start >= end {
                return;
            }
        }
        let mut random = DrawingRandom::for_key(self.seed, k);
        let count = poisson(&mut random, self.rate as f64);
        let area = self.area.unwrap_or(place);
        for _ in 0..count {
            let time = window_start + random.next_double();
            let x = area.x as f64 + area.width as f64 * random.next_double();
            let y = area.y as f64 + area.height as f64 * random.next_double();
            let strength = random.uniform(self.strength.0 as f64, self.strength.1 as f64) as f32;
            let size = random.uniform(self.size.0 as f64, self.size.1 as f64) as f32;
            if time < self.start || self.end.is_some_and(|end| time >= end) {
                continue;
            }
            out.push(RippleDrop { center: PointF::new(x as f32, y as f32), time, strength, size });
        }
    }
}

/// Knuth's Poisson count: multiply uniform numbers until the product falls to `e^-mean`.
fn poisson(random: &mut DrawingRandom, mean: f64) -> u32 {
    let limit = (-mean).exp();
    let mut product = random.next_double();
    let mut count = 0u32;
    while product > limit {
        count += 1;
        product *= random.next_double();
    }
    count
}
