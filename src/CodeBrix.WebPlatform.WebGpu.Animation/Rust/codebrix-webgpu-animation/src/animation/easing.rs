//! `Easing`: every easing curve of CodeBrix.Imaging.Drawing's `Easing` class, as pure functions
//! of a progress value `t` (clamped to 0..1, NaN treated as 0), with the same formulas.

use std::f64::consts::PI;

use crate::animation::cubic_bezier_easing::CubicBezierEasing;
use crate::animation::easing_kind::EasingKind;
use crate::animation::step_position::StepPosition;

/// The easing curves (C# `Easing`, a static class: here a type with associated functions).
/// Every curve takes a progress value, clamps it to 0..1 (NaN becomes 0) and returns the eased
/// value: 0 at 0 and 1 at 1; the Back and Elastic curves overshoot in between.
pub struct Easing;

const ELASTIC_PERIOD_IN: f64 = (2.0 * PI) / 3.0;
const ELASTIC_PERIOD_IN_OUT: f64 = (2.0 * PI) / 4.5;

impl Easing {
    /// The default overshoot of the Back curves (C# `DefaultBackOvershoot`).
    pub const DEFAULT_BACK_OVERSHOOT: f64 = 1.70158;

    /// Clamps `t` to 0..1; NaN becomes 0.
    pub fn clamp01(t: f64) -> f64 {
        if t.is_nan() {
            0.0
        } else {
            t.clamp(0.0, 1.0)
        }
    }

    /// The value `t` of the way from `from` to `to` (not clamped).
    pub fn lerp(from: f64, to: f64, t: f64) -> f64 {
        from + ((to - from) * t)
    }

    /// The single-precision form of [`Easing::lerp`] (C# `Lerp(float, float, double)`).
    pub fn lerp_f32(from: f32, to: f32, t: f64) -> f32 {
        (from as f64 + ((to as f64 - from as f64) * t)) as f32
    }

    /// No easing: `t` clamped.
    pub fn linear(t: f64) -> f64 {
        Easing::clamp01(t)
    }

    /// The smoothstep curve, 3t^2 - 2t^3.
    pub fn smooth_step(t: f64) -> f64 {
        let t = Easing::clamp01(t);
        t * t * (3.0 - (2.0 * t))
    }

    /// Smoothstep of `x` between two edges (C# `SmoothStep(edge0, edge1, x)`). Returns None when
    /// the edges are equal (C# throws ArgumentException).
    pub fn smooth_step_between(edge0: f64, edge1: f64, x: f64) -> Option<f64> {
        if edge0 == edge1 {
            return None;
        }
        Some(Easing::smooth_step((x - edge0) / (edge1 - edge0)))
    }

    /// Perlin's smootherstep, 6t^5 - 15t^4 + 10t^3.
    pub fn smoother_step(t: f64) -> f64 {
        let t = Easing::clamp01(t);
        t * t * t * ((t * ((6.0 * t) - 15.0)) + 10.0)
    }

    /// Quadratic in.
    pub fn quad_in(t: f64) -> f64 {
        power_in(t, 2)
    }

    /// Quadratic out.
    pub fn quad_out(t: f64) -> f64 {
        power_out(t, 2)
    }

    /// Quadratic in and out.
    pub fn quad_in_out(t: f64) -> f64 {
        power_in_out(t, 2)
    }

    /// Cubic in.
    pub fn cubic_in(t: f64) -> f64 {
        power_in(t, 3)
    }

    /// Cubic out.
    pub fn cubic_out(t: f64) -> f64 {
        power_out(t, 3)
    }

    /// Cubic in and out.
    pub fn cubic_in_out(t: f64) -> f64 {
        power_in_out(t, 3)
    }

    /// Quartic in.
    pub fn quart_in(t: f64) -> f64 {
        power_in(t, 4)
    }

    /// Quartic out.
    pub fn quart_out(t: f64) -> f64 {
        power_out(t, 4)
    }

    /// Quartic in and out.
    pub fn quart_in_out(t: f64) -> f64 {
        power_in_out(t, 4)
    }

    /// Quintic in.
    pub fn quint_in(t: f64) -> f64 {
        power_in(t, 5)
    }

    /// Quintic out.
    pub fn quint_out(t: f64) -> f64 {
        power_out(t, 5)
    }

    /// Quintic in and out.
    pub fn quint_in_out(t: f64) -> f64 {
        power_in_out(t, 5)
    }

    /// Sine in.
    pub fn sine_in(t: f64) -> f64 {
        1.0 - (Easing::clamp01(t) * PI / 2.0).cos()
    }

    /// Sine out.
    pub fn sine_out(t: f64) -> f64 {
        (Easing::clamp01(t) * PI / 2.0).sin()
    }

    /// Sine in and out.
    pub fn sine_in_out(t: f64) -> f64 {
        (1.0 - (Easing::clamp01(t) * PI).cos()) / 2.0
    }

