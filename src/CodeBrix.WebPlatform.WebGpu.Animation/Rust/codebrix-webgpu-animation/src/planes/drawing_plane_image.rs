//! `DrawingPlaneImage`: a float colour image of four planes, straight alpha (C#
//! `DrawingPlaneImage`).

use crate::planes::drawing_plane::{validate_blurs, validate_sigma};
use crate::planes::plane_kernels as k;
use crate::planes::{DrawingPlane, DrawingPlaneBloomOptions, DrawingPlaneResampling};
use crate::{Color, Rectangle};

/// A float colour image of four [`DrawingPlane`]s - R, G, B and A - for painting frames in
/// open-ended float colour (C# `DrawingPlaneImage`).
///
/// ALPHA IS STRAIGHT (not premultiplied): R, G and B hold the colour as it would look fully
/// opaque, A holds coverage (normally 0..1). A new image is transparent black. The rules:
/// - colour arithmetic (add, multiply, min, max, clamp, apply, tone curve, gamma, vignette)
///   changes R, G and B only; work on [`DrawingPlaneImage::a_mut`] to change alpha;
/// - compositing (`composite_over`, `composite_add`, `blend_color`, `blend_image`) is
///   Porter-Duff arithmetic on alpha clamped to 0..1, straight colour in and out;
/// - mixing two images (`lerp`, the transitions) and every filter that combines neighbouring
///   pixels (blur, bilinear resize, downsample, bloom) works in PREMULTIPLIED space, so
///   transparent pixels never bleed a dark fringe - unless every alpha is equal (an opaque
///   image), when the colour planes are processed directly (the same result, no rounding drift).
///
/// Same-size operations work IN PLACE and return the image; size-changing ones return a new one.
/// All arithmetic is C#'s, in C#'s order, on one thread.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingPlaneImage {
    r: DrawingPlane,
    g: DrawingPlane,
    b: DrawingPlane,
    a: DrawingPlane,
}

impl DrawingPlaneImage {
    /// A transparent black image (C# `DrawingPlaneImage(width, height)`).
    pub fn new(width: u32, height: u32) -> Result<DrawingPlaneImage, String> {
        Ok(DrawingPlaneImage { r: DrawingPlane::new(width, height)?, g: DrawingPlane::new(width, height)?, b: DrawingPlane::new(width, height)?, a: DrawingPlane::new(width, height)? })
    }

    /// An image of four planes, WITHOUT copying them (C# `DrawingPlaneImage(red, green, blue,
    /// alpha)`; the image takes the planes, [`DrawingPlaneImage::into_planes`] gives them back).
    pub fn from_planes(red: DrawingPlane, green: DrawingPlane, blue: DrawingPlane, alpha: DrawingPlane) -> Result<DrawingPlaneImage, String> {
        red.require_same_size(&green)?;
        red.require_same_size(&blue)?;
        red.require_same_size(&alpha)?;
        Ok(DrawingPlaneImage { r: red, g: green, b: blue, a: alpha })
    }

    /// The four planes, given back: (red, green, blue, alpha).
    pub fn into_planes(self) -> (DrawingPlane, DrawingPlane, DrawingPlane, DrawingPlane) {
        (self.r, self.g, self.b, self.a)
    }

    /// The red plane (C# `R`).
    pub fn r(&self) -> &DrawingPlane {
        &self.r
    }

    /// The green plane (C# `G`).
    pub fn g(&self) -> &DrawingPlane {
        &self.g
    }

    /// The blue plane (C# `B`).
    pub fn b(&self) -> &DrawingPlane {
        &self.b
    }

    /// The straight alpha plane; 0 = transparent, 1 = opaque (C# `A`).
    pub fn a(&self) -> &DrawingPlane {
        &self.a
    }

    /// The red plane, to change.
    pub fn r_mut(&mut self) -> &mut DrawingPlane {
        &mut self.r
    }

    /// The green plane, to change.
    pub fn g_mut(&mut self) -> &mut DrawingPlane {
        &mut self.g
    }

