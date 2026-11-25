//! Host tests of stroke sets in the renderer: a session's stroke set is ONE scene item and ONE
//! operation, its draw block, its content box from the anchors, culling, the counters, one upload
//! per set and per tips, the looks (line leaves it out, flat drops the tips), the fade as a group,
//! ownership maps at the session's time - and the sweep policy every held texture and buffer
//! follows (the dead-picture fix).

use kurbo::{Affine, Rect};

use crate::models::{DrawingTransform, DrawingTransformOptions};
use crate::planes::DrawingPlane;
use crate::rendering::core::build::{Builder, Op};
use crate::rendering::core::texture_ledger::{stroke_uploads, sweep_choice, Residency, ResidencyPolicy, TextureLedger, RESIDENCY};
use crate::rendering::{RenderStats, VectorScene};
use crate::shapes::{BrushStrokesOptions, BrushStrokesShape};
use crate::strokes::stroke_gpu::{DRAW_HEADER_VEC4S, FLAG_BOIL, FLAG_MIRROR, FLAG_TIPS};
use crate::strokes::{BrushStrokeRecord, BrushStrokeSet, BrushTip, BrushTips, StrokeAnchor, StrokeBoil};
use crate::{Color, DrawingSession, DrawingSessionOptions, PointF, SessionPaint, SessionRenderOptions};

fn stroke(x: f32, y: f32, start: f32) -> BrushStrokeRecord {
    let mut r = BrushStrokeRecord::new(PointF::new(x, y), 40.0, 10.0, 0.0, Color::from_rgb(30, 90, 200));
    r.start = start;
    r.end = start + 2.0;
    r
}

/// A session of calibrated units equal to the canvas pixels (640 x 360).
fn session() -> DrawingSession {
    DrawingSession::new(DrawingSessionOptions { calibration_size: (640, 360), layer_opacity: 255, ..DrawingSessionOptions::default() })
}

fn render(s: &DrawingSession, options: &SessionRenderOptions) -> VectorScene {
    let mut scene = VectorScene::new();
    s.render_with(&mut scene, 640, 360, options);
    scene
}

fn set_of(n: usize) -> BrushStrokeSet {
    BrushStrokeSet::new((0..n).map(|i| stroke(50.0 + (i % 20) as f32 * 25.0, 50.0 + (i / 20) as f32 * 20.0, i as f32 * 0.01)).collect()).unwrap()
}

fn tips() -> BrushTips {
    BrushTips::new((16, 8), &[BrushTip::new(DrawingPlane::with_value(4, 4, 0.8).unwrap(), None)]).unwrap()
}

#[test]
fn a_session_records_one_stroke_set_as_one_item_and_the_builder_one_operation() {
    //Arrange
    let mut s = session();
    let shape = BrushStrokesShape::new(set_of(1000), BrushStrokesOptions::default()).unwrap();
    s.add_layer("Paint", Color::WHITE);
    s.draw_brush_strokes(shape);
    s.set_time(100.0);
    let scene = render(&s, &SessionRenderOptions::default());
    let mut b = Builder::default();
    //Act
    b.build(&scene, 640, 360);
    //Assert
    assert_eq!(scene.item_kinds().matches('B').count(), 1, "{}", scene.item_kinds());
    assert_eq!(scene.draw_count(), 1);
    assert_eq!(b.ops.iter().filter(|o| matches!(o, Op::Strokes { .. })).count(), 1);
    assert_eq!(b.stats.stroke_draws, 1);
    assert_eq!(b.stats.strokes, 1000);
    assert_eq!(b.stats.vertices, b.cover.len() + b.stencil.len(), "no vertices per stroke");
    assert!(b.cover.len() <= 12, "only the layer's quads: {}", b.cover.len());
    assert_eq!(b.stroke_ops[0].instances, 1000);
}

