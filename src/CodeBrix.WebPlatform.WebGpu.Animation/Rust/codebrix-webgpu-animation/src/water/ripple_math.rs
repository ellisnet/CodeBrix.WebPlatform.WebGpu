//! The ripple field's GPU half ON THE CPU (no C# equivalent): the records a field is packed into
//! for the frame's `ripples` storage buffer, the field's header for the composite's pass record,
//! and the evaluation of ONE POINT from them in single precision - step for step what the
//! composite shader does per pixel. [`RippleField::height_at`](super::RippleField::height_at)
//! and its siblings evaluate through this code, so the CPU twin and the GPU agree within float
//! rounding.
//!
//! THE RECORDS (the buffer is an array of `vec4<f32>`, 16 bytes each, little-endian):
//! - a DROP is two vec4 (32 bytes): `[centre x, centre y, front, amplitude]`,
//!   `[wavelength, ring width, age, 0]` - all in calibrated units (the age in seconds). The front
//!   (`speed x age`) and the amplitude (`strength x height x e^(-age/decay) x ease(age)`) are
//!   worked out on the CPU in double precision, so the shader never sees the session time;
//! - a SWELL WAVE is one vec4 (16 bytes): `[k dx, k dy, height, phase now]` with `k = 2 pi /
//!   wavelength`, `d` the unit direction it travels and `phase now = phase - k speed t` reduced to
//!   0..2 pi in double precision (exact for hours).
//!
//! A field's records are its drops (in the order of
//! [`RippleField::live_drops`](super::RippleField::live_drops)) followed by its swell waves.
//! THE HEADER (32 bytes: one vec4 of f32, one vec4 of u32): `[left, top, right, bottom]` of the
//! field's place, then `[record offset (in vec4), drop count, swell wave count, flags]` with
//! flag bit 0 = walls (each drop also sounds from its 8 mirror images across the place's edges).
//!
//! ONE DROP at distance `r` (`u = r - front`, `lambda` = wavelength, `w` = ring width, `k = 2 pi /
//! lambda`): `h = amplitude S(r) E(u) sin(k u)` with `S(r) = 1 / sqrt(1 + r / lambda)`, `E(u) =
//! exp(-(u / c)^2)` where `c = w` behind the front (`u < 0`) and `c = lambda / 4` ahead of it.
//! Early out (about five operations): nothing when `u > lambda` (ahead; `E < 1.2e-7`) or `u < -3 w`
//! (behind; `E < 1.3e-4`). The slope is `dh/dr (x - cx, y - cy) / r` (0 at the centre); the focus
//! is the Laplacian `d2h/dr2 + (1 / r) dh/dr` with every factor's derivative kept (exact), `r`
//! held at least `lambda / 4` in the `1 / r` term (the centre of a young drop is a cone's tip).
//! A swell wave adds `height sin(phase)` with `phase = k d . p + phase now`, slope `height k d
//! cos(phase)`, focus `-k^2 h`. Everything is summed (rings pass through each other).

/// vec4s per drop record (32 bytes).
pub(crate) const DROP_VEC4S: usize = 2;
/// vec4s per swell wave record (16 bytes).
pub(crate) const SWELL_VEC4S: usize = 1;
// Only the tests use this: the frame's ripple upload (rendering/core/gpu.rs) writes the records
// as they are (plain f32 data).
#[cfg_attr(not(test), allow(dead_code))]
/// Bytes of a field's header (the composite pass record's ripple part).
pub(crate) const HEADER_BYTES: usize = 32;
/// Header flag: the place's edges reflect the rings (mirror images, first bounce).
pub(crate) const FLAG_WALLS: u32 = 1;
/// Behind the front a drop is skipped beyond this many ring widths.
pub(crate) const BEHIND_REACH: f32 = 3.0;
/// Ahead of the front a drop is skipped beyond this many wavelengths.
pub(crate) const AHEAD_REACH: f32 = 1.0;

/// A packed field's header: where its records are and how to read them (crate-internal).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RippleGpuHeader {
    /// Left, top, right, bottom of the field's place (calibrated units).
    pub(crate) place: [f32; 4],
    /// Record offset (in vec4 units), drop count, swell wave count, flags.
    pub(crate) words: [u32; 4],
}

