//! `ShapedCluster`: one cluster of a shaped line of text - the characters that became one or
//! more glyphs together (no C# counterpart in this step; NoSkia's fuller `TextCluster` belongs to
//! the text-layout step and will sit on this one).

/// One cluster of a shaped line: the characters that became glyphs together (one character, a
/// ligature's characters, a base with its marks). Positions in the text are UTF-8 BYTE offsets
/// into the string (C# counts UTF-16 units); lengths and positions along the line are in the
/// units of whatever owns the cluster (calibrated units for a [`TextShape`](crate::shapes::TextShape)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapedCluster {
    /// Where the cluster's characters start in the text, a UTF-8 byte offset.
    pub text_index: usize,
    /// How many bytes of the text the cluster covers.
    pub text_length: usize,
    /// Where the cluster starts along the line, from the line's start (tracking included).
    pub x: f32,
    /// The cluster's advance (tracking not included).
    pub advance: f32,
    /// True when every character of the cluster is white space (it draws nothing).
    pub is_white_space: bool,
    /// True when the cluster holds a character the font lacks (it drew the font's `.notdef`,
    /// a box, or nothing, by the [`MissingCharacterMode`](crate::text::MissingCharacterMode)).
    pub is_missing: bool,
}
