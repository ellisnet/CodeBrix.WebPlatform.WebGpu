//! Host tests of the sketch helper (Animation 5, Not in C#): the same seed gives the same line,
//! another seed or pass another one; the line strays at most the roughness; sharp corners stay
//! sharp points; boiling follows the clock time; the settings' checks.

use kurbo::{PathEl, Point, Shape};

use crate::geometry::{PointF, RectangleF};
use crate::paths::{PathGeometry, PathGeometryBuilder, SketchOptions};

fn square() -> PathGeometry {
    PathGeometry::rectangle(RectangleF::new(0.0, 0.0, 200.0, 200.0))
}

/// Points along the path at many places (flattened finely).
fn samples(g: &PathGeometry) -> Vec<Point> {
    let mut out = Vec::new();
    kurbo::flatten(g.bez_path().iter(), 0.01, |el| match el {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => out.push(p),
        _ => {}
    });
    out
}

/// The distance from a point to the square's outline.
fn off_square(p: Point) -> f64 {
    let inside = p.x.clamp(0.0, 200.0) == p.x && p.y.clamp(0.0, 200.0) == p.y;
    let to_edge = [p.x.abs(), (p.x - 200.0).abs(), p.y.abs(), (p.y - 200.0).abs()];
    if inside {
        to_edge.iter().cloned().fold(f64::INFINITY, f64::min)
    } else {
        let dx = (0.0 - p.x).max(p.x - 200.0).max(0.0);
        let dy = (0.0 - p.y).max(p.y - 200.0).max(0.0);
        dx.hypot(dy)
    }
}

#[test]
fn the_same_seed_gives_the_same_line_and_another_seed_another() {
    //Arrange
    let options = SketchOptions { roughness: 4.0, wobble_length: 20.0, passes: 1, ..SketchOptions::default() };
    //Act
    let a = square().sketch(&options).unwrap();
    let b = square().sketch(&options).unwrap();
    let c = square().sketch(&SketchOptions { seed: 2, ..options }).unwrap();
    //Assert
    assert_eq!(a.bez_path(), b.bez_path());
    assert_ne!(a.bez_path(), c.bez_path());
}

#[test]
fn each_pass_is_the_path_again_roughened_differently() {
    //Arrange
    let one = SketchOptions { passes: 1, ..SketchOptions::default() };
    let three = SketchOptions { passes: 3, ..SketchOptions::default() };
    //Act
    let a = square().sketch(&one).unwrap();
    let b = square().sketch(&three).unwrap();
    //Assert
    assert_eq!(a.contour_count(), 1);
    assert_eq!(b.contour_count(), 3);
    let first: Vec<PathEl> = b.bez_path().elements().iter().take(a.bez_path().elements().len()).cloned().collect();
    assert_eq!(first, a.bez_path().elements(), "the first pass is the one-pass sketch");
    assert_ne!(b.bez_path().elements()[..first.len()], b.bez_path().elements()[first.len()..first.len() * 2]);
}

#[test]
fn the_line_strays_from_the_path_but_never_by_much_more_than_the_roughness() {
    //Arrange
    let options = SketchOptions { roughness: 5.0, wobble_length: 25.0, passes: 2, ..SketchOptions::default() };
    //Act
    let sketch = square().sketch(&options).unwrap();
    let distances: Vec<f64> = samples(&sketch).into_iter().map(off_square).collect();
    //Assert: it does stray, and within the roughness (a little more where the smooth curve
    // swings between two samples).
    let max = distances.iter().cloned().fold(0.0, f64::max);
    assert!(max > 1.0, "{max}");
    assert!(max <= 5.0 * 1.3, "{max}");
}

