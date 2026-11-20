//! `PathMorph`: an in-between of two paths at any fraction - contours paired in order, each pair
//! resampled by length, the second's start and direction chosen to match the first (C#
//! `PathMorph`, its arithmetic port).

use kurbo::BezPath;

use crate::paths::path_measure::{contours, Contour};
use crate::paths::{PathFillRule, PathGeometry};

const DEFAULT_SAMPLE_SPACING: f64 = 2.0;
const MINIMUM_SAMPLES: usize = 16;
const MAXIMUM_SAMPLES: usize = 4096;

/// Two matched sample lists (equal length) and whether the in-between closes.
#[derive(Clone, Debug, PartialEq)]
struct Pair {
    a: Vec<(f64, f64)>,
    b: Vec<(f64, f64)>,
    closed: bool,
}

/// A morph from one path to another (C# `PathMorph`): the contours are paired in order (a
/// missing partner grows from, or shrinks to, the other's mean point); each pair is sampled at
/// equal distances - by default one sample per 2 units of the longer contour, 16 to 4096 - and
/// for two closed contours the second runs the same way round and starts at the sample that
/// matches best. [`PathMorph::get_path`] gives the in-between as straight segments between the
/// interpolated samples.
#[derive(Clone, Debug, PartialEq)]
pub struct PathMorph {
    from: PathGeometry,
    to: PathGeometry,
    sample_count: usize,
    pairs: Vec<Pair>,
}

impl PathMorph {
    /// A morph (C# `PathMorph(from, to, samplesPerContour = 0)`; 0 picks the count from the
    /// length). The error says what to do with a count of 1 or above 65536.
    pub fn new(from: PathGeometry, to: PathGeometry, samples_per_contour: usize) -> Result<PathMorph, String> {
        if samples_per_contour == 1 || samples_per_contour > 65536 {
            return Err("A morph needs 0 (automatic) or 2 to 65536 samples per contour.".to_string());
        }
        let ca = contours(from.bez_path());
        let cb = contours(to.bez_path());
        let count = ca.len().max(cb.len());
        let pairs = (0..count).map(|i| build_pair(ca.get(i), cb.get(i), samples_per_contour)).collect();
        Ok(PathMorph { from, to, sample_count: samples_per_contour, pairs })
    }

    /// The starting path.
    pub fn from(&self) -> &PathGeometry {
        &self.from
    }

    /// The ending path.
    pub fn to(&self) -> &PathGeometry {
        &self.to
    }

    /// The samples per contour asked for (0: automatic).
    pub fn sample_count(&self) -> usize {
        self.sample_count
    }

    /// The number of contour pairs.
    pub fn pair_count(&self) -> usize {
        self.pairs.len()
    }

    /// The path at a fraction (C# `GetPath`): `from` at 0 or below (and NaN), `to` at 1 or above,
    /// otherwise the in-between, with `from`'s fill rule below one half and `to`'s from there.
    pub fn get_path(&self, fraction: f32) -> PathGeometry {
        if fraction.is_nan() || fraction <= 0.0 {
            return self.from.clone();
        }
        if fraction >= 1.0 {
            return self.to.clone();
        }
        let f = fraction as f64;
        let mut path = BezPath::new();
        for pair in &self.pairs {
            let n = pair.a.len();
            if n < 2 {
                continue;
            }
            for k in 0..n {
                let (ax, ay) = pair.a[k];
                let (bx, by) = pair.b[k];
                let p = ((ax + (bx - ax) * f) as f32 as f64, (ay + (by - ay) * f) as f32 as f64);
                if k == 0 {
                    path.move_to(p);
                } else {
                    path.line_to(p);
                }
            }
            if pair.closed {
                path.close_path();
            }
        }
        let rule: PathFillRule = if fraction < 0.5 { self.from.fill_rule() } else { self.to.fill_rule() };
        PathGeometry::from_bez_path(path, rule)
    }
}

