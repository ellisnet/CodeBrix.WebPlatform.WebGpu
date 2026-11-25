//! `BrushStrokesShape`: a [`BrushStrokeSet`] drawn as ONE instanced GPU draw at the session's
//! time - every stroke painted on, anchored, boiled and turned by its flow on the GPU, in its own
//! colour or in a colour picked from a live picture; optionally from a checkpoint cache when the
//! painting does not move; or a stroke-level transition between two sets at a fraction (no C#
//! equivalent).

use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use kurbo::Affine;

use crate::geometry::RectangleF;
use crate::paths::PathGeometry;
use crate::rendering::VectorScene;
use crate::shapes::drawing_shape::EncodeContext;
use crate::strokes::stroke_anchor::{self, StrokeAnchor};
use crate::strokes::stroke_boil::boil_epoch;
use crate::strokes::stroke_gpu::MAX_ANCHORS;
use crate::formations::{FormationArea, IArrivalOrder};
use crate::strokes::brush_stroke_morph::pair_fraction;
use crate::strokes::stroke_checkpoints::{checkpoint_refusal, CheckpointFacts};
use crate::strokes::{BrushStrokeMorph, BrushStrokeSet, BrushTips, StrokeBoil, StrokeCheckpoints, StrokeColor, StrokeColorSource, StrokeFlow};

static NEXT_MORPH_ID: AtomicU64 = AtomicU64::new(1);

/// How a [`BrushStrokesShape`] draws its set (no C# equivalent). The defaults draw the set as
/// the same [`BrushStrokeShape`](crate::shapes::BrushStrokeShape)s would look (smoother edges):
/// no offset, a straight cut at the brush's head, tips mirrored by seed, no boil, no flow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushStrokesOptions {
    /// The set's times are the session time minus this (seconds; replay a set later). 0.
    pub time_offset: f64,
    /// 0: the painted part ends in a straight cut across the stroke (BrushStrokeShape's rule);
    /// above 0: the head fades out over this fraction of the length, ending at the cut (0..1). 0.
    pub head_softness: f32,
    /// Mirror a stroke's tip across the stroke by its seed's low bit (more variety from few
    /// tips). true.
    pub tip_mirroring: bool,
    /// Boiling: every stroke jittered a few times a second. None.
    pub boil: Option<StrokeBoil>,
    /// Flow: every stroke turned by an animated noise field. None.
    pub flow: Option<StrokeFlow>,
}

impl Default for BrushStrokesOptions {
    fn default() -> BrushStrokesOptions {
        BrushStrokesOptions { time_offset: 0.0, head_softness: 0.0, tip_mirroring: true, boil: None, flow: None }
    }
}

impl BrushStrokesOptions {
    fn validate(&self) -> Result<(), String> {
        if !self.time_offset.is_finite() {
            return Err("A stroke shape's time offset must be finite seconds.".to_string());
        }
        if !(self.head_softness >= 0.0 && self.head_softness <= 1.0) {
            return Err("A stroke shape's head softness must be a fraction of the length, 0..1 (0 for a straight cut).".to_string());
        }
        if let Some(b) = &self.boil {
            b.validate()?;
        }
        if let Some(f) = &self.flow {
            f.validate()?;
        }
        Ok(())
    }
}

