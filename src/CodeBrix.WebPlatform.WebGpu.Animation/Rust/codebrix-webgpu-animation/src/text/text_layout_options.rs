//! `TextLayoutOptions`: how a block of text is laid out (C# `TextLayoutOptions`), with the
//! missing-character choice this crate adds.

use crate::text::{MissingCharacterMode, TextAlign, TextTrimming, TextVerticalAlign};

/// How a block of text is laid out (C# `TextLayoutOptions`, the same members and defaults): the
/// width it wraps to, the height and line count it is cut to, alignment, line spacing, trimming
/// and kerning - and, by the owner's rule, what a character the font lacks shows (default
/// [`MissingCharacterMode::ShowTofu`]). Lengths are calibrated drawing units.
#[derive(Clone, Debug, PartialEq)]
pub struct TextLayoutOptions {
    /// The width lines wrap to; None (the default) for no wrapping (lines end only at line
    /// breaks). Positive and finite.
    pub max_width: Option<f32>,
    /// The block's height: lines that would pass it are cut off (the first line always stays),
    /// and [`vertical_align`](TextLayoutOptions::vertical_align) places the lines in it; None (the
    /// default) for the lines' own height. Positive and finite.
    pub max_height: Option<f32>,
    /// The most lines kept; 0 (the default) for no limit.
    pub max_lines: usize,
    /// How each line sits in the block's width (default Left; Justify spreads every line but a
    /// paragraph's last over the width).
    pub align: TextAlign,
    /// Where the lines sit in [`max_height`](TextLayoutOptions::max_height) (default Top).
    pub vertical_align: TextVerticalAlign,
    /// A multiple of the font's natural line height (ascent + descent + line gap) (default 1).
    pub line_spacing: f32,
    /// An explicit line height in units, used instead of `line_spacing` when above 0 (default 0).
    pub line_height: f32,
    /// What the last kept line shows when lines are cut off (default None).
    pub trimming: TextTrimming,
    /// The text that marks a cut (default "…", U+2026); empty for none.
    pub ellipsis: String,
    /// True (the default) to apply the fonts' kerning.
    pub kerning: bool,
    /// What a character the font lacks shows (default ShowTofu: always a visible box). No C#
    /// counterpart: the owner's rule.
    pub missing_characters: MissingCharacterMode,
}

impl Default for TextLayoutOptions {
    fn default() -> TextLayoutOptions {
        TextLayoutOptions {
            max_width: None,
            max_height: None,
            max_lines: 0,
            align: TextAlign::Left,
            vertical_align: TextVerticalAlign::Top,
            line_spacing: 1.0,
            line_height: 0.0,
            trimming: TextTrimming::None,
            ellipsis: "\u{2026}".to_string(),
            kerning: true,
            missing_characters: MissingCharacterMode::ShowTofu,
        }
    }
}

impl TextLayoutOptions {
    /// Rejects values no layout can honour, naming the member at fault (C# `Validate`, whose
    /// ArgumentOutOfRangeException this error replaces).
    pub(crate) fn validate(&self) -> Result<(), String> {
        if let Some(width) = self.max_width {
            if !(width > 0.0) || !width.is_finite() {
                return Err(format!("max_width must be positive and finite, or None for no wrapping (it was {width})."));
            }
        }
        if let Some(height) = self.max_height {
            if !(height > 0.0) || !height.is_finite() {
                return Err(format!("max_height must be positive and finite, or None for the lines' own height (it was {height})."));
            }
        }
        if !(self.line_spacing > 0.0) || !self.line_spacing.is_finite() {
            return Err(format!("line_spacing must be a positive, finite multiple of the natural line height (it was {}).", self.line_spacing));
        }
        if !(self.line_height >= 0.0) || !self.line_height.is_finite() {
            return Err(format!("line_height must be 0 (use line_spacing) or a positive, finite height in units (it was {}).", self.line_height));
        }
        Ok(())
    }
}
