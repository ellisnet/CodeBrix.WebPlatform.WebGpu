//! `PathGeometry` and `PathGeometryBuilder`: a path of lines and curves with a fill rule (C#
//! `PathGeometry` / `PathGeometryBuilder`): building (with C#'s construction of rectangles,
//! rounded rectangles, ellipses, polygons and stars, so a path starts and runs where C#'s does),
//! SVG path data, bounds, MEASURING (length, the point and direction at a fraction or distance),
//! TRIMMING, DASHING (C#'s `Dash`) and transforming, and polygons with rounded corners (C#'s
//! `RoundedPolygon`). Held as a kurbo `BezPath`. Path booleans and queries (C#'s Union,
//! Contains, DistanceToEdge ...) are in path_geometry_combine.rs and path_geometry_queries.rs.

use kurbo::{Affine, BezPath, Shape};

use crate::geometry::{PointF, RectangleF};
use crate::paths::path_fill_rule::PathFillRule;
use crate::paths::path_measure::{contours, path_from_pieces};
use crate::paths::PathPoint;

/// C#'s `PathGeometryBuilder.Kappa`: the cubic handle length of a quarter circle of radius 1.
const KAPPA: f64 = 0.55228474983079340;

/// A path: contours of lines, quadratic and cubic curves, and its fill rule.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PathGeometry {
    path: BezPath,
    fill_rule: PathFillRule,
}

impl PathGeometry {
    /// The fill rule.
    pub fn fill_rule(&self) -> PathFillRule {
        self.fill_rule
    }

    /// The same path with another fill rule.
    pub fn with_fill_rule(&self, fill_rule: PathFillRule) -> PathGeometry {
        PathGeometry { path: self.path.clone(), fill_rule }
    }

    /// Parses SVG path data ("M 10 10 L 90 10 Z ..."; C# `PathGeometry.Parse`). The error says
    /// what was wrong.
    pub fn parse(svg_path_data: &str) -> Result<PathGeometry, String> {
        BezPath::from_svg(svg_path_data).map(|path| PathGeometry { path, fill_rule: PathFillRule::NonZero }).map_err(|e| format!("The SVG path data could not be read ({e}): check it against the SVG path syntax."))
    }

    /// A path from kurbo's path type (kurbo is this crate's curve library).
    pub fn from_bez_path(path: BezPath, fill_rule: PathFillRule) -> PathGeometry {
        PathGeometry { path, fill_rule }
    }

    /// The path as kurbo's path type.
    pub fn bez_path(&self) -> &BezPath {
        &self.path
    }

    /// True when the path has no segments.
    pub fn is_empty(&self) -> bool {
        self.path.elements().len() < 2
    }

    /// The smallest axis-aligned rectangle around every contour, curves included exactly (their
    /// extreme points, not their control points) (C# `GetBounds`).
    pub fn get_bounds(&self) -> RectangleF {
        let r = self.path.bounding_box();
        RectangleF::new(r.x0 as f32, r.y0 as f32, r.width() as f32, r.height() as f32)
    }

    /// An empty path (C# `PathGeometry.Empty`).
    pub fn empty() -> PathGeometry {
        PathGeometry::default()
    }

    /// The number of contours (C# `Contours.Count`).
    pub fn contour_count(&self) -> usize {
        contours(&self.path).len()
    }

    /// The total length of every contour, a closed contour's closing line included (C#
    /// `GetLength`).
    pub fn get_length(&self) -> f32 {
        contours(&self.path).iter().map(|c| c.total).sum::<f64>() as f32
    }

    /// The point at a FRACTION of the whole length, clamped to 0..1 (NaN counts as 0) (C#
    /// `GetPointAt(fraction)`): position, unit direction, distance and contour. The contours count
    /// one after the other.
    pub fn get_point_at(&self, fraction: f32) -> PathPoint {
        let f = if fraction.is_nan() { 0.0 } else { (fraction as f64).clamp(0.0, 1.0) };
        let total: f64 = contours(&self.path).iter().map(|c| c.total).sum();
        self.point_at_distance(f * total)
    }

