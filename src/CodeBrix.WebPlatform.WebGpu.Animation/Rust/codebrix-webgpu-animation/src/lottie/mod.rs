//! Lottie animations: a Lottie document's JSON TEXT (or bytes, from anywhere)
//! read through velato into a [`DrawingLottie`], played as a FUNCTION OF THE TIME passed in
//! ([`LottiePlayOptions`]: loop, ping-pong or hold, speed, a range of frames or a marker, a begin
//! time and a start offset) and evaluated into a flat [`LottieDrawList`] of plain-data
//! [`LottieDrawCommand`]s - the same shape as the `svg` module's draw list - which the drawing
//! half maps one to one onto renderer calls. [`DrawingLottieShape`] puts a document into a
//! destination rectangle with a [`DrawingLottieAspect`] fit, as the `svg` module's shape does.
//!
//! Nothing here reads a clock, a file or the network, and nothing draws. What a document uses
//! that is not drawn, or drawn in an approximated way, is in [`DrawingLottie::report`].
//! dotLottie (`.lottie`, a zip archive) is NOT read: the backend unpacks it and passes the
//! animation JSON inside.

mod document_check;
mod drawing_lottie;
mod drawing_lottie_aspect;
mod drawing_lottie_options;
mod drawing_lottie_shape;
mod drawing_lottie_warning;
mod group_folding;
mod lottie_blend_mode;
mod lottie_draw_command;
mod lottie_element;
mod lottie_image_asset;
mod lottie_marker;
mod lottie_paint;
mod lottie_play_options;
mod lottie_playback;
mod lottie_range;
mod lottie_stroke;
mod recording_sink;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests_draw;
#[cfg(test)]
mod tests_element;
#[cfg(test)]
mod tests_load;
#[cfg(test)]
mod tests_playback;

pub use drawing_lottie::DrawingLottie;
pub use drawing_lottie_aspect::DrawingLottieAspect;
pub use drawing_lottie_options::DrawingLottieOptions;
pub use drawing_lottie_shape::DrawingLottieShape;
pub use drawing_lottie_warning::{DrawingLottieWarning, DrawingLottieWarningKind};
pub use lottie_blend_mode::LottieBlendMode;
pub use lottie_draw_command::{LottieDrawCommand, LottieDrawList};
pub use lottie_image_asset::LottieImageAsset;
pub use lottie_marker::LottieMarker;
pub use lottie_paint::{LottieGradientStop, LottieLinearGradient, LottiePaint, LottieRadialGradient};
pub use lottie_play_options::LottiePlayOptions;
pub use lottie_playback::LottiePlayback;
pub use lottie_range::LottieRange;
pub use lottie_stroke::{LottieLineCap, LottieLineJoin, LottieStroke};