fn build_pair(a: Option<&Contour>, b: Option<&Contour>, requested: usize) -> Pair {
    let (la, lb) = (a.map_or(0.0, |c| c.total), b.map_or(0.0, |c| c.total));
    let n = if requested > 0 { requested } else { ((la.max(lb) / DEFAULT_SAMPLE_SPACING).ceil() as usize).clamp(MINIMUM_SAMPLES, MAXIMUM_SAMPLES) };
    let closed_a = a.map(|c| c.closed).unwrap_or_else(|| b.is_some_and(|c| c.closed));
    let closed_b = b.map(|c| c.closed).unwrap_or(closed_a);
    if closed_a && closed_b {
        let mut sa = a.map(|c| sample_loop(c, n));
        let mut sb = b.map(|c| sample_loop(c, n));
        if sa.is_none() {
            sa = Some(collapse(sb.as_ref().unwrap()));
        }
        if sb.is_none() {
            sb = Some(collapse(sa.as_ref().unwrap()));
        }
        let (sa, mut sb) = (sa.unwrap(), sb.unwrap());
        if a.is_some() && b.is_some() {
            if signed_area(&sa) * signed_area(&sb) < 0.0 {
                sb = reverse_loop(&sb);
            }
            let shift = best_shift(&sa, &sb);
            sb = rotate(&sb, shift);
        }
        return Pair { a: sa, b: sb, closed: true };
    }
    if !closed_a && !closed_b {
        let mut sa = a.map(|c| sample_open(c, n));
        let mut sb = b.map(|c| sample_open(c, n));
        if sa.is_none() {
            sa = Some(collapse(sb.as_ref().unwrap()));
        }
        if sb.is_none() {
            sb = Some(collapse(sa.as_ref().unwrap()));
        }
        let (sa, mut sb) = (sa.unwrap(), sb.unwrap());
        if a.is_some() && b.is_some() {
            let reversed: Vec<(f64, f64)> = sb.iter().rev().cloned().collect();
            if squared_distance(&sa, &reversed, 0) < squared_distance(&sa, &sb, 0) {
                sb = reversed;
            }
        }
        return Pair { a: sa, b: sb, closed: false };
    }
    // One open, one closed: the loop is cut at its best sample and run back to it.
    let (open, lp) = if closed_a { (b.unwrap(), a.unwrap()) } else { (a.unwrap(), b.unwrap()) };
    let open_samples = sample_open(open, n + 1);
    let forward = sample_loop(lp, n);
    let backward = reverse_loop(&forward);
    let mut cut = Vec::new();
    let mut best = f64::MAX;
    for direction in [&forward, &backward] {
        for s in 0..n {
            let candidate = open_up(direction, s);
            let d = squared_distance(&open_samples, &candidate, 0);
            if d < best {
                best = d;
                cut = candidate;
            }
        }
    }
    if closed_a {
        Pair { a: cut, b: open_samples, closed: false }
    } else {
        Pair { a: open_samples, b: cut, closed: false }
    }
}

/// n samples at distances L * k / n round a closed contour.
fn sample_loop(c: &Contour, n: usize) -> Vec<(f64, f64)> {
    (0..n).map(|k| c.point_at(c.total * k as f64 / n as f64).0).map(|p| (p.x, p.y)).collect()
}

/// n samples at distances L * k / (n - 1) along an open contour, both ends included.
fn sample_open(c: &Contour, n: usize) -> Vec<(f64, f64)> {
    (0..n).map(|k| c.point_at(c.total * k as f64 / (n - 1) as f64).0).map(|p| (p.x, p.y)).collect()
}

/// A partner for a missing contour: every sample at the other's mean point.
fn collapse(samples: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = samples.len() as f64;
    let (sx, sy) = samples.iter().fold((0.0, 0.0), |(x, y), p| (x + p.0, y + p.1));
    vec![(sx / n, sy / n); samples.len()]
}

/// Twice the shoelace area: positive for clockwise travel on screen (y down).
fn signed_area(s: &[(f64, f64)]) -> f64 {
    let n = s.len();
    (0..n).map(|k| {
        let j = (k + 1) % n;
        (s[k].0 * s[j].1) - (s[j].0 * s[k].1)
    }).sum()
}

/// The loop the other way round, keeping sample 0 first.
fn reverse_loop(s: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = s.len();
    (0..n).map(|k| s[(n - k) % n]).collect()
}

/// The loop starting at sample `shift`.
fn rotate(s: &[(f64, f64)], shift: usize) -> Vec<(f64, f64)> {
    let n = s.len();
    (0..n).map(|k| s[(k + shift) % n]).collect()
}

/// The loop starting at sample `shift` and running back to it (n + 1 samples).
fn open_up(s: &[(f64, f64)], shift: usize) -> Vec<(f64, f64)> {
    let n = s.len();
    (0..=n).map(|k| s[(k + shift) % n]).collect()
}

fn best_shift(a: &[(f64, f64)], b: &[(f64, f64)]) -> usize {
    let mut best_shift = 0;
    let mut best = f64::MAX;
    for s in 0..a.len() {
        let d = squared_distance(a, b, s);
        if d < best {
            best = d;
            best_shift = s;
        }
    }
    best_shift
}

/// Sum over k of |a[k] - b[(k + shift) mod n]|^2, in k order.
fn squared_distance(a: &[(f64, f64)], b: &[(f64, f64)], shift: usize) -> f64 {
    let n = a.len();
    (0..n).map(|k| {
        let j = (k + shift) % n;
        let (dx, dy) = (a[k].0 - b[j].0, a[k].1 - b[j].1);
        dx * dx + dy * dy
    }).sum()
}
