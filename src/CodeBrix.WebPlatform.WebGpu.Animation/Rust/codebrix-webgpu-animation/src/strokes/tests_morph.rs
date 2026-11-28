//! Host tests of stroke-level transitions (strokes/brush_stroke_morph.rs): the greedy pairing
//! (deterministic for a seed, every stroke in exactly one slot, the nearest within the radius,
//! colour weighting), the interpolation's CPU mirror (endpoints exactly the two sets, the shorter
//! turn with the bend flipped, Oklab colour, painting off / on), the spread, the two draw orders
//! and their packing, the shape, the MorphFraction binding and the builder's draw.

use crate::animation::{EasingKind, FloatTrack};
use crate::formations::DirectionalArrivalOrder;
use crate::models::AnimatedProperty;
use crate::rendering::{TestBuilder, VectorScene};
use crate::shapes::{BrushStrokesOptions, BrushStrokesShape};
use crate::strokes::brush_stroke_morph::{morph_record, pair_fraction, pair_own_weight, shorter_turn, PairKind};
use crate::strokes::stroke_gpu::{own_weight_of_word, pack_records, FLAG_MORPH};
use crate::strokes::{BrushStrokeMorph, BrushStrokeRecord, BrushStrokeSet, StrokeCheckpoints, StrokePair, StrokePairingOptions};
use crate::{Color, DrawingSession, DrawingSessionOptions, PointF};

fn at(x: f32, y: f32, color: Color) -> BrushStrokeRecord {
    BrushStrokeRecord::new(PointF::new(x, y), 20.0, 6.0, 0.0, color)
}

fn grid(n: usize, dx: f32, color: Color, seed_offset: u32) -> BrushStrokeSet {
    BrushStrokeSet::new(
        (0..n)
            .map(|i| {
                let mut r = at(30.0 + (i % 20) as f32 * 25.0 + dx, 30.0 + (i / 20) as f32 * 25.0, color);
                r.angle = (i * 37 % 180) as f32;
                r.seed = i as u32 + seed_offset;
                r
            })
            .collect(),
    )
    .unwrap()
}

#[test]
fn pairing_is_deterministic_for_a_seed_and_every_stroke_is_in_exactly_one_slot() {
    //Arrange
    let from = grid(200, 0.0, Color::from_rgb(200, 40, 40), 0);
    let to = grid(170, 7.0, Color::from_rgb(40, 40, 200), 1000);
    let options = StrokePairingOptions { seed: 5, ..StrokePairingOptions::default() };
    //Act
    let a = BrushStrokeMorph::pair(&from, &to, options).unwrap();
    let b = BrushStrokeMorph::pair(&from, &to, options).unwrap();
    //Assert
    assert_eq!(a.pairs(), b.pairs(), "the same seed, the same pairs");
    let mut froms: Vec<u32> = a.pairs().iter().filter_map(|p| p.from).collect();
    let mut tos: Vec<u32> = a.pairs().iter().filter_map(|p| p.to).collect();
    froms.sort_unstable();
    tos.sort_unstable();
    assert_eq!(froms, (0..200).collect::<Vec<_>>(), "every stroke of the first set once");
    assert_eq!(tos, (0..170).collect::<Vec<_>>(), "every stroke of the second set once");
    assert!(a.pairs().iter().all(|p| p.from.is_some() || p.to.is_some()));
    assert_eq!(a.paired_count(), 170, "the smaller set is paired whole here (each partner 7 units away)");
    assert_eq!(a.len(), 200);
    assert_eq!(a.pairs()[..200].iter().map(|p| p.from.unwrap()).collect::<Vec<_>>(), (0..200).collect::<Vec<_>>(), "the slots follow the first set's order");
}

#[test]
fn a_stroke_takes_the_nearest_free_partner_within_the_radius() {
    //Arrange
    let from = BrushStrokeSet::new(vec![at(0.0, 0.0, Color::BLACK), at(500.0, 0.0, Color::BLACK)]).unwrap();
    let to = BrushStrokeSet::new(vec![at(30.0, 0.0, Color::BLACK), at(10.0, 0.0, Color::BLACK), at(560.0, 0.0, Color::BLACK)]).unwrap();
    //Act
    let m = BrushStrokeMorph::pair(&from, &to, StrokePairingOptions::default()).unwrap();
    //Assert: (0,0) takes the one 10 away; (500,0) has none within 40 and paints off; the other
    // two of the second set paint on.
    assert_eq!(m.pairs(), &[StrokePair { from: Some(0), to: Some(1) }, StrokePair { from: Some(1), to: None }, StrokePair { from: None, to: Some(0) }, StrokePair { from: None, to: Some(2) }]);
}