#[test]
fn the_draw_block_holds_the_matrix_time_opacity_flags_and_anchors() {
    //Arrange
    let set = set_of(10);
    let boil = StrokeBoil { rate: 10.0, ..StrokeBoil::default() };
    let shape = BrushStrokesShape::new(set, BrushStrokesOptions { time_offset: 1.0, head_softness: 0.2, boil: Some(boil), ..Default::default() })
        .unwrap()
        .with_tips(tips())
        .with_anchors(vec![StrokeAnchor::Fixed(DrawingTransform::IDENTITY), StrokeAnchor::Fixed(DrawingTransform::new(PointF::new(7.0, 8.0), DrawingTransformOptions::default()).unwrap())])
        .unwrap();
    let mut s = session();
    s.add_layer("Paint", Color::WHITE).add_shape(shape);
    s.set_time(3.26);
    let scene = render(&s, &SessionRenderOptions::default());
    let mut b = Builder::default();
    //Act
    b.build(&scene, 640, 360);
    //Assert
    let w = &b.stroke_blocks;
    let f = |i: usize| f32::from_bits(w[i]);
    assert_eq!(b.stroke_ops[0].offset, 0);
    assert_eq!(w.len() * 4, 768, "header (704 bytes: the stroke's 112 and the picked colour's 592) and two anchors (64), padded to 768");
    assert_eq!((f(0), f(1), f(2), f(3), f(4), f(5)), (1.0, 0.0, 0.0, 1.0, 0.0, 0.0), "the identity: calibration = canvas");
    assert!((f(6) - 2.26).abs() < 1e-5, "the session time minus the offset");
    assert_eq!(f(7), 1.0, "the opacity");
    assert_eq!(f(8), 0.2, "the head softness");
    assert_eq!(f(9), 0.0, "no impasto off a finished layer");
    assert!(w[28..176].iter().all(|v| *v == 0), "own colours: the picked colour's part is zero");
    assert_eq!(w[12], FLAG_TIPS | FLAG_MIRROR | FLAG_BOIL);
    assert_eq!(w[13], 2, "two anchors");
    assert_eq!(w[14] as i32, 22, "redraw floor(2.26 x 10)");
    assert_eq!(f(DRAW_HEADER_VEC4S * 4 + 8 + 4), 7.0, "the second anchor's e");
    assert_eq!(f(DRAW_HEADER_VEC4S * 4 + 8 + 5), 8.0, "the second anchor's f");
}

#[test]
fn the_content_box_is_the_anchors_boxes_through_their_transforms() {
    //Arrange: strokes on anchor 0 (identity) and anchor 1 (moved 300 right).
    let mut a = stroke(100.0, 100.0, BrushStrokeRecord::ALWAYS);
    a.end = BrushStrokeRecord::ALWAYS;
    let mut c = a;
    c.anchors = [1, 1];
    let set = BrushStrokeSet::new(vec![a, c]).unwrap();
    let moved = DrawingTransform::new(PointF::new(300.0, 0.0), DrawingTransformOptions::default()).unwrap();
    let shape = BrushStrokesShape::new(set, BrushStrokesOptions::default()).unwrap().with_anchors(vec![StrokeAnchor::Fixed(DrawingTransform::IDENTITY), StrokeAnchor::Fixed(moved)]).unwrap();
    let mut s = session();
    s.add_layer("Paint", Color::WHITE).add_shape(shape);
    let scene = render(&s, &SessionRenderOptions::default());
    let mut b = Builder::default();
    //Act
    b.build(&scene, 640, 360);
    //Assert - each stroke 40 x 10 about its centre, grown by two device pixels.
    assert_eq!(b.root_content, Rect::new(78.0, 93.0, 422.0, 107.0));
}

#[test]
fn a_set_is_culled_before_its_first_stroke_starts_and_off_the_canvas() {
    let mut s = session();
    s.add_layer("Paint", Color::WHITE).add_shape(BrushStrokesShape::new(set_of(5), Default::default()).unwrap());
    let far = BrushStrokesShape::new(set_of(5), Default::default()).unwrap().with_anchors(vec![StrokeAnchor::Fixed(DrawingTransform::new(PointF::new(5000.0, 0.0), DrawingTransformOptions::default()).unwrap())]).unwrap();
    s.get_layer_mut("Paint").unwrap().add_shape(far);
    let mut b = Builder::default();
    s.set_time(-1.0);
    b.build(&render(&s, &SessionRenderOptions::default()), 640, 360);
    assert_eq!((b.stats.draws, b.stats.culled, b.stats.stroke_draws), (2, 2, 0), "before the first start: nothing to draw");
    s.set_time(0.005);
    b.build(&render(&s, &SessionRenderOptions::default()), 640, 360);
    assert_eq!((b.stats.culled, b.stats.stroke_draws, b.stats.strokes), (1, 1, 1), "one started stroke on the canvas; the far set culled");
}

