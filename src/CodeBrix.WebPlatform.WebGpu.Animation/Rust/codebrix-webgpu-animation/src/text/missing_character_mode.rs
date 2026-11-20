//! `MissingCharacterMode`: what is drawn for a character the font lacks (no C# counterpart:
//! NoSkia always draws the font's `.notdef` glyph; the owner's rule adds the choice).

/// What is drawn for a character the font lacks. There is never a fallback font and never a
/// system font: the text keeps the font it was given, and the character gets an advance either
/// way (the `.notdef` glyph's own, else the space's, else half an em), so the line never
/// collapses. Whatever the mode, the shape REPORTS the missing characters
/// ([`TextShape::missing_characters`](crate::shapes::TextShape::missing_characters)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MissingCharacterMode {
    /// A visible box, always (the default): the font's `.notdef` glyph when it has ink (Roboto's
    /// is a box), otherwise a hollow box this crate draws inside that glyph's advance (Open
    /// Sans's `.notdef` has no ink). You see that a character is missing.
    #[default]
    ShowTofu = 0,
    /// Whatever the font's own `.notdef` glyph is, ink or not (what NoSkia draws).
    FontDefault = 1,
    /// Nothing visible: a blank advance, like a space.
    Blank = 2,
}
