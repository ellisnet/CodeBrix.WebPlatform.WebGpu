//! The internal face of one registered font file: the font read from its bytes, its metrics,
//! and the caches that live as long as the face - glyph outlines in font units and shape plans.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use harfrust::font::glyph::outline::{DrawMemory, DrawOptions, PathSink};
use harfrust::font::name::NameId;
use harfrust::{Direction, Feature, Font, GlyphId, Script, ShapePlan, Tag};
use kurbo::{PathEl, Rect};

thread_local! {
    static NEXT_FACE_ID: Cell<u64> = const { Cell::new(1) };
}

/// A glyph's outline in FONT UNITS (y up), drawn once by the font reader, with the box around
/// its points (control points included) - None for a glyph with no ink (a space).
#[derive(Debug)]
pub(crate) struct GlyphOutline {
    pub(crate) elements: Vec<PathEl>,
    pub(crate) bounds: Option<Rect>,
}

/// The reserved outline-cache key of the hollow box drawn for a missing character whose font has
/// an ink-less `.notdef` (glyph ids are below 65,536).
pub(crate) const TOFU_KEY: u32 = u32::MAX;

/// One registered font file.
pub(crate) struct FontFace {
    /// A number no other face of this program has (the renderer's glyph geometry key).
    pub(crate) id: u64,
    pub(crate) font: Font,
    pub(crate) family_name: String,
    pub(crate) units_per_em: f32,
    pub(crate) cap_height: f32,
    outlines: RefCell<HashMap<u32, Rc<GlyphOutline>>>,
    plans: RefCell<HashMap<(bool, u32, bool), Rc<ShapePlan>>>,
}

impl fmt::Debug for FontFace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontFace").field("family_name", &self.family_name).field("units_per_em", &self.units_per_em).finish_non_exhaustive()
    }
}

/// read-fonts' path sink into kurbo elements (font units, y up).
struct Sink(Vec<PathEl>);

impl PathSink for Sink {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(PathEl::MoveTo((x as f64, y as f64).into()));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(PathEl::LineTo((x as f64, y as f64).into()));
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.push(PathEl::QuadTo((cx as f64, cy as f64).into(), (x as f64, y as f64).into()));
    }

    fn curve_to(&mut self, c0x: f32, c0y: f32, c1x: f32, c1y: f32, x: f32, y: f32) {
        self.0.push(PathEl::CurveTo((c0x as f64, c0y as f64).into(), (c1x as f64, c1y as f64).into(), (x as f64, y as f64).into()));
    }

    fn close(&mut self) {
        self.0.push(PathEl::ClosePath);
    }
}

/// The box around every point of the elements (control points included), None when empty.
pub(crate) fn element_bounds(elements: &[PathEl]) -> Option<Rect> {
    let mut bounds: Option<Rect> = None;
    let mut add = |p: kurbo::Point| {
        bounds = Some(match bounds {
            None => Rect::from_points(p, p),
            Some(b) => b.union_pt(p),
        });
    };
    for el in elements {
        match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => add(*p),
            PathEl::QuadTo(a, b) => {
                add(*a);
                add(*b);
            }
            PathEl::CurveTo(a, b, c) => {
                add(*a);
                add(*b);
                add(*c);
            }
            PathEl::ClosePath => {}
        }
    }
    bounds
}

impl FontFace {
    /// Reads a font from its bytes (the first face of a collection). The error says what to do.
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Result<FontFace, String> {
        if bytes.is_empty() {
            return Err("The data could not be read as a font: it is empty. Pass the bytes of a TrueType or OpenType font file.".to_string());
        }
        let Some(font) = Font::new(bytes, 0) else {
            return Err("The data could not be read as a font: pass the bytes of a TrueType or OpenType font file.".to_string());
        };
        if font.num_glyphs() == 0 {
            return Err("The data could not be read as a font: it holds no glyphs. Pass the bytes of a TrueType or OpenType font file.".to_string());
        }
        let metrics = font.metrics();
        let upem = metrics.units_per_em.max(1) as f32;
        let cap_height = metrics.cap_height.map(|c| c.to_f32()).filter(|c| *c > 0.0).or_else(|| metrics.x_height.map(|x| x.to_f32()).filter(|x| *x > 0.0)).unwrap_or(upem * 0.7);
        let family_name = font
            .names()
            .get(NameId::FAMILY_NAME)
            .and_then(|n| n.english_or_first())
            .map(|l| l.name().chars().collect::<String>())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unnamed font".to_string());
        let id = NEXT_FACE_ID.with(|n| {
            let id = n.get();
            n.set(id + 1);
            id
        });
        Ok(FontFace { id, font, family_name, units_per_em: upem, cap_height, outlines: RefCell::new(HashMap::new()), plans: RefCell::new(HashMap::new()) })
    }

