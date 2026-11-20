//! `ArrowShape`: a line with a two-stroke head at its end (C# `ArrowShape`).

use crate::color::Color;
use crate::geometry::{PointF, RectangleF};
use crate::models::DEFAULT_STROKE_WIDTH;
use crate::shapes::basic_shapes::check_thickness;

/// The head's strokes leave the tip 30 degrees off the shaft.
const HEAD_ANGLE: f64 = std::f64::consts::PI / 6.0;

/// The optional arguments of C#'s `ArrowShape` constructor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowShapeOptions {
    /// The line width (default 15, C#'s `Stroke.DefaultWidth`).
    pub stroke_thickness: f32,
    /// The colour; None takes the layer's.
    pub color: Option<Color>,
    /// The length of the head's strokes; None for C#'s default, max(30, 3 x thickness).
    pub head_length: Option<f32>,
}

impl Default for ArrowShapeOptions {
    fn default() -> ArrowShapeOptions {
        ArrowShapeOptions { stroke_thickness: DEFAULT_STROKE_WIDTH, color: None, head_length: None }
    }
}

/// An arrow from (x1, y1) to its tip at (x2, y2), round-capped (C# `ArrowShape`).
#[derive(Clone, Debug, PartialEq)]
pub struct ArrowShape {
    pub(crate) x1: f32,
    pub(crate) y1: f32,
    pub(crate) x2: f32,
    pub(crate) y2: f32,
    pub(crate) stroke_thickness: f32,
    pub(crate) color: Option<Color>,
    pub(crate) head_length: f32,
}

impl ArrowShape {
    /// An arrow (C# `ArrowShape(x1, y1, x2, y2, strokeThickness, color, headLength)`). The error
    /// says what to do with a thickness or head length that is not positive.
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32, options: ArrowShapeOptions) -> Result<ArrowShape, String> {
        check_thickness(options.stroke_thickness)?;
        if options.head_length.is_some_and(|h| !(h > 0.0)) {
            return Err("An arrow's head length must be positive (or None for the default).".to_string());
        }
        let head_length = options.head_length.unwrap_or((options.stroke_thickness * 3.0).max(30.0));
        Ok(ArrowShape { x1, y1, x2, y2, stroke_thickness: options.stroke_thickness, color: options.color, head_length })
    }

    /// The two ends of the head's strokes (the tip is (x2, y2)).
    pub(crate) fn head_points(&self) -> (PointF, PointF) {
        let shaft = ((self.y1 - self.y2) as f64).atan2((self.x1 - self.x2) as f64);
        let at = |angle: f64| PointF::new(self.x2 + (self.head_length as f64 * angle.cos()) as f32, self.y2 + (self.head_length as f64 * angle.sin()) as f32);
        (at(shaft + HEAD_ANGLE), at(shaft - HEAD_ANGLE))
    }

    /// The bounds of the shaft and head, grown by half the thickness (C# `GetBounds`).
    pub fn get_bounds(&self) -> RectangleF {
        let (a, b) = self.head_points();
        let xs = [self.x1, self.x2, a.x, b.x];
        let ys = [self.y1, self.y2, a.y, b.y];
        let half = self.stroke_thickness / 2.0;
        let (l, r) = (xs.iter().cloned().fold(f32::INFINITY, f32::min), xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max));
        let (t, bt) = (ys.iter().cloned().fold(f32::INFINITY, f32::min), ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max));
        RectangleF::new(l - half, t - half, (r - l) + 2.0 * half, (bt - t) + 2.0 * half)
    }
}
