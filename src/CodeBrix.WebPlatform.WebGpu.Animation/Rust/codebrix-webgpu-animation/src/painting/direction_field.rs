//! `DirectionField`: which way strokes should lie at every point of a rectangle - from a
//! picture's structure, from noise or from hand-placed guides - stored as doubled-angle vectors
//! so that orientations blend correctly (no C# equivalent). Planning-time CPU work.

use crate::geometry::{PointF, RectangleF};
use crate::painting::detail::{axis, bilinear, luminance_plane, sobel, DetailMap};
use crate::painting::DirectionGuide;
use crate::planes::{DrawingNoise, DrawingNoiseFractalOptions, DrawingNoiseKind, DrawingPlane, DrawingPlaneImage};

/// The largest field side, cells.
const MAX_SIDE: u32 = 8192;

/// A field of ORIENTATIONS over a rectangle of calibrated units (no C# equivalent): at each
/// cell, a direction (no head or tail: 0 and 180 degrees are the same) and a strength 0..1 (how
/// sure the field is there).
///
/// THE DOUBLED ANGLE: a direction theta is stored as the vector `(cos 2 theta, sin 2 theta)`, so
/// orientations blend correctly - a stroke at 179 degrees and one at 1 degree average to 0, not
/// 90. Angles are degrees clockwise on screen from +x (BrushStrokeShape's unit); the field gives
/// them in (-90, 90].
///
/// SAMPLING: bilinear between cell centres on the doubled-angle vectors (and on the strengths);
/// outside its rectangle a field holds its edge values. Made at PLANNING time (once per painting
/// or keyframe; a picture field costs tens of milliseconds), not per frame. Every function is
/// exact and deterministic: one thread, a fixed loop order.
#[derive(Clone, Debug, PartialEq)]
pub struct DirectionField {
    width: u32,
    height: u32,
    destination: RectangleF,
    c: Vec<f32>,
    s: Vec<f32>,
    strength: Vec<f32>,
}

/// The options of [`DirectionField::from_picture`] (no C# equivalent).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PictureFieldOptions {
    /// The field's size in cells (None: the picture's size). A smaller field is faster to make;
    /// the picture's luminance is resized to it (bilinear) before the gradients are taken.
    pub field_size: Option<(u32, u32)>,
    /// How far the structure is smoothed, field cells (the Gaussian sigma of the structure
    /// tensor's blur; 0 = none). 3 by default.
    pub smoothing: f32,
}

impl Default for PictureFieldOptions {
    fn default() -> PictureFieldOptions {
        PictureFieldOptions { field_size: None, smoothing: 3.0 }
    }
}

/// The options of [`DirectionField::from_noise`] (no C# equivalent).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoiseFieldOptions {
    /// The third noise coordinate (often a time times a speed - passed in, never read from a
    /// clock), or None for 2D noise (the default).
    pub z: Option<f64>,
    /// How many half turns the noise's full swing (-1..1) turns the direction by: the angle is
    /// `base_angle + turns x 180 x noise`. 1 by default.
    pub turns: f32,
    /// The direction where the noise is 0, degrees (0 by default).
    pub base_angle: f32,
    /// The fractal octaves, 1 to 30 (1 = plain noise, the default; lacunarity 2, gain 0.5).
    pub octaves: u32,
}

impl Default for NoiseFieldOptions {
    fn default() -> NoiseFieldOptions {
        NoiseFieldOptions { z: None, turns: 1.0, base_angle: 0.0, octaves: 1 }
    }
}

fn validate_frame(size: (u32, u32), destination: RectangleF) -> Result<(), String> {
    if size.0 < 1 || size.1 < 1 || size.0 > MAX_SIDE || size.1 > MAX_SIDE {
        return Err(format!("A direction field must be 1 x 1 to {MAX_SIDE} x {MAX_SIDE} cells."));
    }
    if !destination.x.is_finite() || !destination.y.is_finite() || !destination.width.is_finite() || !destination.height.is_finite() || destination.is_empty() {
        return Err("The destination must be a finite rectangle with a positive width and height.".to_string());
    }
    Ok(())
}

/// A direction (degrees) from its doubled-angle vector, in (-90, 90]; 0 for a zero vector.
fn half_angle(c: f32, s: f32) -> f32 {
    let a = (s as f64).atan2(c as f64).to_degrees() * 0.5;
    (if a <= -90.0 { a + 180.0 } else { a }) as f32
}

/// The doubled-angle vector of a direction in degrees (double inside).
fn doubled(degrees: f64) -> (f32, f32) {
    let r = (2.0 * degrees).to_radians();
    (r.cos() as f32, r.sin() as f32)
}

