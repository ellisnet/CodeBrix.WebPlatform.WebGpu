//! `TextLayoutResult`: a laid-out block of text - its lines, clusters, pieces and outlines (C#
//! `TextLayoutResult`), and the arithmetic of placing its clusters along a path.

use std::fmt;
use std::rc::Rc;

use kurbo::{Affine, BezPath, PathEl};

use crate::geometry::RectangleF;
use crate::paths::{PathFillRule, PathGeometry};
use crate::text::{MissingCharacter, TextCluster, TextLayoutOptions, TextLineMetrics, TextPiece, TextRunDescriptor, TextUnit};

/// A laid-out block of text (C# `TextLayoutResult`, from
/// [`TextLayoutEngine`](crate::text::TextLayoutEngine)): immutable, shared as an `Rc` by every
/// text block shape and piece drawn from it. Positions are calibrated units from the block's
/// top-left; text positions are UTF-8 BYTE offsets into [`text`](TextLayoutResult::text) (C#
/// counts UTF-16 units).
pub struct TextLayoutResult {
    pub(crate) text: String,
    pub(crate) runs: Vec<TextRunDescriptor>,
    pub(crate) options: TextLayoutOptions,
    pub(crate) size: (f32, f32),
    pub(crate) lines: Vec<TextLineMetrics>,
    pub(crate) clusters: Vec<TextCluster>,
    pub(crate) word_count: usize,
    pub(crate) is_truncated: bool,
    pub(crate) shown_text_length: usize,
    pub(crate) content_bounds: RectangleF,
    pub(crate) ink_bounds: Option<RectangleF>,
    pub(crate) missing: Vec<MissingCharacter>,
}

impl fmt::Debug for TextLayoutResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextLayoutResult").field("text", &self.text).field("size", &self.size).field("lines", &self.lines.len()).field("clusters", &self.clusters.len()).finish_non_exhaustive()
    }
}

/// The union of two optional boxes (C# `TextLayoutResult.Union`).
pub(crate) fn union(a: Option<RectangleF>, b: Option<RectangleF>) -> Option<RectangleF> {
    match (a, b) {
        (None, b) => b,
        (a, None) => a,
        (Some(x), Some(y)) => {
            let (left, top) = (x.x.min(y.x), x.y.min(y.y));
            let (right, bottom) = (x.right().max(y.right()), x.bottom().max(y.bottom()));
            Some(RectangleF::new(left, top, right - left, bottom - top))
        }
    }
}

impl TextLayoutResult {
    /// Builds the result and its two boxes (C#'s constructor).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(text: String, runs: Vec<TextRunDescriptor>, options: TextLayoutOptions, size: (f32, f32), lines: Vec<TextLineMetrics>, clusters: Vec<TextCluster>, word_count: usize, is_truncated: bool, shown_text_length: usize, missing: Vec<MissingCharacter>) -> TextLayoutResult {
        let (mut left, mut top, mut right, mut bottom) = (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for line in &lines {
            left = left.min(line.left);
            right = right.max(line.left + line.width);
            top = top.min(line.top);
            bottom = bottom.max(line.top + line.height);
        }
        let content_bounds = if lines.is_empty() { RectangleF::default() } else { RectangleF::new(left, top, right - left, bottom - top) };
        let ink_bounds = clusters.iter().fold(None, |ink, c| union(ink, c.ink_bounds));
        TextLayoutResult { text, runs, options, size, lines, clusters, word_count, is_truncated, shown_text_length, content_bounds, ink_bounds, missing }
    }

    /// The whole text of the block (the runs' texts joined).
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The runs, as given.
    pub fn runs(&self) -> &[TextRunDescriptor] {
        &self.runs
    }

    /// The options it was laid out with (a copy, as C#'s `Options`).
    pub fn options(&self) -> TextLayoutOptions {
        self.options.clone()
    }

    /// The block's size (width, height): the wrap width (or the widest line) and the given
    /// height (or the lines' own) (C# `Size`).
    pub fn size(&self) -> (f32, f32) {
        self.size
    }

    /// The block's width.
    pub fn width(&self) -> f32 {
        self.size.0
    }

