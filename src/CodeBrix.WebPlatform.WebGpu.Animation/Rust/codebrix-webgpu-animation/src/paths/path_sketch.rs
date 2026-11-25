//! The SKETCH helper: a path in, a roughened, hand-drawn-looking path out (an ordinary
//! [`PathGeometry`], usable with everything a path is used with), and "boiling" - the line
//! redrawn a few times a second from the clock time. Not in C#.
//!
//! How: every contour is flattened, split at its sharp corners (which stay sharp points), each
//! run between corners resampled every `wobble_length`, every sample pushed sideways by up to
//! `roughness` (a deterministic hash of the seed, the pass, the contour and the sample), and a
//! smooth curve (Catmull-Rom, as cubic Beziers) drawn through the samples; a closed contour
//! without corners wraps round smoothly. Each pass is the same path roughened differently.
//! Everything is a function of the path and the options: the same seed gives the same line.

use kurbo::{BezPath, PathEl, Point, Vec2};

use crate::paths::{PathFillRule, PathGeometry};

/// The settings of [`PathGeometry::sketch`]. Lengths are in the path's own units (calibrated
/// drawing units for a session's shapes). Not in C#.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SketchOptions {
    /// How far the line strays from the path, at most (default 2.0). 0 gives the
    /// path back resampled and smoothed but in place.
    pub roughness: f32,
    /// The distance between wobbles along the path (default 30.0): short for a
    /// nervous line, long for a lazy one.
    pub wobble_length: f32,
    /// How many times the path is drawn over itself, each pass roughened differently (default
    /// 2); each pass is a set of contours of its own in the result.
    pub passes: u32,
    /// The seed: the same seed (and settings) gives the same line (default 1).
    pub seed: u64,
    /// A turn sharper than this, in degrees, is a CORNER and stays a sharp point (default 30);
    /// gentler turns (curves) are smoothed through.
    pub corner_angle_degrees: f32,
}

impl Default for SketchOptions {
    fn default() -> SketchOptions {
        SketchOptions { roughness: 2.0, wobble_length: 30.0, passes: 2, seed: 1, corner_angle_degrees: 30.0 }
    }
}

impl SketchOptions {
    /// "Boiling": the same options with the seed advanced `rate` times a second of `time` (the
    /// frame's clock time, seconds) - 8 to 12 a second looks like a drawn film. Driven by the
    /// clock time, so pausing the clock freezes the line and a seek always shows the same line.
    /// A rate of 0 (or below, or not finite) or a time that is not finite leaves the seed as it
    /// is. The step is [`boil_epoch`](crate::strokes::boil_epoch)`(time, rate)`.
    pub fn boiled(&self, time: f64, rate: f32) -> SketchOptions {
        // The crate's one boil timing (shared with a stroke set's StrokeBoil).
        let step = crate::strokes::boil_epoch(time, rate);
        SketchOptions { seed: self.seed.wrapping_add(step as u64), ..*self }
    }
}

/// splitmix64: a deterministic hash of a word.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A value in -1..1 from the seed and a list of indices.
fn random(seed: u64, parts: [u64; 4]) -> f64 {
    let mut h = mix(seed);
    for p in parts {
        h = mix(h ^ p);
    }
    (h >> 11) as f64 / (1u64 << 52) as f64 - 1.0
}

/// One flattened contour: its points (a closed contour without its repeated first point).
struct Polyline {
    points: Vec<Point>,
    closed: bool,
}

fn flatten(path: &BezPath, tolerance: f64) -> Vec<Polyline> {
    let mut out: Vec<Polyline> = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    let finish = |current: &mut Vec<Point>, closed: bool, out: &mut Vec<Polyline>| {
        let mut points = std::mem::take(current);
        points.dedup_by(|a, b| (*a - *b).hypot2() < 1e-18);
        if closed && points.len() > 2 && (points[0] - points[points.len() - 1]).hypot2() < 1e-18 {
            points.pop();
        }
        if points.len() >= 2 {
            out.push(Polyline { points, closed });
        }
    };
    kurbo::flatten(path.iter(), tolerance, |el| match el {
        PathEl::MoveTo(p) => {
            finish(&mut current, false, &mut out);
            current.push(p);
        }
        PathEl::LineTo(p) => current.push(p),
        PathEl::ClosePath => finish(&mut current, true, &mut out),
        _ => {}
    });
    finish(&mut current, false, &mut out);
    out
}

/// The turn at point `i` between its neighbours, radians (0 = straight on).
fn turn(a: Point, b: Point, c: Point) -> f64 {
    let (u, v) = (b - a, c - b);
    if u.hypot2() < 1e-18 || v.hypot2() < 1e-18 {
        return 0.0;
    }
    u.cross(v).atan2(u.dot(v)).abs()
}

/// Points every `spacing` (at least one interval) along a run of points, with each sample's
/// unit normal; the ends are the run's ends.
fn resample(run: &[Point], spacing: f64) -> Vec<(Point, Vec2)> {
    let lengths: Vec<f64> = run.windows(2).map(|w| (w[1] - w[0]).hypot()).collect();
    let total: f64 = lengths.iter().sum();
    let n = ((total / spacing).round() as usize).max(1);
    let normal = |d: Vec2| {
        let l = d.hypot();
        if l > 0.0 {
            Vec2::new(-d.y / l, d.x / l)
        } else {
            Vec2::ZERO
        }
    };
    let mut out = Vec::with_capacity(n + 1);
    let (mut segment, mut start) = (0usize, 0.0f64);
    for k in 0..=n {
        let at = total * k as f64 / n as f64;
        while segment + 1 < lengths.len() && start + lengths[segment] < at {
            start += lengths[segment];
            segment += 1;
        }
        let len = lengths[segment].max(1e-12);
        let f = ((at - start) / len).clamp(0.0, 1.0);
        let (a, b) = (run[segment], run[segment + 1]);
        out.push((a.lerp(b, f), normal(b - a)));
    }
    out
}