/// A [`BrushStrokeSet`] drawn as ONE instanced GPU draw at the session's time (no C#
/// equivalent): the set's records are uploaded once (shared by every shape that draws a clone of
/// the set), and per frame only a small block of numbers is sent - the time, the anchors'
/// transforms, the boil's redraw number - so the CPU's work per frame does not grow with the
/// number of strokes.
///
/// It is ONE element: it takes the properties every element has (transform, opacity, time
/// window, effects, its group's and layer's masks, bindings of position, rotation, scale, anchor
/// and opacity) and uses no nesting level of its own. Paint order inside the set is record order.
/// In ownership maps and hard-edged masks the whole set is one element whose area is the union of
/// its strokes' CPU twins at the session's time ([`BrushStrokesShape::coverage_areas_at`]).
///
/// COLOUR: each record's own colour, or PICKED every frame on the GPU from a live picture at each
/// stroke's position ([`BrushStrokesShape::with_color`], [`StrokeColor`]).
///
/// THE LOOK: a session drawn as a line drawing leaves stroke sets out; drawn flat, each stroke is
/// its own (or its picked) colour without its tip.
///
/// CHECKPOINTS ([`BrushStrokesShape::with_checkpoints`], off by default): a painting that does
/// not move on screen keeps saved states of its canvas on the GPU, so a frame draws only the
/// strokes after the nearest one - every frame still IDENTICAL, pixel for pixel, to the frame
/// drawn whole (see [`StrokeCheckpoints`]).
///
/// A STROKE-LEVEL TRANSITION ([`BrushStrokesShape::morph`]): two sets paired stroke by stroke,
/// drawn at a fraction (bind `AnimatedProperty::MorphFraction`; judged by eye, see
/// [`BrushStrokeMorph`]).
///
/// LIMITS: the GPU draws the analytic lozenge (BrushStrokeShape's formula; its CPU twin
/// [`BrushStrokeRecord::to_shape`](crate::strokes::BrushStrokeRecord::to_shape) is a polygon of the
/// same formula, so edges differ slightly); a tip is stretched over the stroke; at most
/// [`BrushStrokesShape::MAX_ANCHORS`] anchors; sets are immutable (a change is a new set and a
/// new upload).
#[derive(Clone, Debug, PartialEq)]
pub struct BrushStrokesShape {
    set: BrushStrokeSet,
    tips: Option<BrushTips>,
    anchors: Vec<StrokeAnchor>,
    color: StrokeColor,
    options: BrushStrokesOptions,
    checkpoints: Option<StrokeCheckpoints>,
    morph: Option<MorphState>,
    morph_fraction: f32,
}

/// A stroke-level transition as a shape holds it: the morph, each slot's delay under the spread
/// (0..1), the spread, and the identity of this morph-and-spread on the GPU (crate-internal).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MorphState {
    pub(crate) id: u64,
    pub(crate) morph: BrushStrokeMorph,
    pub(crate) delays: Rc<[f32]>,
    pub(crate) spread: f32,
}

/// A transition as one frame draws it: the shape's morph state and its fraction.
#[derive(Clone, Debug)]
pub(crate) struct MorphDraw {
    pub(crate) state: MorphState,
    pub(crate) fraction: f32,
}

/// One stroke draw as the scene records it (crate-internal): the set, the tips, the anchors'
/// transforms this frame, the draw's time and opacity, and the options resolved for the frame.
#[derive(Clone, Debug)]
pub(crate) struct StrokeDraw {
    pub(crate) set: BrushStrokeSet,
    pub(crate) tips: Option<BrushTips>,
    pub(crate) anchors: Vec<Affine>,
    pub(crate) time: f32,
    pub(crate) opacity: f32,
    pub(crate) head_softness: f32,
    pub(crate) mirror: bool,
    /// The boil and this frame's redraw number.
    pub(crate) boil: Option<(StrokeBoil, i64)>,
    pub(crate) flow: Option<StrokeFlow>,
    /// Where the colours come from.
    pub(crate) color: StrokeColor,
    /// The checkpoint cache, when asked for and usable (see [`StrokeCheckpoints`]).
    pub(crate) checkpoints: Option<StrokeCheckpoints>,
    /// Checkpoints asked for that the shape can never use (counted as a miss each frame).
    pub(crate) checkpoints_refused: bool,
    /// A stroke-level transition at its fraction.
    pub(crate) morph: Option<MorphDraw>,
}

impl BrushStrokesShape {
    /// The most anchors a shape has.
    pub const MAX_ANCHORS: usize = MAX_ANCHORS;