    /// The block's height.
    pub fn height(&self) -> f32 {
        self.size.1
    }

    /// The box around the lines' boxes (an empty rectangle when there is no line).
    pub fn content_bounds(&self) -> RectangleF {
        self.content_bounds
    }

    /// The box around every glyph outline; None when nothing has ink.
    pub fn ink_bounds(&self) -> Option<RectangleF> {
        self.ink_bounds
    }

    /// The lines.
    pub fn lines(&self) -> &[TextLineMetrics] {
        &self.lines
    }

    /// The number of lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// The clusters, in text order.
    pub fn clusters(&self) -> &[TextCluster] {
        &self.clusters
    }

    /// The number of clusters.
    pub fn cluster_count(&self) -> usize {
        self.clusters.len()
    }

    /// The number of words.
    pub fn word_count(&self) -> usize {
        self.word_count
    }

    /// True when lines were cut off by `max_lines` or `max_height`.
    pub fn is_truncated(&self) -> bool {
        self.is_truncated
    }

    /// True when the block ends with an ellipsis.
    pub fn has_ellipsis(&self) -> bool {
        self.clusters.last().is_some_and(|c| c.is_ellipsis)
    }

    /// How much of the text is shown: a UTF-8 byte offset - the text's length when nothing was
    /// cut, else where the shown text ends.
    pub fn shown_text_length(&self) -> usize {
        self.shown_text_length
    }

    /// The characters of the shown text (and of an ellipsis, at the cut's offset) that the fonts
    /// lack, in text order (no C# counterpart; the owner's rule).
    pub fn missing_characters(&self) -> &[MissingCharacter] {
        &self.missing
    }

    /// True when a font lacks a character of the shown text.
    pub fn has_missing_characters(&self) -> bool {
        !self.missing.is_empty()
    }

    /// One line (C# `GetLineMetrics`); None for an index past the last line (C# throws).
    pub fn get_line_metrics(&self, line_index: usize) -> Option<&TextLineMetrics> {
        self.lines.get(line_index)
    }

    /// How many units of a kind the block has (C# `GetUnitCount`).
    pub fn get_unit_count(&self, unit: TextUnit) -> usize {
        match unit {
            TextUnit::Word => self.word_count,
            TextUnit::Line => self.lines.len(),
            TextUnit::Character => self.clusters.len(),
        }
    }

    /// The block's characters, words or lines as pieces (C# `GetPieces`): every cluster is a
    /// character piece (white space too); words are the runs of clusters with the same word
    /// number; every line is a line piece.
    pub fn get_pieces(self: &Rc<Self>, unit: TextUnit) -> Vec<TextPiece> {
        let mut pieces = Vec::new();
        match unit {
            TextUnit::Character => {
                for c in &self.clusters {
                    pieces.push(TextPiece {
                        layout: self.clone(),
                        unit,
                        index: pieces.len(),
                        first_cluster: c.index,
                        cluster_count: 1,
                        text_index: c.text_index,
                        text_length: c.text_length,
                        text: c.text.clone(),
                        line_index: c.line_index,
                        bounds: c.bounds,
                        ink_bounds: c.ink_bounds,
                        is_white_space: c.is_white_space,
                    });
                }
            }
            TextUnit::Word => {
                let mut i = 0;
                while i < self.clusters.len() {
                    let Some(word) = self.clusters[i].word_index else {
                        i += 1;
                        continue;
                    };
                    let first = i;
                    while i < self.clusters.len() && self.clusters[i].word_index == Some(word) {
                        i += 1;
                    }
                    let index = pieces.len();
                    pieces.push(self.group(unit, index, first, i - first, self.clusters[first].line_index, None));
                }
            }
            TextUnit::Line => {
                for line in &self.lines {
                    let index = pieces.len();
                    pieces.push(self.group(unit, index, line.first_cluster, line.cluster_count, line.index, Some(line)));
                }
            }
        }
        pieces
    }