    /// The blue plane, to change.
    pub fn b_mut(&mut self) -> &mut DrawingPlane {
        &mut self.b
    }

    /// The alpha plane, to change.
    pub fn a_mut(&mut self) -> &mut DrawingPlane {
        &mut self.a
    }

    /// The width in pixels (C# `Width`).
    pub fn width(&self) -> u32 {
        self.r.width()
    }

    /// The height in pixels (C# `Height`).
    pub fn height(&self) -> u32 {
        self.r.height()
    }

    /// The size in pixels, (width, height) (C# `Size`).
    pub fn size(&self) -> (u32, u32) {
        self.r.size()
    }

    /// One pixel as [red, green, blue, alpha], straight alpha (C# `GetPixel`, a `Vector4`).
    pub fn get_pixel(&self, x: u32, y: u32) -> Result<[f32; 4], String> {
        Ok([self.r.get(x, y)?, self.g.get(x, y)?, self.b.get(x, y)?, self.a.get(x, y)?])
    }

    /// Writes one pixel from [red, green, blue, alpha], straight alpha (C# `SetPixel`).
    pub fn set_pixel(&mut self, x: u32, y: u32, value: [f32; 4]) -> Result<(), String> {
        self.r.set(x, y, value[0])?;
        self.g.set(x, y, value[1])?;
        self.b.set(x, y, value[2])?;
        self.a.set(x, y, value[3])
    }

    /// Copies every value into another image of the same size (C# `CopyTo`).
    pub fn copy_to(&self, destination: &mut DrawingPlaneImage) -> Result<(), String> {
        self.require_same_size(destination)?;
        self.r.copy_to(&mut destination.r)?;
        self.g.copy_to(&mut destination.g)?;
        self.b.copy_to(&mut destination.b)?;
        self.a.copy_to(&mut destination.a)
    }

    // ---- Filling ------------------------------------------------------------------------------

    /// Makes the whole image transparent black (C# `Clear`).
    pub fn clear(&mut self) -> &mut DrawingPlaneImage {
        self.r.clear();
        self.g.clear();
        self.b.clear();
        self.a.clear();
        self
    }

    /// Fills with a colour, each 8-bit channel `byte / 255f`, alpha included (C# `Fill(Color)`).
    pub fn fill_color(&mut self, color: Color) -> &mut DrawingPlaneImage {
        let c = to_vector(color);
        self.fill(c[0], c[1], c[2], Some(c[3]))
    }

    /// Fills with float channels, straight alpha, open-ended (C# `Fill(red, green, blue, alpha =
    /// 1)`).
    pub fn fill(&mut self, red: f32, green: f32, blue: f32, alpha: Option<f32>) -> &mut DrawingPlaneImage {
        self.r.fill(red);
        self.g.fill(green);
        self.b.fill(blue);
        self.a.fill(alpha.unwrap_or(1.0));
        self
    }

    /// Fills a rectangle (clipped) with a colour (C# `Fill(Rectangle, Color)`).
    pub fn fill_area(&mut self, area: Rectangle, color: Color) -> &mut DrawingPlaneImage {
        let c = to_vector(color);
        self.r.fill_area(area, c[0]);
        self.g.fill_area(area, c[1]);
        self.b.fill_area(area, c[2]);
        self.a.fill_area(area, c[3]);
        self
    }

    /// Sets every pixel from a function of its position returning [r, g, b, a], row by row (C#
    /// `Fill(Func)` and `FillParallel`, here on one thread).
    pub fn fill_with(&mut self, function: impl FnMut(u32, u32) -> [f32; 4]) -> &mut DrawingPlaneImage {
        let all = Rectangle::new(0, 0, self.width() as i32, self.height() as i32);
        self.fill_area_with(all, function)
    }

