//! `ColorRamp`: a multi-stop colour ramp sampled at a position (C# `ColorRamp`): straight sRGB
//! interpolation of 8-bit channels, rounded half away from zero, exactly as the C# class does.

use crate::animation::easing::Easing;
use crate::color::Color;
use crate::fills::color_stop::ColorStop;

/// A colour ramp: two or more [`ColorStop`]s, sampled with [`ColorRamp::at`]. A
/// [`LinearGradientFill`](crate::fills::LinearGradientFill) made from the same stops
/// ([`ColorRamp::get_stops`]) interpolates in the same space (straight sRGB).
#[derive(Clone, Debug, PartialEq)]
pub struct ColorRamp {
    stops: Vec<ColorStop>,
}

impl ColorRamp {
    /// A ramp from stops. The error (C# ArgumentException) says what to do: at least two stops,
    /// positions non-decreasing.
    pub fn new(stops: &[ColorStop]) -> Result<ColorRamp, String> {
        Ok(ColorRamp { stops: ColorStop::validate_and_copy(stops)? })
    }

    /// A ramp of evenly spaced colours (C# `new ColorRamp(params Color[] colors)`).
    pub fn from_colors(colors: &[Color]) -> Result<ColorRamp, String> {
        Ok(ColorRamp { stops: ColorStop::evenly_spaced(colors)? })
    }

    /// The number of stops.
    pub fn stop_count(&self) -> usize {
        self.stops.len()
    }

    /// A copy of the stops.
    pub fn get_stops(&self) -> Vec<ColorStop> {
        self.stops.clone()
    }

    /// The colour at `t` (clamped to 0..1). Before the first stop: the first colour; at or after
    /// the last: the last; where two stops share a position, the later one wins.
    pub fn at(&self, t: f64) -> Color {
        let [r, g, b, a] = self.interpolate(t);
        Color::from_rgba(r, g, b, a)
    }

    /// The colour at `t` with its alpha replaced (C# `At(t, alpha)`).
    pub fn at_with_alpha(&self, t: f64, alpha: u8) -> Color {
        let [r, g, b, _] = self.interpolate(t);
        Color::from_rgba(r, g, b, alpha)
    }

    fn interpolate(&self, t: f64) -> [u8; 4] {
        let t = Easing::clamp01(t);
        let last = self.stops.len() - 1;
        if t < self.stops[0].position() as f64 {
            return self.stops[0].color().to_rgba32();
        }
        if t >= self.stops[last].position() as f64 {
            return self.stops[last].color().to_rgba32();
        }
        // The LAST stop at or before t starts the segment, so its successor lies strictly after
        // t - and a position shared by two stops resolves to the later one.
        let mut i = 0;
        while self.stops[i + 1].position() as f64 <= t {
            i += 1;
        }
        let start = self.stops[i].position() as f64;
        let end = self.stops[i + 1].position() as f64;
        let u = (t - start) / (end - start);
        let a = self.stops[i].color().to_rgba32();
        let b = self.stops[i + 1].color().to_rgba32();
        [channel(a[0], b[0], u), channel(a[1], b[1], u), channel(a[2], b[2], u), channel(a[3], b[3], u)]
    }
}

/// One 8-bit channel interpolated and rounded half away from zero (C# `ColorRamp.Channel`, which
/// ColorTrack shares).
pub(crate) fn channel(from: u8, to: u8, u: f64) -> u8 {
    let value = from as f64 + ((to as f64 - from as f64) * u);
    // f64::round rounds half away from zero, as C#'s MidpointRounding.AwayFromZero.
    value.round().clamp(0.0, 255.0) as u8
}
