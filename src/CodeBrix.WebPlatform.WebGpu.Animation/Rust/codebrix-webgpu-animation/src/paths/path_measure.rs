//! Measuring a path along its length (internal): contours of kurbo segments with their arc
//! lengths, the point and direction at a distance, and the piece between two distances - what
//! C#'s `PathContour.Measure` does for `GetPointAt`, `Trim` and `PathMorph`. Arc lengths are
//! kurbo's (to 1e-4 units), not C#'s flattening, so values on curves agree with C# to that
//! accuracy, not bit for bit; on straight segments they are exact.

use kurbo::{BezPath, ParamCurve, ParamCurveArclen, ParamCurveDeriv, PathEl, PathSeg, Point, Vec2};

/// The accuracy of every arc length, calibrated units.
const ACCURACY: f64 = 1e-4;

/// One contour: its segments (the closing line of a closed contour included), each with its
/// length, and whether it is closed.
#[derive(Clone, Debug)]
pub(crate) struct Contour {
    pub(crate) segments: Vec<(PathSeg, f64)>,
    pub(crate) closed: bool,
    pub(crate) start: Point,
    pub(crate) total: f64,
}

impl Contour {
    /// The point and unit direction at a distance along the contour (clamped).
    pub(crate) fn point_at(&self, distance: f64) -> (Point, Vec2) {
        if self.segments.is_empty() {
            return (self.start, Vec2::new(1.0, 0.0));
        }
        let mut left = distance.clamp(0.0, self.total);
        let last = self.segments.len() - 1;
        for (i, (seg, len)) in self.segments.iter().enumerate() {
            if left <= *len || i == last {
                let t = if *len > 0.0 { seg.inv_arclen(left.min(*len), ACCURACY) } else { 0.0 };
                return (seg.eval(t), tangent(seg, t));
            }
            left -= len;
        }
        unreachable!()
    }

    /// The segments between two distances (from <= to, both within the contour).
    pub(crate) fn extract(&self, from: f64, to: f64, out: &mut Vec<PathSeg>) {
        let mut at = 0.0;
        for (seg, len) in &self.segments {
            let (s, e) = (at, at + len);
            at = e;
            if e <= from || s >= to || *len <= 0.0 {
                continue;
            }
            let t0 = if from > s { seg.inv_arclen(from - s, ACCURACY) } else { 0.0 };
            let t1 = if to < e { seg.inv_arclen(to - s, ACCURACY) } else { 1.0 };
            if t1 > t0 {
                out.push(seg.subsegment(t0..t1));
            }
        }
    }
}

/// The unit direction of a segment at a parameter (from a nearby chord where the derivative
/// vanishes, as at a cubic's doubled control point).
fn tangent(seg: &PathSeg, t: f64) -> Vec2 {
    let d = match seg {
        PathSeg::Line(l) => l.p1 - l.p0,
        PathSeg::Quad(q) => q.deriv().eval(t).to_vec2(),
        PathSeg::Cubic(c) => c.deriv().eval(t).to_vec2(),
    };
    let d = if d.hypot() > 1e-12 {
        d
    } else {
        let (a, b) = if t < 0.5 { (t, (t + 1e-3).min(1.0)) } else { ((t - 1e-3).max(0.0), t) };
        seg.eval(b) - seg.eval(a)
    };
    let h = d.hypot();
    if h > 0.0 {
        d / h
    } else {
        Vec2::new(1.0, 0.0)
    }
}

/// The contours of a path, measured.
pub(crate) fn contours(path: &BezPath) -> Vec<Contour> {
    let mut list = Vec::new();
    let mut current: Option<Contour> = None;
    let mut last = Point::ZERO;
    let push = |c: Option<Contour>, list: &mut Vec<Contour>| {
        if let Some(mut c) = c {
            c.total = c.segments.iter().map(|(_, l)| l).sum();
            list.push(c);
        }
    };
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                push(current.take(), &mut list);
                current = Some(Contour { segments: Vec::new(), closed: false, start: p, total: 0.0 });
                last = p;
            }
            PathEl::LineTo(p) => {
                let c = current.get_or_insert(Contour { segments: Vec::new(), closed: false, start: last, total: 0.0 });
                let seg = PathSeg::Line(kurbo::Line::new(last, p));
                c.segments.push((seg, seg.arclen(ACCURACY)));
                last = p;
            }
            PathEl::QuadTo(p1, p2) => {
                let c = current.get_or_insert(Contour { segments: Vec::new(), closed: false, start: last, total: 0.0 });
                let seg = PathSeg::Quad(kurbo::QuadBez::new(last, p1, p2));
                c.segments.push((seg, seg.arclen(ACCURACY)));
                last = p2;
            }
            PathEl::CurveTo(p1, p2, p3) => {
                let c = current.get_or_insert(Contour { segments: Vec::new(), closed: false, start: last, total: 0.0 });
                let seg = PathSeg::Cubic(kurbo::CubicBez::new(last, p1, p2, p3));
                c.segments.push((seg, seg.arclen(ACCURACY)));
                last = p3;
            }
            PathEl::ClosePath => {
                if let Some(mut c) = current.take() {
                    if (last - c.start).hypot() > 1e-9 {
                        let seg = PathSeg::Line(kurbo::Line::new(last, c.start));
                        c.segments.push((seg, seg.arclen(ACCURACY)));
                    }
                    c.closed = true;
                    last = c.start;
                    push(Some(c), &mut list);
                }
            }
        }
    }
    push(current.take(), &mut list);
    list
}

/// A path from pieces: each a list of connected segments (a new contour each), closed or not.
pub(crate) fn path_from_pieces(pieces: &[(Vec<PathSeg>, bool)]) -> BezPath {
    let mut path = BezPath::new();
    for (segments, closed) in pieces {
        let Some(first) = segments.first() else { continue };
        path.move_to(first.start());
        for seg in segments {
            match seg {
                PathSeg::Line(l) => path.line_to(l.p1),
                PathSeg::Quad(q) => path.quad_to(q.p1, q.p2),
                PathSeg::Cubic(c) => path.curve_to(c.p1, c.p2, c.p3),
            }
        }
        if *closed {
            path.close_path();
        }
    }
    path
}
