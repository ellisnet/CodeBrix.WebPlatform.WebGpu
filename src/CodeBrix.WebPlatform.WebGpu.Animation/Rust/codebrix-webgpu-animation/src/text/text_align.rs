//! `TextAlign`: how a line of text sits about its anchor (C# `TextAlign`).

/// How a line of text sits about its anchor's x.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextAlign {
    /// The line starts at the anchor (the default).
    #[default]
    Left = 0,
    /// The line is centred on the anchor.
    Center = 1,
    /// The line ends at the anchor.
    Right = 2,
    /// Lines of a text block are spread to the block's width (the text-layout step; a single
    /// line - a `TextShape` - refuses it, as C# does).
    Justify = 3,
}
