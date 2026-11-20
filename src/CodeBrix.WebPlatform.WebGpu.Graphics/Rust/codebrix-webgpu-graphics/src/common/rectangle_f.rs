//! [`RectangleF`]: a rectangle in floating-point pixels (C#: `System.Drawing.RectangleF`).

use std::fmt;

/// A rectangle given by its top-left corner and its size (C#: `System.Drawing.RectangleF`): in
/// the sprite batch's coordinates, pixels with y pointing down, or texels of a texture.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectangleF {
    /// The left edge (C#: `X`).
    pub x: f32,
    /// The top edge (C#: `Y`).
    pub y: f32,
    /// The width (C#: `Width`).
    pub width: f32,
    /// The height (C#: `Height`).
    pub height: f32,
}

impl RectangleF {
    /// The empty rectangle at the origin (C#: `RectangleF.Empty`).
    pub const EMPTY: RectangleF = RectangleF::new(0.0, 0.0, 0.0, 0.0);

    /// A rectangle from its top-left corner and size (C#: `new RectangleF(x, y, width, height)`).
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> RectangleF {
        RectangleF { x, y, width, height }
    }

    /// A rectangle from its edges (C#: `RectangleF.FromLTRB`).
    pub fn from_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> RectangleF {
        RectangleF::new(left, top, right - left, bottom - top)
    }

    /// The left edge (C#: `Left`).
    pub fn left(&self) -> f32 {
        self.x
    }

    /// The top edge (C#: `Top`).
    pub fn top(&self) -> f32 {
        self.y
    }

    /// The right edge (C#: `Right`).
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// The bottom edge (C#: `Bottom`).
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// True when the point lies inside (left and top edges included, right and bottom not) (C#:
    /// `Contains(x, y)`).
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.left() && x < self.right() && y >= self.top() && y < self.bottom()
    }
}

impl fmt::Display for RectangleF {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{X={},Y={},Width={},Height={}}}", self.x, self.y, self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::RectangleF;

    #[test]
    fn the_edges_follow_the_corner_and_size() {
        let rect = RectangleF::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!((rect.left(), rect.top(), rect.right(), rect.bottom()), (10.0, 20.0, 40.0, 60.0));
        assert_eq!(RectangleF::from_ltrb(10.0, 20.0, 40.0, 60.0), rect);
        assert!(rect.contains(10.0, 20.0));
        assert!(!rect.contains(40.0, 30.0));
        assert_eq!(rect.to_string(), "{X=10,Y=20,Width=30,Height=40}");
    }
}
