//! `BrushStrokeMorph`: a stroke-level transition between two stroke sets - every stroke of one
//! painting paired with a nearby stroke of the other, each pair sliding, turning, resizing and
//! recolouring (in Oklab) from one to the other as a fraction goes from 0 to 1; strokes without a
//! partner paint themselves off (from the first painting) or on (into the second). No C#
//! equivalent.
//!
//! JUDGED BY EYE. Whether "strokes sliding into place" reads as painting rather than as particles
//! is the owner's to judge; nothing here is checked against a picture.
//!
//! LIMITS, plainly:
//! - The pairing is GREEDY (a grid hash over the second set's centres, the first set's strokes
//!   visited in a seeded order, each taking the cheapest free partner within the search radius),
//!   not optimal: no optimal-transport / Hungarian pairing (O(n^3) is out of reach). Strokes may
//!   cross on long journeys; two very different paintings look like a SHUFFLE; very different
//!   stroke counts leave many strokes to paint off or on.
//! - A stroke turns the SHORTER way using its 180-degree symmetry (a turn of more than 90 degrees
//!   turns the other way and the bend changes sign); its brush tip, seed and anchors switch from
//!   the first stroke's to the second's at the pair's halfway point (a tip that is not symmetric
//!   along the stroke shows that switch).
//! - The overlap order is the first painting's until the fraction reaches 0.5, the second's from
//!   then on (each end is then drawn exactly as its painting is drawn alone).
//! - At fraction 0 the transition draws exactly the first set, at 1 exactly the second (each
//!   stroke is then its own record, not an interpolation).

use std::rc::Rc;

use crate::color::Color;
use crate::painting::color_mix::{linear_to_srgb, oklab_to_srgb, srgb_to_linear, srgb_to_oklab};
use crate::planes::DrawingRandom;
use crate::strokes::{BrushStrokeRecord, BrushStrokeSet};

/// How [`BrushStrokeMorph::pair`] pairs strokes (no C# equivalent).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokePairingOptions {
    /// How far apart (set units) two strokes may be to be paired: positive. 40.
    pub search_radius: f32,
    /// How much a colour difference counts against position: the cost of a partner is its
    /// distance in search radii plus this times its Oklab distance in quarters (an Oklab
    /// difference of 0.25 - about black to mid grey - weighs one search radius at 1). 0 or more.
    /// 0.5.
    pub color_weight: f32,
    /// The seed of the order the first set's strokes choose their partners in. 0.
    pub seed: i64,
}

impl Default for StrokePairingOptions {
    fn default() -> StrokePairingOptions {
        StrokePairingOptions { search_radius: 40.0, color_weight: 0.5, seed: 0 }
    }
}

/// One slot of a [`BrushStrokeMorph`]: the stroke of the first set (its index) and its partner in
/// the second; a stroke with no partner has None on the other side (it paints itself off, or on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrokePair {
    /// The stroke of the first set, by index.
    pub from: Option<u32>,
    /// The stroke of the second set, by index.
    pub to: Option<u32>,
}

/// Two stroke sets paired stroke by stroke for a stroke-level transition (no C# equivalent; see
/// the module's text for its limits - it is judged by eye). Drawn by
/// [`BrushStrokesShape::morph`](crate::shapes::BrushStrokesShape::morph) at a fraction (bind
/// `AnimatedProperty::MorphFraction` to it).
#[derive(Clone, Debug, PartialEq)]
pub struct BrushStrokeMorph {
    from: BrushStrokeSet,
    to: BrushStrokeSet,
    /// Every stroke of `from` in its order (with its partner or None), then the strokes of `to`
    /// left without a partner, in their order.
    pairs: Rc<[StrokePair]>,
    seed: i64,
}

/// What a slot does over the transition (crate-internal; the GPU's flag in the target record).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PairKind {
    /// A stroke of each set: it slides, turns, resizes and recolours.
    Paired,
    /// Only in the first set: it paints itself off.
    Leaves,
    /// Only in the second set: it paints itself on.
    Arrives,
}

impl StrokePair {
    pub(crate) fn kind(&self) -> PairKind {
        match (self.from, self.to) {
            (Some(_), Some(_)) => PairKind::Paired,
            (Some(_), None) => PairKind::Leaves,
            _ => PairKind::Arrives,
        }
    }
}

