//! `PaintingTimeline`: when each stroke of a painting is painted - stages ("wash", "block-in",
//! "detail", ...) each with its span, the order its strokes come in and how long one stroke
//! takes - turned into a stroke set whose every stroke has its start and end (no C# equivalent).
//! Planning-time CPU work, deterministic.

use crate::animation::{AnimationTimeline, EasingKind};
use crate::formations::{ArrivalRateProfile, ArrivalSchedule, FormationArea, IArrivalOrder};
use crate::geometry::RectangleF;
use crate::painting::color_mix::color_to_oklab;
use crate::paths::PathGeometry;
use crate::planes::DrawingRandom;
use crate::strokes::{BrushStrokeRecord, BrushStrokeSet};

/// The order a stage's strokes are painted in (no C# equivalent). Smaller keys come first; a key
/// that is NaN or infinite drops the stroke.
pub enum StrokeOrder {
    /// By position: any formation order (random, centre-out, directional sweep, heap, your own),
    /// prepared with the box of the stage's stroke centres and the schedule's seed, keyed at each
    /// stroke's centre.
    Positional(Box<dyn IArrivalOrder>),
    /// By the stroke's own colour, darkest (Oklab L) first: a painter's dark-to-light build-up.
    DarkToLight,
    /// By the stroke's own colour, lightest first.
    LightToDark,
    /// As given (the records' order).
    AsGiven,
    /// Your own key per record.
    Key(Box<dyn Fn(&BrushStrokeRecord) -> f32>),
}

impl std::fmt::Debug for StrokeOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            StrokeOrder::Positional(_) => "Positional",
            StrokeOrder::DarkToLight => "DarkToLight",
            StrokeOrder::LightToDark => "LightToDark",
            StrokeOrder::AsGiven => "AsGiven",
            StrokeOrder::Key(_) => "Key",
        })
    }
}

/// One stage of a [`PaintingTimeline`] (no C# equivalent).
#[derive(Debug)]
pub struct PaintingStage {
    /// The stage's name ("wash", "block-in", "detail", "highlights"): its cue's name. Not empty,
    /// unique in the timeline.
    pub name: String,
    /// When its first stroke starts, session seconds.
    pub first_time: f64,
    /// When its last stroke starts (not before `first_time`).
    pub last_time: f64,
    /// How the starts bunch over the span ([`ArrivalSchedule`]'s rate; None: the default
    /// profile).
    pub profile: Option<ArrivalRateProfile>,
    /// The order its strokes come in.
    pub order: StrokeOrder,
    /// How long one stroke takes, seconds: each stroke's drawn uniformly in this range (min, max;
    /// both 0 or more, min not above max; (0, 0) = each stroke appears whole).
    pub stroke_seconds: (f64, f64),
}

impl PaintingStage {
    /// A stage with the default profile, its strokes as given, each taking `stroke_seconds`.
    pub fn new(name: &str, first_time: f64, last_time: f64, stroke_seconds: (f64, f64)) -> PaintingStage {
        PaintingStage { name: name.to_string(), first_time, last_time, profile: None, order: StrokeOrder::AsGiven, stroke_seconds }
    }

    /// The same stage with another order.
    pub fn with_order(mut self, order: StrokeOrder) -> PaintingStage {
        self.order = order;
        self
    }

    /// The same stage with a rate profile.
    pub fn with_profile(mut self, profile: ArrivalRateProfile) -> PaintingStage {
        self.profile = Some(profile);
        self
    }
}

/// A painting painted as the viewer watches (no C# equivalent): stages, each giving its strokes
/// their start and end, merged into ONE stroke set in painting order. The frame at a time is
/// then simply every stroke at its progress at that time - so pausing, seeking back and drawing
/// frames in any order need nothing more (seek the host clock from the page's own scrub control).
#[derive(Debug, Default)]
pub struct PaintingTimeline {
    stages: Vec<PaintingStage>,
}

impl PaintingTimeline {
    /// An empty timeline.
    pub fn new() -> PaintingTimeline {
        PaintingTimeline { stages: Vec::new() }
    }

    /// The stages, in the order added.
    pub fn stages(&self) -> &[PaintingStage] {
        &self.stages
    }

    /// Adds a stage. The error says what is wrong (an empty or repeated name, times that are
    /// not finite or out of order, stroke seconds out of range, an unusable profile).
    pub fn add_stage(&mut self, stage: PaintingStage) -> Result<&mut PaintingTimeline, String> {
        let name = stage.name.trim();
        if name.is_empty() {
            return Err("A painting stage needs a name (it names the stage's cue).".to_string());
        }
        if self.stages.iter().any(|s| s.name.trim() == name) {
            return Err(format!("A painting stage named '{name}' already exists: give each stage its own name."));
        }
        if !stage.first_time.is_finite() || !stage.last_time.is_finite() || stage.last_time < stage.first_time {
            return Err(format!("Stage '{name}': the first and last times must be finite, the last not before the first."));
        }
        let (low, high) = stage.stroke_seconds;
        if !(low >= 0.0) || !high.is_finite() || high < low {
            return Err(format!("Stage '{name}': the stroke seconds must be finite, 0 or more, the first not above the second."));
        }
        // The profile's own checks, with ArrivalSchedule's messages.
        ArrivalSchedule::new(stage.first_time, stage.last_time, stage.profile).map_err(|e| format!("Stage '{name}': {e}"))?;
        self.stages.push(stage);
        Ok(self)
    }

