//! Host tests of the GPU side of stroke sets: the 64-byte record packing against hand-written
//! bytes, the shader's progress rule, the CPU mirrors of the hull and the coverage against the
//! CPU twin's polygon (away from its chords), brush tips' resampling and mip chains, anchors, the boil's timing and
//! jitter, and how many records a frame draws.

use kurbo::Affine;

use crate::color::Color;
use crate::geometry::PointF;
use crate::models::{AnimatedProperty, DrawingTransform, DrawingTransformOptions};
use crate::planes::DrawingPlane;
use crate::strokes::stroke_anchor::{anchored, blend, resolve_anchors};
use crate::strokes::stroke_boil::{boil_jitter, hash_unit};
use crate::strokes::stroke_gpu::{draw_block_bytes, extent, hull, own_weight_of_word, pack_record, pack_records, pack_set, progress_f32, stroke_coverage, to_local};
use crate::strokes::{boil_epoch, BrushStrokeRecord, BrushStrokeSet, BrushTip, BrushTips, StrokeAnchor, StrokeBoil};

fn record(bend: f32, profile: f32) -> BrushStrokeRecord {
    let mut r = BrushStrokeRecord::new(PointF::new(40.0, 30.0), 60.0, 14.0, 25.0, Color::from_rgb(200, 100, 50));
    r.bend = bend;
    r.profile = profile;
    r
}

#[test]
fn a_record_packs_into_the_64_bytes_the_shader_reads() {
    //Arrange
    let mut r = BrushStrokeRecord::new(PointF::new(1.5, -2.0), 10.0, 4.0, 90.0, Color::from_rgba(10, 20, 30, 200));
    r.bend = 0.25;
    r.profile = 4.0;
    r.load = 0.5;
    r.tip = 3;
    r.anchors = [1, 7];
    r.anchor_blend = 0.25;
    r.start = 2.0;
    r.end = 5.0;
    r.seed = 0xDEAD_BEEF;
    //Act
    let words = pack_record(&r);
    let bytes = pack_records(&[r, r]);
    //Assert
    let expected: [u32; 16] = [0x3FC0_0000, 0xC000_0000, 0x3FC9_0FDB, 0x4120_0000, 0x4080_0000, 0x3E80_0000, 0x4080_0000, 0x3F00_0000, 0x4000_0000, 0x40A0_0000, 0x3E80_0000, 0, 0xC81E_140A, 3, 0x0007_0001, 0xDEAD_BEEF];
    assert_eq!(words, expected);
    assert_eq!(bytes.len(), 128);
    assert_eq!(&bytes[0..4], &[0x00, 0x00, 0xC0, 0x3F], "little-endian, centre x first");
    assert_eq!(&bytes[48..52], &[10, 20, 30, 200], "the colour's bytes are r g b a");
    assert_eq!(&bytes[56..60], &[1, 0, 7, 0], "anchor 0 in the low half, anchor 1 in the high half");
    assert_eq!(&bytes[64..128], &bytes[0..64]);
}

#[test]
fn a_set_without_own_weights_packs_exactly_the_records_and_weights_of_one_do_too() {
    //Arrange
    let mut r = record(0.25, 3.0);
    r.tip = 0xFFFF;
    r.seed = 9;
    let records = vec![r, record(0.0, 4.0), record(-0.1, 2.0)];
    let plain = BrushStrokeSet::new(records.clone()).unwrap();
    let ones = BrushStrokeSet::new(records.clone()).unwrap().with_own_weights(vec![1.0; 3]).unwrap();
    //Act
    let (a, b) = (pack_set(&plain), pack_set(&ones));
    //Assert
    assert_eq!(a, pack_records(&records), "byte for byte the records as packed before own weights");
    assert_eq!(b, a, "weight 1 packs 0 in word 13's top byte");
    assert_eq!(u32::from_le_bytes(a[52..56].try_into().unwrap()), 0xFFFF, "word 13 is the tip alone");
}