impl RippleGpuHeader {
    // Only the tests use this: the header's words go into the composite's pass data as they are.
    #[cfg_attr(not(test), allow(dead_code))]
    /// The header's 32 bytes, little-endian, as the pass record carries them.
    pub(crate) fn to_bytes(self) -> [u8; HEADER_BYTES] {
        let mut out = [0u8; HEADER_BYTES];
        for (i, v) in self.place.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        for (i, v) in self.words.iter().enumerate() {
            out[16 + i * 4..16 + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        out
    }
}

/// A field packed for one frame (crate-internal): its header, its records, and how many live
/// drops the cap left out.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct PackedRipples {
    /// The header (its record offset counts from the start of `records`' buffer).
    pub(crate) header: RippleGpuHeader,
    /// Drop records then swell records.
    pub(crate) records: Vec<[f32; 4]>,
    /// Live drops left out by the cap (the weakest at that time).
    pub(crate) skipped: usize,
}

/// The water at one point: height, slope and Laplacian, in calibrated units (crate-internal).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RippleSample {
    /// The height.
    pub(crate) height: f32,
    /// dh/dx, dh/dy.
    pub(crate) slope: [f32; 2],
    /// The Laplacian (where light focuses: negative on crests).
    pub(crate) focus: f32,
}

/// The two vec4 of one drop's record.
pub(crate) fn drop_record(center: [f32; 2], front: f32, amplitude: f32, wavelength: f32, ring_width: f32, age: f32) -> [[f32; 4]; 2] {
    [[center[0], center[1], front, amplitude], [wavelength, ring_width, age, 0.0]]
}

// Only the tests use this: the frame's ripple upload (rendering/core/gpu.rs) writes the records
// as they are (plain f32 data).
#[cfg_attr(not(test), allow(dead_code))]
/// Records as the bytes the storage buffer takes (little-endian f32).
pub(crate) fn records_to_bytes(records: &[[f32; 4]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(records.len() * 16);
    for record in records {
        for v in record {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    out
}

/// One point of a packed field, exactly as the composite shader evaluates a pixel. `records` is
/// the whole buffer the header's offset points into.
pub(crate) fn evaluate(header: &RippleGpuHeader, records: &[[f32; 4]], x: f32, y: f32) -> RippleSample {
    let mut sample = RippleSample::default();
    let offset = header.words[0] as usize;
    let drops = header.words[1] as usize;
    let waves = header.words[2] as usize;
    let walls = (header.words[3] & FLAG_WALLS) != 0;
    let [left, top, right, bottom] = header.place;
    for i in 0..drops {
        let a = records[offset + i * DROP_VEC4S];
        let b = records[offset + i * DROP_VEC4S + 1];
        if walls {
            let xs = [2.0 * left - a[0], a[0], 2.0 * right - a[0]];
            let ys = [2.0 * top - a[1], a[1], 2.0 * bottom - a[1]];
            for cy in ys {
                for cx in xs {
                    add_ring(&mut sample, x, y, cx, cy, a, b);
                }
            }
        } else {
            add_ring(&mut sample, x, y, a[0], a[1], a, b);
        }
    }
    let base = offset + drops * DROP_VEC4S;
    for j in 0..waves {
        let w = records[base + j * SWELL_VEC4S];
        let phase = w[0] * x + w[1] * y + w[3];
        let (sn, cs) = (phase.sin(), phase.cos());
        let h = w[2] * sn;
        sample.height += h;
        sample.slope[0] += w[2] * cs * w[0];
        sample.slope[1] += w[2] * cs * w[1];
        sample.focus -= (w[0] * w[0] + w[1] * w[1]) * h;
    }
    sample
}

/// One source of rings (a drop or one of its mirror images) added to a sample.
fn add_ring(sample: &mut RippleSample, x: f32, y: f32, cx: f32, cy: f32, a: [f32; 4], b: [f32; 4]) {
    let (front, amplitude, lambda, width) = (a[2], a[3], b[0], b[1]);
    let dx = x - cx;
    let dy = y - cy;
    let r = (dx * dx + dy * dy).sqrt();
    let u = r - front;
    if u > AHEAD_REACH * lambda || u < -BEHIND_REACH * width {
        return;
    }
    let k = std::f32::consts::TAU / lambda;
    let c = if u < 0.0 { width } else { 0.25 * lambda };
    let q = u / c;
    let e = (-(q * q)).exp();
    let e1 = -2.0 * q / c * e;
    let e2 = (4.0 * q * q - 2.0) / (c * c) * e;
    let g = 1.0 + r / lambda;
    let s = 1.0 / g.sqrt();
    let s1 = -0.5 / lambda * s / g;
    let s2 = 0.75 / (lambda * lambda) * s / (g * g);
    let (sn, cs) = ((k * u).sin(), (k * u).cos());
    let p1 = k * cs;
    let p2 = -k * k * sn;
    let h = amplitude * s * e * sn;
    let h1 = amplitude * (s1 * e * sn + s * e1 * sn + s * e * p1);
    let h2 = amplitude * (s2 * e * sn + s * e2 * sn + s * e * p2 + 2.0 * (s1 * e1 * sn + s1 * e * p1 + s * e1 * p1));
    sample.height += h;
    if r > 0.0 {
        sample.slope[0] += h1 * dx / r;
        sample.slope[1] += h1 * dy / r;
    }
    sample.focus += h2 + h1 / r.max(0.25 * lambda);
}
