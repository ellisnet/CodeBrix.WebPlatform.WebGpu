//! The small geometry value types the C# API takes from CodeBrix.Imaging (`PointF`,
//! `RectangleF` in calibrated drawing units; `Rectangle` in whole pixels).

/// A point, in calibrated drawing units (C# `PointF`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointF {
    /// The horizontal position.
    pub x: f32,
    /// The vertical position (down is positive).
    pub y: f32,
}

impl PointF {
    /// A point.
    pub const fn new(x: f32, y: f32) -> PointF {
        PointF { x, y }
    }

    pub(crate) fn to_kurbo(self) -> kurbo::Point {
        kurbo::Point::new(self.x as f64, self.y as f64)
    }
}

impl From<(f32, f32)> for PointF {
    fn from((x, y): (f32, f32)) -> PointF {
        PointF { x, y }
    }
}

/// A rectangle (C# `RectangleF`): left, top, width, height.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectangleF {
    /// The left edge.
    pub x: f32,
    /// The top edge.
    pub y: f32,
    /// The width.
    pub width: f32,
    /// The height.
    pub height: f32,
}

impl RectangleF {
    /// A rectangle.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> RectangleF {
        RectangleF { x, y, width, height }
    }

    /// True when it has no area.
    pub fn is_empty(&self) -> bool {
        !(self.width > 0.0 && self.height > 0.0)
    }

    /// The right edge.
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// The bottom edge.
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
}

/// A rectangle of whole pixels (C# `Rectangle`): left, top, width, height - the part of a
/// picture an [`ImageShape`](crate::shapes::ImageShape) draws, or a cell a
/// [`SpriteSheet`](crate::images::SpriteSheet) cuts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rectangle {
    /// The left edge, pixels.
    pub x: i32,
    /// The top edge, pixels.
    pub y: i32,
    /// The width, pixels.
    pub width: i32,
    /// The height, pixels.
    pub height: i32,
}

impl Rectangle {
    /// A rectangle.
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Rectangle {
        Rectangle { x, y, width, height }
    }

    /// The right edge (exclusive).
    pub fn right(&self) -> i32 {
        self.x + self.width
    }

    /// The bottom edge (exclusive).
    pub fn bottom(&self) -> i32 {
        self.y + self.height
    }
}
