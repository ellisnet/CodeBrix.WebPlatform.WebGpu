//! Crate-internal: how a stroke set's picked colour reaches the GPU - the colour part of the draw
//! block (h7 to h43, see strokes/stroke_gpu.rs) - and the CPU MIRRORS of what strokes.wgsl
//! computes from it: the tap positions along a stroke, the source's texels as linear colour, the
//! crate's Oklab in the shader's form, the palette, broken colour, the mix with the own colour
//! (no C# equivalent). The host tests check the mirrors against painting/color_rules.rs and
//! painting/color_mix.rs, so the GPU gives the CPU's colours (within float rounding).
//!
//! THE COLOUR PART OF THE DRAW BLOCK (words from h7):
//! - h7 (f32): the place's x, y, 1 / width, 1 / height (set units -> the source's uv);
//! - h8 (u32): colour flags (`PICK_*`), the source's texels (`SourceTexels`), taps, pigments;
//! - h9 (f32): own weight (the shader multiplies it by each stroke's own, from its record), the
//!   palette's snap, broken colour's hue (radians) and chroma;
//! - h10 (f32): broken colour's lightness, 0, 0, 0;
//! - h11 (u32): broken colour's seed, 0, 0, 0;
//! - h12 to h43: the pigments in Oklab (L, a, b, 0), up to 32.

use crate::painting::color_mix::color_to_oklab;
use crate::painting::PaletteMode;
use crate::strokes::stroke_color::{SourceTexels, StrokePickup};

/// Colour flag: the colour is picked from the source.
pub(crate) const PICK_ON: u32 = 1;
/// Colour flag: a palette.
pub(crate) const PICK_PALETTE: u32 = 2;
/// Colour flag: the palette's nearest mix of two pigments.
pub(crate) const PICK_MIX_OF_TWO: u32 = 4;
/// Colour flag: broken colour.
pub(crate) const PICK_BROKEN: u32 = 8;
/// The most pigments the block holds.
pub(crate) const MAX_PIGMENTS: usize = 32;
/// The vec4s of the colour part (h7 to h43).
pub(crate) const COLOUR_VEC4S: usize = 5 + MAX_PIGMENTS;

/// The colour part of a draw block: zero for a set in its own colours.
pub(crate) fn colour_words(pickup: Option<(&StrokePickup, SourceTexels)>) -> Vec<u32> {
    let mut w = vec![0u32; COLOUR_VEC4S * 4];
    let Some((p, texels)) = pickup else {
        return w;
    };
    let f = |v: f32| v.to_bits();
    w[0..4].copy_from_slice(&[f(p.place.x), f(p.place.y), f(1.0 / p.place.width), f(1.0 / p.place.height)]);
    let mut flags = PICK_ON;
    let mut pigments = 0u32;
    let (mut snap, mut hue, mut chroma, mut lightness, mut seed) = (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0u32);
    if let Some(rules) = &p.rules {
        if let Some(palette) = rules.palette.as_ref().filter(|p| !p.pigments.is_empty()) {
            flags |= PICK_PALETTE;
            if palette.mode == PaletteMode::MixOfTwo {
                flags |= PICK_MIX_OF_TWO;
            }
            snap = if palette.snap.is_nan() { 0.0 } else { palette.snap.clamp(0.0, 1.0) };
            for (i, c) in palette.pigments.iter().take(MAX_PIGMENTS).enumerate() {
                let lab = color_to_oklab(*c);
                let at = (5 + i) * 4;
                w[at..at + 4].copy_from_slice(&[f(lab[0]), f(lab[1]), f(lab[2]), 0]);
                pigments += 1;
            }
        }
        if let Some(b) = &rules.broken {
            flags |= PICK_BROKEN;
            let finite = |v: f32| if v.is_finite() { v } else { 0.0 };
            hue = (finite(b.hue) as f64).to_radians() as f32;
            chroma = finite(b.chroma);
            lightness = finite(b.lightness);
            seed = b.seed;
        }
    }
    w[4..8].copy_from_slice(&[flags, texels as u32, p.taps.clamp(1, 5), pigments]);
    w[8..12].copy_from_slice(&[f(p.own_weight.clamp(0.0, 1.0)), f(snap), f(hue), f(chroma)]);
    w[12] = f(lightness);
    w[16] = seed;
    w
}

#[cfg(test)]
pub(crate) mod mirror {
    //! The CPU mirrors of strokes.wgsl's colour arithmetic, line for line (f32).

    use kurbo::Affine;

    use crate::painting::color_mix::{linear_to_srgb, srgb_to_linear};
    use crate::painting::color_rules::noise_hash;
    use crate::painting::{ColorRules, PaletteMode};
    use crate::strokes::stroke_color::{SourceTexels, StrokePickup};

