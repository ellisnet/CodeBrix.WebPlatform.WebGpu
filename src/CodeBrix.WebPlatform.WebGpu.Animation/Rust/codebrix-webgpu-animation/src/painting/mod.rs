//! The `Painting` namespace of CodeBrix.Imaging.Drawing - the PLAYBACK side of painterly
//! animation, as calculation on the CPU with NoSkia's values: ownership maps
//! ([`DrawingOwnershipMap`]: which element owns each pixel, by the hard-edged pixel rule, with
//! colour pickup, brush textures measured in each element's own axes ([`ElementPixel`]) and seams
//! between owners) and keyframe players ([`KeyframePlayer`], [`PlaneKeyframePlayer`]: a painting
//! made every few seconds, the frames between made by a transition).
//!
//! Beside them, with no C# equivalent, the PLANNING and COLOUR side of a painting (planning-time
//! CPU work, deterministic; these are not pinned to NoSkia's values): direction fields
//! ([`DirectionField`] from a picture's structure, noise or [`DirectionGuide`]s), detail maps
//! ([`DetailMap`]), stroke layout ([`StrokeLayout`], [`element_area`]) into
//! [`BrushStrokeRecord`](crate::strokes::BrushStrokeRecord)s, the [`PaintingTimeline`] that gives
//! each stroke its start and end, colour rules for picked-up colour ([`ColorRules`]: a palette,
//! broken colour, in Oklab) and colour mixing in a chosen space ([`ColorMix`],
//! [`ColorInterpolation`] - straight sRGB, NoSkia's, by default everywhere).
//!
//! ONE THREAD: a browser gives a frontend one thread, so keyframes are painted on demand (or one
//! ahead per frame with `prefetch`) instead of by C#'s background worker, and every texture
//! function runs row by row on the caller's thread.
//!
//! NOT HERE, on purpose: anything with which a user DRAWS - pointer-driven stroke capture and
//! interactive drawing sessions (C#'s `Stroke`, `PointerPressed` / `PointerMoved` /
//! `PointerReleased`, `UndoLastStroke`). This crate plays animations; it is not a drawing tool.

pub(crate) mod color_mix;
pub(crate) mod color_rules;
mod detail;
mod direction_field;
mod direction_guide;
mod drawing_ownership_map;
mod element_pixel;
pub(crate) mod keyframe_cache;
mod keyframe_player;
mod painting_timeline;
mod plane_keyframe_player;
mod stroke_layout;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_color;
#[cfg(test)]
mod tests_fields;
#[cfg(test)]
mod tests_reference;
#[cfg(test)]
mod tests_timeline;

pub use drawing_ownership_map::DrawingOwnershipMap;
pub use element_pixel::ElementPixel;
pub use keyframe_player::{KeyframePaint, KeyframePlayer, KeyframePlayerOptions};
pub use plane_keyframe_player::{PlaneKeyframePaint, PlaneKeyframePlayer};
// The painterly planning and colour pieces (no C# equivalent).
pub use color_mix::{ColorInterpolation, ColorMix};
pub use color_rules::{BrokenColor, ColorRules, PaintPalette, PaletteMode};
pub use detail::{DetailMap, EdgeDetailOptions};
pub use direction_field::{DirectionField, NoiseFieldOptions, PictureFieldOptions};
pub use direction_guide::DirectionGuide;
pub use painting_timeline::{PaintingStage, PaintingTimeline, StrokeOrder};
pub use stroke_layout::{element_area, StrokeLayout, StrokeLayoutOptions, StrokeSize};
