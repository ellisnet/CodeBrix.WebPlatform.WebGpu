//! Paints for the GPU: solid colours, gradients (linear, two-point radial, sweep) with pad /
//! repeat / reflect, images, and the composite of a group. Every paint carries the matrix from
//! device pixels to its own space.
//!
//! TAKEN FROM vello 0.12.0 (Apache-2.0 OR MIT; Apache-2.0 elected; see the notices): the gradient
//! ramp sampling (`make_ramp`, vello_encoding/src/ramp_cache.rs) and the two-point conical
//! gradient set-up (the DRAWTAG_FILL_RAD_GRADIENT case of vello_shaders/shader/draw_leaf.wgsl,
//! with `two_point_to_unit_line` and `from_poly2`), translated to Rust; the sweep set-up follows
//! the DRAWTAG_FILL_SWEEP_GRADIENT case of the same file.

use std::collections::HashMap;

use kurbo::{Affine, Point};
use peniko::color::Srgb;
use peniko::{Brush, Extend, Gradient, GradientKind, ImageAlphaType, ImageBrush, ImageQuality, InterpolationAlphaSpace};

use crate::painting::color_mix::mix_float;
use crate::painting::ColorInterpolation;
use crate::rendering::vector_scene::Hasher;

/// Samples per gradient ramp (vello's N_SAMPLES).
pub(crate) const RAMP_SAMPLES: usize = 512;
/// Ramps kept on the GPU at once.
pub(crate) const RAMP_SLOTS: usize = 256;

pub(crate) const KIND_SOLID: u32 = 0;
pub(crate) const KIND_LINEAR: u32 = 1;
pub(crate) const KIND_RADIAL: u32 = 2;
pub(crate) const KIND_SWEEP: u32 = 3;
pub(crate) const KIND_IMAGE: u32 = 4;
pub(crate) const KIND_COMPOSITE: u32 = 5;
pub(crate) const KIND_NOISE: u32 = 6;
pub(crate) const KIND_OWNERS: u32 = 7;

// vello's radial kinds (vello_shaders/shader/shared/config.wgsl).
const RAD_GRAD_KIND_CIRCULAR: u32 = 1;
const RAD_GRAD_KIND_STRIP: u32 = 2;
const RAD_GRAD_KIND_FOCAL_ON_CIRCLE: u32 = 3;
const RAD_GRAD_KIND_CONE: u32 = 4;
const RAD_GRAD_SWAPPED: u32 = 1;

/// One paint as the shader reads it (80 bytes; `#[repr(C)]` plain data).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PaintData {
    /// kind, extend (x | y << 4), ramp slot, flags.
    pub(crate) info: [u32; 4],
    /// Solid: linear premultiplied colour. Image: width, height.
    pub(crate) color: [f32; 4],
    /// Device to paint space: a b c d.
    pub(crate) m0: [f32; 4],
    /// e f, then two parameters (radial: focal_x, radius; sweep: t0, t1).
    pub(crate) m1: [f32; 4],
    /// alpha, then unused.
    pub(crate) extra: [f32; 4],
}

pub(crate) fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn extend_code(extend: Extend) -> u32 {
    match extend {
        Extend::Pad => 0,
        Extend::Repeat => 1,
        Extend::Reflect => 2,
    }
}

fn matrix(m: Affine) -> ([f32; 4], [f32; 2]) {
    let c = m.as_coeffs();
    ([c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32], [c[4] as f32, c[5] as f32])
}

/// The paint for a solid colour with an extra alpha.
pub(crate) fn solid(color: peniko::color::AlphaColor<Srgb>, alpha: f32) -> PaintData {
    let [r, g, b, a] = color.components;
    let a = (a * alpha).clamp(0.0, 1.0);
    PaintData { info: [KIND_SOLID, 0, 0, 0], color: [srgb_to_linear(r) * a, srgb_to_linear(g) * a, srgb_to_linear(b) * a, a], extra: [1.0, 0.0, 0.0, 0.0], ..PaintData::default() }
}

/// The paint that composites a group with an alpha and (for a mix mode) the blend mode.
pub(crate) fn composite(alpha: f32, mix: u32, compose: u32) -> PaintData {
    PaintData { info: [KIND_COMPOSITE, 0, 0, (mix << 8) | compose], extra: [alpha, 0.0, 0.0, 0.0], ..PaintData::default() }
}