    /// The shader's taps along the stroke (s from -0.5 to 0.5): 1 -> the middle, 3 or 5 -> evenly
    /// from -0.4 to 0.4.
    pub(crate) fn tap_offsets(taps: u32) -> Vec<f32> {
        let taps = taps.clamp(1, 5);
        (0..taps).map(|k| if taps > 1 { -0.4 + 0.8 * k as f32 / (taps - 1) as f32 } else { 0.0 }).collect()
    }

    /// The source uv of each tap of a stroke as drawn (centre, angle in degrees, length, bend)
    /// through its blended anchor matrix `anchor`, for a pickup's place and taps.
    pub(crate) fn pickup_points(center: (f32, f32), angle_degrees: f32, length: f32, bend: f32, anchor: Affine, pickup: &StrokePickup) -> Vec<(f32, f32)> {
        let a = (angle_degrees as f64).to_radians() as f32;
        let (dir, nrm) = ((a.cos(), a.sin()), (-a.sin(), a.cos()));
        let bow = bend * length;
        let m = anchor.as_coeffs().map(|v| v as f32);
        let p = pickup.place;
        tap_offsets(pickup.taps)
            .into_iter()
            .map(|s| {
                let c = bow * (4.0 * s * s - 1.0);
                let x = center.0 + dir.0 * (length * s) + nrm.0 * c;
                let y = center.1 + dir.1 * (length * s) + nrm.1 * c;
                let qx = m[0] * x + m[2] * y + m[4];
                let qy = m[1] * x + m[3] * y + m[5];
                ((qx - p.x) * (1.0 / p.width), (qy - p.y) * (1.0 / p.height))
            })
            .collect()
    }

    fn srgb3(c: [f32; 3]) -> [f32; 3] {
        // common.wgsl's srgb_to_linear (the same curve as painting's).
        c.map(srgb_to_linear)
    }

    /// strokes.wgsl source_texel: one texel as linear premultiplied colour.
    pub(crate) fn source_texel(t: [f32; 4], texels: SourceTexels) -> [f32; 4] {
        let c = t.map(|v| v.clamp(0.0, 1.0));
        match texels {
            SourceTexels::LinearPremultiplied => c,
            SourceTexels::SrgbPremultiplied => {
                if c[3] <= 0.0 {
                    return [0.0; 4];
                }
                let l = srgb3([c[0] / c[3], c[1] / c[3], c[2] / c[3]]);
                [l[0] * c[3], l[1] * c[3], l[2] * c[3], c[3]]
            }
            SourceTexels::SrgbStraight => {
                let l = srgb3([c[0], c[1], c[2]]);
                [l[0] * c[3], l[1] * c[3], l[2] * c[3], c[3]]
            }
        }
    }

    fn mul3(r: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
        [r[0][0] * v[0] + r[0][1] * v[1] + r[0][2] * v[2], r[1][0] * v[0] + r[1][1] * v[1] + r[1][2] * v[2], r[2][0] * v[0] + r[2][1] * v[1] + r[2][2] * v[2]]
    }