#[test]
fn own_weights_pack_into_word_13s_top_byte_beside_the_tip() {
    //Arrange
    let mut r = record(0.0, 4.0);
    r.tip = 0xFFFF;
    let set = BrushStrokeSet::new(vec![r; 4]).unwrap().with_own_weights(vec![0.0, 0.5, 1.0, 0.2]).unwrap();
    //Act
    let bytes = pack_set(&set);
    let word13 = |k: usize| u32::from_le_bytes(bytes[k * 64 + 52..k * 64 + 56].try_into().unwrap());
    //Assert
    assert_eq!(word13(0), 0xFF00_FFFF, "weight 0: 255 there");
    assert_eq!(word13(1), 0x7F00_FFFF, "weight 0.5 = 128 of 255: 127 there");
    assert_eq!(word13(2), 0x0000_FFFF, "weight 1: 0 there");
    assert_eq!(word13(3), ((255 - 51) << 24) | 0xFFFF);
    assert_eq!(&bytes[64..116], &pack_records(&[r])[0..52], "every other word as before");
    for k in 0..4 {
        assert_eq!(own_weight_of_word(word13(k)), set.own_weight(k).unwrap(), "the shader's reading is the set's weight");
    }
    assert_eq!(own_weight_of_word(0), 1.0, "exactly 1");
    assert_eq!(own_weight_of_word(0xFF00_0000), 0.0, "exactly 0");
    assert_eq!(own_weight_of_word(0x0002_FFFF), 1.0, "the flags and the tip are not the weight");
}

#[test]
fn the_shaders_progress_matches_the_record_at_every_boundary() {
    let mut r = record(0.0, 4.0);
    r.start = 2.0;
    r.end = 6.0;
    for t in [0.0f32, 1.999, 2.0, 3.0, 4.0, 6.0, 9.0] {
        assert_eq!(progress_f32(r.start, r.end, t), r.progress_at(t as f64), "t = {t}");
    }
    assert_eq!(progress_f32(2.0, 6.0, 4.0), 0.5);
    assert_eq!(progress_f32(2.0, 2.0, 1.0), 0.0, "end <= start: nothing before the start");
    assert_eq!(progress_f32(2.0, 2.0, 2.0), 1.0, "end <= start: whole from the start on");
    assert_eq!(progress_f32(BrushStrokeRecord::ALWAYS, BrushStrokeRecord::ALWAYS, -5.0e6), 1.0, "always whole");
    assert_eq!(progress_f32(2.0, 6.0, f32::NAN), 0.0);
}

