//! `TextShape`: a single line of text in a registered font, drawn as filled glyph outlines (C#
//! `TextShape`).

use std::rc::Rc;

use kurbo::{Affine, BezPath};
use peniko::{Brush, Fill};

use crate::color::Color;
use crate::fills::ShapeFill;
use crate::geometry::RectangleF;
use crate::paths::{PathFillRule, PathGeometry};
use crate::rendering::VectorScene;
use crate::text::font_face::{FontFace, TOFU_KEY};
use crate::text::text_shaping;
use crate::text::{check_size_and_tracking, FontRegistry, MissingCharacter, MissingCharacterMode, ShapedCluster, TextAlign};

/// The options of a [`TextShape`] (C#'s default arguments: the layer's colour, left aligned, no
/// tracking, the registry's first font, kerning on) and the missing-character choice this crate
/// adds (default [`MissingCharacterMode::ShowTofu`]).
#[derive(Clone, Debug, PartialEq)]
pub struct TextShapeOptions {
    /// The text's colour; None (the default) for the owning layer's colour. Not used by
    /// [`TextShape::with_fill`].
    pub color: Option<Color>,
    /// How the line sits about x (default Left; Justify is refused).
    pub align: TextAlign,
    /// Extra space after every character but the last, as a fraction of the size (default 0).
    pub tracking: f32,
    /// The family; None (the default) for the registry's first font.
    pub family_name: Option<String>,
    /// True (the default) to apply the font's kerning; false to space glyphs by their plain
    /// advances.
    pub kerning: bool,
    /// What a character the font lacks shows (default ShowTofu: always a visible box).
    pub missing_characters: MissingCharacterMode,
}

impl Default for TextShapeOptions {
    fn default() -> TextShapeOptions {
        TextShapeOptions { color: None, align: TextAlign::Left, tracking: 0.0, family_name: None, kerning: true, missing_characters: MissingCharacterMode::ShowTofu }
    }
}

/// One glyph to draw: its outline key in the face, and where its origin is from the line's
/// left end on the baseline (calibrated units, y down).
#[derive(Clone, Copy, Debug, PartialEq)]
struct DrawnGlyph {
    key: u32,
    x: f32,
    y: f32,
}

/// A single line of text in a font of a [`FontRegistry`] (system fonts are never used), drawn
/// as filled glyph outlines. It is shaped once, when it is made, so its [`width`](TextShape::width)
/// is known before it is drawn. Coordinates and sizes are calibrated drawing units: (x, y) is
/// the anchor ON THE BASELINE, `size` is the em size. It paints in its colour, its layer's
/// colour, or the [`ShapeFill`] it was made with (a gradient across the line, for example).
///
/// Each glyph's outline is read once per font face and kept; the renderer keeps each glyph's
/// flattened geometry and draws it with a transform, so text costs no flattening from frame
/// to frame.
#[derive(Clone, Debug)]
pub struct TextShape {
    text: String,
    x: f32,
    y: f32,
    size: f32,
    align: TextAlign,
    tracking: f32,
    kerning: bool,
    family_name: String,
    missing_mode: MissingCharacterMode,
    pub(crate) color: Option<Color>,
    pub(crate) fill: Option<ShapeFill>,
    face: Rc<FontFace>,
    width: f32,
    right_to_left: bool,
    glyphs: Vec<DrawnGlyph>,
    clusters: Vec<ShapedCluster>,
    missing: Vec<MissingCharacter>,
    ink: Option<RectangleF>,
}

impl PartialEq for TextShape {
    fn eq(&self, other: &TextShape) -> bool {
        Rc::ptr_eq(&self.face, &other.face)
            && self.text == other.text
            && self.x == other.x
            && self.y == other.y
            && self.size == other.size
            && self.align == other.align
            && self.tracking == other.tracking
            && self.kerning == other.kerning
            && self.missing_mode == other.missing_mode
            && self.color == other.color
            && self.fill == other.fill
    }
}

impl TextShape {
    /// A line of text in a flat colour (C# `TextShape(text, x, y, size, fonts, color = null,
    /// align = Left, tracking = 0, familyName = null)` and the overload with `kerning`). The
    /// errors are C#'s exceptions, each saying what to do: no font registered, an unknown
    /// family, a size that is not positive, a tracking that is not finite, Justify.
    pub fn new(text: &str, x: f32, y: f32, size: f32, fonts: &FontRegistry, options: TextShapeOptions) -> Result<TextShape, String> {
        TextShape::build(text, x, y, size, fonts, options, None)
    }

