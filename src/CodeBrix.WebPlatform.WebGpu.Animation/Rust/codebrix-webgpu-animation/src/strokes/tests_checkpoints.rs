//! Host tests of the checkpoint cache (strokes/stroke_checkpoints.rs): the decision whether a draw
//! can use checkpoints (every condition), the whole prefix the CPU computes exactly as the shader
//! decides a stroke is whole, the plan of each frame (which strokes are restored, drawn and baked
//! for a time against the checkpoints held - playing forward, seeking back, capacity, keys,
//! and the builder's key (the residency rule's test is in
//! rendering/core/tests_checkpoints.rs) (the same for a still painting at any time; another for any
//! change to the set's draw or to what is drawn before it).

use kurbo::Affine;

use crate::models::{DrawingTransform, DrawingTransformOptions};
use crate::rendering::{TestBuilder, VectorScene};
use crate::shapes::{BrushStrokesOptions, BrushStrokesShape};
use crate::strokes::stroke_checkpoints::{checkpoint_refusal, CheckpointCache, CheckpointFacts, CheckpointStep};
use crate::strokes::stroke_gpu::progress_f32;
use crate::strokes::{BrushStrokeRecord, BrushStrokeSet, StrokeAnchor, StrokeBoil, StrokeCheckpoints, StrokeColor, StrokeColorSource, StrokeFlow, StrokePickup};
use crate::{Color, DrawingSession, DrawingSessionOptions, PointF, RectangleF};

fn stroke(x: f32, y: f32, start: f32, end: f32) -> BrushStrokeRecord {
    let mut r = BrushStrokeRecord::new(PointF::new(x, y), 30.0, 8.0, 15.0, Color::from_rgb(200, 90, 40));
    r.start = start;
    r.end = end;
    r
}

/// n strokes, each painted over one second, one starting every tenth of a second.
fn painting(n: usize) -> BrushStrokeSet {
    BrushStrokeSet::new((0..n).map(|i| stroke(40.0 + (i % 25) as f32 * 22.0, 40.0 + (i / 25 % 14) as f32 * 22.0, i as f32 * 0.1, i as f32 * 0.1 + 1.0)).collect()).unwrap()
}

fn options(spacing: usize, capacity: usize) -> StrokeCheckpoints {
    StrokeCheckpoints { spacing, capacity }
}

// ---- the decision ----

#[test]
fn checkpoints_are_refused_for_boil_flow_followed_anchors_live_colours_and_transitions() {
    let usable = CheckpointFacts { asked: true, ..CheckpointFacts::default() };
    assert_eq!(checkpoint_refusal(&usable), None, "a still set in its own colours uses them");
    assert_eq!(checkpoint_refusal(&CheckpointFacts { asked: false, boil: true, ..CheckpointFacts::default() }), None, "none asked for: nothing to refuse");
    for (facts, word) in [
        (CheckpointFacts { boil: true, ..usable }, "boil"),
        (CheckpointFacts { flow: true, ..usable }, "flow"),
        (CheckpointFacts { followed_anchor: true, ..usable }, "follows an element"),
        (CheckpointFacts { live_colour: true, ..usable }, "picture target"),
        (CheckpointFacts { morph: true, ..usable }, "transition"),
    ] {
        let refusal = checkpoint_refusal(&facts).unwrap_or_default();
        assert!(refusal.contains(word), "{facts:?}: {refusal}");
    }
}