    /// Sets every pixel inside a rectangle (clipped) from a function of its ABSOLUTE position (C#
    /// `Fill(Rectangle, Func)` / `FillParallel(Rectangle, Func)`).
    pub fn fill_area_with(&mut self, area: Rectangle, mut function: impl FnMut(u32, u32) -> [f32; 4]) -> &mut DrawingPlaneImage {
        if let Some((x0, y0, x1, y1)) = self.r.clip(area) {
            let width = self.width() as usize;
            for y in y0..y1 {
                for x in x0..x1 {
                    let v = function(x as u32, y as u32);
                    let i = (y * width) + x;
                    self.r.data_mut()[i] = v[0];
                    self.g.data_mut()[i] = v[1];
                    self.b.data_mut()[i] = v[2];
                    self.a.data_mut()[i] = v[3];
                }
            }
        }
        self
    }

    // ---- Colour arithmetic (R, G, B only) -------------------------------------------------------

    /// `C = C + other.C * scale` for R, G and B; both alphas ignored (C# `Add(DrawingPlaneImage,
    /// scale = 1)` - light arithmetic; [`DrawingPlaneImage::composite_add`] respects alpha).
    pub fn add_image(&mut self, other: &DrawingPlaneImage, scale: Option<f32>) -> Result<&mut DrawingPlaneImage, String> {
        self.require_same_size(other)?;
        self.r.add_plane(&other.r, scale)?;
        self.g.add_plane(&other.g, scale)?;
        self.b.add_plane(&other.b, scale)?;
        Ok(self)
    }

    /// Adds constants to the colour channels (C# `Add(red, green, blue)`).
    pub fn add(&mut self, red: f32, green: f32, blue: f32) -> &mut DrawingPlaneImage {
        self.r.add(red);
        self.g.add(green);
        self.b.add(blue);
        self
    }

    /// Scales the colour channels by one factor (C# `Multiply(float)`).
    pub fn multiply(&mut self, factor: f32) -> &mut DrawingPlaneImage {
        self.multiply_rgb(factor, factor, factor)
    }

    /// Scales each colour channel by its own factor - a tint (C# `Multiply(red, green, blue)`).
    pub fn multiply_rgb(&mut self, red: f32, green: f32, blue: f32) -> &mut DrawingPlaneImage {
        self.r.multiply(red);
        self.g.multiply(green);
        self.b.multiply(blue);
        self
    }

    /// Multiplies the colour channels by a plane - a vignette, a texture, a shading plane (C#
    /// `Multiply(DrawingPlane)`; to mask coverage multiply the alpha plane instead).
    pub fn multiply_plane(&mut self, factor: &DrawingPlane) -> Result<&mut DrawingPlaneImage, String> {
        self.r.require_same_size(factor)?;
        self.r.multiply_plane(factor)?;
        self.g.multiply_plane(factor)?;
        self.b.multiply_plane(factor)?;
        Ok(self)
    }

    /// Multiplies the colour channels by another image's (C# `Multiply(DrawingPlaneImage)`).
    pub fn multiply_image(&mut self, other: &DrawingPlaneImage) -> Result<&mut DrawingPlaneImage, String> {
        self.require_same_size(other)?;
        self.r.multiply_plane(&other.r)?;
        self.g.multiply_plane(&other.g)?;
        self.b.multiply_plane(&other.b)?;
        Ok(self)
    }

    /// Caps the colour channels (C# `Min(float)`).
    pub fn min(&mut self, value: f32) -> &mut DrawingPlaneImage {
        self.r.min(value);
        self.g.min(value);
        self.b.min(value);
        self
    }

    /// Raises the colour channels to at least `value` (C# `Max(float)`).
    pub fn max(&mut self, value: f32) -> &mut DrawingPlaneImage {
        self.r.max(value);
        self.g.max(value);
        self.b.max(value);
        self
    }

    /// Limits the colour channels to min..max, NaN to min (C# `Clamp`).
    pub fn clamp(&mut self, min: f32, max: f32) -> Result<&mut DrawingPlaneImage, String> {
        self.r.clamp(min, max)?;
        self.g.clamp(min, max)?;
        self.b.clamp(min, max)?;
        Ok(self)
    }

