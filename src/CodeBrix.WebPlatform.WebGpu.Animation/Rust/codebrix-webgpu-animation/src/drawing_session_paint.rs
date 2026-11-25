//! `SessionPaint` and `LinePaint`: how a session is PAINTED at render time - as authored, as a
//! line drawing, or in flat colours - without changing the scene (Not in C#).

use crate::color::Color;
use crate::paths::PathStrokeStyle;

/// How a session's shapes are painted when it is rendered, set on
/// [`SessionRenderOptions::paint`](crate::SessionRenderOptions::paint). The scene is NOT
/// changed: the same session can be drawn differently each frame, or twice in one frame (one
/// side of a [`SessionWipe`](crate::SessionWipe) each). One look per render.
///
/// What each look does to each kind of drawing:
///
/// | drawn as authored             | `Line`                                         | `Flat`                                  |
/// |-------------------------------|------------------------------------------------|-----------------------------------------|
/// | a fill (shapes, paths, glyphs, SVG and Lottie fills, brushstrokes) | its outline in the line style; the fill left out, or the paper colour | one solid colour from its own fill |
/// | a stroke (outlines, lines, SVG and Lottie strokes) | the same path in the line style       | as authored                             |
/// | text                          | outlined letters (each glyph's outline)        | its fill's colour                       |
/// | a picture, a sprite           | left out                                       | its average colour over its rectangle   |
/// | noise                         | left out                                       | the average of its colour ramp          |
/// | an ownership map              | left out                                       | the average of its owners' colours, weighted by the pixels each owns |
/// | particles                     | left out                                       | as authored (already solid colours)     |
///
/// A gradient's representative colour is its average over its length (the stops' colours and
/// alphas, piecewise linear between them, held at the ends); a picture's is the average of its
/// pixels (alpha-weighted colour, mean alpha). The colours of masks - mattes, the "outside" of a
/// path, SVG masks and several-shape clips, a text block's outside-outline cut-out: anything
/// composited destination-in or -out - are coverage, not paint: they are never changed, so a
/// masked element keeps its shape in every look. A shape's opacity (and its fill's alpha) is
/// kept: a faded shape gives a faded line.
///
/// Not in C#.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum SessionPaint {
    /// Every shape as it was authored (the default).
    #[default]
    AsAuthored,
    /// A line drawing: every shape as an outline in one stroke style and colour.
    Line(LinePaint),
    /// Every shape one solid colour taken from its own fill; outlines as authored.
    Flat,
}

/// The line drawing of [`SessionPaint::Line`]: the stroke every shape is outlined with, its
/// colour, and what becomes of the fills. The stroke style may be
/// [fixed-width](crate::paths::StrokeWidthMode::Fixed) (the same line weight everywhere in the
/// drawing's units, whatever the elements' scaling) or scale with each element (the default of a
/// stroke style: a shape drawn twice as large gets a line twice as wide; for text the element
/// is the text block or line of text, so the width is in drawing units at the text's own scale).
///
/// Not in C#.
#[derive(Clone, Debug, PartialEq)]
pub struct LinePaint {
    /// The stroke every outline is drawn with (width, caps, joins, dashes, width mode).
    pub stroke: PathStrokeStyle,
    /// The colour of the lines (its alpha multiplies each shape's own alpha).
    pub color: Color,
    /// None (the default of [`LinePaint::new`]) to leave the fills out - a drawing of lines
    /// alone; Some(colour) to fill every filled shape with that one colour under its outline
    /// (paper white, for a drawing on paper where nearer shapes hide the lines behind them).
    pub fill: Option<Color>,
}

impl LinePaint {
    /// Lines in a stroke style and a colour, the fills left out.
    pub fn new(stroke: PathStrokeStyle, color: Color) -> LinePaint {
        LinePaint { stroke, color, fill: None }
    }

    /// The same lines with every fill replaced by one flat colour (paper white, for example).
    pub fn with_fill(mut self, fill: Color) -> LinePaint {
        self.fill = Some(fill);
        self
    }
}
