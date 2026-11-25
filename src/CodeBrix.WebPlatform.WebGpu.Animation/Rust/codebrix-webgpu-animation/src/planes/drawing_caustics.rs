//! The CAUSTICS pattern (no C# equivalent): the light net of a pool floor, on the CPU with the
//! GPU's own arithmetic - the 32-bit `noise_hash` and single-precision floats - so a plane filled
//! with it and a noise shape drawn with it on the GPU show the same net (within float rounding).
//! Unlike the value and gradient kinds, which follow NoSkia's 64-bit pattern on the CPU and only
//! its character on the GPU, this kind has no C# original to stay bit-exact with.
//!
//! THE PATTERN at `(x, y)` and time-like `z` (`noise.wgsl` `caustics_once` is the same code):
//! 1. Warp, two sines: `wx = x + 0.3 sin(0.73 y + 0.61 z)`, `wy = y + 0.3 sin(0.67 x - 0.53 z)`.
//! 2. The cell `(floor wx, floor wy)` and its 8 neighbours each hold one point:
//!    `h = noise_hash(seed, cell x, cell y, 0)`; its resting place in the cell is
//!    `0.35 + 0.3 (h & 255) / 255` across and `0.35 + 0.3 ((h >> 8) & 255) / 255` down; it circles
//!    that place at radius 0.15, starting at angle `((h >> 16) & 1023) / 1024 x 2 pi`, turning
//!    `0.5 + ((h >> 26) & 31) / 31` radians per unit of z, clockwise when bit 31 is set. (Every
//!    point stays 0.2 to 0.8 inside its cell, so the 3 x 3 cells always hold the two nearest
//!    points: the pattern is seamless.)
//! 3. `F1`, `F2`: the distances to the nearest and second-nearest point; the edge measure `e =
//!    F2 - F1` is 0 along the cells' borders.
//! 4. The line `l = (1 - smoothstep(0, 0.22, e))^2`; the value `2 l - 1`: -1 the dark floor
//!    between the lines, 1 on the brightest line.

use crate::painting::color_rules::noise_hash;
use crate::planes::DrawingNoise;

/// The warp's amplitude, in cells.
pub(crate) const WARP_AMOUNT: f32 = 0.3;
/// The radius each cell's point circles at, in cells.
pub(crate) const ORBIT_RADIUS: f32 = 0.15;
/// The width of the bright line, in units of `F2 - F1`.
pub(crate) const LINE_WIDTH: f32 = 0.22;

impl DrawingNoise {
    /// The CAUSTICS pattern at a point, in -1..1 (no C# equivalent): the bright, curved light
    /// net of a pool floor - lines along the edges of a lattice of cells (one cell per unit of x
    /// and y) whose points circle slowly as `z` grows, the space warped by two sines so the
    /// lines curve. `z` is a time (times a speed) PASSED IN, so the net drifts without ever
    /// looping. -1 is the dark floor, 1 the brightest line.
    ///
    /// THE SAME PATTERN ON THE CPU AND THE GPU (within float rounding): the GPU's 32-bit hash
    /// and single-precision arithmetic are used here too - a deliberate difference from the value
    /// and gradient kinds. The seed's low 32 bits are the hash's seed (so a fractal sum's
    /// per-octave `seed + o` stays the GPU's). The coordinates are taken in single precision:
    /// keep them within some thousands of cells for a clean pattern.
    pub fn caustics(x: f64, y: f64, z: f64, seed: i64) -> f64 {
        caustics_once(seed as u32, x as f32, y as f32, z as f32) as f64
    }
}

/// One octave of the caustics pattern, in single precision, as the GPU computes it.
pub(crate) fn caustics_once(seed: u32, x: f32, y: f32, z: f32) -> f32 {
    let (f1, f2) = nearest_two(seed, x, y, z, 1);
    let t = ((f2 - f1) / LINE_WIDTH).clamp(0.0, 1.0);
    let line = 1.0 - t * t * (3.0 - 2.0 * t);
    2.0 * line * line - 1.0
}