    fn group(self: &Rc<Self>, unit: TextUnit, index: usize, first: usize, count: usize, line_index: usize, line: Option<&TextLineMetrics>) -> TextPiece {
        let mut text = String::new();
        let (mut cells, mut ink) = (None, None);
        let text_start = line.map_or(if count > 0 { self.clusters[first].text_index } else { 0 }, |l| l.text_index);
        let mut text_end = text_start;
        for c in &self.clusters[first..first + count] {
            text.push_str(&c.text);
            cells = union(cells, Some(c.bounds));
            ink = union(ink, c.ink_bounds);
            if !c.is_ellipsis {
                text_end = text_end.max(c.text_index + c.text_length);
            }
        }
        TextPiece {
            layout: self.clone(),
            unit,
            index,
            first_cluster: first,
            cluster_count: count,
            text_index: text_start,
            text_length: line.map_or(text_end - text_start, |l| l.text_length),
            text,
            line_index,
            bounds: line.map(|l| l.bounds()).or(cells).unwrap_or_default(),
            ink_bounds: ink,
            is_white_space: false,
        }
    }

    /// Every glyph outline of the block, from its top-left (C# `GetOutline()`).
    pub fn get_outline(&self) -> PathGeometry {
        self.build_outline(0, self.clusters.len(), 0.0, 0.0)
    }

    /// Every glyph outline of the block, moved by (x, y) (C# `GetOutline(x, y)`).
    pub fn get_outline_at(&self, x: f32, y: f32) -> PathGeometry {
        self.build_outline(0, self.clusters.len(), x, y)
    }

    /// The outlines of clusters [first, first + count), moved by (dx, dy) (C# `BuildOutline`).
    pub(crate) fn build_outline(&self, first: usize, count: usize, dx: f32, dy: f32) -> PathGeometry {
        let mut path = BezPath::new();
        let end = self.clusters.len().min(first.saturating_add(count));
        for cluster in self.clusters.iter().take(end).skip(first) {
            cluster.append_outline(&mut path, dx, dy);
        }
        PathGeometry::from_bez_path(path, PathFillRule::NonZero)
    }

    /// The cluster that holds a text position, a UTF-8 byte offset (C# `GetClusterIndexAt`;
    /// None where C# gives -1: a position no shown cluster covers).
    pub fn get_cluster_index_at(&self, text_index: usize) -> Option<usize> {
        let (mut low, mut high) = (0isize, self.clusters.len() as isize - 1);
        while low <= high {
            let mid = ((low + high) / 2) as usize;
            let c = &self.clusters[mid];
            if c.is_ellipsis || text_index < c.text_index {
                high = mid as isize - 1;
            } else if text_index >= c.text_index + c.text_length {
                low = mid as isize + 1;
            } else {
                return Some(mid);
            }
        }
        None
    }

    /// The line that holds a text position (C# `GetLineIndexAt`; None where C# gives -1). The
    /// shown text's end belongs to the last line.
    pub fn get_line_index_at(&self, text_index: usize) -> Option<usize> {
        if let Some(i) = self.lines.iter().position(|l| text_index >= l.text_index && text_index < l.text_index + l.text_length) {
            return Some(i);
        }
        if !self.lines.is_empty() && text_index == self.shown_text_length {
            return Some(self.lines.len() - 1);
        }
        None
    }

    /// The box of the cluster at a text position, or - for a position no cluster covers, like the
    /// text's end - a zero-width box at its line's end (C# `GetRectForIndex`). The error says why
    /// there is none: a position past the text, or one cut off by trimming.
    pub fn get_rect_for_index(&self, text_index: usize) -> Result<RectangleF, String> {
        if text_index > self.text.len() {
            return Err(format!("The text position {text_index} is past the text's end ({} bytes): pass a UTF-8 byte offset into the text.", self.text.len()));
        }
        if let Some(c) = self.get_cluster_index_at(text_index) {
            return Ok(self.clusters[c].bounds);
        }
        let Some(line) = self.get_line_index_at(text_index) else {
            return Err(format!("The character at text position {text_index} was cut off by trimming: it has no place in the block."));
        };
        let l = &self.lines[line];
        Ok(RectangleF::new(l.left + l.width, l.top, 0.0, l.height))
    }