/// Catmull-Rom through `points` as cubics appended to `path` (which ends at points[0]): the end
/// tangents from the run itself, or wrapping round when `periodic`.
fn spline(path: &mut BezPath, points: &[Point], periodic: bool) {
    let n = points.len();
    let at = |i: isize| -> Point {
        if periodic {
            points[i.rem_euclid(n as isize) as usize]
        } else {
            points[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let segments = if periodic { n } else { n - 1 };
    for i in 0..segments as isize {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let c1 = p1 + (p2 - p0) / 6.0;
        let c2 = p2 - (p3 - p1) / 6.0;
        path.curve_to(c1, c2, p2);
    }
}

impl PathGeometry {
    /// The path SKETCHED: roughened as by a hand drawing it (Not in C#). An ordinary path
    /// (stroke it, trim it, morph it, dash it), with `options.passes` roughened copies of every
    /// contour; sharp corners stay sharp points; the same options give the same path. It
    /// roughens the PATH, not the pen: stroked, the line keeps an even width and one colour.
    /// For "boiling", pass `options.boiled(frame_time, rate)` each frame. The error says what to
    /// do with a setting out of range.
    pub fn sketch(&self, options: &SketchOptions) -> Result<PathGeometry, String> {
        if !options.roughness.is_finite() || options.roughness < 0.0 {
            return Err("A sketch's roughness must be 0 or more (a finite length).".to_string());
        }
        if !options.wobble_length.is_finite() || options.wobble_length <= 0.0 {
            return Err("A sketch's wobble length must be a positive, finite length.".to_string());
        }
        if options.passes == 0 {
            return Err("A sketch needs at least one pass.".to_string());
        }
        if !options.corner_angle_degrees.is_finite() || options.corner_angle_degrees < 0.0 {
            return Err("A sketch's corner angle must be 0 degrees or more.".to_string());
        }
        let wobble = options.wobble_length as f64;
        let rough = options.roughness as f64;
        let corner = (options.corner_angle_degrees as f64).to_radians();
        let contours = flatten(self.bez_path(), (wobble * 0.01).clamp(1e-3, 0.25));
        let mut out = BezPath::new();
        for pass in 0..options.passes as u64 {
            for (c, contour) in contours.iter().enumerate() {
                sketch_contour(&mut out, contour, options.seed, pass, c as u64, wobble, rough, corner);
            }
        }
        Ok(PathGeometry::from_bez_path(out, if self.fill_rule() == PathFillRule::EvenOdd { PathFillRule::EvenOdd } else { PathFillRule::NonZero }))
    }
}

#[allow(clippy::too_many_arguments)]
fn sketch_contour(out: &mut BezPath, contour: &Polyline, seed: u64, pass: u64, index: u64, wobble: f64, rough: f64, corner: f64) {
    let pts = &contour.points;
    let n = pts.len();
    // The corners (an open contour's ends are anchors too).
    let mut anchors: Vec<usize> = (0..n)
        .filter(|&i| {
            if contour.closed {
                turn(pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]) > corner
            } else {
                i == 0 || i == n - 1 || turn(pts[i - 1], pts[i], pts[i + 1]) > corner
            }
        })
        .collect();
    let jitter = |salt: u64, k: u64| random(seed, [pass, index, salt, k]);
    if contour.closed && anchors.is_empty() {
        // A smooth loop: samples all the way round, wrapped.
        let mut ring: Vec<Point> = pts.clone();
        ring.push(pts[0]);
        // At least three samples round, so a small loop stays a loop.
        let length: f64 = ring.windows(2).map(|w| (w[1] - w[0]).hypot()).sum();
        let samples = resample(&ring, wobble.min(length / 3.0).max(1e-9));
        let count = samples.len() - 1;
        let moved: Vec<Point> = samples[..count].iter().enumerate().map(|(k, (p, nrm))| *p + *nrm * (rough * jitter(1, k as u64))).collect();
        out.move_to(moved[0]);
        spline(out, &moved, true);
        out.close_path();
        return;
    }
    // A corner's own jitter, shared by the two runs that meet there.
    let anchor_point = |i: usize| -> Point { pts[i] + Vec2::new(jitter(2, i as u64), jitter(3, i as u64)) * (rough * 0.5) };
    // The runs between anchors (for a closed contour, wrapping from the last to the first).
    let mut runs: Vec<Vec<usize>> = Vec::new();
    if contour.closed {
        let first = anchors[0];
        anchors.push(first + n);
        for w in anchors.windows(2) {
            runs.push((w[0]..=w[1]).map(|i| i % n).collect());
        }
    } else {
        for w in anchors.windows(2) {
            runs.push((w[0]..=w[1]).collect());
        }
    }
    let mut started = false;
    for (r, run) in runs.iter().enumerate() {
        let points: Vec<Point> = run.iter().map(|&i| pts[i]).collect();
        let samples = resample(&points, wobble);
        let last = samples.len() - 1;
        let moved: Vec<Point> = samples
            .iter()
            .enumerate()
            .map(|(k, (p, nrm))| {
                if k == 0 {
                    anchor_point(run[0])
                } else if k == last {
                    anchor_point(run[run.len() - 1])
                } else {
                    *p + *nrm * (rough * jitter(4 + r as u64, k as u64))
                }
            })
            .collect();
        if !started {
            out.move_to(moved[0]);
            started = true;
        }
        spline(out, &moved, false);
    }
    if contour.closed {
        out.close_path();
    }
}
