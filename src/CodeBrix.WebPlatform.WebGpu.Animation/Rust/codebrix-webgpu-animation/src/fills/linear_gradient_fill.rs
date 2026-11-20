//! `LinearGradientFill`: a gradient along a line, in calibrated drawing units (C#
//! `LinearGradientFill`).

use crate::color::Color;
use crate::fills::color_stop::ColorStop;
use crate::fills::gradient_spread::GradientSpread;
use crate::geometry::PointF;

/// A gradient from `start` to `end` (calibrated drawing units) through two or more stops,
/// interpolated in straight sRGB as NoSkia does.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradientFill {
    start: PointF,
    end: PointF,
    stops: Vec<ColorStop>,
    spread: GradientSpread,
}

impl LinearGradientFill {
    /// A gradient with stops and a spread (C# `spread = GradientSpread.Pad` by default: pass
    /// `GradientSpread::default()`). The error (C# ArgumentException) says what to do.
    pub fn new(start: PointF, end: PointF, stops: &[ColorStop], spread: GradientSpread) -> Result<LinearGradientFill, String> {
        let stops = ColorStop::validate_and_copy(stops)?;
        if start == end {
            return Err("A linear gradient needs two different points: move the end away from the start.".to_string());
        }
        Ok(LinearGradientFill { start, end, stops, spread })
    }

    /// A two-colour gradient (C# `LinearGradientFill(start, end, startColor, endColor)`).
    pub fn from_colors(start: PointF, end: PointF, start_color: Color, end_color: Color) -> Result<LinearGradientFill, String> {
        LinearGradientFill::new(start, end, &[ColorStop::new(0.0, start_color).unwrap(), ColorStop::new(1.0, end_color).unwrap()], GradientSpread::Pad)
    }

    /// Where the gradient starts.
    pub fn start(&self) -> PointF {
        self.start
    }

    /// Where it ends.
    pub fn end(&self) -> PointF {
        self.end
    }

    /// How it continues outside its range.
    pub fn spread(&self) -> GradientSpread {
        self.spread
    }

    /// The number of stops.
    pub fn stop_count(&self) -> usize {
        self.stops.len()
    }

    /// A copy of the stops.
    pub fn get_stops(&self) -> Vec<ColorStop> {
        self.stops.clone()
    }

    pub(crate) fn stops(&self) -> &[ColorStop] {
        &self.stops
    }
}
