//! `TextPiece`: a character, word or line of a laid-out text block (C# `TextPiece`).

use std::fmt;
use std::rc::Rc;

use crate::geometry::{PointF, RectangleF};
use crate::paths::PathGeometry;
use crate::text::{TextLayoutResult, TextUnit};

/// A character (cluster), word or line of a laid-out text block, as a range of its clusters
/// (C# `TextPiece`; from [`TextLayoutResult::get_pieces`]). Text positions are UTF-8 byte
/// offsets.
#[derive(Clone)]
pub struct TextPiece {
    pub(crate) layout: Rc<TextLayoutResult>,
    pub(crate) unit: TextUnit,
    pub(crate) index: usize,
    pub(crate) first_cluster: usize,
    pub(crate) cluster_count: usize,
    pub(crate) text_index: usize,
    pub(crate) text_length: usize,
    pub(crate) text: String,
    pub(crate) line_index: usize,
    pub(crate) bounds: RectangleF,
    pub(crate) ink_bounds: Option<RectangleF>,
    pub(crate) is_white_space: bool,
}

impl fmt::Debug for TextPiece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl fmt::Display for TextPiece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = self.bounds;
        write!(f, "{:?} {} \"{}\" (clusters {}+{}, line {}) {{X={},Y={},Width={},Height={}}}", self.unit, self.index, self.text, self.first_cluster, self.cluster_count, self.line_index, b.x, b.y, b.width, b.height)
    }
}

impl TextPiece {
    /// The kind of piece.
    pub fn unit(&self) -> TextUnit {
        self.unit
    }

    /// Its number among the pieces of its kind.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Its first cluster.
    pub fn first_cluster(&self) -> usize {
        self.first_cluster
    }

    /// How many clusters it holds.
    pub fn cluster_count(&self) -> usize {
        self.cluster_count
    }

    /// Where its text starts, a UTF-8 byte offset.
    pub fn text_index(&self) -> usize {
        self.text_index
    }

    /// How many bytes of the text it covers.
    pub fn text_length(&self) -> usize {
        self.text_length
    }

    /// Its characters (an ellipsis included).
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The line it is on (a word's first line).
    pub fn line_index(&self) -> usize {
        self.line_index
    }

    /// Its box: a character's cell, a word's cells together, a line's box.
    pub fn bounds(&self) -> RectangleF {
        self.bounds
    }

    /// The box around its glyph outlines; None when it has no ink.
    pub fn ink_bounds(&self) -> Option<RectangleF> {
        self.ink_bounds
    }

    /// The centre of its box (C# `Center`).
    pub fn center(&self) -> PointF {
        PointF::new(self.bounds.x + (self.bounds.width / 2.0), self.bounds.y + (self.bounds.height / 2.0))
    }

    /// True for a character piece that is white space (words and lines: never).
    pub fn is_white_space(&self) -> bool {
        self.is_white_space
    }

    /// Its glyph outlines at their place in the block (C# `GetOutline`).
    pub fn get_outline(&self) -> PathGeometry {
        self.layout.build_outline(self.first_cluster, self.cluster_count, 0.0, 0.0)
    }
}