    /// Gives every stroke of every stage its start and end, and returns them as one set: each
    /// stage's strokes ranked by its order (ties: the records' order), the starts spaced by the
    /// stage's [`ArrivalSchedule`] (rank r starts at the schedule's r-th time), each duration
    /// drawn uniformly from `stroke_seconds` by `DrawingRandom::for_key(seed, stage index)` in
    /// rank order; then ALL strokes sorted by start (ties: stage, then rank) so later paint lies
    /// over earlier. `strokes[i]` are stage i's records (for example the bands of
    /// [`StrokeLayout::fill`](crate::painting::StrokeLayout::fill)); their other fields are kept.
    /// Strokes whose key is NaN or infinite are dropped. Takes `&mut self` because a positional
    /// order is prepared for each schedule. Deterministic. The error says what is wrong.
    pub fn schedule(&mut self, strokes: Vec<Vec<BrushStrokeRecord>>, seed: i64) -> Result<BrushStrokeSet, String> {
        if strokes.len() != self.stages.len() {
            return Err(format!("Give one list of strokes per stage: the timeline has {} stages and {} lists were given.", self.stages.len(), strokes.len()));
        }
        // (start, stage, rank, record)
        let mut all: Vec<(f64, usize, usize, BrushStrokeRecord)> = Vec::new();
        for (stage_index, (stage, records)) in self.stages.iter_mut().zip(strokes).enumerate() {
            let keys = stage_keys(&mut stage.order, &records, seed);
            let mut ranked: Vec<(f32, usize)> = keys.into_iter().enumerate().filter(|(_, k)| k.is_finite()).map(|(i, k)| (k, i)).collect();
            ranked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1)));
            let times = ArrivalSchedule::new(stage.first_time, stage.last_time, stage.profile)?.get_times(ranked.len());
            let mut random = DrawingRandom::for_key(seed, stage_index as i64);
            let (low, high) = stage.stroke_seconds;
            for (rank, &(_, index)) in ranked.iter().enumerate() {
                let start = times[rank];
                let duration = random.uniform(low, high);
                let mut record = records[index];
                record.start = start as f32;
                record.end = (start + duration) as f32;
                all.push((start, stage_index, rank, record));
            }
        }
        all.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        BrushStrokeSet::new(all.into_iter().map(|(_, _, _, r)| r).collect())
    }

    /// One cue per stage on a timeline, named as the stage, from its first start to its last
    /// start plus the longest stroke time (when every stroke of the stage has surely ended),
    /// linear - for a scrub bar's stage marks and named points. Nothing is added when a name is
    /// already on the timeline (the error names it).
    pub fn add_cues(&self, timeline: &mut AnimationTimeline) -> Result<(), String> {
        for stage in &self.stages {
            let name = stage.name.trim();
            if timeline.cues().iter().any(|c| c.name() == name) {
                return Err(format!("The timeline already has a cue named '{name}': rename the stage or the cue."));
            }
        }
        for stage in &self.stages {
            timeline.add_cue(stage.name.trim(), stage.first_time, stage.last_time + stage.stroke_seconds.1, EasingKind::Linear)?;
        }
        Ok(())
    }
}

/// The key of each record by a stage's order (smaller first).
fn stage_keys(order: &mut StrokeOrder, records: &[BrushStrokeRecord], seed: i64) -> Vec<f32> {
    match order {
        StrokeOrder::AsGiven => (0..records.len()).map(|i| i as f32).collect(),
        StrokeOrder::DarkToLight => records.iter().map(|r| color_to_oklab(r.color)[0]).collect(),
        StrokeOrder::LightToDark => records.iter().map(|r| -color_to_oklab(r.color)[0]).collect(),
        StrokeOrder::Key(key) => records.iter().map(&**key).collect(),
        StrokeOrder::Positional(order) => {
            if records.is_empty() {
                return Vec::new();
            }
            let (mut left, mut top, mut right, mut bottom) = (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
            for r in records {
                left = left.min(r.center.x);
                top = top.min(r.center.y);
                right = right.max(r.center.x);
                bottom = bottom.max(r.center.y);
            }
            // The box of the centres, grown by half a unit so a single stroke or a line of them
            // still has an area.
            let bounds = RectangleF::new(left - 0.5, top - 0.5, (right - left) + 1.0, (bottom - top) + 1.0);
            let Ok(area) = FormationArea::from_geometry(&PathGeometry::rectangle(bounds), None) else {
                return vec![f32::NAN; records.len()];
            };
            order.prepare(&area, seed);
            records.iter().map(|r| order.get_key(r.center.x, r.center.y, area.distance_to_edge(r.center.x, r.center.y))).collect()
        }
    }
}
