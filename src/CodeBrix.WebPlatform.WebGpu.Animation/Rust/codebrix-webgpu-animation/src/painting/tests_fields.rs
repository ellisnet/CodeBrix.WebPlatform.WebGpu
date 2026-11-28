//! Tests of direction fields, guides, detail maps and stroke layout: synthetic pictures (an edge,
//! a disc, a flat picture), doubled-angle blending, guides' directions, weights and reach, a
//! stroke's pose on a circular field, streamlines, determinism, and layouts band by band.

use crate::color::Color;
use crate::formations::FormationArea;
use crate::geometry::{PointF, RectangleF};
use crate::painting::{element_area, DetailMap, DirectionField, EdgeDetailOptions, DirectionGuide, NoiseFieldOptions, PictureFieldOptions, StrokeLayout, StrokeLayoutOptions, StrokeSize};
use crate::paths::{PathGeometry, PathGeometryBuilder};
use crate::planes::{DrawingNoise, DrawingNoiseFractalOptions, DrawingNoiseKind, DrawingPlane, DrawingPlaneImage};
use crate::shapes::{BrushStrokeShape, BrushStrokeShapeOptions};
use crate::{DrawingSession, DrawingSessionOptions};

/// Two directions compared as directions (modulo 180 degrees).
fn same_direction(a: f32, b: f32, tolerance: f32) -> bool {
    let d = (a - b).rem_euclid(180.0);
    d <= tolerance || d >= 180.0 - tolerance
}

fn picture(width: u32, height: u32, mut white: impl FnMut(u32, u32) -> bool) -> DrawingPlaneImage {
    let mut p = DrawingPlaneImage::new(width, height).unwrap();
    p.fill_with(|x, y| if white(x, y) { [1.0, 1.0, 1.0, 1.0] } else { [0.0, 0.0, 0.0, 1.0] });
    p
}

fn square(size: f32) -> RectangleF {
    RectangleF::new(0.0, 0.0, size, size)
}

fn fixed(angle: f32) -> DirectionGuide {
    DirectionGuide::Fixed { angle, center: PointF::new(0.0, 0.0), reach: f32::INFINITY, weight: 1.0 }
}

#[test]
fn a_vertical_edge_gives_ninety_degrees_along_it() {
    let p = picture(40, 40, |x, _| x >= 20);
    let field = DirectionField::from_picture(&p, square(40.0), PictureFieldOptions { smoothing: 2.0, ..PictureFieldOptions::default() }).unwrap();
    for y in [5.0, 20.0, 35.0] {
        assert!(same_direction(field.angle_at(20.0, y), 90.0, 0.5), "{}", field.angle_at(20.0, y));
        assert!(field.strength_at(20.0, y) > 0.95, "{}", field.strength_at(20.0, y));
    }
    let horizontal = DirectionField::from_picture(&picture(40, 40, |_, y| y >= 20), square(40.0), PictureFieldOptions::default()).unwrap();
    assert!(same_direction(horizontal.angle_at(20.0, 20.0), 0.0, 0.5));
}

#[test]
fn a_filled_disc_gives_tangents_on_its_rim() {
    let p = picture(64, 64, |x, y| ((x as f32 + 0.5 - 32.0).powi(2) + (y as f32 + 0.5 - 32.0).powi(2)) <= 144.0);
    let field = DirectionField::from_picture(&p, square(64.0), PictureFieldOptions { smoothing: 1.5, ..PictureFieldOptions::default() }).unwrap();
    for degrees in [0.0f32, 45.0, 90.0, 135.0, 200.0, 300.0] {
        let r = degrees.to_radians();
        let (x, y) = (32.0 + (12.0 * r.cos()), 32.0 + (12.0 * r.sin()));
        assert!(same_direction(field.angle_at(x, y), degrees + 90.0, 6.0), "at {degrees}: {}", field.angle_at(x, y));
        assert!(field.strength_at(x, y) > 0.5);
    }
}