    /// A set drawn with options (no tips, no anchors: every stroke in the set's coordinates). The
    /// error says which option is unusable.
    pub fn new(set: BrushStrokeSet, options: BrushStrokesOptions) -> Result<BrushStrokesShape, String> {
        options.validate()?;
        Ok(BrushStrokesShape { set, tips: None, anchors: Vec::new(), color: StrokeColor::Own, options, checkpoints: None, morph: None, morph_fraction: 0.0 })
    }

    /// A STROKE-LEVEL TRANSITION between the two sets of a [`BrushStrokeMorph`], drawn at the
    /// shape's morph fraction (0 = the first set exactly, 1 = the second exactly; 0 at first -
    /// bind `AnimatedProperty::MorphFraction` to it, as for a morphing path, or set it with
    /// [`BrushStrokesShape::set_morph_fraction`]). Every pair slides, turns, resizes and
    /// recolours (in Oklab) from one stroke to the other; a stroke without a partner paints
    /// itself off or on. `spread` staggers the pairs: an arrival order (e.g. a directional sweep)
    /// prepared over the box of the pairs' midpoints with the morph's seed ranks them, and with
    /// spread `s` (0..1, below 1) the pair ranked `d` (0..1) moves over `s d .. s d + 1 - s` of the
    /// fraction; None moves every pair together. Tips, anchors, colour and options are the
    /// shape's (with_tips, with_anchors, with_color as for any set); checkpoints are not used.
    /// JUDGED BY EYE: see [`BrushStrokeMorph`] for the limits (greedy pairing; very different
    /// paintings look like a shuffle). The error says what is wrong with the options or the
    /// spread.
    pub fn morph(morph: BrushStrokeMorph, options: BrushStrokesOptions, spread: Option<(Box<dyn IArrivalOrder>, f32)>) -> Result<BrushStrokesShape, String> {
        options.validate()?;
        let n = morph.len();
        let (delays, amount) = match spread {
            None => (vec![0.0; n], 0.0),
            Some((mut order, amount)) => {
                if !(0.0..1.0).contains(&amount) {
                    return Err("A stroke transition's spread must be a fraction, 0 or more and below 1 (0 moves every pair together).".to_string());
                }
                (spread_delays(&morph, order.as_mut()), amount)
            }
        };
        let state = MorphState { id: NEXT_MORPH_ID.fetch_add(1, Ordering::Relaxed), morph: morph.clone(), delays: delays.into(), spread: amount };
        Ok(BrushStrokesShape { set: morph.from().clone(), tips: None, anchors: Vec::new(), color: StrokeColor::Own, options, checkpoints: None, morph: Some(state), morph_fraction: 0.0 })
    }

    /// The transition this shape draws, when it was made by [`BrushStrokesShape::morph`].
    pub fn transition(&self) -> Option<&BrushStrokeMorph> {
        self.morph.as_ref().map(|m| &m.morph)
    }

    /// The transition's fraction, 0..1 (C#'s `MorphFraction`, here for stroke sets: Not in C#).
    pub fn morph_fraction(&self) -> f32 {
        self.morph_fraction
    }

    /// Sets the transition's fraction (clamped to 0..1; NaN is 0). A shape that is not a
    /// transition keeps the value and draws as before.
    pub fn set_morph_fraction(&mut self, fraction: f32) {
        self.morph_fraction = if fraction.is_nan() { 0.0 } else { fraction.clamp(0.0, 1.0) };
    }

    /// The same shape at another transition fraction (see [`BrushStrokesShape::set_morph_fraction`]).
    pub fn with_morph_fraction(mut self, fraction: f32) -> BrushStrokesShape {
        self.set_morph_fraction(fraction);
        self
    }

