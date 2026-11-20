//! `DrawingTransitionDirection`: the way a wipe's edge or a sliding frame travels (C#
//! `DrawingTransitionDirection`).

/// The direction a wipe's edge, or a sliding / pushing frame, TRAVELS across the picture (C#
/// `DrawingTransitionDirection`). `Right` means the incoming frame appears at the left edge and
/// advances to the right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrawingTransitionDirection {
    /// Travels towards the left; the incoming frame enters from the right edge.
    Left = 0,
    /// Travels towards the right; the incoming frame enters from the left edge.
    Right = 1,
    /// Travels upwards; the incoming frame enters from the bottom edge.
    Up = 2,
    /// Travels downwards; the incoming frame enters from the top edge.
    Down = 3,
}
