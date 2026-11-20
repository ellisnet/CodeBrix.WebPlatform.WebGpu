//! `TextCluster`: one shaped cluster of a laid-out text block (C# `TextCluster`).

use std::fmt;
use std::rc::Rc;

use kurbo::{Affine, BezPath};

use crate::geometry::{PointF, RectangleF};
use crate::paths::{PathFillRule, PathGeometry};
use crate::text::font_face::FontFace;

/// One glyph a cluster draws: its outline key in the face (a glyph id, or the crate's tofu box)
/// and its origin on the baseline, calibrated units from the block's top-left (y down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ClusterGlyph {
    pub(crate) key: u32,
    pub(crate) x: f32,
    pub(crate) y: f32,
}

/// One cluster of a laid-out text block - the characters that shaped into glyphs together: one
/// character, a ligature's characters, a base with its marks (C# `TextCluster`). Every later
/// step - reveals, pieces, staggers, text on a path - works in clusters.
///
/// Positions are calibrated units from the block's top-left; text positions are UTF-8 BYTE
/// offsets into the block's text (C# counts UTF-16 units). ONE DIFFERENCE FROM C#: a cluster
/// keeps the glyphs the PARAGRAPH's shaping gave it (so contextual forms stay) and draws them
/// from its face's outline cache, where C# re-shapes the cluster's characters alone and stores
/// the outline's points.
#[derive(Clone)]
pub struct TextCluster {
    pub(crate) index: usize,
    pub(crate) text_index: usize,
    pub(crate) text_length: usize,
    pub(crate) text: String,
    pub(crate) line_index: usize,
    pub(crate) word_index: Option<usize>,
    pub(crate) run_index: usize,
    pub(crate) is_white_space: bool,
    pub(crate) is_ellipsis: bool,
    pub(crate) is_missing: bool,
    pub(crate) origin: PointF,
    pub(crate) advance: f32,
    pub(crate) bounds: RectangleF,
    pub(crate) ink_bounds: Option<RectangleF>,
    pub(crate) glyphs: Vec<ClusterGlyph>,
    pub(crate) face: Rc<FontFace>,
    pub(crate) scale: f32,
}

impl fmt::Debug for TextCluster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextCluster")
            .field("index", &self.index)
            .field("text", &self.text)
            .field("text_index", &self.text_index)
            .field("text_length", &self.text_length)
            .field("line_index", &self.line_index)
            .field("word_index", &self.word_index)
            .field("origin", &self.origin)
            .field("advance", &self.advance)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for TextCluster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cluster {} \"{}\" (text {}+{}, line {}, word {}) at ({}, {}), advance {}", self.index, self.text, self.text_index, self.text_length, self.line_index, self.word_index.map_or(-1, |w| w as i64), self.origin.x, self.origin.y, self.advance)
    }
}

impl TextCluster {
    /// The cluster's number in the block, from 0.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Where its characters start in the block's text, a UTF-8 byte offset (an ellipsis: where
    /// the cut text ends).
    pub fn text_index(&self) -> usize {
        self.text_index
    }

    /// How many bytes of the block's text it covers (0 for an ellipsis).
    pub fn text_length(&self) -> usize {
        self.text_length
    }

    /// Its characters (an ellipsis: the ellipsis text).
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The line it is on.
    pub fn line_index(&self) -> usize {
        self.line_index
    }

    /// The word it belongs to, counted through the whole block; None for white space (C# -1).
    pub fn word_index(&self) -> Option<usize> {
        self.word_index
    }

    /// The run it comes from.
    pub fn run_index(&self) -> usize {
        self.run_index
    }

    /// True when every character of it is white space (it draws nothing).
    pub fn is_white_space(&self) -> bool {
        self.is_white_space
    }

    /// True for the ellipsis a cut added.
    pub fn is_ellipsis(&self) -> bool {
        self.is_ellipsis
    }

    /// True when it holds a character the font lacks (no C# counterpart; the owner's rule).
    pub fn is_missing(&self) -> bool {
        self.is_missing
    }

    /// The pen's start, on the baseline.
    pub fn origin(&self) -> PointF {
        self.origin
    }

    /// The advance (justification's stretch included for a space), tracking not included.
    pub fn advance(&self) -> f32 {
        self.advance
    }

    /// Its cell: from the origin, its advance wide and its line's height tall.
    pub fn bounds(&self) -> RectangleF {
        self.bounds
    }

    /// The box around its glyph outlines (control points included); None when it has no ink.
    pub fn ink_bounds(&self) -> Option<RectangleF> {
        self.ink_bounds
    }

    /// True when it draws something (C# `HasInk`): a missing character in
    /// [`MissingCharacterMode::Blank`](crate::text::MissingCharacterMode::Blank), a space, or a
    /// glyph with no outline draws nothing.
    pub fn has_ink(&self) -> bool {
        !self.glyphs.is_empty()
    }

    /// The transform of one of its glyphs' font-unit outline into block units.
    pub(crate) fn glyph_transform(&self, glyph: &ClusterGlyph) -> Affine {
        let s = self.scale as f64;
        Affine::translate((glyph.x as f64, glyph.y as f64)) * Affine::scale_non_uniform(s, -s)
    }

    /// Appends its outline, moved by (dx, dy), to a path.
    pub(crate) fn append_outline(&self, path: &mut BezPath, dx: f32, dy: f32) {
        let shift = Affine::translate((dx as f64, dy as f64));
        for glyph in &self.glyphs {
            let outline = self.face.outline(glyph.key);
            let t = shift * self.glyph_transform(glyph);
            for el in &outline.elements {
                path.push(t * *el);
            }
        }
    }

    /// Its glyph outlines at their place in the block, with the non-zero rule; empty when it has
    /// no ink (C# `GetOutline`).
    pub fn get_outline(&self) -> PathGeometry {
        let mut path = BezPath::new();
        self.append_outline(&mut path, 0.0, 0.0);
        PathGeometry::from_bez_path(path, PathFillRule::NonZero)
    }
}