/// A direction difference wrapped to (-90, 90] (directions have no head).
fn wrap_turn(degrees: f32) -> f32 {
    let mut d = degrees % 180.0;
    if d > 90.0 {
        d -= 180.0;
    } else if d <= -90.0 {
        d += 180.0;
    }
    d
}

impl DirectionField {
    fn with_cells(size: (u32, u32), destination: RectangleF, mut cell: impl FnMut(f64, f64) -> (f32, f32, f32)) -> DirectionField {
        let (w, h) = (size.0 as usize, size.1 as usize);
        let mut field = DirectionField { width: size.0, height: size.1, destination, c: vec![0.0; w * h], s: vec![0.0; w * h], strength: vec![0.0; w * h] };
        for j in 0..h {
            for i in 0..w {
                let (x, y) = field.cell_center(i, j);
                let (c, s, strength) = cell(x, y);
                let k = (j * w) + i;
                field.c[k] = c;
                field.s[k] = s;
                field.strength[k] = strength;
            }
        }
        field
    }

    fn cell_center(&self, i: usize, j: usize) -> (f64, f64) {
        let d = self.destination;
        (d.x as f64 + ((i as f64 + 0.5) * d.width as f64 / self.width as f64), d.y as f64 + ((j as f64 + 0.5) * d.height as f64 / self.height as f64))
    }

    /// From a picture's STRUCTURE: luminance (Rec. 709 weights times alpha) -> Sobel gradients ->
    /// the structure tensor (gx^2, gx gy, gy^2) blurred by `smoothing` -> the direction ALONG
    /// edges (perpendicular to the dominant gradient), strength = the coherence squared,
    /// `((l1 - l2) / (l1 + l2))^2` of the tensor's eigenvalues (1 along a clean edge, 0 where the
    /// picture is flat or has no single direction). The picture is laid over `destination`. The
    /// error says what is wrong.
    pub fn from_picture(picture: &DrawingPlaneImage, destination: RectangleF, options: PictureFieldOptions) -> Result<DirectionField, String> {
        if !options.smoothing.is_finite() || options.smoothing < 0.0 {
            return Err("The smoothing must be a finite number of cells, 0 or more.".to_string());
        }
        let size = options.field_size.unwrap_or(picture.size());
        validate_frame(size, destination)?;
        let mut luminance = luminance_plane(picture)?;
        if luminance.size() != size {
            luminance = luminance.resized(size.0, size.1, None)?;
        }
        let (gx, gy) = sobel(&luminance);
        let tensor = |values: Vec<f32>| -> Result<DrawingPlane, String> {
            let mut plane = DrawingPlane::from_data(size.0, size.1, values)?;
            if options.smoothing > 0.0 {
                plane.gaussian_blur(options.smoothing)?;
            }
            Ok(plane)
        };
        let jxx = tensor(gx.iter().map(|g| g * g).collect())?;
        let jxy = tensor(gx.iter().zip(gy.iter()).map(|(a, b)| a * b).collect())?;
        let jyy = tensor(gy.iter().map(|g| g * g).collect())?;
        let (xx, xy, yy) = (jxx.data(), jxy.data(), jyy.data());
        let mut k = 0usize;
        Ok(DirectionField::with_cells(size, destination, |_, _| {
            let (a, b, d) = (xx[k] as f64, xy[k] as f64, yy[k] as f64);
            k += 1;
            let difference = a - d;
            let r = (difference * difference + (4.0 * b * b)).sqrt();
            let trace = a + d;
            if !(r > 1.0e-12) || !(trace > 1.0e-12) {
                return (1.0, 0.0, 0.0);
            }
            // The gradient's doubled angle is (difference, 2b) / r; along the edge is 90 degrees
            // on, which negates the doubled vector.
            let coherence = (r / trace).min(1.0);
            ((-difference / r) as f32, (-2.0 * b / r) as f32, (coherence * coherence) as f32)
        }))
    }

