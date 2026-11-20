//! Captions (CodeBrix.Imaging.Drawing's `Captions` namespace): a [`CaptionTrack`] of timed
//! [`CaptionCue`]s read from SRT or WebVTT TEXT (no file reading: the application hands the
//! text over), and the [`CaptionPresenter`] that decides, for any time passed in, which cues
//! show and where their boxes and text go in the frame.
//!
//! ONE DELIBERATE DIFFERENCE FROM C# (the owner's rule): captions are placed against the FRAME by
//! default, kept the style's [`margin`](CaptionStyle::margin) from its edges - not inside a
//! safe-area guide. Placing them inside a safe-area guide is an opt-in on the presenter
//! ([`CaptionPlacement::SafeAreaGuide`]).

mod caption_cue;
mod caption_format;
mod caption_parser;
mod caption_placement;
mod caption_presenter;
mod caption_style;
mod caption_track;
mod placed_caption;

#[cfg(test)]
mod tests;

pub use caption_cue::CaptionCue;
pub use caption_format::CaptionFormat;
pub use caption_placement::CaptionPlacement;
pub use caption_presenter::CaptionPresenter;
pub use caption_style::CaptionStyle;
pub use caption_track::CaptionTrack;
pub use placed_caption::PlacedCaption;
