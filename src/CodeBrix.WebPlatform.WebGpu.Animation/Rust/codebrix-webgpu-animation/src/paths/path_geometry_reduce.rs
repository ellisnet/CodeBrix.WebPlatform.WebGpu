//! POINT REDUCTION on `PathGeometry` (Not in C#): `reduce_points` drops the corners of line
//! runs that lie within a tolerance of the line through their neighbours (Ramer-Douglas-Peucker),
//! for the polygons path combining and sketching make, whose points are fixed in calibrated
//! units however small the path is drawn. Curves are left as they are; their end points are
//! never dropped.

use kurbo::{BezPath, PathEl, Point};

use crate::paths::PathGeometry;

impl PathGeometry {
    /// The same path with fewer points (Not in C#; not [`simplify`](PathGeometry::simplify),
    /// which removes overlaps): in each run of straight lines, a point is dropped when it lies
    /// within `tolerance` (calibrated units) of the line that replaces it (Ramer-Douglas-Peucker),
    /// so no point of the path moves farther than `tolerance` from the result. Curves, and the
    /// points where they start and end, are kept as they are; so are each contour's start point
    /// (its MoveTo, where trimming and dashing start) and the fill rule. A closed contour of
    /// lines keeps at least three points (a triangle), an open run its two ends. A tolerance
    /// that is not a positive number (0, negative, NaN) returns the path unchanged.
    pub fn reduce_points(&self, tolerance: f32) -> PathGeometry {
        if !(tolerance > 0.0) {
            return self.clone();
        }
        let tolerance = tolerance as f64;
        let mut out = BezPath::new();
        let mut run: Vec<Point> = Vec::new();
        let mut start = Point::ZERO;
        let mut curves = false;
        for el in self.bez_path().elements() {
            match *el {
                PathEl::MoveTo(p) => {
                    flush(&mut out, &mut run, tolerance, None);
                    out.move_to(p);
                    run.push(p);
                    start = p;
                    curves = false;
                }
                PathEl::LineTo(p) => {
                    if run.is_empty() {
                        run.push(start);
                    }
                    run.push(p);
                }
                PathEl::QuadTo(a, b) => {
                    flush(&mut out, &mut run, tolerance, None);
                    out.quad_to(a, b);
                    run.push(b);
                    curves = true;
                }
                PathEl::CurveTo(a, b, c) => {
                    flush(&mut out, &mut run, tolerance, None);
                    out.curve_to(a, b, c);
                    run.push(c);
                    curves = true;
                }
                PathEl::ClosePath => {
                    flush(&mut out, &mut run, tolerance, Some((start, !curves)));
                    out.close_path();
                }
            }
        }
        flush(&mut out, &mut run, tolerance, None);
        PathGeometry::from_bez_path(out, self.fill_rule())
    }
}

/// Writes a run of lines (its first point already written) reduced, and empties it. `closing`:
/// the run ends its contour's ClosePath - the contour's start point, and whether the contour is
/// lines only (it then keeps a triangle). The closing line back to the start is reduced with the
/// run but written only when the run had it as a line of its own.
fn flush(out: &mut BezPath, run: &mut Vec<Point>, tolerance: f64, closing: Option<(Point, bool)>) {
    let explicit_close = closing.is_some_and(|(start, _)| run.last() == Some(&start));
    if let Some((start, _)) = closing {
        if !explicit_close && !run.is_empty() {
            run.push(start);
        }
    }
    if run.len() >= 2 {
        let mut keep = reduce(run, tolerance);
        if closing.is_some_and(|(_, lines_only)| lines_only) {
            keep_a_triangle(run, &mut keep);
        }
        let last = run.len() - 1;
        for (i, p) in run.iter().enumerate().skip(1) {
            if keep[i] && (i < last || closing.is_none() || explicit_close) {
                out.line_to(*p);
            }
        }
    }
    run.clear();
}

/// Ramer-Douglas-Peucker over a run: which points are kept (always both ends). A span whose
/// ends are the same point measures its points' distances from that point.
fn reduce(points: &[Point], tolerance: f64) -> Vec<bool> {
    let mut keep = vec![false; points.len()];
    let last = points.len() - 1;
    keep[0] = true;
    keep[last] = true;
    let mut spans = vec![(0, last)];
    while let Some((a, b)) = spans.pop() {
        if b <= a + 1 {
            continue;
        }
        let (i, d) = farthest(points, a, b);
        if d > tolerance {
            keep[i] = true;
            spans.push((i, b));
            spans.push((a, i));
        }
    }
    keep
}

/// The point strictly between `a` and `b` farthest from the segment between them (the first of
/// equals), and its distance.
fn farthest(points: &[Point], a: usize, b: usize) -> (usize, f64) {
    let mut best = (a + 1, -1.0);
    for (i, p) in points.iter().enumerate().take(b).skip(a + 1) {
        let d = segment_distance(*p, points[a], points[b]);
        if d > best.1 {
            best = (i, d);
        }
    }
    best
}

/// The distance from a point to the segment a-b (to a when they are the same point).
fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let len2 = ab.hypot2();
    if len2 == 0.0 {
        return (p - a).hypot();
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p - (a + ab * t)).hypot()
}

/// A closed run of lines (start .. start) keeps at least two points between its ends, when it
/// has them: the point farthest from the start, then the point farthest from the segment
/// between those two.
fn keep_a_triangle(points: &[Point], keep: &mut [bool]) {
    let last = points.len() - 1;
    if last < 3 {
        return;
    }
    let kept: Vec<usize> = (1..last).filter(|i| keep[*i]).collect();
    if kept.len() >= 2 {
        return;
    }
    let far = match kept.first() {
        Some(i) => *i,
        None => {
            let (i, _) = farthest(points, 0, last);
            keep[i] = true;
            i
        }
    };
    let mut best = (0, -1.0);
    for (i, p) in points.iter().enumerate().take(last).skip(1) {
        if i == far {
            continue;
        }
        let d = segment_distance(*p, points[0], points[far]);
        if d > best.1 {
            best = (i, d);
        }
    }
    if best.0 > 0 {
        keep[best.0] = true;
    }
}