    /// A line of text painted by a fill - for example a gradient across the line (C# `TextShape(
    /// text, x, y, size, fonts, fill, ...)`). The fill is in calibrated drawing units, like the
    /// text; `options.color` is not used.
    pub fn with_fill(text: &str, x: f32, y: f32, size: f32, fonts: &FontRegistry, fill: ShapeFill, options: TextShapeOptions) -> Result<TextShape, String> {
        TextShape::build(text, x, y, size, fonts, options, Some(fill))
    }

    fn build(text: &str, x: f32, y: f32, size: f32, fonts: &FontRegistry, options: TextShapeOptions, fill: Option<ShapeFill>) -> Result<TextShape, String> {
        if options.align == TextAlign::Justify {
            return Err("A single line of text cannot be justified: use TextAlign::Left, Center or Right (justified text is a text block's).".to_string());
        }
        check_size_and_tracking(size, options.tracking)?;
        let (family_name, face) = fonts.resolve(options.family_name.as_deref())?;
        let line = text_shaping::shape(&face, text, options.kerning);
        let scale = size / face.units_per_em;
        let tracking_units = options.tracking * size;
        let tofu_key = match options.missing_characters {
            MissingCharacterMode::ShowTofu if !face.notdef_has_ink() => Some(TOFU_KEY),
            MissingCharacterMode::Blank => None,
            _ => Some(0),
        };
        let mut glyphs = Vec::with_capacity(line.glyphs.len());
        let mut ink: Option<kurbo::Rect> = None;
        for glyph in &line.glyphs {
            let cluster = &line.clusters[glyph.cluster];
            if cluster.is_white_space {
                continue;
            }
            let key = if glyph.glyph_id == 0 {
                match tofu_key {
                    Some(key) => key,
                    None => continue,
                }
            } else {
                glyph.glyph_id
            };
            let gx = glyph.x * scale + glyph.cluster as f32 * tracking_units;
            let gy = -glyph.y * scale;
            if let Some(b) = face.outline(key).bounds {
                let placed = kurbo::Rect::new(gx as f64 + b.x0 * scale as f64, gy as f64 - b.y1 * scale as f64, gx as f64 + b.x1 * scale as f64, gy as f64 - b.y0 * scale as f64);
                ink = Some(ink.map_or(placed, |i| i.union(placed)));
                glyphs.push(DrawnGlyph { key, x: gx, y: gy });
            }
        }
        let count = line.clusters.len();
        let width = if count == 0 { 0.0 } else { line.advance * scale + (count as f32 - 1.0) * tracking_units };
        let clusters = line
            .clusters
            .iter()
            .enumerate()
            .map(|(i, c)| ShapedCluster { x: c.x * scale + i as f32 * tracking_units, advance: c.advance * scale, ..*c })
            .collect();
        Ok(TextShape {
            text: text.to_string(),
            x,
            y,
            size,
            align: options.align,
            tracking: options.tracking,
            kerning: options.kerning,
            family_name,
            missing_mode: options.missing_characters,
            color: options.color,
            fill,
            face,
            width,
            right_to_left: line.right_to_left,
            glyphs,
            clusters,
            missing: line.missing,
            ink: ink.map(|r| RectangleF::new(r.x0 as f32, r.y0 as f32, r.width() as f32, r.height() as f32)),
        })
    }

    /// The text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The anchor's x (left edge, centre or right edge by the alignment).
    pub fn x(&self) -> f32 {
        self.x
    }

    /// The baseline's y.
    pub fn y(&self) -> f32 {
        self.y
    }

    /// The em size.
    pub fn size(&self) -> f32 {
        self.size
    }

    /// How the line sits about x.
    pub fn align(&self) -> TextAlign {
        self.align
    }

    /// The tracking, a fraction of the size.
    pub fn tracking(&self) -> f32 {
        self.tracking
    }

    /// Whether the font's kerning is applied.
    pub fn kerning(&self) -> bool {
        self.kerning
    }

    /// The family name the text resolved to in its registry.
    pub fn family_name(&self) -> &str {
        &self.family_name
    }

    /// The colour (None: the layer's).
    pub fn color(&self) -> Option<Color> {
        self.color
    }

