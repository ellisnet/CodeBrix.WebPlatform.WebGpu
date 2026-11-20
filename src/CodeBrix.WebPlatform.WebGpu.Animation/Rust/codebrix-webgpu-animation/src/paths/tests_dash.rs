//! Host tests of dashing as geometry and of polygons with rounded corners, ported from NoSkia's
//! PathGeometryTests with their stated values (Dash_splits_..., the shape constructors and their
//! refusals), plus the rounded corner's construction worked by hand.

use kurbo::PathEl;

use crate::geometry::{PointF, RectangleF};
use crate::paths::{PathGeometry, PathGeometryBuilder};

fn line() -> PathGeometry {
    PathGeometryBuilder::new().move_to(0.0, 0.0).line_to(100.0, 0.0).build()
}

/// Each contour's start point, end point and length.
fn contours(geometry: &PathGeometry) -> Vec<(PointF, PointF, f32)> {
    let mut out = Vec::new();
    let mut builder: Option<PathGeometryBuilder> = None;
    let mut ends = (PointF::new(0.0, 0.0), PointF::new(0.0, 0.0));
    let finish = |b: Option<PathGeometryBuilder>, ends: (PointF, PointF), out: &mut Vec<(PointF, PointF, f32)>| {
        if let Some(b) = b {
            out.push((ends.0, ends.1, b.build().get_length()));
        }
    };
    for el in geometry.bez_path().elements() {
        match el {
            PathEl::MoveTo(p) => {
                finish(builder.take(), ends, &mut out);
                let mut b = PathGeometryBuilder::new();
                b.move_to(p.x as f32, p.y as f32);
                builder = Some(b);
                ends = (PointF::new(p.x as f32, p.y as f32), PointF::new(p.x as f32, p.y as f32));
            }
            PathEl::LineTo(p) => {
                builder.as_mut().unwrap().line_to(p.x as f32, p.y as f32);
                ends.1 = PointF::new(p.x as f32, p.y as f32);
            }
            _ => panic!("a dashed line holds only lines"),
        }
    }
    finish(builder.take(), ends, &mut out);
    out
}

#[test]
fn dash_splits_a_line_into_dashes_with_an_offset_dots_and_an_odd_pattern() {
    //Arrange
    let line = line();

    //Act
    let dashes = contours(&line.dash(&[20.0, 10.0], 0.0).unwrap());
    let offset = contours(&line.dash(&[20.0, 10.0], 25.0).unwrap());
    let dots = contours(&line.dash(&[0.0, 25.0], 0.0).unwrap());
    let odd = contours(&line.dash(&[10.0], 0.0).unwrap());

    //Assert
    assert_eq!(dashes.len(), 4);
    assert_eq!(dashes[1].0, PointF::new(30.0, 0.0));
    assert_eq!(dashes[3].1, PointF::new(100.0, 0.0));
    assert_eq!(dashes[3].2, 10.0);
    assert_eq!(offset[0].0, PointF::new(5.0, 0.0));
    assert_eq!(dots.len(), 4);
    assert_eq!(dots[2].0, PointF::new(50.0, 0.0));
    assert_eq!(dots[2].2, 0.0);
    assert_eq!(odd.len(), 5);
}

#[test]
fn dash_keeps_the_fill_rule_and_cuts_every_contour_from_its_own_start() {
    //Arrange
    let two = PathGeometryBuilder::new().move_to(0.0, 0.0).line_to(50.0, 0.0).move_to(0.0, 10.0).line_to(50.0, 10.0).build().with_fill_rule(crate::paths::PathFillRule::EvenOdd);

    //Act
    let dashed = two.dash(&[20.0, 10.0], 0.0).unwrap();
    let parts = contours(&dashed);

    //Assert
    assert_eq!(dashed.fill_rule(), crate::paths::PathFillRule::EvenOdd);
    // 0-20, 30-50 on each line.
    assert_eq!(parts.len(), 4);
    assert_eq!(parts[2].0, PointF::new(0.0, 10.0));
    assert_eq!(parts[3].1, PointF::new(50.0, 10.0));
}

#[test]
fn dash_rejects_patterns_without_length_or_with_negative_entries_and_an_offset_that_is_not_finite() {
    //Arrange
    let square = PathGeometry::rectangle(RectangleF::new(0.0, 0.0, 100.0, 100.0));

    //Act
    let results = [square.dash(&[], 0.0), square.dash(&[0.0, 0.0], 0.0), square.dash(&[5.0, -1.0], 0.0), square.dash(&[f32::NAN], 0.0), square.dash(&[5.0, 5.0], f32::INFINITY)];

    //Assert
    for r in results {
        assert!(r.is_err());
    }
}

