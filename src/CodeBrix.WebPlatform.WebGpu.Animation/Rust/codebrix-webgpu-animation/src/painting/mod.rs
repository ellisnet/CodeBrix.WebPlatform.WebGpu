//! The `Painting` namespace of CodeBrix.Imaging.Drawing - the PLAYBACK side of painterly
//! animation, as calculation on the CPU with NoSkia's values: ownership maps
//! ([`DrawingOwnershipMap`]: which element owns each pixel, by the hard-edged pixel rule, with
//! colour pickup, brush textures measured in each element's own axes ([`ElementPixel`]) and seams
//! between owners) and keyframe players ([`KeyframePlayer`], [`PlaneKeyframePlayer`]: a painting
//! made every few seconds, the frames between made by a transition).
//!
//! ONE THREAD: a browser gives a frontend one thread, so keyframes are painted on demand (or one
//! ahead per frame with `prefetch`) instead of by C#'s background worker, and every texture
//! function runs row by row on the caller's thread.
//!
//! NOT HERE, on purpose: anything with which a user DRAWS - pointer-driven stroke capture and
//! interactive drawing sessions (C#'s `Stroke`, `PointerPressed` / `PointerMoved` /
//! `PointerReleased`, `UndoLastStroke`). This crate plays animations; it is not a drawing tool.

mod drawing_ownership_map;
mod element_pixel;
pub(crate) mod keyframe_cache;
mod keyframe_player;
mod plane_keyframe_player;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_reference;

pub use drawing_ownership_map::DrawingOwnershipMap;
pub use element_pixel::ElementPixel;
pub use keyframe_player::{KeyframePaint, KeyframePlayer, KeyframePlayerOptions};
pub use plane_keyframe_player::{PlaneKeyframePaint, PlaneKeyframePlayer};