fn inside_polygon(points: &[PointF], x: f32, y: f32) -> bool {
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[j]);
        if (a.y > y) != (b.y > y) && x < (b.x - a.x) * (y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn distance_to_polygon(points: &[PointF], x: f32, y: f32) -> f32 {
    let mut best = f32::INFINITY;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (a, b) = (points[j], points[i]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len2 = (dx * dx + dy * dy).max(1e-12);
        let t = (((x - a.x) * dx + (y - a.y) * dy) / len2).clamp(0.0, 1.0);
        let (px, py) = (a.x + t * dx - x, a.y + t * dy - y);
        best = best.min((px * px + py * py).sqrt());
        j = i;
    }
    best
}

#[test]
fn the_coverage_mirror_agrees_with_the_cpu_twins_polygon_away_from_the_edge() {
    let mut checked = 0;
    for (bend, profile, progress) in [(0.0, 4.0, 1.0), (0.12, 4.0, 1.0), (-0.2, 2.0, 1.0), (0.08, 6.0, 0.6), (0.15, 3.0, 0.35)] {
        let mut r = record(bend, profile);
        r.start = 0.0;
        r.end = 1.0;
        let shape = r.to_shape(progress as f64).unwrap();
        let polygon = shape.get_drawn_points();
        let b = r.bounds();
        for iy in 0..60 {
            for ix in 0..60 {
                let (x, y) = (b.x - 2.0 + (b.width + 4.0) * (ix as f32 + 0.5) / 60.0, b.y - 2.0 + (b.height + 4.0) * (iy as f32 + 0.5) / 60.0);
                // Points within a margin of the polygon's edge are left to the eye: the polygon's
                // chords and the analytic curve differ there.
                if distance_to_polygon(&polygon, x, y) < 0.6 {
                    continue;
                }
                let (s, a) = to_local(&r, x, y);
                // Nor the polygon's first and last chord along the stroke: at the tips the profile
                // rises steeply and the chord from the tip cuts inside the curve.
                if s.abs() > 0.5 - 1.0 / 16.0 {
                    continue;
                }
                let analytic = stroke_coverage(&r, progress, s, a);
                let expected = if inside_polygon(&polygon, x, y) { 1.0 } else { 0.0 };
                assert_eq!(analytic, expected, "bend {bend} profile {profile} progress {progress} at ({x}, {y})");
                checked += 1;
            }
        }
    }
    assert!(checked > 10_000);
}

#[test]
fn the_hull_holds_every_point_of_the_outline() {
    for (bend, profile, progress) in [(0.0, 4.0, 1.0), (0.2, 4.0, 1.0), (-0.3, 2.0, 1.0), (0.1, 8.0, 0.5), (0.25, 3.0, 0.1), (-0.15, 1.0, 0.8)] {
        let mut r = record(bend, profile);
        r.start = 0.0;
        r.end = 1.0;
        let h = hull(r.bend * r.length, r.width / 2.0, r.profile, progress, 0.0, 0.0);
        for p in r.to_shape(progress as f64).unwrap().get_drawn_points() {
            let (s, a) = to_local(&r, p.x, p.y);
            // The strip's segment holding s, its low and high edges there.
            let i = (0..3).find(|&i| s >= h[2 * i].0 - 1e-4 && s <= h[2 * i + 2].0 + 1e-4).unwrap_or_else(|| panic!("s {s} outside the hull's length"));
            let (s0, s1) = (h[2 * i].0, h[2 * i + 2].0);
            let f = if s1 > s0 { ((s - s0) / (s1 - s0)).clamp(0.0, 1.0) } else { 0.0 };
            let lo = h[2 * i].1 + (h[2 * i + 2].1 - h[2 * i].1) * f;
            let hi = h[2 * i + 1].1 + (h[2 * i + 3].1 - h[2 * i + 1].1) * f;
            assert!(a >= lo - 1e-3 && a <= hi + 1e-3, "bend {bend} profile {profile} progress {progress}: ({s}, {a}) outside {lo}..{hi}");
        }
    }
    // Grown by a pixel each way, and tighter than the box for a bent stroke.
    let h = hull(6.0, 7.0, 4.0, 1.0, 0.1, 1.0);
    assert_eq!(h[0].0, -0.6);
    assert_eq!(h[7].0, 0.6);
    let (lo, hi) = extent(6.0, 7.0, 4.0, -0.5, 0.5);
    assert_eq!((lo, hi), (-13.0, 7.0), "the middle bows to -bow; the half width 7 either side");
    assert!(h[0].1 > lo - 1.0 && h[1].1 < hi + 1.0, "the end stations are narrower than the whole box");
}

#[test]
fn tips_are_resampled_to_their_size_with_a_mip_chain_to_one_texel() {
    //Arrange
    let half = DrawingPlane::with_value(5, 3, 0.5).unwrap();
    let ramp = DrawingPlane::from_data(4, 1, vec![0.0, 1.0, 1.0, 0.0]).unwrap();
    //Act
    let tips = BrushTips::new((64, 16), &[BrushTip::new(half.clone(), None), BrushTip::new(ramp.clone(), Some(half.clone()))]).unwrap();
    //Assert
    assert_eq!(tips.len(), 2);
    assert_eq!(tips.mip_levels(), 7, "64 x 16 down to 1 x 1: 64, 32, 16, 8, 4, 2, 1");
    let sizes: Vec<(u32, u32)> = tips.levels().iter().map(|l| (l.width, l.height)).collect();
    assert_eq!(sizes, vec![(64, 16), (32, 8), (16, 4), (8, 2), (4, 1), (2, 1), (1, 1)]);
    for level in 0..7 {
        let t0 = tips.texels(0, level).unwrap();
        assert!(t0.chunks_exact(2).all(|p| p == [128, 255]), "a flat tip: coverage 0.5 everywhere, height 1 (flat) - level {level}");
        let t1 = tips.texels(1, level).unwrap();
        assert!(t1.chunks_exact(2).all(|p| p[1] == 128), "the second tip's height plane is 0.5 - level {level}");
    }
    // The ramp resampled bilinearly to 64 wide (as DrawingPlane::resized does), then halved.
    let resized = ramp.resized(64, 16, None).unwrap();
    let level0 = tips.texels(1, 0).unwrap();
    for (k, v) in resized.data().iter().enumerate() {
        assert_eq!(level0[2 * k], (v * 255.0 + 0.5) as u8);
    }
    let level1 = tips.texels(1, 1).unwrap();
    let halved = resized.downsampled(2).unwrap();
    assert_eq!(level1[0], (halved.data()[0] * 255.0 + 0.5) as u8);
    assert_eq!(tips.bytes(), 2 * 2 * (64 * 16 + 32 * 8 + 16 * 4 + 8 * 2 + 4 + 2 + 1));
    assert!(tips.texels(2, 0).is_none() && tips.texels(0, 7).is_none());
    assert_eq!(tips.clone(), tips, "clones are the same tips");
}

#[test]
fn tips_refuse_sizes_that_do_not_halve_and_counts_out_of_range() {
    let p = DrawingPlane::with_value(2, 2, 1.0).unwrap();
    assert!(BrushTips::new((48, 16), &[BrushTip::new(p.clone(), None)]).unwrap_err().contains("power of two"));
    assert!(BrushTips::new((4096, 16), &[BrushTip::new(p.clone(), None)]).is_err());
    assert!(BrushTips::new((16, 16), &[]).unwrap_err().contains("1 to 255"));
    let many = vec![BrushTip::new(DrawingPlane::with_value(1, 1, 1.0).unwrap(), None); 256];
    assert!(BrushTips::new((1, 1), &many).is_err());
    assert!(BrushTips::new((1, 1), &many[..255]).is_ok());
}

#[test]
fn a_tip_from_a_picture_takes_its_alpha_and_its_luminance() {
    let picture = crate::images::ImageSource::from_rgba(&[255, 255, 255, 0, 0, 0, 0, 255], 2, 1).unwrap();
    let tip = BrushTip::from_image(&picture, true).unwrap();
    assert_eq!(tip.coverage.data(), &[0.0, 1.0]);
    let h = tip.height.as_ref().unwrap().data();
    assert!((h[0] - 1.0).abs() < 1e-6 && h[1] == 0.0, "white is thick, black thin");
    assert!(BrushTip::from_image(&picture, false).unwrap().height.is_none());
}

#[test]
fn two_anchors_blend_linearly_and_an_index_past_the_list_is_the_identity() {
    //Arrange
    let mut r = record(0.0, 4.0);
    r.center = PointF::new(10.0, 0.0);
    r.angle = 0.0;
    r.anchors = [0, 1];
    let anchors = [Affine::translate((100.0, 0.0)), Affine::rotate(std::f64::consts::FRAC_PI_2)];
    //Act / Assert
    r.anchor_blend = 0.0;
    let (c, a) = anchored(&r, &anchors);
    assert_eq!((c, a), (PointF::new(110.0, 0.0), 0.0));
    r.anchor_blend = 1.0;
    let (c, a) = anchored(&r, &anchors);
    assert!((c.x).abs() < 1e-5 && (c.y - 10.0).abs() < 1e-5 && (a - 90.0).abs() < 1e-4, "rotated a quarter turn: {c:?} {a}");
    r.anchor_blend = 0.5;
    let (c, a) = anchored(&r, &anchors);
    assert!((c.x - 55.0).abs() < 1e-5 && (c.y - 5.0).abs() < 1e-5 && (a - 45.0).abs() < 1e-4, "half way: {c:?} {a}");
    r.anchors = [5, 5];
    r.anchor_blend = 0.0;
    assert_eq!(anchored(&r, &anchors).0, PointF::new(10.0, 0.0), "anchor 5 is not given: the identity");
    let m = blend(Affine::scale(2.0), Affine::scale(4.0), 0.25);
    assert_eq!(m, Affine::scale(2.5));
}

#[test]
fn a_followed_element_gives_its_transform_at_the_sessions_time_and_again_after_a_seek_back() {
    //Arrange
    use crate::animation::{EasingKind, PointTrack};
    let mut s = crate::DrawingSession::default();
    let id = s.add_layer("Figure", Color::WHITE).add_element(crate::shapes::CircleShape::new(0.0, 0.0, 5.0, Default::default()).unwrap());
    let track = PointTrack::from_keys(&[(0.0, PointF::new(0.0, 0.0), EasingKind::Linear), (2.0, PointF::new(100.0, 40.0), EasingKind::Linear)]).unwrap();
    s.animations_mut().bind(id, AnimatedProperty::Position, track).unwrap();
    let anchors = vec![StrokeAnchor::FollowElement(id), StrokeAnchor::Fixed(DrawingTransform::new(PointF::new(5.0, 6.0), DrawingTransformOptions::default()).unwrap())];
    let mut reports = Vec::new();
    //Act
    s.set_time(1.0);
    let at_one = resolve_anchors(&anchors, s.layers(), &mut |m| reports.push(m));
    s.set_time(2.0);
    s.set_time(1.0);
    let again = resolve_anchors(&anchors, s.layers(), &mut |m| reports.push(m));
    //Assert
    assert_eq!(at_one[0], Affine::translate((50.0, 20.0)));
    assert_eq!(at_one[1], Affine::translate((5.0, 6.0)));
    assert_eq!(again, at_one);
    assert!(reports.is_empty());
}

#[test]
fn an_anchor_on_an_element_the_session_does_not_have_is_reported_once_and_is_the_identity() {
    //Arrange
    let mut s = crate::DrawingSession::default();
    let stranger = crate::shapes::DrawingShape::from(crate::shapes::CircleShape::new(0.0, 0.0, 1.0, Default::default()).unwrap()).id();
    let set = BrushStrokeSet::new(vec![record(0.0, 4.0)]).unwrap();
    let shape = crate::shapes::BrushStrokesShape::new(set, Default::default()).unwrap().with_anchors(vec![StrokeAnchor::FollowElement(stranger)]).unwrap();
    s.add_layer("Paint", Color::WHITE).add_shape(shape);
    let mut scene = crate::rendering::VectorScene::new();
    //Act
    s.render(&mut scene, 640, 360);
    s.render(&mut scene, 640, 360);
    //Assert
    let reports = s.take_reports();
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].contains("which the session does not have"));
    let draw = &scene.stroke_draws[0];
    assert_eq!(draw.anchors, vec![Affine::IDENTITY]);
}

