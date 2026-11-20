//! THE HARD-EDGED PIXEL RULE (internal; C#'s `HardEdgeRasterizer` and `HardEdgeMatrix`), on the
//! CPU in double precision:
//! - A pixel (px, py) belongs to an area when its CENTRE (px + 0.5, py + 0.5), in output pixels,
//!   is inside the area under the area's own fill rule (non-zero: winding number not 0;
//!   even-odd: odd). Curves are first flattened to polylines; every contour counts as closed.
//! - Ties - a centre exactly on an edge - follow the "top-left" rule: an edge counts for a row
//!   when yTop <= cy < yBottom, and a crossing counts for a centre when crossingX <= cx. So a
//!   centre on a left or top edge is inside, on a right or bottom edge outside, and two areas
//!   that share an edge never both take (or both miss) a centre lying on it. Horizontal edges
//!   never count.
//! - Each point is mapped once by the area's matrix; an edge from its top end (xt, yt) to its
//!   bottom end (xb, yb) crosses row cy at x = xt + (cy - yt) * ((xb - xt) / (yb - yt)).
//!
//! The same arithmetic in the same order as C#, so the same polylines and matrices give the same
//! pixels.

use kurbo::Affine;

use crate::paths::{PathFillRule, PathPolyline};

/// An affine map in double precision, in the row-vector convention of C#'s `Matrix3x2`
/// (`x' = x m11 + y m21 + m31`, `y' = x m12 + y m22 + m32`) (C#'s `HardEdgeMatrix`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HardEdgeMatrix {
    pub(crate) m11: f64,
    pub(crate) m12: f64,
    pub(crate) m21: f64,
    pub(crate) m22: f64,
    pub(crate) m31: f64,
    pub(crate) m32: f64,
}

impl HardEdgeMatrix {
    pub(crate) fn new(m11: f64, m12: f64, m21: f64, m22: f64, m31: f64, m32: f64) -> HardEdgeMatrix {
        HardEdgeMatrix { m11, m12, m21, m22, m31, m32 }
    }

    /// The same map as a kurbo affine (kurbo's coefficients a b c d e f are m11 m12 m21 m22 m31
    /// m32).
    pub(crate) fn from_affine(affine: Affine) -> HardEdgeMatrix {
        let [a, b, c, d, e, f] = affine.as_coeffs();
        HardEdgeMatrix { m11: a, m12: b, m21: c, m22: d, m31: e, m32: f }
    }

    /// This map first, then `next` (C# `Then`).
    pub(crate) fn then(&self, next: &HardEdgeMatrix) -> HardEdgeMatrix {
        HardEdgeMatrix {
            m11: (self.m11 * next.m11) + (self.m12 * next.m21),
            m12: (self.m11 * next.m12) + (self.m12 * next.m22),
            m21: (self.m21 * next.m11) + (self.m22 * next.m21),
            m22: (self.m21 * next.m12) + (self.m22 * next.m22),
            m31: (self.m31 * next.m11) + (self.m32 * next.m21) + next.m31,
            m32: (self.m31 * next.m12) + (self.m32 * next.m22) + next.m32,
        }
    }

    pub(crate) fn determinant(&self) -> f64 {
        (self.m11 * self.m22) - (self.m12 * self.m21)
    }

    /// The largest factor by which the map stretches any direction (C# `MaxStretch`).
    pub(crate) fn max_stretch(&self) -> f64 {
        let s = (self.m11 * self.m11) + (self.m12 * self.m12) + (self.m21 * self.m21) + (self.m22 * self.m22);
        let det = self.determinant();
        let root = ((s * s) - (4.0 * det * det)).max(0.0).sqrt();
        ((s + root) / 2.0).sqrt()
    }

    /// The inverse, when there is one (C# `TryInvert`).
    pub(crate) fn try_invert(&self) -> Option<HardEdgeMatrix> {
        let det = self.determinant();
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        let (i11, i12, i21, i22) = (self.m22 / det, -self.m12 / det, -self.m21 / det, self.m11 / det);
        Some(HardEdgeMatrix::new(i11, i12, i21, i22, -((self.m31 * i11) + (self.m32 * i21)), -((self.m31 * i12) + (self.m32 * i22))))
    }

    pub(crate) fn transform(&self, x: f64, y: f64) -> (f64, f64) {
        ((x * self.m11) + (y * self.m21) + self.m31, (x * self.m12) + (y * self.m22) + self.m32)
    }
}

#[derive(Clone, Copy)]
struct Edge {
    x_top: f64,
    y_top: f64,
    slope: f64,
    direction: i32,
    row_start: i32,
    row_end: i32,
}