    /// The point at a distance along the path, clamped to the path (C# `GetPointAtDistance`).
    pub fn get_point_at_distance(&self, distance: f32) -> PathPoint {
        self.point_at_distance(if distance.is_nan() { 0.0 } else { distance as f64 })
    }

    fn point_at_distance(&self, distance: f64) -> PathPoint {
        let list = contours(&self.path);
        if list.is_empty() {
            return PathPoint { position: PointF::new(0.0, 0.0), tangent: PointF::new(1.0, 0.0), distance: 0.0, contour_index: -1 };
        }
        let total: f64 = list.iter().map(|c| c.total).sum();
        let distance = distance.clamp(0.0, total);
        let mut start = 0.0;
        let mut index = list.len() - 1;
        for (i, c) in list.iter().enumerate() {
            if distance < start + c.total || i == list.len() - 1 {
                index = i;
                break;
            }
            start += c.total;
        }
        let (p, t) = list[index].point_at(distance - start);
        PathPoint { position: PointF::new(p.x as f32, p.y as f32), tangent: PointF::new(t.x as f32, t.y as f32), distance: distance as f32, contour_index: index as i32 }
    }

    /// The part of the path between two fractions of its length, the window moved on by `offset`
    /// (in lengths, wrapping round) (C# `Trim(start, end, offset = 0)`): the fractions are clamped
    /// to 0..1 and swapped when reversed; 0..1 is the whole path; a part running past the end goes
    /// on from the start (one piece round a single closed contour, two pieces otherwise). The
    /// error says what to do with a value that is not finite.
    pub fn trim(&self, start: f32, end: f32, offset: f32) -> Result<PathGeometry, String> {
        if !start.is_finite() || !end.is_finite() || !offset.is_finite() {
            return Err("Trim fractions and offset must be finite numbers (fractions of the length).".to_string());
        }
        let (mut s, mut e) = ((start as f64).clamp(0.0, 1.0), (end as f64).clamp(0.0, 1.0));
        if s > e {
            std::mem::swap(&mut s, &mut e);
        }
        if s == 0.0 && e == 1.0 {
            return Ok(self.clone());
        }
        let list = contours(&self.path);
        let total: f64 = list.iter().map(|c| c.total).sum();
        if e - s <= 0.0 || total <= 0.0 {
            return Ok(PathGeometry { path: BezPath::new(), fill_rule: self.fill_rule });
        }
        let shifted = s + offset as f64;
        let from = shifted - shifted.floor();
        let to = from + (e - s);
        let mut pieces: Vec<(Vec<kurbo::PathSeg>, bool)> = Vec::new();
        let part = |a: f64, b: f64, pieces: &mut Vec<(Vec<kurbo::PathSeg>, bool)>| {
            let mut at = 0.0;
            for c in &list {
                let (cs, ce) = (at, at + c.total);
                at = ce;
                let (lo, hi) = (a.max(cs), b.min(ce));
                if hi > lo {
                    let mut segs = Vec::new();
                    c.extract(lo - cs, hi - cs, &mut segs);
                    if !segs.is_empty() {
                        pieces.push((segs, false));
                    }
                }
            }
        };
        if to <= 1.0 {
            part(from * total, to * total, &mut pieces);
        } else if list.len() == 1 && list[0].closed {
            // One closed contour: the wrapped part runs on across the start as one piece.
            let mut segs = Vec::new();
            list[0].extract(from * total, total, &mut segs);
            list[0].extract(0.0, (to - 1.0) * total, &mut segs);
            pieces.push((segs, false));
        } else {
            part(from * total, total, &mut pieces);
            part(0.0, (to - 1.0) * total, &mut pieces);
        }
        Ok(PathGeometry { path: path_from_pieces(&pieces), fill_rule: self.fill_rule })
    }