#[test]
fn rounded_polygon_is_closed_and_as_long_as_the_rounded_rectangle_it_matches() {
    //Arrange
    let corners = [PointF::new(0.0, 0.0), PointF::new(100.0, 0.0), PointF::new(100.0, 100.0), PointF::new(0.0, 100.0)];

    //Act
    let rounded = PathGeometry::rounded_polygon(&corners, 10.0).unwrap();
    let rounded_rectangle = PathGeometry::rounded_rectangle(RectangleF::new(0.0, 0.0, 100.0, 100.0), 10.0);

    //Assert
    assert!(matches!(rounded.bez_path().elements().last(), Some(PathEl::ClosePath)));
    assert!((rounded.get_bounds().width - 100.0).abs() < 1e-3);
    assert!((rounded.get_length() - rounded_rectangle.get_length()).abs() < 0.05);
}

#[test]
fn rounded_polygon_places_each_corner_s_arc_as_csharp_constructs_it() {
    //Arrange
    let corners = [PointF::new(0.0, 0.0), PointF::new(100.0, 0.0), PointF::new(100.0, 100.0), PointF::new(0.0, 100.0)];
    // A right angle: the tangent length is radius / tan(45 degrees) = 10; the arc's handle is
    // 4/3 tan(90 degrees / 4) x 10 = 5.5228475.
    let handle = (4.0 / 3.0 * (std::f64::consts::FRAC_PI_2 / 4.0).tan() * 10.0) as f32;

    //Act
    let rounded = PathGeometry::rounded_polygon(&corners, 10.0).unwrap();
    let elements: Vec<PathEl> = rounded.bez_path().elements().to_vec();

    //Assert
    // Starts where corner 0's arc ends (10, 0); then per corner a line to its arc's start and the arc.
    assert!(matches!(elements[0], PathEl::MoveTo(p) if p.x as f32 == 10.0 && p.y as f32 == 0.0));
    assert!(matches!(elements[1], PathEl::LineTo(p) if p.x as f32 == 90.0 && p.y as f32 == 0.0));
    match elements[2] {
        PathEl::CurveTo(c1, c2, end) => {
            assert!((c1.x as f32 - (90.0 + handle)).abs() < 1e-4 && c1.y == 0.0);
            assert!(c2.x as f32 == 100.0 && (c2.y as f32 - (10.0 - handle)).abs() < 1e-4);
            assert!(end.x as f32 == 100.0 && end.y as f32 == 10.0);
        }
        other => panic!("expected the corner's arc, got {other:?}"),
    }
    assert_eq!(elements.iter().filter(|e| matches!(e, PathEl::CurveTo(..))).count(), 4);
}

#[test]
fn rounded_polygon_keeps_a_straight_run_sharp_shrinks_a_radius_too_large_and_refuses_bad_input() {
    //Arrange
    let with_straight_run = [PointF::new(0.0, 0.0), PointF::new(50.0, 0.0), PointF::new(100.0, 0.0), PointF::new(100.0, 100.0), PointF::new(0.0, 100.0)];
    let small = [PointF::new(0.0, 0.0), PointF::new(10.0, 0.0), PointF::new(10.0, 10.0), PointF::new(0.0, 10.0)];

    //Act
    let run = PathGeometry::rounded_polygon(&with_straight_run, 10.0).unwrap();
    let shrunk = PathGeometry::rounded_polygon(&small, 100.0).unwrap();
    let plain = PathGeometry::rounded_polygon(&small, 0.0).unwrap();

    //Assert
    assert_eq!(run.bez_path().elements().iter().filter(|e| matches!(e, PathEl::CurveTo(..))).count(), 4);
    // Each rounding takes at most half of its edges: a 10 x 10 square becomes a circle-like shape
    // of radius 5 whose length is that of a circle (2 pi 5) within the cubic's error.
    assert!((shrunk.get_length() - 2.0 * std::f32::consts::PI * 5.0).abs() < 0.05);
    assert_eq!(plain.bez_path().elements().iter().filter(|e| matches!(e, PathEl::CurveTo(..))).count(), 0);
    assert!(PathGeometry::rounded_polygon(&small[..2], 2.0).is_err());
    assert!(PathGeometry::rounded_polygon(&small, -1.0).is_err());
    assert!(PathGeometry::rounded_polygon(&small, f32::NAN).is_err());
}
