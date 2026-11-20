//! `TextVerticalAlign`: where a block's lines sit in its height (C# `TextVerticalAlign`).

/// Where a text block's lines sit when the block has a height of its own
/// ([`TextLayoutOptions::max_height`](crate::text::TextLayoutOptions::max_height)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextVerticalAlign {
    /// The lines start at the top (the default).
    #[default]
    Top = 0,
    /// The lines are centred in the height.
    Center = 1,
    /// The lines end at the bottom.
    Bottom = 2,
}
