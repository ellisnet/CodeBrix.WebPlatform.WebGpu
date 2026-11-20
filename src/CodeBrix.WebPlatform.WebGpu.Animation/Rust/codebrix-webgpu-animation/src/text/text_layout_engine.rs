//! `TextLayoutEngine`: lays out blocks of text - wrapped, aligned, spaced, trimmed (C#
//! `TextLayoutEngine`, ported line for line on top of this crate's shaping).

use std::rc::Rc;

use crate::geometry::{PointF, RectangleF};
use crate::text::font_face::{FontFace, TOFU_KEY};
use crate::text::text_cluster::ClusterGlyph;
use crate::text::text_shaping;
use crate::text::{FontRegistry, MissingCharacter, MissingCharacterMode, TextAlign, TextCluster, TextLayoutOptions, TextLayoutResult, TextLineMetrics, TextRunDescriptor, TextTrimming, TextVerticalAlign};

/// Lays out blocks of text - several lines, wrapped to a width, aligned, spaced, trimmed - in
/// fonts of a [`FontRegistry`] (system fonts are never used) (C# `TextLayoutEngine`). Shaping
/// (glyph choice, advances, kerning, ligatures) is harfrust's; line breaking, alignment,
/// justification and trimming are NoSkia's own rules, ported line for line.
///
/// LINE BREAKING (NoSkia's own rules, not the Unicode line-breaking algorithm): the text breaks
/// at its own line breaks (LF, CR, CR LF, NEL, LS, PS, VT, FF) and, with
/// [`max_width`](TextLayoutOptions::max_width), wraps after spaces (not no-break spaces), after a
/// hyphen inside a word (between letters or digits), after a zero width space and before or
/// after a CJK ideograph; a word wider than the line is broken between characters. Spaces at a
/// wrapped line's end hang past the width and do not count in it.
///
/// DIRECTION: lines are laid out left to right, as NoSkia. A run whose first strong character is
/// right-to-left (Arabic, Hebrew) is shaped right to left and its clusters are placed right to
/// left within each line (they stay in text order for breaking, revealing and counting); a run
/// that MIXES directions is not reordered (there is no Unicode bidirectional algorithm).
///
/// Text positions in the result are UTF-8 BYTE offsets (C# counts UTF-16 units).
#[derive(Clone, Copy, Debug)]
pub struct TextLayoutEngine;

impl TextLayoutEngine {
    /// Lays out one uniformly styled block (C# `Layout(fonts, text, size, familyName = null,
    /// options = null)`): the family None for the registry's first font, the options None for the
    /// defaults. The errors are C#'s exceptions, each saying what to do.
    pub fn layout(fonts: &FontRegistry, text: &str, size: f32, family_name: Option<&str>, options: Option<&TextLayoutOptions>) -> Result<Rc<TextLayoutResult>, String> {
        let run = TextRunDescriptor::new(text, size, family_name)?;
        TextLayoutEngine::layout_runs(fonts, &[run], options)
    }

    /// Lays out styled runs as one block, their texts joined in order (C# `Layout(fonts, runs,
    /// options = null)`). At least one run is needed (one with empty text lays out nothing).
    pub fn layout_runs(fonts: &FontRegistry, runs: &[TextRunDescriptor], options: Option<&TextLayoutOptions>) -> Result<Rc<TextLayoutResult>, String> {
        if runs.is_empty() {
            return Err("At least one run is required; pass one run with empty text to lay out nothing.".to_string());
        }
        let settings = options.cloned().unwrap_or_default();
        settings.validate()?;
        Ok(Rc::new(Builder::new(fonts, runs.to_vec(), settings)?.build()))
    }
}

