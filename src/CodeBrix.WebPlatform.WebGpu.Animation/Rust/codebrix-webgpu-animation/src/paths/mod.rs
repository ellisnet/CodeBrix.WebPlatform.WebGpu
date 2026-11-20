//! The path vocabulary of CodeBrix.Imaging.Drawing's `Paths` namespace: geometry with a fill
//! rule (built, measured, trimmed, transformed), points on a path, morphs between paths, and
//! stroke styles; point and area queries (inside, distances, nearest points) on the flattened
//! path, and the path continued past its ends; and path COMBINING - union, intersection,
//! difference, exclusive-or, simplify (remove overlaps), stroke to outline, inflate and deflate -
//! on i_overlay's integer overlay with NoSkia's grid and NoSkia's own stroker.

mod path_area_query;
mod path_boolean_operation;
pub(crate) mod path_clipper;
mod path_fill_rule;
mod path_geometry;
mod path_geometry_combine;
mod path_geometry_queries;
mod path_geometry_travel;
mod path_measure;
mod path_morph;
mod path_point;
mod path_polyline;
mod path_stroke_cap;
mod path_stroke_join;
mod path_stroke_style;

#[cfg(test)]
mod tests_combine;
#[cfg(test)]
mod tests_dash;
#[cfg(test)]
mod tests_queries;

pub(crate) use path_area_query::PathAreaQuery;
pub use path_boolean_operation::PathBooleanOperation;
pub use path_fill_rule::PathFillRule;
pub use path_geometry::{PathGeometry, PathGeometryBuilder};
pub use path_geometry_combine::PathInflateOptions;
pub use path_morph::PathMorph;
pub use path_point::PathPoint;
pub use path_polyline::PathPolyline;
pub use path_stroke_cap::PathStrokeCap;
pub use path_stroke_join::PathStrokeJoin;
pub use path_stroke_style::{PathStrokeStyle, PathStrokeStyleOptions};