    /// Splits every contour into dashes (C# `Dash(pattern, offset = 0)`): `pattern` alternates
    /// dash and gap lengths (calibrated units, starting with a dash), repeated along each contour
    /// from its start; `offset` starts each contour that far into the pattern (SVG's
    /// `stroke-dashoffset`). A pattern with an odd number of entries is used twice over, as SVG
    /// does. Every dash is an open contour; a dash of zero length is kept as a single point (a
    /// zero-length line), so a round or square cap draws it as a dot. The fill rule is kept. The
    /// error says what to do with a pattern that is empty, has a negative or non-finite length,
    /// or sums to 0, or an offset that is not finite.
    pub fn dash(&self, pattern: &[f32], offset: f32) -> Result<PathGeometry, String> {
        let intervals = normalize_dash_pattern(pattern)?;
        if !offset.is_finite() {
            return Err("The dash offset must be a finite distance (calibrated units).".to_string());
        }
        let period: f64 = intervals.iter().sum();
        let mut pieces: Vec<(Vec<kurbo::PathSeg>, bool)> = Vec::new();
        for contour in contours(&self.path) {
            if contour.segments.is_empty() {
                continue;
            }
            let length = contour.total;
            let offset = offset as f64;
            let mut phase = offset - ((offset / period).floor() * period);
            let mut index = 0;
            // An interval the phase reaches exactly is passed, unless it has no length (a dot).
            while phase > intervals[index] || (phase == intervals[index] && intervals[index] > 0.0) {
                phase -= intervals[index];
                index = (index + 1) % intervals.len();
            }
            let mut position = 0.0;
            let mut remaining = intervals[index] - phase;
            while position <= length {
                let next = position + remaining;
                if index & 1 == 0 {
                    let stop = next.min(length);
                    let mut segs = Vec::new();
                    if stop == position {
                        let (p, _) = contour.point_at(position);
                        segs.push(kurbo::PathSeg::Line(kurbo::Line::new(p, p)));
                    } else {
                        contour.extract(position, stop, &mut segs);
                    }
                    if !segs.is_empty() {
                        pieces.push((segs, false));
                    }
                }
                if next >= length {
                    break;
                }
                position = next;
                index = (index + 1) % intervals.len();
                remaining = intervals[index];
            }
        }
        Ok(PathGeometry { path: path_from_pieces(&pieces), fill_rule: self.fill_rule })
    }

    /// The path through a matrix (C# `Transformed(Matrix3x2)`; kurbo's `Affine`, for example a
    /// [`DrawingTransform`](crate::models::DrawingTransform)'s `to_matrix()`).
    pub fn transformed(&self, matrix: Affine) -> PathGeometry {
        PathGeometry { path: matrix * &self.path, fill_rule: self.fill_rule }
    }

    /// A rectangle, clockwise from the top-left corner (C# `PathGeometry.Rectangle`).
    pub fn rectangle(rectangle: RectangleF) -> PathGeometry {
        PathGeometryBuilder::new().add_rectangle(rectangle).build()
    }

    /// A rectangle with circular corners, clockwise from the end of the top-left corner (C#
    /// `PathGeometry.RoundedRectangle`).
    pub fn rounded_rectangle(rectangle: RectangleF, corner_radius: f32) -> PathGeometry {
        PathGeometryBuilder::new().add_rounded_rectangle(rectangle, corner_radius).build()
    }

    /// An ellipse of four cubic curves, clockwise from its rightmost point (C#
    /// `PathGeometry.Ellipse`).
    pub fn ellipse(center: PointF, radius_x: f32, radius_y: f32) -> PathGeometry {
        PathGeometryBuilder::new().add_ellipse(center, radius_x, radius_y).build()
    }