impl BrushStrokeMorph {
    /// Pairs every stroke of `from` with a nearby stroke of `to`: greedy over a grid hash of `to`'s
    /// centres, `from`'s strokes visited in an order shuffled by the seed, each taking the free
    /// stroke of `to` within the search radius that costs least (distance and colour, see
    /// [`StrokePairingOptions`]; ties: the lower index). Deterministic for a seed. Strokes left
    /// over on either side have no partner: they paint off or on. The error says which option is
    /// unusable.
    pub fn pair(from: &BrushStrokeSet, to: &BrushStrokeSet, options: StrokePairingOptions) -> Result<BrushStrokeMorph, String> {
        if !(options.search_radius.is_finite() && options.search_radius > 0.0) {
            return Err("A stroke pairing's search radius must be a positive, finite distance in set units (40 is the default).".to_string());
        }
        if !(options.color_weight.is_finite() && options.color_weight >= 0.0) {
            return Err("A stroke pairing's colour weight must be 0 or more (0.5 is the default; 0 pairs by position only).".to_string());
        }
        if from.len() + to.len() > u32::MAX as usize / 2 {
            return Err("A stroke transition pairs at most two billion strokes.".to_string());
        }
        let radius = options.search_radius;
        let cell = |x: f32, y: f32| ((x / radius).floor() as i64, (y / radius).floor() as i64);
        let mut grid: std::collections::HashMap<(i64, i64), Vec<u32>> = std::collections::HashMap::new();
        for (j, r) in to.records().iter().enumerate() {
            grid.entry(cell(r.center.x, r.center.y)).or_default().push(j as u32);
        }
        let to_lab: Vec<[f32; 3]> = to.records().iter().map(|r| lab(r.color)).collect();
        let mut taken = vec![false; to.len()];
        let mut partner: Vec<Option<u32>> = vec![None; from.len()];
        // The order the strokes of `from` choose in: a seeded shuffle (Fisher-Yates).
        let mut order: Vec<u32> = (0..from.len() as u32).collect();
        let mut random = DrawingRandom::new(options.seed);
        for i in (1..order.len()).rev() {
            let k = (random.next_uint64() % (i as u64 + 1)) as usize;
            order.swap(i, k);
        }
        for &i in &order {
            let a = &from.records()[i as usize];
            let a_lab = lab(a.color);
            let (cx, cy) = cell(a.center.x, a.center.y);
            let mut best: Option<(f32, u32)> = None;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let Some(list) = grid.get_mut(&(cx + dx, cy + dy)) else { continue };
                    // Partners taken already leave the list as it is read.
                    list.retain(|j| !taken[*j as usize]);
                    for &j in list.iter() {
                        let b = &to.records()[j as usize];
                        let d = ((a.center.x - b.center.x).powi(2) + (a.center.y - b.center.y).powi(2)).sqrt();
                        if d > radius {
                            continue;
                        }
                        let cost = d / radius + options.color_weight * dist(a_lab, to_lab[j as usize]) / 0.25;
                        if best.is_none_or(|(c, k)| cost < c || (cost == c && j < k)) {
                            best = Some((cost, j));
                        }
                    }
                }
            }
            if let Some((_, j)) = best {
                taken[j as usize] = true;
                partner[i as usize] = Some(j);
            }
        }
        let mut pairs: Vec<StrokePair> = partner.iter().enumerate().map(|(i, p)| StrokePair { from: Some(i as u32), to: *p }).collect();
        pairs.extend(taken.iter().enumerate().filter(|(_, t)| !**t).map(|(j, _)| StrokePair { from: None, to: Some(j as u32) }));
        Ok(BrushStrokeMorph { from: from.clone(), to: to.clone(), pairs: pairs.into(), seed: options.seed })
    }

    /// The first set (fraction 0).
    pub fn from(&self) -> &BrushStrokeSet {
        &self.from
    }

    /// The second set (fraction 1).
    pub fn to(&self) -> &BrushStrokeSet {
        &self.to
    }

    /// The slots: every stroke of the first set in its order (with its partner, or None), then
    /// the strokes of the second set left without a partner, in their order. Every stroke of
    /// either set is in exactly one slot.
    pub fn pairs(&self) -> &[StrokePair] {
        &self.pairs
    }

    /// The number of slots (the strokes the transition draws).
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// True when both sets are empty.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// How many slots hold a stroke of each set.
    pub fn paired_count(&self) -> usize {
        self.pairs.iter().filter(|p| p.kind() == PairKind::Paired).count()
    }

    /// The pairing's seed (also what a spread's arrival order is prepared with).
    pub fn seed(&self) -> i64 {
        self.seed
    }

    /// The stroke a slot draws at a fraction of ITS OWN (0..1), on the CPU (the GPU computes the
    /// same, within float rounding) and how much of it is painted relative to its record's own
    /// progress: 1 for a pair; `1 - fraction` for a stroke painting itself off, `fraction` for one
    /// painting itself on. At 0 a pair is exactly its first stroke, at 1 exactly its second. None
    /// for a slot past the end.
    pub fn record_at(&self, slot: usize, fraction: f32) -> Option<(BrushStrokeRecord, f32)> {
        let p = self.pairs.get(slot)?;
        let (a, b) = self.ends(p);
        let m = morph_record(&a, &b, p.kind(), fraction);
        Some((m.record, m.painted))
    }

    /// The two records of a slot (a stroke without a partner stands for both).
    pub(crate) fn ends(&self, p: &StrokePair) -> (BrushStrokeRecord, BrushStrokeRecord) {
        let a = p.from.map(|i| self.from.records()[i as usize]);
        let b = p.to.map(|j| self.to.records()[j as usize]);
        match (a, b) {
            (Some(a), Some(b)) => (a, b),
            (Some(a), None) => (a, a),
            (None, Some(b)) => (b, b),
            (None, None) => unreachable!("a slot holds a stroke"),
        }
    }

    /// The slots in the order a variant draws them: 0 = the first painting's order (its strokes,
    /// then those that arrive, in the second's order); 1 = the second's (its strokes, then those
    /// that leave, in the first's order).
    pub(crate) fn variant_order(&self, variant: u8) -> Vec<u32> {
        if variant == 0 {
            return (0..self.pairs.len() as u32).collect();
        }
        let mut by_to: Vec<(u32, u32)> = self.pairs.iter().enumerate().filter_map(|(k, p)| p.to.map(|j| (j, k as u32))).collect();
        by_to.sort_unstable();
        let mut order: Vec<u32> = by_to.into_iter().map(|(_, k)| k).collect();
        order.extend(self.pairs.iter().enumerate().filter(|(_, p)| p.kind() == PairKind::Leaves).map(|(k, _)| k as u32));
        order
    }

    /// The two record buffers of a variant as the GPU reads them (64 bytes a slot each, slot k
    /// of both is one pair): the first strokes (a stroke that arrives stands for itself), and the
    /// targets - the second strokes with the slot's kind in word 13's high bits (1 leaves, 2
    /// arrives) and its delay (the spread's 0..1) in word 11.
    pub(crate) fn pack_variant(&self, delays: &[f32], variant: u8) -> (Vec<u8>, Vec<u8>) {
        use crate::strokes::stroke_gpu::{pack_record, RECORD_BYTES};
        let order = self.variant_order(variant);
        let (mut first, mut second) = (Vec::with_capacity(order.len() * RECORD_BYTES), Vec::with_capacity(order.len() * RECORD_BYTES));
        for k in order {
            let p = &self.pairs[k as usize];
            let (a, b) = self.ends(p);
            for w in pack_record(&a) {
                first.extend_from_slice(&w.to_le_bytes());
            }
            let mut words = pack_record(&b);
            words[11] = delays.get(k as usize).copied().unwrap_or(0.0).to_bits();
            words[13] |= match p.kind() {
                PairKind::Paired => 0,
                PairKind::Leaves => 1 << 16,
                PairKind::Arrives => 2 << 16,
            };
            for w in words {
                second.extend_from_slice(&w.to_le_bytes());
            }
        }
        (first, second)
    }
}