#[test]
fn a_shape_says_why_it_cannot_use_its_checkpoints() {
    //Arrange
    let set = painting(10);
    let asked = Some(StrokeCheckpoints::default());
    let shape = |options: BrushStrokesOptions| BrushStrokesShape::new(set.clone(), options).unwrap().with_checkpoints(asked).unwrap();
    let fixed = StrokeAnchor::Fixed(DrawingTransform::new(PointF::new(5.0, 0.0), DrawingTransformOptions::default()).unwrap());
    let followed = StrokeAnchor::FollowElement(crate::shapes::DrawingShape::from(crate::shapes::CircleShape::new(0.0, 0.0, 1.0, Default::default()).unwrap()).id());
    let picture = crate::images::ImageSource::from_rgba(&[255, 0, 0, 255], 1, 1).unwrap();
    let target = crate::rendering::PictureTarget::new(4, 4).unwrap();
    let pick = |source| StrokeColor::Picked(StrokePickup::new(source, RectangleF::new(0.0, 0.0, 100.0, 100.0)));
    //Act + Assert
    assert_eq!(shape(BrushStrokesOptions::default()).checkpoint_refusal(), None);
    assert_eq!(shape(BrushStrokesOptions::default()).with_anchors(vec![fixed]).unwrap().checkpoint_refusal(), None, "a fixed anchor holds still");
    assert_eq!(shape(BrushStrokesOptions::default()).with_color(pick(StrokeColorSource::Picture(picture))).unwrap().checkpoint_refusal(), None, "a picture does not change");
    assert_eq!(shape(BrushStrokesOptions { boil: Some(StrokeBoil { rate: 0.0, ..StrokeBoil::default() }), ..Default::default() }).checkpoint_refusal(), None, "a boil at rate 0 does not boil");
    assert!(shape(BrushStrokesOptions { boil: Some(StrokeBoil::default()), ..Default::default() }).checkpoint_refusal().unwrap().contains("boil"));
    assert!(shape(BrushStrokesOptions { flow: Some(StrokeFlow::default()), ..Default::default() }).checkpoint_refusal().unwrap().contains("flow"));
    assert!(shape(BrushStrokesOptions::default()).with_anchors(vec![followed]).unwrap().checkpoint_refusal().unwrap().contains("follows an element"));
    assert!(shape(BrushStrokesOptions::default()).with_color(pick(StrokeColorSource::Target(target))).unwrap().checkpoint_refusal().unwrap().contains("picture target"));
    assert_eq!(BrushStrokesShape::new(set.clone(), BrushStrokesOptions::default()).unwrap().checkpoint_refusal(), None, "none asked for");
    assert!(shape(BrushStrokesOptions::default()).with_checkpoints(Some(StrokeCheckpoints { spacing: 0, capacity: 4 })).is_err());
    assert!(shape(BrushStrokesOptions::default()).with_checkpoints(Some(StrokeCheckpoints { spacing: 10, capacity: 0 })).is_err());
}

#[test]
fn the_defaults_and_the_bytes_of_a_checkpoint() {
    assert_eq!(StrokeCheckpoints::default(), StrokeCheckpoints { spacing: 2000, capacity: 4 });
    assert_eq!(StrokeCheckpoints::bytes_at(1280, 720), 14_745_600);
    assert_eq!(StrokeCheckpoints::bytes_at(2560, 1440), 58_982_400);
    assert_eq!(BrushStrokesShape::new(painting(1), BrushStrokesOptions::default()).unwrap().checkpoints(), None, "off by default");
}

// ---- the whole prefix ----

#[test]
fn the_whole_prefix_stops_at_the_first_stroke_still_being_painted() {
    //Arrange: stroke 1 is long; strokes 2 and 3 end before it.
    let set = BrushStrokeSet::new(vec![stroke(0.0, 0.0, 0.0, 1.0), stroke(0.0, 0.0, 0.5, 10.0), stroke(0.0, 0.0, 1.0, 2.0), stroke(0.0, 0.0, 1.5, 3.0)]).unwrap();
    //Act + Assert
    assert_eq!(set.whole_count(0.99), 0);
    assert_eq!(set.whole_count(1.0), 1, "whole exactly at its end");
    assert_eq!(set.whole_count(5.0), 1, "a long stroke early blocks the finished ones after it");
    assert_eq!(set.whole_count(10.0), 4);
    assert_eq!(set.whole_count(f32::NAN), 0);
    let always = BrushStrokeSet::new(vec![BrushStrokeRecord::new(PointF::new(0.0, 0.0), 10.0, 2.0, 0.0, Color::BLACK); 3]).unwrap();
    assert_eq!(always.whole_count(-1.0e6), 3, "always whole");
}

