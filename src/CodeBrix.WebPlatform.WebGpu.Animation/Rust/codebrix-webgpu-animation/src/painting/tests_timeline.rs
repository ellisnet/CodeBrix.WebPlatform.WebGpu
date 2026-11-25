//! Tests of the painting timeline: stage spans, starts on the schedule's quantiles, the orders
//! (dark to light by Oklab L, positional, keys, dropped strokes), durations per seed, the final
//! sort with its ties, determinism and the cues.

use crate::animation::AnimationTimeline;
use crate::color::Color;
use crate::formations::{ArrivalRateProfile, ArrivalSchedule, DirectionalArrivalOrder};
use crate::geometry::PointF;
use crate::painting::color_mix::color_to_oklab;
use crate::painting::{PaintingStage, PaintingTimeline, StrokeOrder};
use crate::strokes::BrushStrokeRecord;

fn stroke(x: f32, grey: u8) -> BrushStrokeRecord {
    BrushStrokeRecord::new(PointF::new(x, 10.0), 10.0, 3.0, 0.0, Color::from_rgb(grey, grey, grey))
}

fn row(n: usize) -> Vec<BrushStrokeRecord> {
    (0..n).map(|i| stroke(i as f32 * 10.0, ((i * 37) % 256) as u8)).collect()
}

#[test]
fn starts_follow_the_schedule_and_stay_in_the_span() {
    let mut timeline = PaintingTimeline::new();
    let profile = ArrivalRateProfile { start_rate: 0.5, ..ArrivalRateProfile::default() };
    timeline.add_stage(PaintingStage::new("wash", 2.0, 12.0, (1.0, 1.0)).with_profile(profile)).unwrap();
    let set = timeline.schedule(vec![row(25)], 7).unwrap();
    let times = ArrivalSchedule::new(2.0, 12.0, Some(profile)).unwrap().get_times(25);
    assert_eq!(set.len(), 25);
    for (i, r) in set.records().iter().enumerate() {
        assert_eq!(r.start, times[i] as f32);
        assert_eq!(r.end, (times[i] + 1.0) as f32);
        assert!(r.start >= 2.0 && r.start <= 12.0);
        // As given: rank i is record i.
        assert_eq!(r.center.x, i as f32 * 10.0);
    }
}

#[test]
fn dark_to_light_ranks_by_oklab_lightness() {
    let records = vec![stroke(0.0, 200), stroke(1.0, 20), stroke(2.0, 120), BrushStrokeRecord { color: Color::from_rgb(0, 0, 255), ..stroke(3.0, 0) }];
    let mut timeline = PaintingTimeline::new();
    timeline.add_stage(PaintingStage::new("tones", 0.0, 3.0, (0.0, 0.0)).with_order(StrokeOrder::DarkToLight)).unwrap();
    let set = timeline.schedule(vec![records.clone()], 1).unwrap();
    let lightness: Vec<f32> = set.records().iter().map(|r| color_to_oklab(r.color)[0]).collect();
    assert!(lightness.windows(2).all(|w| w[0] <= w[1]), "{lightness:?}");
    assert_eq!(set.records()[0].center.x, 1.0);
    let mut reverse = PaintingTimeline::new();
    reverse.add_stage(PaintingStage::new("tones", 0.0, 3.0, (0.0, 0.0)).with_order(StrokeOrder::LightToDark)).unwrap();
    let set = reverse.schedule(vec![records], 1).unwrap();
    assert_eq!(set.records()[0].center.x, 0.0);
    assert!(set.records().iter().all(|r| r.start == r.end), "(0, 0) seconds: each stroke appears whole");
}

#[test]
fn positional_and_key_orders_and_dropped_strokes() {
    // A sweep from the right: the rightmost stroke first.
    let mut timeline = PaintingTimeline::new();
    timeline.add_stage(PaintingStage::new("sweep", 0.0, 5.0, (0.5, 0.5)).with_order(StrokeOrder::Positional(Box::new(DirectionalArrivalOrder::new(180.0, 0.0).unwrap())))).unwrap();
    let set = timeline.schedule(vec![row(6)], 1).unwrap();
    let xs: Vec<f32> = set.records().iter().map(|r| r.center.x).collect();
    let mut sorted = xs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!(xs == sorted || xs.iter().rev().cloned().collect::<Vec<_>>() == sorted, "a directional sweep is monotonic: {xs:?}");
    // A key of your own; NaN drops a stroke.
    let mut keyed = PaintingTimeline::new();
    keyed.add_stage(PaintingStage::new("keyed", 0.0, 1.0, (0.0, 0.0)).with_order(StrokeOrder::Key(Box::new(|r: &BrushStrokeRecord| if r.center.x == 20.0 { f32::NAN } else { -r.center.x })))).unwrap();
    let set = keyed.schedule(vec![row(5)], 1).unwrap();
    let xs: Vec<f32> = set.records().iter().map(|r| r.center.x).collect();
    assert_eq!(xs, vec![40.0, 30.0, 10.0, 0.0]);
}

