//! `ColorRules`: what happens to a colour a stroke picks up before it is painted - a limited
//! palette it snaps to, and "broken colour" (a small seeded shift of hue, chroma and lightness
//! per stroke) - worked out in Oklab (no C# equivalent). The CPU definition, used by CPU pickup
//! and planning and mirrored by the stroke shader for pickup on the GPU.

use crate::color::Color;
use crate::painting::color_mix::{oklab_to_srgb, srgb_to_oklab, to_byte};

/// The most pigments a [`PaintPalette`] may have.
const MAX_PIGMENTS: usize = 32;

/// The rules applied to a picked-up colour (no C# equivalent): the palette first, then broken
/// colour. Both are off by default (`None`): the colour is left as it is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ColorRules {
    /// A limited palette the colour is pulled towards (None: no palette).
    pub palette: Option<PaintPalette>,
    /// A seeded per-stroke shift of hue, chroma and lightness (None: none).
    pub broken: Option<BrokenColor>,
}

/// A limited palette (no C# equivalent): THE APPLICATION'S pigments (the crate ships none) and
/// how strongly, and how, a colour is pulled onto them.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintPalette {
    /// The pigments, 1 to 32 colours (their alpha is not used).
    pub pigments: Vec<Color>,
    /// How far the colour moves onto the palette, in Oklab: 0 = untouched .. 1 = fully on it.
    pub snap: f32,
    /// What "on the palette" means.
    pub mode: PaletteMode,
}

impl PaintPalette {
    /// A palette of pigments, fully snapped (`snap` 1), the nearest pigment
    /// ([`PaletteMode::Nearest`]).
    pub fn new(pigments: Vec<Color>) -> PaintPalette {
        PaintPalette { pigments, snap: 1.0, mode: PaletteMode::Nearest }
    }
}

/// What a [`PaintPalette`] pulls a colour onto (no C# equivalent).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteMode {
    /// The nearest pigment in Oklab.
    Nearest,
    /// The nearest MIX OF TWO pigments: the closest point, in Oklab, on any straight segment
    /// between two pigments - how a limited palette is really used (one pigment: that pigment).
    MixOfTwo,
}

/// Broken colour (no C# equivalent): each stroke's colour shifted by a small amount of its own -
/// in Oklch hue and chroma, and Oklab lightness - by a 32-bit hash of the rules' seed and the
/// stroke's seed, so neighbouring strokes of one colour vibrate the way a painter's do. Every
/// amount is "up to, either way"; each stroke draws three numbers in -1..1 from the hash.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrokenColor {
    /// Up to this many degrees of Oklch hue either way (12).
    pub hue: f32,
    /// Up to this fraction of the chroma either way (0.1).
    pub chroma: f32,
    /// Up to this much Oklab lightness either way (0 = the value is kept).
    pub lightness: f32,
    /// The rules' own seed (0).
    pub seed: u32,
}

impl Default for BrokenColor {
    fn default() -> BrokenColor {
        BrokenColor { hue: 12.0, chroma: 0.1, lightness: 0.0, seed: 0 }
    }
}

impl ColorRules {
    /// Checks the rules; the error says what to change. [`ColorRules::apply`] never fails: it
    /// reads unusable values as their nearest usable ones (the palette's first 32 pigments, an
    /// empty palette as none, `snap` clamped to 0..1 with NaN as 0, an amount that is not finite
    /// as 0).
    pub fn validate(&self) -> Result<(), String> {
        if let Some(p) = &self.palette {
            if p.pigments.is_empty() || p.pigments.len() > MAX_PIGMENTS {
                return Err(format!("A paint palette needs 1 to {MAX_PIGMENTS} pigments; it has {}.", p.pigments.len()));
            }
            if !(p.snap >= 0.0 && p.snap <= 1.0) {
                return Err("A paint palette's snap must be a fraction, 0..1 (0 = untouched, 1 = fully on the palette).".to_string());
            }
        }
        if let Some(b) = &self.broken {
            if !(b.hue >= 0.0 && b.hue <= 180.0) {
                return Err("Broken colour's hue must be 0 to 180 degrees.".to_string());
            }
            if !(b.chroma >= 0.0 && b.chroma <= 1.0) {
                return Err("Broken colour's chroma must be a fraction, 0..1.".to_string());
            }
            if !(b.lightness >= 0.0 && b.lightness <= 1.0) {
                return Err("Broken colour's lightness must be 0..1 (Oklab lightness; 0 keeps the value).".to_string());
            }
        }
        Ok(())
    }

    /// The rules applied to one colour for a stroke's seed: the palette first, then broken
    /// colour; alpha unchanged. Channels that leave the sRGB gamut are clamped, and the result is
    /// rounded to 8 bits.
    pub fn apply(&self, color: Color, stroke_seed: u32) -> Color {
        let [r, g, b, _] = self.apply_float([color.r as f32 / 255.0, color.g as f32 / 255.0, color.b as f32 / 255.0, 1.0], stroke_seed);
        Color::from_rgba(to_byte(r), to_byte(g), to_byte(b), color.a)
    }

