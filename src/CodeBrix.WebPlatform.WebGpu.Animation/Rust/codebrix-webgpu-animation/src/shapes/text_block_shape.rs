//! `TextBlockShape`: a laid-out block of text placed in a drawing, with its reveal (C#
//! `TextBlockShape`) - the calculation half: what is shown, how much of each cluster, and its
//! outline and bounds. Painting the glyphs is the renderer's.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kurbo::{Affine, BezPath};
use peniko::{BlendMode, Brush, Compose, Fill};

use crate::color::Color;
use crate::fills::ShapeFill;
use crate::geometry::RectangleF;
use crate::paths::{PathFillRule, PathGeometry, PathStrokeStyle};
use crate::rendering::VectorScene;
use crate::shapes::text_shape::glyph_key;
use crate::shapes::EncodeContext;
use crate::text::{TextCluster, TextLayoutResult, TextRunDescriptor, TextStrokePlacement, TextUnit};

/// The optional arguments of C#'s outlined constructor `TextBlockShape(layout, x, y, strokeStyle,
/// strokeColor = null, fillColor = null, fill = null)`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextBlockStrokeOptions {
    /// The outline's colour; None (the default) for the layer's colour.
    pub stroke_color: Option<Color>,
    /// The colour the glyphs are filled with under the outline; None (the default) for no fill
    /// unless `fill` is given.
    pub fill_color: Option<Color>,
    /// A fill the glyphs are painted with under the outline; None (the default) for none.
    pub fill: Option<ShapeFill>,
}

/// One cluster a text block shows, and how much of it (from
/// [`TextBlockShape::get_revealed_clusters`]): the renderer draws the cluster's glyphs with its
/// paint's alpha multiplied by `alpha`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RevealedCluster {
    /// The cluster's index in the layout.
    pub cluster: usize,
    /// Its reveal alpha, above 0 and at most 1 (1 = fully shown).
    pub alpha: f32,
}

/// A piece of a text block made for travelling along a path (the text side of C#
/// `DrawingAnimations.BindTextAlongPath`): the piece, and the y of its line's baseline in the
/// drawing - where a piece rolling on its own line rolls.
#[derive(Clone, Debug)]
pub struct TextPathPiece {
    /// The piece (a character, word or line of the block, never only white space).
    pub shape: TextBlockShape,
    /// Its line's baseline in drawing units (the block's y plus the line's baseline).
    pub baseline: f32,
}

/// A laid-out text block placed with its top-left at (x, y), in calibrated units (C#
/// `TextBlockShape`). It paints in its colour, its layer's, or a fill; a run with a colour or fill
/// of its own paints in that. It can be OUTLINED (a stroke, centred on the glyph outlines or only
/// outside them), and REVEALED unit by unit - characters, words or lines - by a fraction or a
/// count, with an optional soft edge; drive either with a track as a function of the time (the
/// animation model's `RevealFraction` / `RevealCount` bindings). A block can also be split into
/// pieces (characters, words, lines), each a block of its own over the SAME layout, to be moved,
/// staggered or sent along a path one by one.
#[derive(Clone, Debug)]
pub struct TextBlockShape {
    layout: Rc<TextLayoutResult>,
    x: f32,
    y: f32,
    stroke_style: Option<PathStrokeStyle>,
    pub(crate) color: Option<Color>,
    fill_color: Option<Color>,
    pub(crate) fill: Option<ShapeFill>,
    is_filled: bool,
    first: usize,
    count: usize,
    reveal_unit: TextUnit,
    reveal: f32,
    reveal_softness: f32,
    stroke_placement: TextStrokePlacement,
    merge_overlaps: bool,
    /// Each cluster's merged outline at the block's place (MergeOverlaps), made once and shared
    /// by clones and pieces (same layout, same place).
    merged: Rc<RefCell<HashMap<usize, Rc<BezPath>>>>,
}

impl PartialEq for TextBlockShape {
    fn eq(&self, other: &TextBlockShape) -> bool {
        Rc::ptr_eq(&self.layout, &other.layout)
            && self.x == other.x
            && self.y == other.y
            && self.stroke_style == other.stroke_style
            && self.color == other.color
            && self.fill_color == other.fill_color
            && self.fill == other.fill
            && self.is_filled == other.is_filled
            && self.first == other.first
            && self.count == other.count
            && self.reveal_unit == other.reveal_unit
            && self.reveal.to_bits() == other.reveal.to_bits()
            && self.reveal_softness == other.reveal_softness
            && self.stroke_placement == other.stroke_placement
            && self.merge_overlaps == other.merge_overlaps
    }
}

