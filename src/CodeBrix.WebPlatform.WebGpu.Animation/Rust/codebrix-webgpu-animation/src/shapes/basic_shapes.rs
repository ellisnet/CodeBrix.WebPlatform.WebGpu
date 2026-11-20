//! The basic shapes of CodeBrix.Imaging.Drawing's `Shapes` namespace: rectangle (with rounded
//! corners), circle, ellipse, line and polyline (C# `RectangleShape`, `CircleShape`,
//! `EllipseShape`, `LineShape`, `PolylineShape`). Coordinates are calibrated drawing units.

use crate::color::Color;
use crate::fills::ShapeFill;
use crate::geometry::{PointF, RectangleF};
use crate::shapes::shape_options::{LineShapeOptions, PolylineShapeOptions, RectangleShapeOptions, ShapeOptions};

/// A rectangle, outlined or filled, with optional rounded corners.
#[derive(Clone, Debug, PartialEq)]
pub struct RectangleShape {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) corner_radius: f32,
    pub(crate) stroke_thickness: f32,
    pub(crate) color: Option<Color>,
    pub(crate) is_filled: bool,
    pub(crate) fill: Option<ShapeFill>,
}

impl RectangleShape {
    /// A rectangle. The error (C# ArgumentOutOfRangeException) says what to do: a positive
    /// width and height and a corner radius that is not negative.
    pub fn new(x: f32, y: f32, width: f32, height: f32, options: RectangleShapeOptions) -> Result<RectangleShape, String> {
        check_size(width, height)?;
        if options.corner_radius < 0.0 {
            return Err("A corner radius cannot be negative: use 0 for square corners.".to_string());
        }
        check_thickness(options.stroke_thickness)?;
        Ok(RectangleShape { x, y, width, height, corner_radius: options.corner_radius, stroke_thickness: options.stroke_thickness, color: options.color, is_filled: options.is_filled, fill: None })
    }

    /// A filled rectangle painted by a fill instead of a flat colour (C# `RectangleShape(x, y,
    /// width, height, fill, cornerRadius = 0)`).
    pub fn with_fill(x: f32, y: f32, width: f32, height: f32, fill: ShapeFill, corner_radius: f32) -> Result<RectangleShape, String> {
        let mut shape = RectangleShape::new(x, y, width, height, RectangleShapeOptions { corner_radius, is_filled: true, ..RectangleShapeOptions::default() })?;
        shape.fill = Some(fill);
        Ok(shape)
    }

    /// The left edge.
    pub fn x(&self) -> f32 {
        self.x
    }

    /// The top edge.
    pub fn y(&self) -> f32 {
        self.y
    }

    /// The width.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// The height.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// The corner radius.
    pub fn corner_radius(&self) -> f32 {
        self.corner_radius
    }

    /// True when filled.
    pub fn is_filled(&self) -> bool {
        self.is_filled
    }

    /// The bounds, the outline's half-thickness included.
    pub fn get_bounds(&self) -> RectangleF {
        let half = if self.is_filled { 0.0 } else { self.stroke_thickness / 2.0 };
        RectangleF::new(self.x - half, self.y - half, self.width + 2.0 * half, self.height + 2.0 * half)
    }
}

/// A circle, outlined or filled.
#[derive(Clone, Debug, PartialEq)]
pub struct CircleShape {
    pub(crate) center_x: f32,
    pub(crate) center_y: f32,
    pub(crate) radius: f32,
    pub(crate) stroke_thickness: f32,
    pub(crate) color: Option<Color>,
    pub(crate) is_filled: bool,
    pub(crate) fill: Option<ShapeFill>,
}

impl CircleShape {
    /// A circle. The error says what to do: a positive radius.
    pub fn new(center_x: f32, center_y: f32, radius: f32, options: ShapeOptions) -> Result<CircleShape, String> {
        if !(radius > 0.0) {
            return Err("A circle's radius must be positive.".to_string());
        }
        check_thickness(options.stroke_thickness)?;
        Ok(CircleShape { center_x, center_y, radius, stroke_thickness: options.stroke_thickness, color: options.color, is_filled: options.is_filled, fill: None })
    }

    /// A filled circle painted by a fill (C# `CircleShape(centerX, centerY, radius, fill)`).
    pub fn with_fill(center_x: f32, center_y: f32, radius: f32, fill: ShapeFill) -> Result<CircleShape, String> {
        let mut shape = CircleShape::new(center_x, center_y, radius, ShapeOptions { is_filled: true, ..ShapeOptions::default() })?;
        shape.fill = Some(fill);
        Ok(shape)
    }

    /// The centre's x.
    pub fn center_x(&self) -> f32 {
        self.center_x
    }

    /// The centre's y.
    pub fn center_y(&self) -> f32 {
        self.center_y
    }

