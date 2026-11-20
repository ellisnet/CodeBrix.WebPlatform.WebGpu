//! `DrawingNoiseFractalOptions` and `DrawingNoiseFillOptions`: the defaulted arguments of C#'s
//! `DrawingNoise.Fractal` and `DrawingNoise.Fill`.

/// The defaulted arguments of [`DrawingNoise::fractal`](crate::planes::DrawingNoise::fractal)
/// and [`DrawingNoise::fractal_3d`](crate::planes::DrawingNoise::fractal_3d) (C# `octaves = 4,
/// lacunarity = 2, gain = 0.5`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawingNoiseFractalOptions {
    /// The number of octaves, 1 to 30 (4 by default).
    pub octaves: u32,
    /// The frequency multiplier from one octave to the next (2 by default).
    pub lacunarity: f64,
    /// The amplitude multiplier from one octave to the next (0.5 by default).
    pub gain: f64,
}

impl Default for DrawingNoiseFractalOptions {
    fn default() -> DrawingNoiseFractalOptions {
        DrawingNoiseFractalOptions { octaves: 4, lacunarity: 2.0, gain: 0.5 }
    }
}

/// The defaulted arguments of [`DrawingNoise::fill`](crate::planes::DrawingNoise::fill) (C# `z =
/// null, octaves = 1, lacunarity = 2, gain = 0.5, offsetX = 0, offsetY = 0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawingNoiseFillOptions {
    /// The third coordinate for evolving noise (often a time times a speed - passed in, never
    /// read from a clock), or None for 2D noise (the default).
    pub z: Option<f64>,
    /// The number of fractal octaves, 1 to 30 (1 = plain noise, the default).
    pub octaves: u32,
    /// The frequency multiplier per octave (2 by default).
    pub lacunarity: f64,
    /// The amplitude multiplier per octave (0.5 by default).
    pub gain: f64,
    /// Shifts the pattern, pixels (scrolling; 0 by default).
    pub offset_x: f64,
    /// Shifts the pattern vertically, pixels (0 by default).
    pub offset_y: f64,
}

impl Default for DrawingNoiseFillOptions {
    fn default() -> DrawingNoiseFillOptions {
        DrawingNoiseFillOptions { z: None, octaves: 1, lacunarity: 2.0, gain: 0.5, offset_x: 0.0, offset_y: 0.0 }
    }
}
