//! `RadialGradientFill`: a gradient out from a centre, in calibrated drawing units (C#
//! `RadialGradientFill`).

use crate::color::Color;
use crate::fills::color_stop::ColorStop;
use crate::fills::gradient_spread::GradientSpread;
use crate::geometry::PointF;
use crate::painting::ColorInterpolation;

/// A gradient from `center` (position 0) out to `radius` (position 1), calibrated drawing units,
/// interpolated in straight sRGB as NoSkia does (another space when chosen with
/// [`RadialGradientFill::with_interpolation`]).
#[derive(Clone, Debug, PartialEq)]
pub struct RadialGradientFill {
    center: PointF,
    radius: f32,
    stops: Vec<ColorStop>,
    spread: GradientSpread,
    interpolation: ColorInterpolation,
}

impl RadialGradientFill {
    /// A gradient with stops and a spread. The error (C# ArgumentException /
    /// ArgumentOutOfRangeException) says what to do.
    pub fn new(center: PointF, radius: f32, stops: &[ColorStop], spread: GradientSpread) -> Result<RadialGradientFill, String> {
        let stops = ColorStop::validate_and_copy(stops)?;
        if !(radius > 0.0) || radius.is_infinite() {
            return Err("A radial gradient's radius must be positive and finite.".to_string());
        }
        Ok(RadialGradientFill { center, radius, stops, spread, interpolation: ColorInterpolation::Srgb })
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

    /// The same gradient interpolating in another space (no C# equivalent; Srgb, the default, is
    /// NoSkia's straight sRGB). The renderer samples the chosen space into the gradient's ramp.
    pub fn with_interpolation(mut self, space: ColorInterpolation) -> RadialGradientFill {
        self.interpolation = space;
        self
    }

    /// Sets the space the gradient interpolates in (no C# equivalent; see `with_interpolation`).
    pub fn set_interpolation(&mut self, space: ColorInterpolation) {
        self.interpolation = space;
    }

    /// The space the gradient interpolates in (Srgb unless chosen).
    pub fn interpolation(&self) -> ColorInterpolation {
        self.interpolation
    }

    pub(crate) fn stops(&self) -> &[ColorStop] {
        &self.stops
    }
}