    /// A circle (C# `PathGeometry.Circle`).
    pub fn circle(center: PointF, radius: f32) -> PathGeometry {
        PathGeometry::ellipse(center, radius, radius)
    }

    /// A regular polygon, its first corner straight up when `rotation` is 0 (degrees) (C#
    /// `PathGeometry.RegularPolygon`). The error says what to do with fewer than 3 sides.
    pub fn regular_polygon(center: PointF, radius: f32, sides: usize, rotation: f32) -> Result<PathGeometry, String> {
        if sides < 3 {
            return Err("A regular polygon needs at least 3 sides.".to_string());
        }
        Ok(PathGeometryBuilder::new().add_polygon(&corner_points(center, radius, radius, sides, rotation), true).build())
    }

    /// A closed polygon whose corners are rounded with circular arcs of `corner_radius` (C#
    /// `PathGeometry.RoundedPolygon`) - tangent to both edges at each corner, the radius made
    /// smaller where an edge is too short to hold two such roundings (each rounding then takes
    /// at most half of its edges); each arc is one cubic curve. Corners of a straight run (and
    /// repeated points) are kept sharp; a radius of 0 gives the plain polygon. The error says
    /// what to do with fewer than three points or a radius that is negative or not finite.
    pub fn rounded_polygon(points: &[PointF], corner_radius: f32) -> Result<PathGeometry, String> {
        if points.len() < 3 {
            return Err("A polygon needs at least three points.".to_string());
        }
        if !corner_radius.is_finite() || corner_radius < 0.0 {
            return Err("The corner radius must be a finite distance of 0 or more (calibrated units).".to_string());
        }
        if corner_radius == 0.0 {
            return Ok(PathGeometryBuilder::new().add_polygon(points, true).build());
        }
        let n = points.len();
        let p = |x: f64, y: f64| PointF::new(x as f32, y as f32);
        let mut entries = points.to_vec();
        let mut exits = points.to_vec();
        let mut controls_in = vec![PointF::new(0.0, 0.0); n];
        let mut controls_out = vec![PointF::new(0.0, 0.0); n];
        let mut rounded = vec![false; n];
        for i in 0..n {
            let (prev, at, next) = (points[(i + n - 1) % n], points[i], points[(i + 1) % n]);
            let (ax, ay) = (prev.x as f64 - at.x as f64, prev.y as f64 - at.y as f64);
            let (bx, by) = (next.x as f64 - at.x as f64, next.y as f64 - at.y as f64);
            let (la, lb) = (ax.hypot(ay), bx.hypot(by));
            if la <= 0.0 || lb <= 0.0 {
                continue;
            }
            let cos = ((ax * bx + ay * by) / (la * lb)).clamp(-1.0, 1.0);
            let interior = cos.acos();
            if interior <= 1e-6 || std::f64::consts::PI - interior <= 1e-6 {
                continue;
            }
            let half = interior / 2.0;
            let tangent = (corner_radius as f64 / half.tan()).min(la.min(lb) / 2.0);
            let radius = tangent * half.tan();
            let turn = std::f64::consts::PI - interior;
            let handle = 4.0 / 3.0 * (turn / 4.0).tan() * radius;
            let (ux, uy, vx, vy) = (ax / la, ay / la, bx / lb, by / lb);
            entries[i] = p(at.x as f64 + ux * tangent, at.y as f64 + uy * tangent);
            exits[i] = p(at.x as f64 + vx * tangent, at.y as f64 + vy * tangent);
            controls_in[i] = p(entries[i].x as f64 - ux * handle, entries[i].y as f64 - uy * handle);
            controls_out[i] = p(exits[i].x as f64 - vx * handle, exits[i].y as f64 - vy * handle);
            rounded[i] = true;
        }
        let mut builder = PathGeometryBuilder::new();
        builder.move_to(exits[0].x, exits[0].y);
        for k in 1..=n {
            let i = k % n;
            builder.line_to(entries[i].x, entries[i].y);
            if rounded[i] {
                builder.cubic_to(controls_in[i], controls_out[i], exits[i]);
            }
        }
        builder.close();
        Ok(builder.build())
    }