#[test]
fn a_flat_picture_has_no_strength() {
    let field = DirectionField::from_picture(&picture(16, 16, |_, _| true), square(16.0), PictureFieldOptions::default()).unwrap();
    for (x, y) in [(1.0, 1.0), (8.0, 8.0), (15.0, 3.0)] {
        assert_eq!(field.strength_at(x, y), 0.0);
    }
    // A smaller field size is made from the resized picture.
    let small = DirectionField::from_picture(&picture(40, 40, |x, _| x >= 20), square(40.0), PictureFieldOptions { field_size: Some((10, 10)), smoothing: 1.0 }).unwrap();
    assert_eq!(small.size(), (10, 10));
    assert!(same_direction(small.angle_at(20.0, 20.0), 90.0, 1.0));
}

#[test]
fn doubled_angles_blend_one_and_179_degrees_to_zero() {
    let a = DirectionField::from_guides((4, 4), square(40.0), &[fixed(1.0)]).unwrap();
    let b = DirectionField::from_guides((4, 4), square(40.0), &[fixed(179.0)]).unwrap();
    assert!((a.angle_at(20.0, 20.0) - 1.0).abs() < 1.0e-3);
    assert!((b.angle_at(20.0, 20.0) + 1.0).abs() < 1.0e-3, "179 degrees reads as -1: {}", b.angle_at(20.0, 20.0));
    let half = DrawingPlane::with_value(4, 4, 0.5).unwrap();
    let mixed = a.blend(&b, &half).unwrap();
    assert!(mixed.angle_at(20.0, 20.0).abs() < 1.0e-3, "{}", mixed.angle_at(20.0, 20.0));
    assert!(mixed.strength_at(20.0, 20.0) > 0.999);
    // The same with two guides in one field.
    let both = DirectionField::from_guides((4, 4), square(40.0), &[fixed(1.0), fixed(179.0)]).unwrap();
    assert!(both.angle_at(10.0, 30.0).abs() < 1.0e-3);
    // Weight 0 and 1 give each side; another-sized field is sampled at this field's cells.
    let none = DrawingPlane::with_value(1, 1, 0.0).unwrap();
    let all = DrawingPlane::with_value(9, 3, 1.0).unwrap();
    assert!((a.blend(&b, &none).unwrap().angle_at(5.0, 5.0) - 1.0).abs() < 1.0e-3);
    let other_size = DirectionField::from_guides((7, 2), square(40.0), &[fixed(60.0)]).unwrap();
    assert!((a.blend(&other_size, &all).unwrap().angle_at(5.0, 5.0) - 60.0).abs() < 1.0e-3);
    assert!(a.blend(&b, &DrawingPlane::with_value(2, 2, f32::NAN).unwrap()).is_err());
}

