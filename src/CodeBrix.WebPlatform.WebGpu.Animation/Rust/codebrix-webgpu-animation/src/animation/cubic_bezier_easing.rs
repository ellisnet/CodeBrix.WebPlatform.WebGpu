//! `CubicBezierEasing`: the CSS cubic-bezier() timing function (C# `CubicBezierEasing`, the same
//! Newton-then-bisection solver).

use crate::animation::easing::Easing;

const NEWTON_ITERATIONS: usize = 8;
const EPSILON: f64 = 1e-7;

/// The CSS `cubic-bezier(x1, y1, x2, y2)` timing function: the curve from (0, 0) to (1, 1) with
/// those two control points, evaluated as y for a given x (the progress).
///
/// A CURVE AS A CONSTANT: [`CubicBezierEasing::new`] is a `const fn`, so a curve of the
/// application's own is declared once, checked when the program is COMPILED (Not in C#, where the
/// constructor runs and throws at run time):
/// `const SNAP: CubicBezierEasing = CubicBezierEasing::new(0.2, 0.0, 0.0, 1.0).unwrap();`.
/// Control points outside the rule make `new` return None, and `.unwrap()` in a constant then
/// stops the build with an error. The fields stay private: x1 and x2 must lie in 0..1 (the curve's
/// x must rise for the solver to find one y for each progress), and private fields keep every
/// curve that exists inside that rule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezierEasing {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl CubicBezierEasing {
    /// CSS `ease`: cubic-bezier(0.25, 0.1, 0.25, 1).
    pub const EASE: CubicBezierEasing = CubicBezierEasing { x1: 0.25, y1: 0.1, x2: 0.25, y2: 1.0 };
    /// CSS `ease-in`: cubic-bezier(0.42, 0, 1, 1).
    pub const EASE_IN: CubicBezierEasing = CubicBezierEasing { x1: 0.42, y1: 0.0, x2: 1.0, y2: 1.0 };
    /// CSS `ease-out`: cubic-bezier(0, 0, 0.58, 1).
    pub const EASE_OUT: CubicBezierEasing = CubicBezierEasing { x1: 0.0, y1: 0.0, x2: 0.58, y2: 1.0 };
    /// CSS `ease-in-out`: cubic-bezier(0.42, 0, 0.58, 1).
    pub const EASE_IN_OUT: CubicBezierEasing = CubicBezierEasing { x1: 0.42, y1: 0.0, x2: 0.58, y2: 1.0 };

    /// The curve with these control points. None where C# throws ArgumentOutOfRangeException:
    /// x1 or x2 outside 0..1, or any value that is not finite. A `const fn` (Not in C#): usable
    /// in a `const` with `.unwrap()`, which turns a curve outside the rule into a build error.
    pub const fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Option<CubicBezierEasing> {
        // Written without RangeInclusive::contains, which is not a const fn. A NaN fails every
        // comparison, so it is refused by the range test as well as by is_finite.
        if !x1.is_finite() || !(x1 >= 0.0 && x1 <= 1.0) || !x2.is_finite() || !(x2 >= 0.0 && x2 <= 1.0) || !y1.is_finite() || !y2.is_finite() {
            return None;
        }
        Some(CubicBezierEasing { x1, y1, x2, y2 })
    }

    /// The first control point's x.
    pub const fn x1(&self) -> f64 {
        self.x1
    }

    /// The first control point's y.
    pub const fn y1(&self) -> f64 {
        self.y1
    }

    /// The second control point's x.
    pub const fn x2(&self) -> f64 {
        self.x2
    }

    /// The second control point's y.
    pub const fn y2(&self) -> f64 {
        self.y2
    }

    /// The eased value at progress `t` (clamped to 0..1).
    pub fn evaluate(&self, t: f64) -> f64 {
        let t = Easing::clamp01(t);
        if t == 0.0 || t == 1.0 {
            return t;
        }
        sample(self.solve_for_x(t), self.y1, self.y2)
    }

    fn solve_for_x(&self, x: f64) -> f64 {
        let mut s = x;
        for _ in 0..NEWTON_ITERATIONS {
            let error = sample(s, self.x1, self.x2) - x;
            if error.abs() < EPSILON {
                return s;
            }
            let slope = sample_derivative(s, self.x1, self.x2);
            if slope.abs() < 1e-6 {
                break;
            }
            s -= error / slope;
        }
        // Bisection: x(s) is monotonic on 0..1 because both control x values lie in 0..1.
        let mut low = 0.0;
        let mut high = 1.0;
        s = x;
        while high - low > EPSILON {
            let value = sample(s, self.x1, self.x2);
            if (value - x).abs() < EPSILON {
                return s;
            }
            if value < x {
                low = s;
            } else {
                high = s;
            }
            s = (low + high) / 2.0;
        }
        s
    }
}

impl std::fmt::Display for CubicBezierEasing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cubic-bezier({}, {}, {}, {})", self.x1, self.y1, self.x2, self.y2)
    }
}

// B(s) = 3(1-s)^2 s P1 + 3(1-s) s^2 P2 + s^3, written in polynomial form.
fn sample(s: f64, p1: f64, p2: f64) -> f64 {
    let c = 3.0 * p1;
    let b = (3.0 * (p2 - p1)) - c;
    let a = 1.0 - c - b;
    ((((a * s) + b) * s) + c) * s
}

fn sample_derivative(s: f64, p1: f64, p2: f64) -> f64 {
    let c = 3.0 * p1;
    let b = (3.0 * (p2 - p1)) - c;
    let a = 1.0 - c - b;
    (((3.0 * a * s) + (2.0 * b)) * s) + c
}
