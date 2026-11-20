//! `DrawingMaskKind` (C# `DrawingMaskKind`, the same members and numbers).

/// What a [`DrawingMask`](crate::effects::DrawingMask) keeps of the content it is applied to (C#
/// `DrawingMaskKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrawingMaskKind {
    /// Only what lies inside a path (a clip).
    Inside = 0,
    /// Only what lies outside a path (a cut-out).
    Outside = 1,
    /// The content where another layer's elements cover the frame (a matte), or where they do not
    /// when inverted.
    Layer = 2,
    /// The content scaled by the values of a pixel plane (an alpha mask; C#'s `FromPlane`).
    Plane = 3,
}
