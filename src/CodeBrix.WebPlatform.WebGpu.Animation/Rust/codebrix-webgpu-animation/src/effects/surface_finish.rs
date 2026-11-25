//! `SurfaceFinish` and `CanvasWeave`: a layer drawn as a painted SURFACE - the canvas's weave and
//! the thickness of the paint, lit from a chosen direction (no C# equivalent).
//!
//! HOW IT IS DRAWN: a finished layer is drawn through its effect group (the group its effects and
//! frame operations ride on - ONE nesting level, shared with them; a layer that has one already
//! takes no further level). Just before the group's composite, every stroke set on the layer
//! ([`BrushStrokesShape`](crate::shapes::BrushStrokesShape)) is drawn a second time into the
//! layer's HEIGHT RECORD (a canvas-size `R16Float` texture, single-sampled, additively: the tip's
//! height x coverage x load x opacity x `impasto` - paint piles up where strokes overlap). The
//! composite then lights the layer's colour: height = paint + weave x (1 - smoothstep(0, 0.6,
//! paint)) (thick paint hides the canvas), the weave an analytic function of the pixel's position
//! in the LAYER's coordinates (it stays put on the painting and scales with zoom), the normal from
//! central differences of that height one device pixel either side (in layer units), the shading
//! `1 + relief x (dot(n, l) - dot(n_flat, l))` (flat areas unchanged) and a small Blinn highlight
//! on the paint, applied to the straight LINEAR colour before the frame operations.
//!
//! LIMITS: only stroke sets have thickness - vector shapes, pictures and ownership maps on a
//! finished layer show only the weave; the height pass ignores clips and masks (a stroke clipped
//! by a mask still raises relief at the mask's edge - the composite shows the finish only where
//! the layer drew); a finished layer inside a wipe inside a blended session uses all three
//! nesting levels (any further group inside it is drawn as a clip and counted `unsupported`);
//! the line and flat looks and the effects switch off (`SessionRenderOptions`) draw no finish;
//! the weave fades out where its threads are narrower than about two device pixels (it would
//! alias).

use kurbo::Affine;

use crate::rendering::core_fx_pass_data as FxPassData;

/// A layer's surface finish (no C# equivalent): the light, how strongly relief shades, how thick
/// the paint stands, the gloss of its ridges and the canvas's weave. Set it with
/// [`DrawingLayer::set_surface_finish`](crate::models::DrawingLayer::set_surface_finish). The
/// defaults: light from the upper left (225 degrees) at 35 degrees, relief 0.6, impasto 1, gloss
/// 0.15, the default weave.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceFinish {
    /// The direction the light comes FROM, degrees clockwise on screen from +x (225: the upper
    /// left).
    pub light_angle: f32,
    /// The light's height above the canvas, degrees (above 0, at most 90: straight above). 35.
    pub light_elevation: f32,
    /// How strongly the relief shades the colour (0: not at all). 0.6.
    pub relief: f32,
    /// The paint's thickness scale: 0 a smooth glaze (no paint relief), 1 thick; above 1 thicker
    /// still. 1.
    pub impasto: f32,
    /// The highlight on paint ridges, 0..1. 0.15.
    pub gloss: f32,
    /// The canvas's weave, or None for a smooth ground.
    pub weave: Option<CanvasWeave>,
}

impl Default for SurfaceFinish {
    fn default() -> SurfaceFinish {
        SurfaceFinish { light_angle: 225.0, light_elevation: 35.0, relief: 0.6, impasto: 1.0, gloss: 0.15, weave: Some(CanvasWeave::default()) }
    }
}

/// A canvas's weave (no C# equivalent): two crossed families of threads (a plain weave, over and
/// under) in the layer's coordinates. The defaults: spacing 3, depth 0.35, irregularity 0.3,
/// angle 0, seed 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasWeave {
    /// Layer units (calibrated units) from one thread to the next (above 0). 3.
    pub spacing: f32,
    /// The threads' height, in the paint's height units (a stroke at load 1 and impasto 1 is 1
    /// high). 0.35.
    pub depth: f32,
    /// How unevenly the threads run and how their thickness varies, 0..1. 0.3.
    pub irregularity: f32,
    /// The weave turned by this many degrees (clockwise). 0.
    pub angle: f32,
    /// The irregularity's seed. 0.
    pub seed: u32,
}