#[test]
fn guides_give_their_directions_with_weight_and_reach() {
    let around = DirectionField::from_guides((100, 100), square(100.0), &[DirectionGuide::Around { center: PointF::new(50.0, 50.0), reach: f32::INFINITY, inward: 0.0, weight: 1.0 }]).unwrap();
    assert!(same_direction(around.angle_at(80.5, 50.5), 90.0, 1.5));
    assert!(same_direction(around.angle_at(50.5, 20.5), 0.0, 1.5));
    assert!(same_direction(around.angle_at(71.5, 71.5), 135.0, 1.5));
    let spiral = DirectionField::from_guides((100, 100), square(100.0), &[DirectionGuide::Around { center: PointF::new(50.0, 50.0), reach: f32::INFINITY, inward: 20.0, weight: 1.0 }]).unwrap();
    assert!(same_direction(spiral.angle_at(80.5, 50.5), 110.0, 1.5));
    let radial = DirectionField::from_guides((100, 100), square(100.0), &[DirectionGuide::Radial { center: PointF::new(50.0, 50.0), reach: 10.0, weight: 1.0 }]).unwrap();
    assert!(same_direction(radial.angle_at(55.5, 50.5), 0.0, 6.0));
    assert_eq!(radial.strength_at(90.5, 90.5), 0.0, "out of reach: no pull");
    // Along a path: its tangent near it, nothing beyond the reach.
    let path = PathGeometryBuilder::new().move_to(0.0, 50.0).line_to(100.0, 50.0).build();
    let along = DirectionField::from_guides((50, 50), square(100.0), &[DirectionGuide::Along { path, reach: 30.0, weight: 1.0 }]).unwrap();
    assert!(along.angle_at(51.0, 61.0).abs() < 1.0e-3);
    assert!(along.strength_at(51.0, 61.0) > 0.999);
    assert_eq!(along.strength_at(51.0, 95.0), 0.0);
    let diagonal = PathGeometryBuilder::new().move_to(0.0, 0.0).line_to(100.0, 100.0).build();
    let field = DirectionField::from_guides((50, 50), square(100.0), &[DirectionGuide::Along { path: diagonal, reach: f32::INFINITY, weight: 1.0 }]).unwrap();
    assert!((field.angle_at(31.0, 51.0) - 45.0).abs() < 1.0e-3);
    // Weights: 0 degrees at 1 against 90 degrees at 3, one place: 90 wins, agreement 0.5.
    let c = PointF::new(5.0, 5.0);
    let weighted = DirectionField::from_guides((1, 1), square(10.0), &[DirectionGuide::Fixed { angle: 0.0, center: c, reach: 100.0, weight: 1.0 }, DirectionGuide::Fixed { angle: 90.0, center: c, reach: 100.0, weight: 3.0 }]).unwrap();
    assert!(same_direction(weighted.angle_at(5.0, 5.0), 90.0, 1.0e-3));
    assert!((weighted.strength_at(5.0, 5.0) - 0.5).abs() < 1.0e-4);
    // Nearer guides pull harder.
    let near_far = DirectionField::from_guides((10, 1), RectangleF::new(0.0, 0.0, 100.0, 10.0), &[DirectionGuide::Fixed { angle: 0.0, center: PointF::new(0.0, 5.0), reach: f32::INFINITY, weight: 1.0 }, DirectionGuide::Fixed { angle: 60.0, center: PointF::new(100.0, 5.0), reach: f32::INFINITY, weight: 1.0 }]).unwrap();
    assert!(near_far.angle_at(5.0, 5.0) < 5.0 && near_far.angle_at(95.0, 5.0) > 55.0);
}

#[test]
fn stroke_pose_on_a_circular_field_is_an_arc() {
    let field = DirectionField::from_guides((400, 400), RectangleF::new(-200.0, -200.0, 400.0, 400.0), &[DirectionGuide::Around { center: PointF::new(0.0, 0.0), reach: f32::INFINITY, inward: 0.0, weight: 1.0 }]).unwrap();
    let (radius, length) = (100.0f32, 20.0f32);
    let expected = ((length / (2.0 * radius)).atan() / 2.0).tan() / 2.0;
    for degrees in [0.0f32, 90.0, 210.0] {
        let r = degrees.to_radians();
        let center = PointF::new(radius * r.cos(), radius * r.sin());
        let (angle, bend) = field.stroke_pose(center, length);
        assert!(same_direction(angle, degrees + 90.0, 0.2), "{angle}");
        assert!((bend.abs() - expected).abs() < 1.0e-3, "{bend} vs {expected}");
        // The stroke's centre line - its two ends and its middle - lies on one circle round the
        // field's centre: the stroke curves the way the field does.
        let shape = BrushStrokeShape::new(center.x, center.y, length, 2.0, angle, BrushStrokeShapeOptions { bend, ..BrushStrokeShapeOptions::default() }).unwrap();
        let points = shape.get_points();
        let n = BrushStrokeShape::POINTS_PER_SIDE;
        let middle_upper = points[n / 2];
        let middle_lower = points[(2 * n) - 1 - (n / 2) - 1];
        let middle = PointF::new((middle_upper.x + middle_lower.x) / 2.0, (middle_upper.y + middle_lower.y) / 2.0);
        let distance = |p: PointF| p.x.hypot(p.y);
        assert!((distance(middle) - distance(points[0])).abs() < 0.02, "{} vs {}", distance(middle), distance(points[0]));
    }
    // A straight field gives no bend.
    let straight = DirectionField::from_guides((4, 4), square(40.0), &[fixed(30.0)]).unwrap();
    let (angle, bend) = straight.stroke_pose(PointF::new(20.0, 20.0), 10.0);
    assert!((angle - 30.0).abs() < 1.0e-3 && bend.abs() < 1.0e-5);
}