/// True when a solid colour is fully transparent after `alpha` (nothing to draw).
pub(crate) fn is_invisible(brush: &Brush, alpha: f32) -> bool {
    match brush {
        Brush::Solid(c) => c.components[3] * alpha <= 0.0,
        Brush::Image(i) => alpha * i.sampler.alpha <= 0.0,
        _ => alpha <= 0.0,
    }
}

/// The gradient's paint: `to_paint` maps the shape's own space (the brush's, after
/// `brush_transform`) to device pixels; the paint carries its inverse.
pub(crate) fn gradient(g: &Gradient, to_device: Affine, alpha: f32, ramp_slot: u32) -> Option<PaintData> {
    if to_device.determinant().abs() < 1e-12 {
        return None;
    }
    let inv = to_device.inverse();
    let extend = extend_code(g.extend);
    let mut p = PaintData { info: [0, extend, ramp_slot, 0], extra: [alpha, 0.0, 0.0, 0.0], ..PaintData::default() };
    let (m, params) = match &g.kind {
        GradientKind::Linear(l) => {
            if l.start == l.end {
                return None;
            }
            p.info[0] = KIND_LINEAR;
            (two_point_to_unit_line(l.start, l.end) * inv, [0.0, 0.0])
        }
        GradientKind::Radial(r) => {
            p.info[0] = KIND_RADIAL;
            let (xform, focal_x, radius, kind, flags) = radial_setup(r.start_center, r.start_radius, r.end_center, r.end_radius);
            p.info[3] = (flags << 3) | kind;
            (xform * inv, [focal_x, radius])
        }
        GradientKind::Sweep(s) => {
            p.info[0] = KIND_SWEEP;
            let t0 = s.start_angle / std::f32::consts::TAU;
            let t1 = s.end_angle / std::f32::consts::TAU;
            if t0 == t1 {
                return None;
            }
            (Affine::translate(-s.center.to_vec2()) * inv, [t0, t1])
        }
    };
    let (m0, e) = matrix(m);
    p.m0 = m0;
    p.m1 = [e[0], e[1], params[0], params[1]];
    Some(p)
}

/// The noise paint (a [`NoiseShape`](crate::shapes::NoiseShape)): device pixels to the shape's
/// calibrated space, where the noise is evaluated per pixel and mapped through the ramp.
/// info: kind, (gradient | has z << 1 | caustics << 2 | octaves << 8), ramp slot, seed; color: frequency, z,
/// lacunarity, gain; m1.zw: the offset.
pub(crate) fn noise(n: &crate::shapes::noise_shape::NoisePaintParams, to_device: Affine, alpha: f32, ramp_slot: u32) -> Option<PaintData> {
    if to_device.determinant().abs() < 1e-12 {
        return None;
    }
    let (m0, e) = matrix(to_device.inverse());
    let mode = (n.gradient as u32) | ((n.z.is_some() as u32) << 1) | ((n.caustics as u32) << 2) | (n.octaves.clamp(1, 30) << 8);
    Some(PaintData { info: [KIND_NOISE, mode, ramp_slot, n.seed], color: [n.frequency, n.z.unwrap_or(0.0), n.lacunarity, n.gain], m0, m1: [e[0], e[1], n.offset[0], n.offset[1]], extra: [alpha, 0.0, 0.0, 0.0] })
}

/// The ownership paint (an [`OwnershipShape`](crate::shapes::OwnershipShape)): device pixels to
/// map pixels; info: kind, unowned colour (RGBA bytes), seam colour (RGBA bytes), flags (1:
/// seams); color: the map's width, height, the seam strength.
pub(crate) fn owners(brush: &ImageBrush, o: &crate::shapes::ownership_shape::OwnersPaintParams, to_device: Affine, alpha: f32) -> Option<PaintData> {
    if to_device.determinant().abs() < 1e-12 {
        return None;
    }
    let (m0, e) = matrix(to_device.inverse());
    Some(PaintData {
        info: [KIND_OWNERS, o.unowned, o.seam.unwrap_or(0), o.seam.is_some() as u32],
        color: [o.map_size.0 as f32, o.map_size.1 as f32, o.seam_strength, 0.0],
        m0,
        m1: [e[0], e[1], 0.0, 0.0],
        extra: [alpha * brush.sampler.alpha, 0.0, 0.0, 0.0],
    })
}

