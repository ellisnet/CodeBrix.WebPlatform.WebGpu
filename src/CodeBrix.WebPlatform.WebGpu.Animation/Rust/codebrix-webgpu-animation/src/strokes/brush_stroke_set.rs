//! `BrushStrokeSet`: brushstrokes in painting order, checked once and shared by clones (no C#
//! equivalent).

use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::geometry::RectangleF;
use crate::strokes::BrushStrokeRecord;

static NEXT_SET_ID: AtomicU64 = AtomicU64::new(1);

/// Brushstrokes in PAINTING ORDER (later over earlier), checked once, shared by clones (an
/// `Rc`), uploaded to the GPU once per set: every shape that draws a clone of the set uses that
/// one upload, as pictures share their pixels (no C# equivalent).
///
/// A set is immutable: a change - one stroke edited, a new keyframe painting - is a new set (and
/// a new upload). Each set made by [`BrushStrokeSet::new`] has its own identity; clones share it
/// and are equal, two sets made separately are not equal even with the same records.
#[derive(Clone, Debug, PartialEq)]
pub struct BrushStrokeSet {
    id: u64,
    records: Rc<[BrushStrokeRecord]>,
    bounds: RectangleF,
    anchor_bounds: Rc<[RectangleF]>,
    first_start: f32,
    /// Per record, the earliest start of it and every record after it (non-decreasing): how many
    /// records the GPU must draw at a time is a binary search (crate-internal).
    suffix_start: Rc<[f32]>,
    /// Per record, the latest end of it and every record before it (non-decreasing): how many
    /// records from the first are WHOLE at a time is a binary search (the checkpoint cache's
    /// prefix; crate-internal).
    prefix_end: Rc<[f32]>,
    /// The largest distance from a stroke's centre to any point of it, set units (how far a
    /// stroke can reach when it is turned about its centre).
    reach: f32,
}

impl BrushStrokeSet {
    /// A set of records in painting order. Every record is checked: the error says which record
    /// is unusable and why (BrushStrokeShape's checks, start and end not finite or the end
    /// before the start, the load or the anchor blend outside 0..1). An empty set is allowed
    /// (it draws nothing).
    pub fn new(records: Vec<BrushStrokeRecord>) -> Result<BrushStrokeSet, String> {
        let mut bounds: Option<(f32, f32, f32, f32)> = None;
        let mut anchors: Vec<Option<(f32, f32, f32, f32)>> = Vec::new();
        let mut first_start = f32::INFINITY;
        let mut reach = 0f32;
        for (i, record) in records.iter().enumerate() {
            record.validate().map_err(|message| format!("Brushstroke record {i} is unusable: {message}"))?;
            let b = record.bounds();
            let edges = (b.x, b.y, b.right(), b.bottom());
            grow(&mut bounds, edges);
            // A stroke between two anchors moves with both, so it counts in both anchors' boxes.
            let used: &[(u16, bool)] = &[(record.anchors[0], record.anchor_blend < 1.0), (record.anchors[1], record.anchor_blend > 0.0)];
            for &(anchor, uses) in used {
                if !uses {
                    continue;
                }
                let index = anchor as usize;
                if anchors.len() <= index {
                    anchors.resize(index + 1, None);
                }
                grow(&mut anchors[index], edges);
            }
            first_start = first_start.min(record.start);
            let bow = (record.bend * record.length).abs();
            reach = reach.max(((record.length / 2.0).powi(2) + (bow + record.width / 2.0).powi(2)).sqrt());
        }
        let mut suffix_start = vec![f32::INFINITY; records.len()];
        let mut earliest = f32::INFINITY;
        for (i, record) in records.iter().enumerate().rev() {
            earliest = earliest.min(record.start);
            suffix_start[i] = earliest;
        }
        let mut prefix_end = Vec::with_capacity(records.len());
        let mut latest = f32::NEG_INFINITY;
        for record in &records {
            latest = latest.max(record.end);
            prefix_end.push(latest);
        }
        let to_rect = |e: Option<(f32, f32, f32, f32)>| match e {
            Some((l, t, r, b)) => RectangleF::new(l, t, r - l, b - t),
            None => RectangleF::default(),
        };
        Ok(BrushStrokeSet {
            id: NEXT_SET_ID.fetch_add(1, Ordering::Relaxed),
            records: records.into(),
            bounds: to_rect(bounds),
            anchor_bounds: anchors.into_iter().map(to_rect).collect::<Vec<_>>().into(),
            first_start,
            suffix_start: suffix_start.into(),
            prefix_end: prefix_end.into(),
            reach,
        })
    }

    /// The set's identity: the same for every clone, different for every set made (what the
    /// renderer keys the set's one upload by).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The records, in painting order.
    pub fn records(&self) -> &[BrushStrokeRecord] {
        &self.records
    }

    /// The number of strokes.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// True when the set has no strokes.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The box of every whole stroke (bend included; see [`BrushStrokeRecord::bounds`]), set
    /// coordinates - before any anchor moves the strokes. Empty for an empty set.
    pub fn bounds(&self) -> RectangleF {
        self.bounds
    }

    /// Per anchor index, the box of the whole strokes that follow that anchor (a stroke between
    /// two anchors counts in both; one with blend 0 only in its first, with blend 1 only in its
    /// second), set coordinates before the anchor's transform. The list runs to the highest anchor
    /// index any stroke uses; an index no stroke uses has an empty box (0 x 0) - skip it.
    pub fn anchor_bounds(&self) -> &[RectangleF] {
        &self.anchor_bounds
    }

    /// The earliest start of any stroke (session seconds, [`BrushStrokeRecord::ALWAYS`] when a
    /// stroke is always whole; infinity for an empty set): before it the set shows nothing.
    pub fn first_start(&self) -> f32 {
        self.first_start
    }

    /// How many records, from the first, the GPU must draw at a time: past the last record that
    /// has started (`start <= time`) nothing shows (a binary search; records not yet started
    /// before it draw nothing either). Crate-internal.
    pub(crate) fn drawn_count(&self, seconds: f64) -> usize {
        // suffix_start[i] <= t for exactly the records up to the last started one.
        self.suffix_start.partition_point(|s| (*s as f64) <= seconds)
    }

    /// How many records, from the first, are WHOLE at a time given as the GPU gets it (f32): the
    /// longest prefix whose every record has ended (`end <= time`, the shader's rule for a
    /// stroke painted to 1 - an exact comparison, so the CPU and the GPU agree on every record).
    /// A record that has ended after one still being painted is not counted (the prefix stops at
    /// the first unfinished record). NaN gives 0. Crate-internal (the checkpoint cache).
    pub(crate) fn whole_count(&self, time: f32) -> usize {
        self.prefix_end.partition_point(|e| *e <= time)
    }

    /// The largest distance from a stroke's centre to any point of it, set units.
    pub(crate) fn reach(&self) -> f32 {
        self.reach
    }

    /// How many strokes have started at a time (`start <= time`).
    pub fn started_count(&self, seconds: f64) -> usize {
        self.records.iter().filter(|r| r.has_started(seconds)).count()
    }
}

fn grow(edges: &mut Option<(f32, f32, f32, f32)>, (l, t, r, b): (f32, f32, f32, f32)) {
    *edges = Some(match *edges {
        Some((l0, t0, r0, b0)) => (l0.min(l), t0.min(t), r0.max(r), b0.max(b)),
        None => (l, t, r, b),
    });
}