#[test]
fn trace_stays_on_a_circle() {
    let field = DirectionField::from_guides((400, 400), RectangleF::new(-200.0, -200.0, 400.0, 400.0), &[DirectionGuide::Around { center: PointF::new(0.0, 0.0), reach: f32::INFINITY, inward: 0.0, weight: 1.0 }]).unwrap();
    let points = field.trace(PointF::new(100.0, 0.0), 2.0, 320);
    assert_eq!(points.len(), 321);
    for p in &points {
        assert!((p.x.hypot(p.y) - 100.0).abs() < 1.0, "{p:?}");
    }
    // It goes round, not back and forth.
    assert!(points.iter().any(|p| p.x < -90.0));
    assert_eq!(field.trace(PointF::new(1.0, 1.0), 0.0, 10).len(), 1);
}

#[test]
fn noise_fields_are_the_noise_exactly() {
    let options = NoiseFieldOptions { turns: 0.5, base_angle: 10.0, ..NoiseFieldOptions::default() };
    let field = DirectionField::from_noise((8, 4), RectangleF::new(0.0, 0.0, 80.0, 40.0), DrawingNoiseKind::Gradient, 42, 0.05, options).unwrap();
    let n = DrawingNoise::fractal(DrawingNoiseKind::Gradient, 25.0 * 0.05, 15.0 * 0.05, 42, DrawingNoiseFractalOptions { octaves: 1, ..DrawingNoiseFractalOptions::default() }).unwrap();
    assert!(same_direction(field.angle_at(25.0, 15.0), 10.0 + (0.5 * 180.0 * n as f32), 1.0e-3));
    assert_eq!(field.strength_at(25.0, 15.0), 1.0);
    let again = DirectionField::from_noise((8, 4), RectangleF::new(0.0, 0.0, 80.0, 40.0), DrawingNoiseKind::Gradient, 42, 0.05, options).unwrap();
    assert_eq!(field, again);
    let evolving = DirectionField::from_noise((8, 4), RectangleF::new(0.0, 0.0, 80.0, 40.0), DrawingNoiseKind::Value, 42, 0.05, NoiseFieldOptions { z: Some(1.5), octaves: 3, ..options }).unwrap();
    assert_ne!(field, evolving);
    assert!(DirectionField::from_noise((8, 4), square(10.0), DrawingNoiseKind::Value, 1, 0.0, options).is_err());
    assert!(DirectionField::from_noise((8, 4), square(10.0), DrawingNoiseKind::Value, 1, 0.1, NoiseFieldOptions { octaves: 0, ..options }).is_err());
}

