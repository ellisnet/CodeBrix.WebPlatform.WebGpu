//! `DrawingLayout::from_guide` - a layout whose safe area is a [`SafeAreaGuide`]'s safe
//! rectangle (C#'s `DrawingLayout(calibrationSize, guide, frameSize = null, seconds = null,
//! duration = null)`).

use crate::geometry::RectangleF;
use crate::layout::DrawingLayout;
use crate::rendering::CanvasCalibration;
use crate::safe_areas::SafeAreaGuide;

/// The optional arguments of [`DrawingLayout::from_guide`] (C#'s defaults).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawingLayoutGuideOptions {
    /// The frame the video will be, pixels (None: the calibration size).
    pub frame_size: Option<(u32, u32)>,
    /// The time, for guides with a time rule (None: they apply).
    pub seconds: Option<f64>,
    /// The video's duration, for final-seconds rules.
    pub duration: Option<f64>,
}

impl DrawingLayout {
    /// A layout whose SAFE AREA is the guide's safe rectangle for a frame of `frame_size`,
    /// mapped back into calibrated units through the frame's aspect fit (C#'s constructor that
    /// takes a guide); None for the guide is the whole frame. A guide that leaves nothing gives an
    /// empty safe area. The error says what to do with a size below 1. (C#'s `Guide` and
    /// `FrameSize` properties are not kept: the layout stays a small copyable value.)
    pub fn from_guide(calibration_size: (u32, u32), guide: Option<&SafeAreaGuide>, options: DrawingLayoutGuideOptions) -> Result<DrawingLayout, String> {
        let layout = DrawingLayout::new(calibration_size)?;
        let frame = options.frame_size.unwrap_or(calibration_size);
        if frame.0 < 1 || frame.1 < 1 {
            return Err("The frame size must be at least 1 x 1.".to_string());
        }
        let Some(guide) = guide else { return Ok(layout) };
        let safe = guide.get_safe_rectangle((frame.0 as f32, frame.1 as f32), options.seconds, options.duration);
        let drawing_rect = CanvasCalibration::get_drawing_rect(frame.0 as f32, frame.1 as f32, calibration_size);
        let scale = drawing_rect.width / calibration_size.0 as f32;
        let area = if safe.width <= 0.0 || safe.height <= 0.0 || !(scale > 0.0) {
            RectangleF::default()
        } else {
            RectangleF::new((safe.x - drawing_rect.x) / scale, (safe.y - drawing_rect.y) / scale, safe.width / scale, safe.height / scale)
        };
        DrawingLayout::with_safe_area(calibration_size, area)
    }
}
