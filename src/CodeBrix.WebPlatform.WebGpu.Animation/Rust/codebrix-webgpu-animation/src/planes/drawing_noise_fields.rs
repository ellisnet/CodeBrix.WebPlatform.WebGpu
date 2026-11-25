//! The rest of C#'s `DrawingNoise`: value noise, 3D noise (z is usually a time passed in),
//! any kind by [`DrawingNoiseKind`], fractal sums and plane fills - beside the 2D gradient
//! noise the formations brought ([`DrawingNoise::gradient`]). The caustics kind (not in C#)
//! lives in drawing_caustics.rs and takes part here through the two `sample` arms.

use crate::planes::drawing_noise::{clamp_unit, fade, hash, lerp};
use crate::planes::drawing_random::mix;
use crate::planes::{DrawingNoise, DrawingNoiseFillOptions, DrawingNoiseFractalOptions, DrawingNoiseKind, DrawingPlane};

const SCALE_3D: f64 = 0.816_496_580_927_726;
const GRADIENTS_3D: [f64; 36] = [
    1.0, 1.0, 0.0, -1.0, 1.0, 0.0, 1.0, -1.0, 0.0, -1.0, -1.0, 0.0, //
    1.0, 0.0, 1.0, -1.0, 0.0, 1.0, 1.0, 0.0, -1.0, -1.0, 0.0, -1.0, //
    0.0, 1.0, 1.0, 0.0, -1.0, 1.0, 0.0, 1.0, -1.0, 0.0, -1.0, -1.0,
];

impl DrawingNoise {
    /// 2D value noise at a point, in -1..1 (C# `Value(x, y, seed)`): the lattice values
    /// `(h >> 11) * 2^-53 * 2 - 1` blended with the quintic fade, x first, then y.
    pub fn value(x: f64, y: f64, seed: i64) -> f64 {
        let (ix, iy) = (x.floor() as i64, y.floor() as i64);
        let (fx, fy) = (x - ix as f64, y - iy as f64);
        let (u, v) = (fade(fx), fade(fy));
        let a = lerp(lattice_value(seed, ix, iy), lattice_value(seed, ix.wrapping_add(1), iy), u);
        let b = lerp(lattice_value(seed, ix, iy.wrapping_add(1)), lattice_value(seed, ix.wrapping_add(1), iy.wrapping_add(1)), u);
        clamp_unit(lerp(a, b, v))
    }

    /// 3D value noise at a point - use z as a time (times a speed) for 2D noise that changes
    /// smoothly (C# `Value(x, y, z, seed)`).
    pub fn value_3d(x: f64, y: f64, z: f64, seed: i64) -> f64 {
        let (ix, iy, iz) = (x.floor() as i64, y.floor() as i64, z.floor() as i64);
        let (u, v, w) = (fade(x - ix as f64), fade(y - iy as f64), fade(z - iz as f64));
        let (ix1, iy1, iz1) = (ix.wrapping_add(1), iy.wrapping_add(1), iz.wrapping_add(1));
        let c00 = lerp(lattice_value_3d(seed, ix, iy, iz), lattice_value_3d(seed, ix1, iy, iz), u);
        let c10 = lerp(lattice_value_3d(seed, ix, iy1, iz), lattice_value_3d(seed, ix1, iy1, iz), u);
        let c01 = lerp(lattice_value_3d(seed, ix, iy, iz1), lattice_value_3d(seed, ix1, iy, iz1), u);
        let c11 = lerp(lattice_value_3d(seed, ix, iy1, iz1), lattice_value_3d(seed, ix1, iy1, iz1), u);
        clamp_unit(lerp(lerp(c00, c10, v), lerp(c01, c11, v), w))
    }

