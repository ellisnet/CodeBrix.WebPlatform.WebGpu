//! The polygon side of the path tools (internal; C#'s `PathClipper`): boolean combination,
//! simplification, stroke outlining and inflating. Curves are flattened with NoSkia's own
//! flattening in double precision, every point is rounded ONCE to a grid of 1/1024 unit (C#'s
//! `GridScale`), and the polygons go through i_overlay's INTEGER overlay (i64), so the grid is the
//! engine's input exactly. Results are closed polygons of lines whose corners are grid points
//! (exact as f32), with the non-zero rule.
//!
//! DIFFERENCE FROM NoSkia: NoSkia's engine is CodeBrix.PolygonTools; this one is i_overlay. The
//! polygons cover the same area, but their start vertices, directions and order are i_overlay's,
//! so a trim (draw-on) or a morph of a combined path starts elsewhere than in NoSkia. The STROKER
//! below is NoSkia's own (pieces sharing exact grid edges, united by the overlay).

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay::{Overlay, ShapeType};
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::i_float::int::point::IntPoint;
use kurbo::BezPath;

use crate::paths::path_area_query::{flatten_f64, FlatPoint};
use crate::paths::{PathBooleanOperation, PathFillRule, PathGeometry, PathStrokeCap, PathStrokeJoin, PathStrokeStyle};

/// Grid steps per calibrated unit; a power of two, so a grid point divides back to an exact float.
pub(crate) const GRID_SCALE: f64 = 1024.0;

/// The largest coordinate magnitude accepted, calibrated units.
pub(crate) const MAX_COORDINATE: f64 = 1_000_000.0;

const MAX_GRID: i64 = (MAX_COORDINATE * GRID_SCALE) as i64;

type Point = IntPoint<i64>;
type Polygons = Vec<Vec<Point>>;

/// A point on the grid (C# `ToGrid`: rounded half away from zero); the error says what to do.
pub(crate) fn to_grid(x: f64, y: f64) -> Result<Point, String> {
    let gx = (x * GRID_SCALE).round();
    let gy = (y * GRID_SCALE).round();
    if x.is_nan() || y.is_nan() || gx.abs() > MAX_GRID as f64 || gy.abs() > MAX_GRID as f64 {
        return Err(format!("A path coordinate ({x}, {y}) lies beyond the {MAX_COORDINATE} units the polygon tools accept: keep paths within a million units of the origin."));
    }
    Ok(Point::new(gx as i64, gy as i64))
}

fn add_distinct(polygon: &mut Vec<Point>, point: Point) {
    if polygon.last() != Some(&point) {
        polygon.push(point);
    }
}

fn fill_of(rule: PathFillRule) -> FillRule {
    if rule == PathFillRule::EvenOdd {
        FillRule::EvenOdd
    } else {
        FillRule::NonZero
    }
}

fn rule_of(operation: PathBooleanOperation) -> OverlayRule {
    match operation {
        PathBooleanOperation::Union => OverlayRule::Union,
        PathBooleanOperation::Intersection => OverlayRule::Intersect,
        PathBooleanOperation::Difference => OverlayRule::Difference,
        PathBooleanOperation::ExclusiveOr => OverlayRule::Xor,
    }
}

/// One run of the overlay: the contours of every resulting shape (outer and holes alike).
fn overlay(subject: &Polygons, clip: &Polygons, rule: OverlayRule, fill: FillRule) -> Polygons {
    let capacity = subject.iter().chain(clip.iter()).map(|p| p.len()).sum();
    let mut engine = Overlay::<i64>::new(capacity);
    engine.add_source(subject, ShapeType::Subject);
    engine.add_source(clip, ShapeType::Clip);
    engine.overlay(rule, fill).into_iter().flatten().filter(|c| c.len() >= 3).collect()
}

/// Every contour as a closed polygon on the grid (an open contour fills as if closed) (C#
/// `ToPolygons`).
fn to_polygons(geometry: &PathGeometry, tolerance: f64) -> Result<Polygons, String> {
    let mut polygons = Vec::new();
    for (points, _) in flatten_f64(geometry, tolerance) {
        let mut polygon = Vec::with_capacity(points.len());
        for p in &points {
            add_distinct(&mut polygon, to_grid(p.x, p.y)?);
        }
        if polygon.len() > 1 && polygon.last() == polygon.first() {
            polygon.pop();
        }
        if polygon.len() >= 3 {
            polygons.push(polygon);
        }
    }
    Ok(polygons)
}

