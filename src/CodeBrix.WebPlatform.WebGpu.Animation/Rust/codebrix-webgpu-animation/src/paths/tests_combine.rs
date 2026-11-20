//! Host tests of path combining: areas against values worked by hand from NoSkia's rules (the
//! 1/1024 grid, the non-zero result, NoSkia's stroker pieces), and inside tests at chosen points.

use kurbo::PathEl;

use crate::geometry::{PointF, RectangleF};
use crate::paths::{PathBooleanOperation, PathFillRule, PathGeometry, PathGeometryBuilder, PathInflateOptions, PathStrokeCap, PathStrokeJoin, PathStrokeStyle, PathStrokeStyleOptions};

fn rect(x: f32, y: f32, w: f32, h: f32) -> PathGeometry {
    PathGeometry::rectangle(RectangleF::new(x, y, w, h))
}

/// The area a result of polygons fills: the absolute sum of its contours' signed areas works for
/// non-overlapping outer contours; holes are wound against them, so the signed sum is the area.
fn area(g: &PathGeometry) -> f64 {
    let mut total = 0.0;
    let mut contour: Vec<(f64, f64)> = Vec::new();
    let flush = |c: &mut Vec<(f64, f64)>, total: &mut f64| {
        let n = c.len();
        let mut s = 0.0;
        for i in 0..n {
            let (a, b) = (c[i], c[(i + 1) % n]);
            s += a.0 * b.1 - b.0 * a.1;
        }
        *total += s / 2.0;
        c.clear();
    };
    for el in g.bez_path().elements() {
        match el {
            PathEl::MoveTo(p) => {
                flush(&mut contour, &mut total);
                contour.push((p.x, p.y));
            }
            PathEl::LineTo(p) => contour.push((p.x, p.y)),
            PathEl::ClosePath => flush(&mut contour, &mut total),
            _ => panic!("a combined path has lines only"),
        }
    }
    flush(&mut contour, &mut total);
    total.abs()
}

fn inside(g: &PathGeometry, x: f32, y: f32) -> bool {
    g.contains(PointF::new(x, y), None).unwrap()
}

fn on_grid(g: &PathGeometry) -> bool {
    g.bez_path().elements().iter().all(|el| match el {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => (p.x * 1024.0).fract() == 0.0 && (p.y * 1024.0).fract() == 0.0,
        _ => true,
    })
}

#[test]
fn union_intersect_subtract_and_xor_of_two_overlapping_squares() {
    //Arrange
    let a = rect(0.0, 0.0, 100.0, 100.0);
    let b = rect(50.0, 50.0, 100.0, 100.0);

    //Act
    let union = a.union(&b, None).unwrap();
    let both = a.intersect(&b, None).unwrap();
    let cut = a.subtract(&b, None).unwrap();
    let odd = a.exclusive_or(&b, None).unwrap();

    //Assert
    assert_eq!(area(&union), 17_500.0);
    assert_eq!(area(&both), 2_500.0);
    assert_eq!(area(&cut), 7_500.0);
    assert_eq!(area(&odd), 15_000.0);
    assert!(inside(&union, 140.0, 140.0) && inside(&union, 10.0, 10.0));
    assert!(inside(&both, 75.0, 75.0) && !inside(&both, 25.0, 25.0));
    assert!(inside(&cut, 25.0, 25.0) && !inside(&cut, 75.0, 75.0));
    assert!(!inside(&odd, 75.0, 75.0) && inside(&odd, 125.0, 125.0));
    for g in [&union, &both, &cut, &odd] {
        assert_eq!(g.fill_rule(), PathFillRule::NonZero);
        assert!(on_grid(g));
    }
}

#[test]
fn combine_with_matches_the_named_operations_and_disjoint_intersection_is_empty() {
    let a = rect(0.0, 0.0, 10.0, 10.0);
    let b = rect(20.0, 0.0, 10.0, 10.0);
    assert!(a.intersect(&b, None).unwrap().is_empty());
    assert_eq!(a.combine_with(&b, PathBooleanOperation::Union, None).unwrap(), a.union(&b, None).unwrap());
    assert_eq!(area(&a.union(&b, None).unwrap()), 200.0);
}