#[test]
fn the_colour_weight_prefers_a_partner_of_a_closer_colour() {
    //Arrange: a red stroke between a red partner 20 away and a blue one 15 away.
    let red = Color::from_rgb(220, 30, 30);
    let from = BrushStrokeSet::new(vec![at(100.0, 100.0, red)]).unwrap();
    let to = BrushStrokeSet::new(vec![at(115.0, 100.0, Color::from_rgb(30, 30, 220)), at(80.0, 100.0, red)]).unwrap();
    //Act
    let by_place = BrushStrokeMorph::pair(&from, &to, StrokePairingOptions { color_weight: 0.0, ..Default::default() }).unwrap();
    let by_colour = BrushStrokeMorph::pair(&from, &to, StrokePairingOptions::default()).unwrap();
    //Assert
    assert_eq!(by_place.pairs()[0].to, Some(0));
    assert_eq!(by_colour.pairs()[0].to, Some(1));
}

#[test]
fn unusable_pairing_options_say_what_to_change() {
    let s = grid(3, 0.0, Color::BLACK, 0);
    assert!(BrushStrokeMorph::pair(&s, &s, StrokePairingOptions { search_radius: 0.0, ..Default::default() }).unwrap_err().contains("search radius"));
    assert!(BrushStrokeMorph::pair(&s, &s, StrokePairingOptions { color_weight: -1.0, ..Default::default() }).unwrap_err().contains("colour weight"));
    let empty = BrushStrokeSet::new(Vec::new()).unwrap();
    assert!(BrushStrokeMorph::pair(&empty, &empty, StrokePairingOptions::default()).unwrap().is_empty());
}

#[test]
fn the_endpoints_are_exactly_the_two_sets() {
    //Arrange
    let from = grid(60, 0.0, Color::from_rgb(200, 120, 40), 0);
    let to = grid(45, 9.0, Color::from_rgb(40, 120, 200), 77);
    let m = BrushStrokeMorph::pair(&from, &to, StrokePairingOptions::default()).unwrap();
    //Act + Assert
    for (k, p) in m.pairs().iter().enumerate() {
        let (start, start_paint) = m.record_at(k, 0.0).unwrap();
        let (end, end_paint) = m.record_at(k, 1.0).unwrap();
        match (p.from, p.to) {
            (Some(i), Some(j)) => {
                assert_eq!((start, start_paint), (from.records()[i as usize], 1.0));
                assert_eq!((end, end_paint), (to.records()[j as usize], 1.0));
            }
            (Some(i), None) => {
                assert_eq!((start, start_paint), (from.records()[i as usize], 1.0), "a stroke that leaves is whole at 0");
                assert_eq!(end_paint, 0.0, "and painted off at 1");
            }
            (None, Some(j)) => {
                assert_eq!(start_paint, 0.0, "a stroke that arrives shows nothing at 0");
                assert_eq!((end, end_paint), (to.records()[j as usize], 1.0));
            }
            _ => unreachable!(),
        }
    }
    assert!(m.record_at(m.len(), 0.5).is_none());
}

