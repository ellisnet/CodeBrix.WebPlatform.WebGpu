//! `PathGeometry` continued past its ends, and its length and points at double-precision
//! distances (C# `GetPointAt(fraction, extendBeyondEnds)`, `GetPointAtDistance(distance,
//! extendBeyondEnds)`, and the internal `LengthValue`, `PointAtDistance`,
//! `PointAtDistanceExtended` that the travel, roll and train arithmetic use).

use crate::geometry::PointF;
use crate::paths::path_measure::contours;
use crate::paths::{PathGeometry, PathPoint};

impl PathGeometry {
    /// The total length in double precision (C#'s internal `LengthValue`).
    pub(crate) fn length_value(&self) -> f64 {
        contours(self.bez_path()).iter().map(|c| c.total).sum()
    }

    /// The point at a distance, clamped to the path (C#'s internal `PointAtDistance(double)`): a
    /// distance exactly where one contour ends and the next begins lies at the start of the next.
    pub(crate) fn point_at_distance_value(&self, distance: f64) -> PathPoint {
        let list = contours(self.bez_path());
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

    /// The point at a distance; below 0 or past the length, continued straight along the first or
    /// last point's direction (C#'s internal `PointAtDistanceExtended`).
    pub(crate) fn point_at_distance_extended(&self, distance: f64) -> PathPoint {
        let total = self.length_value();
        if self.contour_count() == 0 || (distance >= 0.0 && distance <= total) {
            return self.point_at_distance_value(distance);
        }
        let end = self.point_at_distance_value(if distance < 0.0 { 0.0 } else { total });
        let beyond = if distance < 0.0 { distance } else { distance - total };
        PathPoint {
            position: PointF::new((end.position.x as f64 + end.tangent.x as f64 * beyond) as f32, (end.position.y as f64 + end.tangent.y as f64 * beyond) as f32),
            tangent: end.tangent,
            distance: distance as f32,
            contour_index: end.contour_index,
        }
    }

    /// The point at a fraction of the length, as [`get_point_at`](PathGeometry::get_point_at) -
    /// but with `extend_beyond_ends`, a fraction below 0 or above 1 is NOT clamped: the path is
    /// continued in a straight line along its first point's direction (before the start) or its
    /// last point's (after the end), so an overshoot keeps moving at the same rate; the point's
    /// distance is then below 0 or above the length (C# `GetPointAt(fraction, extendBeyondEnds)`).
    pub fn get_point_at_with(&self, fraction: f32, extend_beyond_ends: bool) -> PathPoint {
        if !extend_beyond_ends || fraction.is_nan() {
            return self.get_point_at(fraction);
        }
        self.point_at_distance_extended(fraction as f64 * self.length_value())
    }

    /// The point at a distance, as [`get_point_at_distance`](PathGeometry::get_point_at_distance) -
    /// but with `extend_beyond_ends`, continued straight past either end instead of clamped (C#
    /// `GetPointAtDistance(distance, extendBeyondEnds)`).
    pub fn get_point_at_distance_with(&self, distance: f32, extend_beyond_ends: bool) -> PathPoint {
        if !extend_beyond_ends || distance.is_nan() {
            return self.get_point_at_distance(distance);
        }
        self.point_at_distance_extended(distance as f64)
    }
}
