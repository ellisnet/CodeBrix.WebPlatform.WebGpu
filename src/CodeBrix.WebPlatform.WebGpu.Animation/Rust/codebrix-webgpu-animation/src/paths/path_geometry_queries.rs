//! `PathGeometry`'s POINT AND AREA QUERIES (C# `Contains`, `DistanceToEdge`,
//! `GetSignedDistance`, `GetNearestPoint`, `Distance`, and `Flatten`): every query flattens the
//! geometry within a tolerance (None: 0.25 units, C#'s `DefaultFlattenTolerance`) exactly as C#
//! does - Wang's formula, the same subdivisions, points held as f32 - and then answers with
//! single-precision arithmetic in C#'s fixed order, so the answers are C#'s.
//!
//! COST: C# keeps the flattened form with the geometry for the last tolerance asked; here each
//! call flattens again (a geometry is a plain value). For many questions of one area - target
//! filling, spacing searches - use a [`FormationArea`](crate::formations::FormationArea), which
//! flattens once.

use crate::geometry::PointF;
use crate::paths::path_area_query::{check_tolerance, flatten, PathAreaQuery};
use crate::paths::{PathGeometry, PathPolyline};

impl PathGeometry {
    /// The geometry flattened into polylines within `tolerance` of the curves (C# `Flatten(tolerance
    /// = 0.25)`): Wang's formula chooses how many uniform parameter steps each curve gets; lines
    /// keep their two points; a closed contour gives a closed polyline that does not repeat its
    /// start. The error says what to do with a tolerance that is not positive.
    pub fn flatten(&self, tolerance: Option<f32>) -> Result<Vec<PathPolyline>, String> {
        Ok(flatten(self, check_tolerance(tolerance)?))
    }

    /// True when a point lies inside the filled area under the geometry's own fill rule, every
    /// contour taken as closed - the area a filled path shape paints (C# `Contains(point,
    /// tolerance = 0.25)`). A point exactly on an edge may fall either way; false for an empty
    /// geometry.
    pub fn contains(&self, point: PointF, tolerance: Option<f32>) -> Result<bool, String> {
        Ok(PathAreaQuery::new(self, tolerance)?.contains(point.x, point.y))
    }

    /// The distance from a point to the nearest EDGE of the filled area (every contour closed),
    /// inside or outside (C# `DistanceToEdge(point, tolerance = 0.25)`); infinity for an empty
    /// geometry.
    pub fn distance_to_edge(&self, point: PointF, tolerance: Option<f32>) -> Result<f32, String> {
        Ok(PathAreaQuery::new(self, tolerance)?.distance_to_edge(point.x, point.y))
    }

    /// The signed distance to the edge: negative inside (by [`contains`](PathGeometry::contains)),
    /// positive outside (C# `GetSignedDistance(point, tolerance = 0.25)`).
    pub fn get_signed_distance(&self, point: PointF, tolerance: Option<f32>) -> Result<f32, String> {
        let query = PathAreaQuery::new(self, tolerance)?;
        let distance = query.distance_to_edge(point.x, point.y);
        Ok(if query.contains(point.x, point.y) { -distance } else { distance })
    }

    /// The point ON THE PATH nearest to a point - on the contours as drawn (an open contour is
    /// not closed); where several are equally near, the first along the contours (C#
    /// `GetNearestPoint(point, tolerance = 0.25)`). The error says so for an empty geometry (C#
    /// throws).
    pub fn get_nearest_point(&self, point: PointF, tolerance: Option<f32>) -> Result<PointF, String> {
        let query = PathAreaQuery::new(self, tolerance)?;
        if query.is_empty() {
            return Err("An empty geometry has no points: give it a contour first.".to_string());
        }
        Ok(query.nearest_point(point.x, point.y))
    }

    /// The smallest distance between this geometry's filled area and another's: 0 when they
    /// touch or overlap (a vertex of either inside the other, or edges crossing), otherwise the
    /// gap between their nearest points; infinity when either is empty (C# `Distance(other,
    /// tolerance = 0.25)`).
    pub fn distance(&self, other: &PathGeometry, tolerance: Option<f32>) -> Result<f32, String> {
        self.distance_with_offset(other, PointF::new(0.0, 0.0), tolerance)
    }

    /// The distance between this area and another MOVED by `other_offset` - "how far apart would
    /// they be if I put it there?" without building a moved geometry (C# `Distance(other,
    /// otherOffset, tolerance = 0.25)`). The error says what to do with an offset that is not
    /// finite.
    pub fn distance_with_offset(&self, other: &PathGeometry, other_offset: PointF, tolerance: Option<f32>) -> Result<f32, String> {
        if !other_offset.x.is_finite() || !other_offset.y.is_finite() {
            return Err("The offset must be finite numbers.".to_string());
        }
        let a = PathAreaQuery::new(self, tolerance)?;
        let b = PathAreaQuery::new(other, tolerance)?;
        Ok(a.distance_to(&b, other_offset.x, other_offset.y))
    }
}
