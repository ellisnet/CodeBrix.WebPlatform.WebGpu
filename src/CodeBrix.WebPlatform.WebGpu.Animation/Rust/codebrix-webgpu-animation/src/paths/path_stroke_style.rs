//! `PathStrokeStyle`: width, caps, joins, miter limit and dashes of a stroke (C#
//! `PathStrokeStyle`; C#'s default arguments are the fields of [`PathStrokeStyleOptions`]).

use crate::paths::path_stroke_cap::PathStrokeCap;
use crate::paths::path_stroke_join::PathStrokeJoin;

/// The optional parts of a [`PathStrokeStyle`] (C#'s default arguments: round caps, round joins,
/// miter limit 4, no dashes, dash offset 0).
#[derive(Clone, Debug, PartialEq)]
pub struct PathStrokeStyleOptions {
    /// The caps.
    pub cap: PathStrokeCap,
    /// The joins.
    pub join: PathStrokeJoin,
    /// The miter limit (a ratio of the width).
    pub miter_limit: f32,
    /// Dash and gap lengths, alternating (None for a solid stroke).
    pub dash_pattern: Option<Vec<f32>>,
    /// How far into the pattern the stroke starts.
    pub dash_offset: f32,
}

impl Default for PathStrokeStyleOptions {
    fn default() -> PathStrokeStyleOptions {
        PathStrokeStyleOptions { cap: PathStrokeCap::Round, join: PathStrokeJoin::Round, miter_limit: 4.0, dash_pattern: None, dash_offset: 0.0 }
    }
}

/// How a path is stroked.
#[derive(Clone, Debug, PartialEq)]
pub struct PathStrokeStyle {
    width: f32,
    options: PathStrokeStyleOptions,
}

impl PathStrokeStyle {
    /// A stroke style. The error (C# ArgumentOutOfRangeException / ArgumentException) says what
    /// to do: a positive finite width; a dash pattern of at least one length, none negative, not
    /// all zero.
    pub fn new(width: f32, options: PathStrokeStyleOptions) -> Result<PathStrokeStyle, String> {
        if !width.is_finite() || width <= 0.0 {
            return Err("A stroke width must be positive and finite.".to_string());
        }
        if let Some(pattern) = &options.dash_pattern {
            if pattern.is_empty() {
                return Err("A dash pattern needs at least one length: pass None for a solid stroke.".to_string());
            }
            if pattern.iter().any(|v| !v.is_finite() || *v < 0.0) || pattern.iter().all(|v| *v == 0.0) {
                return Err("Dash lengths must be finite, not negative, and not all zero.".to_string());
            }
        }
        Ok(PathStrokeStyle { width, options })
    }

    /// The width.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// The caps.
    pub fn cap(&self) -> PathStrokeCap {
        self.options.cap
    }

    /// The joins.
    pub fn join(&self) -> PathStrokeJoin {
        self.options.join
    }

    /// The miter limit.
    pub fn miter_limit(&self) -> f32 {
        self.options.miter_limit
    }

    /// The dash offset.
    pub fn dash_offset(&self) -> f32 {
        self.options.dash_offset
    }

    /// True when the stroke is dashed.
    pub fn is_dashed(&self) -> bool {
        self.options.dash_pattern.is_some()
    }

    /// A copy of the dash pattern (empty for a solid stroke).
    pub fn get_dash_pattern(&self) -> Vec<f32> {
        self.options.dash_pattern.clone().unwrap_or_default()
    }

    /// The same style with another width.
    pub fn with_width(&self, width: f32) -> Result<PathStrokeStyle, String> {
        PathStrokeStyle::new(width, self.options.clone())
    }

    /// The same style with another dash offset (for marching dashes).
    pub fn with_dash_offset(&self, dash_offset: f32) -> PathStrokeStyle {
        PathStrokeStyle { width: self.width, options: PathStrokeStyleOptions { dash_offset, ..self.options.clone() } }
    }

    /// How far beyond half the width the stroke can reach (miter joins, square caps) - C#'s
    /// `ReachFactor`, for bounds.
    pub(crate) fn reach_factor(&self) -> f32 {
        let mut factor: f32 = 1.0;
        if self.options.join == PathStrokeJoin::Miter {
            factor = factor.max(self.options.miter_limit);
        }
        if self.options.cap == PathStrokeCap::Square {
            factor = factor.max(std::f32::consts::SQRT_2);
        }
        factor
    }

    /// kurbo's stroke for this style.
    pub(crate) fn to_kurbo(&self) -> kurbo::Stroke {
        let mut stroke = kurbo::Stroke::new(self.width as f64).with_caps(self.options.cap.to_kurbo()).with_join(self.options.join.to_kurbo()).with_miter_limit(self.options.miter_limit as f64);
        if let Some(pattern) = &self.options.dash_pattern {
            let mut pattern: Vec<f64> = pattern.iter().map(|v| *v as f64).collect();
            // An odd pattern repeats twice (SVG's rule), so dashes and gaps alternate.
            if pattern.len() % 2 == 1 {
                pattern.extend_from_within(..);
            }
            stroke = stroke.with_dashes(self.options.dash_offset as f64, pattern);
        }
        stroke
    }
}