/// The image paint: device pixels to image pixels.
pub(crate) fn image(brush: &ImageBrush, to_device: Affine, alpha: f32, tint: Option<[f32; 4]>) -> Option<PaintData> {
    if to_device.determinant().abs() < 1e-12 {
        return None;
    }
    let (m0, e) = matrix(to_device.inverse());
    let extend = extend_code(brush.sampler.x_extend) | (extend_code(brush.sampler.y_extend) << 4);
    let quality = if brush.sampler.quality == ImageQuality::Low { 0 } else { 1 };
    let premultiplied = if brush.image.alpha_type == ImageAlphaType::AlphaPremultiplied { 1 } else { 0 };
    // A tint (pictures and sprites) multiplies the straight sRGB values channel by channel,
    // alpha included (C#'s colour matrix): flag 4, its four factors in color.zw and extra.yz.
    let (flag, t) = match tint {
        Some(t) => (4u32, t),
        None => (0, [1.0; 4]),
    };
    Some(PaintData {
        info: [KIND_IMAGE, extend, 0, quality | (premultiplied << 1) | flag],
        color: [brush.image.width as f32, brush.image.height as f32, t[0], t[1]],
        m0,
        m1: [e[0], e[1], 0.0, 0.0],
        extra: [alpha * brush.sampler.alpha, t[2], t[3], 0.0],
    })
}

/// vello's `two_point_to_unit_line`: maps p0 to (0, 0) and p1 to (1, 0).
fn two_point_to_unit_line(p0: Point, p1: Point) -> Affine {
    let tmp1 = from_poly2(p0, p1);
    let inv = tmp1.inverse();
    let tmp2 = from_poly2(Point::ZERO, Point::new(1.0, 0.0));
    tmp2 * inv
}

fn from_poly2(p0: Point, p1: Point) -> Affine {
    Affine::new([p1.y - p0.y, p0.x - p1.x, p1.x - p0.x, p1.y - p0.y, p0.x, p0.y])
}

/// The two-point conical gradient set-up of vello's draw_leaf.wgsl (after Skia's design note
/// "Two-point conical gradients"): the transform from gradient space to the shader's space,
/// focal_x, radius, kind and flags.
pub(crate) fn radial_setup(p0: Point, r0: f32, p1: Point, r1: f32) -> (Affine, f32, f32, u32, u32) {
    // This epsilon matches what Skia uses.
    let epsilon = 1.0 / (1u32 << 12) as f64;
    let (mut p0, mut p1, mut r0, mut r1) = (p0, p1, r0 as f64, r1 as f64);
    if (r0 - r1).abs() <= epsilon {
        // Equal radii: a strip gradient.
        let scaled = r0 / p0.distance(p1);
        return (two_point_to_unit_line(p0, p1), 0.0, (scaled * scaled) as f32, RAD_GRAD_KIND_STRIP, 0);
    }
    let mut kind = RAD_GRAD_KIND_CONE;
    let mut flags = 0;
    if p0 == p1 {
        kind = RAD_GRAD_KIND_CIRCULAR;
        // Nudge p0 a bit to avoid denormals.
        p0 = Point::new(p0.x + epsilon, p0.y + epsilon);
    }
    if r1 == 0.0 {
        // If r1 is 0, swap the points and radii.
        flags |= RAD_GRAD_SWAPPED;
        std::mem::swap(&mut p0, &mut p1);
        std::mem::swap(&mut r0, &mut r1);
    }
    let focal_x = r0 / (r0 - r1);
    let cf = Point::new((1.0 - focal_x) * p0.x + focal_x * p1.x, (1.0 - focal_x) * p0.y + focal_x * p1.y);
    let radius = r1 / cf.distance(p1);
    let user_to_unit_line = two_point_to_unit_line(cf, p1);
    let user_to_scaled = if (radius - 1.0).abs() <= epsilon {
        // The focal point is on the circle.
        kind = RAD_GRAD_KIND_FOCAL_ON_CIRCLE;
        let scale = 0.5 * (1.0 - focal_x).abs();
        Affine::new([scale, 0.0, 0.0, scale, 0.0, 0.0]) * user_to_unit_line
    } else {
        let a = radius * radius - 1.0;
        let scale_ratio = (1.0 - focal_x).abs() / a;
        let scale_x = radius * scale_ratio;
        let scale_y = a.abs().sqrt() * scale_ratio;
        Affine::new([scale_x, 0.0, 0.0, scale_y, 0.0, 0.0]) * user_to_unit_line
    };
    (user_to_scaled, focal_x as f32, radius as f32, kind, flags)
}

