//! `DrawingLayout`: placing things against the frame or a safe rectangle, so one scene
//! description works for any frame shape (C# `DrawingLayout`).

use crate::geometry::{PointF, RectangleF};
use crate::layout::{LayoutAnchor, LayoutPosition, LayoutReference};

/// A layout of a calibrated frame (C# `DrawingLayout`): the FRAME rectangle, a SAFE AREA inside it
/// (the frame itself when there is none), and placement against either - points at anchors with
/// offsets, and boxes whose own anchor sits there. Make it from a
/// [`SafeAreaGuide`](crate::safe_areas::SafeAreaGuide) with [`DrawingLayout::from_guide`] (C#'s
/// constructor that takes a guide); a guide of your own margins is
/// [`SafeAreaGuide::new`](crate::safe_areas::SafeAreaGuide::new).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawingLayout {
    calibration_size: (u32, u32),
    frame: RectangleF,
    safe_area: RectangleF,
}

impl DrawingLayout {
    /// A layout whose safe area is the whole frame (C# `DrawingLayout(calibrationSize)`). The
    /// error says what to do with a size below 1.
    pub fn new(calibration_size: (u32, u32)) -> Result<DrawingLayout, String> {
        if calibration_size.0 < 1 || calibration_size.1 < 1 {
            return Err("A layout's calibration size must be at least 1 x 1.".to_string());
        }
        let frame = RectangleF::new(0.0, 0.0, calibration_size.0 as f32, calibration_size.1 as f32);
        Ok(DrawingLayout { calibration_size, frame, safe_area: frame })
    }

    /// A layout with a safe rectangle, in calibrated units (crate-internal: what
    /// [`DrawingLayout::from_guide`] computes from a guide; an empty rectangle is allowed, as C#'s
    /// for a guide that leaves nothing).
    pub(crate) fn with_safe_area(calibration_size: (u32, u32), safe_area: RectangleF) -> Result<DrawingLayout, String> {
        let mut layout = DrawingLayout::new(calibration_size)?;
        layout.safe_area = safe_area;
        Ok(layout)
    }

    /// The calibrated size.
    pub fn calibration_size(&self) -> (u32, u32) {
        self.calibration_size
    }

    /// The whole frame, (0, 0) to the calibrated size.
    pub fn frame(&self) -> RectangleF {
        self.frame
    }

    /// The safe rectangle (the frame when there is none).
    pub fn safe_area(&self) -> RectangleF {
        self.safe_area
    }

    /// True when the frame is taller than wide.
    pub fn is_portrait(&self) -> bool {
        self.calibration_size.1 > self.calibration_size.0
    }

    /// The shorter side, calibrated units - a size unit that suits both formats.
    pub fn short_side(&self) -> f32 {
        self.calibration_size.0.min(self.calibration_size.1) as f32
    }

    /// The frame or the safe area (C# `GetRectangle`).
    pub fn get_rectangle(&self, reference: LayoutReference) -> RectangleF {
        match reference {
            LayoutReference::SafeArea => self.safe_area,
            LayoutReference::Frame => self.frame,
        }
    }

    /// The point a position names (C# `Place(LayoutPosition)`): the anchor in its rectangle, plus
    /// the fraction offset times that rectangle's size, plus the unit offset.
    pub fn place(&self, position: &LayoutPosition) -> PointF {
        let r = self.get_rectangle(position.reference());
        let a = position.anchor().point_in(r);
        PointF::new(a.x + (position.fraction_offset().x * r.width) + position.offset().x, a.y + (position.fraction_offset().y * r.height) + position.offset().y)
    }

    /// The point at an anchor, moved by units (C# `Place(anchor, reference = Frame, offsetX = 0,
    /// offsetY = 0)`).
    pub fn place_at(&self, anchor: LayoutAnchor, reference: LayoutReference, offset_x: f32, offset_y: f32) -> PointF {
        let r = self.get_rectangle(reference);
        let a = anchor.point_in(r);
        let (ox, oy) = if offset_x.is_finite() && offset_y.is_finite() { (offset_x, offset_y) } else { (0.0, 0.0) };
        PointF::new(a.x + ox, a.y + oy)
    }

    /// A box of a size whose own anchor (the position's) lies at the position (C# `PlaceBox`): a
    /// top-right position gives a box hanging down and left from it.
    pub fn place_box(&self, size: (f32, f32), position: &LayoutPosition) -> RectangleF {
        let p = self.place(position);
        RectangleF::new(p.x - (position.anchor().x() * size.0), p.y - (position.anchor().y() * size.1), size.0, size.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutPositionOptions;

    #[test]
    fn places_against_the_frame_or_the_safe_area_as_csharp() {
        // The Instagram Reels ad margins (top 14 %, bottom 35 %, sides 6 %) of a 1080 x 1920 frame.
        let safe = RectangleF::new(64.8, 268.8, 950.4, 979.2);
        let layout = DrawingLayout::with_safe_area((1080, 1920), safe).unwrap();
        assert!(layout.is_portrait() && layout.short_side() == 1080.0);
        assert_eq!(layout.place_at(LayoutAnchor::CENTER, LayoutReference::Frame, 0.0, 0.0), PointF::new(540.0, 960.0));
        let c = layout.place_at(LayoutAnchor::CENTER, LayoutReference::SafeArea, 0.0, 0.0);
        assert!((c.x - 540.0).abs() < 1e-3 && (c.y - 758.4).abs() < 1e-3, "{c:?}");
        // BottomCenter of the safe area, 5 % of its height up.
        let pos = LayoutPosition::new(LayoutAnchor::BOTTOM_CENTER, LayoutPositionOptions { reference: LayoutReference::SafeArea, fraction_y: -0.05, ..LayoutPositionOptions::default() }).unwrap();
        let p = layout.place(&pos);
        assert!((p.y - (1248.0 - 0.05 * 979.2)).abs() < 1e-2, "{p:?}");
        // A 100 x 50 box at the safe area's top-right, 20 units in: it hangs down and left.
        let tr = LayoutPosition::new(LayoutAnchor::TOP_RIGHT, LayoutPositionOptions { reference: LayoutReference::SafeArea, offset_x: -20.0, offset_y: 20.0, ..LayoutPositionOptions::default() }).unwrap();
        let b = layout.place_box((100.0, 50.0), &tr);
        assert!((b.x - (1015.2 - 20.0 - 100.0)).abs() < 1e-3 && (b.y - 288.8).abs() < 1e-3, "{b:?}");
        assert!(DrawingLayout::new((0, 10)).is_err() && LayoutAnchor::new(f32::NAN, 0.0).is_err());
        assert_eq!(DrawingLayout::new((10, 10)).unwrap().safe_area(), RectangleF::new(0.0, 0.0, 10.0, 10.0));
    }
}