fn lab(c: Color) -> [f32; 3] {
    srgb_to_oklab([c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0])
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// A pair's own fraction under a spread: `clamp((fraction - spread x delay) / (1 - spread), 0, 1)`.
/// With spread 0 every pair moves together; with spread s a pair whose delay is d (0..1, its rank
/// in the spread's order) moves over (s d .. s d + 1 - s) of the whole. The GPU's formula.
pub(crate) fn pair_fraction(fraction: f32, spread: f32, delay: f32) -> f32 {
    ((fraction - spread * delay) / (1.0 - spread).max(1e-6)).clamp(0.0, 1.0)
}

/// What a slot draws at its own fraction (the CPU mirror of strokes.wgsl `morph_record`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Morphed {
    /// The stroke (its colour the Oklab mix, rounded to bytes).
    pub(crate) record: BrushStrokeRecord,
    /// The share of its own progress that is painted.
    pub(crate) painted: f32,
    /// The pair turns the other way (more than 90 degrees apart): the second stroke is drawn
    /// reversed, so its tip is sampled reversed from halfway on.
    pub(crate) reversed: bool,
}

/// The angle difference from `a` to `b` in degrees, wrapped into (-180, 180], and whether the
/// shorter turn uses the stroke's 180-degree symmetry (more than 90 apart: the other way round,
/// the bend negated).
pub(crate) fn shorter_turn(a: f32, b: f32) -> (f32, bool) {
    let mut d = (b - a) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d <= -180.0 {
        d += 360.0;
    }
    if d.abs() > 90.0 {
        (d - 180f32.copysign(d), true)
    } else {
        (d, false)
    }
}

