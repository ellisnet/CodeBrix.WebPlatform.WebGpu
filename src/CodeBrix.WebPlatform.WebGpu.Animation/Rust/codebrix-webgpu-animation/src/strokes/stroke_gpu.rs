//! Crate-internal: how a stroke set reaches the GPU and the CPU mirrors of what strokes.wgsl
//! computes - the 64-byte record, the per-draw block, the 8-vertex hull and the coverage of the
//! analytic lozenge - so the host tests can check the shader's rules (no C# equivalent).
//!
//! THE RECORD (64 bytes, four vec4 of the storage buffer `records`):
//! - words 0..3 (f32): centre x, centre y, angle in radians, length;
//! - words 4..7 (f32): width, bend, profile, load;
//! - words 8..11 (f32): start, end, anchor blend, 0;
//! - words 12..15 (u32): the colour as straight sRGB bytes `r | g << 8 | b << 16 | a << 24`,
//!   `tip | flags << 16`, `anchor0 | anchor1 << 16`, the seed.
//!
//! A TRANSITION's draw binds two record buffers of the same length (group 2, bindings 0 and 1;
//! strokes/brush_stroke_morph.rs `pack_variant`): slot k of both is one pair. In the TARGET
//! records word 11 is the slot's delay under the spread (0..1) and word 13's flags say what the
//! slot does (0 travels, 1 paints off, 2 paints on). A plain set binds its records at both.
//!
//! THE DRAW BLOCK (one per stroke draw per frame, at a 256-byte-aligned dynamic offset of the
//! frame's uniform buffer; `DRAW_HEADER_VEC4S` vec4 of header, then two vec4 per anchor):
//! - h0 (f32): the element-to-device matrix a b c d (x' = a x + c y + e);
//! - h1 (f32): e, f, the draw's time (session seconds minus the shape's offset), the opacity;
//! - h2 (f32): head softness, the impasto of the finished layer the set is on (the height pass's
//!   scale; 0 off a finished layer), a transition's fraction and spread (0 for a plain set);
//! - h3 (u32): flags (`FLAG_*`), anchor count, the boil's redraw number (i32 bits), the boil's
//!   seed;
//! - h4 (f32): the boil's offset, turn (radians), size, 0;
//! - h5 (f32): the flow's frequency, speed, turn (radians), 0;
//! - h6 (u32): the flow's seed, 0, 0, 0;
//! - h7 to h43: the picked colour (zero for a set in its own colours): the place, the colour
//!   flags, texels, taps and pigment count, the own weight and the rules, the palette in Oklab -
//!   laid out in strokes/stroke_color_gpu.rs;
//! - then per anchor (a b c d), (e f 0 0).

use crate::color::Color;
use crate::strokes::BrushStrokeRecord;

/// Bytes of one record in the GPU's storage buffer.
pub(crate) const RECORD_BYTES: usize = 64;
/// The draw block's header, vec4s (seven of the stroke's own, then the picked colour's).
pub(crate) const DRAW_HEADER_VEC4S: usize = 7 + crate::strokes::stroke_color_gpu::COLOUR_VEC4S;
/// The most anchors a shape has (two vec4 each in the draw block).
pub(crate) const MAX_ANCHORS: usize = 256;
/// The bytes a draw block's binding covers (the header and every anchor slot).
pub(crate) const DRAW_BINDING_BYTES: usize = (DRAW_HEADER_VEC4S + (2 * MAX_ANCHORS)) * 16;
/// The alignment of draw blocks (WebGPU's minimum uniform offset alignment).
pub(crate) const DRAW_ALIGN: usize = 256;

/// Draw flag: the shape has tips (and the look keeps them).
pub(crate) const FLAG_TIPS: u32 = 1;
/// Draw flag: mirror a stroke's tip across by its seed's low bit.
pub(crate) const FLAG_MIRROR: u32 = 2;
/// Draw flag: boiling.
pub(crate) const FLAG_BOIL: u32 = 4;
/// Draw flag: flow.
pub(crate) const FLAG_FLOW: u32 = 8;
/// Draw flag: a stroke-level transition (the targets at group 2 binding 1; h2.z the fraction,
/// h2.w the spread).
pub(crate) const FLAG_MORPH: u32 = 16;

/// The colour as the record's word: straight sRGB bytes, red lowest.
pub(crate) fn pack_color(c: Color) -> u32 {
    c.r as u32 | ((c.g as u32) << 8) | ((c.b as u32) << 16) | ((c.a as u32) << 24)
}

/// One record as the GPU reads it.
pub(crate) fn pack_record(r: &BrushStrokeRecord) -> [u32; 16] {
    let f = |v: f32| v.to_bits();
    [
        f(r.center.x),
        f(r.center.y),
        f((r.angle as f64).to_radians() as f32),
        f(r.length),
        f(r.width),
        f(r.bend),
        f(r.profile),
        f(r.load),
        f(r.start),
        f(r.end),
        f(r.anchor_blend),
        0,
        pack_color(r.color),
        r.tip as u32,
        r.anchors[0] as u32 | ((r.anchors[1] as u32) << 16),
        r.seed,
    ]
}