#[test]
fn a_pair_turns_the_shorter_way_using_the_strokes_symmetry() {
    assert_eq!(shorter_turn(10.0, 80.0), (70.0, false));
    assert_eq!(shorter_turn(10.0, 170.0), (-20.0, true), "160 apart: 20 the other way, the stroke reversed");
    assert_eq!(shorter_turn(350.0, 20.0), (30.0, false), "across 360");
    assert_eq!(shorter_turn(0.0, -100.0), (80.0, true));
    //Arrange
    let mut a = at(0.0, 0.0, Color::BLACK);
    a.angle = 10.0;
    a.bend = 0.1;
    let mut b = at(100.0, 50.0, Color::BLACK);
    b.angle = 170.0;
    b.bend = 0.2;
    b.tip = 3;
    b.seed = 9;
    //Act
    let mid = morph_record(&a, &b, PairKind::Paired, 0.5);
    let early = morph_record(&a, &b, PairKind::Paired, 0.25);
    //Assert
    assert_eq!(mid.record.angle, 0.0, "halfway along the 20-degree turn");
    assert!((mid.record.bend - (-0.05)).abs() < 1e-6, "towards -0.2: the reversed second stroke's bend");
    assert_eq!(mid.record.center, PointF::new(50.0, 25.0));
    assert!(mid.reversed && !early.reversed, "the second stroke's tip, reversed, from halfway on");
    assert_eq!((mid.record.tip, mid.record.seed, early.record.tip, early.record.seed), (3, 9, 0, 0), "tip and seed switch at halfway");
}

#[test]
fn the_colour_travels_in_oklab() {
    //Arrange: blue to yellow - the sRGB midpoint is a grey.
    let a = at(0.0, 0.0, Color::from_rgb(0, 0, 255));
    let b = at(0.0, 0.0, Color::from_rgb(255, 255, 0));
    //Act
    let mid = morph_record(&a, &b, PairKind::Paired, 0.5).record.color;
    //Assert
    let chroma = |c: Color| {
        let l = crate::painting::color_mix::color_to_oklab(c);
        (l[1] * l[1] + l[2] * l[2]).sqrt()
    };
    assert!(chroma(mid) > chroma(Color::from_rgb(128, 128, 128)) + 0.02, "{mid:?}");
    assert_eq!(mid.a, 255);
}

#[test]
fn a_spread_staggers_each_pairs_own_fraction() {
    assert_eq!(pair_fraction(0.5, 0.0, 0.7), 0.5, "no spread: every pair together");
    assert_eq!(pair_fraction(0.25, 0.5, 0.0), 0.5);
    assert_eq!(pair_fraction(0.25, 0.5, 1.0), 0.0, "the last pair has not started");
    assert_eq!(pair_fraction(1.0, 0.5, 1.0), 1.0, "every pair arrives at 1");
    assert_eq!(pair_fraction(0.0, 0.9, 0.0), 0.0, "every pair is home at 0");
    //Arrange: a left-to-right sweep over three pairs.
    let from = BrushStrokeSet::new(vec![at(300.0, 0.0, Color::BLACK), at(0.0, 0.0, Color::BLACK), at(150.0, 0.0, Color::BLACK)]).unwrap();
    let m = BrushStrokeMorph::pair(&from, &from, StrokePairingOptions::default()).unwrap();
    //Act
    let shape = BrushStrokesShape::morph(m.clone(), BrushStrokesOptions::default(), Some((Box::new(DirectionalArrivalOrder::new(0.0, 0.0).unwrap()), 0.5))).unwrap();
    //Assert
    let delays = delays_of(&shape);
    assert_eq!(delays, vec![1.0, 0.0, 0.5], "ranked left to right");
    assert!(BrushStrokesShape::morph(m.clone(), BrushStrokesOptions::default(), Some((Box::new(DirectionalArrivalOrder::new(0.0, 0.0).unwrap()), 1.0))).is_err());
    assert!(BrushStrokesShape::morph(m, BrushStrokesOptions::default(), None).is_ok());
}

fn delays_of(shape: &BrushStrokesShape) -> Vec<f32> {
    // The state is crate-internal: read it back through the scene the shape records.
    let mut s = DrawingSession::new(DrawingSessionOptions::default());
    s.add_layer("Paint", Color::WHITE).add_shape(shape.clone());
    let mut scene = VectorScene::new();
    s.render(&mut scene, 640, 360);
    scene.stroke_draws[0].morph.as_ref().unwrap().state.delays.to_vec()
}