impl Default for CanvasWeave {
    fn default() -> CanvasWeave {
        CanvasWeave { spacing: 3.0, depth: 0.35, irregularity: 0.3, angle: 0.0, seed: 0 }
    }
}

impl SurfaceFinish {
    /// Checks the finish; the error says what to change.
    pub fn validate(&self) -> Result<(), String> {
        if !self.light_angle.is_finite() {
            return Err("A surface finish's light angle must be finite degrees (225: from the upper left).".to_string());
        }
        if !(self.light_elevation > 0.0 && self.light_elevation <= 90.0) {
            return Err("A surface finish's light elevation must be above 0 and at most 90 degrees (90: straight above the canvas).".to_string());
        }
        if !(self.relief >= 0.0 && self.relief.is_finite()) {
            return Err("A surface finish's relief must be finite and not negative (0: no shading).".to_string());
        }
        if !(self.impasto >= 0.0 && self.impasto.is_finite()) {
            return Err("A surface finish's impasto must be finite and not negative (0: a smooth glaze, 1: thick paint).".to_string());
        }
        if !(self.gloss >= 0.0 && self.gloss <= 1.0) {
            return Err("A surface finish's gloss must be a fraction, 0..1.".to_string());
        }
        if let Some(w) = &self.weave {
            if !(w.spacing > 0.0 && w.spacing.is_finite()) {
                return Err("A canvas weave's spacing must be finite layer units above 0.".to_string());
            }
            if !(w.depth >= 0.0 && w.depth.is_finite()) {
                return Err("A canvas weave's depth must be finite and not negative.".to_string());
            }
            if !(w.irregularity >= 0.0 && w.irregularity <= 1.0) {
                return Err("A canvas weave's irregularity must be a fraction, 0..1.".to_string());
            }
            if !w.angle.is_finite() {
                return Err("A canvas weave's angle must be finite degrees.".to_string());
            }
        }
        Ok(())
    }

    /// The unit vector TOWARDS the light, screen axes (x right, y down, z out of the canvas).
    pub(crate) fn light_vector(&self) -> [f32; 3] {
        let (az, el) = ((self.light_angle as f64).to_radians(), (self.light_elevation as f64).to_radians());
        [(el.cos() * az.cos()) as f32, (el.cos() * az.sin()) as f32, el.sin() as f32]
    }
}

/// A finish as the renderer takes it for one layer this frame (crate-internal): the light, the
/// amounts, the weave, and the layer's device-to-layer matrix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FinishParams {
    pub(crate) light: [f32; 3],
    pub(crate) relief: f32,
    pub(crate) gloss: f32,
    /// The height pass's scale (written into each stroke draw's block).
    pub(crate) impasto: f32,
    pub(crate) weave: Option<CanvasWeave>,
    /// Device pixels -> layer units: a b c d e f (x' = a x + c y + e).
    pub(crate) inverse: [f32; 6],
    /// The layer's device pixels per layer unit (the square root of its matrix's determinant).
    pub(crate) pixels_per_unit: f32,
}

/// The device form of a layer's finish under `layer_transform` (layer units -> device pixels);
/// None when the transform cannot be inverted (nothing is drawn then).
pub(crate) fn finish_params(finish: &SurfaceFinish, layer_transform: Affine) -> Option<FinishParams> {
    let det = layer_transform.determinant();
    if !(det.abs() > 1e-12) || !det.is_finite() {
        return None;
    }
    let inverse = layer_transform.inverse().as_coeffs().map(|v| v as f32);
    Some(FinishParams { light: finish.light_vector(), relief: finish.relief, gloss: finish.gloss, impasto: finish.impasto, weave: finish.weave, inverse, pixels_per_unit: det.abs().sqrt() as f32 })
}

/// The finish's words of the effect composite's pass data (v10 to v13, u2; zero without one).
pub(crate) fn finish_pass_data(finish: Option<&FinishParams>) -> FxPassData {
    let Some(f) = finish else {
        return FxPassData::default();
    };
    let (flags, w) = match f.weave {
        Some(w) => (3u32, w),
        None => (1u32, CanvasWeave { depth: 0.0, ..CanvasWeave::default() }),
    };
    let m = f.inverse;
    FxPassData {
        v10: [f.light[0], f.light[1], f.light[2], f.relief],
        v11: [f.gloss, w.depth, w.spacing, w.irregularity],
        v12: [m[0], m[1], m[2], m[3]],
        v13: [m[4], m[5], (w.angle as f64).to_radians() as f32, f.pixels_per_unit],
        u2: [flags, w.seed, 0, 0],
        ..FxPassData::default()
    }
}

