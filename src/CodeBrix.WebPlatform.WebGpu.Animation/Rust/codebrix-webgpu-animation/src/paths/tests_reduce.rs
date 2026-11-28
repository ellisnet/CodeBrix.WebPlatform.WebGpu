//! Host tests of point reduction (Not in C#): point counts, the largest distance a dropped point
//! lies from the result (within the tolerance), closed and open contours, curves kept, and
//! degenerate paths.

use kurbo::{PathEl, Point};

use crate::geometry::{PointF, RectangleF};
use crate::paths::{PathFillRule, PathGeometry, PathGeometryBuilder};

/// A closed polygon of `n` points round a wobbly circle (as path combining makes them).
fn wobbly(n: usize, radius: f32) -> PathGeometry {
    let points: Vec<PointF> = (0..n)
        .map(|k| {
            let a = k as f32 / n as f32 * std::f32::consts::TAU;
            let r = radius * (1.0 + 0.05 * (a * 7.0).sin());
            PointF::new(400.0 + r * a.cos(), 300.0 + r * a.sin())
        })
        .collect();
    PathGeometryBuilder::new().add_polygon(&points, true).build()
}

/// Each contour's points (end points of every element) and whether it is closed.
fn contours(g: &PathGeometry) -> Vec<(Vec<Point>, bool)> {
    let mut list: Vec<(Vec<Point>, bool)> = Vec::new();
    for el in g.bez_path().elements() {
        match el {
            PathEl::MoveTo(p) => list.push((vec![*p], false)),
            PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => list.last_mut().unwrap().0.push(*p),
            PathEl::ClosePath => list.last_mut().unwrap().1 = true,
        }
    }
    list
}

fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let t = if ab.hypot2() == 0.0 { 0.0 } else { ((p - a).dot(ab) / ab.hypot2()).clamp(0.0, 1.0) };
    (p - (a + ab * t)).hypot()
}

/// The largest distance from a point of `original` to the reduced polyline of the same contour
/// (a closed contour closed back to its start).
fn deviation(original: &PathGeometry, reduced: &PathGeometry) -> f64 {
    let mut worst: f64 = 0.0;
    for ((points, _), (kept, closed)) in contours(original).iter().zip(contours(reduced).iter()) {
        let mut ring = kept.clone();
        if *closed {
            ring.push(kept[0]);
        }
        for p in points {
            let d = ring.windows(2).map(|w| segment_distance(*p, w[0], w[1])).fold(f64::INFINITY, f64::min);
            worst = worst.max(if ring.len() == 1 { (*p - ring[0]).hypot() } else { d });
        }
    }
    worst
}

#[test]
fn reduce_points_drops_points_within_the_tolerance_and_keeps_the_start_and_fill_rule() {
    //Arrange
    let dense = wobbly(2000, 200.0).with_fill_rule(PathFillRule::EvenOdd);
    //Act
    let fine = dense.reduce_points(0.05);
    let coarse = dense.reduce_points(1.0);
    //Assert
    let count = |g: &PathGeometry| g.bez_path().elements().len();
    assert!(count(&fine) < count(&dense) && count(&coarse) < count(&fine) / 2, "{} {} {}", count(&dense), count(&fine), count(&coarse));
    assert!(deviation(&dense, &fine) <= 0.05 + 1e-9);
    assert!(deviation(&dense, &coarse) <= 1.0 + 1e-9);
    assert_eq!(coarse.bez_path().elements()[0], dense.bez_path().elements()[0], "the start point stays");
    assert!(matches!(coarse.bez_path().elements().last(), Some(PathEl::ClosePath)));
    assert_eq!(coarse.fill_rule(), PathFillRule::EvenOdd);
    assert_eq!(coarse.contour_count(), 1);
}

#[test]
fn reduce_points_on_an_open_run_keeps_both_ends_and_the_corners_that_matter() {
    //Arrange: a straight run of many points, then a corner, then another straight run.
    let mut b = PathGeometryBuilder::new();
    b.move_to(0.0, 0.0);
    for i in 1..100 {
        b.line_to(i as f32, if i % 2 == 0 { 0.01 } else { -0.01 });
    }
    b.line_to(100.0, 0.0);
    for i in 1..=100 {
        b.line_to(100.0, i as f32);
    }
    let path = b.build();
    //Act
    let reduced = path.reduce_points(0.1);
    //Assert
    let points = &contours(&reduced)[0];
    assert!(!points.1);
    assert_eq!(points.0, vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(100.0, 100.0)]);
    assert_eq!(path.reduce_points(0.001).bez_path().elements().len(), 1 + 100 + 1, "a tolerance below the wiggle keeps it (the straight run still goes)");
}

#[test]
fn a_closed_contour_keeps_at_least_a_triangle() {
    //Arrange: a small square, far below the tolerance.
    let square = PathGeometry::rectangle(RectangleF::new(10.0, 10.0, 1.0, 1.0));
    //Act
    let reduced = square.reduce_points(100.0);
    //Assert
    let list = contours(&reduced);
    assert_eq!(list.len(), 1);
    assert!(list[0].1, "still closed");
    assert_eq!(list[0].0.len(), 3, "a triangle: {:?}", list[0].0);
    assert_eq!(list[0].0[0], Point::new(10.0, 10.0), "from the same start");
    assert!(!reduced.reduce_points(100.0).is_empty());
}