#[test]
fn the_two_orders_are_each_paintings_own_and_the_targets_carry_the_kind_and_delay() {
    //Arrange: from (0, 1), to (0, 1, 2) paired from 0 -> to 1, from 1 -> to 0; to 2 arrives.
    let from = BrushStrokeSet::new(vec![at(0.0, 0.0, Color::BLACK), at(100.0, 0.0, Color::BLACK)]).unwrap();
    let to = BrushStrokeSet::new(vec![at(105.0, 0.0, Color::WHITE), at(5.0, 0.0, Color::WHITE), at(400.0, 0.0, Color::WHITE)]).unwrap();
    let m = BrushStrokeMorph::pair(&from, &to, StrokePairingOptions::default()).unwrap();
    //Act
    let first = m.variant_order(0);
    let second = m.variant_order(1);
    let (a, b) = m.pack_variant(&[0.25, 0.5, 0.75], 1);
    //Assert
    assert_eq!(first, vec![0, 1, 2], "the first painting's order, then those that arrive");
    assert_eq!(second, vec![1, 0, 2], "the second painting's order: to 0 is slot 1's, to 1 slot 0's, to 2 slot 2's");
    let word = |bytes: &[u8], k: usize, w: usize| u32::from_le_bytes(bytes[k * 64 + w * 4..k * 64 + w * 4 + 4].try_into().unwrap());
    assert_eq!(f32::from_bits(word(&b, 0, 11)), 0.5, "slot 1's delay in the first place of the second order");
    assert_eq!(word(&b, 2, 13) >> 16, 2, "the arriving stroke's kind");
    assert_eq!(word(&b, 0, 13) >> 16, 0);
    assert_eq!(f32::from_bits(word(&a, 2, 0)), 400.0, "an arriving stroke stands for itself in the first records");
    assert_eq!(a.len(), 3 * 64);
}

#[test]
fn both_record_buffers_carry_each_strokes_own_weight_beside_the_kind_and_delay() {
    //Arrange: as above; from 0 -> to 1, from 1 -> to 0, to 2 arrives, from weights (0, 0.2),
    // to weights (1, 0.6, 0.4).
    let from = BrushStrokeSet::new(vec![at(0.0, 0.0, Color::BLACK), at(100.0, 0.0, Color::BLACK)]).unwrap().with_own_weights(vec![0.0, 0.2]).unwrap();
    let to = BrushStrokeSet::new(vec![at(105.0, 0.0, Color::WHITE), at(5.0, 0.0, Color::WHITE), at(400.0, 0.0, Color::WHITE)]).unwrap().with_own_weights(vec![1.0, 0.6, 0.4]).unwrap();
    let m = BrushStrokeMorph::pair(&from, &to, StrokePairingOptions::default()).unwrap();
    //Act
    let (a, b) = m.pack_variant(&[0.25, 0.5, 0.75], 0);
    //Assert
    let word = |bytes: &[u8], k: usize, w: usize| u32::from_le_bytes(bytes[k * 64 + w * 4..k * 64 + w * 4 + 4].try_into().unwrap());
    let weights = |bytes: &[u8]| (0..3).map(|k| own_weight_of_word(word(bytes, k, 13))).collect::<Vec<_>>();
    let w = |set: &BrushStrokeSet, i: usize| set.own_weight(i).unwrap();
    assert_eq!(weights(&a), vec![w(&from, 0), w(&from, 1), w(&to, 2)], "the first strokes' own; an arriving stroke stands for itself");
    assert_eq!(weights(&b), vec![w(&to, 1), w(&to, 0), w(&to, 2)], "the second strokes' own");
    assert_eq!((word(&b, 2, 13) >> 16) & 0xFF, 2, "the kind beside the weight");
    assert_eq!(word(&b, 2, 13) >> 24, 255 - 102, "0.4 = 102 of 255");
    assert_eq!(f32::from_bits(word(&b, 1, 11)), 0.5, "the delay");
    let plain = BrushStrokeMorph::pair(&BrushStrokeSet::new(from.records().to_vec()).unwrap(), &BrushStrokeSet::new(to.records().to_vec()).unwrap(), StrokePairingOptions::default()).unwrap();
    let (pa, _) = plain.pack_variant(&[0.25, 0.5, 0.75], 0);
    let firsts: Vec<BrushStrokeRecord> = plain.pairs().iter().map(|p| plain.ends(p).0).collect();
    assert_eq!(pa, pack_records(&firsts), "sets without weights: the records exactly as before");
}