    /// One per-value function on R, then G, then B (C# `Apply(Func)`).
    pub fn apply(&mut self, mut function: impl FnMut(f32) -> f32) -> &mut DrawingPlaneImage {
        self.r.apply(&mut function);
        self.g.apply(&mut function);
        self.b.apply(&mut function);
        self
    }

    /// A function per colour channel - a per-channel tone curve (C# `Apply(red, green, blue)`).
    pub fn apply_rgb(&mut self, red: impl FnMut(f32) -> f32, green: impl FnMut(f32) -> f32, blue: impl FnMut(f32) -> f32) -> &mut DrawingPlaneImage {
        self.r.apply(red);
        self.g.apply(green);
        self.b.apply(blue);
        self
    }

    /// The tone curve `v / (1 + k v) * gain` on R, G and B (C# `ApplyToneCurve`).
    pub fn apply_tone_curve(&mut self, k: f32, gain: f32) -> &mut DrawingPlaneImage {
        self.r.apply_tone_curve(k, gain);
        self.g.apply_tone_curve(k, gain);
        self.b.apply_tone_curve(k, gain);
        self
    }

    /// Raises positive R, G and B values to a power (C# `ApplyGamma`).
    pub fn apply_gamma(&mut self, exponent: f32) -> Result<&mut DrawingPlaneImage, String> {
        self.r.apply_gamma(exponent)?;
        self.g.apply_gamma(exponent)?;
        self.b.apply_gamma(exponent)?;
        Ok(self)
    }

    /// Darkens R, G and B towards the edges with the factor of [`DrawingPlane::create_vignette`]
    /// (C# `ApplyVignette`: it builds that plane and multiplies by it).
    pub fn apply_vignette(&mut self, center_x: f64, center_y: f64, radius_x: f64, radius_y: f64, strength: f64, minimum: Option<f64>) -> Result<&mut DrawingPlaneImage, String> {
        let vignette = DrawingPlane::create_vignette(self.width(), self.height(), center_x, center_y, radius_x, radius_y, strength, minimum)?;
        self.multiply_plane(&vignette)
    }

    // ---- Mixing and compositing ---------------------------------------------------------------

    /// Mixes this image towards another by a fraction, in premultiplied space (C# `Lerp(image,
    /// t)`): with `s = 1 - t`, equal alphas give `C = Ca s + Cb t` with alpha kept; otherwise `A
    /// = Aa s + Ab t`, `C = (Ca Aa s + Cb Ab t) / A` (zeros when A is 0). t at or below 0 keeps
    /// this image, at or above 1 copies `other` exactly.
    pub fn lerp(&mut self, other: &DrawingPlaneImage, t: f32) -> Result<&mut DrawingPlaneImage, String> {
        self.require_same_size(other)?;
        mix_in_place(self, other, t, None);
        Ok(self)
    }

    /// The same mix with a per-pixel fraction from a mask (C# `Lerp(image, DrawingPlane)`).
    pub fn lerp_mask(&mut self, other: &DrawingPlaneImage, mask: &DrawingPlane) -> Result<&mut DrawingPlaneImage, String> {
        self.require_same_size(other)?;
        self.r.require_same_size(mask)?;
        mix_in_place(self, other, 0.0, Some(mask));
        Ok(self)
    }

    /// Composites another image OVER this one (Porter-Duff source-over) with an overall opacity
    /// (C# `CompositeOver(source, opacity = 1)`): `as = clamp(source.A * opacity)`, `ad =
    /// clamp(A)`, `Ao = as + ad (1 - as)` (as `ad + as (1 - ad)` when ad is the larger), `C = (Cs
    /// as + Cd ad (1 - as)) / Ao`; as = 0 changes nothing, as = 1 or ad = 0 copies the source.
    pub fn composite_over(&mut self, source: &DrawingPlaneImage, opacity: Option<f32>) -> Result<&mut DrawingPlaneImage, String> {
        self.require_same_size(source)?;
        let opacity = opacity.unwrap_or(1.0);
        let (sr, sg, sb, sa) = (source.r.data(), source.g.data(), source.b.data(), source.a.data());
        let mut d = Channels::of(self);
        for i in 0..sr.len() {
            d.over(i, [sr[i], sg[i], sb[i]], sa[i] * opacity);
        }
        Ok(self)
    }