#[test]
fn the_boil_redraw_number_is_the_floor_of_time_times_rate() {
    assert_eq!(boil_epoch(0.0, 10.0), 0);
    assert_eq!(boil_epoch(0.0999, 10.0), 0);
    assert_eq!(boil_epoch(0.1, 10.0), 1);
    assert_eq!(boil_epoch(1.25, 8.0), 10);
    assert_eq!(boil_epoch(-0.05, 10.0), -1, "before zero: the redraws count down");
    assert_eq!(boil_epoch(-0.1, 10.0), -1);
    assert_eq!(boil_epoch(-0.1001, 10.0), -2);
    assert_eq!(boil_epoch(5.0, 0.0), 0);
    assert_eq!(boil_epoch(5.0, -3.0), 0);
    assert_eq!(boil_epoch(f64::NAN, 10.0), 0);
    assert_eq!(boil_epoch(5.0, f32::INFINITY), 0);
    // The sketch helper's boiling uses the same rule.
    let o = crate::paths::SketchOptions::default();
    assert_eq!(o.boiled(1.25, 8.0).seed, o.seed + 10);
}

#[test]
fn the_boil_jitter_mirror_is_the_32_bit_hash_by_hand() {
    // noise_hash(7, 12345, 3, k) for k = 0..3, computed by hand (a Python transcription of the
    // WGSL): 0x463f3be9, 0x8623621d, 0x0af61101, 0x64789802.
    let boil = StrokeBoil { rate: 10.0, offset: 2.0, turn: 5.0, size: 0.1, seed: 7 };
    let j = boil_jitter(&boil, 12345, 3);
    let unit = |h: u32| (h as f32 / 4_294_967_295.0) * 2.0 - 1.0;
    assert_eq!(j.dx, 2.0 * unit(0x463f_3be9));
    assert_eq!(j.dy, 2.0 * unit(0x8623_621d));
    assert_eq!(j.turn_degrees, 5.0 * unit(0x0af6_1101));
    assert_eq!(j.size, 1.0 + 0.1 * unit(0x6478_9802));
    assert_eq!(hash_unit(0), -1.0);
    assert_eq!(hash_unit(u32::MAX), 1.0);
    assert_eq!(boil_jitter(&boil, 12345, 3), j, "the same redraw, the same jitter");
    assert_ne!(boil_jitter(&boil, 12345, 4), j, "the next redraw differs");
    assert!(StrokeBoil { size: 1.0, ..boil }.validate().is_err());
    assert!(StrokeBoil { rate: f32::NAN, ..boil }.validate().is_err());
}

