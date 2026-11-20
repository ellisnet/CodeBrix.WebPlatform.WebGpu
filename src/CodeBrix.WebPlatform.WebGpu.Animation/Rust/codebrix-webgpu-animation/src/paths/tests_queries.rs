//! Host tests of the path queries and the path continued past its ends, ported from NoSkia's
//! PathGeometryQueryTests with their stated values, plus C#'s flattening worked by hand.

use kurbo::Affine;

use crate::geometry::{PointF, RectangleF};
use crate::paths::{PathFillRule, PathGeometry, PathGeometryBuilder};

fn square() -> PathGeometry {
    PathGeometry::rectangle(RectangleF::new(0.0, 0.0, 100.0, 100.0))
}

fn points(list: &[(f32, f32)], closed: bool) -> PathGeometry {
    let p: Vec<PointF> = list.iter().map(|(x, y)| PointF::new(*x, *y)).collect();
    PathGeometryBuilder::new().add_polygon(&p, closed).build()
}

fn nested_squares(rule: PathFillRule) -> PathGeometry {
    PathGeometryBuilder::new().add_rectangle(RectangleF::new(0.0, 0.0, 100.0, 100.0)).add_rectangle(RectangleF::new(25.0, 25.0, 50.0, 50.0)).build().with_fill_rule(rule)
}

fn p(x: f32, y: f32) -> PointF {
    PointF::new(x, y)
}

#[test]
fn flatten_follows_csharps_wang_subdivisions_and_evaluation() {
    // A quadratic (0,0) (100,0) (100,100): m = |p0 - 2c + p2| = 141.42..., n = ceil(sqrt(0.25 m /
    // 0.25)) = 12 steps; t = 1/2 gives (75, 25).
    let turn = PathGeometryBuilder::new().move_to(0.0, 0.0).quadratic_to(p(100.0, 0.0), p(100.0, 100.0)).build();
    let lines = turn.flatten(None).unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].count(), 13);
    assert!(!lines[0].is_closed());
    assert_eq!(lines[0].get_points()[6], p(75.0, 25.0));
    assert_eq!(lines[0].get_points()[12], p(100.0, 100.0));
    // A closed square keeps its four corners and does not repeat the first.
    let sq = square().flatten(None).unwrap();
    assert_eq!(sq[0].get_points(), &[p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(0.0, 100.0)]);
    assert!(sq[0].is_closed());
    assert!(square().flatten(Some(0.0)).is_err());
}

#[test]
fn contains_answers_inside_and_outside_a_square() {
    assert!(square().contains(p(50.0, 50.0), None).unwrap());
    assert!(square().contains(p(1.0, 99.0), None).unwrap());
    assert!(!square().contains(p(-1.0, 50.0), None).unwrap());
    assert!(!square().contains(p(50.0, 101.0), None).unwrap());
    assert!(!PathGeometry::empty().contains(p(0.0, 0.0), None).unwrap());
}

#[test]
fn contains_follows_the_geometry_fill_rule() {
    let frame = nested_squares(PathFillRule::EvenOdd);
    let solid = nested_squares(PathFillRule::NonZero);
    assert!(!frame.contains(p(50.0, 50.0), None).unwrap());
    assert!(frame.contains(p(10.0, 10.0), None).unwrap());
    assert!(solid.contains(p(50.0, 50.0), None).unwrap());
}

#[test]
fn contains_treats_an_open_contour_as_closed_like_a_fill() {
    let open = points(&[(0.0, 0.0), (100.0, 0.0), (0.0, 100.0)], false);
    assert!(open.contains(p(20.0, 20.0), None).unwrap());
    assert!(!open.contains(p(80.0, 80.0), None).unwrap());
}

#[test]
fn distance_to_edge_measures_to_the_nearest_edge_inside_and_out() {
    assert_eq!(square().distance_to_edge(p(50.0, 50.0), None).unwrap(), 50.0);
    assert_eq!(square().distance_to_edge(p(10.0, 40.0), None).unwrap(), 10.0);
    assert_eq!(square().distance_to_edge(p(103.0, 104.0), None).unwrap(), 5.0);
    assert!(square().distance_to_edge(p(100.0, 30.0), None).unwrap().abs() <= 1e-5);
    assert!(PathGeometry::empty().distance_to_edge(p(0.0, 0.0), None).unwrap().is_infinite());
}

#[test]
fn distance_to_edge_of_a_circle_is_within_the_tolerance_of_the_true_distance() {
    let circle = PathGeometry::circle(p(0.0, 0.0), 100.0);
    let coarse = circle.distance_to_edge(p(0.0, 0.0), Some(2.0)).unwrap();
    let fine = circle.distance_to_edge(p(0.0, 0.0), Some(0.05)).unwrap();
    assert!((98.0..=100.0).contains(&coarse), "{coarse}");
    assert!((99.95..=100.0).contains(&fine), "{fine}");
}

#[test]
fn get_signed_distance_is_negative_inside_and_positive_outside() {
    assert_eq!(square().get_signed_distance(p(50.0, 40.0), None).unwrap(), -40.0);
    assert_eq!(square().get_signed_distance(p(50.0, 130.0), None).unwrap(), 30.0);
}

#[test]
fn get_nearest_point_lies_on_the_drawn_path_and_leaves_open_contours_open() {
    let corner = points(&[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0)], false);
    assert_eq!(corner.get_nearest_point(p(40.0, -20.0), None).unwrap(), p(40.0, 0.0));
    let near_missing_line = corner.get_nearest_point(p(30.0, 60.0), None).unwrap();
    assert!((near_missing_line.x - 30.0).abs() <= 1e-4);
    assert_eq!(near_missing_line.y, 0.0);
    assert_eq!(square().get_nearest_point(p(-5.0, 30.0), None).unwrap(), p(0.0, 30.0));
    assert!(PathGeometry::empty().get_nearest_point(p(1.0, 1.0), None).is_err());
}

