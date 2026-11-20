//! `TextLineMetrics`: one laid-out line of a text block (C# `TextLineMetrics`).

use std::fmt;

use crate::geometry::RectangleF;

/// One line of a laid-out text block (C# `TextLineMetrics`). Positions are calibrated units
/// from the block's top-left; text positions are UTF-8 BYTE offsets into the block's text (C#
/// counts UTF-16 units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLineMetrics {
    pub(crate) index: usize,
    pub(crate) text_index: usize,
    pub(crate) text_length: usize,
    pub(crate) top: f32,
    pub(crate) height: f32,
    pub(crate) baseline_offset: f32,
    pub(crate) left: f32,
    pub(crate) width: f32,
    pub(crate) ascent: f32,
    pub(crate) descent: f32,
    pub(crate) first_cluster: usize,
    pub(crate) cluster_count: usize,
    pub(crate) ends_paragraph: bool,
}

impl TextLineMetrics {
    /// The line's number, from 0.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Where the line's text starts, a UTF-8 byte offset (a wrapped line starts at its first
    /// cluster; a paragraph's first line at the paragraph's start).
    pub fn text_index(&self) -> usize {
        self.text_index
    }

    /// How many bytes of the text the line covers, its line break and hanging spaces included.
    pub fn text_length(&self) -> usize {
        self.text_length
    }

    /// The line's top.
    pub fn top(&self) -> f32 {
        self.top
    }

    /// The line's height (the explicit line height, or the tallest run's natural height times
    /// the line spacing).
    pub fn height(&self) -> f32 {
        self.height
    }

    /// From the line's top to its baseline.
    pub fn baseline_offset(&self) -> f32 {
        self.baseline_offset
    }

    /// The baseline's y (C# `Baseline`).
    pub fn baseline(&self) -> f32 {
        self.top + self.baseline_offset
    }

    /// Where the line's first cluster starts after alignment.
    pub fn left(&self) -> f32 {
        self.left
    }

    /// From the first cluster's start to the last visible cluster's end (hanging spaces not
    /// counted).
    pub fn width(&self) -> f32 {
        self.width
    }

    /// The tallest ascent of the line's runs.
    pub fn ascent(&self) -> f32 {
        self.ascent
    }

    /// The deepest descent of the line's runs (positive, below the baseline).
    pub fn descent(&self) -> f32 {
        self.descent
    }

    /// The index of the line's first cluster in the block.
    pub fn first_cluster(&self) -> usize {
        self.first_cluster
    }

    /// How many clusters the line holds.
    pub fn cluster_count(&self) -> usize {
        self.cluster_count
    }

    /// True when the line ends its paragraph (a line break or the text's end follows; also the
    /// last line kept by a cut).
    pub fn ends_paragraph(&self) -> bool {
        self.ends_paragraph
    }

    /// The line's box: left, top, width, height (C# `Bounds`).
    pub fn bounds(&self) -> RectangleF {
        RectangleF::new(self.left, self.top, self.width, self.height)
    }
}

impl fmt::Display for TextLineMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: text [{}, {}), top {}, height {}, baseline {}, left {}, width {}", self.index, self.text_index, self.text_index + self.text_length, self.top, self.height, self.baseline(), self.left, self.width)
    }
}