    /// From noise (exact: [`DrawingNoise::fractal`] / `fractal_3d` at each cell centre's
    /// calibrated position times `frequency`): angle = `base_angle + turns x 180 x noise`,
    /// strength 1. The error says what is wrong.
    pub fn from_noise(size: (u32, u32), destination: RectangleF, kind: DrawingNoiseKind, seed: i64, frequency: f64, options: NoiseFieldOptions) -> Result<DirectionField, String> {
        validate_frame(size, destination)?;
        if !frequency.is_finite() || frequency <= 0.0 {
            return Err("The frequency must be a positive, finite number (noise cells per calibrated unit).".to_string());
        }
        if !options.turns.is_finite() || !options.base_angle.is_finite() {
            return Err("The turns and the base angle must be finite.".to_string());
        }
        if options.z.is_some_and(|z| !z.is_finite()) {
            return Err("The noise's z must be finite.".to_string());
        }
        let fractal = DrawingNoiseFractalOptions { octaves: options.octaves, ..DrawingNoiseFractalOptions::default() };
        // Checks the octaves once, with the noise's own message.
        DrawingNoise::fractal(kind, 0.0, 0.0, seed, fractal)?;
        Ok(DirectionField::with_cells(size, destination, |x, y| {
            let n = match options.z {
                Some(z) => DrawingNoise::fractal_3d(kind, x * frequency, y * frequency, z, seed, fractal),
                None => DrawingNoise::fractal(kind, x * frequency, y * frequency, seed, fractal),
            }
            .unwrap_or(0.0);
            let (c, s) = doubled(options.base_angle as f64 + (options.turns as f64 * 180.0 * n));
            (c, s, 1.0)
        }))
    }

    /// From hand-placed guides: at each cell the guides' doubled-angle vectors summed, each
    /// weighted by its pull `weight / (1 + d^2) x (1 - (d / reach)^2)^2` (inverse-distance
    /// weighting, d the distance in calibrated units to the guide's path or centre, tapered to 0
    /// at the reach; the taper is 1 for an infinite reach). The direction is the sum's; the
    /// strength is how far the guides agree there (the sum's length over the total pull: 1 for one
    /// guide, less where they disagree; 0 where no guide reaches - the direction there is 0).
    /// The error names the unusable guide.
    pub fn from_guides(size: (u32, u32), destination: RectangleF, guides: &[DirectionGuide]) -> Result<DirectionField, String> {
        validate_frame(size, destination)?;
        let prepared = guides.iter().enumerate().map(|(i, g)| g.prepare(i)).collect::<Result<Vec<_>, String>>()?;
        Ok(DirectionField::with_cells(size, destination, |x, y| {
            let (mut sc, mut ss, mut total) = (0.0f64, 0.0f64, 0.0f64);
            for guide in &prepared {
                if let Some((angle, w)) = guide.sample(x, y) {
                    let r = (2.0 * angle).to_radians();
                    sc += w * r.cos();
                    ss += w * r.sin();
                    total += w;
                }
            }
            if !(total > 0.0) {
                return (1.0, 0.0, 0.0);
            }
            let length = sc.hypot(ss);
            if !(length > 0.0) {
                return (1.0, 0.0, 0.0);
            }
            ((sc / length) as f32, (ss / length) as f32, (length / total).min(1.0) as f32)
        }))
    }

    /// This field and another mixed by a weight plane (0 = this, 1 = the other; laid over this
    /// field's rectangle and sampled bilinearly, clamped to 0..1), on the doubled-angle vectors:
    /// the result has this field's size and rectangle, the other field sampled at its cell
    /// centres, the strengths mixed by the same weight. Where the two point exactly across each
    /// other at an even mix, the heavier side wins (this field at exactly 0.5). The error says
    /// what is wrong with the plane.
    pub fn blend(&self, other: &DirectionField, weight: &DrawingPlane) -> Result<DirectionField, String> {
        if weight.data().iter().any(|v| !v.is_finite()) {
            return Err("The weight plane must hold finite values (0 = this field, 1 = the other).".to_string());
        }
        // Cells are visited row by row, left to right: the k-th visit is cell k.
        let mut k = 0usize;
        Ok(DirectionField::with_cells((self.width, self.height), self.destination, |x, y| {
            let idx = k;
            k += 1;
            let t = DetailMap::sample(weight, self.destination, x as f32, y as f32).clamp(0.0, 1.0);
            let (oc, os) = other.unit_vector_at(x as f32, y as f32);
            let c = (self.c[idx] * (1.0 - t)) + (oc * t);
            let s = (self.s[idx] * (1.0 - t)) + (os * t);
            let length = c.hypot(s);
            let strength = (self.strength[idx] * (1.0 - t)) + (other.strength_at(x as f32, y as f32) * t);
            if length > 1.0e-6 {
                (c / length, s / length, strength)
            } else if t > 0.5 {
                (oc, os, strength)
            } else {
                (self.c[idx], self.s[idx], strength)
            }
        }))
    }

