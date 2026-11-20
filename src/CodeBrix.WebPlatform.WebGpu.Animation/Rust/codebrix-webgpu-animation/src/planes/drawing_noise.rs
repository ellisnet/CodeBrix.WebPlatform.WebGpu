//! `DrawingNoise`: seeded lattice noise (C# `DrawingNoise`) - the 2D gradient noise here; the
//! value and 3D kinds, fractal sums and plane fills are in drawing_noise_fields.rs.

use crate::planes::drawing_random::mix;

const DIAGONAL: f64 = std::f64::consts::FRAC_1_SQRT_2;
const SCALE_2D: f64 = std::f64::consts::SQRT_2;
const GRADIENTS_2D: [f64; 16] = [1.0, 0.0, DIAGONAL, DIAGONAL, 0.0, 1.0, -DIAGONAL, DIAGONAL, -1.0, 0.0, -DIAGONAL, -DIAGONAL, 0.0, -1.0, DIAGONAL, -DIAGONAL];

/// Seeded coherent noise in 2D and 3D (C# `DrawingNoise`) - z is usually a time, PASSED IN, for
/// noise that evolves smoothly from frame to frame: value and gradient noise, fractal sums, plane
/// fills. A 64-bit hash of the seed and the lattice corner picks each corner's value or
/// gradient; quintic fade; the same arithmetic as C#, in the same order.
pub struct DrawingNoise;

impl DrawingNoise {
    /// 2D gradient (Perlin-style) noise at a point, in -1..1 (C# `Gradient(x, y, seed)`).
    pub fn gradient(x: f64, y: f64, seed: i64) -> f64 {
        let (ix, iy) = (x.floor() as i64, y.floor() as i64);
        let (fx, fy) = (x - ix as f64, y - iy as f64);
        let (u, v) = (fade(fx), fade(fy));
        let a = lerp(dot2(seed, ix, iy, fx, fy), dot2(seed, ix.wrapping_add(1), iy, fx - 1.0, fy), u);
        let b = lerp(dot2(seed, ix, iy.wrapping_add(1), fx, fy - 1.0), dot2(seed, ix.wrapping_add(1), iy.wrapping_add(1), fx - 1.0, fy - 1.0), u);
        clamp_unit(lerp(a, b, v) * SCALE_2D)
    }
}

pub(super) fn fade(t: f64) -> f64 {
    t * t * t * ((t * ((t * 6.0) - 15.0)) + 10.0)
}

pub(super) fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (t * (b - a))
}

pub(super) fn clamp_unit(value: f64) -> f64 {
    value.clamp(-1.0, 1.0)
}

pub(super) fn hash(seed: i64, ix: i64, iy: i64) -> u64 {
    mix(mix(mix(seed as u64).wrapping_add(ix as u64)).wrapping_add(iy as u64))
}

fn dot2(seed: i64, ix: i64, iy: i64, dx: f64, dy: f64) -> f64 {
    let g = (hash(seed, ix, iy) >> 61) as usize * 2;
    (GRADIENTS_2D[g] * dx) + (GRADIENTS_2D[g + 1] * dy)
}