    /// Composites another image ADDITIVELY (Porter-Duff plus) with an overall opacity (C#
    /// `CompositeAdd(source, opacity = 1)`): `Ao = min(1, as + ad)`, `C = (Cs as + Cd ad) / Ao`;
    /// as = 0 changes nothing, ad = 0 copies the source colour. Use it for glows.
    pub fn composite_add(&mut self, source: &DrawingPlaneImage, opacity: Option<f32>) -> Result<&mut DrawingPlaneImage, String> {
        self.require_same_size(source)?;
        let opacity = opacity.unwrap_or(1.0);
        let (sr, sg, sb, sa) = (source.r.data(), source.g.data(), source.b.data(), source.a.data());
        let c = Channels::of(self);
        for i in 0..sr.len() {
            let a = k::clamp01(sa[i] * opacity);
            if !(a > 0.0) {
                continue;
            }
            let d = k::clamp01(c.a[i]);
            if !(d > 0.0) {
                (c.r[i], c.g[i], c.b[i], c.a[i]) = (sr[i], sg[i], sb[i], a);
                continue;
            }
            let mut ao = a + d;
            if ao > 1.0 {
                ao = 1.0;
            }
            c.r[i] = ((sr[i] * a) + (c.r[i] * d)) / ao;
            c.g[i] = ((sg[i] * a) + (c.g[i] * d)) / ao;
            c.b[i] = ((sb[i] * a) + (c.b[i] * d)) / ao;
            c.a[i] = ao;
        }
        Ok(self)
    }

    /// Paints a colour through a coverage mask: source-over of the colour with alpha `color.a *
    /// mask` (C# `BlendColor(Color, DrawingPlane)`; channels `byte / 255f`).
    pub fn blend_color(&mut self, color: Color, mask: &DrawingPlane) -> Result<&mut DrawingPlaneImage, String> {
        let c = to_vector(color);
        self.blend_color_values(c[0], c[1], c[2], c[3], mask)
    }

    /// Paints a float colour through a coverage mask (C# `BlendColor(red, green, blue, alpha,
    /// mask)`).
    pub fn blend_color_values(&mut self, red: f32, green: f32, blue: f32, alpha: f32, mask: &DrawingPlane) -> Result<&mut DrawingPlaneImage, String> {
        self.r.require_same_size(mask)?;
        let m = mask.data();
        let mut d = Channels::of(self);
        for (i, coverage) in m.iter().enumerate() {
            d.over(i, [red, green, blue], alpha * *coverage);
        }
        Ok(self)
    }

    /// Paints another image through a coverage mask: source-over with alpha `source.A * mask` (C#
    /// `BlendImage`).
    pub fn blend_image(&mut self, source: &DrawingPlaneImage, mask: &DrawingPlane) -> Result<&mut DrawingPlaneImage, String> {
        self.require_same_size(source)?;
        self.r.require_same_size(mask)?;
        let (sr, sg, sb, sa, m) = (source.r.data(), source.g.data(), source.b.data(), source.a.data(), mask.data());
        let mut d = Channels::of(self);
        for i in 0..sr.len() {
            d.over(i, [sr[i], sg[i], sb[i]], sa[i] * m[i]);
        }
        Ok(self)
    }

    // ---- Filters ---------------------------------------------------------------------------------