#[test]
fn a_closed_contour_written_back_to_its_start_keeps_that_line() {
    //Arrange: a square whose last line returns to the start before it is closed, with points
    //along its edges.
    let mut b = PathGeometryBuilder::new();
    b.move_to(0.0, 0.0).line_to(50.0, 0.0).line_to(100.0, 0.0).line_to(100.0, 100.0).line_to(0.0, 100.0).line_to(0.0, 50.0).line_to(0.0, 0.0).close();
    let path = b.build();
    //Act
    let reduced = path.reduce_points(0.5);
    //Assert
    let els = reduced.bez_path().elements();
    assert_eq!(els, [PathEl::MoveTo(Point::new(0.0, 0.0)), PathEl::LineTo(Point::new(100.0, 0.0)), PathEl::LineTo(Point::new(100.0, 100.0)), PathEl::LineTo(Point::new(0.0, 100.0)), PathEl::LineTo(Point::new(0.0, 0.0)), PathEl::ClosePath]);
}

#[test]
fn curves_and_their_end_points_are_left_alone() {
    //Arrange: lines, a cubic, more lines, closed.
    let mut b = PathGeometryBuilder::new();
    b.move_to(0.0, 0.0).line_to(10.0, 0.001).line_to(20.0, 0.0);
    b.cubic_to(PointF::new(30.0, 10.0), PointF::new(40.0, 10.0), PointF::new(50.0, 0.0));
    b.line_to(60.0, 0.001).line_to(70.0, 0.0).line_to(70.0, -20.0).close();
    let circle = PathGeometry::circle(PointF::new(0.0, 0.0), 30.0);
    let path = b.build();
    //Act
    let reduced = path.reduce_points(0.5);
    //Assert
    assert_eq!(reduced.bez_path().elements(), [PathEl::MoveTo(Point::new(0.0, 0.0)), PathEl::LineTo(Point::new(20.0, 0.0)), PathEl::CurveTo(Point::new(30.0, 10.0), Point::new(40.0, 10.0), Point::new(50.0, 0.0)), PathEl::LineTo(Point::new(70.0, 0.0)), PathEl::LineTo(Point::new(70.0, -20.0)), PathEl::ClosePath]);
    assert_eq!(circle.reduce_points(5.0), circle, "a path of curves only is unchanged");
}

#[test]
fn degenerate_paths_and_tolerances_give_the_path_back() {
    let empty = PathGeometry::empty();
    assert_eq!(empty.reduce_points(1.0), empty);
    let mut b = PathGeometryBuilder::new();
    b.move_to(5.0, 5.0);
    let lone = b.build();
    assert_eq!(lone.reduce_points(1.0), lone, "a lone MoveTo");
    let mut b = PathGeometryBuilder::new();
    b.move_to(0.0, 0.0).line_to(10.0, 0.0);
    let line = b.build();
    assert_eq!(line.reduce_points(100.0), line, "a single line keeps its two ends");
    let mut b = PathGeometryBuilder::new();
    b.move_to(0.0, 0.0).line_to(10.0, 0.0).line_to(10.0, 10.0).close();
    let triangle = b.build();
    assert_eq!(triangle.reduce_points(100.0), triangle, "a closed triangle stays a triangle");
    let dense = wobbly(300, 50.0);
    for bad in [0.0, -1.0, f32::NAN] {
        assert_eq!(dense.reduce_points(bad), dense, "tolerance {bad}");
    }
    let mut b = PathGeometryBuilder::new();
    b.move_to(1.0, 1.0).line_to(1.0, 1.0).line_to(1.0, 1.0).line_to(1.0, 1.0).close();
    let point = b.build();
    assert_eq!(contours(&point.reduce_points(1.0))[0].0.len(), 4, "a contour of one repeated point stays a (degenerate) triangle and its line back to the start");
}

#[test]
fn every_contour_of_a_combined_path_is_reduced_on_its_own() {
    //Arrange: two dense rings far apart and a union result's polygons.
    let mut b = PathGeometryBuilder::new();
    b.add_path(&wobbly(500, 100.0));
    b.add_path(&wobbly(500, 20.0));
    let rings = b.build();
    let union = PathGeometry::circle(PointF::new(0.0, 0.0), 400.0).union(&PathGeometry::circle(PointF::new(320.0, 0.0), 400.0), None).unwrap();
    //Act
    let reduced = rings.reduce_points(0.5);
    let reduced_union = union.reduce_points(0.5);
    //Assert
    assert_eq!(reduced.contour_count(), 2);
    assert!(deviation(&rings, &reduced) <= 0.5 + 1e-9);
    assert!(deviation(&union, &reduced_union) <= 0.5 + 1e-9);
    assert!(reduced_union.bez_path().elements().len() < union.bez_path().elements().len());
}
