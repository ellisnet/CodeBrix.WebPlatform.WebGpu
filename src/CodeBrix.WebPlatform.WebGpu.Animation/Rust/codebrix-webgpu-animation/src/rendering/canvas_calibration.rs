//! `CanvasCalibration`: the aspect-fit of a calibrated drawing space into a canvas (C#
//! `CanvasCalibration`, the members that make sense in a browser).

use crate::geometry::RectangleF;

/// The aspect-fit of a calibrated drawing space into a canvas or view (C# `CanvasCalibration`).
pub struct CanvasCalibration;

impl CanvasCalibration {
    /// The rectangle a drawing of `calibration_size` occupies in a view of this size: as large
    /// as fits, centred (C# `GetDrawingRect`). Empty when either size is unusable.
    pub fn get_drawing_rect(view_width: f32, view_height: f32, calibration_size: (u32, u32)) -> RectangleF {
        let (cw, ch) = calibration_size;
        if !(view_width > 0.0) || !(view_height > 0.0) || cw < 1 || ch < 1 {
            return RectangleF::default();
        }
        let scale = (view_width / cw as f32).min(view_height / ch as f32);
        let width = cw as f32 * scale;
        let height = ch as f32 * scale;
        RectangleF::new((view_width - width) / 2.0, (view_height - height) / 2.0, width, height)
    }

    /// A view point in calibrated units (C# `ViewPointToCalibrated`, rounded half away from
    /// zero and clamped to the calibrated size); None outside the drawing unless
    /// `clamp_to_drawing_area`.
    pub fn view_point_to_calibrated(view_x: f32, view_y: f32, view_width: f32, view_height: f32, calibration_size: (u32, u32), clamp_to_drawing_area: bool) -> Option<(i32, i32)> {
        let rect = CanvasCalibration::get_drawing_rect(view_width, view_height, calibration_size);
        if rect.is_empty() {
            return None;
        }
        if !clamp_to_drawing_area && !(view_x >= rect.x && view_x < rect.right() && view_y >= rect.y && view_y < rect.bottom()) {
            return None;
        }
        let (cw, ch) = calibration_size;
        let x = (((view_x - rect.x) / rect.width) * cw as f32).round() as i32;
        let y = (((view_y - rect.y) / rect.height) * ch as f32).round() as i32;
        Some((x.clamp(0, cw as i32), y.clamp(0, ch as i32)))
    }

    /// A calibrated stroke width in canvas pixels, at least 1 (C# `ScaleStrokeWidth`).
    pub fn scale_stroke_width(calibrated_width: f32, calibration_size: (u32, u32), drawing_rect: RectangleF) -> f32 {
        if calibrated_width <= 0.0 || calibration_size.0 < 1 || drawing_rect.width <= 0.0 {
            return calibrated_width;
        }
        (calibrated_width * (drawing_rect.width / calibration_size.0 as f32)).max(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::CanvasCalibration;
    use crate::geometry::RectangleF;

    #[test]
    fn a_wide_canvas_letterboxes_left_and_right() {
        assert_eq!(CanvasCalibration::get_drawing_rect(2000.0, 1000.0, (1000, 1000)), RectangleF::new(500.0, 0.0, 1000.0, 1000.0));
        assert_eq!(CanvasCalibration::get_drawing_rect(2560.0, 1440.0, (1280, 720)), RectangleF::new(0.0, 0.0, 2560.0, 1440.0));
        assert!(CanvasCalibration::get_drawing_rect(0.0, 10.0, (1, 1)).is_empty());
    }

    #[test]
    fn view_points_map_to_calibrated_units() {
        assert_eq!(CanvasCalibration::view_point_to_calibrated(1000.0, 500.0, 2000.0, 1000.0, (1000, 1000), false), Some((500, 500)));
        assert_eq!(CanvasCalibration::view_point_to_calibrated(100.0, 500.0, 2000.0, 1000.0, (1000, 1000), false), None);
        assert_eq!(CanvasCalibration::view_point_to_calibrated(100.0, 500.0, 2000.0, 1000.0, (1000, 1000), true), Some((0, 500)));
        assert_eq!(CanvasCalibration::scale_stroke_width(0.4, (1000, 1000), RectangleF::new(0.0, 0.0, 1000.0, 1000.0)), 1.0);
    }
}