    /// A star of `points` tips, alternating outer and inner corners, the first tip straight up
    /// when `rotation` is 0 (degrees) (C# `PathGeometry.Star`). The error says what to do with
    /// fewer than 2 points.
    pub fn star(center: PointF, outer_radius: f32, inner_radius: f32, points: usize, rotation: f32) -> Result<PathGeometry, String> {
        if points < 2 {
            return Err("A star needs at least 2 points.".to_string());
        }
        Ok(PathGeometryBuilder::new().add_polygon(&corner_points(center, outer_radius, inner_radius, points * 2, rotation), true).build())
    }
}

/// C#'s `PathStrokeStyle.NormalizeDashPattern`: the lengths checked (at least one, none negative
/// or non-finite, a positive sum) and an odd-length pattern used twice over.
fn normalize_dash_pattern(pattern: &[f32]) -> Result<Vec<f64>, String> {
    if pattern.is_empty() {
        return Err("A dash pattern needs at least one length.".to_string());
    }
    if pattern.iter().any(|v| !v.is_finite() || *v < 0.0) {
        return Err("Dash lengths must be finite and not negative.".to_string());
    }
    if pattern.iter().map(|v| *v as f64).sum::<f64>() <= 0.0 {
        return Err("A dash pattern must have some length: give at least one positive length.".to_string());
    }
    let count = if pattern.len() % 2 == 0 { pattern.len() } else { pattern.len() * 2 };
    Ok((0..count).map(|i| pattern[i % pattern.len()] as f64).collect())
}

/// C#'s CornerPoints: `count` corners from straight up (plus `rotation` degrees), clockwise,
/// even ones at `outer`, odd ones at `inner`.
fn corner_points(center: PointF, outer: f32, inner: f32, count: usize, rotation: f32) -> Vec<PointF> {
    let first = (rotation as f64 - 90.0) * (std::f64::consts::PI / 180.0);
    (0..count)
        .map(|i| {
            let angle = first + (2.0 * std::f64::consts::PI * i as f64 / count as f64);
            let radius = if i & 1 == 0 { outer as f64 } else { inner as f64 };
            PointF::new((center.x as f64 + radius * angle.cos()) as f32, (center.y as f64 + radius * angle.sin()) as f32)
        })
        .collect()
}

/// Builds a [`PathGeometry`] one segment at a time (C# `PathGeometryBuilder`).
#[derive(Clone, Debug, Default)]
pub struct PathGeometryBuilder {
    path: BezPath,
    /// The fill rule the built geometry carries (non-zero by default).
    pub fill_rule: PathFillRule,
}

impl PathGeometryBuilder {
    /// An empty builder.
    pub fn new() -> PathGeometryBuilder {
        PathGeometryBuilder::default()
    }

    /// Starts a contour.
    pub fn move_to(&mut self, x: f32, y: f32) -> &mut PathGeometryBuilder {
        self.path.move_to((x as f64, y as f64));
        self
    }

    /// A line to a point.
    pub fn line_to(&mut self, x: f32, y: f32) -> &mut PathGeometryBuilder {
        self.path.line_to((x as f64, y as f64));
        self
    }

    /// A quadratic curve.
    pub fn quadratic_to(&mut self, control: PointF, end: PointF) -> &mut PathGeometryBuilder {
        self.path.quad_to(control.to_kurbo(), end.to_kurbo());
        self
    }

    /// A cubic curve.
    pub fn cubic_to(&mut self, control1: PointF, control2: PointF, end: PointF) -> &mut PathGeometryBuilder {
        self.path.curve_to(control1.to_kurbo(), control2.to_kurbo(), end.to_kurbo());
        self
    }