#[test]
fn simplify_follows_the_fill_rule_and_removes_overlaps() {
    //Arrange: a square with a square inside it, by both rules
    let nested = PathGeometryBuilder::new().add_rectangle(RectangleF::new(0.0, 0.0, 100.0, 100.0)).add_rectangle(RectangleF::new(25.0, 25.0, 50.0, 50.0)).build();

    //Act
    let even_odd = nested.with_fill_rule(PathFillRule::EvenOdd).simplify(None).unwrap();
    let non_zero = nested.with_fill_rule(PathFillRule::NonZero).simplify(None).unwrap();
    let overlapping = PathGeometryBuilder::new().add_rectangle(RectangleF::new(0.0, 0.0, 60.0, 10.0)).add_rectangle(RectangleF::new(40.0, 0.0, 60.0, 10.0)).build().simplify(None).unwrap();

    //Assert: even-odd leaves a hole, non-zero (same winding) fills it; overlaps become one contour
    assert_eq!(area(&even_odd), 7_500.0);
    assert!(!inside(&even_odd, 50.0, 50.0));
    assert_eq!(area(&non_zero), 10_000.0);
    assert_eq!(overlapping.contour_count(), 1);
    assert_eq!(area(&overlapping), 1_000.0);
}

#[test]
fn differing_fill_rules_combine_each_by_its_own() {
    let ring = PathGeometryBuilder::new().add_rectangle(RectangleF::new(0.0, 0.0, 100.0, 100.0)).add_rectangle(RectangleF::new(25.0, 25.0, 50.0, 50.0)).build().with_fill_rule(PathFillRule::EvenOdd);
    let bar = rect(40.0, -10.0, 20.0, 120.0);
    let union = ring.union(&bar, None).unwrap();
    // ring 7,500 + the bar's 2,400 - their overlap (2 x 20 x 25 = 1,000) = 8,900.
    assert_eq!(area(&union), 8_900.0);
    assert!(inside(&union, 50.0, 50.0));
    assert!(!inside(&union, 30.0, 50.0));
}

#[test]
fn combine_all_unions_intersects_subtracts_and_xors_many() {
    let list = [rect(0.0, 0.0, 30.0, 10.0), rect(10.0, 0.0, 30.0, 10.0), rect(20.0, 0.0, 30.0, 10.0)];
    assert_eq!(area(&PathGeometry::combine_all(&list, PathBooleanOperation::Union, None).unwrap()), 500.0);
    assert_eq!(area(&PathGeometry::combine_all(&list, PathBooleanOperation::Intersection, None).unwrap()), 100.0);
    assert_eq!(area(&PathGeometry::combine_all(&list, PathBooleanOperation::Difference, None).unwrap()), 100.0);
    // Odd coverage: x 0-10 (1), 10-20 (2), 20-30 (3), 30-40 (2), 40-50 (1) -> 30 units wide.
    assert_eq!(area(&PathGeometry::combine_all(&list, PathBooleanOperation::ExclusiveOr, None).unwrap()), 300.0);
    assert!(PathGeometry::combine_all(&[], PathBooleanOperation::Union, None).unwrap().is_empty());
    assert_eq!(area(&PathGeometry::combine_all(&list[..1], PathBooleanOperation::Intersection, None).unwrap()), 300.0);
}

#[test]
fn stroke_outline_of_a_line_has_the_caps_area() {
    //Arrange
    let line = PathGeometryBuilder::new().move_to(0.0, 0.0).line_to(100.0, 0.0).build();
    let style = |cap| PathStrokeStyle::new(10.0, PathStrokeStyleOptions { cap, join: PathStrokeJoin::Miter, ..PathStrokeStyleOptions::default() }).unwrap();

    //Act
    let butt = line.to_stroke_outline(&style(PathStrokeCap::Butt), None).unwrap();
    let square = line.to_stroke_outline(&style(PathStrokeCap::Square), None).unwrap();
    let round = line.to_stroke_outline(&style(PathStrokeCap::Round), None).unwrap();

    //Assert: 100 x 10; plus 2 x 5 x 10; plus a disc of radius 5 as NoSkia's chords: the arc step
    //  is 2 pi / ceil(2 pi / (2 acos(1 - 0.25 / 5))) = 2 pi / 10, so the two caps make a 10-gon
    assert_eq!(area(&butt), 1_000.0);
    assert_eq!(area(&square), 1_100.0);
    let ten_gon = 5.0 * 25.0 * (std::f64::consts::PI / 5.0).sin();
    assert!((area(&round) - (1_000.0 + ten_gon)).abs() < 0.01, "{}", area(&round));
    assert!(on_grid(&round));
}