/// Every record of a set, as the bytes of the one upload.
pub(crate) fn pack_records(records: &[BrushStrokeRecord]) -> Vec<u8> {
    let mut out = Vec::with_capacity(records.len() * RECORD_BYTES);
    for r in records {
        for w in pack_record(r) {
            out.extend_from_slice(&w.to_le_bytes());
        }
    }
    out
}

#[cfg(test)]
/// The shader's progress at a time in f32 (BrushStrokeRecord::progress_at's rule): exactly 1 from
/// the end on (a comparison, not a division - what lets the checkpoint cache know on the CPU which
/// strokes the GPU draws whole), 0 before the start.
pub(crate) fn progress_f32(start: f32, end: f32, t: f32) -> f32 {
    if t >= end {
        return 1.0;
    }
    if !(t >= start) {
        return 0.0;
    }
    ((t - start) / (end - start)).clamp(0.0, 1.0)
}

#[cfg(test)]
/// The centre line's offset across the axis at s (`bend x length x (4 s^2 - 1)`, `bow` = bend x
/// length).
pub(crate) fn centre_at(bow: f32, s: f32) -> f32 {
    bow * ((4.0 * s * s) - 1.0)
}

#[cfg(test)]
/// The half width at s (`half x sqrt(max(0, 1 - |2 s|^profile))`).
pub(crate) fn half_at(half: f32, profile: f32, s: f32) -> f32 {
    half * (1.0 - (2.0 * s).abs().powf(profile)).max(0.0).sqrt()
}

#[cfg(test)]
/// The lowest and highest across-axis edges of the stroke over `a..b` (conservative: the centre
/// line's extremes plus the widest half width there).
pub(crate) fn extent(bow: f32, half: f32, profile: f32, a: f32, b: f32) -> (f32, f32) {
    let (ca, cb) = (centre_at(bow, a), centre_at(bow, b));
    let (mut lo, mut hi) = (ca.min(cb), ca.max(cb));
    if a <= 0.0 && b >= 0.0 {
        lo = lo.min(-bow);
        hi = hi.max(-bow);
    }
    let h = half_at(half, profile, 0f32.clamp(a, b));
    (lo - h, hi + h)
}

#[cfg(test)]
/// The 8 vertices (s, across) of the hull strip the vertex shader emits for a stroke painted to
/// `progress`: four stations along the painted part (at 0, 1/4, 3/4 and all of it), each with
/// its lowest and highest edge over the stations either side - so every straight edge of the
/// strip lies outside the outline - grown by `grow_s` along (the end stations) and `grow_a`
/// across (one device pixel each, for the anti-aliasing). Order: (s0, low), (s0, high), (s1,
/// low), ... - a triangle strip.
pub(crate) fn hull(bow: f32, half: f32, profile: f32, progress: f32, grow_s: f32, grow_a: f32) -> [(f32, f32); 8] {
    let k = [0.0, 0.25, 0.75, 1.0];
    let s = k.map(|k| -0.5 + k * progress);
    let spans = [(s[0], s[1]), (s[0], s[2]), (s[1], s[3]), (s[2], s[3])];
    let mut out = [(0.0, 0.0); 8];
    for i in 0..4 {
        let (lo, hi) = extent(bow, half, profile, spans[i].0, spans[i].1);
        let si = match i {
            0 => s[0] - grow_s,
            3 => s[3] + grow_s,
            _ => s[i],
        };
        out[2 * i] = (si, lo - grow_a);
        out[2 * i + 1] = (si, hi + grow_a);
    }
    out
}

#[cfg(test)]
/// The CPU mirror of the fragment shader's coverage, hard-edged (the GPU smooths the same
/// outline over a device pixel): 1 inside the painted part of the analytic lozenge at (s along
/// from -0.5 to 0.5, `across` in set units), else 0.
pub(crate) fn stroke_coverage(record: &BrushStrokeRecord, progress: f32, s: f32, across: f32) -> f32 {
    if !(progress > 0.0) || s < -0.5 || s > -0.5 + progress {
        return 0.0;
    }
    let c = centre_at(record.bend * record.length, s);
    let h = half_at(record.width / 2.0, record.profile, s);
    if (across - c).abs() < h {
        1.0
    } else {
        0.0
    }
}

#[cfg(test)]
/// A point in set coordinates as (s, across) of a record (the inverse of BrushStrokeShape's
/// placement).
pub(crate) fn to_local(record: &BrushStrokeRecord, x: f32, y: f32) -> (f32, f32) {
    let r = (record.angle as f64).to_radians();
    let (cos, sin) = (r.cos(), r.sin());
    let (dx, dy) = ((x - record.center.x) as f64, (y - record.center.y) as f64);
    let along = dx * cos + dy * sin;
    let across = -dx * sin + dy * cos;
    ((along / record.length as f64) as f32, across as f32)
}

/// The size of a draw block with `anchors` anchors, rounded up to the alignment.
pub(crate) fn draw_block_bytes(anchors: usize) -> usize {
    ((DRAW_HEADER_VEC4S + 2 * anchors) * 16).div_ceil(DRAW_ALIGN) * DRAW_ALIGN
}
