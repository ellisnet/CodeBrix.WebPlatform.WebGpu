//! `LayoutReference`: what a layout position is measured against (C# `LayoutReference`).

/// What a [`LayoutPosition`](crate::layout::LayoutPosition) is measured against (C#
/// `LayoutReference`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LayoutReference {
    /// The whole calibrated frame (the default).
    #[default]
    Frame = 0,
    /// The layout's safe rectangle.
    SafeArea = 1,
}
