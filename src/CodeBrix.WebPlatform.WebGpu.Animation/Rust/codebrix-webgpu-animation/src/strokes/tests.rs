//! Tests of the stroke-set data: record validation messages (kept equal to BrushStrokeShape's),
//! progress at the boundaries, the CPU twin, set bounds with the bend, anchor bounds and
//! started counts.

use crate::color::Color;
use crate::geometry::PointF;
use crate::shapes::{BrushStrokeShape, BrushStrokeShapeOptions};
use crate::strokes::{BrushStrokeRecord, BrushStrokeSet};

const RED: Color = Color::from_rgb(200, 30, 40);

fn record() -> BrushStrokeRecord {
    BrushStrokeRecord::new(PointF::new(50.0, 40.0), 20.0, 6.0, 0.0, RED)
}

fn timed(start: f32, end: f32) -> BrushStrokeRecord {
    BrushStrokeRecord { start, end, ..record() }
}

fn close(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

#[test]
fn new_record_is_always_whole_with_the_defaults() {
    let r = record();
    assert_eq!(r.start, BrushStrokeRecord::ALWAYS);
    assert_eq!(r.end, BrushStrokeRecord::ALWAYS);
    assert_eq!(r.profile, BrushStrokeShape::DEFAULT_PROFILE);
    assert_eq!((r.bend, r.load, r.tip, r.anchors, r.anchor_blend, r.seed), (0.0, 1.0, 0, [0, 0], 0.0, 0));
    assert_eq!(r.progress_at(-1.0e20), 1.0);
    assert_eq!(r.progress_at(0.0), 1.0);
    assert_eq!(r.progress_at(1.0e9), 1.0);
}

#[test]
fn progress_at_the_boundaries() {
    let r = timed(2.0, 6.0);
    assert_eq!(r.progress_at(1.999), 0.0);
    assert_eq!(r.progress_at(2.0), 0.0);
    assert_eq!(r.progress_at(4.0), 0.5);
    assert_eq!(r.progress_at(5.0), 0.75);
    assert_eq!(r.progress_at(6.0), 1.0);
    assert_eq!(r.progress_at(60.0), 1.0);
    assert_eq!(r.progress_at(f64::NAN), 0.0);
    // end == start: whole from the start on, nothing before.
    let instant = timed(3.0, 3.0);
    assert_eq!(instant.progress_at(2.99), 0.0);
    assert_eq!(instant.progress_at(3.0), 1.0);
    assert_eq!(instant.progress_at(10.0), 1.0);
    // end < start (unusable in a set) still answers by the formula's first rule.
    let backwards = timed(5.0, 1.0);
    assert_eq!(backwards.progress_at(4.0), 0.0);
    assert_eq!(backwards.progress_at(5.0), 1.0);
}

#[test]
fn to_shape_equals_brush_stroke_shape_with_progress() {
    let r = BrushStrokeRecord { bend: 0.12, profile: 3.0, angle: 33.0, ..timed(1.0, 3.0) };
    let options = BrushStrokeShapeOptions { bend: 0.12, profile: 3.0, color: Some(RED) };
    for t in [0.0, 1.0, 1.5, 2.2, 3.0, 9.0] {
        let expected = BrushStrokeShape::new(50.0, 40.0, 20.0, 6.0, 33.0, options).unwrap().with_progress(r.progress_at(t));
        assert_eq!(r.to_shape(t).unwrap(), expected, "at {t}");
    }
}

#[test]
fn to_shape_multiplies_alpha_by_load() {
    let r = BrushStrokeRecord { load: 0.5, color: Color::from_rgba(10, 20, 30, 201), ..record() };
    assert_eq!(r.to_shape(0.0).unwrap().color(), Some(Color::from_rgba(10, 20, 30, 101)));
}

#[test]
fn from_shape_round_trips_geometry_and_colour() {
    let shape = BrushStrokeShape::new(5.0, 6.0, 30.0, 4.0, -20.0, BrushStrokeShapeOptions { bend: -0.1, profile: 2.0, color: Some(RED) }).unwrap().with_progress(0.3);
    let r = BrushStrokeRecord::from_shape(&shape);
    assert_eq!((r.center, r.length, r.width, r.angle, r.bend, r.profile, r.color), (PointF::new(5.0, 6.0), 30.0, 4.0, -20.0, -0.1, 2.0, RED));
    assert_eq!((r.start, r.end, r.load), (BrushStrokeRecord::ALWAYS, BrushStrokeRecord::ALWAYS, 1.0));
    assert_eq!(r.to_shape(0.0).unwrap(), shape.with_progress(1.0));
    let layer_coloured = BrushStrokeShape::new(5.0, 6.0, 30.0, 4.0, 0.0, BrushStrokeShapeOptions::default()).unwrap();
    assert_eq!(BrushStrokeRecord::from_shape(&layer_coloured).color, Color::BLACK);
}

#[test]
fn record_messages_are_brush_stroke_shapes_own() {
    let cases: Vec<(BrushStrokeRecord, Result<BrushStrokeShape, String>)> = vec![
        (BrushStrokeRecord { center: PointF::new(f32::NAN, 0.0), ..record() }, BrushStrokeShape::new(f32::NAN, 0.0, 20.0, 6.0, 0.0, BrushStrokeShapeOptions::default())),
        (BrushStrokeRecord { length: 0.0, ..record() }, BrushStrokeShape::new(0.0, 0.0, 0.0, 6.0, 0.0, BrushStrokeShapeOptions::default())),
        (BrushStrokeRecord { width: -1.0, ..record() }, BrushStrokeShape::new(0.0, 0.0, 20.0, -1.0, 0.0, BrushStrokeShapeOptions::default())),
        (BrushStrokeRecord { angle: f32::INFINITY, ..record() }, BrushStrokeShape::new(0.0, 0.0, 20.0, 6.0, f32::INFINITY, BrushStrokeShapeOptions::default())),
        (BrushStrokeRecord { bend: f32::NAN, ..record() }, BrushStrokeShape::new(0.0, 0.0, 20.0, 6.0, 0.0, BrushStrokeShapeOptions { bend: f32::NAN, ..BrushStrokeShapeOptions::default() })),
        (BrushStrokeRecord { profile: 0.0, ..record() }, BrushStrokeShape::new(0.0, 0.0, 20.0, 6.0, 0.0, BrushStrokeShapeOptions { profile: 0.0, ..BrushStrokeShapeOptions::default() })),
    ];
    for (r, shape) in cases {
        let expected = shape.unwrap_err();
        assert_eq!(r.validate().unwrap_err(), expected);
        assert_eq!(r.to_shape(0.0).unwrap_err(), expected);
    }
}

#[test]
fn set_errors_name_the_record() {
    let bad = |r: BrushStrokeRecord| BrushStrokeSet::new(vec![record(), record(), r]).unwrap_err();
    assert_eq!(bad(BrushStrokeRecord { length: 0.0, ..record() }), "Brushstroke record 2 is unusable: A brushstroke's length must be a positive, finite distance.");
    assert!(bad(timed(3.0, 2.0)).contains("record 2") && bad(timed(3.0, 2.0)).contains("end must not be before its start"));
    assert!(bad(timed(f32::NAN, 2.0)).contains("start and end must be finite"));
    assert!(bad(BrushStrokeRecord { load: 1.5, ..record() }).contains("load must be a fraction"));
    assert!(bad(BrushStrokeRecord { load: f32::NAN, ..record() }).contains("load must be a fraction"));
    assert!(bad(BrushStrokeRecord { anchor_blend: -0.1, ..record() }).contains("anchor blend"));
    assert!(BrushStrokeSet::new(vec![BrushStrokeRecord { load: 0.0, anchor_blend: 1.0, ..timed(1.0, 1.0) }]).is_ok());
}

#[test]
fn set_bounds_include_the_bend() {
    // Horizontal, straight: exactly the stroke's rectangle.
    let straight = BrushStrokeSet::new(vec![record()]).unwrap();
    assert_eq!(straight.bounds().x, 40.0);
    assert_eq!(straight.bounds().width, 20.0);
    assert_eq!(straight.bounds().y, 37.0);
    assert_eq!(straight.bounds().height, 6.0);
    // Bent by 0.25 of 20 = 5 to the left of travel (up on screen for angle 0).
    let bent = BrushStrokeSet::new(vec![BrushStrokeRecord { bend: 0.25, ..record() }]).unwrap();
    assert!(close(bent.bounds().y, 32.0, 1e-4) && close(bent.bounds().bottom(), 43.0, 1e-4), "{:?}", bent.bounds());
    // The CPU twin's polygon lies inside the box, turned or not.
    for angle in [0.0, 30.0, 90.0, 137.0, -60.0] {
        for bend in [-0.3, 0.0, 0.2] {
            let r = BrushStrokeRecord { angle, bend, ..record() };
            let b = BrushStrokeSet::new(vec![r]).unwrap().bounds();
            for p in r.to_shape(0.0).unwrap().get_points() {
                assert!(p.x >= b.x - 1e-3 && p.x <= b.right() + 1e-3 && p.y >= b.y - 1e-3 && p.y <= b.bottom() + 1e-3, "{angle} {bend} {p:?} {b:?}");
            }
        }
    }
    // Several strokes: the union.
    let two = BrushStrokeSet::new(vec![record(), BrushStrokeRecord { center: PointF::new(100.0, 90.0), ..record() }]).unwrap();
    assert_eq!((two.bounds().x, two.bounds().y, two.bounds().right(), two.bounds().bottom()), (40.0, 37.0, 110.0, 93.0));
    let empty = BrushStrokeSet::new(Vec::new()).unwrap();
    assert!(empty.is_empty() && empty.bounds().is_empty() && empty.first_start() == f32::INFINITY && empty.anchor_bounds().is_empty());
}

#[test]
fn anchor_bounds_follow_the_anchors_each_stroke_uses() {
    let a = BrushStrokeRecord { anchors: [0, 0], ..record() };
    let b = BrushStrokeRecord { center: PointF::new(200.0, 40.0), anchors: [2, 3], anchor_blend: 0.0, ..record() };
    let c = BrushStrokeRecord { center: PointF::new(300.0, 40.0), anchors: [1, 3], anchor_blend: 0.5, ..record() };
    let d = BrushStrokeRecord { center: PointF::new(400.0, 40.0), anchors: [0, 1], anchor_blend: 1.0, ..record() };
    let set = BrushStrokeSet::new(vec![a, b, c, d]).unwrap();
    let ab = set.anchor_bounds();
    assert_eq!(ab.len(), 4);
    assert_eq!((ab[0].x, ab[0].right()), (40.0, 60.0));
    assert_eq!((ab[1].x, ab[1].right()), (290.0, 410.0));
    assert_eq!((ab[2].x, ab[2].right()), (190.0, 210.0));
    assert_eq!((ab[3].x, ab[3].right()), (290.0, 310.0));
}

#[test]
fn started_count_and_first_start() {
    let set = BrushStrokeSet::new(vec![timed(1.0, 2.0), timed(3.0, 3.0), timed(0.5, 4.0), timed(5.0, 6.0)]).unwrap();
    assert_eq!(set.first_start(), 0.5);
    assert_eq!(set.started_count(0.0), 0);
    assert_eq!(set.started_count(0.5), 1);
    assert_eq!(set.started_count(2.9), 2);
    assert_eq!(set.started_count(3.0), 3);
    assert_eq!(set.started_count(100.0), 4);
    let always = BrushStrokeSet::new(vec![record(), timed(5.0, 6.0)]).unwrap();
    assert_eq!(always.first_start(), BrushStrokeRecord::ALWAYS);
    assert_eq!(always.started_count(0.0), 1);
}

#[test]
fn clones_share_one_identity_and_sets_do_not() {
    let set = BrushStrokeSet::new(vec![record()]).unwrap();
    let clone = set.clone();
    let other = BrushStrokeSet::new(vec![record()]).unwrap();
    assert_eq!(set.id(), clone.id());
    assert_eq!(set, clone);
    assert_ne!(set.id(), other.id());
    assert_ne!(set, other);
    assert!(std::ptr::eq(set.records().as_ptr(), clone.records().as_ptr()));
    assert_eq!(set.len(), 1);
}