/// A font's ascender, descender (negative) and line gap in font units, chosen as CodeBrix.Imaging's
/// font metrics choose them (FreeType's approach): the typographic line when OS/2 asks for it,
/// else hhea; when either end is then zero, the typographic line if it has an end, else the
/// Windows clipping line (no gap).
pub(crate) fn font_line_metrics(face: &FontFace) -> (f32, f32, f32) {
    let m = face.font.metrics();
    let line = |l: Option<harfrust::font::metrics::LineBox>| l.map_or((0.0, 0.0, 0.0), |l| (l.ascender.to_f32(), l.descender.to_f32(), l.line_gap.to_f32()));
    let typo = line(m.typo_line);
    let (mut ascender, mut descender, mut gap) = if m.use_typo_metrics && m.typo_line.is_some() { typo } else { line(m.hhea_line) };
    if ascender == 0.0 || descender == 0.0 {
        if typo.0 != 0.0 || typo.1 != 0.0 {
            (ascender, descender, gap) = typo;
        } else {
            let win = m.win_line.map_or((0.0, 0.0), |w| (w.ascender.to_f32(), w.descender.to_f32()));
            (ascender, descender, gap) = (win.0, win.1, 0.0);
        }
    }
    (ascender, descender, gap)
}

/// A run's font and vertical metrics, resolved once.
struct RunFont {
    face: Rc<FontFace>,
    /// Font units to calibrated units.
    scale: f32,
    ascent: f32,
    descent: f32,
    natural: f32,
    /// The tracking in units (fraction x size).
    tracking: f32,
}

/// One glyph of an item: its id and its offset from the item's pen start (calibrated units,
/// x right, y UP as the font's).
#[derive(Clone, Copy, Debug)]
struct ItemGlyph {
    glyph_id: u32,
    dx: f32,
    dy: f32,
}

/// A cluster while the block is being laid out.
#[derive(Clone, Debug, Default)]
struct Item {
    start: usize,
    length: usize,
    advance: f32,
    is_white_space: bool,
    is_breaking_space: bool,
    run: usize,
    break_after: bool,
    is_ellipsis: bool,
    ellipsis_text: String,
    x: f32,
    right_to_left: bool,
    is_missing: bool,
    glyphs: Vec<ItemGlyph>,
    missing: Vec<MissingCharacter>,
}

/// A line while the block is being laid out: a range of items.
#[derive(Debug, Default)]
struct Line {
    items: Vec<Item>,
    text_start: usize,
    text_end: usize,
    ends_paragraph: bool,
    empty_run: usize,
    width: f32,
    ascent: f32,
    descent: f32,
    height: f32,
}

struct Builder {
    runs: Vec<TextRunDescriptor>,
    options: TextLayoutOptions,
    run_fonts: Vec<RunFont>,
    run_starts: Vec<usize>,
    text: String,
}

impl Builder {
    fn new(fonts: &FontRegistry, runs: Vec<TextRunDescriptor>, options: TextLayoutOptions) -> Result<Builder, String> {
        let mut text = String::new();
        let mut run_starts = Vec::with_capacity(runs.len() + 1);
        let mut run_fonts = Vec::with_capacity(runs.len());
        for run in &runs {
            run_starts.push(text.len());
            text.push_str(run.text());
            let (_, face) = fonts.resolve(run.family_name())?;
            let size = run.size();
            let scale = size / face.units_per_em;
            let (ascender, descender, gap) = font_line_metrics(&face);
            let (ascent, descent) = (ascender * scale, -descender * scale);
            run_fonts.push(RunFont { face, scale, ascent, descent, natural: ascent + descent + gap * scale, tracking: run.tracking() * size });
        }
        run_starts.push(text.len());
        Ok(Builder { runs, options, run_fonts, run_starts, text })
    }