#[test]
fn distance_between_separate_areas_is_the_gap_between_their_nearest_points() {
    let right = PathGeometry::rectangle(RectangleF::new(130.0, 40.0, 50.0, 50.0));
    let below = PathGeometry::rectangle(RectangleF::new(140.0, 140.0, 10.0, 10.0));
    assert_eq!(square().distance(&right, None).unwrap(), 30.0);
    assert_eq!(right.distance(&square(), None).unwrap(), 30.0);
    assert_eq!(square().distance(&below, None).unwrap(), ((40.0f32 * 40.0) + (40.0 * 40.0)).sqrt());
}

#[test]
fn distance_is_zero_for_overlap_containment_and_crossing_edges() {
    let inside = PathGeometry::rectangle(RectangleF::new(40.0, 40.0, 10.0, 10.0));
    let wide = PathGeometry::rectangle(RectangleF::new(-50.0, 40.0, 200.0, 10.0));
    let tall = PathGeometry::rectangle(RectangleF::new(45.0, -50.0, 10.0, 200.0));
    assert_eq!(square().distance(&inside, None).unwrap(), 0.0);
    assert_eq!(inside.distance(&square(), None).unwrap(), 0.0);
    assert_eq!(wide.distance(&tall, None).unwrap(), 0.0, "a plus sign: no corner inside the other, yet they cross");
    assert!(square().distance(&PathGeometry::empty(), None).unwrap().is_infinite());
}

#[test]
fn distance_with_an_offset_equals_the_distance_to_the_moved_geometry() {
    let other = PathGeometry::rectangle(RectangleF::new(0.0, 0.0, 40.0, 40.0));
    let shifted = square().distance_with_offset(&other, p(125.0, 0.0), None).unwrap();
    let moved = square().distance(&other.transformed(Affine::translate((125.0, 0.0))), None).unwrap();
    assert_eq!(shifted, 25.0);
    assert_eq!(moved, shifted);
}

#[test]
fn queries_reject_a_tolerance_that_is_not_positive() {
    assert!(square().contains(p(1.0, 1.0), Some(0.0)).is_err());
    assert!(square().distance_to_edge(p(1.0, 1.0), Some(f32::NAN)).is_err());
    assert!(square().distance_with_offset(&square(), p(f32::INFINITY, 0.0), None).is_err());
}

#[test]
fn get_point_at_extending_beyond_the_ends_continues_straight_along_the_end_directions() {
    let line = points(&[(0.0, 10.0), (200.0, 10.0)], false);
    let before = line.get_point_at_with(-0.25, true);
    let after = line.get_point_at_with(1.1, true);
    assert_eq!((before.position, before.distance), (p(-50.0, 10.0), -50.0));
    assert!((after.position.x - 220.0).abs() <= 1e-3 && (after.distance - 220.0).abs() <= 1e-3);
    assert_eq!(line.get_point_at_with(1.1, false).position, p(200.0, 10.0));
    assert_eq!(line.get_point_at_distance_with(230.0, true).position, p(230.0, 10.0));
    assert_eq!(line.get_point_at_with(0.5, true), line.get_point_at(0.5));
}

#[test]
fn get_point_at_extending_a_curve_follows_its_last_tangent() {
    let turn = PathGeometryBuilder::new().move_to(0.0, 0.0).quadratic_to(p(100.0, 0.0), p(100.0, 100.0)).build();
    let length = turn.get_length();
    let end = turn.get_point_at(1.0);
    let past = turn.get_point_at_distance_with(length + 30.0, true);
    assert_eq!(past.tangent, end.tangent);
    assert!((past.position.x - (end.position.x + end.tangent.x * 30.0)).abs() <= 1e-3);
    assert!((past.position.y - (end.position.y + end.tangent.y * 30.0)).abs() <= 1e-3);
}

// ---- EXACT AGREEMENT WITH A NoSkia RUN ----
// Printed by NoSkia 1.0.317.314 for the same calls: FNV-1a 64 over the bit patterns, in order.

fn fnv(values: impl IntoIterator<Item = f32>) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for v in values {
        for b in v.to_bits().to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
    }
    h
}

#[test]
fn flattening_and_queries_equal_a_noskia_run_bit_for_bit() {
    let disc = PathGeometry::circle(p(200.0, 200.0), 100.0);
    let b = disc.get_bounds();
    assert_eq!([b.x, b.y, b.width, b.height].map(f32::to_bits), [0x42C80000, 0x42C80000, 0x43480000, 0x43480000], "bounds");
    let flat = disc.flatten(None).unwrap();
    assert_eq!(flat[0].count(), 48);
    assert_eq!(fnv(flat.iter().flat_map(|l| l.get_points().iter().flat_map(|q| [q.x, q.y]).collect::<Vec<_>>())), 0xD8F2C96201A7A2D9, "flatten");
    let mut values = Vec::new();
    for y in (80..=320).step_by(7) {
        for x in (80..=320).step_by(7) {
            let q = p(x as f32 + 0.3, y as f32 + 0.6);
            values.push(disc.get_signed_distance(q, None).unwrap());
            let n = disc.get_nearest_point(q, Some(0.5)).unwrap();
            values.push(n.x);
            values.push(n.y);
        }
    }
    assert_eq!(fnv(values), 0x0F48011D3B41C225, "signed distances and nearest points");
    let rect = PathGeometry::rectangle(RectangleF::new(0.0, 0.0, 40.0, 30.0));
    assert_eq!(disc.distance_with_offset(&rect, p(330.5, 190.25), Some(0.5)).unwrap().to_bits(), 0x41F40000);
}