    /// 3D gradient (improved Perlin-style) noise at a point, in -1..1 (C# `Gradient(x, y, z,
    /// seed)`): one of the 12 cube-edge directions per lattice point, scaled by 1 / sqrt 1.5.
    pub fn gradient_3d(x: f64, y: f64, z: f64, seed: i64) -> f64 {
        let (ix, iy, iz) = (x.floor() as i64, y.floor() as i64, z.floor() as i64);
        let (fx, fy, fz) = (x - ix as f64, y - iy as f64, z - iz as f64);
        let (u, v, w) = (fade(fx), fade(fy), fade(fz));
        let (ix1, iy1, iz1) = (ix.wrapping_add(1), iy.wrapping_add(1), iz.wrapping_add(1));
        let c00 = lerp(dot3(seed, ix, iy, iz, fx, fy, fz), dot3(seed, ix1, iy, iz, fx - 1.0, fy, fz), u);
        let c10 = lerp(dot3(seed, ix, iy1, iz, fx, fy - 1.0, fz), dot3(seed, ix1, iy1, iz, fx - 1.0, fy - 1.0, fz), u);
        let c01 = lerp(dot3(seed, ix, iy, iz1, fx, fy, fz - 1.0), dot3(seed, ix1, iy, iz1, fx - 1.0, fy, fz - 1.0), u);
        let c11 = lerp(dot3(seed, ix, iy1, iz1, fx, fy - 1.0, fz - 1.0), dot3(seed, ix1, iy1, iz1, fx - 1.0, fy - 1.0, fz - 1.0), u);
        clamp_unit(lerp(lerp(c00, c10, v), lerp(c01, c11, v), w) * SCALE_3D)
    }

    /// 2D noise of any kind (C# `Sample(kind, x, y, seed)`; the caustics kind - not in C# - at z
    /// = 0).
    pub fn sample(kind: DrawingNoiseKind, x: f64, y: f64, seed: i64) -> f64 {
        match kind {
            DrawingNoiseKind::Value => DrawingNoise::value(x, y, seed),
            DrawingNoiseKind::Gradient => DrawingNoise::gradient(x, y, seed),
            DrawingNoiseKind::Caustics => DrawingNoise::caustics(x, y, 0.0, seed),
        }
    }

    /// 3D noise of any kind (C# `Sample(kind, x, y, z, seed)`; for the caustics kind - not in C#
    /// - z is the time-like coordinate the net drifts with).
    pub fn sample_3d(kind: DrawingNoiseKind, x: f64, y: f64, z: f64, seed: i64) -> f64 {
        match kind {
            DrawingNoiseKind::Value => DrawingNoise::value_3d(x, y, z, seed),
            DrawingNoiseKind::Gradient => DrawingNoise::gradient_3d(x, y, z, seed),
            DrawingNoiseKind::Caustics => DrawingNoise::caustics(x, y, z, seed),
        }
    }

    /// 2D fractal noise (fBm) (C# `Fractal(kind, x, y, seed, octaves = 4, lacunarity = 2, gain =
    /// 0.5)`): octave o samples at `(x, y) * lacunarity^o` with seed `seed + o` and amplitude
    /// `gain^o`; the amplitudes' sum divides the total. One octave equals [`DrawingNoise::sample`].
    /// The error says the octaves must be 1 to 30.
    pub fn fractal(kind: DrawingNoiseKind, x: f64, y: f64, seed: i64, options: DrawingNoiseFractalOptions) -> Result<f64, String> {
        validate_octaves(options.octaves)?;
        Ok(fractal_2d(kind, x, y, seed, options))
    }

    /// 3D fractal noise (fBm) with the octave rule of [`DrawingNoise::fractal`]; all three
    /// coordinates are multiplied by the octave frequency (C# `Fractal(kind, x, y, z, seed, ...)`).
    pub fn fractal_3d(kind: DrawingNoiseKind, x: f64, y: f64, z: f64, seed: i64, options: DrawingNoiseFractalOptions) -> Result<f64, String> {
        validate_octaves(options.octaves)?;
        Ok(fractal_3d(kind, x, y, z, seed, options))
    }