    /// Exponential in.
    pub fn expo_in(t: f64) -> f64 {
        let t = Easing::clamp01(t);
        if t == 0.0 {
            0.0
        } else {
            2f64.powf((10.0 * t) - 10.0)
        }
    }

    /// Exponential out.
    pub fn expo_out(t: f64) -> f64 {
        let t = Easing::clamp01(t);
        if t == 1.0 {
            1.0
        } else {
            1.0 - 2f64.powf(-10.0 * t)
        }
    }

    /// Exponential in and out.
    pub fn expo_in_out(t: f64) -> f64 {
        let t = Easing::clamp01(t);
        if t == 0.0 || t == 1.0 {
            return t;
        }
        if t < 0.5 {
            2f64.powf((20.0 * t) - 10.0) / 2.0
        } else {
            (2.0 - 2f64.powf((-20.0 * t) + 10.0)) / 2.0
        }
    }

    /// Circular in.
    pub fn circ_in(t: f64) -> f64 {
        let t = Easing::clamp01(t);
        1.0 - (1.0 - (t * t)).sqrt()
    }

    /// Circular out.
    pub fn circ_out(t: f64) -> f64 {
        let t = Easing::clamp01(t) - 1.0;
        (1.0 - (t * t)).sqrt()
    }

    /// Circular in and out.
    pub fn circ_in_out(t: f64) -> f64 {
        let t = Easing::clamp01(t);
        if t < 0.5 {
            let u = 2.0 * t;
            return (1.0 - (1.0 - (u * u)).sqrt()) / 2.0;
        }
        let v = (-2.0 * t) + 2.0;
        ((1.0 - (v * v)).sqrt() + 1.0) / 2.0
    }

    /// Back in, with the default overshoot.
    pub fn back_in(t: f64) -> f64 {
        Easing::back_in_with(t, Easing::DEFAULT_BACK_OVERSHOOT)
    }

    /// Back in with an overshoot (C# `BackIn(t, overshoot)`).
    pub fn back_in_with(t: f64, overshoot: f64) -> f64 {
        let t = Easing::clamp01(t);
        ((overshoot + 1.0) * t * t * t) - (overshoot * t * t)
    }

    /// Back out, with the default overshoot.
    pub fn back_out(t: f64) -> f64 {
        Easing::back_out_with(t, Easing::DEFAULT_BACK_OVERSHOOT)
    }

    /// Back out with an overshoot (C# `BackOut(t, overshoot)`).
    pub fn back_out_with(t: f64, overshoot: f64) -> f64 {
        let u = Easing::clamp01(t) - 1.0;
        1.0 + ((overshoot + 1.0) * u * u * u) + (overshoot * u * u)
    }

    /// Back in and out, with the default overshoot.
    pub fn back_in_out(t: f64) -> f64 {
        Easing::back_in_out_with(t, Easing::DEFAULT_BACK_OVERSHOOT)
    }

    /// Back in and out with an overshoot (C# `BackInOut(t, overshoot)`).
    pub fn back_in_out_with(t: f64, overshoot: f64) -> f64 {
        let t = Easing::clamp01(t);
        let s = overshoot * 1.525;
        if t < 0.5 {
            let u = 2.0 * t;
            return (u * u * (((s + 1.0) * u) - s)) / 2.0;
        }
        let v = (2.0 * t) - 2.0;
        ((v * v * (((s + 1.0) * v) + s)) + 2.0) / 2.0
    }

    /// Elastic in.
    pub fn elastic_in(t: f64) -> f64 {
        let t = Easing::clamp01(t);
        if t == 0.0 || t == 1.0 {
            return t;
        }
        -2f64.powf((10.0 * t) - 10.0) * (((10.0 * t) - 10.75) * ELASTIC_PERIOD_IN).sin()
    }

    /// Elastic out.
    pub fn elastic_out(t: f64) -> f64 {
        let t = Easing::clamp01(t);
        if t == 0.0 || t == 1.0 {
            return t;
        }
        (2f64.powf(-10.0 * t) * (((10.0 * t) - 0.75) * ELASTIC_PERIOD_IN).sin()) + 1.0
    }

    /// Elastic in and out.
    pub fn elastic_in_out(t: f64) -> f64 {
        let t = Easing::clamp01(t);
        if t == 0.0 || t == 1.0 {
            return t;
        }
        let wave = (((20.0 * t) - 11.125) * ELASTIC_PERIOD_IN_OUT).sin();
        if t < 0.5 {
            -(2f64.powf((20.0 * t) - 10.0) * wave) / 2.0
        } else {
            ((2f64.powf((-20.0 * t) + 10.0) * wave) / 2.0) + 1.0
        }
    }

    /// Bounce in.
    pub fn bounce_in(t: f64) -> f64 {
        1.0 - Easing::bounce_out(1.0 - Easing::clamp01(t))
    }