#[test]
fn a_set_and_its_tips_are_uploaded_once_however_often_they_are_drawn() {
    //Arrange: the same set (a clone) drawn twice in a frame, with the same tips.
    let set = set_of(100);
    let t = tips();
    let mut s = session();
    let layer = s.add_layer("Paint", Color::WHITE);
    layer.add_shape(BrushStrokesShape::new(set.clone(), Default::default()).unwrap().with_tips(t.clone()));
    layer.add_shape(BrushStrokesShape::new(set.clone(), Default::default()).unwrap().with_tips(t.clone()));
    s.set_time(10.0);
    let mut b = Builder::default();
    let (mut sets, mut tip_sets) = (Residency::default(), Residency::default());
    let mut frame = |n: u64, b: &mut Builder| {
        b.build(&render(&s, &SessionRenderOptions::default()), 640, 360);
        let mut stats = RenderStats::default();
        for op in &b.stroke_ops {
            stroke_uploads(&mut sets, &mut tip_sets, op, n, &|_, _| true, &mut stats);
        }
        stats
    };
    //Act
    let first = frame(1, &mut b);
    let second = frame(2, &mut b);
    //Assert
    assert_eq!(b.stats.stroke_draws, 2, "two draws");
    assert_eq!((first.stroke_uploads, first.stroke_upload_bytes), (1, 100 * 64));
    assert_eq!((first.tip_uploads, first.tip_upload_bytes), (1, t.bytes()));
    assert_eq!((second.stroke_uploads, second.tip_uploads), (0, 0), "nothing the next frame");
    // A new set is a new upload; one the GPU lost is uploaded again.
    let other = crate::rendering::core::build::StrokeOp { set: set_of(3), tips: None, source: crate::rendering::core::build::StrokeSource::None, offset: 0, instances: 3, checkpoint: None, morph: None };
    let mut stats = RenderStats::default();
    assert_eq!(stroke_uploads(&mut sets, &mut tip_sets, &other, 3, &|_, _| true, &mut stats), (true, false));
    assert_eq!(stroke_uploads(&mut sets, &mut tip_sets, &other, 3, &|_, _| false, &mut stats), (true, false));
    assert_eq!(stats.stroke_upload_bytes, 2 * 3 * 64);
}

#[test]
fn a_line_drawing_leaves_a_set_out_and_a_flat_one_drops_its_tips() {
    //Arrange
    let mut s = session();
    s.add_layer("Paint", Color::WHITE).add_shape(BrushStrokesShape::new(set_of(10), Default::default()).unwrap().with_tips(tips()));
    s.set_time(10.0);
    let ink = crate::LinePaint::new(crate::paths::PathStrokeStyle::new(1.0, Default::default()).unwrap(), Color::BLACK);
    //Act
    let authored = render(&s, &SessionRenderOptions::default());
    let line = render(&s, &SessionRenderOptions { paint: SessionPaint::Line(ink), ..Default::default() });
    let flat = render(&s, &SessionRenderOptions { paint: SessionPaint::Flat, ..Default::default() });
    //Assert
    assert!(authored.stroke_draws[0].tips.is_some());
    assert!(!line.item_kinds().contains('B'), "{}", line.item_kinds());
    assert!(flat.item_kinds().contains('B'));
    assert!(flat.stroke_draws[0].tips.is_none(), "flat: each stroke its own colour, no tip");
    let exempt = render(&s, &SessionRenderOptions { paint: SessionPaint::Flat, authored_layers: vec!["Paint".into()], ..Default::default() });
    assert!(exempt.stroke_draws[0].tips.is_some(), "a layer drawn as authored keeps its tips");
}

#[test]
fn a_faded_set_is_drawn_whole_and_faded_as_a_group() {
    let mut s = session();
    let shape = crate::shapes::DrawingShape::from(BrushStrokesShape::new(set_of(10), Default::default()).unwrap()).with_opacity(0.5);
    s.add_layer("Paint", Color::WHITE).add_shape(shape);
    s.set_time(10.0);
    let scene = render(&s, &SessionRenderOptions::default());
    assert!(scene.item_kinds().contains("LBP"), "{}", scene.item_kinds());
    assert_eq!(scene.stroke_draws[0].opacity, 1.0);
}

#[test]
fn an_ownership_map_takes_the_set_as_one_element_painted_to_the_sessions_time() {
    //Arrange: one stroke painted on from 0 to 2 s.
    let mut s = session();
    let mut r = BrushStrokeRecord::new(PointF::new(320.0, 180.0), 200.0, 40.0, 0.0, Color::WHITE);
    r.start = 0.0;
    r.end = 2.0;
    s.add_layer("Paint", Color::WHITE).add_shape(BrushStrokesShape::new(BrushStrokeSet::new(vec![r]).unwrap(), Default::default()).unwrap());
    let owned = |s: &DrawingSession| s.export_ownership_map((640, 360)).unwrap().owners().iter().filter(|o| **o >= 0).count();
    //Act
    s.set_time(-1.0);
    let before = owned(&s);
    s.set_time(1.0);
    let half = owned(&s);
    s.set_time(5.0);
    let whole = owned(&s);
    //Assert
    assert_eq!(before, 0);
    assert!(half > 0 && whole > half, "{half} then {whole}");
    let ratio = half as f64 / whole as f64;
    assert!((ratio - 0.5).abs() < 0.05, "half painted covers about half: {ratio}");
    assert_eq!(s.export_ownership_map((64, 36)).unwrap().element_count(), 1);
}

