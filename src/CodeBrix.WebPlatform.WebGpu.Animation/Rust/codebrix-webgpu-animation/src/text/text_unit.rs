//! `TextUnit`: the pieces a laid-out text is counted, revealed and split in (C# `TextUnit`).

/// The unit a laid-out text is counted, revealed and split into pieces by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextUnit {
    /// A shaped cluster: one character, or the characters a ligature or a base with its marks
    /// joined (the default).
    #[default]
    Character = 0,
    /// A word: a run of clusters that are not white space.
    Word = 1,
    /// A line.
    Line = 2,
}

