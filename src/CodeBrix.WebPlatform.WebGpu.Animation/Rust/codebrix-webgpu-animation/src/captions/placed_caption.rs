//! `PlacedCaption`: one cue as it shows at a time - its box and its text, placed in the frame (no
//! C# public counterpart: C#'s presenter returns these as shapes in a group).

use std::rc::Rc;

use crate::color::Color;
use crate::fills::ShapeFill;
use crate::geometry::RectangleF;
use crate::paths::{PathStrokeStyle, PathStrokeStyleOptions};
use crate::shapes::{TextBlockShape, TextBlockStrokeOptions};
use crate::text::TextLayoutResult;

/// One cue as it shows (from [`CaptionPresenter::get_placed_captions_at`](crate::captions::CaptionPresenter::get_placed_captions_at)),
/// in calibrated units of the frame: what C#'s presenter turns into a rounded rectangle and one or
/// two text blocks, drawn in this order - the box, the outline, the text.
#[derive(Clone, Debug)]
pub struct PlacedCaption {
    /// The cue's index in the track.
    pub cue_index: usize,
    /// The box: the text's lines plus the padding, placed in the frame.
    pub box_bounds: RectangleF,
    /// The box's colour; None for no box (C# draws none, or none when the box is empty).
    pub background_color: Option<Color>,
    /// The box's corner radius, units.
    pub corner_radius: f32,
    /// The cue's laid-out text (kept per cue: the same `Rc` every time it shows).
    pub text: Rc<TextLayoutResult>,
    /// Where the text block's top-left goes (the box's corner plus the padding, less the
    /// content's own offset in its layout).
    pub text_x: f32,
    /// See `text_x`.
    pub text_y: f32,
    /// The text's colour.
    pub text_color: Color,
    /// The text's fill instead of the colour, when the style has one.
    pub text_fill: Option<ShapeFill>,
    /// The outline under the text: its colour and its stroke width (twice the style's outline
    /// width times the size); None for no outline.
    pub outline: Option<(Color, f32)>,
}

impl PlacedCaption {
    /// True when the box is drawn (a colour and a box with area).
    pub fn has_box(&self) -> bool {
        self.background_color.is_some() && self.box_bounds.width > 0.0 && self.box_bounds.height > 0.0
    }

    /// The text blocks C#'s presenter draws for the cue, in drawing order: the outline (stroked,
    /// under the fill, so it shows only outside the glyphs) when there is one, then the text
    /// painted in its fill or colour.
    pub fn create_text_shapes(&self) -> Result<Vec<TextBlockShape>, String> {
        let mut shapes = Vec::with_capacity(2);
        if let Some((color, width)) = self.outline {
            let style = PathStrokeStyle::new(width, PathStrokeStyleOptions::default())?;
            shapes.push(TextBlockShape::with_stroke(self.text.clone(), self.text_x, self.text_y, Some(style), TextBlockStrokeOptions { stroke_color: Some(color), ..TextBlockStrokeOptions::default() })?);
        }
        shapes.push(match &self.text_fill {
            Some(fill) => TextBlockShape::with_fill(self.text.clone(), self.text_x, self.text_y, fill.clone())?,
            None => TextBlockShape::new(self.text.clone(), self.text_x, self.text_y, Some(self.text_color))?,
        });
        Ok(shapes)
    }
}