    /// Blurs with the Gaussian of [`DrawingPlane::gaussian_blur`], in place (C# `GaussianBlur`):
    /// uniform alpha blurs the colour planes and keeps alpha; otherwise C*A and A are blurred and
    /// colour becomes blurred(C*A) / blurred(A) (0 where that alpha is 0).
    pub fn gaussian_blur(&mut self, sigma: f32) -> Result<&mut DrawingPlaneImage, String> {
        validate_sigma(sigma)?;
        if !(sigma > 0.0) {
            return Ok(self);
        }
        if self.has_uniform_alpha() {
            self.r.gaussian_blur(sigma)?;
            self.g.gaussian_blur(sigma)?;
            self.b.gaussian_blur(sigma)?;
            return Ok(self);
        }
        premultiply(self);
        self.r.gaussian_blur(sigma)?;
        self.g.gaussian_blur(sigma)?;
        self.b.gaussian_blur(sigma)?;
        self.a.gaussian_blur(sigma)?;
        unpremultiply(self);
        Ok(self)
    }

    /// A resized copy (C# `Resized`; bilinear by default): nearest copies all four channels;
    /// bilinear works premultiplied unless every alpha is equal.
    pub fn resized(&self, width: u32, height: u32, resampling: Option<DrawingPlaneResampling>) -> Result<DrawingPlaneImage, String> {
        if resampling == Some(DrawingPlaneResampling::Nearest) {
            return DrawingPlaneImage::from_planes(self.r.resized(width, height, resampling)?, self.g.resized(width, height, resampling)?, self.b.resized(width, height, resampling)?, self.a.resized(width, height, resampling)?);
        }
        self.spatial(|p| p.resized(width, height, resampling))
    }

    /// A copy shrunk by a whole factor with the box filter of [`DrawingPlane::downsampled`]
    /// (premultiplied unless every alpha is equal) (C# `Downsampled`).
    pub fn downsampled(&self, factor: u32) -> Result<DrawingPlaneImage, String> {
        if factor < 1 {
            return Err("The downsample factor must be 1 or more.".to_string());
        }
        self.spatial(|p| p.downsampled(factor))
    }

    /// A glow image with the steps of [`DrawingPlane::create_bloom`] (C# `CreateBloom`). Uniform
    /// alpha blooms only the colour planes and keeps that alpha (add it with
    /// [`DrawingPlaneImage::add_image`]); otherwise colour (with the gain) and alpha (without)
    /// are bloomed premultiplied, giving a straight-alpha glow to composite with
    /// [`DrawingPlaneImage::composite_add`] or [`DrawingPlaneImage::composite_over`].
    pub fn create_bloom(&self, downsample_factor: u32, blurs: &[(f32, f32)], options: DrawingPlaneBloomOptions) -> Result<DrawingPlaneImage, String> {
        validate_blurs(blurs)?;
        if self.has_uniform_alpha() {
            let (w, h) = options.output_size.unwrap_or(self.size());
            let alpha = self.a.data()[0];
            return DrawingPlaneImage::from_planes(
                self.r.create_bloom(downsample_factor, blurs, options)?,
                self.g.create_bloom(downsample_factor, blurs, options)?,
                self.b.create_bloom(downsample_factor, blurs, options)?,
                DrawingPlane::with_value(w, h, alpha)?,
            );
        }
        let mut premultiplied = self.clone();
        premultiply(&mut premultiplied);
        let alpha_options = DrawingPlaneBloomOptions { gain: 1.0, ..options };
        let mut result = DrawingPlaneImage::from_planes(
            premultiplied.r.create_bloom(downsample_factor, blurs, options)?,
            premultiplied.g.create_bloom(downsample_factor, blurs, options)?,
            premultiplied.b.create_bloom(downsample_factor, blurs, options)?,
            premultiplied.a.create_bloom(downsample_factor, blurs, alpha_options)?,
        )?;
        unpremultiply(&mut result);
        Ok(result)
    }

    // ---- 8-bit conversion (in-memory pixel access only: no file encoders) ------------------------