    /// The glyph of a character in the font's character map (None: the font lacks it).
    pub(crate) fn glyph_for_char(&self, c: char) -> Option<u32> {
        self.font.charmap().map_unicode(c).map(|g| g.to_u32())
    }

    /// The glyph's own advance width, font units.
    pub(crate) fn advance_of(&self, glyph_id: u32) -> f32 {
        self.font.glyph_metrics().h_advance(GlyphId::new(glyph_id))
    }

    /// The advance a missing character takes (the owner's rule): the `.notdef` glyph's own when
    /// it is above 0; else the space's; else half an em. Font units.
    pub(crate) fn missing_advance(&self) -> f32 {
        let notdef = self.advance_of(0);
        if notdef > 0.0 {
            return notdef;
        }
        if let Some(space) = self.glyph_for_char(' ') {
            let a = self.advance_of(space);
            if a > 0.0 {
                return a;
            }
        }
        self.units_per_em * 0.5
    }

    /// The glyph's outline (font units, y up), drawn once and kept. [`TOFU_KEY`] is the hollow
    /// box of [`tofu_outline`](FontFace::tofu_outline).
    pub(crate) fn outline(&self, glyph_id: u32) -> Rc<GlyphOutline> {
        if let Some(found) = self.outlines.borrow().get(&glyph_id) {
            return found.clone();
        }
        let elements = if glyph_id == TOFU_KEY {
            self.tofu_elements()
        } else {
            let mut sink = Sink(Vec::new());
            if let Some(glyph) = self.font.outline_glyphs().get(GlyphId::new(glyph_id)) {
                if glyph.draw(&DrawOptions::new(), DrawMemory::Scratch, &mut sink).is_err() {
                    sink.0.clear();
                }
            }
            sink.0
        };
        let mut elements = elements;
        elements.shrink_to_fit();
        let bounds = element_bounds(&elements);
        let outline = Rc::new(GlyphOutline { elements, bounds });
        self.outlines.borrow_mut().insert(glyph_id, outline.clone());
        outline
    }

    /// True when the `.notdef` glyph (glyph 0) has ink.
    pub(crate) fn notdef_has_ink(&self) -> bool {
        self.outline(0).bounds.is_some()
    }

    /// The hollow box drawn for a missing character when the font's `.notdef` has no ink: inside
    /// the missing advance, inset by a tenth of an em on each side, from the baseline to the cap
    /// height, its wall 0.06 em - two contours of opposite winding, so it fills as one path.
    fn tofu_elements(&self) -> Vec<PathEl> {
        let em = self.units_per_em as f64;
        let advance = self.missing_advance() as f64;
        let inset = (em * 0.1).min(advance * 0.25);
        let (x0, x1) = (inset, (advance - inset).max(inset + em * 0.15));
        let (y0, y1) = (0.0, (self.cap_height as f64).max(em * 0.3));
        let wall = em * 0.06;
        let rect = |x0: f64, y0: f64, x1: f64, y1: f64, clockwise: bool| {
            let points = if clockwise { [(x0, y0), (x0, y1), (x1, y1), (x1, y0)] } else { [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] };
            vec![PathEl::MoveTo(points[0].into()), PathEl::LineTo(points[1].into()), PathEl::LineTo(points[2].into()), PathEl::LineTo(points[3].into()), PathEl::ClosePath]
        };
        let mut elements = rect(x0, y0, x1, y1, false);
        elements.extend(rect(x0 + wall, y0 + wall, x1 - wall, y1 - wall, true));
        elements
    }

    /// The shape plan for a direction, a script and the kerning switch, made once and kept.
    pub(crate) fn plan(&self, direction: Direction, script: Option<Script>, kerning: bool) -> Rc<ShapePlan> {
        let key = (direction == Direction::RightToLeft, script.map(|s| u32::from_be_bytes(s.tag().to_be_bytes())).unwrap_or(0), kerning);
        if let Some(found) = self.plans.borrow().get(&key) {
            return found.clone();
        }
        let features: Vec<Feature> = if kerning { Vec::new() } else { vec![Feature::new(Tag::new(b"kern"), 0, ..)] };
        let plan = Rc::new(ShapePlan::new(&self.font, direction, script, None, &features));
        self.plans.borrow_mut().insert(key, plan.clone());
        plan
    }
}