/// The area a geometry fills, as non-overlapping polygons (C# `Region`).
pub(crate) fn region(geometry: &PathGeometry, tolerance: f64) -> Result<Polygons, String> {
    Ok(overlay(&to_polygons(geometry, tolerance)?, &Vec::new(), OverlayRule::Subject, fill_of(geometry.fill_rule())))
}

/// Grid polygons back to a geometry: one closed contour of lines per polygon, non-zero rule (C#
/// `ToGeometry`).
fn to_geometry(polygons: &Polygons) -> PathGeometry {
    let mut path = BezPath::new();
    for polygon in polygons.iter().filter(|p| p.len() >= 3) {
        for (i, p) in polygon.iter().enumerate() {
            let point = (((p.x as f64 / GRID_SCALE) as f32) as f64, ((p.y as f64 / GRID_SCALE) as f32) as f64);
            if i == 0 {
                path.move_to(point);
            } else {
                path.line_to(point);
            }
        }
        path.close_path();
    }
    if path.elements().is_empty() {
        PathGeometry::empty()
    } else {
        PathGeometry::from_bez_path(path, PathFillRule::NonZero)
    }
}

/// Two geometries combined (C# `Combine`). C# hands the engine each geometry's own fill rule;
/// i_overlay takes one rule for both, so when the rules differ each is first reduced to its
/// region (non-overlapping polygons) and those are combined by the non-zero rule - the same
/// area.
pub(crate) fn combine(subject: &PathGeometry, clip: &PathGeometry, operation: PathBooleanOperation, tolerance: f64) -> Result<PathGeometry, String> {
    let solution = if subject.fill_rule() == clip.fill_rule() {
        overlay(&to_polygons(subject, tolerance)?, &to_polygons(clip, tolerance)?, rule_of(operation), fill_of(subject.fill_rule()))
    } else {
        overlay(&region(subject, tolerance)?, &region(clip, tolerance)?, rule_of(operation), FillRule::NonZero)
    };
    Ok(to_geometry(&solution))
}

/// The fill as clean, non-overlapping polygons (C# `Simplify`).
pub(crate) fn simplify(geometry: &PathGeometry, tolerance: f64) -> Result<PathGeometry, String> {
    Ok(to_geometry(&region(geometry, tolerance)?))
}

/// Many geometries combined (C# `CombineAll`): union and difference in one run over the regions,
/// intersection and exclusive-or folded from the left.
pub(crate) fn combine_all(operation: PathBooleanOperation, geometries: &[PathGeometry], tolerance: f64) -> Result<PathGeometry, String> {
    let Some(first) = geometries.first() else { return Ok(PathGeometry::empty()) };
    if matches!(operation, PathBooleanOperation::Union | PathBooleanOperation::Difference) {
        let start = if operation == PathBooleanOperation::Union { 0 } else { 1 };
        let mut others = Vec::new();
        for g in &geometries[start..] {
            others.extend(region(g, tolerance)?);
        }
        let solution = if operation == PathBooleanOperation::Union {
            overlay(&others, &Vec::new(), OverlayRule::Subject, FillRule::NonZero)
        } else {
            overlay(&region(first, tolerance)?, &others, OverlayRule::Difference, FillRule::NonZero)
        };
        return Ok(to_geometry(&solution));
    }
    let mut result = region(first, tolerance)?;
    for g in &geometries[1..] {
        // C# runs the result by non-zero and the next one by its own rule: an even-odd one is
        // reduced to its region first (one rule for both in i_overlay).
        let clip = if g.fill_rule() == PathFillRule::EvenOdd { region(g, tolerance)? } else { to_polygons(g, tolerance)? };
        result = overlay(&result, &clip, rule_of(operation), FillRule::NonZero);
    }
    Ok(to_geometry(&result))
}

