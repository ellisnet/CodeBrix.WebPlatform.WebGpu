//! `CaptionPlacement`: what the caption area is (no C# counterpart: the owner's rule that captions
//! sit against the frame by default, with a safe-area guide as an opt-in).

use crate::geometry::RectangleF;
use crate::layout::{DrawingLayout, DrawingLayoutGuideOptions};
use crate::safe_areas::SafeAreaGuide;

/// What a [`CaptionPresenter`](crate::captions::CaptionPresenter)'s caption area - the rectangle
/// a position measured against [`LayoutReference::SafeArea`](crate::layout::LayoutReference::SafeArea)
/// means - is. C# always uses the layout's safe area; here the frame is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum CaptionPlacement {
    /// The frame, kept the style's [`margin`](crate::captions::CaptionStyle::margin) (times the
    /// text size) from each edge (the default).
    #[default]
    Frame,
    /// Inside a safe-area guide's rectangle, in calibrated units (the margin is not used) - C#'s
    /// placement, for a preview that must keep captions out of a platform's unsafe areas. Make it
    /// from a guide with [`CaptionPlacement::from_guide`].
    SafeAreaGuide(RectangleF),
}

impl CaptionPlacement {
    /// THE OPT-IN: captions inside a guide's safe rectangle for a calibrated frame, in
    /// calibrated units ([`DrawingLayout::from_guide`]'s safe area). The error says what to do
    /// with a size below 1.
    pub fn from_guide(calibration_size: (u32, u32), guide: &SafeAreaGuide, options: DrawingLayoutGuideOptions) -> Result<CaptionPlacement, String> {
        Ok(CaptionPlacement::SafeAreaGuide(DrawingLayout::from_guide(calibration_size, Some(guide), options)?.safe_area()))
    }
}
