//! `NoiseShape`: a rectangle filled with noise DRAWN ON THE GPU (no C# type: C# fills a plane
//! with `DrawingNoise.Fill` on the CPU and draws the plane as a picture).

use crate::color::Color;
use crate::fills::ColorStop;
use crate::geometry::RectangleF;
use crate::planes::{DrawingNoiseFillOptions, DrawingNoiseKind};

/// The options of a [`NoiseShape`]: the planes' fill options (a z for evolving noise - often the
/// time times a speed, passed in - octaves, lacunarity, gain, offsets in calibrated units) and the
/// colour stops the noise value is mapped through.
#[derive(Clone, Debug, PartialEq)]
pub struct NoiseShapeOptions {
    /// The planes' fill options (C# `DrawingNoise.Fill`'s defaulted arguments).
    pub fill: DrawingNoiseFillOptions,
    /// The colours along the value, from -1 (position 0) to 1 (position 1); two stops at the same
    /// position make a hard threshold (a dissolve pattern). Empty: black to white.
    pub stops: Vec<ColorStop>,
}

impl Default for NoiseShapeOptions {
    fn default() -> NoiseShapeOptions {
        NoiseShapeOptions { fill: DrawingNoiseFillOptions::default(), stops: Vec::new() }
    }
}

/// A rectangle filled with seeded coherent noise COMPUTED ON THE GPU per pixel: grain, clouds,
/// dissolve patterns, textured fills, pool light. The planes' parameters - kind (value, gradient
/// or caustics), seed, frequency (per calibrated unit), z, octaves, lacunarity, gain, offset - and
/// the planes' rules (the pixel centre sampled, fractal octaves with the seed advanced per octave,
/// the sum divided by the amplitudes and clamped to -1..1) with THE SAME CHARACTER, but NOT the
/// same pattern as the CPU's for a seed: the GPU hashes the lattice with 32-bit integers and works
/// in 32-bit floats, where the CPU's [`DrawingNoise`](crate::planes::DrawingNoise) is NoSkia's
/// exact 64-bit arithmetic - except the caustics kind (no C# original), which is the same pattern
/// on both (within float rounding). Use the CPU noise where a value is READ (positions, jitter)
/// and this where noise is SEEN. The value maps through the colour stops (premultiplied colours interpolated
/// straight, as the gradient fills). Masks and effects apply as to any shape; bind its z to the
/// time through an [`AnimatedShape`](crate::shapes::AnimatedShape) for noise that evolves.
#[derive(Clone, Debug, PartialEq)]
pub struct NoiseShape {
    pub(crate) area: RectangleF,
    pub(crate) kind: DrawingNoiseKind,
    pub(crate) seed: i64,
    pub(crate) frequency: f32,
    pub(crate) options: NoiseShapeOptions,
}

impl NoiseShape {
    /// A noise rectangle at (x, y) of width x height calibrated units. The error says what is
    /// wrong: an empty rectangle, a frequency that is not a finite positive number, octaves
    /// outside 1 to 30.
    pub fn new(x: f32, y: f32, width: f32, height: f32, kind: DrawingNoiseKind, seed: i64, frequency: f32, options: NoiseShapeOptions) -> Result<NoiseShape, String> {
        if !(width > 0.0) || !(height > 0.0) || !x.is_finite() || !y.is_finite() || !width.is_finite() || !height.is_finite() {
            return Err("A noise rectangle needs a finite position and a width and height above 0.".to_string());
        }
        if !(frequency > 0.0) || !frequency.is_finite() {
            return Err(format!("A noise frequency must be a finite number above 0 (it was {frequency})."));
        }
        if !(1..=30).contains(&options.fill.octaves) {
            return Err(format!("A fractal noise needs 1 to 30 octaves (asked for {}).", options.fill.octaves));
        }
        Ok(NoiseShape { area: RectangleF::new(x, y, width, height), kind, seed, frequency, options })
    }

    /// A copy at another z (the third coordinate: evolving noise).
    pub fn with_z(mut self, z: f64) -> NoiseShape {
        self.options.fill.z = Some(z);
        self
    }

    /// The rectangle, calibrated units.
    pub fn area(&self) -> RectangleF {
        self.area
    }

    /// The kind of noise.
    pub fn kind(&self) -> DrawingNoiseKind {
        self.kind
    }

    /// The seed.
    pub fn seed(&self) -> i64 {
        self.seed
    }

    /// The frequency, per calibrated unit.
    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    /// The options.
    pub fn options(&self) -> &NoiseShapeOptions {
        &self.options
    }

    /// The bounds (the rectangle).
    pub fn get_bounds(&self) -> RectangleF {
        self.area
    }

    /// The peniko gradient of the stops (the renderer's ramp), at `opacity`.
    pub(crate) fn ramp(&self, opacity: f32) -> peniko::Gradient {
        let mut list = peniko::ColorStops::new();
        if self.options.stops.is_empty() {
            list.0.push(peniko::ColorStop::from((0.0, Color::BLACK.to_peniko(opacity))));
            list.0.push(peniko::ColorStop::from((1.0, Color::WHITE.to_peniko(opacity))));
        } else {
            for stop in &self.options.stops {
                list.0.push(peniko::ColorStop::from((stop.position(), stop.color().to_peniko(opacity))));
            }
        }
        peniko::Gradient { stops: list, interpolation_alpha_space: peniko::InterpolationAlphaSpace::Unpremultiplied, ..peniko::Gradient::new_linear((0.0, 0.0), (1.0, 0.0)) }
    }

    /// The parameters the GPU's noise paint takes.
    pub(crate) fn gpu_params(&self) -> NoisePaintParams {
        let f = &self.options.fill;
        // The 64-bit seed folded to the 32 bits the GPU hashes with. The caustics kind takes the
        // low 32 bits instead, as its CPU twin does, so each octave's `seed + o` is the CPU's too.
        let seed = match self.kind {
            DrawingNoiseKind::Caustics => self.seed as u32,
            _ => (self.seed as u64 ^ ((self.seed as u64) >> 32)) as u32,
        };
        NoisePaintParams { gradient: self.kind == DrawingNoiseKind::Gradient, caustics: self.kind == DrawingNoiseKind::Caustics, seed, frequency: self.frequency, z: f.z.map(|z| z as f32), octaves: f.octaves, lacunarity: f.lacunarity as f32, gain: f.gain as f32, offset: [f.offset_x as f32, f.offset_y as f32] }
    }
}

/// What the GPU's noise paint needs (crate-internal).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NoisePaintParams {
    pub(crate) gradient: bool,
    /// The caustics kind (mode bit 4).
    pub(crate) caustics: bool,
    pub(crate) seed: u32,
    pub(crate) frequency: f32,
    pub(crate) z: Option<f32>,
    pub(crate) octaves: u32,
    pub(crate) lacunarity: f32,
    pub(crate) gain: f32,
    pub(crate) offset: [f32; 2],
}