    /// Writes tightly packed 8-bit RGBA, STRAIGHT alpha, row-major, with the rounding rule of
    /// [`DrawingPlane::to_bytes`] (C# `ToRgba(Span)`).
    pub fn to_rgba_into(&self, destination: &mut [u8]) -> Result<(), String> {
        let count = self.r.length();
        if destination.len() < count * 4 {
            return Err("The destination must hold at least width x height x 4 bytes.".to_string());
        }
        for i in 0..count {
            let o = i * 4;
            destination[o] = k::unit_to_byte(self.r.data()[i]);
            destination[o + 1] = k::unit_to_byte(self.g.data()[i]);
            destination[o + 2] = k::unit_to_byte(self.b.data()[i]);
            destination[o + 3] = k::unit_to_byte(self.a.data()[i]);
        }
        Ok(())
    }

    /// The image as tightly packed 8-bit straight-alpha RGBA (C# `ToRgba()`).
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; self.r.length() * 4];
        for i in 0..self.r.length() {
            let o = i * 4;
            bytes[o] = k::unit_to_byte(self.r.data()[i]);
            bytes[o + 1] = k::unit_to_byte(self.g.data()[i]);
            bytes[o + 2] = k::unit_to_byte(self.b.data()[i]);
            bytes[o + 3] = k::unit_to_byte(self.a.data()[i]);
        }
        bytes
    }

    /// Overwrites the image from tightly packed 8-bit straight-alpha RGBA, each channel `byte /
    /// 255f` - every frame round-trips exactly (C# `CopyFromRgba`).
    pub fn copy_from_rgba(&mut self, rgba: &[u8]) -> Result<(), String> {
        let count = self.r.length();
        if rgba.len() < count * 4 {
            return Err("There must be at least width x height x 4 bytes.".to_string());
        }
        for i in 0..count {
            let o = i * 4;
            self.r.data_mut()[i] = k::byte_to_unit(rgba[o]);
            self.g.data_mut()[i] = k::byte_to_unit(rgba[o + 1]);
            self.b.data_mut()[i] = k::byte_to_unit(rgba[o + 2]);
            self.a.data_mut()[i] = k::byte_to_unit(rgba[o + 3]);
        }
        Ok(())
    }

    /// An image from tightly packed 8-bit straight-alpha RGBA (C# `FromRgba`).
    pub fn from_rgba(rgba: &[u8], width: u32, height: u32) -> Result<DrawingPlaneImage, String> {
        let mut image = DrawingPlaneImage::new(width, height)?;
        image.copy_from_rgba(rgba)?;
        Ok(image)
    }

    // ---- Internals -------------------------------------------------------------------------------

    pub(crate) fn require_same_size(&self, other: &DrawingPlaneImage) -> Result<(), String> {
        if other.width() != self.width() || other.height() != self.height() {
            return Err(format!("The images must be the same size ({} x {} and {} x {}).", self.width(), self.height(), other.width(), other.height()));
        }
        Ok(())
    }

    /// C#'s `HasUniformAlpha`: every alpha equal to the first, which is not NaN.
    pub(crate) fn has_uniform_alpha(&self) -> bool {
        let a = self.a.data();
        let first = a[0];
        for v in &a[1..] {
            if *v != first {
                return false;
            }
        }
        !first.is_nan()
    }

    fn spatial(&self, filter: impl Fn(&DrawingPlane) -> Result<DrawingPlane, String>) -> Result<DrawingPlaneImage, String> {
        if self.has_uniform_alpha() {
            let r = filter(&self.r)?;
            let (w, h) = r.size();
            let (g, b) = (filter(&self.g)?, filter(&self.b)?);
            return DrawingPlaneImage::from_planes(r, g, b, DrawingPlane::with_value(w, h, self.a.data()[0])?);
        }
        let mut premultiplied = self.clone();
        premultiply(&mut premultiplied);
        let mut result = DrawingPlaneImage::from_planes(filter(&premultiplied.r)?, filter(&premultiplied.g)?, filter(&premultiplied.b)?, filter(&premultiplied.a)?)?;
        unpremultiply(&mut result);
        Ok(result)
    }
}

