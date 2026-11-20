//! `DrawingPlane`: one rectangular plane of open-ended float values (C# `DrawingPlane`).

use crate::planes::plane_kernels as k;
use crate::planes::{DrawingPlaneBloomOptions, DrawingPlaneResampling};
use crate::Rectangle;

/// The most values one plane holds (C#'s `Array.MaxLength`).
const MAX_LENGTH: u64 = 0x7FFF_FFC7;

/// One rectangular plane of open-ended float values (C# `DrawingPlane`) - a colour channel, an
/// alpha channel, a coverage mask or any per-pixel quantity (light, height, noise). Values are
/// NOT clamped: lights may add up past 1.0 and a tone curve or the 8-bit conversion brings them
/// back.
///
/// Storage is row-major: pixel (x, y) is `data()[y * width + x]`, and [`DrawingPlane::data_mut`]
/// is the live backing store for fast direct access.
///
/// Operations that keep the size (fills, arithmetic, blur, tone curve ...) change the plane IN
/// PLACE and return it, so calls chain; clone first to keep the original. Operations that change
/// the size ([`resized`](DrawingPlane::resized), [`downsampled`](DrawingPlane::downsampled),
/// [`create_bloom`](DrawingPlane::create_bloom)) return a NEW plane.
///
/// Every computation is C#'s, in C#'s order, on ONE THREAD (a browser gives a frontend one
/// thread; C# shares rows out across threads but its results never depend on that): integer and
/// float arithmetic gives the same values as C# bit for bit; operations that call the math
/// library (`exp` in the blur kernel, `powf` in the gamma) match C# on the same machine and may
/// differ in the last bit in a browser, whose math library is Rust's own.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingPlane {
    width: u32,
    height: u32,
    data: Vec<f32>,
}

impl DrawingPlane {
    /// A plane of zeros (C# `DrawingPlane(width, height)`). The error says what to do with a
    /// size below 1 x 1 or one too large.
    pub fn new(width: u32, height: u32) -> Result<DrawingPlane, String> {
        validate_size(width, height)?;
        Ok(DrawingPlane { width, height, data: vec![0.0; width as usize * height as usize] })
    }

    /// A plane with every value set to `value` (C# `DrawingPlane(width, height, value)`).
    pub fn with_value(width: u32, height: u32, value: f32) -> Result<DrawingPlane, String> {
        validate_size(width, height)?;
        Ok(DrawingPlane { width, height, data: vec![value; width as usize * height as usize] })
    }

    /// A plane over existing values WITHOUT copying them (C# `DrawingPlane(width, height,
    /// float[])`; the plane takes the vector, [`DrawingPlane::into_data`] gives it back).
    pub fn from_data(width: u32, height: u32, data: Vec<f32>) -> Result<DrawingPlane, String> {
        validate_size(width, height)?;
        if data.len() != width as usize * height as usize {
            return Err("The data must hold exactly width x height values.".to_string());
        }
        Ok(DrawingPlane { width, height, data })
    }

