//! `PathPoint`: a point on a path with its direction and distance (C# `PathPoint`).

use crate::geometry::PointF;

/// A point on a path (C# `PathPoint`): where it is, the unit direction of travel there, how far
/// along the path it is, and which contour it lies on (-1 for an empty path).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathPoint {
    /// The position.
    pub position: PointF,
    /// The unit direction of travel.
    pub tangent: PointF,
    /// The distance along the whole path, calibrated units.
    pub distance: f32,
    /// The contour's index (-1 for an empty path).
    pub contour_index: i32,
}

impl PathPoint {
    /// The direction as an angle, degrees clockwise on screen from +x (C# `Angle`).
    pub fn angle(&self) -> f32 {
        ((self.tangent.y as f64).atan2(self.tangent.x as f64) * (180.0 / std::f64::consts::PI)) as f32
    }
}
