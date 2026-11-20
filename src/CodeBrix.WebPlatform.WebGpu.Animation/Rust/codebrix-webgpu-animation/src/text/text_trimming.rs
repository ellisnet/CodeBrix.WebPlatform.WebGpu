//! `TextTrimming`: how a block that does not fit ends (C# `TextTrimming`).

/// What a text block shows when its lines are cut off by
/// [`max_lines`](crate::text::TextLayoutOptions::max_lines) or
/// [`max_height`](crate::text::TextLayoutOptions::max_height).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextTrimming {
    /// The last line kept simply ends (the default).
    #[default]
    None = 0,
    /// The ellipsis follows the last character that fits with it.
    CharacterEllipsis = 1,
    /// The ellipsis follows the last whole word that fits with it (characters when no word fits).
    WordEllipsis = 2,
}