/// Numbers the units of the clusters of a range as they are walked in order: characters by
/// position, words and lines by how many distinct ones came before (a space takes the number of
/// the word after it, which it is drawn - not at all - with) (C# `UnitCounter`).
struct UnitCounter {
    unit: TextUnit,
    first: usize,
    units: i64,
    previous: Option<usize>,
}

impl UnitCounter {
    fn new(unit: TextUnit, first: usize) -> UnitCounter {
        UnitCounter { unit, first, units: -1, previous: None }
    }

    fn next(&mut self, cluster: &TextCluster) -> i64 {
        if self.unit == TextUnit::Character {
            return cluster.index as i64 - self.first as i64;
        }
        let key = if self.unit == TextUnit::Line { Some(cluster.line_index) } else { cluster.word_index };
        let Some(key) = key else { return (self.units + 1).max(0) };
        if Some(key) != self.previous {
            self.units += 1;
            self.previous = Some(key);
        }
        self.units
    }
}

impl TextBlockShape {
    /// A block painted in a colour - None for the layer's (C# `TextBlockShape(layout, x, y,
    /// color = null)`). The error says what to do with a position that is not finite.
    pub fn new(layout: Rc<TextLayoutResult>, x: f32, y: f32, color: Option<Color>) -> Result<TextBlockShape, String> {
        TextBlockShape::build(layout, x, y, None, color, None, None, true, 0, None)
    }

    /// A block painted by a fill - a gradient across the block, for example (C# `TextBlockShape(
    /// layout, x, y, fill)`).
    pub fn with_fill(layout: Rc<TextLayoutResult>, x: f32, y: f32, fill: ShapeFill) -> Result<TextBlockShape, String> {
        TextBlockShape::build(layout, x, y, None, None, None, Some(fill), true, 0, None)
    }