    /// The same shape with a CHECKPOINT CACHE (Some) or without (None, the default): see
    /// [`StrokeCheckpoints`]. Every frame stays identical, pixel for pixel, to the frame drawn
    /// whole; the cache is used only while that holds and the set is drawn whole otherwise,
    /// automatically ([`BrushStrokesShape::checkpoint_refusal`] says when it can never be used).
    /// OFF BY DEFAULT because it costs GPU memory the crate cannot weigh for the application - 16
    /// bytes per canvas pixel per checkpoint (59 MB at 2560 x 1440) - and a texture copy per frame
    /// that pays only for a large painting that holds still. The error says which value is
    /// unusable.
    pub fn with_checkpoints(mut self, checkpoints: Option<StrokeCheckpoints>) -> Result<BrushStrokesShape, String> {
        if let Some(c) = &checkpoints {
            c.validate()?;
        }
        self.checkpoints = checkpoints;
        Ok(self)
    }

    /// The checkpoint cache asked for.
    pub fn checkpoints(&self) -> Option<StrokeCheckpoints> {
        self.checkpoints
    }

    /// Why this shape's checkpoints can never be used (it is then drawn whole every frame; a
    /// session reports it once), or None: boil, flow, an anchor following an element, colours
    /// picked from a picture target or an application's texture, a transition. None also when no
    /// checkpoints are asked for. (Whether a frame can use them besides - the painting holding
    /// still - is decided each frame; see [`StrokeCheckpoints`].)
    pub fn checkpoint_refusal(&self) -> Option<String> {
        checkpoint_refusal(&self.checkpoint_facts()).map(|r| format!("A stroke set asks for checkpoints but cannot use them: {r}; it is drawn whole every frame."))
    }

    fn checkpoint_facts(&self) -> CheckpointFacts {
        let live_colour = matches!(&self.color, StrokeColor::Picked(p) if !matches!(p.source, StrokeColorSource::Picture(_)));
        CheckpointFacts {
            asked: self.checkpoints.is_some(),
            boil: self.options.boil.is_some_and(|b| b.rate > 0.0),
            flow: self.options.flow.is_some(),
            followed_anchor: self.anchors.iter().any(|a| matches!(a, StrokeAnchor::FollowElement(_))),
            live_colour,
            morph: self.morph.is_some(),
        }
    }

    /// The same shape painted with tips (a record's `tip` is 1-based into them; 0, or an index
    /// past them, is a solid lozenge).
    pub fn with_tips(mut self, tips: BrushTips) -> BrushStrokesShape {
        self.tips = Some(tips);
        self
    }

    /// The same shape with anchors (a record's `anchors` index them; an index past the list is
    /// the identity). The error says when there are more than [`BrushStrokesShape::MAX_ANCHORS`].
    pub fn with_anchors(mut self, anchors: Vec<StrokeAnchor>) -> Result<BrushStrokesShape, String> {
        if anchors.len() > MAX_ANCHORS {
            return Err(format!("A stroke shape has at most {MAX_ANCHORS} anchors ({} were given): group strokes that move together under one anchor.", anchors.len()));
        }
        self.anchors = anchors;
        Ok(self)
    }

    /// The same shape with its colours from a source ([`StrokeColor::Picked`]) or its records
    /// ([`StrokeColor::Own`], the default). The error says what is wrong with the pickup (taps,
    /// own weight, place, rules).
    pub fn with_color(mut self, color: StrokeColor) -> Result<BrushStrokesShape, String> {
        if let StrokeColor::Picked(p) = &color {
            p.validate()?;
        }
        self.color = color;
        Ok(self)
    }

    /// Where the colours come from.
    pub fn color(&self) -> &StrokeColor {
        &self.color
    }

    /// The same shape with other options. The error says which option is unusable.
    pub fn with_options(mut self, options: BrushStrokesOptions) -> Result<BrushStrokesShape, String> {
        options.validate()?;
        self.options = options;
        Ok(self)
    }

    /// The set.
    pub fn set(&self) -> &BrushStrokeSet {
        &self.set
    }

