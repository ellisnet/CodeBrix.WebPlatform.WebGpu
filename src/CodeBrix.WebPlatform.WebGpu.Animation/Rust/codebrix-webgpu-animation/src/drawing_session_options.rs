//! `DrawingSessionOptions`: how a drawing session is set up (C# `DrawingSessionOptions`, the
//! members that make sense for real-time drawing; the freehand-stroke members belong to the
//! interactive drawing tools, which are not part of this crate).

use crate::color::Color;
use crate::models::DEFAULT_STROKE_WIDTH;
use crate::text::FontRegistry;

/// How a [`DrawingSession`](crate::DrawingSession) is set up. The defaults are C#'s.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingSessionOptions {
    /// The size of the calibrated drawing space, in calibrated units (default 1000 x 1000). The
    /// session aspect-fits it into the canvas ([`CanvasCalibration`](crate::rendering::CanvasCalibration)).
    pub calibration_size: (u32, u32),
    /// The opacity every layer is composited at, 0..255 (default 100, C#'s - a heritage of the
    /// drawing tools' highlighter look; animations normally set 255).
    pub layer_opacity: u8,
    /// The colour the drawing area is filled with under the layers (default transparent, so the
    /// layers below this one on the canvas show through).
    pub background_fill_color: Color,
    /// The default stroke thickness of the session's Draw* calls, calibrated units (default 15).
    pub stroke_width: f32,
    /// A font registry for the session to use as its [`fonts`](crate::DrawingSession::fonts)
    /// (a shared handle: several sessions can draw with one registry); None (the default) gives
    /// the session an empty registry of its own.
    pub fonts: Option<FontRegistry>,
}

impl Default for DrawingSessionOptions {
    fn default() -> DrawingSessionOptions {
        DrawingSessionOptions { calibration_size: (1000, 1000), layer_opacity: 100, background_fill_color: Color::TRANSPARENT, stroke_width: DEFAULT_STROKE_WIDTH, fonts: None }
    }
}
