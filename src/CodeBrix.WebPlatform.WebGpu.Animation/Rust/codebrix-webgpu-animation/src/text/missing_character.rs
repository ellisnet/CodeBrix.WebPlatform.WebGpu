//! `MissingCharacter`: a character of a text that the font lacks (no C# counterpart: NoSkia has
//! no report of missing characters; the owner's rule adds one).

/// A character of a text that the font lacks, and where it is (a UTF-8 BYTE offset into the
/// text; C# would count UTF-16 units).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MissingCharacter {
    /// Where the character is in the text, a UTF-8 byte offset.
    pub text_index: usize,
    /// The character.
    pub character: char,
}