    /// The radius.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// True when filled.
    pub fn is_filled(&self) -> bool {
        self.is_filled
    }
}

/// An ellipse, outlined or filled.
#[derive(Clone, Debug, PartialEq)]
pub struct EllipseShape {
    pub(crate) center_x: f32,
    pub(crate) center_y: f32,
    pub(crate) radius_x: f32,
    pub(crate) radius_y: f32,
    pub(crate) stroke_thickness: f32,
    pub(crate) color: Option<Color>,
    pub(crate) is_filled: bool,
    pub(crate) fill: Option<ShapeFill>,
}

impl EllipseShape {
    /// An ellipse. The error says what to do: positive radii.
    pub fn new(center_x: f32, center_y: f32, radius_x: f32, radius_y: f32, options: ShapeOptions) -> Result<EllipseShape, String> {
        if !(radius_x > 0.0) || !(radius_y > 0.0) {
            return Err("An ellipse's radii must be positive.".to_string());
        }
        check_thickness(options.stroke_thickness)?;
        Ok(EllipseShape { center_x, center_y, radius_x, radius_y, stroke_thickness: options.stroke_thickness, color: options.color, is_filled: options.is_filled, fill: None })
    }

    /// A filled ellipse painted by a fill.
    pub fn with_fill(center_x: f32, center_y: f32, radius_x: f32, radius_y: f32, fill: ShapeFill) -> Result<EllipseShape, String> {
        let mut shape = EllipseShape::new(center_x, center_y, radius_x, radius_y, ShapeOptions { is_filled: true, ..ShapeOptions::default() })?;
        shape.fill = Some(fill);
        Ok(shape)
    }

    /// The horizontal radius.
    pub fn radius_x(&self) -> f32 {
        self.radius_x
    }

    /// The vertical radius.
    pub fn radius_y(&self) -> f32 {
        self.radius_y
    }
}

/// A straight line with round caps.
#[derive(Clone, Debug, PartialEq)]
pub struct LineShape {
    pub(crate) x1: f32,
    pub(crate) y1: f32,
    pub(crate) x2: f32,
    pub(crate) y2: f32,
    pub(crate) stroke_thickness: f32,
    pub(crate) color: Option<Color>,
}

impl LineShape {
    /// A line. The error says what to do: a positive thickness.
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32, options: LineShapeOptions) -> Result<LineShape, String> {
        check_thickness(options.stroke_thickness)?;
        Ok(LineShape { x1, y1, x2, y2, stroke_thickness: options.stroke_thickness, color: options.color })
    }

    /// The start point.
    pub fn start(&self) -> PointF {
        PointF::new(self.x1, self.y1)
    }

    /// The end point.
    pub fn end(&self) -> PointF {
        PointF::new(self.x2, self.y2)
    }
}

/// A polyline, or a polygon when closed, outlined (round joins and caps) or filled.
#[derive(Clone, Debug, PartialEq)]
pub struct PolylineShape {
    pub(crate) points: Vec<PointF>,
    pub(crate) stroke_thickness: f32,
    pub(crate) color: Option<Color>,
    pub(crate) is_closed: bool,
    pub(crate) is_filled: bool,
    pub(crate) fill: Option<ShapeFill>,
}

impl PolylineShape {
    /// A polyline. The error says what to do: at least two points.
    pub fn new(points: &[PointF], options: PolylineShapeOptions) -> Result<PolylineShape, String> {
        if points.len() < 2 {
            return Err("A polyline needs at least two points.".to_string());
        }
        check_thickness(options.stroke_thickness)?;
        Ok(PolylineShape { points: points.to_vec(), stroke_thickness: options.stroke_thickness, color: options.color, is_closed: options.is_closed || options.is_filled, is_filled: options.is_filled, fill: None })
    }

    /// A filled polygon painted by a fill.
    pub fn with_fill(points: &[PointF], fill: ShapeFill) -> Result<PolylineShape, String> {
        let mut shape = PolylineShape::new(points, PolylineShapeOptions { is_filled: true, is_closed: true, ..PolylineShapeOptions::default() })?;
        shape.fill = Some(fill);
        Ok(shape)
    }

    /// The number of points.
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// A copy of the points.
    pub fn get_points(&self) -> Vec<PointF> {
        self.points.clone()
    }

    /// True for a polygon.
    pub fn is_closed(&self) -> bool {
        self.is_closed
    }
}

fn check_size(width: f32, height: f32) -> Result<(), String> {
    if !(width > 0.0) || !(height > 0.0) {
        return Err("A rectangle's width and height must be positive.".to_string());
    }
    Ok(())
}

pub(crate) fn check_thickness(thickness: f32) -> Result<(), String> {
    if !(thickness > 0.0) || !thickness.is_finite() {
        return Err("A stroke thickness must be positive and finite (it must be given even for a filled shape; 1 will do).".to_string());
    }
    Ok(())
}