    /// Fills a plane with (fractal) noise (C# `Fill(plane, kind, seed, frequency, z = null,
    /// octaves = 1, ...)`): pixel (x, y) samples `((x + 0.5 + offset_x) * frequency, (y + 0.5 +
    /// offset_y) * frequency)` - the pixel centre - and, with a `z`, 3D noise at that z as is.
    /// Values are in -1..1 (map them with `multiply` and `add`, e.g. * 0.5 + 0.5 for 0..1). One
    /// thread, row by row; each value is a pure function of its position, as in C#.
    pub fn fill(plane: &mut DrawingPlane, kind: DrawingNoiseKind, seed: i64, frequency: f64, options: DrawingNoiseFillOptions) -> Result<&mut DrawingPlane, String> {
        validate_octaves(options.octaves)?;
        let fractal = DrawingNoiseFractalOptions { octaves: options.octaves, lacunarity: options.lacunarity, gain: options.gain };
        let width = plane.width() as usize;
        let height = plane.height() as usize;
        let data = plane.data_mut();
        for y in 0..height {
            let py = (y as f64 + 0.5 + options.offset_y) * frequency;
            let row = y * width;
            for x in 0..width {
                let px = (x as f64 + 0.5 + options.offset_x) * frequency;
                data[row + x] = match options.z {
                    Some(z) => fractal_3d(kind, px, py, z, seed, fractal),
                    None => fractal_2d(kind, px, py, seed, fractal),
                } as f32;
            }
        }
        Ok(plane)
    }
}

fn validate_octaves(octaves: u32) -> Result<(), String> {
    if !(1..=30).contains(&octaves) {
        return Err(format!("A fractal noise needs 1 to 30 octaves (asked for {octaves})."));
    }
    Ok(())
}

fn fractal_2d(kind: DrawingNoiseKind, x: f64, y: f64, seed: i64, options: DrawingNoiseFractalOptions) -> f64 {
    let (mut total, mut norm, mut amplitude, mut frequency) = (0.0f64, 0.0f64, 1.0f64, 1.0f64);
    for o in 0..options.octaves {
        total += amplitude * DrawingNoise::sample(kind, x * frequency, y * frequency, seed.wrapping_add(o as i64));
        norm += amplitude;
        amplitude *= options.gain;
        frequency *= options.lacunarity;
    }
    if norm == 0.0 {
        0.0
    } else {
        clamp_unit(total / norm)
    }
}

fn fractal_3d(kind: DrawingNoiseKind, x: f64, y: f64, z: f64, seed: i64, options: DrawingNoiseFractalOptions) -> f64 {
    let (mut total, mut norm, mut amplitude, mut frequency) = (0.0f64, 0.0f64, 1.0f64, 1.0f64);
    for o in 0..options.octaves {
        total += amplitude * DrawingNoise::sample_3d(kind, x * frequency, y * frequency, z * frequency, seed.wrapping_add(o as i64));
        norm += amplitude;
        amplitude *= options.gain;
        frequency *= options.lacunarity;
    }
    if norm == 0.0 {
        0.0
    } else {
        clamp_unit(total / norm)
    }
}

fn hash_3d(seed: i64, ix: i64, iy: i64, iz: i64) -> u64 {
    mix(hash(seed, ix, iy).wrapping_add(iz as u64))
}

fn to_signed(h: u64) -> f64 {
    ((h >> 11) as f64 * (1.0 / 9007199254740992.0) * 2.0) - 1.0
}

fn lattice_value(seed: i64, ix: i64, iy: i64) -> f64 {
    to_signed(hash(seed, ix, iy))
}

fn lattice_value_3d(seed: i64, ix: i64, iy: i64, iz: i64) -> f64 {
    to_signed(hash_3d(seed, ix, iy, iz))
}

#[allow(clippy::too_many_arguments)]
fn dot3(seed: i64, ix: i64, iy: i64, iz: i64, dx: f64, dy: f64, dz: f64) -> f64 {
    let g = ((((hash_3d(seed, ix, iy, iz) >> 32).wrapping_mul(12)) >> 32) * 3) as usize;
    (GRADIENTS_3D[g] * dx) + (GRADIENTS_3D[g + 1] * dy) + (GRADIENTS_3D[g + 2] * dz)
}