/// The area a stroke of this style covers (dashes cut first), as polygons (C# `StrokeOutline`).
pub(crate) fn stroke_outline(geometry: &PathGeometry, style: &PathStrokeStyle, tolerance: f64) -> Result<PathGeometry, String> {
    let dashed;
    let source = if style.is_dashed() {
        dashed = dash(geometry, &style.get_dash_pattern(), style.dash_offset());
        &dashed
    } else {
        geometry
    };
    let mut stroker = Stroker::new(style.width() as f64 / 2.0, style.cap(), style.join(), style.miter_limit() as f64, tolerance);
    for (points, closed) in flatten_f64(source, tolerance) {
        stroker.add_contour(&points, closed)?;
    }
    Ok(to_geometry(&overlay(&stroker.pieces, &Vec::new(), OverlayRule::Subject, FillRule::NonZero)))
}

/// The pieces of a stroke's band without their union (filled non-zero they cover the same area as
/// `stroke_outline`) - for the hard-edged pixel rule, which needs no union.
pub(crate) fn stroke_band_pieces(geometry: &PathGeometry, style: &PathStrokeStyle, tolerance: f64) -> Result<PathGeometry, String> {
    let dashed;
    let source = if style.is_dashed() {
        dashed = dash(geometry, &style.get_dash_pattern(), style.dash_offset());
        &dashed
    } else {
        geometry
    };
    let mut stroker = Stroker::new(style.width() as f64 / 2.0, style.cap(), style.join(), style.miter_limit() as f64, tolerance);
    for (points, closed) in flatten_f64(source, tolerance) {
        stroker.add_contour(&points, closed)?;
    }
    Ok(to_geometry(&stroker.pieces))
}

/// The geometry cut into its dashes (open contours), kurbo's dash iterator over the path.
fn dash(geometry: &PathGeometry, pattern: &[f32], offset: f32) -> PathGeometry {
    let pattern: Vec<f64> = pattern.iter().map(|d| *d as f64).collect();
    if pattern.iter().all(|d| *d <= 0.0) {
        return geometry.clone();
    }
    let path: BezPath = kurbo::dash(geometry.bez_path().elements().iter().copied(), offset as f64, &pattern).collect();
    PathGeometry::from_bez_path(path, geometry.fill_rule())
}

/// The fill grown (distance > 0) or shrunk (distance < 0), corners per the join (C# `Inflate`):
/// the region's boundary stroked to twice the distance, added to or taken from the region.
pub(crate) fn inflate(geometry: &PathGeometry, distance: f64, join: PathStrokeJoin, miter_limit: f64, tolerance: f64) -> Result<PathGeometry, String> {
    let region = region(geometry, tolerance)?;
    if distance == 0.0 || region.is_empty() {
        return Ok(to_geometry(&region));
    }
    let mut stroker = Stroker::new(distance.abs(), PathStrokeCap::Butt, join, miter_limit, tolerance);
    for polygon in &region {
        let points: Vec<FlatPoint> = polygon.iter().map(|p| FlatPoint { x: p.x as f64 / GRID_SCALE, y: p.y as f64 / GRID_SCALE, smooth: false }).collect();
        stroker.add_contour(&points, true)?;
    }
    let rule = if distance > 0.0 { OverlayRule::Union } else { OverlayRule::Difference };
    Ok(to_geometry(&overlay(&region, &stroker.pieces, rule, FillRule::NonZero)))
}

/// Twice a polygon's signed area (the shoelace sum), in grid units squared.
fn area2(polygon: &[Point]) -> i128 {
    let n = polygon.len();
    (0..n)
        .map(|i| {
            let (a, b) = (polygon[i], polygon[(i + 1) % n]);
            a.x as i128 * b.y as i128 - b.x as i128 * a.y as i128
        })
        .sum()
}

/// NoSkia's stroker (C# `PathClipper.Stroker`): a stroke as overlapping simple pieces - a
/// hexagon per segment whose end edges run through the grid point of the contour point itself,
/// a wedge per corner on its outer side (bevel triangle, miter point within the limit, or round
/// fan), caps at the ends of open contours, dots for zero-length ones - all wound the same way,
/// united by the non-zero rule. Pieces that meet share EXACT edges.
struct Stroker {
    half: f64,
    cap: PathStrokeCap,
    join: PathStrokeJoin,
    miter_limit: f64,
    arc_step: f64,
    pieces: Polygons,
}

impl Stroker {
    fn new(half: f64, cap: PathStrokeCap, join: PathStrokeJoin, miter_limit: f64, tolerance: f64) -> Stroker {
        // Chords of an arc of radius r stay within the tolerance when each spans 2 acos(1 - tol / r).
        let steps = if tolerance < half { (2.0 * std::f64::consts::PI / (2.0 * (1.0 - (tolerance / half)).acos())).ceil() as i64 } else { 8 };
        let arc_step = 2.0 * std::f64::consts::PI / steps.clamp(8, 4096) as f64;
        Stroker { half, cap, join, miter_limit, arc_step, pieces: Vec::new() }
    }