#[test]
fn a_pairs_own_weight_travels_with_its_fraction_and_a_stroke_off_or_on_keeps_its_own() {
    assert_eq!(pair_own_weight(0.2, 0.8, PairKind::Paired, 0.0), 0.2, "exactly the first at 0");
    assert_eq!(pair_own_weight(0.2, 0.8, PairKind::Paired, -1.0), 0.2);
    assert_eq!(pair_own_weight(0.2, 0.8, PairKind::Paired, f32::NAN), 0.2);
    assert_eq!(pair_own_weight(0.2, 0.8, PairKind::Paired, 1.0), 0.8, "exactly the second at 1");
    assert!((pair_own_weight(0.2, 0.8, PairKind::Paired, 0.25) - 0.35).abs() < 1e-6, "linear between");
    assert_eq!(pair_own_weight(0.2, 0.8, PairKind::Leaves, 0.5), 0.2);
    assert_eq!(pair_own_weight(0.2, 0.8, PairKind::Arrives, 0.5), 0.8);
}

#[test]
fn the_morph_fraction_binding_reaches_a_transition_and_is_refused_on_a_plain_set() {
    //Arrange
    let from = grid(20, 0.0, Color::BLACK, 0);
    let to = grid(20, 5.0, Color::WHITE, 0);
    let m = BrushStrokeMorph::pair(&from, &to, StrokePairingOptions::default()).unwrap();
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (640, 360), ..DrawingSessionOptions::default() });
    s.add_layer("Paint", Color::WHITE);
    let id = s.draw_brush_strokes(BrushStrokesShape::morph(m, BrushStrokesOptions::default(), None).unwrap());
    let plain = s.draw_brush_strokes(BrushStrokesShape::new(from, BrushStrokesOptions::default()).unwrap());
    let track = FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (4.0, 1.0, EasingKind::Linear)]).unwrap();
    //Act
    s.animations_mut().bind(id, AnimatedProperty::MorphFraction, track.clone()).unwrap();
    let refused = s.animations_mut().bind(plain, AnimatedProperty::MorphFraction, track);
    //Assert
    assert!(refused.unwrap_err().contains("stroke-level transition"));
    let fraction = |s: &mut DrawingSession, t: f64| {
        s.set_time(t);
        let mut scene = VectorScene::new();
        s.render(&mut scene, 640, 360);
        scene.stroke_draws[0].morph.as_ref().unwrap().fraction
    };
    assert_eq!(fraction(&mut s, 1.0), 0.25);
    assert_eq!(fraction(&mut s, 3.0), 0.75);
    assert_eq!(fraction(&mut s, 1.0), 0.25, "scrubbed back: the same fraction");
}

#[test]
fn the_builder_draws_every_slot_in_the_order_of_the_nearer_painting_with_the_fraction_in_the_block() {
    //Arrange
    let from = grid(30, 0.0, Color::BLACK, 0);
    let to = grid(40, 4.0, Color::WHITE, 0);
    let m = BrushStrokeMorph::pair(&from, &to, StrokePairingOptions::default()).unwrap();
    let n = m.len() as u32;
    let draw = |fraction: f32| {
        let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (640, 360), layer_opacity: 255, ..DrawingSessionOptions::default() });
        let shape = BrushStrokesShape::morph(m.clone(), BrushStrokesOptions::default(), Some((Box::new(DirectionalArrivalOrder::new(0.0, 0.0).unwrap()), 0.4))).unwrap().with_morph_fraction(fraction).with_checkpoints(Some(StrokeCheckpoints::default())).unwrap();
        s.add_layer("Paint", Color::WHITE).add_shape(shape);
        let mut scene = VectorScene::new();
        s.render(&mut scene, 640, 360);
        let mut b = TestBuilder::default();
        b.build(&scene, 640, 360);
        let o = &b.stroke_ops[0];
        let block = &b.stroke_blocks[o.offset as usize / 4..];
        (o.instances, o.morph.as_ref().map(|(_, v)| *v), o.checkpoint.is_some(), block[12] & FLAG_MORPH, f32::from_bits(block[10]), f32::from_bits(block[11]), b.stats.checkpoint_misses)
    };
    //Act + Assert
    assert_eq!(draw(0.2), (n, Some(0), false, FLAG_MORPH, 0.2, 0.4, 1), "below halfway: the first painting's order; checkpoints refused");
    assert_eq!(draw(0.5).1, Some(1), "from halfway: the second's");
    assert_eq!(draw(1.0).0, n, "every slot is drawn");
}