/// A slot at its own fraction: at 0 (or below) exactly `a`, at 1 (or above) exactly `b`; between,
/// centre, length, width, profile, load, the anchor blend, start and end interpolated linearly,
/// the angle through the shorter turn (bend flipped when it uses the symmetry), the colour mixed
/// in Oklab (alpha linearly), the tip, seed and anchors `a`'s below halfway and `b`'s from then on.
/// A stroke leaving or arriving keeps its record and paints off / on.
pub(crate) fn morph_record(a: &BrushStrokeRecord, b: &BrushStrokeRecord, kind: PairKind, f: f32) -> Morphed {
    let f = if f.is_nan() { 0.0 } else { f };
    match kind {
        PairKind::Leaves => return Morphed { record: *a, painted: 1.0 - f.clamp(0.0, 1.0), reversed: false },
        PairKind::Arrives => return Morphed { record: *b, painted: f.clamp(0.0, 1.0), reversed: false },
        PairKind::Paired => {}
    }
    if f <= 0.0 {
        return Morphed { record: *a, painted: 1.0, reversed: false };
    }
    if f >= 1.0 {
        return Morphed { record: *b, painted: 1.0, reversed: false };
    }
    let mix = |x: f32, y: f32| x * (1.0 - f) + y * f;
    let (turn, flipped) = shorter_turn(a.angle, b.angle);
    let la = lab(a.color);
    let lb = lab(b.color);
    let rgb = oklab_to_srgb([mix(la[0], lb[0]), mix(la[1], lb[1]), mix(la[2], lb[2])]).map(|c| srgb_to_linear(c).clamp(0.0, 1.0));
    let byte = |c: f32| (linear_to_srgb(c) * 255.0).round().clamp(0.0, 255.0) as u8;
    let alpha = (mix(a.color.a as f32, b.color.a as f32)).round().clamp(0.0, 255.0) as u8;
    let late = f >= 0.5;
    let side = if late { b } else { a };
    let record = BrushStrokeRecord {
        center: crate::PointF::new(mix(a.center.x, b.center.x), mix(a.center.y, b.center.y)),
        length: mix(a.length, b.length),
        width: mix(a.width, b.width),
        angle: a.angle + turn * f,
        bend: mix(a.bend, if flipped { -b.bend } else { b.bend }),
        profile: mix(a.profile, b.profile),
        color: Color::from_rgba(byte(rgb[0]), byte(rgb[1]), byte(rgb[2]), alpha),
        load: mix(a.load, b.load),
        tip: side.tip,
        anchors: side.anchors,
        anchor_blend: mix(a.anchor_blend, b.anchor_blend),
        start: mix(a.start, b.start),
        end: mix(a.end, b.end),
        seed: side.seed,
    };
    Morphed { record, painted: 1.0, reversed: flipped && late }
}
