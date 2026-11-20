//! `LottieRange`: which frames of a document play.

/// Which frames of a document play.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum LottieRange {
    /// From the document's in point to its out point (the default).
    #[default]
    Whole,
    /// From frame `start` to frame `end` (end exclusive, as Lottie's out point); clamped to the
    /// document's in and out points.
    Frames {
        /// The first frame.
        start: f64,
        /// The frame playback stops before.
        end: f64,
    },
    /// The span of the marker of that name (its `tm` to `tm + dr`). A marker without a
    /// duration plays from its frame to the document's out point. An unknown name is an error
    /// of [`DrawingLottie::resolve_range`](crate::lottie::DrawingLottie::resolve_range).
    Marker(String),
}
