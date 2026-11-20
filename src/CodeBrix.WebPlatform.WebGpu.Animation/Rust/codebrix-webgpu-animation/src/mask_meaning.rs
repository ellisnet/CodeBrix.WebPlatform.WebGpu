//! `MaskMeaning`: what an exported mask measures (C# `MaskMeaning`).

/// What a mask exported by [`DrawingSession::export_mask`](crate::DrawingSession::export_mask)
/// measures (C# `MaskMeaning`).
///
/// Only [`MaskMeaning::HardEdged`] is computed on the CPU from geometry, here. `Coverage` and
/// `VisibleAlpha` measure the DRAWN frame - in this crate that is the GPU's, and reading it back is
/// kept for testing only - so `export_mask` refuses them and says what to use instead.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MaskMeaning {
    /// COVERAGE (C#'s default): where each layer's ink is, as drawn - anti-aliased edges and
    /// colour alpha count, layer and session opacities do not. Measured from the drawn frame
    /// (GPU; no CPU export here).
    #[default]
    Coverage = 0,
    /// VISIBLE ALPHA: the alpha of the frame as it would be drawn over transparency - every
    /// opacity counts. Measured from the drawn frame (GPU; no CPU export here).
    VisibleAlpha = 1,
    /// HARD-EDGED COVERAGE: where each layer's elements are, 0 or 1 per pixel by ONE written-down
    /// rule - a pixel is covered when its CENTRE lies inside an element's area
    /// ([`DrawingShape::get_coverage_areas`](crate::shapes::DrawingShape::get_coverage_areas))
    /// under that area's fill rule; a centre exactly on an edge is inside on a left or top edge
    /// and outside on a right or bottom edge. Computed from geometry on the CPU, with C#'s
    /// arithmetic. Elements count when they would draw at the session's time (inside their time
    /// window, opacity above 0); transforms, groups, the layer view and the camera apply;
    /// colours, fills, effects and masks do not, nor do layer and session opacities. Hidden layers
    /// are skipped. Layers combine as a union.
    HardEdged = 2,
}
