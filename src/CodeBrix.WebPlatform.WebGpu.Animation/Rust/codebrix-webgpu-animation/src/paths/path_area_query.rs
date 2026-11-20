//! The point queries of one geometry at one flattening tolerance (internal; C#'s
//! `PathAreaQuery`): inside tests, distances to the edge, nearest points, area-to-area
//! distances. The geometry is flattened ONCE (C#'s `Flatten`: Wang's formula, the same
//! subdivisions and the same double evaluation, each point rounded to f32 as C# stores it), and
//! every query is single-precision arithmetic in C#'s fixed order on those polylines - so the
//! answers are C#'s exactly for the same polylines.

use kurbo::PathEl;

use crate::geometry::{PointF, RectangleF};
use crate::paths::{PathFillRule, PathGeometry, PathPolyline};

/// The default flattening tolerance, calibrated units (C# `PathGeometry.DefaultFlattenTolerance`).
pub(crate) const DEFAULT_FLATTEN_TOLERANCE: f32 = 0.25;

/// C#'s `PathMath.MaxSubdivisions`.
const MAX_SUBDIVISIONS: i32 = 4096;

/// A segment in double precision: p0, c1, c2, p3 (a line keeps nothing in the controls; a
/// quadratic keeps its one control in c1) (C#'s `PathMath.Seg`).
#[derive(Clone, Copy)]
enum Seg {
    Line([f64; 4]),
    Quad([f64; 6]),
    Cubic([f64; 8]),
}

fn f(p: kurbo::Point) -> (f64, f64) {
    // C# keeps every point as single precision: round as C#'s PointF would hold it.
    ((p.x as f32) as f64, (p.y as f32) as f64)
}

fn length(dx: f64, dy: f64) -> f64 {
    ((dx * dx) + (dy * dy)).sqrt()
}

/// Wang's formula (C#'s `PathMath.Subdivisions`).
fn subdivisions(s: &Seg, tolerance: f64) -> i32 {
    let (m, factor) = match s {
        Seg::Line(_) => return 1,
        Seg::Quad([x0, y0, x1, y1, x3, y3]) => (length(x0 - (2.0 * x1) + x3, y0 - (2.0 * y1) + y3), 0.25),
        Seg::Cubic([x0, y0, x1, y1, x2, y2, x3, y3]) => {
            let m1 = length(x0 - (2.0 * x1) + x2, y0 - (2.0 * y1) + y2);
            let m2 = length(x1 - (2.0 * x2) + x3, y1 - (2.0 * y2) + y3);
            (m1.max(m2), 0.75)
        }
    };
    let n = (factor * m / tolerance).sqrt().ceil();
    if n.is_nan() || n < 1.0 {
        return 1;
    }
    if n > MAX_SUBDIVISIONS as f64 {
        MAX_SUBDIVISIONS
    } else {
        n as i32
    }
}

/// The point at parameter t (C#'s `PathMath.Evaluate`).
fn evaluate(s: &Seg, t: f64) -> (f64, f64) {
    match s {
        Seg::Line([x0, y0, x3, y3]) => (x0 + ((x3 - x0) * t), y0 + ((y3 - y0) * t)),
        Seg::Quad([x0, y0, x1, y1, x3, y3]) => {
            let u = 1.0 - t;
            let (a, b, c) = (u * u, 2.0 * u * t, t * t);
            ((a * x0) + (b * x1) + (c * x3), (a * y0) + (b * y1) + (c * y3))
        }
        Seg::Cubic([x0, y0, x1, y1, x2, y2, x3, y3]) => {
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            ((a * x0) + (b * x1) + (c * x2) + (d * x3), (a * y0) + (b * y1) + (c * y2) + (d * y3))
        }
    }
}

fn end(s: &Seg) -> (f64, f64) {
    match s {
        Seg::Line([_, _, x, y]) | Seg::Quad([_, _, _, _, x, y]) | Seg::Cubic([_, _, _, _, _, _, x, y]) => (*x, *y),
    }
}

fn p32(x: f64, y: f64) -> PointF {
    PointF::new(x as f32, y as f32)
}