#[test]
fn rotation_determinism_and_errors() {
    let field = DirectionField::from_guides((4, 4), square(40.0), &[fixed(20.0)]).unwrap();
    assert!((field.rotated(30.0).angle_at(10.0, 10.0) - 50.0).abs() < 1.0e-3);
    assert!(same_direction(field.rotated(100.0).angle_at(10.0, 10.0), 120.0, 1.0e-3));
    let p = picture(30, 20, |x, y| (x * 7 + y * 3) % 11 < 5);
    let a = DirectionField::from_picture(&p, RectangleF::new(5.0, 5.0, 60.0, 40.0), PictureFieldOptions::default()).unwrap();
    let b = DirectionField::from_picture(&p, RectangleF::new(5.0, 5.0, 60.0, 40.0), PictureFieldOptions::default()).unwrap();
    assert_eq!(a, b, "the same inputs give the same bits");
    assert!(DirectionField::from_guides((0, 4), square(4.0), &[]).is_err());
    assert!(DirectionField::from_guides((4, 4), RectangleF::new(0.0, 0.0, 0.0, 4.0), &[]).is_err());
    let bad = DirectionField::from_guides((4, 4), square(4.0), &[fixed(0.0), DirectionGuide::Radial { center: PointF::new(0.0, 0.0), reach: 0.0, weight: 1.0 }]).unwrap_err();
    assert!(bad.contains("guide 1") && bad.contains("reach"), "{bad}");
    let empty = DirectionField::from_guides((4, 4), square(4.0), &[DirectionGuide::Along { path: PathGeometry::empty(), reach: 3.0, weight: 1.0 }]).unwrap_err();
    assert!(empty.contains("no length"), "{empty}");
    assert!(DirectionField::from_picture(&p, square(10.0), PictureFieldOptions { smoothing: -1.0, ..PictureFieldOptions::default() }).is_err());
    // No guide: direction 0, strength 0.
    let nothing = DirectionField::from_guides((2, 2), square(4.0), &[]).unwrap();
    assert_eq!((nothing.angle_at(1.0, 1.0), nothing.strength_at(1.0, 1.0)), (0.0, 0.0));
    // Outside its rectangle a field holds its edge values.
    assert_eq!(field.angle_at(-100.0, 500.0), field.angle_at(0.0, 40.0));
}

#[test]
fn a_full_detail_percentile_brings_mid_strength_edges_up_and_one_is_the_old_result() {
    //Arrange: a strong edge (black to white at x = 10) and a weak one (white to 80% at x = 30).
    let mut p = DrawingPlaneImage::new(40, 20).unwrap();
    p.fill_with(|x, _| match x {
        0..=9 => [0.0, 0.0, 0.0, 1.0],
        10..=29 => [1.0, 1.0, 1.0, 1.0],
        _ => [0.8, 0.8, 0.8, 1.0],
    });
    //Act
    let old = DetailMap::from_picture_edges(&p, 1.0).unwrap();
    let one = DetailMap::from_picture_edges_with(&p, &EdgeDetailOptions { smoothing: 1.0, full_detail_percentile: 1.0 }).unwrap();
    let default = DetailMap::from_picture_edges_with(&p, &EdgeDetailOptions::default()).unwrap();
    let low = DetailMap::from_picture_edges_with(&p, &EdgeDetailOptions { smoothing: 1.0, full_detail_percentile: 0.5 }).unwrap();
    //Assert
    assert_eq!(one.data(), old.data(), "percentile 1 is exactly from_picture_edges");
    assert_eq!(default.data(), DetailMap::from_picture_edges(&p, 0.0).unwrap().data());
    let (weak_old, weak_low) = (old.get(29, 10).unwrap(), low.get(29, 10).unwrap());
    assert!(weak_old < 0.3 && weak_low > weak_old * 1.5, "{weak_old} -> {weak_low}");
    assert_eq!(low.get(9, 10).unwrap(), 1.0, "the strong edge is clamped to full detail");
    assert!(low.data().iter().all(|v| (0.0..=1.0).contains(v)));
    for bad in [0.0, -0.5, 1.5, f32::NAN] {
        assert!(DetailMap::from_picture_edges_with(&p, &EdgeDetailOptions { smoothing: 0.0, full_detail_percentile: bad }).is_err(), "{bad}");
    }
    let flat = DetailMap::from_picture_edges_with(&picture(8, 8, |_, _| false), &EdgeDetailOptions { smoothing: 0.0, full_detail_percentile: 0.5 }).unwrap();
    assert!(flat.data().iter().all(|v| *v == 0.0), "a flat picture gives 0 everywhere");
}

