//! `LayoutAnchor`: a point of a rectangle as fractions of its width and height (C#
//! `LayoutAnchor`).

use crate::geometry::{PointF, RectangleF};

/// A point of a rectangle given as fractions of its size: (0, 0) the top-left corner, (1, 1) the
/// bottom-right (C# `LayoutAnchor`, with its nine named anchors).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutAnchor {
    x: f32,
    y: f32,
}

impl LayoutAnchor {
    /// The top-left corner.
    pub const TOP_LEFT: LayoutAnchor = LayoutAnchor { x: 0.0, y: 0.0 };
    /// The middle of the top edge.
    pub const TOP_CENTER: LayoutAnchor = LayoutAnchor { x: 0.5, y: 0.0 };
    /// The top-right corner.
    pub const TOP_RIGHT: LayoutAnchor = LayoutAnchor { x: 1.0, y: 0.0 };
    /// The middle of the left edge.
    pub const CENTER_LEFT: LayoutAnchor = LayoutAnchor { x: 0.0, y: 0.5 };
    /// The centre.
    pub const CENTER: LayoutAnchor = LayoutAnchor { x: 0.5, y: 0.5 };
    /// The middle of the right edge.
    pub const CENTER_RIGHT: LayoutAnchor = LayoutAnchor { x: 1.0, y: 0.5 };
    /// The bottom-left corner.
    pub const BOTTOM_LEFT: LayoutAnchor = LayoutAnchor { x: 0.0, y: 1.0 };
    /// The middle of the bottom edge.
    pub const BOTTOM_CENTER: LayoutAnchor = LayoutAnchor { x: 0.5, y: 1.0 };
    /// The bottom-right corner.
    pub const BOTTOM_RIGHT: LayoutAnchor = LayoutAnchor { x: 1.0, y: 1.0 };

    /// An anchor (C# `LayoutAnchor(x, y)`). The error says what to do with a value that is not
    /// finite.
    pub fn new(x: f32, y: f32) -> Result<LayoutAnchor, String> {
        if !x.is_finite() || !y.is_finite() {
            return Err("A layout anchor's fractions must be finite numbers (0 to 1 lie on the rectangle).".to_string());
        }
        Ok(LayoutAnchor { x, y })
    }

    /// The horizontal fraction.
    pub fn x(&self) -> f32 {
        self.x
    }

    /// The vertical fraction.
    pub fn y(&self) -> f32 {
        self.y
    }

    /// The point in a rectangle (C# `PointIn`).
    pub fn point_in(&self, rectangle: RectangleF) -> PointF {
        PointF::new(rectangle.x + (self.x * rectangle.width), rectangle.y + (self.y * rectangle.height))
    }
}