#[test]
fn stroke_outline_of_a_closed_square_with_miter_joins_is_a_frame() {
    let frame = rect(0.0, 0.0, 100.0, 100.0).to_stroke_outline(&PathStrokeStyle::new(10.0, PathStrokeStyleOptions { join: PathStrokeJoin::Miter, ..PathStrokeStyleOptions::default() }).unwrap(), None).unwrap();
    // 110 x 110 less 90 x 90.
    assert_eq!(area(&frame), 4_000.0);
    assert!(inside(&frame, -4.0, -4.0));
    assert!(!inside(&frame, 50.0, 50.0));
    let bevel = rect(0.0, 0.0, 100.0, 100.0).to_stroke_outline(&PathStrokeStyle::new(10.0, PathStrokeStyleOptions { join: PathStrokeJoin::Bevel, ..PathStrokeStyleOptions::default() }).unwrap(), None).unwrap();
    // Each corner loses a right triangle of legs 5: 4 x 12.5.
    assert_eq!(area(&bevel), 3_950.0);
}

#[test]
fn dashed_stroke_outline_cuts_the_dashes_first_and_a_dot_is_a_square() {
    let line = PathGeometryBuilder::new().move_to(0.0, 0.0).line_to(100.0, 0.0).build();
    let dashed = PathStrokeStyle::new(4.0, PathStrokeStyleOptions { cap: PathStrokeCap::Butt, dash_pattern: Some(vec![10.0, 10.0]), ..PathStrokeStyleOptions::default() }).unwrap();
    let outline = line.to_stroke_outline(&dashed, None).unwrap();
    assert_eq!(outline.contour_count(), 5);
    assert_eq!(area(&outline), 200.0);
    let dot = PathGeometryBuilder::new().move_to(5.0, 5.0).line_to(5.0, 5.0).build();
    let square = PathStrokeStyle::new(4.0, PathStrokeStyleOptions { cap: PathStrokeCap::Square, ..PathStrokeStyleOptions::default() }).unwrap();
    assert_eq!(area(&dot.to_stroke_outline(&square, None).unwrap()), 16.0);
}

#[test]
fn inflate_and_deflate_a_square() {
    let square = rect(0.0, 0.0, 100.0, 100.0);
    let miter = PathInflateOptions { join: PathStrokeJoin::Miter, ..PathInflateOptions::default() };
    assert_eq!(area(&square.inflate(10.0, miter).unwrap()), 14_400.0);
    assert_eq!(area(&square.deflate(10.0, miter).unwrap()), 6_400.0);
    assert_eq!(area(&square.inflate(0.0, miter).unwrap()), 10_000.0);
    // Round corners as NoSkia's fans: the arc step is 2 pi / 15 (radius 10, tolerance 0.25), so
    //  each quarter turn takes ceil((pi / 2) / (2 pi / 15)) = 4 chords of pi / 8.
    let round = square.inflate(10.0, PathInflateOptions::default()).unwrap();
    let fans = 4.0 * 4.0 * 0.5 * 100.0 * (std::f64::consts::PI / 8.0).sin();
    assert!((area(&round) - (14_000.0 + fans)).abs() < 0.05, "{}", area(&round)); // grid rounding of the fan points
    assert!(square.deflate(60.0, miter).unwrap().is_empty());
}

#[test]
fn errors_say_what_to_do() {
    let far = rect(2_000_000.0, 0.0, 10.0, 10.0);
    let error = far.union(&rect(0.0, 0.0, 1.0, 1.0), None).unwrap_err();
    assert!(error.contains("million units"), "{error}");
    assert!(rect(0.0, 0.0, 1.0, 1.0).simplify(Some(0.0)).unwrap_err().contains("tolerance"));
    assert!(rect(0.0, 0.0, 1.0, 1.0).inflate(f32::NAN, PathInflateOptions::default()).unwrap_err().contains("finite"));
    assert!(rect(0.0, 0.0, 1.0, 1.0).inflate(1.0, PathInflateOptions { miter_limit: 0.5, ..PathInflateOptions::default() }).unwrap_err().contains("miter limit"));
}

#[test]
fn a_circle_union_keeps_its_chords_on_the_grid_within_the_tolerance() {
    let circle = PathGeometry::circle(PointF::new(0.0, 0.0), 50.0);
    let merged = circle.union(&rect(-10.0, -100.0, 20.0, 200.0), Some(0.25)).unwrap();
    assert!(on_grid(&merged));
    // The circle's chords lie within 0.25 of it: the area is a little under pi r^2 plus the bar's
    // parts outside it.
    let a = area(&merged);
    let bar_out = 20.0 * 200.0 - 2.0 * 50.0 * 20.0 * 0.98;
    assert!(a > 7_853.0 - 40.0 + bar_out - 100.0 && a < 7_854.0 + 4_000.0, "{a}");
}