    fn build(mut self) -> TextLayoutResult {
        let mut lines = self.break_lines();

        // Line heights, then the lines that fit.
        for line in lines.iter_mut() {
            self.measure(line);
        }
        let mut kept = 0usize;
        let mut used = 0.0f32;
        for line in &lines {
            if self.options.max_lines > 0 && kept >= self.options.max_lines {
                break;
            }
            if kept > 0 && self.options.max_height.is_some_and(|h| used + line.height > h) {
                break;
            }
            used += line.height;
            kept += 1;
        }
        let truncated = kept < lines.len();
        lines.truncate(kept);
        let mut shown_end = self.text.len();
        if truncated {
            let last_index = lines.len() - 1;
            let last = &mut lines[last_index];
            shown_end = last.items.last().map_or(last.text_start, |i| i.start + i.length);
            last.text_end = shown_end;
            last.ends_paragraph = true;
            if self.options.trimming != TextTrimming::None {
                let mut last = std::mem::take(&mut lines[last_index]);
                self.add_ellipsis(&mut last);
                shown_end = last.text_end;
                lines[last_index] = last;
            }
        }
        for line in lines.iter_mut() {
            line.width = self.visible_width(&line.items, line.items.len());
        }

        // The box, then each line's place in it.
        let mut box_width = self.options.max_width.unwrap_or(0.0);
        if self.options.max_width.is_none() {
            for line in &lines {
                box_width = box_width.max(line.width);
            }
        }
        let content_height: f32 = lines.iter().map(|l| l.height).sum();
        let box_height = self.options.max_height.unwrap_or(content_height);
        let mut top = match self.options.vertical_align {
            TextVerticalAlign::Center => (box_height - content_height) / 2.0,
            TextVerticalAlign::Bottom => box_height - content_height,
            TextVerticalAlign::Top => 0.0,
        };

        let mut metrics = Vec::with_capacity(lines.len());
        let mut clusters: Vec<TextCluster> = Vec::new();
        let mut missing: Vec<MissingCharacter> = Vec::new();
        let mut words = 0usize;
        for (l, line) in lines.iter_mut().enumerate() {
            let (left, extra) = self.place(line, box_width);
            let baseline_offset = ((line.height - (line.ascent + line.descent)) / 2.0) + line.ascent;
            let baseline = top + baseline_offset;
            let first_cluster = clusters.len();
            let mut previous_was_word = false;
            self.position_items(line, left, extra);
            if extra > 0.0 {
                line.width = self.visible_width(&line.items, line.items.len());
            }
            for item in &line.items {
                let mut word = None;
                if !item.is_white_space {
                    if !previous_was_word {
                        words += 1;
                    }
                    word = Some(words - 1);
                }
                previous_was_word = !item.is_white_space;
                missing.extend(item.missing.iter().copied());
                let index = clusters.len();
                clusters.push(self.make_cluster(index, item, l, word, top, line.height, baseline));
            }
            metrics.push(TextLineMetrics {
                index: l,
                text_index: line.text_start,
                text_length: line.text_end.saturating_sub(line.text_start),
                top,
                height: line.height,
                baseline_offset,
                left,
                width: line.width,
                ascent: line.ascent,
                descent: line.descent,
                first_cluster,
                cluster_count: clusters.len() - first_cluster,
                ends_paragraph: line.ends_paragraph,
            });
            top += line.height;
        }
        missing.sort_by_key(|m| m.text_index);

        let text = std::mem::take(&mut self.text);
        let runs = std::mem::take(&mut self.runs);
        TextLayoutResult::new(text, runs, self.options.clone(), (box_width, box_height), metrics, clusters, words, truncated, shown_end, missing)
    }