    fn add_contour(&mut self, source: &[FlatPoint], closed: bool) -> Result<(), String> {
        // Drop repeated points (a corner wins over a smooth point at the same place).
        let mut points: Vec<FlatPoint> = Vec::with_capacity(source.len());
        for p in source {
            if let Some(last) = points.last_mut() {
                if last.x == p.x && last.y == p.y {
                    if !p.smooth {
                        *last = *p;
                    }
                    continue;
                }
            }
            points.push(*p);
        }
        if closed && points.len() > 1 && points[points.len() - 1].x == points[0].x && points[points.len() - 1].y == points[0].y {
            points.pop();
        }
        if points.is_empty() {
            return Ok(());
        }
        if points.len() == 1 {
            if !closed {
                self.add_dot(points[0])?;
            }
            return Ok(());
        }
        let closed = closed && points.len() != 2;
        let n = points.len();
        let segment_count = if closed { n } else { n - 1 };
        let (mut ux, mut uy) = (vec![0.0; segment_count], vec![0.0; segment_count]);
        let mut centres = Vec::with_capacity(n);
        for p in &points {
            centres.push(to_grid(p.x, p.y)?);
        }
        let (mut left_start, mut right_start, mut left_end, mut right_end) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for i in 0..segment_count {
            let (a, b) = (points[i], points[(i + 1) % n]);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let length = ((dx * dx) + (dy * dy)).sqrt();
            ux[i] = dx / length;
            uy[i] = dy / length;
            let (nx, ny) = (-uy[i] * self.half, ux[i] * self.half);
            left_start.push(to_grid(a.x + nx, a.y + ny)?);
            right_start.push(to_grid(a.x - nx, a.y - ny)?);
            left_end.push(to_grid(b.x + nx, b.y + ny)?);
            right_end.push(to_grid(b.x - nx, b.y - ny)?);
            self.add_piece(vec![left_start[i], left_end[i], centres[(i + 1) % n], right_end[i], right_start[i], centres[i]]);
        }
        let (first_join, last_join) = if closed { (0, n as i64 - 1) } else { (1, n as i64 - 2) };
        let mut v = first_join;
        while v <= last_join {
            let vi = v as usize;
            let in_segment = (vi + segment_count - 1) % segment_count;
            let out_segment = vi % segment_count;
            self.add_join(points[vi], centres[vi], (ux[in_segment], uy[in_segment]), (ux[out_segment], uy[out_segment]), [left_end[in_segment], right_end[in_segment], left_start[out_segment], right_start[out_segment]])?;
            v += 1;
        }
        if !closed {
            // The start cap faces back along the first segment, the end cap on along the last.
            self.add_cap(points[0], centres[0], -ux[0], -uy[0], right_start[0], left_start[0])?;
            let last = segment_count - 1;
            self.add_cap(points[n - 1], centres[n - 1], ux[last], uy[last], left_end[last], right_end[last])?;
        }
        Ok(())
    }

