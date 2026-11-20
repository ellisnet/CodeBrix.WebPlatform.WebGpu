//! `SafeAreaTimeRule`: when a guide applies - its final seconds, or between two times (C#
//! `SafeAreaTimeRule`).

/// When a guide applies (C# `SafeAreaTimeRule`): during the video's final seconds, or between two
/// times.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SafeAreaTimeRule {
    is_final: bool,
    start_time: f64,
    end_time: f64,
    duration: f64,
}

impl SafeAreaTimeRule {
    /// The final `seconds` of the video (C# `FinalSeconds`). The error says what to do with a
    /// value that is not positive and finite.
    pub fn final_seconds(seconds: f64) -> Result<SafeAreaTimeRule, String> {
        if !(seconds > 0.0) || seconds.is_infinite() {
            return Err("The final seconds must be a positive, finite number.".to_string());
        }
        Ok(SafeAreaTimeRule { is_final: true, start_time: 0.0, end_time: 0.0, duration: seconds })
    }

    /// Between two times, inclusive (C# `Between`). The error says what is wrong.
    pub fn between(start_time: f64, end_time: f64) -> Result<SafeAreaTimeRule, String> {
        if !start_time.is_finite() {
            return Err("The start time must be a finite number of seconds.".to_string());
        }
        if !end_time.is_finite() || end_time < start_time {
            return Err("The end time must be finite and not before the start.".to_string());
        }
        Ok(SafeAreaTimeRule { is_final: false, start_time, end_time, duration: end_time - start_time })
    }

    /// The start (C# `StartTime`; 0 for final seconds).
    pub fn start_time(&self) -> f64 {
        self.start_time
    }

    /// The end (C# `EndTime`; 0 for final seconds).
    pub fn end_time(&self) -> f64 {
        self.end_time
    }

    /// How long it applies (C# `Duration`).
    pub fn duration(&self) -> f64 {
        self.duration
    }

    /// True for a final-seconds rule (C# `IsFinalSeconds`).
    pub fn is_final_seconds(&self) -> bool {
        self.is_final
    }

    /// Whether it applies at a time of a video of a duration (C# `AppliesAt`): with no time,
    /// always; a final-seconds rule with no duration, always.
    pub fn applies_at(&self, seconds: Option<f64>, duration: Option<f64>) -> bool {
        let Some(s) = seconds else { return true };
        if self.is_final {
            return match duration {
                None => true,
                Some(d) => s >= d - self.duration,
            };
        }
        s >= self.start_time && s <= self.end_time
    }
}