    /// The tips, when it has them.
    pub fn tips(&self) -> Option<&BrushTips> {
        self.tips.as_ref()
    }

    /// The anchors.
    pub fn anchors(&self) -> &[StrokeAnchor] {
        &self.anchors
    }

    /// The options.
    pub fn options(&self) -> &BrushStrokesOptions {
        &self.options
    }

    /// The box of every whole stroke in the shape's coordinates: the set's anchor boxes through
    /// their FIXED anchors (a followed element's box is given untransformed - it moves with the
    /// element). Boil and flow may move strokes a little beyond it.
    pub fn get_bounds(&self) -> RectangleF {
        match &self.morph {
            None => self.bounds_of(&self.set),
            Some(m) => union(self.bounds_of(m.morph.from()), self.bounds_of(m.morph.to())),
        }
    }

    fn bounds_of(&self, set: &BrushStrokeSet) -> RectangleF {
        if self.anchors.is_empty() || set.anchor_bounds().is_empty() {
            return set.bounds();
        }
        let mut out: Option<RectangleF> = None;
        for (i, b) in set.anchor_bounds().iter().enumerate() {
            if b.width <= 0.0 && b.height <= 0.0 {
                continue;
            }
            let r = match self.anchors.get(i) {
                Some(StrokeAnchor::Fixed(t)) if !t.is_identity() => t.transform_bounds(*b),
                _ => *b,
            };
            out = Some(match out {
                None => r,
                Some(o) => union(o, r),
            });
        }
        out.unwrap_or_else(|| set.bounds())
    }

    /// The area the painted strokes cover at a session time, in the shape's coordinates: each
    /// started stroke's CPU twin outline ([`BrushStrokeRecord::to_shape`](crate::strokes::BrushStrokeRecord::to_shape)
    /// at the set's time) through its FIXED anchors, blended (a followed element's anchor counts as
    /// the identity here; boil and flow are not applied). O(strokes): for exports, not per frame.
    pub fn coverage_areas_at(&self, seconds: f64) -> Vec<PathGeometry> {
        let t = seconds - self.options.time_offset;
        let fixed: Vec<Affine> = self
            .anchors
            .iter()
            .map(|a| match a {
                StrokeAnchor::Fixed(m) => m.to_matrix(),
                StrokeAnchor::FollowElement(_) => Affine::IDENTITY,
            })
            .collect();
        // The strokes at the time (a transition: each slot's stroke at its own fraction).
        let records: Vec<(crate::strokes::BrushStrokeRecord, f32)> = match &self.morph {
            None => self.set.records().iter().map(|r| (*r, 1.0)).collect(),
            Some(m) => (0..m.morph.len()).filter_map(|k| m.morph.record_at(k, pair_fraction(self.morph_fraction, m.spread, m.delays[k]))).collect(),
        };
        let mut areas = Vec::new();
        for (r, painted) in &records {
            let progress = r.progress_at(t) * painted;
            if !(progress > 0.0) {
                continue;
            }
            let Ok(shape) = r.to_shape(t).map(|s| s.with_progress(progress)) else { continue };
            let outline = shape.get_outline();
            if outline.is_empty() {
                continue;
            }
            let m = stroke_anchor::blend(stroke_anchor::anchor_at(&fixed, r.anchors[0]), stroke_anchor::anchor_at(&fixed, r.anchors[1]), r.anchor_blend);
            areas.push(if m == Affine::IDENTITY { outline } else { outline.transformed(m) });
        }
        areas
    }