/// The colours of a gradient sampled at RAMP_SAMPLES points, premultiplied, sRGB-encoded
/// (vello's `make_ramp`, with f32 output instead of packed 8-bit, so that low alpha keeps its
/// precision). `Unpremultiplied` interpolates straight colours (NoSkia's choice);
/// `Premultiplied` interpolates premultiplied ones (peniko's default, what velato asks for).
/// ONE CHANGE from vello's: the first stop's own position is honoured (vello starts its first
/// segment at 0), so a gradient whose first stop is past 0 holds that stop's colour up to it, as
/// NoSkia (and CSS) do.
/// THE OPT-IN SPACES (not vello's): a gradient whose `interpolation_cs` is linear sRGB, Oklab or
/// Oklch (set by a fill's `with_interpolation`) has each segment mixed in that space by
/// `painting::color_mix::mix_float` (premultiplied, as `ColorMix::mix`), then premultiplied for
/// the ramp; every other tag - the default sRGB, all the crate sets otherwise - is sampled as
/// before.
pub(crate) fn make_ramp(gradient: &Gradient, out: &mut Vec<[f32; 4]>) {
    let stops = &gradient.stops;
    let space = gradient.interpolation_alpha_space;
    let mix = ColorInterpolation::from_color_space_tag(gradient.interpolation_cs);
    out.clear();
    if stops.is_empty() {
        out.resize(RAMP_SAMPLES, [0.0; 4]);
        return;
    }
    let color = |i: usize| stops[i].color.to_alpha_color::<Srgb>().components;
    let mut last_u = 0.0f32;
    let mut last_c = color(0);
    let mut this_u = stops[0].offset.clamp(0.0, 1.0);
    let mut this_c = last_c;
    let mut j = 0;
    for i in 0..RAMP_SAMPLES {
        let u = i as f32 / (RAMP_SAMPLES - 1) as f32;
        while u > this_u {
            last_u = this_u;
            last_c = this_c;
            if let Some(s) = stops.get(j + 1) {
                this_u = s.offset;
                this_c = s.color.to_alpha_color::<Srgb>().components;
                j += 1;
            } else {
                break;
            }
        }
        let du = this_u - last_u;
        let c = if du < 1e-9 {
            premultiply(this_c)
        } else {
            let t = (u - last_u) / du;
            if mix != ColorInterpolation::Srgb {
                premultiply(mix_float(last_c, this_c, t, mix))
            } else {
                match space {
                    InterpolationAlphaSpace::Premultiplied => {
                        let (a, b) = (premultiply(last_c), premultiply(this_c));
                        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, a[3] + (b[3] - a[3]) * t]
                    }
                    InterpolationAlphaSpace::Unpremultiplied => premultiply([last_c[0] + (this_c[0] - last_c[0]) * t, last_c[1] + (this_c[1] - last_c[1]) * t, last_c[2] + (this_c[2] - last_c[2]) * t, last_c[3] + (this_c[3] - last_c[3]) * t]),
                }
            }
        };
        out.push(c);
    }
}

fn premultiply(c: [f32; 4]) -> [f32; 4] {
    [c[0] * c[3], c[1] * c[3], c[2] * c[3], c[3]]
}

/// The gradient ramps on the GPU: a slot per distinct stop list, kept between frames, the
/// least recently used one replaced when all are taken.
#[derive(Default)]
pub(crate) struct RampCache {
    slots: HashMap<u64, (u32, u64)>,
    next: u32,
    /// Ramps to upload this frame: (slot, samples).
    pub(crate) uploads: Vec<(u32, Vec<[f32; 4]>)>,
    scratch: Vec<[f32; 4]>,
}

