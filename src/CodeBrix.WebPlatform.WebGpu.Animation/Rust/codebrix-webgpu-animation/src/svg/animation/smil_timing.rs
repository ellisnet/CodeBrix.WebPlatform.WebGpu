//! Crate-internal: SMIL timing - clock values, begin / end lists (offsets and syncbases), the
//! active duration and the intervals, evaluated for a time passed in.

/// A SMIL clock value in seconds: "02:30:03", "02:33", "3.2h", "45min", "30s", "5ms", "12.467".
pub(crate) fn clock(text: &str) -> Option<f64> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let v = if t.contains(':') {
        let parts: Vec<&str> = t.split(':').collect();
        let nums: Option<Vec<f64>> = parts.iter().map(|p| p.trim().parse::<f64>().ok()).collect();
        let n = nums?;
        match n.len() {
            2 => n[0] * 60.0 + n[1],
            3 => n[0] * 3600.0 + n[1] * 60.0 + n[2],
            _ => return None,
        }
    } else if let Some(v) = t.strip_suffix("ms") {
        v.trim().parse::<f64>().ok()? / 1000.0
    } else if let Some(v) = t.strip_suffix("min") {
        v.trim().parse::<f64>().ok()? * 60.0
    } else if let Some(v) = t.strip_suffix('h') {
        v.trim().parse::<f64>().ok()? * 3600.0
    } else if let Some(v) = t.strip_suffix('s') {
        v.trim().parse::<f64>().ok()?
    } else {
        t.parse::<f64>().ok()?
    };
    if v.is_finite() {
        Some(v)
    } else {
        None
    }
}

/// An offset with an optional sign ("+2s", "-0.5s", "1").
fn signed_clock(text: &str) -> Option<f64> {
    let t = text.trim();
    if let Some(r) = t.strip_prefix('-') {
        clock(r).map(|v| -v)
    } else if let Some(r) = t.strip_prefix('+') {
        clock(r)
    } else {
        clock(t)
    }
}

/// One entry of a begin or end list.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TimeSpec {
    Offset(f64),
    /// `id.begin+offset` / `id.end+offset`.
    Sync { id: String, end: bool, offset: f64 },
    Indefinite,
    /// Anything that is not time-based (events, repeat(n), accessKey, wallclock): kept as text for
    /// the report.
    Ignored(String),
}

/// Parses a begin / end list.
pub(crate) fn time_list(text: &str) -> Vec<TimeSpec> {
    text.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).map(time_spec).collect()
}

fn time_spec(s: &str) -> TimeSpec {
    if s == "indefinite" {
        return TimeSpec::Indefinite;
    }
    if let Some(v) = signed_clock(s) {
        return TimeSpec::Offset(v);
    }
    for (token, end) in [(".begin", false), (".end", true)] {
        if let Some(at) = s.find(token) {
            let id = &s[..at];
            let rest = s[at + token.len()..].trim();
            let offset = if rest.is_empty() { Some(0.0) } else { signed_clock(&rest.replace(' ', "")) };
            if !id.is_empty() && !id.contains('(') {
                if let Some(offset) = offset {
                    return TimeSpec::Sync { id: id.to_string(), end, offset };
                }
            }
        }
    }
    TimeSpec::Ignored(s.to_string())
}

/// `restart`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Restart {
    #[default]
    Always,
    WhenNotActive,
    Never,
}

/// The timing attributes of one animation element.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Timing {
    pub(crate) begin: Vec<TimeSpec>,
    pub(crate) end: Vec<TimeSpec>,
    /// The simple duration; None = indefinite (or unspecified).
    pub(crate) dur: Option<f64>,
    /// None = unspecified; Some(inf) = indefinite.
    pub(crate) repeat_count: Option<f64>,
    pub(crate) repeat_dur: Option<f64>,
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) restart: Restart,
    pub(crate) freeze: bool,
}

/// One interval: begin, end (may be infinite) and the active duration it was cut from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Interval {
    pub(crate) begin: f64,
    pub(crate) end: f64,
}

/// Where an animation is at a time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Phase {
    /// The interval's begin (the sandwich's priority).
    pub(crate) begin: f64,
    /// The time within the simple duration (0..dur; dur itself at a frozen whole-iteration end).
    pub(crate) simple: f64,
    /// The repeat iteration (0 first).
    pub(crate) iteration: f64,
}

impl Timing {
    /// The intended active duration of an interval beginning at `begin`, before end cuts.
    fn active_duration(&self, begin: f64, ends: &[f64], has_end: bool) -> Option<f64> {
        let mut ad = match (self.repeat_count, self.repeat_dur) {
            (None, None) => self.dur.unwrap_or(f64::INFINITY),
            (rc, rd) => {
                let a = rc.map(|n| if n.is_infinite() { f64::INFINITY } else { self.dur.map(|d| d * n).unwrap_or(f64::INFINITY) });
                let b = rd;
                match (a, b) {
                    (Some(a), Some(b)) => a.min(b),
                    (Some(a), None) => a,
                    (None, Some(b)) => b,
                    (None, None) => f64::INFINITY,
                }
            }
        };
        if has_end {
            match ends.iter().find(|e| **e > begin) {
                Some(e) => ad = ad.min(e - begin),
                None => {
                    if ends.is_empty() {
                        // only indefinite / ignored ends: no cut
                    } else {
                        return None;
                    }
                }
            }
        }
        Some(ad.max(self.min).min(self.max))
    }

    /// The intervals for begin and end instance times (sorted), up to `horizon`.
    pub(crate) fn intervals(&self, begins: &[f64], ends: &[f64], horizon: f64) -> Vec<Interval> {
        let has_end = !self.end.is_empty() && self.end.iter().any(|e| matches!(e, TimeSpec::Offset(_) | TimeSpec::Sync { .. }));
        let mut out: Vec<Interval> = Vec::new();
        let mut from = f64::NEG_INFINITY;
        let mut last_begin = f64::NEG_INFINITY;
        while out.len() < 10_000 {
            let Some(&b) = begins.iter().find(|b| **b >= from && **b > last_begin) else { break };
            if b > horizon {
                break;
            }
            if self.restart == Restart::Never && !out.is_empty() {
                break;
            }
            let Some(ad) = self.active_duration(b, ends, has_end) else { break };
            let mut end = b + ad;
            if self.restart == Restart::Always {
                if let Some(&b2) = begins.iter().find(|x| **x > b && **x < end) {
                    end = b2;
                }
            }
            out.push(Interval { begin: b, end });
            if end.is_infinite() {
                break;
            }
            from = end;
            last_begin = b;
        }
        out
    }

    /// The phase at `t`, or None when the animation has no effect then (before its first begin,
    /// or after an end with fill="remove").
    pub(crate) fn phase(&self, intervals: &[Interval], t: f64) -> Option<Phase> {
        let current = intervals.iter().rev().find(|i| i.begin <= t)?;
        let active = t < current.end;
        if !active && !self.freeze {
            return None;
        }
        let elapsed = if active { t - current.begin } else { current.end - current.begin };
        let (simple, iteration) = match self.dur {
            Some(d) if d > 0.0 && elapsed.is_finite() => {
                let n = (elapsed / d).floor();
                let s = elapsed - n * d;
                if !active && s.abs() < 1e-9 && n > 0.0 {
                    (d, n - 1.0)
                } else {
                    (s, n)
                }
            }
            _ => (elapsed, 0.0),
        };
        Some(Phase { begin: current.begin, simple, iteration })
    }
}