#[test]
fn detail_maps_from_edges_and_sampling() {
    let p = picture(40, 20, |x, _| x >= 20);
    let detail = DetailMap::from_picture_edges(&p, 1.0).unwrap();
    assert_eq!(detail.size(), (40, 20));
    let max = detail.data().iter().fold(0.0f32, |m, v| m.max(*v));
    assert_eq!(max, 1.0);
    assert!(detail.get(19, 10).unwrap() > 0.5 && detail.get(2, 10).unwrap() < 1.0e-3 && detail.get(37, 10).unwrap() < 1.0e-3);
    let flat = DetailMap::from_picture_edges(&picture(8, 8, |_, _| false), 2.0).unwrap();
    assert!(flat.data().iter().all(|v| *v == 0.0));
    assert!(DetailMap::from_picture_edges(&p, f32::NAN).is_err());
    // Sampling: cell centres give the cells, between them a mix, 0 outside.
    let plane = DrawingPlane::from_data(2, 1, vec![0.0, 1.0]).unwrap();
    let area = RectangleF::new(10.0, 10.0, 20.0, 10.0);
    assert_eq!(DetailMap::sample(&plane, area, 15.0, 15.0), 0.0);
    assert_eq!(DetailMap::sample(&plane, area, 25.0, 15.0), 1.0);
    assert_eq!(DetailMap::sample(&plane, area, 20.0, 12.0), 0.5);
    assert_eq!(DetailMap::sample(&plane, area, 11.0, 19.0), 0.0);
    assert_eq!(DetailMap::sample(&plane, area, 29.0, 19.0), 1.0);
    assert_eq!(DetailMap::sample(&plane, area, 31.0, 15.0), 0.0);
    assert_eq!(DetailMap::sample(&plane, area, 25.0, 9.0), 0.0);
}

fn layout_options(seed: i64, angle_jitter: f32) -> StrokeLayoutOptions {
    StrokeLayoutOptions {
        sizes: vec![StrokeSize::new(20.0, 6.0, 12.0), StrokeSize { min_detail: 0.5, variation: 0.25, ..StrokeSize::new(8.0, 3.0, 6.0) }],
        angle_jitter,
        seed,
        ..StrokeLayoutOptions::default()
    }
}