// ---- CPU mirrors of effects.wgsl's finish (tests; keep them equal to the shader) -------------

#[cfg(test)]
/// effects.wgsl weave_noise: value noise in -1..1 over the integer lattice (the renderer's 32-bit
/// hash at the corners, smoothstep between them).
pub(crate) fn weave_noise(seed: u32, x: f32, y: f32) -> f32 {
    use crate::painting::color_rules::noise_hash;
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (x - ix, y - iy);
    let corner = |dx: i32, dy: i32| noise_hash(seed, ix as i32 + dx, iy as i32 + dy, 0) as f32 / 4_294_967_295.0 * 2.0 - 1.0;
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let top = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * sx;
    let bottom = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * sx;
    top + (bottom - top) * sy
}

#[cfg(test)]
/// effects.wgsl weave_height: the weave's height (0..depth, before the thickness variation) at
/// a point in LAYER units: the point turned by -angle into thread units (one per spacing),
/// wobbled by the irregularity, then a plain weave - warp |sin(pi x)| over in one cell, weft
/// |sin(pi y)| over in the next (warp = |a|/2 + ab/2, weft = |b|/2 - ab/2, a = sin(pi x),
/// b = sin(pi y)), the higher of the two, its thickness varied by the irregularity.
pub(crate) fn weave_height(w: &CanvasWeave, u: f32, v: f32) -> f32 {
    let angle = (w.angle as f64).to_radians() as f32;
    let (c, s) = (angle.cos(), angle.sin());
    let x0 = (c * u + s * v) / w.spacing;
    let y0 = (-s * u + c * v) / w.spacing;
    let x = x0 + w.irregularity * 0.3 * weave_noise(w.seed, x0 * 0.25, y0 * 0.25);
    let y = y0 + w.irregularity * 0.3 * weave_noise(w.seed.wrapping_add(1), x0 * 0.25, y0 * 0.25);
    let a = (std::f32::consts::PI * x).sin();
    let b = (std::f32::consts::PI * y).sin();
    let warp = 0.5 * a.abs() + 0.5 * a * b;
    let weft = 0.5 * b.abs() - 0.5 * a * b;
    let thickness = 1.0 + w.irregularity * 0.25 * weave_noise(w.seed.wrapping_add(2), x * 0.5, y * 0.5);
    w.depth * warp.max(weft) * thickness
}

#[cfg(test)]
/// effects.wgsl finish_shade: the factor a colour is multiplied by and the highlight added to it
/// for a surface whose height rises by `gradient` (height units per layer unit along screen x and
/// y) with `paint` of paint there: `1 + relief (dot(n, l) - l.z)` and the Blinn highlight above a
/// flat surface's, on paint only.
pub(crate) fn finish_shade(light: [f32; 3], relief: f32, gloss: f32, gradient: [f32; 2], paint: f32) -> (f32, f32) {
    let n = {
        let (x, y, z) = (-gradient[0], -gradient[1], 1.0f32);
        let l = (x * x + y * y + z * z).sqrt();
        [x / l, y / l, z / l]
    };
    let dot = n[0] * light[0] + n[1] * light[1] + n[2] * light[2];
    let shade = (1.0 + relief * (dot - light[2])).max(0.0);
    let h = {
        let (x, y, z) = (light[0], light[1], light[2] + 1.0);
        let l = (x * x + y * y + z * z).sqrt();
        [x / l, y / l, z / l]
    };
    let nh = (n[0] * h[0] + n[1] * h[1] + n[2] * h[2]).max(0.0);
    let on_paint = smoothstep(0.0, 0.3, paint);
    let highlight = (gloss * (nh.powf(32.0) - h[2].powf(32.0))).max(0.0) * on_paint;
    (shade, highlight)
}

#[cfg(test)]
/// WGSL's smoothstep.
pub(crate) fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
/// effects.wgsl: how much of the weave shows through `paint` of paint (thick paint hides it).
pub(crate) fn weave_showing(paint: f32) -> f32 {
    1.0 - smoothstep(0.0, 0.6, paint)
}
