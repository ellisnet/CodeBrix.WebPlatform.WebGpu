//! Text, single lines and laid-out blocks (CodeBrix.Imaging.Drawing's `Text` namespace): the
//! [`FontRegistry`] that takes font BYTES (a variable font at chosen axis values:
//! [`FontVariation`]), [`TextAlign`], and what this crate adds
//! by the owner's rule - the choice of what a missing character shows
//! ([`MissingCharacterMode`]) and the report of what was missing ([`MissingCharacter`],
//! [`ShapedCluster`]). The line itself is a [`TextShape`](crate::shapes::TextShape).
//! A block of text - wrapped, aligned, trimmed, in styled runs - is laid out by
//! [`TextLayoutEngine`] with NoSkia's own line breaker into a shared [`TextLayoutResult`], and
//! drawn as a [`TextBlockShape`](crate::shapes::TextBlockShape).
//!
//! Shaping is harfrust's (kerning, ligatures, contextual forms); glyph outlines come from its
//! own font reader, unhinted, and are kept per font face. A line is shaped as ONE run in the
//! direction of its first strong character: left-to-right text as NoSkia, and a line written
//! entirely right to left (Arabic, Hebrew) comes out in the right order too; a line that MIXES
//! directions is not reordered (there is no Unicode bidirectional algorithm). Text positions
//! are UTF-8 byte offsets.

pub(crate) mod font_face;
mod font_registry;
mod font_variation;
mod missing_character;
mod missing_character_mode;
mod shaped_cluster;
mod text_align;
pub(crate) mod text_cluster;
mod text_layout_engine;
mod text_layout_options;
pub(crate) mod text_layout_result;
mod text_line_metrics;
mod text_piece;
mod text_run_descriptor;
mod text_stroke_placement;
mod text_trimming;
mod text_unit;
mod text_vertical_align;
pub(crate) mod text_shaping;

pub use font_registry::{FontRegistry, MeasureTextOptions};
pub use font_variation::{FontVariation, FontVariationAxis};
pub use missing_character::MissingCharacter;
pub use missing_character_mode::MissingCharacterMode;
pub use shaped_cluster::ShapedCluster;
pub use text_align::TextAlign;
pub use text_cluster::TextCluster;
pub use text_layout_engine::TextLayoutEngine;
pub use text_layout_options::TextLayoutOptions;
pub use text_layout_result::TextLayoutResult;
pub use text_line_metrics::TextLineMetrics;
pub use text_piece::TextPiece;
pub use text_run_descriptor::TextRunDescriptor;
pub use text_stroke_placement::TextStrokePlacement;
pub use text_trimming::TextTrimming;
pub use text_unit::TextUnit;
pub use text_vertical_align::TextVerticalAlign;

pub(crate) use font_registry::check_size_and_tracking;

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod layout_tests;