#[test]
fn stroke_layout_fills_band_by_band() {
    let area_rect = RectangleF::new(0.0, 0.0, 200.0, 100.0);
    let area = FormationArea::from_geometry(&PathGeometry::rectangle(area_rect), None).unwrap();
    let field = DirectionField::from_guides((4, 4), area_rect, &[fixed(30.0)]).unwrap();
    let detail = DrawingPlane::from_data(2, 1, vec![0.0, 1.0]).unwrap();
    let color_at = |x: f32, y: f32| Color::from_rgb(x as u8, y as u8, 7);
    let bands = StrokeLayout::fill(&area, &field, Some((&detail, area_rect)), &color_at, &layout_options(3, 0.0)).unwrap();
    assert_eq!(bands.len(), 2);
    // Broad strokes everywhere: about one per hexagonal cell.
    let expected = (200.0 * 100.0) / (12.0 * 12.0 * 0.866);
    assert!((bands[0].len() as f32) > expected * 0.8 && (bands[0].len() as f32) < expected * 1.2, "{}", bands[0].len());
    assert!(bands[0].iter().any(|r| r.center.x < 50.0) && bands[0].iter().any(|r| r.center.x > 150.0));
    // Fine strokes only where the detail map asks.
    assert!(!bands[1].is_empty());
    for r in &bands[1] {
        assert!(DetailMap::sample(&detail, area_rect, r.center.x, r.center.y) >= 0.5, "{:?}", r.center);
        assert!(r.length >= 6.0 - 1.0e-4 && r.length <= 10.0 + 1.0e-4 && (r.width / r.length - 3.0 / 8.0).abs() < 1.0e-4);
    }
    for band in &bands {
        for r in band {
            assert!(area.contains(r.center.x, r.center.y));
            assert!((r.angle - 30.0).abs() < 1.0e-3 && r.bend.abs() < 1.0e-5);
            assert_eq!(r.color, color_at(r.center.x, r.center.y));
            assert_eq!(r.start, crate::strokes::BrushStrokeRecord::ALWAYS);
        }
    }
    assert!(bands[0].iter().all(|r| r.length == 20.0 && r.width == 6.0));
    // The same seed gives the same records; another seed others.
    assert_eq!(StrokeLayout::fill(&area, &field, Some((&detail, area_rect)), &color_at, &layout_options(3, 0.0)).unwrap(), bands);
    assert_ne!(StrokeLayout::fill(&area, &field, Some((&detail, area_rect)), &color_at, &layout_options(4, 0.0)).unwrap(), bands);
    let jittered = StrokeLayout::fill(&area, &field, Some((&detail, area_rect)), &color_at, &layout_options(3, 6.0)).unwrap();
    assert!(jittered[0].iter().any(|r| (r.angle - 30.0).abs() > 1.0));
    // Seeds differ per stroke.
    assert!(bands[0].windows(2).any(|w| w[0].seed != w[1].seed));
    // Without a detail map only the bands for "everywhere" fill.
    let plain = StrokeLayout::fill(&area, &field, None, &color_at, &layout_options(3, 0.0)).unwrap();
    assert!(!plain[0].is_empty() && plain[1].is_empty());
    // Errors say which option.
    assert!(StrokeLayout::fill(&area, &field, None, &color_at, &StrokeLayoutOptions::default()).unwrap_err().contains("at least one stroke size"));
    let bad = StrokeLayoutOptions { sizes: vec![StrokeSize::new(10.0, 0.0, 5.0)], ..StrokeLayoutOptions::default() };
    assert!(StrokeLayout::fill(&area, &field, None, &color_at, &bad).unwrap_err().contains("Stroke size 0"));
    let bad = StrokeLayoutOptions { sizes: vec![StrokeSize { variation: 1.0, ..StrokeSize::new(10.0, 2.0, 5.0) }], ..StrokeLayoutOptions::default() };
    assert!(StrokeLayout::fill(&area, &field, None, &color_at, &bad).unwrap_err().contains("variation"));
}

#[test]
fn element_area_is_in_the_elements_own_coordinates() {
    let mut session = DrawingSession::new(DrawingSessionOptions { calibration_size: (200, 100), ..DrawingSessionOptions::default() });
    session.add_layer("Ink", Color::BLACK);
    let id = session.draw_rectangle(10.0, 20.0, 50.0, 30.0, None, None, true, 0.0).unwrap();
    if let Some(e) = session.find_element_mut(id) {
        e.set_transform(crate::models::DrawingTransform::translate(70.0, 5.0));
    }
    let area = element_area(&session, id, 0.0).unwrap();
    let b = area.bounds();
    assert!((b.x - 10.0).abs() < 1.0e-3 && (b.y - 20.0).abs() < 1.0e-3 && (b.width - 50.0).abs() < 1.0e-3 && (b.height - 30.0).abs() < 1.0e-3, "{b:?}");
    assert!(area.contains(30.0, 30.0) && !area.contains(90.0, 30.0));
    let other = session.draw_circle(150.0, 50.0, 10.0, None, None, true).unwrap();
    let circle = element_area(&session, other, 0.0).unwrap();
    assert!(circle.contains(150.0, 50.0) && !circle.contains(150.0, 62.0));
    let mut elsewhere = DrawingSession::new(DrawingSessionOptions::default());
    elsewhere.add_layer("Ink", Color::BLACK);
    let missing = elsewhere.draw_rectangle(0.0, 0.0, 1.0, 1.0, None, None, true, 0.0).unwrap();
    assert!(element_area(&session, missing, 0.0).unwrap_err().contains("has no element"));
}