impl RampCache {
    /// The slot of this gradient's ramp, made (and queued for upload) when new.
    pub(crate) fn slot(&mut self, gradient: &Gradient, frame: u64) -> u32 {
        let mut h = Hasher::new();
        h.word(gradient.interpolation_alpha_space as u64);
        h.word(gradient.interpolation_cs as u64);
        for stop in gradient.stops.iter() {
            h.f64(stop.offset as f64);
            for c in stop.color.to_alpha_color::<Srgb>().components {
                h.f64(c as f64);
            }
        }
        let key = h.finish();
        if let Some(entry) = self.slots.get_mut(&key) {
            entry.1 = frame;
            return entry.0;
        }
        let slot = if (self.next as usize) < RAMP_SLOTS {
            self.next += 1;
            self.next - 1
        } else {
            // Replace the least recently used ramp that this frame has not used.
            let victim = self.slots.iter().filter(|(_, v)| v.1 != frame).min_by_key(|(_, v)| v.1).map(|(k, v)| (*k, v.0));
            match victim {
                Some((old, slot)) => {
                    self.slots.remove(&old);
                    slot
                }
                None => 0,
            }
        };
        make_ramp(gradient, &mut self.scratch);
        self.uploads.push((slot, self.scratch.clone()));
        self.slots.insert(key, (slot, frame));
        slot
    }

