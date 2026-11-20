//! `ArrivalSchedule`: when each of n members arrives, from a rate over the run (C#
//! `ArrivalSchedule`).

use crate::formations::ArrivalRateProfile;

const STEPS: usize = 4000;

/// Arrival times between a first and a last, spaced by a rate (C# `ArrivalSchedule`): the rate is
/// summed over 4000 steps and the times are its quantiles - where the rate is high, members
/// arrive close together.
#[derive(Clone, Debug, PartialEq)]
pub struct ArrivalSchedule {
    first_time: f64,
    last_time: f64,
    cumulative: Vec<f64>,
}

impl ArrivalSchedule {
    /// A schedule with a rate profile (None: the default profile) (C# `ArrivalSchedule(firstTime,
    /// lastTime, profile = null)`). The error says what is wrong.
    pub fn new(first_time: f64, last_time: f64, profile: Option<ArrivalRateProfile>) -> Result<ArrivalSchedule, String> {
        let profile = profile.unwrap_or_default();
        profile.validate()?;
        ArrivalSchedule::with_rate(first_time, last_time, move |u| profile.get_rate(u))
    }

    /// A schedule with a rate function of the run's fraction (C# `ArrivalSchedule(firstTime,
    /// lastTime, Func<double, double> rate)`). The error says what is wrong.
    pub fn with_rate(first_time: f64, last_time: f64, rate: impl Fn(f64) -> f64) -> Result<ArrivalSchedule, String> {
        if !first_time.is_finite() {
            return Err("The first time must be a finite number of seconds.".to_string());
        }
        if !last_time.is_finite() || last_time < first_time {
            return Err("The last time must be finite and not before the first.".to_string());
        }
        let mut cumulative = vec![0.0f64; STEPS + 1];
        for i in 1..=STEPS {
            let u = (i as f64 - 0.5) / STEPS as f64;
            let r = rate(u);
            if !r.is_finite() || r < 0.0 {
                return Err("The rate must be finite and not negative.".to_string());
            }
            cumulative[i] = cumulative[i - 1] + r;
        }
        if !(cumulative[STEPS] > 0.0) {
            return Err("The rate is 0 everywhere: give it a positive value somewhere.".to_string());
        }
        Ok(ArrivalSchedule { first_time, last_time, cumulative })
    }

    /// The first arrival (C# `FirstTime`).
    pub fn first_time(&self) -> f64 {
        self.first_time
    }

    /// The last arrival (C# `LastTime`).
    pub fn last_time(&self) -> f64 {
        self.last_time
    }

    /// The arrival times of `count` members, in order, the last exactly at the last time (C#
    /// `GetTimes`).
    pub fn get_times(&self, count: usize) -> Vec<f64> {
        let c = &self.cumulative;
        let mut result = vec![0.0f64; count];
        let mut j = 0usize;
        for (k, slot) in result.iter_mut().enumerate() {
            let target = c[STEPS] * k as f64 / (count.saturating_sub(1)).max(1) as f64;
            while j < STEPS && c[j + 1] < target {
                j += 1;
            }
            let span = c[STEPS.min(j + 1)] - c[j];
            let f = if span <= 0.0 { 0.0 } else { (target - c[j]) / span };
            let u = ((j as f64 + f) / STEPS as f64).clamp(0.0, 1.0);
            *slot = self.first_time + ((self.last_time - self.first_time) * u);
        }
        if count > 0 {
            result[count - 1] = self.last_time;
        }
        result
    }
}