    /// The fill, when it was made with one.
    pub fn fill(&self) -> Option<&ShapeFill> {
        self.fill.as_ref()
    }

    /// What a character the font lacks shows in this line.
    pub fn missing_character_mode(&self) -> MissingCharacterMode {
        self.missing_mode
    }

    /// The line's advance width - from the pen's start to its end, tracking included - in
    /// calibrated units. Alignment uses it.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Where the line starts after alignment.
    pub fn left(&self) -> f32 {
        match self.align {
            TextAlign::Center => self.x - self.width / 2.0,
            TextAlign::Right => self.x - self.width,
            _ => self.x,
        }
    }

    /// True when the line was shaped right to left (its first strong character is of a
    /// right-to-left script). Its glyphs are still placed from `left()` rightwards, in the
    /// order they are seen.
    pub fn is_right_to_left(&self) -> bool {
        self.right_to_left
    }

    /// The line's clusters in drawing order (left to right), positioned from `left()`, with
    /// UTF-8 byte offsets into the text and which of them hold a missing character.
    pub fn clusters(&self) -> &[ShapedCluster] {
        &self.clusters
    }

    /// The characters of the text the font lacks, in text order (empty when it has them all).
    pub fn missing_characters(&self) -> &[MissingCharacter] {
        &self.missing
    }

    /// True when the font lacks a character of the text.
    pub fn has_missing_characters(&self) -> bool {
        !self.missing.is_empty()
    }

    /// The outline keys of the glyphs it draws (tests).
    #[cfg(test)]
    pub(crate) fn glyph_keys(&self) -> Vec<u32> {
        self.glyphs.iter().map(|g| g.key).collect()
    }

    fn glyph_transform(&self, glyph: &DrawnGlyph) -> Affine {
        let s = (self.size / self.face.units_per_em) as f64;
        Affine::translate(((self.left() + glyph.x) as f64, (self.y + glyph.y) as f64)) * Affine::scale_non_uniform(s, -s)
    }

    /// The glyph outlines exactly as drawn (calibrated units, placed at the text's position),
    /// with the non-zero rule; empty for text with no visible glyphs (C# `GetOutline`).
    pub fn get_outline(&self) -> PathGeometry {
        let mut path = BezPath::new();
        for glyph in &self.glyphs {
            let outline = self.face.outline(glyph.key);
            let t = self.glyph_transform(glyph);
            for el in &outline.elements {
                path.push(t * *el);
            }
        }
        PathGeometry::from_bez_path(path, PathFillRule::NonZero)
    }

    /// The box around the glyph outlines (control points included), calibrated units; an empty
    /// rectangle at the anchor for text with no visible glyphs (C# `GetBounds`).
    pub fn get_bounds(&self) -> RectangleF {
        match self.ink {
            Some(r) => RectangleF::new(r.x + self.left(), r.y + self.y, r.width, r.height),
            None => RectangleF::new(self.left(), self.y, 0.0, 0.0),
        }
    }

    /// Records the glyphs: each glyph's cached outline under its own transform (the renderer
    /// keeps its flattened geometry by face and glyph), in the colour or fill.
    pub(crate) fn encode(&self, scene: &mut VectorScene, transform: Affine, layer_color: Color, opacity: f32) {
        if self.glyphs.is_empty() {
            return;
        }
        let brush = match &self.fill {
            Some(fill) => fill.to_brush(opacity),
            None => Brush::Solid(self.color.unwrap_or(layer_color).to_peniko(opacity)),
        };
        let is_gradient = self.fill.is_some();
        for glyph in &self.glyphs {
            let outline = self.face.outline(glyph.key);
            let local = self.glyph_transform(glyph);
            // A gradient is in calibrated units: placed back from the glyph's own space.
            let brush_transform = if is_gradient { Some(local.inverse()) } else { None };
            scene.fill_cached(Fill::NonZero, transform, local, &brush, brush_transform, &outline.elements, glyph_key(self.face.id, glyph.key));
        }
    }
}

/// The renderer's geometry key of a glyph of a face.
pub(crate) fn glyph_key(face: u64, glyph: u32) -> u64 {
    let mut h = crate::rendering::GeometryKey::new();
    h.word(0x6C79_7068);
    h.word(face);
    h.word(glyph as u64);
    h.finish()
}