    /// Closes the contour.
    pub fn close(&mut self) -> &mut PathGeometryBuilder {
        self.path.close_path();
        self
    }

    /// A polygon (or an open polyline).
    pub fn add_polygon(&mut self, points: &[PointF], closed: bool) -> &mut PathGeometryBuilder {
        for (i, p) in points.iter().enumerate() {
            if i == 0 {
                self.path.move_to(p.to_kurbo());
            } else {
                self.path.line_to(p.to_kurbo());
            }
        }
        if closed && !points.is_empty() {
            self.path.close_path();
        }
        self
    }

    /// A rectangle.
    pub fn add_rectangle(&mut self, rectangle: RectangleF) -> &mut PathGeometryBuilder {
        let (l, t, r, b) = (rectangle.x as f64, rectangle.y as f64, rectangle.right() as f64, rectangle.bottom() as f64);
        self.path.move_to((l, t));
        self.path.line_to((r, t));
        self.path.line_to((r, b));
        self.path.line_to((l, b));
        self.path.close_path();
        self
    }

    /// A rectangle with rounded corners.
    pub fn add_rounded_rectangle(&mut self, rectangle: RectangleF, corner_radius: f32) -> &mut PathGeometryBuilder {
        let r = (corner_radius.max(0.0) as f64).min((rectangle.width.abs().min(rectangle.height.abs()) as f64) / 2.0);
        if r <= 0.0 {
            return self.add_rectangle(rectangle);
        }
        let (l, t, rt, b) = (rectangle.x as f64, rectangle.y as f64, rectangle.right() as f64, rectangle.bottom() as f64);
        let k = r * KAPPA;
        let p = &mut self.path;
        p.move_to((l + r, t));
        p.line_to((rt - r, t));
        p.curve_to((rt - r + k, t), (rt, t + r - k), (rt, t + r));
        p.line_to((rt, b - r));
        p.curve_to((rt, b - r + k), (rt - r + k, b), (rt - r, b));
        p.line_to((l + r, b));
        p.curve_to((l + r - k, b), (l, b - r + k), (l, b - r));
        p.line_to((l, t + r));
        p.curve_to((l, t + r - k), (l + r - k, t), (l + r, t));
        p.close_path();
        self
    }

    /// An ellipse.
    pub fn add_ellipse(&mut self, center: PointF, radius_x: f32, radius_y: f32) -> &mut PathGeometryBuilder {
        let (cx, cy, rx, ry) = (center.x as f64, center.y as f64, radius_x.max(0.0) as f64, radius_y.max(0.0) as f64);
        let (kx, ky) = (rx * KAPPA, ry * KAPPA);
        let p = &mut self.path;
        p.move_to((cx + rx, cy));
        p.curve_to((cx + rx, cy + ky), (cx + kx, cy + ry), (cx, cy + ry));
        p.curve_to((cx - kx, cy + ry), (cx - rx, cy + ky), (cx - rx, cy));
        p.curve_to((cx - rx, cy - ky), (cx - kx, cy - ry), (cx, cy - ry));
        p.curve_to((cx + kx, cy - ry), (cx + rx, cy - ky), (cx + rx, cy));
        p.close_path();
        self
    }

    /// A circle.
    pub fn add_circle(&mut self, center: PointF, radius: f32) -> &mut PathGeometryBuilder {
        self.add_ellipse(center, radius, radius)
    }

    /// Another geometry's contours.
    pub fn add_path(&mut self, geometry: &PathGeometry) -> &mut PathGeometryBuilder {
        self.path.extend(geometry.path.elements().iter().copied());
        self
    }

    /// The geometry built so far.
    pub fn build(&self) -> PathGeometry {
        PathGeometry { path: self.path.clone(), fill_rule: self.fill_rule }
    }

    /// Starts again, empty.
    pub fn clear(&mut self) -> &mut PathGeometryBuilder {
        self.path = BezPath::new();
        self
    }
}