/// A colour's 8-bit channels as `byte / 255f` (C#'s internal `ToVector`).
pub(crate) fn to_vector(color: Color) -> [f32; 4] {
    [k::byte_to_unit(color.r), k::byte_to_unit(color.g), k::byte_to_unit(color.b), k::byte_to_unit(color.a)]
}

/// The four channels of an image as slices to change at once.
struct Channels<'a> {
    r: &'a mut [f32],
    g: &'a mut [f32],
    b: &'a mut [f32],
    a: &'a mut [f32],
}

impl<'a> Channels<'a> {
    fn of(image: &'a mut DrawingPlaneImage) -> Channels<'a> {
        Channels { r: image.r.data_mut(), g: image.g.data_mut(), b: image.b.data_mut(), a: image.a.data_mut() }
    }

    /// Porter-Duff source-over of one straight-alpha source pixel onto pixel i (C#'s `Over`).
    fn over(&mut self, i: usize, s: [f32; 3], source_alpha: f32) {
        let a = k::clamp01(source_alpha);
        if !(a > 0.0) {
            return;
        }
        let d = k::clamp01(self.a[i]);
        if a >= 1.0 || !(d > 0.0) {
            (self.r[i], self.g[i], self.b[i], self.a[i]) = (s[0], s[1], s[2], a);
            return;
        }
        let rest = d * (1.0 - a);
        let ao = if d >= a { d + (a * (1.0 - d)) } else { a + rest };
        self.r[i] = ((s[0] * a) + (self.r[i] * rest)) / ao;
        self.g[i] = ((s[1] * a) + (self.g[i] * rest)) / ao;
        self.b[i] = ((s[2] * a) + (self.b[i] * rest)) / ao;
        self.a[i] = ao;
    }
}

/// C#'s `MixInto(a, b, destination = a, t, mask)`.
pub(crate) fn mix_in_place(a: &mut DrawingPlaneImage, b: &DrawingPlaneImage, t: f32, mask: Option<&DrawingPlane>) {
    let (br, bg, bb, ba) = (b.r.data(), b.g.data(), b.b.data(), b.a.data());
    let d = Channels::of(a);
    for i in 0..br.len() {
        let tt = mask.map_or(t, |m| m.data()[i]);
        let v = k::mix_unit(d.r[i], d.g[i], d.b[i], d.a[i], br[i], bg[i], bb[i], ba[i], tt);
        (d.r[i], d.g[i], d.b[i], d.a[i]) = (v[0], v[1], v[2], v[3]);
    }
}

/// C#'s `MixInto(a, b, destination, t, mask)` with a destination of its own.
pub(crate) fn mix_to(a: &DrawingPlaneImage, b: &DrawingPlaneImage, destination: &mut DrawingPlaneImage, t: f32, mask: Option<&DrawingPlane>) {
    let (ar, ag, ab, aa) = (a.r.data(), a.g.data(), a.b.data(), a.a.data());
    let (br, bg, bb, ba) = (b.r.data(), b.g.data(), b.b.data(), b.a.data());
    let d = Channels::of(destination);
    for i in 0..ar.len() {
        let tt = mask.map_or(t, |m| m.data()[i]);
        let v = k::mix_unit(ar[i], ag[i], ab[i], aa[i], br[i], bg[i], bb[i], ba[i], tt);
        (d.r[i], d.g[i], d.b[i], d.a[i]) = (v[0], v[1], v[2], v[3]);
    }
}

fn premultiply(image: &mut DrawingPlaneImage) {
    let c = Channels::of(image);
    for i in 0..c.a.len() {
        let a = c.a[i];
        c.r[i] *= a;
        c.g[i] *= a;
        c.b[i] *= a;
    }
}

fn unpremultiply(image: &mut DrawingPlaneImage) {
    let c = Channels::of(image);
    for i in 0..c.a.len() {
        let alpha = c.a[i];
        if alpha == 0.0 {
            (c.r[i], c.g[i], c.b[i]) = (0.0, 0.0, 0.0);
        } else {
            c.r[i] /= alpha;
            c.g[i] /= alpha;
            c.b[i] /= alpha;
        }
    }
}