    /// The same for a straight-alpha float colour of 0..1 sRGB-encoded channels - the form
    /// [`DrawingOwnershipMap::get_mean_colors`](crate::painting::DrawingOwnershipMap::get_mean_colors)
    /// gives. Alpha is passed through; the channels come back clamped to 0..1.
    pub fn apply_float(&self, color: [f32; 4], stroke_seed: u32) -> [f32; 4] {
        let mut lab = srgb_to_oklab([color[0], color[1], color[2]]);
        if let Some(p) = &self.palette {
            if let Some(target) = palette_target(p, lab) {
                let snap = if p.snap.is_nan() { 0.0 } else { p.snap.clamp(0.0, 1.0) };
                lab = [lab[0] + ((target[0] - lab[0]) * snap), lab[1] + ((target[1] - lab[1]) * snap), lab[2] + ((target[2] - lab[2]) * snap)];
            }
        }
        if let Some(b) = &self.broken {
            lab = broken(b, lab, stroke_seed);
        }
        let [r, g, bl] = oklab_to_srgb(lab).map(|v| if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) });
        [r, g, bl, color[3]]
    }
}

fn finite_or_zero(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

fn distance_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (d0, d1, d2) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    (d0 * d0) + (d1 * d1) + (d2 * d2)
}

/// The point of the palette a colour (Oklab) is pulled onto; None for an empty palette. Ties go
/// to the earlier pigment (or pair).
fn palette_target(palette: &PaintPalette, lab: [f32; 3]) -> Option<[f32; 3]> {
    let pigments: Vec<[f32; 3]> = palette.pigments.iter().take(MAX_PIGMENTS).map(|c| srgb_to_oklab([c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0])).collect();
    let first = *pigments.first()?;
    let mut best = first;
    let mut best_distance = distance_squared(lab, first);
    for &p in &pigments[1..] {
        let d = distance_squared(lab, p);
        if d < best_distance {
            best = p;
            best_distance = d;
        }
    }
    if palette.mode == PaletteMode::MixOfTwo {
        for i in 0..pigments.len() {
            for j in (i + 1)..pigments.len() {
                let q = nearest_on_segment(lab, pigments[i], pigments[j]);
                let d = distance_squared(lab, q);
                if d < best_distance {
                    best = q;
                    best_distance = d;
                }
            }
        }
    }
    Some(best)
}

/// The closest point to `p` on the segment from `a` to `b`.
fn nearest_on_segment(p: [f32; 3], a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let length_squared = (ab[0] * ab[0]) + (ab[1] * ab[1]) + (ab[2] * ab[2]);
    if !(length_squared > 0.0) {
        return a;
    }
    let t = ((((p[0] - a[0]) * ab[0]) + ((p[1] - a[1]) * ab[1]) + ((p[2] - a[2]) * ab[2])) / length_squared).clamp(0.0, 1.0);
    [a[0] + (ab[0] * t), a[1] + (ab[1] * t), a[2] + (ab[2] * t)]
}

/// Broken colour on an Oklab colour: three numbers in -1..1 from the hash, then the hue turned
/// (Oklch), the chroma scaled, the lightness moved (clamped to 0..1).
fn broken(b: &BrokenColor, lab: [f32; 3], stroke_seed: u32) -> [f32; 3] {
    let [u_hue, u_chroma, u_lightness] = broken_jitter(b.seed, stroke_seed);
    let chroma = lab[1].hypot(lab[2]) * (1.0 + (finite_or_zero(b.chroma) * u_chroma)).max(0.0);
    let hue = lab[2].atan2(lab[1]) + (finite_or_zero(b.hue) * u_hue).to_radians();
    let lightness = (lab[0] + (finite_or_zero(b.lightness) * u_lightness)).clamp(0.0, 1.0);
    [lightness, chroma * hue.cos(), chroma * hue.sin()]
}

/// The three numbers in -1..1 broken colour draws for a stroke: `noise_hash(rules seed, stroke
/// seed as i32, k, 0)` for k = 0 (hue), 1 (chroma), 2 (lightness), each as `f32(h) / 4294967295
/// x 2 - 1` - the same in the stroke shader.
pub(crate) fn broken_jitter(rules_seed: u32, stroke_seed: u32) -> [f32; 3] {
    [0, 1, 2].map(|k| ((noise_hash(rules_seed, stroke_seed as i32, k, 0) as f32 / 4_294_967_295.0) * 2.0) - 1.0)
}

/// The renderer's 32-bit GPU hash (paint.wgsl `noise_hash`), bit for bit on the CPU.
pub(crate) fn noise_hash(seed: u32, x: i32, y: i32, z: i32) -> u32 {
    let mut h = seed ^ 0x9E37_79B9;
    h = (h ^ x as u32).wrapping_mul(0x85EB_CA6B);
    h = (h ^ (h >> 13) ^ y as u32).wrapping_mul(0xC2B2_AE35);
    h = (h ^ (h >> 16) ^ z as u32).wrapping_mul(0x27D4_EB2F);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 12)
}
