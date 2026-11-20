//! `RadialGradientFill`: a gradient out from a centre, in calibrated drawing units (C#
//! `RadialGradientFill`).

use crate::color::Color;
use crate::fills::color_stop::ColorStop;
use crate::fills::gradient_spread::GradientSpread;
use crate::geometry::PointF;

/// A gradient from `center` (position 0) out to `radius` (position 1), calibrated drawing units.
#[derive(Clone, Debug, PartialEq)]
pub struct RadialGradientFill {
    center: PointF,
    radius: f32,
    stops: Vec<ColorStop>,
    spread: GradientSpread,
}

impl RadialGradientFill {
    /// A gradient with stops and a spread. The error (C# ArgumentException /
    /// ArgumentOutOfRangeException) says what to do.
    pub fn new(center: PointF, radius: f32, stops: &[ColorStop], spread: GradientSpread) -> Result<RadialGradientFill, String> {
        let stops = ColorStop::validate_and_copy(stops)?;
        if !(radius > 0.0) || radius.is_infinite() {
            return Err("A radial gradient's radius must be positive and finite.".to_string());
        }
        Ok(RadialGradientFill { center, radius, stops, spread })
    }

    /// A two-colour gradient (C# `RadialGradientFill(center, radius, centerColor, edgeColor)`).
    pub fn from_colors(center: PointF, radius: f32, center_color: Color, edge_color: Color) -> Result<RadialGradientFill, String> {
        RadialGradientFill::new(center, radius, &[ColorStop::new(0.0, center_color).unwrap(), ColorStop::new(1.0, edge_color).unwrap()], GradientSpread::Pad)
    }

    /// The centre.
    pub fn center(&self) -> PointF {
        self.center
    }

    /// The radius.
    pub fn radius(&self) -> f32 {
        self.radius
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
