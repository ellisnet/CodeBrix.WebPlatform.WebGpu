//! `CaptionStyle`: how captions look and where they go (C# `CaptionStyle`), with the frame margin
//! and the missing-character choice this crate adds.

use crate::color::Color;
use crate::fills::ShapeFill;
use crate::layout::{LayoutAnchor, LayoutPosition, LayoutPositionOptions, LayoutReference};
use crate::text::{MissingCharacterMode, TextAlign};

/// How captions look and where they go (C# `CaptionStyle`, the same members and defaults): white
/// text 4.5% of the frame's short side, centred, wrapped to 86% of the caption area's width, on a
/// translucent black rounded box, at the bottom centre of the CAPTION AREA. Lengths given as
/// fractions are fractions of the text size.
///
/// THE CAPTION AREA (the owner's rule - a deliberate difference from C#, where captions sit in
/// the layout's safe area): a position measured against [`LayoutReference::SafeArea`] is measured
/// against the caption area, which is the FRAME kept [`margin`](CaptionStyle::margin) from its
/// edges by default, or a safe-area guide when the presenter opts in
/// ([`CaptionPlacement::SafeAreaGuide`](crate::captions::CaptionPlacement::SafeAreaGuide); the
/// margin is then not used). A position measured against [`LayoutReference::Frame`] (WebVTT's
/// `line:` setting) is the frame itself, as C#.
#[derive(Clone, Debug, PartialEq)]
pub struct CaptionStyle {
    /// The family; None (the default) for the registry's first font.
    pub family_name: Option<String>,
    /// The text's em size in calibrated units; 0 (the default) for 4.5% of the layout's short
    /// side.
    pub size: f32,
    /// The text's colour (default white).
    pub text_color: Color,
    /// A fill for the text instead of the colour (default none).
    pub text_fill: Option<ShapeFill>,
    /// The colour of an outline around the glyphs (default none: no outline).
    pub outline_color: Option<Color>,
    /// The outline's width beyond the glyphs as a fraction of the size (default 0.08; the stroke
    /// is twice this, drawn under the fill).
    pub outline_width: f32,
    /// The box's colour; None for no box (default black at alpha 176).
    pub background_color: Option<Color>,
    /// The space between the text and the box's left and right edges (default 0.45).
    pub padding_x: f32,
    /// The space between the text and the box's top and bottom edges (default 0.22).
    pub padding_y: f32,
    /// The box's corner radius (default 0.25).
    pub corner_radius: f32,
    /// The widest a box may be, as a fraction of its reference rectangle's width (default 0.86).
    pub max_width_fraction: f32,
    /// How the lines sit in the box (default Center).
    pub align: TextAlign,
    /// The line spacing, a multiple of the natural line height (default 1.1).
    pub line_spacing: f32,
    /// The most lines a cue shows, the last ending in an ellipsis after a whole word; 0 (the
    /// default) for no limit.
    pub max_lines: usize,
    /// Where a cue's box goes - its own anchor lands on the position (default the bottom centre of
    /// the caption area).
    pub position: LayoutPosition,
    /// The gap between the boxes of cues showing at the same time (default 0.3).
    pub cue_spacing: f32,
    /// How far the caption area is kept from the frame's edges, as a fraction of the size
    /// (default 1: one text size). No C# counterpart: the owner's rule that captions sit against
    /// the frame by default. Not used with a safe-area guide.
    pub margin: f32,
    /// What a character the font lacks shows (default ShowTofu). No C# counterpart.
    pub missing_characters: MissingCharacterMode,
}

impl Default for CaptionStyle {
    fn default() -> CaptionStyle {
        CaptionStyle {
            family_name: None,
            size: 0.0,
            text_color: Color::WHITE,
            text_fill: None,
            outline_color: None,
            outline_width: 0.08,
            background_color: Some(Color::from_rgba(0, 0, 0, 176)),
            padding_x: 0.45,
            padding_y: 0.22,
            corner_radius: 0.25,
            max_width_fraction: 0.86,
            align: TextAlign::Center,
            line_spacing: 1.1,
            max_lines: 0,
            position: LayoutPosition::new(LayoutAnchor::BOTTOM_CENTER, LayoutPositionOptions { reference: LayoutReference::SafeArea, ..LayoutPositionOptions::default() }).unwrap_or_else(|_| unreachable!()),
            cue_spacing: 0.3,
            margin: 1.0,
            missing_characters: MissingCharacterMode::ShowTofu,
        }
    }
}

impl CaptionStyle {
    /// Rejects values no caption can be drawn with, naming the member at fault (C# `Validate`).
    pub(crate) fn validate(&self) -> Result<(), String> {
        let non_negative = [("size", self.size), ("outline_width", self.outline_width), ("padding_x", self.padding_x), ("padding_y", self.padding_y), ("corner_radius", self.corner_radius), ("cue_spacing", self.cue_spacing), ("margin", self.margin)];
        for (name, value) in non_negative {
            if !(value >= 0.0) || !value.is_finite() {
                return Err(format!("A caption style's {name} must be 0 or a positive, finite number (it was {value})."));
            }
        }
        for (name, value) in [("max_width_fraction", self.max_width_fraction), ("line_spacing", self.line_spacing)] {
            if !(value > 0.0) || !value.is_finite() {
                return Err(format!("A caption style's {name} must be a positive, finite number (it was {value})."));
            }
        }
        Ok(())
    }
}
