//! `FontVariation` and `FontVariationAxis`: the axis values a variable font is registered at,
//! and the axes a registered font has (Not in C#: the desktop library registers the default
//! instance only).

use std::fmt;

/// One axis value of a VARIABLE font, given when the font is registered
/// ([`FontRegistry::register_font_with_variations`](crate::text::FontRegistry::register_font_with_variations)):
/// the axis's four-letter tag and the value in the axis's own units (user space - the numbers a
/// font's documentation and CSS `font-variation-settings` use: a weight of 700, a width of 87.5).
/// Not in C#.
///
/// The registered axes are `wght` (weight), `wdth` (width), `slnt` (slant), `ital` (italic) and
/// `opsz` (optical size); a font may define axes of its own (tags in capitals).
/// [`FontRegistry::get_variation_axes`](crate::text::FontRegistry::get_variation_axes) lists the
/// axes a font has, with their ranges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontVariation {
    /// The axis's tag: four ASCII characters, such as `*b"wght"`.
    pub tag: [u8; 4],
    /// The value on that axis, in the axis's own units (it must lie in the axis's range).
    pub value: f32,
}

impl FontVariation {
    /// An axis value: `FontVariation::new(*b"wdth", 87.5)`.
    pub const fn new(tag: [u8; 4], value: f32) -> FontVariation {
        FontVariation { tag, value }
    }

    /// A value on the weight axis (`wght`): 400 is regular, 700 bold.
    pub const fn weight(value: f32) -> FontVariation {
        FontVariation::new(*b"wght", value)
    }

    /// A value on the width axis (`wdth`), in percent of the normal width.
    pub const fn width(value: f32) -> FontVariation {
        FontVariation::new(*b"wdth", value)
    }

    /// The tag as text (`"wght"`); a byte that is not printable ASCII shows as `?`.
    pub fn tag_text(&self) -> String {
        tag_text(self.tag)
    }
}

/// One variation axis of a registered font, as its `fvar` table defines it: the tag, and the
/// smallest, default and largest value in the axis's own units. Not in C#.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontVariationAxis {
    /// The axis's tag, such as `*b"wght"`.
    pub tag: [u8; 4],
    /// The smallest value the axis takes.
    pub min_value: f32,
    /// The value of the font's default instance (what a font registered without this axis's
    /// value is drawn at).
    pub default_value: f32,
    /// The largest value the axis takes.
    pub max_value: f32,
    /// True when the font asks for the axis to be kept out of user interfaces.
    pub is_hidden: bool,
}

impl FontVariationAxis {
    /// The tag as text (`"wght"`); a byte that is not printable ASCII shows as `?`.
    pub fn tag_text(&self) -> String {
        tag_text(self.tag)
    }
}

impl fmt::Display for FontVariationAxis {
    /// `wght 100..900 (default 400)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}..{} (default {})", self.tag_text(), self.min_value, self.max_value, self.default_value)
    }
}

/// A tag's four bytes as text, `?` for a byte that is not printable ASCII.
pub(crate) fn tag_text(tag: [u8; 4]) -> String {
    tag.iter().map(|b| if b.is_ascii_graphic() || *b == b' ' { *b as char } else { '?' }).collect()
}