/// The contours of a path as C# holds them: start point, segments, closed - with C#'s rules (a
/// contour with no segment is dropped; a closed contour whose end is not its start gets its
/// closing line as a drawn segment).
fn contours(geometry: &PathGeometry) -> Vec<((f64, f64), Vec<Seg>, bool)> {
    let mut list = Vec::new();
    let mut start = (0.0, 0.0);
    let mut last = (0.0, 0.0);
    let mut segments: Vec<Seg> = Vec::new();
    let finish = |list: &mut Vec<((f64, f64), Vec<Seg>, bool)>, start: (f64, f64), segments: &mut Vec<Seg>, last: (f64, f64), closed: bool| {
        if segments.is_empty() {
            return;
        }
        if closed && (last.0 as f32 != start.0 as f32 || last.1 as f32 != start.1 as f32) {
            segments.push(Seg::Line([last.0, last.1, start.0, start.1]));
        }
        list.push((start, std::mem::take(segments), closed));
    };
    for el in geometry.bez_path().elements() {
        match *el {
            PathEl::MoveTo(p) => {
                finish(&mut list, start, &mut segments, last, false);
                start = f(p);
                last = start;
            }
            PathEl::LineTo(p) => {
                let e = f(p);
                segments.push(Seg::Line([last.0, last.1, e.0, e.1]));
                last = e;
            }
            PathEl::QuadTo(c, p) => {
                let (c, e) = (f(c), f(p));
                segments.push(Seg::Quad([last.0, last.1, c.0, c.1, e.0, e.1]));
                last = e;
            }
            PathEl::CurveTo(c1, c2, p) => {
                let (c1, c2, e) = (f(c1), f(c2), f(p));
                segments.push(Seg::Cubic([last.0, last.1, c1.0, c1.1, c2.0, c2.1, e.0, e.1]));
                last = e;
            }
            PathEl::ClosePath => {
                finish(&mut list, start, &mut segments, last, true);
                last = start;
            }
        }
    }
    finish(&mut list, start, &mut segments, last, false);
    list
}

/// The geometry flattened (C# `PathGeometry.Flatten`): one polyline per contour.
pub(crate) fn flatten(geometry: &PathGeometry, tolerance: f32) -> Vec<PathPolyline> {
    let tolerance = tolerance as f64;
    contours(geometry)
        .into_iter()
        .map(|(start, segments, closed)| {
            let mut points = vec![p32(start.0, start.1)];
            for s in &segments {
                if !matches!(s, Seg::Line(_)) {
                    let n = subdivisions(s, tolerance);
                    for k in 1..n {
                        let (x, y) = evaluate(s, k as f64 / n as f64);
                        points.push(p32(x, y));
                    }
                }
                let (x, y) = end(s);
                points.push(p32(x, y));
            }
            if closed && points.len() > 1 && points[points.len() - 1] == points[0] {
                points.pop();
            }
            PathPolyline::new(points, closed)
        })
        .collect()
}

/// One flattened point in double precision for the polygon tools (C#'s `PathClipper.FlatPoint`):
/// `smooth` marks a point inside a divided curve (its corner is joined round, whatever the join).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FlatPoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) smooth: bool,
}

/// The geometry flattened for the polygon tools (C# `PathClipper.Flatten`, per contour): the
/// same subdivisions as [`flatten`], kept in DOUBLE precision (not rounded to f32 before the
/// grid), curve-interior points marked smooth; a closed contour does not repeat its start.
pub(crate) fn flatten_f64(geometry: &PathGeometry, tolerance: f64) -> Vec<(Vec<FlatPoint>, bool)> {
    contours(geometry)
        .into_iter()
        .map(|(start, segments, closed)| {
            let mut points = vec![FlatPoint { x: start.0, y: start.1, smooth: false }];
            for s in &segments {
                if !matches!(s, Seg::Line(_)) {
                    let n = subdivisions(s, tolerance);
                    for k in 1..n {
                        let (x, y) = evaluate(s, k as f64 / n as f64);
                        points.push(FlatPoint { x, y, smooth: true });
                    }
                }
                let (x, y) = end(s);
                points.push(FlatPoint { x, y, smooth: false });
            }
            if closed && points.len() > 1 && points[points.len() - 1].x == points[0].x && points[points.len() - 1].y == points[0].y {
                points.pop();
            }
            (points, closed)
        })
        .collect()
}

/// Checks a tolerance (None: the default); the error says what to do.
pub(crate) fn check_tolerance(tolerance: Option<f32>) -> Result<f32, String> {
    let t = tolerance.unwrap_or(DEFAULT_FLATTEN_TOLERANCE);
    if !t.is_finite() || t <= 0.0 {
        return Err("The flattening tolerance must be a positive, finite distance (0.25 is the default).".to_string());
    }
    Ok(t)
}

/// One geometry flattened at one tolerance, for point queries (C#'s `PathAreaQuery`).
#[derive(Clone, Debug)]
pub(crate) struct PathAreaQuery {
    rings: Vec<Vec<PointF>>,
    closed: Vec<bool>,
    even_odd: bool,
    bounds: RectangleF,
}

fn cross(a: PointF, b: PointF, x: f32, y: f32) -> f32 {
    ((b.x - a.x) * (y - a.y)) - ((x - a.x) * (b.y - a.y))
}

fn orient(a: PointF, b: PointF, p: PointF) -> f64 {
    ((b.x as f64 - a.x as f64) * (p.y as f64 - a.y as f64)) - ((p.x as f64 - a.x as f64) * (b.y as f64 - a.y as f64))
}