    fn add_join(&mut self, p: FlatPoint, centre: Point, (ax, ay): (f64, f64), (bx, by): (f64, f64), [in_left, in_right, out_left, out_right]: [Point; 4]) -> Result<(), String> {
        let cross = (ax * by) - (ay * bx);
        let dot = (ax * bx) + (ay * by);
        if cross == 0.0 && dot > 0.0 {
            return Ok(());
        }
        // Turning towards the left normal puts the corner's outside on the right.
        let outside_right = cross > 0.0;
        let from = if outside_right { in_right } else { in_left };
        let to = if outside_right { out_right } else { out_left };
        let mut join = if p.smooth { PathStrokeJoin::Round } else { self.join };
        if join == PathStrokeJoin::Miter {
            // Miter length / width = 1 / cos(turn / 2); cos(turn / 2)^2 = (1 + cos(turn)) / 2.
            let half_cos2 = (1.0 + dot) / 2.0;
            if half_cos2 > 0.0 && 1.0 / half_cos2.sqrt() <= self.miter_limit {
                let sign = if outside_right { -1.0 } else { 1.0 };
                let (mx, my) = (sign * (-ay - by), sign * (ax + bx));
                let k = self.half / (1.0 + dot);
                let tip = to_grid(p.x + (mx * k), p.y + (my * k))?;
                self.add_piece(vec![centre, from, tip, to]);
                return Ok(());
            }
            join = PathStrokeJoin::Bevel;
        }
        if join == PathStrokeJoin::Bevel {
            self.add_piece(vec![centre, from, to]);
            return Ok(());
        }
        // Round: a fan from the incoming outer point to the outgoing one, round the corner.
        let turn = cross.atan2(dot);
        let steps = ((turn.abs() / self.arc_step).ceil() as i64).max(1);
        let mut fan = vec![centre, from];
        let (sx, sy) = if outside_right { (ay, -ax) } else { (-ay, ax) };
        for j in 1..steps {
            let angle = turn * j as f64 / steps as f64;
            let (sin, cos) = angle.sin_cos();
            fan.push(to_grid(p.x + (((sx * cos) - (sy * sin)) * self.half), p.y + (((sx * sin) + (sy * cos)) * self.half))?);
        }
        fan.push(to);
        self.add_piece(fan);
        Ok(())
    }

    /// A cap at an end point facing (dx, dy); `left` and `right` are the stroke's edge points
    /// there, seen looking along the facing direction.
    fn add_cap(&mut self, p: FlatPoint, centre: Point, dx: f64, dy: f64, left: Point, right: Point) -> Result<(), String> {
        match self.cap {
            PathStrokeCap::Butt => Ok(()),
            PathStrokeCap::Square => {
                let (ex, ey) = (dx * self.half, dy * self.half);
                let (nx, ny) = (dy * self.half, -dx * self.half);
                let piece = vec![left, to_grid(p.x - nx + ex, p.y - ny + ey)?, to_grid(p.x + nx + ex, p.y + ny + ey)?, right, centre];
                self.add_piece(piece);
                Ok(())
            }
            PathStrokeCap::Round => {
                // A half-disc from the left edge point, round the front, to the right one.
                let steps = ((std::f64::consts::PI / self.arc_step).ceil() as i64).max(2);
                let mut fan = vec![left];
                let (sx, sy) = (-dy, dx);
                for j in 1..steps {
                    let angle = -std::f64::consts::PI * j as f64 / steps as f64;
                    let (sin, cos) = angle.sin_cos();
                    fan.push(to_grid(p.x + (((sx * cos) - (sy * sin)) * self.half), p.y + (((sx * sin) + (sy * cos)) * self.half))?);
                }
                fan.push(right);
                fan.push(centre);
                self.add_piece(fan);
                Ok(())
            }
        }
    }

    /// A zero-length contour: a disc for round caps, an axis-aligned square for square caps.
    fn add_dot(&mut self, p: FlatPoint) -> Result<(), String> {
        let h = self.half;
        match self.cap {
            PathStrokeCap::Butt => Ok(()),
            PathStrokeCap::Square => {
                let piece = vec![to_grid(p.x - h, p.y - h)?, to_grid(p.x + h, p.y - h)?, to_grid(p.x + h, p.y + h)?, to_grid(p.x - h, p.y + h)?];
                self.add_piece(piece);
                Ok(())
            }
            PathStrokeCap::Round => {
                let steps = ((2.0 * std::f64::consts::PI / self.arc_step).ceil() as i64).max(8);
                let mut disc = Vec::with_capacity(steps as usize);
                for j in 0..steps {
                    let angle = 2.0 * std::f64::consts::PI * j as f64 / steps as f64;
                    disc.push(to_grid(p.x + (angle.cos() * h), p.y + (angle.sin() * h))?);
                }
                self.add_piece(disc);
                Ok(())
            }
        }
    }

    /// Adds a piece wound the positive way (so the non-zero union never cancels two pieces).
    fn add_piece(&mut self, piece: Vec<Point>) {
        let mut clean = Vec::with_capacity(piece.len());
        for p in piece {
            add_distinct(&mut clean, p);
        }
        if clean.len() > 1 && clean.last() == clean.first() {
            clean.pop();
        }
        if clean.len() < 3 {
            return;
        }
        let area = area2(&clean);
        if area == 0 {
            return;
        }
        if area < 0 {
            clean.reverse();
        }
        self.pieces.push(clean);
    }
}