#[test]
fn every_stroke_of_the_whole_prefix_is_drawn_whole_by_the_shader_rule() {
    //Arrange: awkward f32 times, where (t - start) / (end - start) may round below 1.
    let mut records = Vec::new();
    let mut state = 7u64;
    for _ in 0..2000 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let start = (state >> 40) as f32 / (1u64 << 24) as f32 * 977.3;
        let len = 0.01 + ((state >> 20) & 0xFFFF) as f32 / 65536.0 * 3.7;
        records.push(stroke(0.0, 0.0, start, start + len));
    }
    records.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    let set = BrushStrokeSet::new(records.clone()).unwrap();
    //Act + Assert: at each stroke's own end, every stroke of the prefix has progress exactly 1.
    for r in records.iter().step_by(37) {
        let t = r.end;
        let whole = set.whole_count(t);
        assert!(records[..whole].iter().all(|q| progress_f32(q.start, q.end, t) == 1.0), "t = {t}");
        if whole < records.len() {
            assert!(progress_f32(records[whole].start, records[whole].end, t) < 1.0, "the first stroke after the prefix is not whole");
        }
    }
    assert_eq!(progress_f32(0.1, 0.3, 0.3), 1.0, "exactly 1 at the end, by comparison");
}

// ---- the plan ----

fn step(cache: &mut CheckpointCache, whole: u32, frame: u64, o: StrokeCheckpoints) -> CheckpointStep {
    cache.step(42, whole, o, frame, 1000)
}

#[test]
fn a_still_key_bakes_on_its_second_frame_and_restores_from_then_on() {
    //Arrange
    let mut cache = CheckpointCache::default();
    let o = options(100, 4);
    //Act + Assert
    assert_eq!(step(&mut cache, 250, 1, o), CheckpointStep::default(), "a key's first frame: drawn whole, nothing made");
    let second = step(&mut cache, 250, 2, o);
    assert_eq!(second, CheckpointStep { restore: None, bake: Some((0, 200)) }, "made at floor(250 / 100) x 100 after drawing up to it");
    assert_eq!(second.ranges(260), vec![0..200, 200..260], "strokes 0..200 then the copy out, then the rest");
    let third = step(&mut cache, 250, 3, o);
    assert_eq!(third, CheckpointStep { restore: Some((0, 200)), bake: None });
    assert_eq!(third.first(), 200);
    assert_eq!(third.ranges(260), vec![200..260], "only the strokes after the checkpoint are drawn");
    assert!(third.splits());
}

#[test]
fn playing_forward_restores_the_last_checkpoint_and_bakes_the_next_one_at_most_once_a_frame() {
    //Arrange
    let mut cache = CheckpointCache::default();
    let o = options(100, 4);
    step(&mut cache, 150, 1, o);
    step(&mut cache, 150, 2, o);
    //Act: the prefix jumps from 150 to 420 in one frame.
    let s = step(&mut cache, 420, 3, o);
    //Assert: one bake, at 400 (not 200 and 300 as well).
    assert_eq!(s, CheckpointStep { restore: Some((0, 100)), bake: Some((1, 400)) });
    assert_eq!(s.ranges(450), vec![100..400, 400..450]);
    assert_eq!(cache.held_for(42), vec![100, 400]);
    assert_eq!(step(&mut cache, 430, 4, o), CheckpointStep { restore: Some((1, 400)), bake: None });
}

#[test]
fn seeking_back_restores_the_nearest_earlier_checkpoint_or_draws_whole_and_bakes_there() {
    //Arrange: checkpoints at 100, 200, 300.
    let mut cache = CheckpointCache::default();
    let o = options(100, 4);
    let mut frame = 1;
    for whole in [100, 100, 200, 300] {
        step(&mut cache, whole, frame, o);
        frame += 1;
    }
    assert_eq!(cache.held_for(42), vec![100, 200, 300]);
    //Act + Assert: back to 250 - the checkpoint at 200; back to 60 - nothing earlier, drawn whole.
    assert_eq!(step(&mut cache, 250, frame, o), CheckpointStep { restore: Some((1, 200)), bake: None });
    assert_eq!(step(&mut cache, 60, frame + 1, o), CheckpointStep::default(), "below the first checkpoint: drawn whole, none to make");
    assert_eq!(step(&mut cache, 120, frame + 2, o), CheckpointStep { restore: Some((0, 100)), bake: None });
}