    /// Records the draw: the anchors resolved at the session's time, the boil's redraw number.
    pub(crate) fn encode(&self, scene: &mut VectorScene, transform: Affine, opacity: f32, ctx: &EncodeContext<'_>) {
        if self.set.is_empty() {
            return;
        }
        let time = ctx.time - self.options.time_offset;
        let anchors = stroke_anchor::resolve_anchors(&self.anchors, ctx.layers, &mut |m| ctx.reports.borrow_mut().report(m));
        let boil = self.options.boil.filter(|b| b.rate > 0.0).map(|b| (b, boil_epoch(time, b.rate)));
        // A picture target read before its layer drew it this frame shows the previous frame's.
        if let StrokeColor::Picked(p) = &self.color {
            if let StrokeColorSource::Target(t) = &p.source {
                use crate::rendering::picture_target::{canvas_frame, early_read_report, read_too_early};
                if read_too_early(t.drawn_frame(), canvas_frame()) {
                    ctx.reports.borrow_mut().report(early_read_report(t));
                }
            }
        }
        // Checkpoints asked for that can never be used are reported once (the set is drawn whole).
        let refusal = self.checkpoint_refusal();
        let checkpoints_refused = refusal.is_some();
        let checkpoints = match refusal {
            Some(message) => {
                ctx.reports.borrow_mut().report(message);
                None
            }
            None => self.checkpoints,
        };
        let morph = self.morph.as_ref().map(|m| MorphDraw { state: m.clone(), fraction: self.morph_fraction });
        let draw = StrokeDraw { set: self.set.clone(), tips: self.tips.clone(), anchors, time: time as f32, opacity, head_softness: self.options.head_softness, mirror: self.options.tip_mirroring, boil, flow: self.options.flow, color: self.color.clone(), checkpoints, checkpoints_refused, morph };
        scene.draw_strokes(transform, draw);
    }
}

fn union(o: RectangleF, r: RectangleF) -> RectangleF {
    if o.width <= 0.0 && o.height <= 0.0 {
        return r;
    }
    if r.width <= 0.0 && r.height <= 0.0 {
        return o;
    }
    let (l, t) = (o.x.min(r.x), o.y.min(r.y));
    RectangleF::new(l, t, o.right().max(r.right()) - l, o.bottom().max(r.bottom()) - t)
}

/// Each slot's delay under a spread: its rank (0..1) by the arrival order's key at the slot's
/// midpoint (between its two strokes' centres), the order prepared over the box of the midpoints
/// with the morph's seed; a key that is not a number ranks last; ties by slot.
fn spread_delays(morph: &BrushStrokeMorph, order: &mut dyn IArrivalOrder) -> Vec<f32> {
    let n = morph.len();
    if n == 0 {
        return Vec::new();
    }
    let mids: Vec<(f32, f32)> = morph
        .pairs()
        .iter()
        .map(|p| {
            let (a, b) = morph.ends(p);
            ((a.center.x + b.center.x) / 2.0, (a.center.y + b.center.y) / 2.0)
        })
        .collect();
    let (mut left, mut top, mut right, mut bottom) = (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for (x, y) in &mids {
        left = left.min(*x);
        top = top.min(*y);
        right = right.max(*x);
        bottom = bottom.max(*y);
    }
    let bounds = RectangleF::new(left - 0.5, top - 0.5, (right - left) + 1.0, (bottom - top) + 1.0);
    let keys: Vec<f32> = match FormationArea::from_geometry(&PathGeometry::rectangle(bounds), None) {
        Ok(area) => {
            order.prepare(&area, morph.seed());
            mids.iter().map(|(x, y)| order.get_key(*x, *y, area.distance_to_edge(*x, *y))).collect()
        }
        Err(_) => vec![0.0; n],
    };
    let mut ranked: Vec<u32> = (0..n as u32).collect();
    ranked.sort_by(|a, b| {
        let (ka, kb) = (keys[*a as usize], keys[*b as usize]);
        match (ka.is_nan(), kb.is_nan()) {
            (true, true) => a.cmp(b),
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal).then(a.cmp(b)),
        }
    });
    let mut delays = vec![0.0; n];
    let last = (n - 1).max(1) as f32;
    for (rank, slot) in ranked.iter().enumerate() {
        delays[*slot as usize] = rank as f32 / last;
    }
    delays
}