    /// The width in pixels (C# `Width`).
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels (C# `Height`).
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The size in pixels, (width, height) (C# `Size`).
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The number of values, width x height (C# `Length`).
    pub fn length(&self) -> usize {
        self.data.len()
    }

    /// The values, row-major (C# `Data` / `AsSpan()`).
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    /// The values to change, row-major - the fastest way to do custom per-pixel work (C# `Data`).
    pub fn data_mut(&mut self) -> &mut [f32] {
        &mut self.data
    }

    /// The values, given back.
    pub fn into_data(self) -> Vec<f32> {
        self.data
    }

    /// One pixel's value (C#'s indexer). The error names the position outside the plane.
    pub fn get(&self, x: u32, y: u32) -> Result<f32, String> {
        self.check_position(x, y)?;
        Ok(self.data[self.index(x, y)])
    }

    /// Sets one pixel's value (C#'s indexer).
    pub fn set(&mut self, x: u32, y: u32, value: f32) -> Result<(), String> {
        self.check_position(x, y)?;
        let i = self.index(x, y);
        self.data[i] = value;
        Ok(())
    }

    /// One row of values (C# `GetRow`).
    pub fn get_row(&self, y: u32) -> Result<&[f32], String> {
        if y >= self.height {
            return Err(format!("Row {y} is outside the plane (its height is {}).", self.height));
        }
        let start = y as usize * self.width as usize;
        Ok(&self.data[start..start + self.width as usize])
    }

    /// One row of values to change (C# `GetRow`).
    pub fn get_row_mut(&mut self, y: u32) -> Result<&mut [f32], String> {
        if y >= self.height {
            return Err(format!("Row {y} is outside the plane (its height is {}).", self.height));
        }
        let start = y as usize * self.width as usize;
        let width = self.width as usize;
        Ok(&mut self.data[start..start + width])
    }

    /// Copies every value into another plane of the same size (C# `CopyTo`).
    pub fn copy_to(&self, destination: &mut DrawingPlane) -> Result<(), String> {
        self.require_same_size(destination)?;
        destination.data.copy_from_slice(&self.data);
        Ok(())
    }

    // ---- Filling --------------------------------------------------------------------------

    /// Sets every value to zero (C# `Clear`).
    pub fn clear(&mut self) -> &mut DrawingPlane {
        self.data.fill(0.0);
        self
    }

    /// Sets every value (C# `Fill(value)`).
    pub fn fill(&mut self, value: f32) -> &mut DrawingPlane {
        self.data.fill(value);
        self
    }

    /// Sets every value inside a rectangle, clipped to the plane (C# `Fill(Rectangle, value)`).
    pub fn fill_area(&mut self, area: Rectangle, value: f32) -> &mut DrawingPlane {
        if let Some((x0, y0, x1, y1)) = self.clip(area) {
            for y in y0..y1 {
                let row = y * self.width as usize;
                self.data[row + x0..row + x1].fill(value);
            }
        }
        self
    }

    /// Sets every value from a function of its pixel position (x, y), row by row from the top
    /// (C# `Fill(Func)` - and `FillParallel`, whose result C# defines as this one's: here it runs
    /// on the one thread a browser has).
    pub fn fill_with(&mut self, function: impl FnMut(u32, u32) -> f32) -> &mut DrawingPlane {
        let all = Rectangle::new(0, 0, self.width as i32, self.height as i32);
        self.fill_area_with(all, function)
    }

    /// Sets every value inside a rectangle (clipped to the plane) from a function of its
    /// ABSOLUTE pixel position (C# `Fill(Rectangle, Func)` / `FillParallel(Rectangle, Func)`).
    pub fn fill_area_with(&mut self, area: Rectangle, mut function: impl FnMut(u32, u32) -> f32) -> &mut DrawingPlane {
        if let Some((x0, y0, x1, y1)) = self.clip(area) {
            for y in y0..y1 {
                let row = y * self.width as usize;
                for x in x0..x1 {
                    self.data[row + x] = function(x as u32, y as u32);
                }
            }
        }
        self
    }

    // ---- Arithmetic (in place) ------------------------------------------------------------

    /// Adds a constant to every value (C# `Add(float)`).
    pub fn add(&mut self, value: f32) -> &mut DrawingPlane {
        self.data.iter_mut().for_each(|d| *d += value);
        self
    }

    /// `this = this + other * scale` (C# `Add(DrawingPlane, scale = 1)`).
    pub fn add_plane(&mut self, other: &DrawingPlane, scale: Option<f32>) -> Result<&mut DrawingPlane, String> {
        self.require_same_size(other)?;
        let scale = scale.unwrap_or(1.0);
        self.data.iter_mut().zip(&other.data).for_each(|(d, o)| *d += *o * scale);
        Ok(self)
    }

    /// `this = this - other` (C# `Subtract`).
    pub fn subtract(&mut self, other: &DrawingPlane) -> Result<&mut DrawingPlane, String> {
        self.require_same_size(other)?;
        self.data.iter_mut().zip(&other.data).for_each(|(d, o)| *d -= *o);
        Ok(self)
    }

    /// Multiplies every value by a constant (C# `Multiply(float)`).
    pub fn multiply(&mut self, factor: f32) -> &mut DrawingPlane {
        self.data.iter_mut().for_each(|d| *d *= factor);
        self
    }

    /// Multiplies by another plane, pixel by pixel - a coverage mask, for example (C#
    /// `Multiply(DrawingPlane)`).
    pub fn multiply_plane(&mut self, other: &DrawingPlane) -> Result<&mut DrawingPlane, String> {
        self.require_same_size(other)?;
        self.data.iter_mut().zip(&other.data).for_each(|(d, o)| *d *= *o);
        Ok(self)
    }

    /// `this = this * (1 - t) + other * t`, `1 - t` computed once as a float; t not clamped (C#
    /// `Lerp(DrawingPlane, float)`).
    pub fn lerp(&mut self, other: &DrawingPlane, t: f32) -> Result<&mut DrawingPlane, String> {
        self.require_same_size(other)?;
        let s = 1.0 - t;
        self.data.iter_mut().zip(&other.data).for_each(|(d, o)| *d = (*d * s) + (*o * t));
        Ok(self)
    }

    /// `this = this * (1 - m) + other * m` with m from a mask, not clamped (C# `Lerp(DrawingPlane,
    /// DrawingPlane)`).
    pub fn lerp_mask(&mut self, other: &DrawingPlane, mask: &DrawingPlane) -> Result<&mut DrawingPlane, String> {
        self.require_same_size(other)?;
        self.require_same_size(mask)?;
        for i in 0..self.data.len() {
            let t = mask.data[i];
            self.data[i] = (self.data[i] * (1.0 - t)) + (other.data[i] * t);
        }
        Ok(self)
    }

    /// Caps every value at `value`; NaN stays NaN (C# `Min(float)`).
    pub fn min(&mut self, value: f32) -> &mut DrawingPlane {
        self.data.iter_mut().for_each(|d| {
            if *d > value {
                *d = value;
            }
        });
        self
    }

    /// The smaller of each value and the other plane's (C# `Min(DrawingPlane)`).
    pub fn min_plane(&mut self, other: &DrawingPlane) -> Result<&mut DrawingPlane, String> {
        self.require_same_size(other)?;
        self.data.iter_mut().zip(&other.data).for_each(|(d, o)| {
            if *d > *o {
                *d = *o;
            }
        });
        Ok(self)
    }

    /// Raises every value to at least `value`; NaN stays NaN (C# `Max(float)`).
    pub fn max(&mut self, value: f32) -> &mut DrawingPlane {
        self.data.iter_mut().for_each(|d| {
            if *d < value {
                *d = value;
            }
        });
        self
    }

    /// The larger of each value and the other plane's (C# `Max(DrawingPlane)`).
    pub fn max_plane(&mut self, other: &DrawingPlane) -> Result<&mut DrawingPlane, String> {
        self.require_same_size(other)?;
        self.data.iter_mut().zip(&other.data).for_each(|(d, o)| {
            if *d < *o {
                *d = *o;
            }
        });
        Ok(self)
    }

    /// Limits every value to min..max; NaN becomes min (C# `Clamp`).
    pub fn clamp(&mut self, min: f32, max: f32) -> Result<&mut DrawingPlane, String> {
        if min > max {
            return Err("The minimum must not be greater than the maximum.".to_string());
        }
        self.data.iter_mut().for_each(|d| *d = clamp_value(*d, min, max));
        Ok(self)
    }

    /// Replaces every value by a function of itself (C# `Apply(Func<float, float>)`).
    pub fn apply(&mut self, mut function: impl FnMut(f32) -> f32) -> &mut DrawingPlane {
        self.data.iter_mut().for_each(|d| *d = function(*d));
        self
    }

    // ---- Tone, gamma, vignette ------------------------------------------------------------

    /// The soft-shoulder tone curve `v / (1 + k * v) * gain`, in float, in that order (C#
    /// `ApplyToneCurve`).
    pub fn apply_tone_curve(&mut self, k: f32, gain: f32) -> &mut DrawingPlane {
        self.data.iter_mut().for_each(|d| *d = tone_curve_value(*d, k, gain));
        self
    }

    /// Raises every positive value to a power (`powf`); values at or below 0 and NaN are kept (C#
    /// `ApplyGamma`). The error says the exponent must be a finite number above 0.
    pub fn apply_gamma(&mut self, exponent: f32) -> Result<&mut DrawingPlane, String> {
        validate_exponent(exponent)?;
        self.data.iter_mut().for_each(|d| {
            if *d > 0.0 {
                *d = d.powf(exponent);
            }
        });
        Ok(self)
    }

    /// Multiplies every value by the vignette factor of [`DrawingPlane::create_vignette`],
    /// computed on the fly, identically (C# `ApplyVignette`; `minimum` 0 by default).
    pub fn apply_vignette(&mut self, center_x: f64, center_y: f64, radius_x: f64, radius_y: f64, strength: f64, minimum: Option<f64>) -> Result<&mut DrawingPlane, String> {
        validate_vignette(radius_x, radius_y)?;
        let minimum = minimum.unwrap_or(0.0);
        let width = self.width as usize;
        for y in 0..self.height as usize {
            let row = y * width;
            let dy = vignette_axis(y, center_y, radius_y);
            for x in 0..width {
                self.data[row + x] *= vignette_factor(vignette_axis(x, center_x, radius_x), dy, strength, minimum);
            }
        }
        Ok(self)
    }

    /// A vignette factor plane (C# `CreateVignette`): `dx = (x - center_x) / radius_x`, `dy = (y -
    /// center_y) / radius_y`, `factor = clamp(1 - strength * (dx*dx + dy*dy), minimum, 1)` in
    /// double, stored as float. Multiply an image or a plane by it to darken towards the edges.
    #[allow(clippy::too_many_arguments)]
    pub fn create_vignette(width: u32, height: u32, center_x: f64, center_y: f64, radius_x: f64, radius_y: f64, strength: f64, minimum: Option<f64>) -> Result<DrawingPlane, String> {
        let mut plane = DrawingPlane::with_value(width, height, 1.0)?;
        plane.apply_vignette(center_x, center_y, radius_x, radius_y, strength, minimum)?;
        Ok(plane)
    }

    // ---- Filters ---------------------------------------------------------------------------

    /// Blurs the plane with a Gaussian, in place (C# `GaussianBlur`): radius `(int)(4 sigma +
    /// 0.5)`, double weights normalised in double and rounded once; separable (horizontal, then
    /// vertical); edges MIRRORED including the edge pixel. A sigma of 0 or less does nothing; the
    /// error says a sigma must be a finite number.
    pub fn gaussian_blur(&mut self, sigma: f32) -> Result<&mut DrawingPlane, String> {
        validate_sigma(sigma)?;
        k::blur(&mut self.data, self.width as usize, self.height as usize, sigma);
        Ok(self)
    }

    /// A resized copy (C# `Resized`; bilinear by default - see [`DrawingPlaneResampling`]).
    pub fn resized(&self, width: u32, height: u32, resampling: Option<DrawingPlaneResampling>) -> Result<DrawingPlane, String> {
        validate_size(width, height)?;
        let (sw, sh, w, h) = (self.width as usize, self.height as usize, width as usize, height as usize);
        let data = match resampling.unwrap_or_default() {
            DrawingPlaneResampling::Nearest => k::resize_nearest(&self.data, sw, sh, w, h),
            DrawingPlaneResampling::Bilinear => k::resize_bilinear(&self.data, sw, sh, w, h, false),
            DrawingPlaneResampling::BilinearAlignedCorners => k::resize_bilinear(&self.data, sw, sh, w, h, true),
        };
        Ok(DrawingPlane { width, height, data })
    }

    /// A copy shrunk by a whole factor with a box filter (C# `Downsampled`): `ceiling(width /
    /// factor) x ceiling(height / factor)`, each pixel the mean of its block (partial blocks at
    /// the right and bottom average what they have).
    pub fn downsampled(&self, factor: u32) -> Result<DrawingPlane, String> {
        if factor < 1 {
            return Err("The downsample factor must be 1 or more.".to_string());
        }
        let width = ((self.width - 1) / factor) + 1;
        let height = ((self.height - 1) / factor) + 1;
        let data = k::downsample(&self.data, self.width as usize, self.height as usize, factor as usize, width as usize, height as usize);
        Ok(DrawingPlane { width, height, data })
    }

    /// A bloom (glow) plane from this plane of light values (C# `CreateBloom`): downsample by
    /// `downsample_factor`; for each (sigma, weight) in order blur a copy of the small plane and
    /// add it times the weight (from 0); multiply by the gain; resize to the output size. Sigmas
    /// are in SMALL-plane pixels. Add the result to the scene to make the lights glow.
    pub fn create_bloom(&self, downsample_factor: u32, blurs: &[(f32, f32)], options: DrawingPlaneBloomOptions) -> Result<DrawingPlane, String> {
        validate_blurs(blurs)?;
        let (w, h) = options.output_size.unwrap_or(self.size());
        validate_size(w, h)?;
        let small = self.downsampled(downsample_factor)?;
        let mut sum = DrawingPlane::new(small.width, small.height)?;
        let mut scratch = DrawingPlane::new(small.width, small.height)?;
        for (sigma, weight) in blurs {
            small.copy_to(&mut scratch)?;
            scratch.gaussian_blur(*sigma)?;
            sum.add_plane(&scratch, Some(*weight))?;
        }
        sum.multiply(options.gain);
        sum.resized(w, h, Some(options.upsampling))
    }

    // ---- 8-bit conversion -------------------------------------------------------------------

    /// Writes one byte per pixel, row-major (C# `ToBytes(Span)`): NaN and values at or below 0
    /// give 0, at or above 1 give 255, else `(byte)(value * 255 + 0.5)`.
    pub fn to_bytes_into(&self, destination: &mut [u8]) -> Result<(), String> {
        if destination.len() < self.data.len() {
            return Err("The destination must hold at least one byte per pixel.".to_string());
        }
        for (d, v) in destination.iter_mut().zip(&self.data) {
            *d = k::unit_to_byte(*v);
        }
        Ok(())
    }

    /// One byte per pixel (C# `ToBytes()`).
    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.iter().map(|v| k::unit_to_byte(*v)).collect()
    }

    /// Overwrites the plane from one byte per pixel, each `byte / 255f` (C# `CopyFromBytes`).
    pub fn copy_from_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() < self.data.len() {
            return Err("There must be at least one byte per pixel.".to_string());
        }
        for (d, b) in self.data.iter_mut().zip(bytes) {
            *d = k::byte_to_unit(*b);
        }
        Ok(())
    }

    /// A plane from one byte per pixel, each `byte / 255f` (C# `FromBytes`).
    pub fn from_bytes(bytes: &[u8], width: u32, height: u32) -> Result<DrawingPlane, String> {
        let mut plane = DrawingPlane::new(width, height)?;
        plane.copy_from_bytes(bytes)?;
        Ok(plane)
    }

    // ---- Helpers ------------------------------------------------------------------------------

    fn index(&self, x: u32, y: u32) -> usize {
        (y as usize * self.width as usize) + x as usize
    }

    fn check_position(&self, x: u32, y: u32) -> Result<(), String> {
        if x >= self.width || y >= self.height {
            return Err(format!("The position ({x}, {y}) is outside the plane of {} x {} pixels.", self.width, self.height));
        }
        Ok(())
    }

    /// C#'s `Clip`: the rectangle clipped to the plane, None when nothing is left.
    pub(crate) fn clip(&self, area: Rectangle) -> Option<(usize, usize, usize, usize)> {
        let x0 = area.x.max(0) as i64;
        let y0 = area.y.max(0) as i64;
        let x1 = (area.x as i64 + area.width.max(0) as i64).min(self.width as i64);
        let y1 = (area.y as i64 + area.height.max(0) as i64).min(self.height as i64);
        if x0 < x1 && y0 < y1 {
            Some((x0 as usize, y0 as usize, x1 as usize, y1 as usize))
        } else {
            None
        }
    }

    pub(crate) fn require_same_size(&self, other: &DrawingPlane) -> Result<(), String> {
        if other.width != self.width || other.height != self.height {
            return Err(format!("The planes must be the same size ({} x {} and {} x {}).", self.width, self.height, other.width, other.height));
        }
        Ok(())
    }
}

pub(crate) fn validate_size(width: u32, height: u32) -> Result<(), String> {
    if width < 1 || height < 1 {
        return Err(format!("A plane must be at least 1 x 1 pixels (asked for {width} x {height})."));
    }
    if width as u64 * height as u64 > MAX_LENGTH {
        return Err(format!("A plane of {width} x {height} pixels is too large; keep width x height at or below {MAX_LENGTH}."));
    }
    Ok(())
}

pub(crate) fn validate_sigma(sigma: f32) -> Result<(), String> {
    if !sigma.is_finite() {
        return Err("A blur's sigma must be a finite number of pixels.".to_string());
    }
    Ok(())
}

pub(crate) fn validate_blurs(blurs: &[(f32, f32)]) -> Result<(), String> {
    if blurs.is_empty() {
        return Err("A bloom needs at least one (sigma, weight) blur.".to_string());
    }
    blurs.iter().try_for_each(|(sigma, _)| validate_sigma(*sigma))
}

pub(crate) fn validate_exponent(exponent: f32) -> Result<(), String> {
    if !(exponent > 0.0) || exponent.is_infinite() {
        return Err("The gamma exponent must be a finite number greater than 0.".to_string());
    }
    Ok(())
}

pub(crate) fn validate_vignette(radius_x: f64, radius_y: f64) -> Result<(), String> {
    if !(radius_x > 0.0) || !(radius_y > 0.0) {
        return Err("The vignette's radii must be greater than 0 pixels.".to_string());
    }
    Ok(())
}

/// C# `Clamp`'s value rule: `!(v > min) ? min : (v > max ? max : v)`.
pub(crate) fn clamp_value(v: f32, min: f32, max: f32) -> f32 {
    if !(v > min) {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

pub(crate) fn tone_curve_value(value: f32, k: f32, gain: f32) -> f32 {
    value / (1.0 + (k * value)) * gain
}

pub(crate) fn vignette_axis(position: usize, center: f64, radius: f64) -> f64 {
    (position as f64 - center) / radius
}

pub(crate) fn vignette_factor(dx: f64, dy: f64, strength: f64, minimum: f64) -> f32 {
    let mut factor = 1.0 - (strength * ((dx * dx) + (dy * dy)));
    if factor > 1.0 {
        factor = 1.0;
    }
    if factor < minimum {
        factor = minimum;
    }
    factor as f32
}