    /// Splits the text into paragraphs, shapes each paragraph's runs and wraps them into lines.
    fn break_lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        let mut start = 0usize;
        loop {
            let (end, break_length) = self.find_break(start);
            let last = end >= self.text.len();
            let items = self.shape_paragraph(start, end);
            self.wrap_paragraph(items, start, end + break_length, &mut lines);
            if last {
                break;
            }
            start = end + break_length;
            if start >= self.text.len() {
                // A break at the very end leaves one empty line after it.
                lines.push(Line { text_start: start, text_end: start, ends_paragraph: true, empty_run: self.run_at(start), ..Line::default() });
                break;
            }
        }
        lines
    }

    /// The next line break at or after `start`: its byte offset and length in bytes (the text's
    /// end and 0 when there is none).
    fn find_break(&self, start: usize) -> (usize, usize) {
        let bytes = self.text.as_bytes();
        for (offset, c) in self.text[start..].char_indices() {
            let i = start + offset;
            if c == '\r' {
                return (i, if bytes.get(i + 1) == Some(&b'\n') { 2 } else { 1 });
            }
            if matches!(c, '\n' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}') {
                return (i, c.len_utf8());
            }
        }
        (self.text.len(), 0)
    }

    fn run_at(&self, index: usize) -> usize {
        (0..self.runs.len()).find(|r| index < self.run_starts[r + 1]).unwrap_or(self.runs.len() - 1)
    }

    /// Shapes each run's part of a paragraph separately and marks where a line may break.
    fn shape_paragraph(&self, start: usize, end: usize) -> Vec<Item> {
        let mut items = Vec::new();
        for r in 0..self.runs.len() {
            let s = start.max(self.run_starts[r]);
            let e = end.min(self.run_starts[r + 1]);
            if e <= s {
                continue;
            }
            items.extend(self.shape_segment(r, s, e));
        }
        for i in 0..items.len().saturating_sub(1) {
            items[i].break_after = self.can_break_between(&items[i], &items[i + 1]);
        }
        items
    }

    /// One run's part of a paragraph as items in TEXT order (a right-to-left segment's clusters
    /// come from the shaper in visual order and are put back in text order here).
    fn shape_segment(&self, r: usize, s: usize, e: usize) -> Vec<Item> {
        let font = &self.run_fonts[r];
        let segment = &self.text[s..e];
        let shaped = text_shaping::shape(&font.face, segment, self.options.kerning);
        let mut order: Vec<usize> = (0..shaped.clusters.len()).collect();
        order.sort_by_key(|&i| shaped.clusters[i].text_index);
        let mut items: Vec<Item> = Vec::with_capacity(order.len());
        let mut index_of = vec![0usize; shaped.clusters.len()];
        for (position, &i) in order.iter().enumerate() {
            let c = &shaped.clusters[i];
            index_of[i] = position;
            items.push(Item {
                start: s + c.text_index,
                length: c.text_length,
                advance: c.advance * font.scale,
                is_white_space: c.is_white_space,
                is_breaking_space: c.is_white_space && !segment[c.text_index..c.text_index + c.text_length].chars().any(is_no_break_space),
                run: r,
                right_to_left: shaped.right_to_left,
                is_missing: c.is_missing,
                ..Item::default()
            });
        }
        // Leading characters with no glyph of their own join the first cluster (as C#).
        if let Some(first) = items.first_mut() {
            if first.start > s {
                first.length += first.start - s;
                first.start = s;
            }
        }
        for glyph in &shaped.glyphs {
            let c = &shaped.clusters[glyph.cluster];
            items[index_of[glyph.cluster]].glyphs.push(ItemGlyph { glyph_id: glyph.glyph_id, dx: (glyph.x - c.x) * font.scale, dy: glyph.y * font.scale });
        }
        for m in &shaped.missing {
            let at = s + m.text_index;
            if let Some(item) = items.iter_mut().rev().find(|i| i.start <= at) {
                item.missing.push(MissingCharacter { text_index: at, character: m.character });
            }
        }
        items
    }

    /// NoSkia's break opportunities between two neighbouring clusters (C# `CanBreakBetween`).
    fn can_break_between(&self, a: &Item, b: &Item) -> bool {
        if a.is_breaking_space {
            return !b.is_white_space;
        }
        if a.is_white_space || b.is_white_space {
            return false;
        }
        let a_text = &self.text[a.start..a.start + a.length];
        let b_text = &self.text[b.start..b.start + b.length];
        let last_a = a_text.chars().next_back().map_or(0, |c| c as u32);
        let first_b = b_text.chars().next().map_or(0, |c| c as u32);
        if last_a == 0x200B {
            return true;
        }
        if is_ideograph(last_a) || is_ideograph(first_b) {
            return true;
        }
        if matches!(last_a, 0x2D | 0x2010 | 0x2012 | 0x2013) && a_text.chars().count() == 1 {
            // A hyphen inside a word: only between letters or digits.
            let before = self.text[..a.start].chars().next_back();
            let after = self.text[b.start..].chars().next();
            return before.is_some_and(is_word_char) && after.is_some_and(is_word_char);
        }
        false
    }

    /// Greedy wrapping (C# `WrapParagraph`): a line takes items until the next visible one would
    /// pass the width, then breaks at the last opportunity (or, with none, before that item).
    fn wrap_paragraph(&self, items: Vec<Item>, paragraph_start: usize, next_paragraph: usize, lines: &mut Vec<Line>) {
        if items.is_empty() {
            lines.push(Line { text_start: paragraph_start, text_end: next_paragraph, ends_paragraph: true, empty_run: self.run_at(paragraph_start), ..Line::default() });
            return;
        }
        let mut s = 0usize;
        while s < items.len() {
            let mut e = items.len() - 1;
            if let Some(max_width) = self.options.max_width {
                let mut pen = 0.0f32;
                let mut last_break: Option<usize> = None;
                let mut any_visible = false;
                for (i, item) in items.iter().enumerate().skip(s) {
                    if !item.is_white_space {
                        if any_visible && pen + item.advance > max_width {
                            e = match last_break {
                                Some(b) if b >= s => b,
                                _ => i - 1,
                            };
                            break;
                        }
                        any_visible = true;
                    }
                    pen += item.advance + self.run_fonts[item.run].tracking;
                    if item.break_after {
                        last_break = Some(i);
                    }
                }
            }
            let ends_paragraph = e == items.len() - 1;
            lines.push(Line {
                items: items[s..=e].to_vec(),
                text_start: if s == 0 { paragraph_start } else { items[s].start },
                text_end: if ends_paragraph { next_paragraph } else { items[e + 1].start },
                ends_paragraph,
                empty_run: items[s].run,
                ..Line::default()
            });
            s = e + 1;
        }
    }

    fn measure(&self, line: &mut Line) {
        let (mut ascent, mut descent, mut natural) = (0.0f32, 0.0f32, 0.0f32);
        if line.items.is_empty() {
            let run = &self.run_fonts[line.empty_run];
            (ascent, descent, natural) = (run.ascent, run.descent, run.natural);
        }
        for item in &line.items {
            let run = &self.run_fonts[item.run];
            ascent = ascent.max(run.ascent);
            descent = descent.max(run.descent);
            natural = natural.max(run.natural);
        }
        line.ascent = ascent;
        line.descent = descent;
        line.height = if self.options.line_height > 0.0 { self.options.line_height } else { natural * self.options.line_spacing };
    }

    /// The width from the first item's pen start to the last visible item's end, tracking between
    /// items included, over the first `count` items.
    fn visible_width(&self, items: &[Item], count: usize) -> f32 {
        let mut last = count as isize - 1;
        while last >= 0 && items[last as usize].is_white_space {
            last -= 1;
        }
        let mut width = 0.0f32;
        for i in 0..=last {
            let item = &items[i as usize];
            width += item.advance;
            if i < last {
                width += self.run_fonts[item.run].tracking;
            }
        }
        width
    }

    /// Cuts the last kept line back until the ellipsis fits after it (C# `AddEllipsis`).
    fn add_ellipsis(&self, line: &mut Line) {
        let run = line.items.last().map_or(line.empty_run, |i| i.run);
        let ellipsis = self.options.ellipsis.clone();
        let font = &self.run_fonts[run];
        let mut ellipsis_advance = 0.0f32;
        let mut ellipsis_glyphs = Vec::new();
        let mut ellipsis_missing = false;
        let mut ellipsis_chars = Vec::new();
        if !ellipsis.is_empty() {
            let shaped = text_shaping::shape(&font.face, &ellipsis, self.options.kerning);
            for c in &shaped.clusters {
                ellipsis_advance += c.advance * font.scale;
                ellipsis_missing |= c.is_missing;
            }
            for g in &shaped.glyphs {
                ellipsis_glyphs.push(ItemGlyph { glyph_id: g.glyph_id, dx: g.x * font.scale, dy: g.y * font.scale });
            }
            ellipsis_chars = shaped.missing.iter().map(|m| m.character).collect();
        }

        let available = self.options.max_width.unwrap_or(f32::INFINITY);
        let items = &mut line.items;
        let mut keep = items.len();
        while keep > 0 && items[keep - 1].is_white_space {
            keep -= 1;
        }
        let mut character_keep = keep;
        while character_keep > 0 && !self.fits(items, character_keep, ellipsis_advance, available) {
            character_keep -= 1;
            while character_keep > 0 && items[character_keep - 1].is_white_space {
                character_keep -= 1;
            }
        }
        let mut new_count = character_keep;
        if self.options.trimming == TextTrimming::WordEllipsis {
            let mut word_keep = keep;
            while word_keep > 0 && !(self.fits(items, word_keep, ellipsis_advance, available) && ends_word(items, word_keep)) {
                word_keep -= 1;
                while word_keep > 0 && items[word_keep - 1].is_white_space {
                    word_keep -= 1;
                }
            }
            if word_keep > 0 {
                new_count = word_keep;
            }
        }

        items.truncate(new_count);
        let cut = if new_count > 0 { items[new_count - 1].start + items[new_count - 1].length } else { line.text_start };
        line.text_end = cut;
        if !ellipsis.is_empty() {
            items.push(Item {
                start: cut,
                length: 0,
                advance: ellipsis_advance,
                run,
                is_ellipsis: true,
                ellipsis_text: ellipsis,
                is_missing: ellipsis_missing,
                glyphs: ellipsis_glyphs,
                missing: ellipsis_chars.into_iter().map(|character| MissingCharacter { text_index: cut, character }).collect(),
                ..Item::default()
            });
        }
    }

    /// Whether the first `count` items plus the ellipsis fit the width.
    fn fits(&self, items: &[Item], count: usize, ellipsis: f32, available: f32) -> bool {
        let mut width = self.visible_width(items, count);
        if count > 0 {
            width += self.run_fonts[items[count - 1].run].tracking;
        }
        width + ellipsis <= available
    }

    /// The line's left edge in the box, and (justified) the stretch each inner space gets.
    fn place(&self, line: &Line, box_width: f32) -> (f32, f32) {
        let slack = box_width - line.width;
        match self.options.align {
            TextAlign::Center => (slack / 2.0, 0.0),
            TextAlign::Right => (slack, 0.0),
            TextAlign::Justify => {
                let mut extra = 0.0;
                if !line.ends_paragraph && slack > 0.0 {
                    let spaces = count_inner_spaces(&line.items);
                    if spaces > 0 {
                        extra = slack / spaces as f32;
                    }
                }
                (0.0, extra)
            }
            TextAlign::Left => (0.0, 0.0),
        }
    }

    /// Gives each item its x (C# `PositionItems`), with one addition: the items of a run shaped
    /// right to left are placed right to left (each contiguous stretch of them reversed, the
    /// line's trailing spaces left at its end).
    fn position_items(&self, line: &mut Line, left: f32, extra: f32) {
        let items = &mut line.items;
        let count = items.len();
        let first = items.iter().position(|i| !i.is_white_space).unwrap_or(count);
        let last = items.iter().rposition(|i| !i.is_white_space);
        for (i, item) in items.iter_mut().enumerate() {
            if extra > 0.0 && item.is_white_space && i > first && last.is_some_and(|l| i < l) {
                item.advance += extra;
            }
        }
        let visual = visual_order(items, last);
        let mut x = left;
        for (n, &i) in visual.iter().enumerate() {
            items[i].x = x;
            x += items[i].advance;
            if n + 1 < count {
                x += self.run_fonts[items[i].run].tracking;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn make_cluster(&self, index: usize, item: &Item, line_index: usize, word_index: Option<usize>, line_top: f32, line_height: f32, baseline: f32) -> TextCluster {
        let run = &self.run_fonts[item.run];
        let text = if item.is_ellipsis { item.ellipsis_text.clone() } else { self.text[item.start..item.start + item.length].to_string() };
        let mut glyphs = Vec::new();
        let mut ink: Option<kurbo::Rect> = None;
        if !item.is_white_space && !text.is_empty() {
            let notdef_key = match self.options.missing_characters {
                MissingCharacterMode::ShowTofu if !run.face.notdef_has_ink() => Some(TOFU_KEY),
                MissingCharacterMode::Blank => None,
                _ => Some(0),
            };
            let s = run.scale as f64;
            for g in &item.glyphs {
                let key = if g.glyph_id == 0 {
                    match notdef_key {
                        Some(key) => key,
                        None => continue,
                    }
                } else {
                    g.glyph_id
                };
                let (gx, gy) = (item.x + g.dx, baseline - g.dy);
                if let Some(b) = run.face.outline(key).bounds {
                    let placed = kurbo::Rect::new(gx as f64 + b.x0 * s, gy as f64 - b.y1 * s, gx as f64 + b.x1 * s, gy as f64 - b.y0 * s);
                    ink = Some(ink.map_or(placed, |i| i.union(placed)));
                    glyphs.push(ClusterGlyph { key, x: gx, y: gy });
                }
            }
        }
        TextCluster {
            index,
            text_index: item.start,
            text_length: item.length,
            text,
            line_index,
            word_index,
            run_index: item.run,
            is_white_space: item.is_white_space,
            is_ellipsis: item.is_ellipsis,
            is_missing: item.is_missing,
            origin: PointF::new(item.x, baseline),
            advance: item.advance,
            bounds: RectangleF::new(item.x, line_top, item.advance, line_height),
            ink_bounds: ink.map(|r| RectangleF::new(r.x0 as f32, r.y0 as f32, r.width() as f32, r.height() as f32)),
            glyphs,
            face: run.face.clone(),
            scale: run.scale,
        }
    }
}

/// The order items are placed in from the left: text order, except that each contiguous stretch
/// of right-to-left items before the line's trailing spaces is reversed.
fn visual_order(items: &[Item], last_visible: Option<usize>) -> Vec<usize> {
    let mut order: Vec<usize> = (0..items.len()).collect();
    let Some(end) = last_visible else { return order };
    let mut i = 0;
    while i <= end {
        if items[i].right_to_left {
            let mut j = i;
            while j < end && items[j + 1].right_to_left {
                j += 1;
            }
            order[i..=j].reverse();
            i = j + 1;
        } else {
            i += 1;
        }
    }
    order
}

/// Whether a cut after the first `count` items falls at a word's end.
fn ends_word(items: &[Item], count: usize) -> bool {
    count == items.len() || items[count].is_white_space || items[count - 1].break_after
}

fn count_inner_spaces(items: &[Item]) -> usize {
    let Some(first) = items.iter().position(|i| !i.is_white_space) else { return 0 };
    let last = items.iter().rposition(|i| !i.is_white_space).unwrap_or(0);
    items[first..last].iter().filter(|i| i.is_white_space).count()
}

/// The no-break spaces (a line never breaks after them) (C# `IsNoBreakSpace`).
pub(crate) fn is_no_break_space(c: char) -> bool {
    matches!(c, '\u{00A0}' | '\u{2007}' | '\u{202F}')
}

/// NoSkia's CJK ranges: radicals, kana, ideographs, compatibility ideographs, Hangul syllables and
/// the supplementary ideograph planes (C# `IsIdeograph`).
pub(crate) fn is_ideograph(value: u32) -> bool {
    (0x2E80..=0x2FDF).contains(&value)
        || (0x3040..=0x30FF).contains(&value)
        || (0x3400..=0x4DBF).contains(&value)
        || (0x4E00..=0x9FFF).contains(&value)
        || (0xF900..=0xFAFF).contains(&value)
        || (0xAC00..=0xD7AF).contains(&value)
        || (0x20000..=0x3FFFF).contains(&value)
}

/// A letter or digit, for the hyphen rule (C# `Rune.IsLetterOrDigit`; Rust's alphanumeric
/// property, which also counts letter-like numbers and the marks Unicode calls alphabetic).
pub(crate) fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}