    /// strokes.wgsl cbrt3 (sign x pow(max(|x|, 1e-30), 1/3); WGSL's sign of 0 is 0).
    fn cbrt(v: f32) -> f32 {
        let sign = if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 };
        sign * v.abs().max(1e-30).powf(1.0 / 3.0)
    }

    /// strokes.wgsl linear_to_oklab.
    pub(crate) fn linear_to_oklab(c: [f32; 3]) -> [f32; 3] {
        let lms = mul3([[0.412_221_46, 0.536_332_55, 0.051_445_995], [0.211_903_5, 0.680_699_5, 0.107_396_96], [0.088_302_46, 0.281_718_85, 0.629_978_7]], c).map(cbrt);
        mul3([[0.210_454_26, 0.793_617_8, -0.004_072_047], [1.977_998_5, -2.428_592_2, 0.450_593_7], [0.025_904_037, 0.782_771_77, -0.808_675_77]], lms)
    }

    /// strokes.wgsl oklab_to_linear (not clamped).
    pub(crate) fn oklab_to_linear(lab: [f32; 3]) -> [f32; 3] {
        let l = mul3([[1.0, 0.396_337_78, 0.215_803_76], [1.0, -0.105_561_346, -0.063_854_17], [1.0, -0.089_484_18, -1.291_485_5]], lab);
        let lms = l.map(|x| x * x * x);
        mul3([[4.076_741_7, -3.307_711_6, 0.230_969_94], [-1.268_438, 2.609_757_4, -0.341_319_38], [-0.004_196_086_3, -0.703_418_6, 1.707_614_7]], lms)
    }

    fn dist2(a: [f32; 3], b: [f32; 3]) -> f32 {
        (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
    }

    fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
    }

    fn hash_unit(h: u32) -> f32 {
        h as f32 / 4_294_967_295.0 * 2.0 - 1.0
    }

    /// strokes.wgsl palette_target over the pigments in Oklab (the block's h12 onwards).
    pub(crate) fn palette_target(lab: [f32; 3], pigments: &[[f32; 3]], mix_of_two: bool) -> [f32; 3] {
        let mut best = pigments[0];
        let mut best_d = dist2(lab, best);
        for &p in &pigments[1..] {
            let d = dist2(lab, p);
            if d < best_d {
                best = p;
                best_d = d;
            }
        }
        if mix_of_two {
            for i in 0..pigments.len() {
                for j in i + 1..pigments.len() {
                    let a = pigments[i];
                    let ab = [pigments[j][0] - a[0], pigments[j][1] - a[1], pigments[j][2] - a[2]];
                    let len2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
                    let mut q = a;
                    if len2 > 0.0 {
                        let t = (((lab[0] - a[0]) * ab[0] + (lab[1] - a[1]) * ab[1] + (lab[2] - a[2]) * ab[2]) / len2).clamp(0.0, 1.0);
                        q = [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t];
                    }
                    let d = dist2(lab, q);
                    if d < best_d {
                        best = q;
                        best_d = d;
                    }
                }
            }
        }
        best
    }

    /// strokes.wgsl colour_rules on a straight linear colour.
    pub(crate) fn colour_rules(c: [f32; 3], rules: &ColorRules, stroke_seed: u32) -> [f32; 3] {
        let mut lab = linear_to_oklab(c);
        if let Some(p) = rules.palette.as_ref().filter(|p| !p.pigments.is_empty()) {
            let pigments: Vec<[f32; 3]> = p.pigments.iter().take(32).map(|c| crate::painting::color_mix::color_to_oklab(*c)).collect();
            let snap = if p.snap.is_nan() { 0.0 } else { p.snap.clamp(0.0, 1.0) };
            lab = lerp3(lab, palette_target(lab, &pigments, p.mode == PaletteMode::MixOfTwo), snap);
        }
        if let Some(b) = &rules.broken {
            let seed = stroke_seed as i32;
            let u_hue = hash_unit(noise_hash(b.seed, seed, 0, 0));
            let u_chroma = hash_unit(noise_hash(b.seed, seed, 1, 0));
            let u_light = hash_unit(noise_hash(b.seed, seed, 2, 0));
            let chroma = (lab[1] * lab[1] + lab[2] * lab[2]).sqrt() * (1.0 + b.chroma * u_chroma).max(0.0);
            let hue = lab[2].atan2(lab[1]) + (b.hue as f64).to_radians() as f32 * u_hue;
            lab = [(lab[0] + b.lightness * u_light).clamp(0.0, 1.0), chroma * hue.cos(), chroma * hue.sin()];
        }
        oklab_to_linear(lab).map(|v| v.clamp(0.0, 1.0))
    }

    /// strokes.wgsl picked_colour: the taps' texels (as sampled) averaged, the rules, the mix with
    /// the own colour (straight linear), for a stroke of own weight 1. Returns (rgb straight
    /// linear, the alpha factor).
    pub(crate) fn picked_colour(own: [f32; 3], texels: &[[f32; 4]], kind: SourceTexels, pickup: &StrokePickup, stroke_seed: u32) -> ([f32; 3], f32) {
        picked_colour_weighted(own, texels, kind, pickup, stroke_seed, 1.0)
    }

    /// strokes.wgsl picked_colour for a stroke of own weight `stroke_weight` (its record's,
    /// strokes/stroke_gpu.rs own_weight_of_word, or a pair's): the mix with the own colour is by
    /// the pickup's own weight times it.
    pub(crate) fn picked_colour_weighted(own: [f32; 3], texels: &[[f32; 4]], kind: SourceTexels, pickup: &StrokePickup, stroke_seed: u32, stroke_weight: f32) -> ([f32; 3], f32) {
        let mut sum = [0.0f32; 4];
        for t in texels {
            let c = source_texel(*t, kind);
            for i in 0..4 {
                sum[i] += c[i];
            }
        }
        let n = texels.len().max(1) as f32;
        let mean = sum.map(|v| v / n);
        let mut rgb = own;
        if mean[3] > 1e-6 {
            rgb = [mean[0] / mean[3], mean[1] / mean[3], mean[2] / mean[3]];
        }
        if let Some(rules) = pickup.rules.as_ref().filter(|r| r.palette.as_ref().map_or(false, |p| !p.pigments.is_empty()) || r.broken.is_some()) {
            rgb = colour_rules(rgb, rules, stroke_seed);
        }
        let w = pickup.own_weight.clamp(0.0, 1.0) * stroke_weight;
        if w > 0.0 {
            rgb = oklab_to_linear(lerp3(linear_to_oklab(rgb), linear_to_oklab(own), w)).map(|v| v.clamp(0.0, 1.0));
        }
        (rgb, mean[3] + (1.0 - mean[3]) * w)
    }

    /// Straight linear rgb as encoded sRGB (0..1).
    pub(crate) fn encode(c: [f32; 3]) -> [f32; 3] {
        c.map(linear_to_srgb)
    }

    /// Encoded sRGB (0..1) as straight linear rgb.
    pub(crate) fn decode(c: [f32; 3]) -> [f32; 3] {
        c.map(srgb_to_linear)
    }
}