/// The distances to the nearest and second-nearest cell point of the warped position, searching
/// `reach` cells around (the pattern uses 1; the tests check 2 finds the same).
pub(crate) fn nearest_two(seed: u32, x: f32, y: f32, z: f32, reach: i32) -> (f32, f32) {
    let wx = x + WARP_AMOUNT * (0.73 * y + 0.61 * z).sin();
    let wy = y + WARP_AMOUNT * (0.67 * x - 0.53 * z).sin();
    let (ix, iy) = (wx.floor(), wy.floor());
    let (fx, fy) = (wx - ix, wy - iy);
    let (cx, cy) = (ix as i32, iy as i32);
    let mut f1 = 1.0e9f32;
    let mut f2 = 1.0e9f32;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let (px, py) = cell_point(seed, cx.wrapping_add(dx), cy.wrapping_add(dy), z);
            let ox = dx as f32 + px - fx;
            let oy = dy as f32 + py - fy;
            let d = (ox * ox + oy * oy).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2)
}

/// A cell's point at a time, as an offset from the cell's corner (0.2 to 0.8 on each axis).
pub(crate) fn cell_point(seed: u32, cell_x: i32, cell_y: i32, z: f32) -> (f32, f32) {
    let h = noise_hash(seed, cell_x, cell_y, 0);
    let rest_x = 0.35 + 0.3 * (h & 255) as f32 / 255.0;
    let rest_y = 0.35 + 0.3 * ((h >> 8) & 255) as f32 / 255.0;
    let start = ((h >> 16) & 1023) as f32 / 1024.0 * std::f32::consts::TAU;
    let rate = 0.5 + ((h >> 26) & 31) as f32 / 31.0;
    let turn = if (h >> 31) != 0 { -rate } else { rate };
    let angle = start + turn * z;
    (rest_x + ORBIT_RADIUS * angle.cos(), rest_y + ORBIT_RADIUS * angle.sin())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caustics_stays_in_range_and_reaches_both_ends() {
        //Arrange
        let (mut low, mut high) = (f64::MAX, f64::MIN);
        //Act
        for j in 0..200 {
            for i in 0..200 {
                let v = DrawingNoise::caustics(i as f64 * 0.037, j as f64 * 0.041, 0.7, 5);
                low = low.min(v);
                high = high.max(v);
            }
        }
        //Assert
        assert!(low >= -1.0 && high <= 1.0, "{low} {high}");
        assert!(low == -1.0, "the dark floor is reached: {low}");
        assert!(high > 0.95, "the brightest line is reached: {high}");
    }

    #[test]
    fn caustics_three_by_three_cells_find_the_same_two_points_as_five_by_five() {
        for j in 0..150 {
            for i in 0..150 {
                let (x, y, z) = (i as f32 * 0.071 - 3.0, j as f32 * 0.067 - 2.0, (i + j) as f32 * 0.013);
                assert_eq!(nearest_two(11, x, y, z, 1), nearest_two(11, x, y, z, 2), "at {x} {y} {z}");
            }
        }
    }

    #[test]
    fn caustics_has_no_seams_across_cell_borders() {
        // Neighbouring samples 0.002 apart differ by a bounded amount everywhere, including where
        // they straddle a border of the warped lattice.
        let step = 0.002f64;
        // The value's steepest slope: 4 l x 1.5 / LINE_WIDTH per unit of F2 - F1 (l <= 1, the
        // smoothstep's slope <= 1.5); F2 - F1 moves at most 2 per unit of warped position, and
        // the warp stretches position by at most 1 + 0.3 (0.73 + 0.67). A seam would jump by up
        // to 2.
        let bound = 4.0 * 1.5 / LINE_WIDTH as f64 * 2.0 * (1.0 + 0.3 * 1.4) * step * 1.05;
        let mut crossings = 0;
        for j in 0..60 {
            let y = j as f64 * 0.113;
            let mut previous = DrawingNoise::caustics(0.0, y, 0.4, 3);
            for i in 1..2500 {
                let x = i as f64 * step;
                let v = DrawingNoise::caustics(x, y, 0.4, 3);
                let (a, b) = (x - step, x);
                if (a + 0.3 * (0.73 * y + 0.61 * 0.4).sin()).floor() != (b + 0.3 * (0.73 * y + 0.61 * 0.4).sin()).floor() {
                    crossings += 1;
                }
                assert!((v - previous).abs() <= bound, "a jump of {} at x {x}, y {y}", (v - previous).abs());
                previous = v;
            }
        }
        assert!(crossings > 100, "the walk crossed cell borders ({crossings})");
    }

    #[test]
    fn caustics_changes_a_little_for_a_small_step_in_z_and_a_lot_for_a_large_one() {
        //Arrange
        let mut small = 0.0f64;
        let mut large = 0.0f64;
        //Act
        for j in 0..80 {
            for i in 0..80 {
                let (x, y) = (i as f64 * 0.05, j as f64 * 0.05);
                let v = DrawingNoise::caustics(x, y, 2.0, 9);
                small = small.max((DrawingNoise::caustics(x, y, 2.001, 9) - v).abs());
                large += (DrawingNoise::caustics(x, y, 4.0, 9) - v).abs();
            }
        }
        //Assert
        assert!(small < 0.05, "a step of 0.001 in z moves values a little: {small}");
        assert!(large / 6400.0 > 0.1, "a step of 2 in z moves the net: {}", large / 6400.0);
    }

    #[test]
    fn caustics_differs_by_seed_and_repeats_for_the_same_inputs() {
        //Arrange
        let mut differ = 0;
        //Act
        for i in 0..400 {
            let (x, y) = (i as f64 * 0.173, i as f64 * 0.091);
            let a = DrawingNoise::caustics(x, y, 1.5, 1);
            assert_eq!(a.to_bits(), DrawingNoise::caustics(x, y, 1.5, 1).to_bits());
            if (a - DrawingNoise::caustics(x, y, 1.5, 2)).abs() > 0.01 {
                differ += 1;
            }
        }
        //Assert
        assert!(differ > 150, "another seed gives another net ({differ} of 400 differ)");
    }

    #[test]
    fn caustics_seed_is_the_low_32_bits() {
        assert_eq!(DrawingNoise::caustics(1.3, 2.7, 0.5, 7).to_bits(), DrawingNoise::caustics(1.3, 2.7, 0.5, 7 + (5i64 << 32)).to_bits());
        assert_eq!(DrawingNoise::caustics(1.3, 2.7, 0.5, -1).to_bits(), (caustics_once(u32::MAX, 1.3, 2.7, 0.5) as f64).to_bits());
    }

    /// paint.wgsl `noise_value` for the caustics kind, transcribed: f32 throughout, the seed
    /// advanced per octave in 32 bits, z scaled per octave in 3D.
    fn gpu_noise_value(seed32: u32, x: f32, y: f32, z: Option<f32>, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
        let (mut total, mut norm, mut amplitude, mut scale) = (0.0f32, 0.0f32, 1.0f32, 1.0f32);
        for o in 0..octaves {
            let qz = z.map_or(0.0, |z| z * scale);
            total += amplitude * caustics_once(seed32.wrapping_add(o), x * scale, y * scale, qz);
            norm += amplitude;
            amplitude *= gain;
            scale *= lacunarity;
        }
        (total / norm).clamp(-1.0, 1.0)
    }

    #[test]
    fn caustics_kind_samples_and_fractal_sums_are_the_gpu_shader_values() {
        use crate::planes::{DrawingNoiseFractalOptions, DrawingNoiseKind};
        let options = DrawingNoiseFractalOptions { octaves: 2, lacunarity: 1.7, gain: 0.6 };
        let mut worst = 0.0f64;
        for seed in [7i64, -3, (1i64 << 33) + 5, i64::MIN + 1] {
            for i in 0..300 {
                let (x, y, z) = (i as f64 * 0.137 - 9.0, i as f64 * 0.071 + 2.0, i as f64 * 0.01);
                assert_eq!(DrawingNoise::sample(DrawingNoiseKind::Caustics, x, y, seed), DrawingNoise::caustics(x, y, 0.0, seed));
                assert_eq!(DrawingNoise::sample_3d(DrawingNoiseKind::Caustics, x, y, z, seed), DrawingNoise::caustics(x, y, z, seed));
                let cpu_3d = DrawingNoise::fractal_3d(DrawingNoiseKind::Caustics, x, y, z, seed, options).unwrap();
                let cpu_2d = DrawingNoise::fractal(DrawingNoiseKind::Caustics, x, y, seed, options).unwrap();
                let gpu_3d = gpu_noise_value(seed as u32, x as f32, y as f32, Some(z as f32), 2, 1.7, 0.6) as f64;
                let gpu_2d = gpu_noise_value(seed as u32, x as f32, y as f32, None, 2, 1.7, 0.6) as f64;
                worst = worst.max((cpu_3d - gpu_3d).abs()).max((cpu_2d - gpu_2d).abs());
            }
        }
        // The CPU scales the coordinates per octave in double precision, the shader in single.
        assert!(worst < 1e-3, "CPU fractal against the shader's octave loop: {worst}");
    }

    #[test]
    fn caustics_kind_fills_a_plane_with_the_pattern_at_the_pixel_centres() {
        use crate::planes::{DrawingNoiseFillOptions, DrawingNoiseKind, DrawingPlane};
        //Arrange
        let mut plane = DrawingPlane::new(40, 30).unwrap();
        let options = DrawingNoiseFillOptions { z: Some(1.25), ..DrawingNoiseFillOptions::default() };
        //Act
        DrawingNoise::fill(&mut plane, DrawingNoiseKind::Caustics, 4, 0.08, options).unwrap();
        //Assert
        let data = plane.data();
        for y in 0..30 {
            for x in 0..40 {
                let expected = DrawingNoise::caustics((x as f64 + 0.5) * 0.08, (y as f64 + 0.5) * 0.08, 1.25, 4) as f32;
                assert_eq!(data[y * 40 + x], expected);
            }
        }
        assert!(data.iter().any(|&v| v > 0.5) && data.iter().any(|&v| v == -1.0), "lines and dark floor both present");
    }

    #[test]
    fn caustics_cell_points_follow_the_hash_and_stay_inside_their_cells() {
        // The hash is the WGSL noise_hash (hand-computed values are pinned in
        // painting/tests_color.rs); here the point's construction from it is checked by hand.
        let h = noise_hash(0, 0, 0, 0);
        assert_eq!(h, 3_680_091_388);
        let (x, y) = cell_point(0, 0, 0, 0.0);
        let rest_x = 0.35 + 0.3 * (h & 255) as f32 / 255.0;
        let rest_y = 0.35 + 0.3 * ((h >> 8) & 255) as f32 / 255.0;
        let start = ((h >> 16) & 1023) as f32 / 1024.0 * std::f32::consts::TAU;
        assert_eq!((x, y), (rest_x + 0.15 * start.cos(), rest_y + 0.15 * start.sin()));
        for cell in 0..2000 {
            for z in [0.0f32, 1.7, 40.0] {
                let (px, py) = cell_point(77, cell, -cell * 3, z);
                assert!((0.2 - 1e-6..=0.8 + 1e-6).contains(&px) && (0.2 - 1e-6..=0.8 + 1e-6).contains(&py), "{px} {py}");
            }
        }
    }
}
