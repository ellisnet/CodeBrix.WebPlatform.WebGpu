//! `TextStrokePlacement`: where an outlined text block's stroke lies (C# `TextStrokePlacement`).

/// Where the outline of a stroked [`TextBlockShape`](crate::shapes::TextBlockShape) lies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextStrokePlacement {
    /// Centred on the glyph outline, half inside and half outside (the default).
    #[default]
    Center = 0,
    /// Only the outside half shows: the glyphs' own shape is cut out of the stroke.
    Outside = 1,
}
