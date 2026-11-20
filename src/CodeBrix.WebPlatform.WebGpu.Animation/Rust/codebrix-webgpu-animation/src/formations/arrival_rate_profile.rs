//! `ArrivalRateProfile`: how fast a formation's members arrive over its run - ramping up,
//! steady, tapering off (C# `ArrivalRateProfile`).

/// The arrival rate over the run, 0..1 (C# `ArrivalRateProfile`, the same defaults): from
/// `start_rate` up to 1 by `ramp_exponent` until `ramp_until`, 1 until `taper_from`, then down by
/// `taper_exponent` towards `end_rate`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrivalRateProfile {
    /// Where the ramp ends (0.3).
    pub ramp_until: f64,
    /// Where the taper begins (0.72).
    pub taper_from: f64,
    /// The rate at the start (0.06).
    pub start_rate: f64,
    /// The ramp's exponent (1.6).
    pub ramp_exponent: f64,
    /// The taper's exponent (2.1).
    pub taper_exponent: f64,
    /// The rate added at the end (0.012).
    pub end_rate: f64,
}

impl Default for ArrivalRateProfile {
    fn default() -> ArrivalRateProfile {
        ArrivalRateProfile { ramp_until: 0.3, taper_from: 0.72, start_rate: 0.06, ramp_exponent: 1.6, taper_exponent: 2.1, end_rate: 0.012 }
    }
}

impl ArrivalRateProfile {
    /// The rate at a fraction of the run (C# `GetRate`).
    pub fn get_rate(&self, fraction: f64) -> f64 {
        let u = fraction;
        if u < self.ramp_until {
            self.start_rate + ((1.0 - self.start_rate) * (u / self.ramp_until).powf(self.ramp_exponent))
        } else if u < self.taper_from {
            1.0
        } else {
            (1.0 - ((u - self.taper_from) / (1.0 - self.taper_from))).powf(self.taper_exponent) + self.end_rate
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        let ok = |v: f64| v.is_finite();
        if !ok(self.ramp_until) || self.ramp_until < 0.0 || self.ramp_until > 1.0 {
            return Err("RampUntil must be a fraction, 0..1.".to_string());
        }
        if !ok(self.taper_from) || self.taper_from < self.ramp_until || self.taper_from >= 1.0 {
            return Err("TaperFrom must be at least RampUntil and below 1.".to_string());
        }
        if !ok(self.start_rate) || self.start_rate < 0.0 || !ok(self.end_rate) || self.end_rate < 0.0 {
            return Err("The start and end rates must be finite, 0 or more.".to_string());
        }
        if !ok(self.ramp_exponent) || self.ramp_exponent <= 0.0 || !ok(self.taper_exponent) || self.taper_exponent <= 0.0 {
            return Err("The exponents must be positive and finite.".to_string());
        }
        Ok(())
    }
}