fn segments_cross(a: PointF, b: PointF, c: PointF, d: PointF) -> bool {
    let (d1, d2, d3, d4) = (orient(c, d, a), orient(c, d, b), orient(a, b, c), orient(a, b, d));
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

fn segment_t(a: PointF, b: PointF, x: f32, y: f32) -> (f32, f32, f32) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length = (dx * dx) + (dy * dy);
    let t = if length <= 0.0 { 0.0 } else { ((((x - a.x) * dx) + ((y - a.y) * dy)) / length).clamp(0.0, 1.0) };
    (t, dx, dy)
}

fn segment_distance(a: PointF, b: PointF, x: f32, y: f32) -> f32 {
    let (t, dx, dy) = segment_t(a, b, x, y);
    let ex = a.x + (t * dx) - x;
    let ey = a.y + (t * dy) - y;
    ((ex * ex) + (ey * ey)).sqrt()
}

fn segment_nearest(a: PointF, b: PointF, x: f32, y: f32) -> PointF {
    let (t, dx, dy) = segment_t(a, b, x, y);
    PointF::new(a.x + (t * dx), a.y + (t * dy))
}

impl PathAreaQuery {
    /// The query of a geometry at a tolerance (None: 0.25). The error says what to do with a
    /// tolerance that is not positive.
    pub(crate) fn new(geometry: &PathGeometry, tolerance: Option<f32>) -> Result<PathAreaQuery, String> {
        let tolerance = check_tolerance(tolerance)?;
        let lines = flatten(geometry, tolerance);
        let closed = lines.iter().map(|l| l.is_closed()).collect();
        let rings = lines.into_iter().map(|l| l.get_points().to_vec()).collect();
        Ok(PathAreaQuery { rings, closed, even_odd: geometry.fill_rule() == PathFillRule::EvenOdd, bounds: geometry.get_bounds() })
    }

    pub(crate) fn bounds(&self) -> RectangleF {
        self.bounds
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.rings.is_empty()
    }

    /// The winding number of the flattened area (every contour closed) about a point; inside by
    /// the geometry's own fill rule.
    pub(crate) fn contains(&self, x: f32, y: f32) -> bool {
        let mut winding = 0i32;
        for ring in &self.rings {
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                if a.y <= y {
                    if b.y > y && cross(a, b, x, y) > 0.0 {
                        winding += 1;
                    }
                } else if b.y <= y && cross(a, b, x, y) < 0.0 {
                    winding -= 1;
                }
            }
        }
        if self.even_odd {
            (winding & 1) != 0
        } else {
            winding != 0
        }
    }

    /// The distance to the nearest edge of the filled area (closing lines of open contours count).
    pub(crate) fn distance_to_edge(&self, x: f32, y: f32) -> f32 {
        let mut best = f32::INFINITY;
        for ring in &self.rings {
            for i in 0..ring.len() {
                best = best.min(segment_distance(ring[i], ring[(i + 1) % ring.len()], x, y));
            }
        }
        best
    }

    /// The nearest point on the path as drawn (open contours stay open).
    pub(crate) fn nearest_point(&self, x: f32, y: f32) -> PointF {
        let mut best = f32::INFINITY;
        let mut nearest = PointF::new(x, y);
        for (r, ring) in self.rings.iter().enumerate() {
            let count = if self.closed[r] || ring.len() == 1 { ring.len() } else { ring.len() - 1 };
            for i in 0..count {
                let p = segment_nearest(ring[i], ring[(i + 1) % ring.len()], x, y);
                let (dx, dy) = (p.x - x, p.y - y);
                let d = ((dx * dx) + (dy * dy)).sqrt();
                if d < best {
                    best = d;
                    nearest = p;
                }
            }
        }
        nearest
    }

    /// The smallest distance between this area and another moved by (ox, oy): 0 when they
    /// overlap (a vertex of either inside the other, or two edges crossing); else the least
    /// vertex-to-edge distance both ways.
    pub(crate) fn distance_to(&self, other: &PathAreaQuery, ox: f32, oy: f32) -> f32 {
        if self.is_empty() || other.is_empty() {
            return f32::INFINITY;
        }
        let mut best = f32::INFINITY;
        for ring in &other.rings {
            for p in ring {
                if self.contains(p.x + ox, p.y + oy) {
                    return 0.0;
                }
                best = best.min(self.distance_to_edge(p.x + ox, p.y + oy));
            }
        }
        for ring in &self.rings {
            for p in ring {
                if other.contains(p.x - ox, p.y - oy) {
                    return 0.0;
                }
                best = best.min(other.distance_to_edge(p.x - ox, p.y - oy));
            }
        }
        if best > 0.0 && self.edges_cross(other, ox, oy) {
            0.0
        } else {
            best
        }
    }

    fn edges_cross(&self, other: &PathAreaQuery, ox: f32, oy: f32) -> bool {
        for ring in &self.rings {
            for i in 0..ring.len() {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                for other_ring in &other.rings {
                    for j in 0..other_ring.len() {
                        let (c, d) = (other_ring[j], other_ring[(j + 1) % other_ring.len()]);
                        if segments_cross(a, b, PointF::new(c.x + ox, c.y + oy), PointF::new(d.x + ox, d.y + oy)) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}