#[test]
fn sharp_corners_stay_sharp_points() {
    //Arrange
    let options = SketchOptions { roughness: 3.0, wobble_length: 40.0, passes: 1, ..SketchOptions::default() };
    //Act
    let sketch = square().sketch(&options).unwrap();
    //Assert: every corner of the square is an end point of the curves, within half the
    // roughness, and the curve turns sharply there (the incoming and outgoing tangents differ).
    let els = sketch.bez_path().elements();
    let ends: Vec<Point> = els.iter().filter_map(|e| match e {
        PathEl::MoveTo(p) | PathEl::CurveTo(_, _, p) => Some(*p),
        _ => None,
    }).collect();
    for corner in [Point::new(0.0, 0.0), Point::new(200.0, 0.0), Point::new(200.0, 200.0), Point::new(0.0, 200.0)] {
        let near = ends.iter().cloned().fold(f64::INFINITY, |m, p| m.min((p - corner).hypot()));
        assert!(near <= 3.0 * 0.5 * std::f64::consts::SQRT_2 + 1e-9, "corner {corner:?} at {near}");
    }
    // At the corner (200, 0) - the end of a curve - the curve arrives along +x and leaves along +y.
    let i = els.iter().position(|e| matches!(e, PathEl::CurveTo(_, _, p) if (*p - Point::new(200.0, 0.0)).hypot() < 3.0)).unwrap();
    let (PathEl::CurveTo(_, c2, p), PathEl::CurveTo(c1, _, _)) = (els[i], els[i + 1]) else { panic!() };
    let (inc, out) = (p - c2, c1 - p);
    let angle = inc.cross(out).atan2(inc.dot(out)).abs().to_degrees();
    assert!(angle > 60.0, "{angle}");
}

#[test]
fn a_smooth_closed_contour_wraps_round_without_a_corner() {
    //Arrange
    let circle = PathGeometry::circle(PointF::new(100.0, 100.0), 80.0);
    //Act
    let sketch = circle.sketch(&SketchOptions { passes: 1, ..SketchOptions::default() }).unwrap();
    //Assert: closed, and the bounds near the circle's.
    let els = sketch.bez_path().elements();
    assert!(matches!(els.last(), Some(PathEl::ClosePath)));
    let b = sketch.bez_path().bounding_box();
    assert!((b.width() - 160.0).abs() < 10.0 && (b.height() - 160.0).abs() < 10.0, "{b:?}");
}

#[test]
fn an_open_path_keeps_its_ends_as_anchors() {
    //Arrange
    let mut b = PathGeometryBuilder::new();
    b.move_to(0.0, 0.0).line_to(300.0, 0.0);
    let line = b.build();
    //Act
    let sketch = line.sketch(&SketchOptions { roughness: 4.0, wobble_length: 30.0, passes: 1, ..SketchOptions::default() }).unwrap();
    let els = sketch.bez_path().elements();
    //Assert: about ten wobbles (300 / 30), open, starting and ending near the ends.
    assert_eq!(els.iter().filter(|e| matches!(e, PathEl::CurveTo(..))).count(), 10);
    assert!(!els.iter().any(|e| matches!(e, PathEl::ClosePath)));
    let (PathEl::MoveTo(start), PathEl::CurveTo(_, _, end)) = (els[0], els[els.len() - 1]) else { panic!() };
    assert!(start.to_vec2().hypot() <= 3.0 && (end - Point::new(300.0, 0.0)).hypot() <= 3.0);
}

#[test]
fn boiling_changes_the_line_rate_times_a_second_of_clock_time_and_a_seek_shows_the_same_line() {
    //Arrange
    let options = SketchOptions::default();
    //Act
    let at = |t: f64| square().sketch(&options.boiled(t, 10.0)).unwrap();
    //Assert: within one tenth of a second the same line; the next tenth another; back again the same.
    assert_eq!(at(1.00).bez_path(), at(1.09).bez_path());
    assert_ne!(at(1.09).bez_path(), at(1.10).bez_path());
    assert_eq!(at(1.05).bez_path(), at(1.05).bez_path());
    assert_eq!(options.boiled(1.25, 10.0).seed, options.seed + 12);
    assert_eq!(options.boiled(5.0, 0.0), options, "rate 0: no boiling");
    assert_eq!(options.boiled(f64::NAN, 10.0), options);
}

#[test]
fn roughness_zero_keeps_the_line_in_place() {
    //Act
    let sketch = square().sketch(&SketchOptions { roughness: 0.0, passes: 1, ..SketchOptions::default() }).unwrap();
    //Assert
    let max = samples(&sketch).into_iter().map(off_square).fold(0.0, f64::max);
    assert!(max < 1e-6, "{max}");
}

#[test]
fn the_settings_are_checked() {
    //Assert
    assert!(square().sketch(&SketchOptions { roughness: -1.0, ..SketchOptions::default() }).is_err());
    assert!(square().sketch(&SketchOptions { wobble_length: 0.0, ..SketchOptions::default() }).is_err());
    assert!(square().sketch(&SketchOptions { passes: 0, ..SketchOptions::default() }).is_err());
    assert!(square().sketch(&SketchOptions { corner_angle_degrees: f32::NAN, ..SketchOptions::default() }).is_err());
    assert!(PathGeometry::empty().sketch(&SketchOptions::default()).unwrap().is_empty());
}