    /// The same field with every direction turned by `degrees` (clockwise on screen).
    pub fn rotated(&self, degrees: f32) -> DirectionField {
        let (rc, rs) = doubled(degrees as f64);
        let mut field = self.clone();
        for k in 0..field.c.len() {
            let (c, s) = (self.c[k], self.s[k]);
            field.c[k] = (c * rc) - (s * rs);
            field.s[k] = (c * rs) + (s * rc);
        }
        field
    }

    /// The field's size, cells.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The rectangle the field lies over, calibrated units.
    pub fn destination(&self) -> RectangleF {
        self.destination
    }

    /// The direction at a point, degrees clockwise from +x in (-90, 90] (bilinear on the
    /// doubled-angle vectors; edge values outside the rectangle).
    pub fn angle_at(&self, x: f32, y: f32) -> f32 {
        let (c, s) = self.vector_at(x, y);
        half_angle(c, s)
    }

    /// The strength at a point, 0..1 (bilinear; edge values outside the rectangle).
    pub fn strength_at(&self, x: f32, y: f32) -> f32 {
        bilinear(&self.strength, self.width as usize, self.height as usize, self.destination, x, y)
    }

    fn vector_at(&self, x: f32, y: f32) -> (f32, f32) {
        let (w, h) = (self.width as usize, self.height as usize);
        (bilinear(&self.c, w, h, self.destination, x, y), bilinear(&self.s, w, h, self.destination, x, y))
    }

    fn unit_vector_at(&self, x: f32, y: f32) -> (f32, f32) {
        let (c, s) = self.vector_at(x, y);
        let length = c.hypot(s);
        if length > 1.0e-6 {
            (c / length, s / length)
        } else {
            // Cancelling neighbours: the nearest cell's own vector.
            let (i, _, _) = axis(x, self.destination.x, self.destination.width, self.width as usize);
            let (j, _, _) = axis(y, self.destination.y, self.destination.height, self.height as usize);
            let k = (j * self.width as usize) + i;
            (self.c[k], self.s[k])
        }
    }

    /// The angle and bend of a stroke of `length` centred at a point that follows the field:
    /// the angle is the field's at the centre; the bend comes from the turn between the
    /// directions at its two ends (`center -+ length / 2` along that angle): a circular arc's bow,
    /// `bend = tan(turn / 4) / 2`, the turn from the first tip's end to the other's (clockwise
    /// positive) - BrushStrokeShape's sign, so the stroke curves the way the field does.
    pub fn stroke_pose(&self, center: PointF, length: f32) -> (f32, f32) {
        let angle = self.angle_at(center.x, center.y);
        let radians = (angle as f64).to_radians();
        let (dx, dy) = ((radians.cos() * length as f64 / 2.0) as f32, (radians.sin() * length as f64 / 2.0) as f32);
        let first = self.angle_at(center.x - dx, center.y - dy);
        let last = self.angle_at(center.x + dx, center.y + dy);
        let turn = wrap_turn(last - first) as f64;
        let bend = ((turn / 4.0).to_radians().tan() / 2.0) as f32;
        (angle, if bend.is_finite() { bend } else { 0.0 })
    }

    /// A streamline from a point: `steps` steps of `step` calibrated units along the field
    /// (midpoint rule), each step keeping the heading of the one before (a direction has no head,
    /// so the line does not turn back); the first heading is the field's angle at the start.
    /// `steps + 1` points, the start first; only the start for a step that is not a positive,
    /// finite distance.
    pub fn trace(&self, start: PointF, step: f32, steps: usize) -> Vec<PointF> {
        let mut points = vec![start];
        if !step.is_finite() || step <= 0.0 {
            return points;
        }
        let direction = |p: (f64, f64), heading: (f64, f64)| -> (f64, f64) {
            let r = (self.angle_at(p.0 as f32, p.1 as f32) as f64).to_radians();
            let d = (r.cos(), r.sin());
            if (d.0 * heading.0) + (d.1 * heading.1) < 0.0 {
                (-d.0, -d.1)
            } else {
                d
            }
        };
        let first = (self.angle_at(start.x, start.y) as f64).to_radians();
        let mut heading = (first.cos(), first.sin());
        let mut p = (start.x as f64, start.y as f64);
        let step = step as f64;
        for _ in 0..steps {
            let d0 = direction(p, heading);
            let middle = (p.0 + (d0.0 * step / 2.0), p.1 + (d0.1 * step / 2.0));
            let d1 = direction(middle, d0);
            p = (p.0 + (d1.0 * step), p.1 + (d1.1 * step));
            heading = d1;
            points.push(PointF::new(p.0 as f32, p.1 as f32));
        }
        points
    }
}
