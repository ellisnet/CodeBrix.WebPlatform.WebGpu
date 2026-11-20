//! The option structs that stand for the C# shape constructors' default arguments.

use crate::color::Color;
use crate::models::DEFAULT_STROKE_WIDTH;

/// The optional arguments of the circle, ellipse and path shapes (C#: `strokeThickness = 15`,
/// `color = null` (the layer's colour), `isFilled = false`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeOptions {
    /// The outline thickness, calibrated drawing units (not used when filled).
    pub stroke_thickness: f32,
    /// The colour, or None for the owning layer's colour.
    pub color: Option<Color>,
    /// True to fill; false for an outline.
    pub is_filled: bool,
}

impl Default for ShapeOptions {
    fn default() -> ShapeOptions {
        ShapeOptions { stroke_thickness: DEFAULT_STROKE_WIDTH, color: None, is_filled: false }
    }
}

/// The optional arguments of [`RectangleShape`](crate::shapes::RectangleShape) (C#:
/// `strokeThickness = 15`, `color = null`, `isFilled = false`, `cornerRadius = 0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectangleShapeOptions {
    /// The outline thickness, calibrated drawing units (not used when filled).
    pub stroke_thickness: f32,
    /// The colour, or None for the owning layer's colour.
    pub color: Option<Color>,
    /// True to fill; false for an outline.
    pub is_filled: bool,
    /// The corner radius; 0 for square corners.
    pub corner_radius: f32,
}

impl Default for RectangleShapeOptions {
    fn default() -> RectangleShapeOptions {
        RectangleShapeOptions { stroke_thickness: DEFAULT_STROKE_WIDTH, color: None, is_filled: false, corner_radius: 0.0 }
    }
}

/// The optional arguments of [`LineShape`](crate::shapes::LineShape) (C#: `strokeThickness =
/// 15`, `color = null`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineShapeOptions {
    /// The thickness, calibrated drawing units.
    pub stroke_thickness: f32,
    /// The colour, or None for the owning layer's colour.
    pub color: Option<Color>,
}

impl Default for LineShapeOptions {
    fn default() -> LineShapeOptions {
        LineShapeOptions { stroke_thickness: DEFAULT_STROKE_WIDTH, color: None }
    }
}

/// The optional arguments of [`PolylineShape`](crate::shapes::PolylineShape) (C#:
/// `strokeThickness = 15`, `color = null`, `closed = false`, `filled = false`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolylineShapeOptions {
    /// The thickness, calibrated drawing units (not used when filled).
    pub stroke_thickness: f32,
    /// The colour, or None for the owning layer's colour.
    pub color: Option<Color>,
    /// True for a polygon (the last point joins the first).
    pub is_closed: bool,
    /// True to fill (a filled polyline is a polygon).
    pub is_filled: bool,
}

impl Default for PolylineShapeOptions {
    fn default() -> PolylineShapeOptions {
        PolylineShapeOptions { stroke_thickness: DEFAULT_STROKE_WIDTH, color: None, is_closed: false, is_filled: false }
    }
}