#[test]
fn a_frame_draws_the_records_up_to_the_last_one_that_has_started() {
    let mut rs: Vec<BrushStrokeRecord> = (0..5).map(|_| record(0.0, 4.0)).collect();
    for (r, start) in rs.iter_mut().zip([5.0, 1.0, 9.0, 3.0, 20.0]) {
        r.start = start;
        r.end = start + 1.0;
    }
    let set = BrushStrokeSet::new(rs).unwrap();
    assert_eq!(set.drawn_count(0.5), 0);
    assert_eq!(set.drawn_count(1.0), 2, "record 1 has started: records 0 and 1 are drawn (0 shows nothing yet)");
    assert_eq!(set.drawn_count(4.0), 4);
    assert_eq!(set.drawn_count(19.0), 4);
    assert_eq!(set.drawn_count(20.0), 5);
    let always = BrushStrokeSet::new(vec![record(0.0, 4.0); 3]).unwrap();
    assert_eq!(always.drawn_count(-1.0e9), 3);
    assert!((record(0.0, 4.0).length / 2.0) <= always.reach());
}

#[test]
fn draw_blocks_are_256_byte_aligned_and_hold_the_anchors() {
    // The header: 7 vec4 of the stroke's own and 37 of the picked colour (704 bytes).
    assert_eq!(crate::strokes::stroke_gpu::DRAW_HEADER_VEC4S, 44);
    assert_eq!(draw_block_bytes(0), 768);
    assert_eq!(draw_block_bytes(4), 1024, "704 + 128 bytes");
    assert_eq!(draw_block_bytes(5), 1024);
    assert_eq!(draw_block_bytes(256), 8960, "704 + 8192 bytes, padded");
    assert!(crate::strokes::stroke_gpu::DRAW_BINDING_BYTES <= 65536, "within WebGPU's uniform binding limit");
}