#[test]
fn a_full_cache_gives_the_texture_of_the_checkpoint_farthest_from_the_new_one() {
    //Arrange: capacity 2, checkpoints at 100 and 200.
    let mut cache = CheckpointCache::default();
    let o = options(100, 2);
    for (frame, whole) in [(1, 100), (2, 100), (3, 200)] {
        step(&mut cache, whole, frame, o);
    }
    //Act: 500 - farthest is 100 (slot 0), which gives up its texture.
    let s = step(&mut cache, 520, 4, o);
    //Assert
    assert_eq!(s, CheckpointStep { restore: Some((1, 200)), bake: Some((0, 500)) });
    assert_eq!(cache.held_for(42), vec![200, 500]);
    assert_eq!(cache.held(), 2);
    assert_eq!(cache.held_bytes(), 2000);
}

#[test]
fn another_key_never_restores_and_a_key_that_changes_every_frame_never_bakes() {
    //Arrange
    let mut cache = CheckpointCache::default();
    let o = options(10, 4);
    cache.step(1, 50, o, 1, 8);
    cache.step(1, 50, o, 2, 8);
    //Act + Assert
    assert_eq!(cache.step(2, 50, o, 3, 8), CheckpointStep::default(), "another key: drawn whole");
    for frame in 4..10 {
        assert_eq!(cache.step(100 + frame, 50, o, frame, 8).bake, None, "a moving painting (a new key each frame) makes nothing");
    }
    assert_eq!(cache.held(), 1);
}

#[test]
fn fewer_whole_strokes_than_the_spacing_make_no_checkpoint() {
    let mut cache = CheckpointCache::default();
    let o = options(2000, 4);
    for frame in 1..5 {
        assert_eq!(step(&mut cache, 1999, frame, o), CheckpointStep::default());
    }
}

// ---- the builder's key ----

fn session(width: f32) -> DrawingSession {
    DrawingSession::new(DrawingSessionOptions { calibration_size: (640, 360), layer_opacity: 255, background_fill_color: Color::from_rgb(240, 235, 220), ..DrawingSessionOptions::default() })
        .tap(|s| {
            s.add_layer("Paint", Color::WHITE);
            s.draw_rectangle(10.0, 10.0, width, 40.0, None, Some(Color::from_rgb(20, 40, 60)), true, 0.0).unwrap();
        })
}

trait Tap: Sized {
    fn tap(self, f: impl FnOnce(&mut Self)) -> Self;
}

impl Tap for DrawingSession {
    fn tap(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }
}

/// The builder's (key, whole prefix, instances) of the session's first stroke draw at a time.
fn ask(b: &mut TestBuilder, s: &mut DrawingSession, time: f64, size: (u32, u32)) -> Option<(u64, u32, u32)> {
    s.set_time(time);
    let mut scene = VectorScene::new();
    s.render_with_background(&mut scene, size.0, size.1);
    b.build(&scene, size.0, size.1);
    b.stroke_ops.first().and_then(|o| o.checkpoint.map(|a| (a.key, a.whole, o.instances)))
}

fn with_set(s: &mut DrawingSession, shape: BrushStrokesShape) -> crate::models::ElementId {
    s.draw_brush_strokes(shape)
}

#[test]
fn a_still_painting_keeps_its_key_while_it_is_painted_on() {
    //Arrange
    let mut s = session(100.0);
    with_set(&mut s, BrushStrokesShape::new(painting(300), BrushStrokesOptions::default()).unwrap().with_checkpoints(Some(StrokeCheckpoints::default())).unwrap());
    let mut b = TestBuilder::default();
    //Act
    let at_5 = ask(&mut b, &mut s, 5.0, (640, 360)).unwrap();
    let at_20 = ask(&mut b, &mut s, 20.0, (640, 360)).unwrap();
    let back = ask(&mut b, &mut s, 5.0, (640, 360)).unwrap();
    //Assert: the key does not depend on the time; the whole prefix does.
    assert_eq!(at_5.0, at_20.0);
    assert_eq!(at_5, back, "a seek back asks exactly as before");
    assert_eq!((at_5.1, at_5.2), (41, 51), "at 5 s: strokes 0..41 ended (start + 1 <= 5), 51 started");
    assert_eq!(at_20.1, 191);
}