#[derive(Clone, Copy)]
struct Crossing {
    x: f64,
    direction: i32,
}

/// The smallest whole p in 0..limit with p + 0.5 >= value (limit when none below it qualifies);
/// compared exactly in double, so a centre exactly on the value is "at or after" it (C#
/// `FirstCentreAtOrAfter`).
pub(crate) fn first_centre_at_or_after(value: f64, limit: i32) -> i32 {
    if !(value > 0.5) {
        return 0;
    }
    if value > limit as f64 - 0.5 {
        return limit;
    }
    let mut p = (value - 0.5).ceil() as i32;
    while (p as f64) + 0.5 < value {
        p += 1;
    }
    while p > 0 && (p as f64) - 0.5 >= value {
        p -= 1;
    }
    p.clamp(0, limit)
}

fn add_edge(edges: &mut Vec<Edge>, x0: f64, y0: f64, x1: f64, y1: f64, height: i32) {
    if y0 == y1 {
        return;
    }
    if !x0.is_finite() || !y0.is_finite() || !x1.is_finite() || !y1.is_finite() {
        return;
    }
    let direction = if y1 > y0 { 1 } else { -1 };
    let (xt, yt, xb, yb) = if y1 < y0 { (x1, y1, x0, y0) } else { (x0, y0, x1, y1) };
    let row_start = first_centre_at_or_after(yt, height);
    let row_end = first_centre_at_or_after(yb, height);
    if row_end <= row_start {
        return;
    }
    edges.push(Edge { x_top: xt, y_top: yt, slope: (xb - xt) / (yb - yt), direction, row_start, row_end });
}

/// Fills one area (C# `Fill`): its flattened contours in the area's own coordinates, mapped by
/// `matrix`; `writer(y, x0, x1)` receives every covered span `[x0, x1)` of row y, already
/// clipped to the frame.
pub(crate) fn fill(polylines: &[PathPolyline], matrix: &HardEdgeMatrix, fill_rule: PathFillRule, width: i32, height: i32, writer: &mut dyn FnMut(i32, i32, i32)) {
    let mut edges: Vec<Edge> = Vec::new();
    for polyline in polylines {
        let points = polyline.get_points();
        let count = points.len();
        if count < 2 {
            continue;
        }
        let (start_x, start_y) = matrix.transform(points[0].x as f64, points[0].y as f64);
        let (mut previous_x, mut previous_y) = (start_x, start_y);
        for i in 1..=count {
            let (x, y) = if i < count { matrix.transform(points[i].x as f64, points[i].y as f64) } else { (start_x, start_y) };
            add_edge(&mut edges, previous_x, previous_y, x, y, height);
            previous_x = x;
            previous_y = y;
        }
    }
    if edges.is_empty() {
        return;
    }

    // The order among edges that start on the same row does not matter: crossings are sorted
    // by x below, and equal x values span no pixel centre.
    edges.sort_by_key(|e| e.row_start);
    let first_row = edges[0].row_start;
    let last_row = edges.iter().fold(0, |m, e| m.max(e.row_end));

    let mut active: Vec<Edge> = Vec::new();
    let mut crossings: Vec<Crossing> = Vec::new();
    let mut next = 0;
    let even_odd = fill_rule == PathFillRule::EvenOdd;
    for row in first_row..last_row {
        while next < edges.len() && edges[next].row_start <= row {
            active.push(edges[next]);
            next += 1;
        }
        active.retain(|e| e.row_end > row);
        if active.is_empty() {
            continue;
        }

        let cy = row as f64 + 0.5;
        crossings.clear();
        for edge in &active {
            crossings.push(Crossing { x: edge.x_top + ((cy - edge.y_top) * edge.slope), direction: edge.direction });
        }
        sort_by_x(&mut crossings);

        let mut winding = 0;
        for k in 0..crossings.len().saturating_sub(1) {
            winding += crossings[k].direction;
            let inside = if even_odd { (winding & 1) != 0 } else { winding != 0 };
            if !inside {
                continue;
            }
            let x0 = first_centre_at_or_after(crossings[k].x, width);
            let x1 = first_centre_at_or_after(crossings[k + 1].x, width);
            if x1 > x0 {
                writer(row, x0, x1);
            }
        }
    }
}

// Insertion sort by x, as C# (rows hold few crossings)
fn sort_by_x(crossings: &mut [Crossing]) {
    for i in 1..crossings.len() {
        let item = crossings[i];
        let mut j = i;
        while j > 0 && crossings[j - 1].x > item.x {
            crossings[j] = crossings[j - 1];
            j -= 1;
        }
        crossings[j] = item;
    }
}