    /// Where each cluster goes along a path (the arithmetic of C# `GetClusterOutlinesAlongPath`):
    /// for each cluster with ink, the transform that takes it from its place in the block to the
    /// path - its advance's centre to the point at `start_distance` + (its x + half its advance)
    /// along the path, turned to the path's direction there, with the first line's baseline
    /// `vertical_offset` below the path (negative: above; "below" is along the normal (-ty, tx) of
    /// a y-down canvas, as C#). `start_distance` is how far along the path the block's left edge
    /// starts. A single closed contour wraps round; on any other
    /// path a cluster beyond either end is left out (None), as are clusters with no ink and
    /// everything on an empty path. The error says what to do with a value that is not finite.
    pub fn get_cluster_transforms_along_path(&self, path: &PathGeometry, start_distance: f32, vertical_offset: f32) -> Result<Vec<Option<Affine>>, String> {
        if !start_distance.is_finite() {
            return Err("start_distance must be a finite distance along the path, calibrated units.".to_string());
        }
        if !vertical_offset.is_finite() {
            return Err("vertical_offset must be a finite distance from the path, calibrated units.".to_string());
        }
        let length = path.get_length();
        let closed = is_single_closed_contour(path.bez_path());
        let baseline = self.lines.first().map_or(0.0, |l| l.baseline());
        let mut result = Vec::with_capacity(self.clusters.len());
        for cluster in &self.clusters {
            if !cluster.has_ink() || path.is_empty() {
                result.push(None);
                continue;
            }
            let centre = cluster.origin.x + (cluster.advance / 2.0);
            let mut distance = start_distance + centre;
            if closed && length > 0.0 {
                distance -= (distance / length).floor() * length;
            } else if distance < 0.0 || distance > length {
                result.push(None);
                continue;
            }
            let point = path.get_point_at_distance(distance);
            let (tx, ty) = (point.tangent.x, point.tangent.y);
            // p' = P + (x - centre) T + (y - baseline + offset) N, with N = (-ty, tx) (y down):
            // C#'s Matrix3x2(tx, ty, -ty, tx, m31, m32) in kurbo's column order.
            let m31 = point.position.x - (centre * tx) + ((baseline - vertical_offset) * ty);
            let m32 = point.position.y - (centre * ty) + ((vertical_offset - baseline) * tx);
            result.push(Some(Affine::new([tx as f64, ty as f64, -ty as f64, tx as f64, m31 as f64, m32 as f64])));
        }
        Ok(result)
    }

    /// Each cluster's outline placed along a path (C# `GetClusterOutlinesAlongPath(path,
    /// startDistance = 0, verticalOffset = 0)`): one geometry per cluster, empty for those left
    /// out (see [`get_cluster_transforms_along_path`](TextLayoutResult::get_cluster_transforms_along_path)).
    pub fn get_cluster_outlines_along_path(&self, path: &PathGeometry, start_distance: f32, vertical_offset: f32) -> Result<Vec<PathGeometry>, String> {
        let transforms = self.get_cluster_transforms_along_path(path, start_distance, vertical_offset)?;
        Ok(self.clusters.iter().zip(transforms).map(|(c, t)| match t {
            Some(t) => c.get_outline().transformed(t),
            None => PathGeometry::empty(),
        }).collect())
    }

    /// The whole text's outline placed along a path, as one geometry (C# `GetOutlineAlongPath`).
    pub fn get_outline_along_path(&self, path: &PathGeometry, start_distance: f32, vertical_offset: f32) -> Result<PathGeometry, String> {
        let mut combined = BezPath::new();
        for outline in self.get_cluster_outlines_along_path(path, start_distance, vertical_offset)? {
            for el in outline.bez_path().elements() {
                combined.push(*el);
            }
        }
        Ok(PathGeometry::from_bez_path(combined, PathFillRule::NonZero))
    }
}

/// True for a path of exactly one contour that is closed (C# `Contours.Count == 1 &&
/// Contours[0].IsClosed`).
fn is_single_closed_contour(path: &BezPath) -> bool {
    let moves = path.elements().iter().filter(|e| matches!(e, PathEl::MoveTo(_))).count();
    moves == 1 && path.elements().iter().any(|e| matches!(e, PathEl::ClosePath))
}
