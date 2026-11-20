//! `PathPolyline`: one flattened contour - points joined by straight lines (C# `PathPolyline`).

use crate::geometry::PointF;

/// One flattened contour (C# `PathPolyline`): its points and whether it closes back to the first
/// (a closed polyline does not repeat its first point).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PathPolyline {
    points: Vec<PointF>,
    is_closed: bool,
}

impl PathPolyline {
    /// A polyline (C#'s constructor).
    pub fn new(points: Vec<PointF>, is_closed: bool) -> PathPolyline {
        PathPolyline { points, is_closed }
    }

    /// True when it closes back to its first point (C# `IsClosed`).
    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    /// The number of points (C# `Count`).
    pub fn count(&self) -> usize {
        self.points.len()
    }

    /// The points (C# `GetPoints` / the indexer).
    pub fn get_points(&self) -> &[PointF] {
        &self.points
    }
}
