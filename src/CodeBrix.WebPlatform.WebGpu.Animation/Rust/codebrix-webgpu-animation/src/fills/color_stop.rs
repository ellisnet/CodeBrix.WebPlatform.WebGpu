//! `ColorStop`: a colour at a position along a gradient or a colour ramp (C# `ColorStop`).

use crate::color::Color;

/// A colour at a position (0..1) along a gradient or a [`ColorRamp`](crate::animation::ColorRamp).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorStop {
    position: f32,
    color: Color,
}

impl ColorStop {
    /// A stop. None where C# throws ArgumentOutOfRangeException: a position outside 0..1 or NaN.
    pub fn new(position: f32, color: Color) -> Option<ColorStop> {
        if position.is_nan() || !(0.0..=1.0).contains(&position) {
            return None;
        }
        Some(ColorStop { position, color })
    }

    /// The position, 0..1.
    pub fn position(&self) -> f32 {
        self.position
    }

    /// The colour.
    pub fn color(&self) -> Color {
        self.color
    }

    /// Validates and copies a stop list: at least two stops, positions non-decreasing. The error
    /// says what to do.
    pub(crate) fn validate_and_copy(stops: &[ColorStop]) -> Result<Vec<ColorStop>, String> {
        if stops.len() < 2 {
            return Err("At least two color stops are required: pass two or more ColorStop values.".to_string());
        }
        for pair in stops.windows(2) {
            if pair[1].position < pair[0].position {
                return Err("Color stop positions must be in non-decreasing order: sort the stops by position.".to_string());
            }
        }
        Ok(stops.to_vec())
    }

    /// Evenly spaced stops for a plain list of colours (the last at exactly 1).
    pub(crate) fn evenly_spaced(colors: &[Color]) -> Result<Vec<ColorStop>, String> {
        if colors.len() < 2 {
            return Err("At least two colors are required: pass two or more colors.".to_string());
        }
        let last = colors.len() - 1;
        Ok(colors
            .iter()
            .enumerate()
            .map(|(i, color)| ColorStop { position: if i == last { 1.0 } else { i as f32 / last as f32 }, color: *color })
            .collect())
    }
}