    /// Forgets every ramp (the GPU buffer was made again).
    pub(crate) fn clear(&mut self) {
        self.slots.clear();
        self.next = 0;
        self.uploads.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use peniko::color::AlphaColor;
    use peniko::ColorStop;

    fn stops(space: InterpolationAlphaSpace) -> Gradient {
        let mut g = Gradient::new_linear((0.0, 0.0), (1.0, 0.0));
        g.stops.0.push(ColorStop::from((0.0, AlphaColor::<Srgb>::new([1.0, 0.0, 0.0, 1.0]))));
        g.stops.0.push(ColorStop::from((1.0, AlphaColor::<Srgb>::new([0.0, 0.0, 1.0, 0.0]))));
        g.interpolation_alpha_space = space;
        g
    }

    /// NoSkia's gradient colour at t (DrawingShader.BuildLut, before its 256-entry
    /// quantisation): straight sRGB components and straight alpha, linearly between the stops
    /// around t; the first stop's colour before it, the last's after it.
    fn noskia_color_at(stops: &[(f32, [u8; 4])], t: f32) -> [f32; 4] {
        let c = |i: usize| stops[i].1.map(|v| v as f32 / 255.0);
        if t <= stops[0].0 {
            return c(0);
        }
        let mut index = 0;
        while index < stops.len() - 1 && t > stops[index + 1].0 {
            index += 1;
        }
        if index >= stops.len() - 1 {
            return c(stops.len() - 1);
        }
        let segment = stops[index + 1].0 - stops[index].0;
        let f = if segment > 0.0 { ((t - stops[index].0) / segment).clamp(0.0, 1.0) } else { 0.0 };
        let (a, b) = (c(index), c(index + 1));
        [a[0] * (1.0 - f) + b[0] * f, a[1] * (1.0 - f) + b[1] * f, a[2] * (1.0 - f) + b[2] * f, a[3] * (1.0 - f) + b[3] * f]
    }

    #[test]
    fn session_gradient_fills_interpolate_as_noskia_does() {
        use crate::fills::{ColorStop as Stop, GradientSpread, LinearGradientFill, RadialGradientFill, ShapeFill};
        use crate::{Color, PointF};
        // The demo's halo (a first stop past 0, alpha falling to 0), a three-stop ramp and a hard edge.
        let cases: Vec<Vec<(f32, [u8; 4])>> = vec![
            vec![(0.55, [255, 200, 120, 150]), (1.0, [255, 200, 120, 0])],
            vec![(0.0, [40, 220, 170, 255]), (0.5, [70, 110, 255, 255]), (1.0, [230, 80, 200, 255])],
            vec![(0.0, [30, 40, 110, 255]), (0.4, [200, 90, 120, 255]), (0.4, [255, 190, 120, 128]), (1.0, [0, 0, 0, 0])],
        ];
        for (n, case) in cases.iter().enumerate() {
            let list: Vec<Stop> = case.iter().map(|(p, c)| Stop::new(*p, Color::from_rgba(c[0], c[1], c[2], c[3])).unwrap()).collect();
            let fill: ShapeFill = if n % 2 == 0 {
                RadialGradientFill::new(PointF::new(10.0, 10.0), 50.0, &list, GradientSpread::Pad).unwrap().into()
            } else {
                LinearGradientFill::new(PointF::new(0.0, 0.0), PointF::new(100.0, 0.0), &list, GradientSpread::Reflect).unwrap().into()
            };
            let Brush::Gradient(g) = fill.to_brush(1.0) else { panic!("a gradient fill is a gradient brush") };
            let mut out = Vec::new();
            make_ramp(&g, &mut out);
            for (i, sample) in out.iter().enumerate() {
                let t = i as f32 / (RAMP_SAMPLES - 1) as f32;
                let want = noskia_color_at(case, t);
                let a = sample[3];
                assert!((a - want[3]).abs() < 1e-5, "case {n} sample {i}: alpha {a} vs {}", want[3]);
                for k in 0..3 {
                    // The ramp is premultiplied: compare the premultiplied value.
                    assert!((sample[k] - want[k] * want[3]).abs() < 1e-5, "case {n} sample {i} channel {k}: {} vs {}", sample[k], want[k] * want[3]);
                }
            }
        }
    }

    #[test]
    fn a_ramp_interpolates_straight_or_premultiplied() {
        let mut out = Vec::new();
        make_ramp(&stops(InterpolationAlphaSpace::Unpremultiplied), &mut out);
        assert_eq!(out.len(), RAMP_SAMPLES);
        assert_eq!(out[0], [1.0, 0.0, 0.0, 1.0]);
        // Half way, straight: (0.5, 0, 0.5) at alpha 0.5, then premultiplied.
        let mid = out[255];
        let t = 255.0 / 511.0;
        assert!((mid[3] - (1.0 - t)).abs() < 1e-6);
        assert!((mid[2] - t * (1.0 - t)).abs() < 1e-6);
        make_ramp(&stops(InterpolationAlphaSpace::Premultiplied), &mut out);
        // Premultiplied: the transparent blue contributes nothing.
        assert!(out[255][2].abs() < 1e-6);
        assert_eq!(out[511], [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn a_ramp_in_an_opt_in_space_is_the_colour_mix_premultiplied() {
        use crate::painting::color_mix::mix_float;
        use crate::painting::ColorInterpolation;
        use peniko::color::ColorSpaceTag;
        let (blue, yellow, half_red) = ([0.0, 0.0, 1.0, 1.0], [1.0, 1.0, 0.0, 1.0], [1.0, 0.0, 0.0, 0.5]);
        for (tag, space) in [(ColorSpaceTag::Oklab, ColorInterpolation::Oklab), (ColorSpaceTag::Oklch, ColorInterpolation::Oklch), (ColorSpaceTag::LinearSrgb, ColorInterpolation::LinearRgb)] {
            let mut g = Gradient::new_linear((0.0, 0.0), (1.0, 0.0)).with_interpolation_cs(tag);
            g.stops.0.push(ColorStop::from((0.25, AlphaColor::<Srgb>::new(blue))));
            g.stops.0.push(ColorStop::from((0.75, AlphaColor::<Srgb>::new(yellow))));
            g.stops.0.push(ColorStop::from((1.0, AlphaColor::<Srgb>::new(half_red))));
            g.interpolation_alpha_space = InterpolationAlphaSpace::Unpremultiplied;
            let mut out = Vec::new();
            make_ramp(&g, &mut out);
            assert_eq!(out.len(), RAMP_SAMPLES);
            // Before the first stop its colour is held, as in sRGB.
            assert_eq!(out[0], blue);
            for (i, sample) in out.iter().enumerate() {
                let u = i as f32 / (RAMP_SAMPLES - 1) as f32;
                let want = if u <= 0.25 {
                    blue
                } else if u <= 0.75 {
                    mix_float(blue, yellow, (u - 0.25) / 0.5, space)
                } else {
                    mix_float(yellow, half_red, (u - 0.75) / 0.25, space)
                };
                let want = [want[0] * want[3], want[1] * want[3], want[2] * want[3], want[3]];
                for k in 0..4 {
                    assert!((sample[k] - want[k]).abs() < 1e-5, "{space:?} sample {i} channel {k}: {} vs {}", sample[k], want[k]);
                }
            }
            // Oklab's middle keeps more colour than the sRGB grey (128, 128, 128).
            if space == ColorInterpolation::Oklab {
                let mid = out[255];
                assert!((mid[0] - mid[2]).abs() > 0.05 || (mid[1] - mid[2]).abs() > 0.05, "{mid:?}");
            }
        }
    }

    #[test]
    fn the_same_stops_in_another_space_get_their_own_ramp() {
        let mut cache = RampCache::default();
        let srgb = stops(InterpolationAlphaSpace::Unpremultiplied);
        let oklab = stops(InterpolationAlphaSpace::Unpremultiplied).with_interpolation_cs(peniko::color::ColorSpaceTag::Oklab);
        let a = cache.slot(&srgb, 1);
        let b = cache.slot(&oklab, 1);
        assert_ne!(a, b);
        assert_ne!(cache.uploads[0].1, cache.uploads[1].1);
    }

    #[test]
    fn the_radial_set_up_sorts_the_kinds_as_vello_does() {
        assert_eq!(radial_setup(Point::new(0.0, 0.0), 10.0, Point::new(50.0, 0.0), 10.0).3, RAD_GRAD_KIND_STRIP);
        let (xform, focal, _, kind, _) = radial_setup(Point::new(5.0, 5.0), 0.0, Point::new(5.0, 5.0), 10.0);
        assert_eq!(kind, RAD_GRAD_KIND_CIRCULAR);
        assert!(focal.abs() < 1e-6);
        // A point on the end circle lands at distance 1 from the focus in the shader's space.
        let p = xform * Point::new(15.0, 5.0);
        assert!((p.to_vec2().hypot() - 1.0).abs() < 1e-3, "{p:?}");
        assert_eq!(radial_setup(Point::new(0.0, 0.0), 10.0, Point::new(5.0, 0.0), 0.0).4, RAD_GRAD_SWAPPED);
        assert_eq!(radial_setup(Point::new(0.0, 0.0), 2.0, Point::new(30.0, 0.0), 20.0).3, RAD_GRAD_KIND_CONE);
    }

    #[test]
    fn ramps_are_kept_and_replaced_least_recently_used_first() {
        let mut cache = RampCache::default();
        let a = cache.slot(&stops(InterpolationAlphaSpace::Unpremultiplied), 1);
        let b = cache.slot(&stops(InterpolationAlphaSpace::Premultiplied), 1);
        assert_ne!(a, b);
        assert_eq!(cache.uploads.len(), 2);
        cache.uploads.clear();
        assert_eq!(cache.slot(&stops(InterpolationAlphaSpace::Unpremultiplied), 2), a);
        assert!(cache.uploads.is_empty());
    }

    #[test]
    fn noise_mode_has_the_caustics_bit_only_for_the_caustics_kind() {
        use crate::planes::DrawingNoiseKind;
        use crate::shapes::{NoiseShape, NoiseShapeOptions};
        let seed = (5i64 << 32) + 7;
        let paint = |kind| {
            let shape = NoiseShape::new(0.0, 0.0, 10.0, 10.0, kind, seed, 0.1, NoiseShapeOptions::default()).unwrap().with_z(2.0);
            noise(&shape.gpu_params(), Affine::IDENTITY, 1.0, 0).unwrap()
        };
        let (value, gradient, caustics) = (paint(DrawingNoiseKind::Value), paint(DrawingNoiseKind::Gradient), paint(DrawingNoiseKind::Caustics));
        // The existing kinds keep their bits and the 64-bit seed folded with xor.
        assert_eq!(value.info[1], 2 | (1 << 8));
        assert_eq!(gradient.info[1], 1 | 2 | (1 << 8));
        assert_eq!(value.info[3], 7 ^ 5);
        assert_eq!(gradient.info[3], 7 ^ 5);
        // The caustics kind: bit 4, and the seed's low 32 bits (its CPU twin's).
        assert_eq!(caustics.info[1], 4 | 2 | (1 << 8));
        assert_eq!(caustics.info[3], 7);
        assert_eq!(caustics.color[1], 2.0);
    }

    #[test]
    fn solids_are_linear_and_premultiplied() {
        let p = solid(AlphaColor::<Srgb>::new([1.0, 0.5, 0.0, 0.5]), 0.5);
        assert_eq!(p.color[3], 0.25);
        assert!((p.color[1] - srgb_to_linear(0.5) * 0.25).abs() < 1e-7);
    }
}