#[test]
fn the_key_changes_with_the_transform_the_size_the_options_and_what_is_drawn_before_but_not_after() {
    //Arrange
    let base = || BrushStrokesShape::new(painting(300), BrushStrokesOptions::default()).unwrap().with_checkpoints(Some(StrokeCheckpoints::default())).unwrap();
    let key = |s: &mut DrawingSession, size: (u32, u32)| ask(&mut TestBuilder::default(), s, 8.0, size).unwrap().0;
    let set = painting(300);
    let shape_of = |set: &BrushStrokeSet| BrushStrokesShape::new(set.clone(), BrushStrokesOptions::default()).unwrap().with_checkpoints(Some(StrokeCheckpoints::default())).unwrap();
    let mut plain = session(100.0);
    with_set(&mut plain, shape_of(&set));
    let reference = key(&mut plain, (640, 360));
    //Act + Assert
    let mut same = session(100.0);
    with_set(&mut same, shape_of(&set));
    assert_eq!(key(&mut same, (640, 360)), reference, "the same drawing, the same key");
    let mut other_set = session(100.0);
    with_set(&mut other_set, base());
    assert_ne!(key(&mut other_set, (640, 360)), reference, "another set");
    assert_ne!(key(&mut plain, (1280, 720)), reference, "another canvas size");
    let mut before = session(120.0);
    with_set(&mut before, shape_of(&set));
    assert_ne!(key(&mut before, (640, 360)), reference, "a shape before the set changed");
    let mut after = session(100.0);
    with_set(&mut after, shape_of(&set));
    after.draw_circle(500.0, 300.0, 20.0, None, Some(Color::from_rgb(250, 0, 0)), true).unwrap();
    assert_eq!(key(&mut after, (640, 360)), reference, "a shape after the set does not matter");
    let mut moved = session(100.0);
    let id = with_set(&mut moved, shape_of(&set));
    let track = crate::animation::PointTrack::from_keys(&[(0.0, PointF::new(0.0, 0.0), crate::animation::EasingKind::Linear), (100.0, PointF::new(50.0, 0.0), crate::animation::EasingKind::Linear)]).unwrap();
    moved.animations_mut().bind(id, crate::models::AnimatedProperty::Position, track).unwrap();
    let k1 = ask(&mut TestBuilder::default(), &mut moved, 8.0, (640, 360)).unwrap().0;
    let k2 = ask(&mut TestBuilder::default(), &mut moved, 9.0, (640, 360)).unwrap().0;
    assert_ne!(k1, k2, "a moving painting: a new key every frame");
    let mut soft = session(100.0);
    with_set(&mut soft, BrushStrokesShape::new(set.clone(), BrushStrokesOptions { head_softness: 0.3, ..Default::default() }).unwrap().with_checkpoints(Some(StrokeCheckpoints::default())).unwrap());
    assert_ne!(key(&mut soft, (640, 360)), reference, "another draw block");
    let _ = Affine::IDENTITY;
}

#[test]
fn a_refused_draw_asks_nothing_counts_a_miss_and_is_reported_once() {
    //Arrange
    let mut s = session(100.0);
    with_set(&mut s, BrushStrokesShape::new(painting(50), BrushStrokesOptions { boil: Some(StrokeBoil::default()), ..Default::default() }).unwrap().with_checkpoints(Some(StrokeCheckpoints::default())).unwrap());
    let mut b = TestBuilder::default();
    //Act
    let asked = ask(&mut b, &mut s, 8.0, (640, 360));
    ask(&mut b, &mut s, 9.0, (640, 360));
    //Assert
    assert_eq!(asked, None);
    assert_eq!(b.stats.checkpoint_misses, 1);
    let reports = s.take_reports();
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].contains("checkpoints but cannot use them") && reports[0].contains("boil"), "{reports:?}");
}
