//! The internal shaping step: one line of text in one face becomes positioned glyphs grouped
//! into clusters (harfrust shapes; the grouping is by harfrust's cluster value, a UTF-8 byte
//! offset into the text), with the characters the font lacks recorded.

use harfrust::{Buffer, Direction, Feature, ShapeOptions, ShaperFont, Tag};

use crate::text::font_face::FontFace;
use crate::text::missing_character::MissingCharacter;
use crate::text::shaped_cluster::ShapedCluster;

/// One glyph of a shaped line: its id, its pen position and vertical offset (FONT UNITS,
/// x from the line's start, y up), and the cluster it belongs to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlacedGlyph {
    pub(crate) glyph_id: u32,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) cluster: usize,
}

/// A shaped line: glyphs and clusters in drawing order (left to right), the advance in font
/// units (no tracking), the direction, and the missing characters in text order.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ShapedLine {
    pub(crate) glyphs: Vec<PlacedGlyph>,
    pub(crate) clusters: Vec<ShapedCluster>,
    pub(crate) advance: f32,
    pub(crate) right_to_left: bool,
    pub(crate) missing: Vec<MissingCharacter>,
}

/// Shapes `text` as ONE run (the direction and script of its first strong character; a line that
/// mixes directions is not reordered). Kerning off = the `kern` feature switched off. A glyph 0
/// (`.notdef`, a character the font lacks) takes [`FontFace::missing_advance`].
pub(crate) fn shape(face: &FontFace, text: &str, kerning: bool) -> ShapedLine {
    if text.is_empty() {
        return ShapedLine::default();
    }
    let shaper = ShaperFont::new(&face.font);
    let mut buffer = Buffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    let direction = buffer.direction();
    let script = buffer.script();
    let plan = face.plan(direction, script, kerning);
    let features: Vec<Feature> = if kerning { Vec::new() } else { vec![Feature::new(Tag::new(b"kern"), 0, ..)] };
    if harfrust::shape(&shaper, &mut buffer, ShapeOptions::new().plan(Some(&plan)).features(&features)).is_err() {
        return ShapedLine::default();
    }
    let missing_advance = face.missing_advance();
    let infos = buffer.glyph_infos();
    let positions = buffer.glyph_positions();

    // Each cluster's byte range: from its start to the next larger start (or the text's end).
    let mut starts: Vec<usize> = infos.iter().map(|i| i.cluster as usize).collect();
    starts.sort_unstable();
    starts.dedup();
    // The next cluster start after this one, by binary search over the sorted starts.
    let end_of = |start: usize| {
        let i = starts.partition_point(|s| *s <= start);
        starts.get(i).copied().unwrap_or(text.len())
    };

    let mut line = ShapedLine { right_to_left: direction == Direction::RightToLeft, ..ShapedLine::default() };
    let mut pen = 0.0f32;
    let mut current: Option<u32> = None;
    for (info, position) in infos.iter().zip(positions) {
        if current != Some(info.cluster) {
            current = Some(info.cluster);
            let start = info.cluster as usize;
            let end = end_of(start).max(start);
            let piece = text.get(start..end).unwrap_or("");
            line.clusters.push(ShapedCluster {
                text_index: start,
                text_length: end - start,
                x: pen,
                advance: 0.0,
                is_white_space: !piece.is_empty() && piece.chars().all(char::is_whitespace),
                is_missing: false,
            });
        }
        let index = line.clusters.len() - 1;
        let is_missing = info.glyph_id == 0;
        let advance = if is_missing { missing_advance } else { position.x_advance as f32 };
        line.glyphs.push(PlacedGlyph { glyph_id: info.glyph_id, x: pen + position.x_offset as f32, y: position.y_offset as f32, cluster: index });
        line.clusters[index].advance += advance;
        if is_missing {
            line.clusters[index].is_missing = true;
        }
        pen += advance;
    }
    line.advance = pen;

    // The characters the font lacks, in text order: those of a missing cluster that the font's
    // character map does not hold (the cluster's first character when it holds them all).
    let mut missing_clusters: Vec<&ShapedCluster> = line.clusters.iter().filter(|c| c.is_missing).collect();
    missing_clusters.sort_by_key(|c| c.text_index);
    for cluster in missing_clusters {
        let piece = &text[cluster.text_index..cluster.text_index + cluster.text_length];
        let mut found = false;
        for (offset, character) in piece.char_indices() {
            if face.glyph_for_char(character).is_none() && !is_default_ignorable(character) {
                line.missing.push(MissingCharacter { text_index: cluster.text_index + offset, character });
                found = true;
            }
        }
        if !found {
            if let Some(character) = piece.chars().next() {
                line.missing.push(MissingCharacter { text_index: cluster.text_index, character });
            }
        }
    }
    line
}

/// Unicode's default-ignorable code points (format characters, variation selectors and the
/// like): never reported as missing - shaping makes them invisible whether the font has them or
/// not.
pub(crate) fn is_default_ignorable(c: char) -> bool {
    matches!(c as u32,
        0x00AD | 0x034F | 0x061C | 0x115F..=0x1160 | 0x17B4..=0x17B5 | 0x180B..=0x180F
        | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0x3164 | 0xFE00..=0xFE0F
        | 0xFEFF | 0xFFA0 | 0xFFF0..=0xFFF8 | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A
        | 0xE0000..=0xE0FFF)
}