    /// Bounce out.
    pub fn bounce_out(t: f64) -> f64 {
        const N: f64 = 7.5625;
        const D: f64 = 2.75;
        let mut t = Easing::clamp01(t);
        if t < 1.0 / D {
            return N * t * t;
        }
        if t < 2.0 / D {
            t -= 1.5 / D;
            return (N * t * t) + 0.75;
        }
        if t < 2.5 / D {
            t -= 2.25 / D;
            return (N * t * t) + 0.9375;
        }
        t -= 2.625 / D;
        (N * t * t) + 0.984375
    }

    /// Bounce in and out.
    pub fn bounce_in_out(t: f64) -> f64 {
        let t = Easing::clamp01(t);
        if t < 0.5 {
            (1.0 - Easing::bounce_out(1.0 - (2.0 * t))) / 2.0
        } else {
            (1.0 + Easing::bounce_out((2.0 * t) - 1.0)) / 2.0
        }
    }

    /// A staircase of `count` steps (the CSS `steps()` function; `position` is CSS's jump
    /// term). Returns None where C# throws ArgumentOutOfRangeException: a count below 1, or
    /// `StepPosition::None` with a count of 1.
    pub fn steps(t: f64, count: i32, position: StepPosition) -> Option<f64> {
        let jumps = match position {
            StepPosition::End | StepPosition::Start => count,
            StepPosition::None => count - 1,
            StepPosition::Both => count + 1,
        };
        if count < 1 || jumps < 1 {
            return None;
        }
        let t = Easing::clamp01(t);
        let mut step = (t * count as f64).floor() as i64;
        if position == StepPosition::Start || position == StepPosition::Both {
            step += 1;
        }
        let step = step.clamp(0, jumps as i64);
        Some(step as f64 / jumps as f64)
    }

    /// The CSS cubic-bezier curve through (x1, y1) and (x2, y2). None where C# throws (an x
    /// outside 0..1, or a value that is not finite).
    pub fn cubic_bezier(t: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> Option<f64> {
        CubicBezierEasing::new(x1, y1, x2, y2).map(|curve| curve.evaluate(t))
    }

    /// The named curve at `t`.
    pub fn evaluate(kind: EasingKind, t: f64) -> f64 {
        (Easing::get_function(kind))(t)
    }

    /// The named curve as a function (C# `GetFunction`, which returns a cached delegate).
    pub fn get_function(kind: EasingKind) -> fn(f64) -> f64 {
        match kind {
            EasingKind::Linear => Easing::linear,
            EasingKind::SmoothStep => Easing::smooth_step,
            EasingKind::SmootherStep => Easing::smoother_step,
            EasingKind::QuadIn => Easing::quad_in,
            EasingKind::QuadOut => Easing::quad_out,
            EasingKind::QuadInOut => Easing::quad_in_out,
            EasingKind::CubicIn => Easing::cubic_in,
            EasingKind::CubicOut => Easing::cubic_out,
            EasingKind::CubicInOut => Easing::cubic_in_out,
            EasingKind::QuartIn => Easing::quart_in,
            EasingKind::QuartOut => Easing::quart_out,
            EasingKind::QuartInOut => Easing::quart_in_out,
            EasingKind::QuintIn => Easing::quint_in,
            EasingKind::QuintOut => Easing::quint_out,
            EasingKind::QuintInOut => Easing::quint_in_out,
            EasingKind::SineIn => Easing::sine_in,
            EasingKind::SineOut => Easing::sine_out,
            EasingKind::SineInOut => Easing::sine_in_out,
            EasingKind::ExpoIn => Easing::expo_in,
            EasingKind::ExpoOut => Easing::expo_out,
            EasingKind::ExpoInOut => Easing::expo_in_out,
            EasingKind::CircIn => Easing::circ_in,
            EasingKind::CircOut => Easing::circ_out,
            EasingKind::CircInOut => Easing::circ_in_out,
            EasingKind::BackIn => Easing::back_in,
            EasingKind::BackOut => Easing::back_out,
            EasingKind::BackInOut => Easing::back_in_out,
            EasingKind::ElasticIn => Easing::elastic_in,
            EasingKind::ElasticOut => Easing::elastic_out,
            EasingKind::ElasticInOut => Easing::elastic_in_out,
            EasingKind::BounceIn => Easing::bounce_in,
            EasingKind::BounceOut => Easing::bounce_out,
            EasingKind::BounceInOut => Easing::bounce_in_out,
        }
    }
}

fn power_in(t: f64, power: i32) -> f64 {
    Easing::clamp01(t).powf(power as f64)
}

fn power_out(t: f64, power: i32) -> f64 {
    1.0 - (1.0 - Easing::clamp01(t)).powf(power as f64)
}

fn power_in_out(t: f64, power: i32) -> f64 {
    let t = Easing::clamp01(t);
    if t < 0.5 {
        2f64.powf((power - 1) as f64) * t.powf(power as f64)
    } else {
        1.0 - (((-2.0 * t) + 2.0).powf(power as f64) / 2.0)
    }
}