#[test]
fn durations_are_drawn_per_seed_within_the_range() {
    let run = |seed: i64| {
        let mut timeline = PaintingTimeline::new();
        timeline.add_stage(PaintingStage::new("detail", 0.0, 10.0, (0.5, 2.0))).unwrap();
        timeline.schedule(vec![row(40)], seed).unwrap()
    };
    let a = run(11);
    for r in a.records() {
        let d = r.end - r.start;
        assert!(((0.5 - 1.0e-5)..=(2.0 + 1.0e-5)).contains(&d), "{d}");
    }
    let durations = |s: &crate::strokes::BrushStrokeSet| s.records().iter().map(|r| r.end - r.start).collect::<Vec<_>>();
    assert_eq!(durations(&a), durations(&run(11)), "the same seed gives the same set");
    assert_eq!(a.records(), run(11).records());
    assert_ne!(durations(&a), durations(&run(12)));
}

#[test]
fn all_stages_are_sorted_by_start_with_stage_then_rank_ties() {
    let mut timeline = PaintingTimeline::new();
    timeline.add_stage(PaintingStage::new("block-in", 0.0, 10.0, (1.0, 1.0))).unwrap();
    timeline.add_stage(PaintingStage::new("detail", 5.0, 15.0, (1.0, 1.0))).unwrap();
    timeline.add_stage(PaintingStage::new("signature", 5.0, 5.0, (0.0, 0.0))).unwrap();
    let first: Vec<BrushStrokeRecord> = (0..11).map(|i| BrushStrokeRecord { seed: 100 + i, ..stroke(0.0, 10) }).collect();
    let second: Vec<BrushStrokeRecord> = (0..11).map(|i| BrushStrokeRecord { seed: 200 + i, ..stroke(0.0, 10) }).collect();
    let third: Vec<BrushStrokeRecord> = (0..2).map(|i| BrushStrokeRecord { seed: 300 + i, ..stroke(0.0, 10) }).collect();
    let set = timeline.schedule(vec![first, second, third], 3).unwrap();
    assert_eq!(set.len(), 24);
    let records = set.records();
    assert!(records.windows(2).all(|w| w[0].start <= w[1].start));
    // At 5 s: block-in's stroke at 5 s (if any) before detail's first, then the signature's two.
    let at_five: Vec<u32> = records.iter().filter(|r| r.start == 5.0).map(|r| r.seed).collect();
    let stages: Vec<u32> = at_five.iter().map(|s| s / 100).collect();
    assert!(stages.windows(2).all(|w| w[0] <= w[1]), "{at_five:?}");
    assert!(at_five.contains(&300) && at_five.contains(&301) && at_five.contains(&200));
    let signature: Vec<usize> = records.iter().enumerate().filter(|(_, r)| r.seed >= 300).map(|(i, _)| i).collect();
    assert_eq!(records[signature[0]].seed, 300);
    assert_eq!(records[signature[1]].seed, 301);
}

#[test]
fn stages_and_schedules_say_what_is_wrong() {
    let mut timeline = PaintingTimeline::new();
    assert!(timeline.add_stage(PaintingStage::new("  ", 0.0, 1.0, (0.0, 0.0))).unwrap_err().contains("needs a name"));
    timeline.add_stage(PaintingStage::new("wash", 0.0, 1.0, (0.0, 0.0))).unwrap();
    assert!(timeline.add_stage(PaintingStage::new("wash", 0.0, 1.0, (0.0, 0.0))).unwrap_err().contains("already exists"));
    assert!(timeline.add_stage(PaintingStage::new("late", 2.0, 1.0, (0.0, 0.0))).unwrap_err().contains("last not before the first"));
    assert!(timeline.add_stage(PaintingStage::new("slow", 0.0, 1.0, (2.0, 1.0))).unwrap_err().contains("stroke seconds"));
    assert!(timeline.add_stage(PaintingStage::new("nan", 0.0, 1.0, (f64::NAN, 1.0))).is_err());
    let bad_profile = ArrivalRateProfile { ramp_until: 2.0, ..ArrivalRateProfile::default() };
    assert!(timeline.add_stage(PaintingStage::new("odd", 0.0, 1.0, (0.0, 0.0)).with_profile(bad_profile)).unwrap_err().contains("Stage 'odd'"));
    assert_eq!(timeline.stages().len(), 1);
    assert!(timeline.schedule(vec![row(2), row(2)], 1).unwrap_err().contains("one list of strokes per stage"));
    let mut bad = row(2);
    bad[1].length = -1.0;
    assert!(timeline.schedule(vec![bad], 1).unwrap_err().contains("record 1"));
    assert!(timeline.schedule(vec![Vec::new()], 1).unwrap().is_empty());
}

#[test]
fn add_cues_names_and_spans() {
    let mut timeline = PaintingTimeline::new();
    timeline.add_stage(PaintingStage::new("wash", 0.0, 10.0, (0.5, 2.0))).unwrap();
    timeline.add_stage(PaintingStage::new("highlights", 40.0, 55.0, (0.25, 0.25))).unwrap();
    let mut cues = AnimationTimeline::new(60.0, 30.0).unwrap();
    timeline.add_cues(&mut cues).unwrap();
    let wash = cues.get_cue("wash").unwrap();
    assert_eq!((wash.start_time(), wash.end_time()), (0.0, 12.0));
    let highlights = cues.get_cue("highlights").unwrap();
    assert_eq!((highlights.start_time(), highlights.end_time()), (40.0, 55.25));
    // A clash adds nothing.
    let mut clash = AnimationTimeline::new(60.0, 30.0).unwrap();
    clash.add_marker("highlights", 1.0).unwrap();
    assert!(timeline.add_cues(&mut clash).unwrap_err().contains("highlights"));
    assert_eq!(clash.cues().len(), 1);
}
