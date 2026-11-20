//! Path COMBINING on `PathGeometry` (C#'s `Union`, `Intersect`, `Subtract`, `ExclusiveOr`,
//! `CombineWith`, `CombineAll`, `Simplify`, `ToStrokeOutline`, `Inflate`, `Deflate`), on
//! i_overlay's integer overlay with NoSkia's grid and NoSkia's own stroker (see the internal
//! `path_clipper`).

use crate::paths::path_area_query::check_tolerance;
use crate::paths::{path_clipper, PathBooleanOperation, PathGeometry, PathStrokeJoin, PathStrokeStyle};

/// The optional arguments of C#'s `Inflate(distance, join = Round, miterLimit = 4, tolerance =
/// 0.25)` and `Deflate(...)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathInflateOptions {
    /// How grown (or, inside, shrunk) corners are shaped (Round by default - the exact offset).
    pub join: PathStrokeJoin,
    /// For miter joins: how far a corner's tip may reach, as a multiple of the distance (at
    /// least 1; 4 by default).
    pub miter_limit: f32,
    /// How closely curves and round corners are followed, calibrated units (0.25 by default).
    pub tolerance: f32,
}

impl Default for PathInflateOptions {
    fn default() -> PathInflateOptions {
        PathInflateOptions { join: PathStrokeJoin::Round, miter_limit: 4.0, tolerance: 0.25 }
    }
}

impl PathGeometry {
    /// Everything this geometry or `other` fills, as one outline (C# `Union`; see
    /// [`combine_with`](PathGeometry::combine_with) for how the result is made). `tolerance`:
    /// how closely curves are followed when flattened (None: 0.25 units).
    pub fn union(&self, other: &PathGeometry, tolerance: Option<f32>) -> Result<PathGeometry, String> {
        self.combine_with(other, PathBooleanOperation::Union, tolerance)
    }

    /// Only what both this geometry and `other` fill (C# `Intersect`).
    pub fn intersect(&self, other: &PathGeometry, tolerance: Option<f32>) -> Result<PathGeometry, String> {
        self.combine_with(other, PathBooleanOperation::Intersection, tolerance)
    }

    /// What this geometry fills less what `other` fills - a cut-out (C# `Subtract`).
    pub fn subtract(&self, other: &PathGeometry, tolerance: Option<f32>) -> Result<PathGeometry, String> {
        self.combine_with(other, PathBooleanOperation::Difference, tolerance)
    }

    /// What exactly one of this geometry and `other` fills (C# `ExclusiveOr`).
    pub fn exclusive_or(&self, other: &PathGeometry, tolerance: Option<f32>) -> Result<PathGeometry, String> {
        self.combine_with(other, PathBooleanOperation::ExclusiveOr, tolerance)
    }

    /// Combines the AREAS this geometry and `other` fill - each by its own fill rule, every
    /// contour taken as closed - into one outline (C# `CombineWith`). HOW: curves are flattened
    /// within `tolerance` (NoSkia's flattening), every point is rounded to the nearest 1/1024 of a
    /// unit, and the polygons are combined in exact integer arithmetic by i_overlay. The result
    /// is polygons (closed contours of lines) whose corners lie on that grid, non-overlapping,
    /// with the non-zero rule; [`PathGeometry::empty`] when nothing is left. Coordinates must
    /// stay within a million units. DIFFERENCE FROM NoSkia: the same area, but the contours'
    /// start points, directions and order are i_overlay's (a draw-on or morph of the result
    /// starts elsewhere). The error says what to do with a bad tolerance or a coordinate out of
    /// range.
    pub fn combine_with(&self, other: &PathGeometry, operation: PathBooleanOperation, tolerance: Option<f32>) -> Result<PathGeometry, String> {
        let tolerance = check_tolerance(tolerance)? as f64;
        path_clipper::combine(self, other, operation, tolerance)
    }

    /// Combines the areas of any number of geometries (C# `CombineAll`): Union - everything any
    /// of them fills; Intersection - only what all fill; Difference - the first less all the
    /// others; ExclusiveOr - what an odd number fill. One geometry gives its
    /// [`simplify`](PathGeometry::simplify); none gives [`PathGeometry::empty`].
    pub fn combine_all(geometries: &[PathGeometry], operation: PathBooleanOperation, tolerance: Option<f32>) -> Result<PathGeometry, String> {
        let tolerance = check_tolerance(tolerance)? as f64;
        path_clipper::combine_all(operation, geometries, tolerance)
    }

    /// The area this geometry fills, by its fill rule, as clean polygons: self-intersections
    /// resolved and overlapping contours merged - REMOVE OVERLAPS (C# `Simplify`).
    pub fn simplify(&self, tolerance: Option<f32>) -> Result<PathGeometry, String> {
        let tolerance = check_tolerance(tolerance)? as f64;
        path_clipper::simplify(self, tolerance)
    }

    /// STROKE TO OUTLINE: the area a stroke of this geometry in `style` covers, as a fillable
    /// outline - width, caps, joins (miter within the limit, bevelled beyond it; round; bevel),
    /// the dash pattern cut first (C# `ToStrokeOutline`). It is NoSkia's own stroker: points
    /// inside a divided curve are joined round, round caps and joins are chords within the
    /// tolerance, a zero-length open contour gives a disc (round caps), a square (square caps)
    /// or nothing (butt caps). DIFFERENCE FROM NoSkia: the dashes are cut by kurbo's dash
    /// iterator on the curves (C# cuts its own way); the outline's start points are
    /// i_overlay's.
    pub fn to_stroke_outline(&self, style: &PathStrokeStyle, tolerance: Option<f32>) -> Result<PathGeometry, String> {
        let tolerance = check_tolerance(tolerance)? as f64;
        path_clipper::stroke_outline(self, style, tolerance)
    }

    /// INFLATE: the area this geometry fills grown outward by `distance` - or shrunk inward for
    /// a negative one (C# `Inflate`); corners take the join. Made by uniting (or taking away) the
    /// fill's boundary stroked to twice the distance, so joins are exactly a stroke's. 0 gives
    /// [`simplify`](PathGeometry::simplify). The error says what to do with a value that is not
    /// finite or a miter limit below 1.
    pub fn inflate(&self, distance: f32, options: PathInflateOptions) -> Result<PathGeometry, String> {
        if !distance.is_finite() {
            return Err("The inflate distance must be a finite number of calibrated units (negative shrinks).".to_string());
        }
        if !options.miter_limit.is_finite() || options.miter_limit < 1.0 {
            return Err("The miter limit must be a finite number of at least 1 (4 is the default).".to_string());
        }
        let tolerance = check_tolerance(Some(options.tolerance))? as f64;
        path_clipper::inflate(self, distance as f64, options.join, options.miter_limit as f64, tolerance)
    }

    /// DEFLATE: the area shrunk inward by `distance` - [`inflate`](PathGeometry::inflate) with the
    /// distance negated (C# `Deflate`).
    pub fn deflate(&self, distance: f32, options: PathInflateOptions) -> Result<PathGeometry, String> {
        self.inflate(-distance, options)
    }
}