#[test]
fn the_sweep_drops_what_was_not_drawn_for_the_keep_frames() {
    let policy = ResidencyPolicy { keep_frames: 120, idle_budget_bytes: 1000 };
    let entries = [(1u64, 100, 10), (2, 0, 10), (3, 200, 10), (4, 79, 10)];
    assert_eq!(sweep_choice(&entries, 200, policy), vec![2, 4], "unused for more than 120 frames");
    assert!(sweep_choice(&entries, 199, policy).contains(&2));
    assert!(!sweep_choice(&entries, 199, policy).contains(&4), "79 + 120 = 199 is still kept");
}

#[test]
fn over_the_budget_the_least_recently_drawn_idle_entries_go_first_but_never_the_last_two_frames() {
    let policy = ResidencyPolicy { keep_frames: 120, idle_budget_bytes: 25 };
    // At frame 100: drawn now (A), last frame (B), older (C at 90, D at 95, E at 50).
    let entries = [('A', 100, 50), ('B', 99, 10), ('C', 90, 10), ('D', 95, 10), ('E', 50, 10)];
    // Idle: B C D E = 40 bytes > 25; B (last frame) is never a candidate; E then C go.
    assert_eq!(sweep_choice(&entries, 100, policy), vec!['E', 'C']);
    // Under the budget nothing goes before the keep frames.
    assert!(sweep_choice(&entries, 100, ResidencyPolicy { keep_frames: 120, idle_budget_bytes: 40 }).is_empty());
}

#[test]
fn a_new_picture_every_frame_keeps_only_about_the_budget_of_dead_textures() {
    //Arrange: 1 MiB pictures, a new one each frame for 300 frames (the cache bug's scene).
    let mut ledger = TextureLedger::default();
    let mut held_most = 0;
    //Act
    for frame in 1..=300u64 {
        ledger.acquire_bytes((frame, 512), None, frame, 1 << 20);
        ledger.sweep(frame, RESIDENCY);
        held_most = held_most.max(ledger.textures());
    }
    //Assert - the budget is 16 MiB of idle textures: 16 dead + the one drawn now; before the
    // fix up to 180 were held (unused for 120 frames, swept every 60).
    assert_eq!(held_most, 17);
    assert_eq!(ledger.textures(), 17);
}

#[test]
fn pictures_drawn_every_other_frame_or_small_ones_left_idle_are_kept() {
    let mut ledger = TextureLedger::default();
    for frame in 1..=200u64 {
        // Two 10 MiB pictures in turn: each is idle one frame in two, never two frames running.
        let key = (frame % 2, 1024);
        ledger.acquire_bytes(key, None, frame, 10 << 20);
        ledger.sweep(frame, RESIDENCY);
        assert_eq!(ledger.textures(), if frame == 1 { 1 } else { 2 }, "frame {frame}");
    }
    // A small picture left idle stays the keep frames, then goes.
    let mut small = TextureLedger::default();
    small.acquire_bytes((9, 8), None, 1, 256);
    for frame in 2..=121u64 {
        small.sweep(frame, RESIDENCY);
    }
    assert_eq!(small.textures(), 1);
    small.sweep(122, RESIDENCY);
    assert_eq!(small.textures(), 0);
}

#[test]
fn stroke_sets_and_tips_follow_the_same_policy() {
    let mut sets: Residency<u64> = Residency::default();
    for frame in 1..=100u64 {
        // A new 4 MiB set every frame.
        assert!(sets.use_now(frame, 4 << 20, frame));
        sets.sweep(frame, RESIDENCY);
        assert!(sets.len() <= 6, "frame {frame}: {}", sets.len());
    }
    let mut kept: Residency<u64> = Residency::default();
    kept.use_now(1, 64, 1);
    assert!(!kept.use_now(1, 64, 2), "held: no upload");
    assert!(kept.sweep(122, RESIDENCY).is_empty());
    assert_eq!(kept.sweep(123, RESIDENCY), vec![1]);
    assert!(kept.use_now(1, 64, 124), "dropped: uploaded again when drawn again");
    let _ = Affine::IDENTITY;
}