    /// An outlined block (C# `TextBlockShape(layout, x, y, strokeStyle, strokeColor = null,
    /// fillColor = null, fill = null)`): stroked when `stroke_style` is given, filled when the
    /// options give a fill colour or a fill. The error says what to do when it would be neither.
    pub fn with_stroke(layout: Rc<TextLayoutResult>, x: f32, y: f32, stroke_style: Option<PathStrokeStyle>, options: TextBlockStrokeOptions) -> Result<TextBlockShape, String> {
        let is_filled = options.fill_color.is_some() || options.fill.is_some();
        if stroke_style.is_none() && !is_filled {
            return Err("A text block needs an outline, a fill or both: pass a stroke style, a fill colour or a fill.".to_string());
        }
        TextBlockShape::build(layout, x, y, stroke_style, options.stroke_color, options.fill_color, options.fill, is_filled, 0, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn build(layout: Rc<TextLayoutResult>, x: f32, y: f32, stroke_style: Option<PathStrokeStyle>, color: Option<Color>, fill_color: Option<Color>, fill: Option<ShapeFill>, is_filled: bool, first: usize, count: Option<usize>) -> Result<TextBlockShape, String> {
        if !x.is_finite() || !y.is_finite() {
            return Err("A text block's position must be finite numbers (its top-left, calibrated units).".to_string());
        }
        let total = layout.cluster_count();
        let first = first.min(total);
        let count = count.map_or(total - first, |c| c.min(total - first));
        Ok(TextBlockShape { layout, x, y, stroke_style, color, fill_color, fill, is_filled, first, count, reveal_unit: TextUnit::Character, reveal: f32::INFINITY, reveal_softness: 0.0, stroke_placement: TextStrokePlacement::Center, merge_overlaps: false, merged: Rc::new(RefCell::new(HashMap::new())) })
    }

    /// The laid-out text it draws.
    pub fn layout(&self) -> &Rc<TextLayoutResult> {
        &self.layout
    }

    /// The block's left edge.
    pub fn x(&self) -> f32 {
        self.x
    }

    /// The block's top edge.
    pub fn y(&self) -> f32 {
        self.y
    }

    /// The layout's box at the block's place (C# `BoxBounds`).
    pub fn box_bounds(&self) -> RectangleF {
        RectangleF::new(self.x, self.y, self.layout.width(), self.layout.height())
    }

    /// The outline's style (None: not outlined).
    pub fn stroke_style(&self) -> Option<&PathStrokeStyle> {
        self.stroke_style.as_ref()
    }

    /// True when it is outlined.
    pub fn is_stroked(&self) -> bool {
        self.stroke_style.is_some()
    }

    /// True when the glyphs are filled.
    pub fn is_filled(&self) -> bool {
        self.is_filled
    }

    /// The colour: the paint colour, or (outlined) the outline's colour; None for the layer's.
    pub fn color(&self) -> Option<Color> {
        self.color
    }

    /// The fill, when it has one.
    pub fn fill(&self) -> Option<&ShapeFill> {
        self.fill.as_ref()
    }

    /// The fill colour of an outlined block (C# `FillColor`).
    pub fn fill_color(&self) -> Option<Color> {
        self.fill_color
    }

    /// Where the outline lies (default Center).
    pub fn stroke_placement(&self) -> TextStrokePlacement {
        self.stroke_placement
    }

    /// Sets where the outline lies.
    pub fn set_stroke_placement(&mut self, placement: TextStrokePlacement) {
        self.stroke_placement = placement;
    }

    /// The tolerance glyph contours are merged at (C# `MergeTolerance`).
    pub const MERGE_TOLERANCE: f32 = 0.1;

    /// True when each glyph's overlapping contours are merged into one clean outline before it is
    /// filled and outlined (C# `MergeOverlaps`: each cluster's outline simplified by
    /// [`PathGeometry::simplify`] within [`TextBlockShape::MERGE_TOLERANCE`], one by one).
    pub fn merge_overlaps(&self) -> bool {
        self.merge_overlaps
    }

    /// Sets whether overlapping contours are merged.
    pub fn set_merge_overlaps(&mut self, merge: bool) {
        self.merge_overlaps = merge;
    }

    /// The first cluster it draws (a piece's; 0 for a whole block).
    pub fn first_cluster(&self) -> usize {
        self.first
    }

    /// How many clusters it draws.
    pub fn cluster_count(&self) -> usize {
        self.count
    }

    /// The unit the reveal counts in (default Character).
    pub fn reveal_unit(&self) -> TextUnit {
        self.reveal_unit
    }

    /// Sets the unit the reveal counts in.
    pub fn set_reveal_unit(&mut self, unit: TextUnit) {
        self.reveal_unit = unit;
    }

    /// How much is revealed, 0..1 of the units (C# `RevealFraction`; 1 when fully revealed).
    pub fn reveal_fraction(&self) -> f32 {
        let units = self.get_unit_count();
        if self.reveal.is_infinite() || units == 0 {
            return 1.0;
        }
        (self.reveal / units as f32).min(1.0)
    }

    /// Reveals a fraction of the units: 1 shows everything (C# `RevealFraction { set; }`). The
    /// error says what to do with a value outside 0..1.
    pub fn set_reveal_fraction(&mut self, fraction: f32) -> Result<(), String> {
        if fraction.is_nan() || !(0.0..=1.0).contains(&fraction) {
            return Err(format!("A reveal fraction must be between 0 and 1 (it was {fraction}): 0 shows nothing, 1 everything."));
        }
        self.reveal = if fraction == 1.0 { f32::INFINITY } else { fraction * self.get_unit_count() as f32 };
        Ok(())
    }

    /// How many units are revealed (C# `RevealCount`; the unit count when fully revealed).
    pub fn reveal_count(&self) -> f32 {
        if self.reveal.is_infinite() {
            self.get_unit_count() as f32
        } else {
            self.reveal
        }
    }

    /// Reveals a number of units; a fractional count with a soft edge fades the next unit in (C#
    /// `RevealCount { set; }`). The error says what to do with a negative or NaN count.
    pub fn set_reveal_count(&mut self, count: f32) -> Result<(), String> {
        if count.is_nan() || count < 0.0 {
            return Err(format!("A reveal count must be 0 or more (it was {count}): the number of units shown."));
        }
        self.reveal = count;
        Ok(())
    }

    /// The soft edge of the reveal, in units: each unit fades in over this many units before it
    /// is fully shown; 0 (the default) shows each whole unit at once (C# `RevealSoftness`).
    pub fn reveal_softness(&self) -> f32 {
        self.reveal_softness
    }

    /// Sets the reveal's soft edge. The error says what to do with a value that is negative or
    /// not finite.
    pub fn set_reveal_softness(&mut self, softness: f32) -> Result<(), String> {
        if !softness.is_finite() || softness < 0.0 {
            return Err(format!("A reveal softness must be 0 or a positive, finite number of units (it was {softness})."));
        }
        self.reveal_softness = softness;
        Ok(())
    }

    /// What a `RevealFraction` binding does with a track's value at a time (C# clamps it to 0..1,
    /// NaN as 0): never an error, so a frame is always drawable.
    pub fn apply_reveal_fraction(&mut self, value: f32) {
        let clamped = if value.is_nan() { 0.0 } else { value.clamp(0.0, 1.0) };
        let _ = self.set_reveal_fraction(clamped);
    }

    /// What a `RevealCount` binding does with a track's value at a time (C# counts a negative or
    /// NaN value as 0).
    pub fn apply_reveal_count(&mut self, value: f32) {
        let _ = self.set_reveal_count(if value.is_nan() || value < 0.0 { 0.0 } else { value });
    }

    /// True when every unit is fully shown.
    pub fn is_fully_revealed(&self) -> bool {
        self.reveal.is_infinite() || self.reveal >= self.get_unit_count() as f32
    }

    /// The number of units of the reveal's unit (C# `GetUnitCount()`).
    pub fn get_unit_count(&self) -> usize {
        self.get_unit_count_of(self.reveal_unit)
    }

    /// The number of units of a kind among its clusters (C# `GetUnitCount(unit)`).
    pub fn get_unit_count_of(&self, unit: TextUnit) -> usize {
        if unit == TextUnit::Character || self.count == 0 {
            return self.count;
        }
        let mut units = 0;
        let mut previous = None;
        for cluster in &self.layout.clusters()[self.first..self.first + self.count] {
            let key = if unit == TextUnit::Line { Some(cluster.line_index) } else { cluster.word_index };
            if key.is_none() || key == previous {
                continue;
            }
            units += 1;
            previous = key;
        }
        units
    }

    /// The alpha of a unit by the reveal (C# `AlphaOf`): a unit is fully shown once the count
    /// reaches its end; with softness it fades in over the `softness` units before that.
    pub(crate) fn alpha_of(&self, unit: i64) -> f32 {
        if self.reveal.is_infinite() {
            return 1.0;
        }
        if unit < 0 {
            return 0.0;
        }
        let end = unit as f32 + 1.0;
        if self.reveal_softness <= 0.0 {
            return if end <= self.reveal { 1.0 } else { 0.0 };
        }
        (((self.reveal - end) / self.reveal_softness) + 1.0).clamp(0.0, 1.0)
    }

    /// The clusters it shows NOW and how much of each (the reveal applied): the clusters of its
    /// range that have ink and an alpha above 0, in text order. This is what the renderer draws -
    /// each cluster's glyphs from the glyph cache, its paint's alpha times `alpha` - and what
    /// [`get_outline`](TextBlockShape::get_outline) and [`get_bounds`](TextBlockShape::get_bounds)
    /// cover.
    pub fn get_revealed_clusters(&self) -> Vec<RevealedCluster> {
        let clusters = self.layout.clusters();
        let mut counter = UnitCounter::new(self.reveal_unit, self.first);
        let mut shown = Vec::new();
        for cluster in &clusters[self.first..self.first + self.count] {
            let alpha = self.alpha_of(counter.next(cluster));
            if alpha <= 0.0 || !cluster.has_ink() {
                continue;
            }
            shown.push(RevealedCluster { cluster: cluster.index, alpha });
        }
        shown
    }

    /// Splits it into pieces - its characters, words or lines - each a block of its own over the
    /// same layout, at the same place, with the same paint, outline and settings (the reveal
    /// starts fully shown) (C# `CreatePieces`). Only pieces inside its own range are made.
    pub fn create_pieces(&self, unit: TextUnit) -> Vec<TextBlockShape> {
        let mut shapes = Vec::new();
        for piece in self.layout.get_pieces(unit) {
            let first = piece.first_cluster.max(self.first);
            let end = (piece.first_cluster + piece.cluster_count).min(self.first + self.count);
            if end < first || (piece.cluster_count > 0 && end == first) {
                continue;
            }
            let mut shape = TextBlockShape { first, count: end - first, reveal_unit: TextUnit::Character, reveal: f32::INFINITY, reveal_softness: 0.0, ..self.clone() };
            shape.stroke_placement = self.stroke_placement;
            shape.merge_overlaps = self.merge_overlaps;
            shapes.push(shape);
        }
        shapes
    }

    /// Its pieces of a unit as one [`DrawingGroup`](crate::shapes::DrawingGroup) - one element to
    /// add, whose children can be bound one by one (C# `CreatePieceGroup`).
    pub fn create_piece_group(&self, unit: TextUnit) -> crate::shapes::DrawingGroup {
        let mut group = crate::shapes::DrawingGroup::new(None);
        for piece in self.create_pieces(unit) {
            group.add(piece);
        }
        group
    }

    /// The pieces to send along a path (the text side of C# `BindTextAlongPath`): its pieces of a
    /// unit that are not only white space, each with its line's baseline in drawing units.
    pub fn create_path_pieces(&self, unit: TextUnit) -> Vec<TextPathPiece> {
        let clusters = self.layout.clusters();
        self.create_pieces(unit)
            .into_iter()
            .filter(|p| clusters[p.first..p.first + p.count].iter().any(|c| !c.is_white_space))
            .map(|p| {
                let baseline = self.y + self.layout.lines()[clusters[p.first].line_index].baseline();
                TextPathPiece { shape: p, baseline }
            })
            .collect()
    }

    /// The outlines of the glyphs it shows now (the reveal applied), at the block's place, with
    /// the non-zero rule - merged per glyph when [`merge_overlaps`](TextBlockShape::merge_overlaps)
    /// is set (C# `GetOutline`).
    pub fn get_outline(&self) -> PathGeometry {
        let mut path = BezPath::new();
        for shown in self.get_revealed_clusters() {
            self.append_cluster(&mut path, shown.cluster);
        }
        PathGeometry::from_bez_path(path, PathFillRule::NonZero)
    }

    /// The box around the glyphs it shows now, widened for the outline's reach; an empty
    /// rectangle at its top-left when nothing shows (C# `GetBounds`).
    pub fn get_bounds(&self) -> RectangleF {
        let clusters = self.layout.clusters();
        let ink = self.get_revealed_clusters().iter().fold(None, |ink, s| crate::text::text_layout_result::union(ink, clusters[s.cluster].ink_bounds));
        let Some(ink) = ink else { return RectangleF::new(self.x, self.y, 0.0, 0.0) };
        let b = RectangleF::new(ink.x + self.x, ink.y + self.y, ink.width, ink.height);
        match &self.stroke_style {
            None => b,
            Some(style) => {
                let d = style.width() / 2.0 * style.reach_factor();
                RectangleF::new(b.x - d, b.y - d, b.width + 2.0 * d, b.height + 2.0 * d)
            }
        }
    }

    /// One cluster's outline at the block's place: the font's contours, or the merged ones.
    fn append_cluster(&self, path: &mut BezPath, index: usize) {
        let cluster = &self.layout.clusters()[index];
        if !self.merge_overlaps {
            cluster.append_outline(path, self.x, self.y);
            return;
        }
        let merged = self.merged.borrow().get(&index).cloned();
        let merged = merged.unwrap_or_else(|| {
            let mut own = BezPath::new();
            cluster.append_outline(&mut own, self.x, self.y);
            let geometry = PathGeometry::from_bez_path(own, PathFillRule::NonZero);
            let simplified = geometry.simplify(Some(TextBlockShape::MERGE_TOLERANCE)).unwrap_or(geometry);
            let shared = Rc::new(simplified.bez_path().clone());
            self.merged.borrow_mut().insert(index, shared.clone());
            shared
        });
        path.extend(merged.iter());
    }

    /// The run a cluster paints with when it paints differently from the block (C#'s
    /// `PaintKey`: a run with a colour or fill of its own), else None.
    fn own_run(&self, cluster: &TextCluster) -> Option<&TextRunDescriptor> {
        let run = &self.layout.runs()[cluster.run_index()];
        (run.fill().is_some() || run.color().is_some()).then_some(run)
    }

    /// Records the block (C# `Draw`): every cluster the reveal shows, a partly shown one with its
    /// paint's alpha multiplied by its reveal alpha (no layer of its own - glyphs of different
    /// clusters that overlap differ from C#'s layer per glyph there). A filled block without an
    /// outline or merging draws each glyph from the renderer's geometry cache; an outlined or
    /// merged block draws its clusters' outlines as paths: an OUTSIDE outline in a layer of its
    /// own with the glyphs cut out of it (destination-out), then the fill; a CENTRED outline over
    /// the fill.
    pub(crate) fn encode(&self, scene: &mut VectorScene, transform: Affine, layer_color: Color, opacity: f32, ctx: &EncodeContext<'_>) {
        let color = self.color.unwrap_or(layer_color);
        let shown = self.get_revealed_clusters();
        if shown.is_empty() || opacity <= 0.0 {
            return;
        }
        let clusters = self.layout.clusters();
        let simple = self.stroke_style.is_none() && !self.merge_overlaps;
        if simple {
            if !self.is_filled {
                return;
            }
            let block = Affine::translate((self.x as f64, self.y as f64));
            for s in &shown {
                let cluster = &clusters[s.cluster];
                let alpha = opacity * s.alpha;
                let (brush, is_gradient) = self.fill_brush(self.own_run(cluster), color, alpha);
                for g in &cluster.glyphs {
                    let outline = cluster.face.outline(g.key);
                    let local = block * cluster.glyph_transform(g);
                    // A gradient is in calibrated units: placed back from the glyph's own space.
                    let brush_transform = if is_gradient { Some(local.inverse()) } else { None };
                    scene.fill_cached(Fill::NonZero, transform, local, &brush, brush_transform, &outline.elements, glyph_key(cluster.face.id, g.key));
                }
            }
            return;
        }
        // Fully shown clusters in one batch per paint (C#'s batches), partly shown ones alone.
        let mut batches: Vec<(Option<usize>, f32, BezPath)> = Vec::new();
        for s in &shown {
            let cluster = &clusters[s.cluster];
            let key = self.own_run(cluster).map(|_| cluster.run_index());
            if s.alpha >= 1.0 {
                if let Some(batch) = batches.iter_mut().find(|b| b.1 >= 1.0 && b.0 == key) {
                    self.append_cluster(&mut batch.2, s.cluster);
                    continue;
                }
            }
            let mut path = BezPath::new();
            self.append_cluster(&mut path, s.cluster);
            batches.push((key, s.alpha, path));
        }
        for (key, alpha, path) in &batches {
            let run = key.map(|k| &self.layout.runs()[k]);
            self.paint(scene, transform, path, run, color, opacity * alpha, ctx);
        }
    }

    /// The fill's brush for a run of its own or the block, and whether it is a gradient.
    fn fill_brush(&self, run: Option<&TextRunDescriptor>, color: Color, alpha: f32) -> (Brush, bool) {
        match run {
            Some(run) => match run.fill() {
                Some(fill) => (fill.to_brush(alpha), true),
                None => (Brush::Solid(run.color().unwrap_or(color).to_peniko(alpha)), false),
            },
            None => match &self.fill {
                Some(fill) => (fill.to_brush(alpha), true),
                None => (Brush::Solid(self.fill_color.unwrap_or(color).to_peniko(alpha)), false),
            },
        }
    }

    /// C#'s `Paint` for one batch of glyph outlines in block-placed drawing units.
    #[allow(clippy::too_many_arguments)]
    fn paint(&self, scene: &mut VectorScene, transform: Affine, path: &BezPath, run: Option<&TextRunDescriptor>, color: Color, alpha: f32, ctx: &EncodeContext<'_>) {
        let run_color = run.and_then(|r| r.color()).unwrap_or(color);
        let stroke_color = if run.is_some() && !self.is_filled { run_color } else { color };
        let outside = self.stroke_style.is_some() && self.stroke_placement == TextStrokePlacement::Outside;
        if let (true, Some(style)) = (outside, &self.stroke_style) {
            scene.push_layer(BlendMode::default(), 1.0, Affine::IDENTITY, &ctx.area);
            scene.stroke_with_style(style, transform, ctx.drawing, &Brush::Solid(stroke_color.to_peniko(alpha)), None, path);
            scene.push_layer(Compose::DestOut, 1.0, Affine::IDENTITY, &ctx.area);
            scene.fill(Fill::NonZero, transform, &Brush::Solid(Color::WHITE.to_peniko(1.0)), None, path);
            scene.pop_layer();
            scene.pop_layer();
        }
        if self.is_filled {
            let (brush, _) = self.fill_brush(run, color, alpha);
            scene.fill(Fill::NonZero, transform, &brush, None, path);
        }
        if let (false, Some(style)) = (outside, &self.stroke_style) {
            scene.stroke_with_style(style, transform, ctx.drawing, &Brush::Solid(stroke_color.to_peniko(alpha)), None, path);
        }
    }
}
