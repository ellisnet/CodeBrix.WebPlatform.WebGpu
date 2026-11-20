//! Tests of the painterly playback: NoSkia's own tests with their stated values
//! (BrushStrokeShapeTests, DrawingOwnershipMapTests, DrawingSessionHardEdgedMaskTests,
//! KeyframePlayerTests), what has no C# counterpart (painting a stroke on; the one-thread
//! players), and the stroke-area pieces.

use std::cell::RefCell;
use std::rc::Rc;

use crate::color::Color;
use crate::geometry::{PointF, Rectangle, RectangleF};
use crate::mask_meaning::MaskMeaning;
use crate::models::{DrawingTransform, ElementAxes};
use crate::painting::{DrawingOwnershipMap, KeyframePlayer, KeyframePlayerOptions, PlaneKeyframePlayer};
use crate::paths::{PathFillRule, PathGeometryBuilder};
use crate::planes::{DrawingPlane, DrawingPlaneImage, DrawingTransition, DrawingTransitionDirection};
use crate::shapes::{BrushStrokeShape, BrushStrokeShapeOptions, DrawingGroup, DrawingShape, PathShape, RectangleShape, RectangleShapeOptions, ShapeKind, ShapeOptions};
use crate::{DrawingSession, DrawingSessionOptions};

const N: i32 = DrawingOwnershipMap::NOBODY;
const PPS: usize = BrushStrokeShape::POINTS_PER_SIDE;

// A session whose calibrated units are output pixels
fn session(width: u32, height: u32) -> DrawingSession {
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (width, height), layer_opacity: 255, background_fill_color: Color::TRANSPARENT, ..DrawingSessionOptions::default() });
    s.add_layer("Ink", Color::from_rgb(255, 0, 0));
    s
}

fn rect(s: &mut DrawingSession, x: f32, y: f32, w: f32, h: f32) {
    s.draw_rectangle(x, y, w, h, None, None, true, 0.0).unwrap();
}

fn filled_rect(x: f32, y: f32, w: f32, h: f32) -> RectangleShape {
    RectangleShape::new(x, y, w, h, RectangleShapeOptions { is_filled: true, ..RectangleShapeOptions::default() }).unwrap()
}

// The plane as rows of '#' (1) and '.' (0)
fn rows(plane: &DrawingPlane) -> Vec<String> {
    (0..plane.height()).map(|y| (0..plane.width()).map(|x| {
        let v = plane.get(x, y).unwrap();
        if v == 1.0 {
            '#'
        } else if v == 0.0 {
            '.'
        } else {
            '?'
        }
    }).collect()).collect()
}

fn stroke(cx: f32, cy: f32, length: f32, width: f32, angle: f32, bend: f32, profile: f32) -> BrushStrokeShape {
    BrushStrokeShape::new(cx, cy, length, width, angle, BrushStrokeShapeOptions { bend, profile, color: None }).unwrap()
}

fn close(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

// ---- BrushStrokeShape (C# BrushStrokeShapeTests) ------------------------------------------------

#[test]
fn constructor_builds_a_polygon_from_tip_to_tip_along_both_sides() {
    //Arrange - a straight stroke along +x: tips at x 40 and 60, half width 5 at the middle
    let s = stroke(50.0, 20.0, 20.0, 10.0, 0.0, 0.0, 4.0);
    //Act
    let points = s.get_points();
    //Assert
    assert_eq!(points.len(), (2 * PPS) - 2);
    assert_eq!(points[0], PointF::new(40.0, 20.0));
    assert_eq!(points[PPS - 1], PointF::new(60.0, 20.0));
    assert_eq!(points[8], PointF::new(50.0, 25.0));
    assert_eq!(points[(2 * PPS) - 2 - 8], PointF::new(50.0, 15.0));
    assert_eq!(s.get_bounds(), RectangleF::new(40.0, 15.0, 20.0, 10.0));
}

#[test]
fn bend_moves_the_middle_of_the_centre_line_across_by_bend_times_length() {
    //Arrange
    let straight = stroke(0.0, 0.0, 100.0, 10.0, 0.0, 0.0, 4.0);
    let bent = stroke(0.0, 0.0, 100.0, 10.0, 0.0, 0.1, 4.0);
    //Act
    let (middle, tip) = (bent.get_points()[8], bent.get_points()[0]);
    //Assert - bend x length x (4 s^2 - 1) at s = 0 is -10; the tips stay on the axis
    assert_eq!(middle.y, straight.get_points()[8].y - 10.0);
    assert_eq!(tip, PointF::new(-50.0, 0.0));
}

#[test]
fn profile_sets_the_width_profile_exponent() {
    //Arrange - point 4 of 17 is at s = -0.25, where |2s| = 0.5
    let blunt = stroke(0.0, 0.0, 32.0, 20.0, 0.0, 0.0, BrushStrokeShape::DEFAULT_PROFILE);
    let pointed = stroke(0.0, 0.0, 32.0, 20.0, 0.0, 0.0, 2.0);
    //Assert - 10 x sqrt(1 - 0.5^p)
    assert_eq!(blunt.get_points()[4].y, (10.0 * (1.0f64 - 0.0625).sqrt()) as f32);
    assert_eq!(pointed.get_points()[4].y, (10.0 * (1.0f64 - 0.25).sqrt()) as f32);
    assert_eq!(pointed.profile(), 2.0);
    assert_eq!(BrushStrokeShapeOptions::default().profile, 4.0);
}

#[test]
fn angle_turns_the_stroke_about_its_centre() {
    //Arrange - pointing straight down the screen
    let s = stroke(10.0, 10.0, 20.0, 4.0, 90.0, 0.0, 4.0);
    //Act
    let points = s.get_points();
    //Assert
    assert!(close(points[0].x, 10.0, 1e-5) && close(points[0].y, 0.0, 1e-5));
    assert!(close(points[PPS - 1].y, 20.0, 1e-5));
    assert!(close(points[8].x, 8.0, 1e-5));
}

#[test]
fn a_path_angle_passes_straight_to_a_stroke_and_matches_a_lines_axes() {
    //Arrange - a path heading straight down the screen, and a line along it
    let path = PathGeometryBuilder::new().move_to(10.0, 0.0).line_to(10.0, 20.0).build();
    let point = path.get_point_at(0.5);
    let line = DrawingShape::from(crate::shapes::LineShape::new(10.0, 0.0, 10.0, 20.0, crate::shapes::LineShapeOptions { stroke_thickness: 2.0, color: None }).unwrap());
    //Act - all three speak degrees, clockwise on screen from +x
    let s = stroke(point.position.x, point.position.y, 20.0, 4.0, point.angle(), 0.0, 4.0);
    let line_axes = line.get_axes().unwrap();
    //Assert
    assert_eq!(point.angle(), 90.0);
    assert_eq!(line_axes.angle(), 90.0);
    assert_eq!(s.get_axes().angle(), line_axes.angle());
    assert!(close(s.get_points()[0].x, 10.0, 1e-5) && close(s.get_points()[0].y, 0.0, 1e-5));
}

#[test]
fn get_outline_get_coverage_areas_and_get_axes_describe_the_stroke() {
    //Arrange
    let s = BrushStrokeShape::new(5.0, 6.0, 30.0, 8.0, 23.0, BrushStrokeShapeOptions { bend: 0.05, profile: 3.0, color: Some(Color::from_rgb(255, 215, 0)) }).unwrap();
    //Act
    let outline = s.get_outline();
    let areas = s.get_coverage_areas(0.1);
    //Assert
    assert_eq!(outline.contour_count(), 1);
    assert_eq!(outline.fill_rule(), PathFillRule::NonZero);
    assert_eq!(outline.bez_path().elements().last(), Some(&kurbo::PathEl::ClosePath));
    assert_eq!(areas, vec![outline]);
    assert_eq!(s.get_axes(), ElementAxes::new(PointF::new(5.0, 6.0), 23.0, 30.0, 8.0).unwrap());
    assert_eq!(s.color(), Some(Color::from_rgb(255, 215, 0)));
}

#[test]
fn constructor_rejects_unusable_values() {
    let o = BrushStrokeShapeOptions::default();
    assert!(BrushStrokeShape::new(0.0, 0.0, 0.0, 5.0, 0.0, o).is_err());
    assert!(BrushStrokeShape::new(0.0, 0.0, 5.0, -1.0, 0.0, o).is_err());
    assert!(BrushStrokeShape::new(0.0, 0.0, 5.0, 5.0, f32::NAN, o).is_err());
    assert!(BrushStrokeShape::new(0.0, 0.0, 5.0, 5.0, 0.0, BrushStrokeShapeOptions { profile: 0.0, ..o }).is_err());
    assert!(ElementAxes::new(PointF::new(0.0, 0.0), 0.0, -1.0, 0.0).is_err());
}

#[test]
fn draw_brush_stroke_adds_the_shape_to_the_active_layer() {
    //Arrange
    let mut s = session(200, 200);
    //Act
    let id = s.draw_brush_stroke(100.0, 100.0, 80.0, 20.0, 57.0, BrushStrokeShapeOptions { bend: 0.1, color: Some(Color::from_rgb(0, 0, 255)), ..BrushStrokeShapeOptions::default() }).unwrap();
    //Assert
    let layer = s.get_layer("Ink").unwrap();
    assert_eq!(layer.element_count(), 1);
    let ShapeKind::BrushStroke(b) = layer.find_element(id).unwrap().kind() else { panic!("not a brushstroke") };
    assert_eq!((b.bend(), b.angle(), b.color()), (0.1, 57.0, Some(Color::from_rgb(0, 0, 255))));
}

// ---- Painting a stroke on (no C# equivalent) ----------------------------------------------------

#[test]
fn a_stroke_painted_to_progress_one_is_the_whole_stroke_and_to_zero_nothing() {
    //Arrange
    let whole = stroke(40.0, 30.0, 50.0, 12.0, 23.0, 0.08, 4.0);
    //Act
    let full = whole.clone().with_progress(1.0);
    let none = whole.clone().with_progress(0.0);
    //Assert
    assert_eq!(full.get_drawn_points(), whole.get_points());
    assert_eq!(full.get_outline(), whole.get_outline());
    assert!(none.get_outline().is_empty());
    assert!(none.get_coverage_areas(0.1).is_empty());
    assert_eq!(none.get_axes(), whole.get_axes());
}

#[test]
fn a_partly_painted_stroke_keeps_the_outline_up_to_the_cut_and_cuts_straight_across() {
    //Arrange - along +x from x 0 to 32: s = -0.5 + progress; points every 2 units
    let whole = stroke(16.0, 0.0, 32.0, 8.0, 0.0, 0.0, 4.0);
    //Act - progress 0.3: the cut at x 9.6, after the side points at x 0, 2, 4, 6, 8
    let part = whole.clone().with_progress(0.3);
    let drawn = part.get_drawn_points();
    //Assert - 5 points along the first side, 2 at the cut, 4 back along the other side
    assert_eq!(drawn.len(), 11);
    assert_eq!(&drawn[..5], &whole.get_points()[..5]);
    assert!(close(drawn[5].x, 9.6, 1e-5) && close(drawn[6].x, 9.6, 1e-5));
    assert!(drawn[5].y > 0.0 && drawn[6].y < 0.0 && close(drawn[5].y, -drawn[6].y, 1e-6));
    assert_eq!(drawn[7], whole.get_points()[(2 * PPS) - 2 - 4]);
    assert_eq!(drawn[10], whole.get_points()[(2 * PPS) - 2 - 1]);
    assert!(close(part.get_bounds().width, 9.6, 1e-5));
    assert_eq!(part.get_axes(), whole.get_axes());
}

#[test]
fn a_painted_stroke_owns_more_pixels_as_its_progress_grows() {
    //Arrange
    let whole = stroke(30.0, 20.0, 50.0, 14.0, 12.0, 0.1, 4.0);
    let mut counts = Vec::new();
    //Act
    for p in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let mut s = session(60, 40);
        s.draw_shape(whole.clone().with_progress(p));
        counts.push(s.export_ownership_map((60, 40)).unwrap().get_pixel_count(0).unwrap());
    }
    //Assert
    assert_eq!(counts[0], 0);
    assert!(counts.windows(2).all(|w| w[0] < w[1]), "{counts:?}");
    let mut s = session(60, 40);
    s.draw_shape(whole);
    assert_eq!(counts[4], s.export_ownership_map((60, 40)).unwrap().get_pixel_count(0).unwrap());
}

// ---- Hard-edged masks (C# DrawingSessionHardEdgedMaskTests) -------------------------------------

#[test]
fn export_mask_hard_edged_covers_exactly_the_pixels_whose_centres_are_inside() {
    let mut s = session(6, 5);
    rect(&mut s, 1.0, 1.0, 3.0, 2.0);
    let mask = s.export_mask((6, 5), MaskMeaning::HardEdged).unwrap();
    assert_eq!(rows(&mask), ["......", ".###..", ".###..", "......", "......"]);
}

#[test]
fn export_mask_hard_edged_keeps_centres_on_left_and_top_edges_and_drops_right_and_bottom() {
    let mut s = session(5, 4);
    rect(&mut s, 1.5, 0.5, 2.0, 2.0);
    let mask = s.export_mask((5, 4), MaskMeaning::HardEdged).unwrap();
    assert_eq!(rows(&mask), [".##..", ".##..", ".....", "....."]);
}

#[test]
fn export_mask_hard_edged_honours_the_even_odd_fill_rule() {
    //Arrange - a 6 x 6 square with a 2 x 2 square inside it, filled even-odd
    let mut s = session(6, 6);
    let mut b = PathGeometryBuilder::new();
    b.add_rectangle(RectangleF::new(0.0, 0.0, 6.0, 6.0)).add_rectangle(RectangleF::new(2.0, 2.0, 2.0, 2.0));
    b.fill_rule = PathFillRule::EvenOdd;
    s.draw_path(PathShape::new(b.build(), ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap());
    //Act
    let mask = s.export_mask((6, 6), MaskMeaning::HardEdged).unwrap();
    //Assert
    assert_eq!(rows(&mask), ["######", "######", "##..##", "##..##", "######", "######"]);
}

#[test]
fn export_mask_hard_edged_covers_an_outline_band_and_not_the_inside() {
    //Arrange - a circle of radius 8 outlined 2 units thick: the band 7 .. 9 from the centre
    let mut s = session(20, 20);
    s.draw_circle(10.0, 10.0, 8.0, Some(2.0), None, false).unwrap();
    //Act
    let mask = s.export_mask((20, 20), MaskMeaning::HardEdged).unwrap();
    //Assert
    assert_eq!(mask.get(9, 9).unwrap(), 0.0);
    assert_eq!(mask.get(10, 1).unwrap(), 1.0);
    assert_eq!(mask.get(10, 3).unwrap(), 0.0);
    assert_eq!(mask.get(0, 10).unwrap(), 0.0);
}

#[test]
fn export_mask_hard_edged_counts_a_line_with_round_caps_by_its_width() {
    //Arrange - a horizontal line from (3, 5) to (7, 5), 4 thick: round caps of radius 2 (C#'s dot
    //  test uses a freehand stroke, which this crate does not have; the caps are the same geometry)
    let mut s = session(12, 10);
    s.draw_line(3.0, 5.0, 7.0, 5.0, Some(4.0), None).unwrap();
    //Act
    let mask = s.export_mask((12, 10), MaskMeaning::HardEdged).unwrap();
    //Assert - centres within 2 of the segment; (1.5, 3.5) is 2.12 away from (3, 5)
    assert_eq!(rows(&mask), ["............", "............", "............", "..######....", ".########...", ".########...", "..######....", "............", "............", "............"]);
}

#[test]
fn export_mask_hard_edged_ignores_paint_and_layer_opacity_but_not_element_opacity_or_time() {
    //Arrange
    let mut s = session(8, 2);
    s.get_layer_mut("Ink").unwrap().set_opacity(0.0);
    s.draw_rectangle(0.0, 0.0, 2.0, 2.0, None, Some(Color::from_rgba(0, 0, 0, 0)), true, 0.0).unwrap();
    s.draw_shape(DrawingShape::from(filled_rect(2.0, 0.0, 2.0, 2.0)).with_opacity(0.0));
    s.draw_shape(DrawingShape::from(filled_rect(4.0, 0.0, 2.0, 2.0)).with_time_window(Some(5.0), None));
    s.draw_shape(DrawingShape::from(filled_rect(6.0, 0.0, 2.0, 2.0)).with_opacity(0.01));
    //Act
    let mask = s.export_mask((8, 2), MaskMeaning::HardEdged).unwrap();
    s.set_time(6.0);
    let later = s.export_mask((8, 2), MaskMeaning::HardEdged).unwrap();
    //Assert
    assert_eq!(rows(&mask), ["##....##", "##....##"]);
    assert_eq!(rows(&later), ["##..####", "##..####"]);
}

#[test]
fn export_mask_hard_edged_follows_transforms_and_the_aspect_fit() {
    //Arrange - a 4 x 4 calibration exported 8 x 4: the drawing sits at x 2 .. 6, scale 1
    let mut s = session(4, 4);
    s.draw_shape(DrawingShape::from(filled_rect(0.0, 0.0, 1.0, 1.0)).with_transform(DrawingTransform::translate(1.0, 2.0)));
    //Act
    let mask = s.export_mask((8, 4), MaskMeaning::HardEdged).unwrap();
    //Assert
    assert_eq!(rows(&mask), ["........", "........", "...#....", "........"]);
}

#[test]
fn export_mask_hard_edged_of_one_layer_and_of_the_union() {
    //Arrange
    let mut s = session(4, 1);
    rect(&mut s, 0.0, 0.0, 1.0, 1.0);
    s.add_layer("Second", Color::from_rgb(0, 0, 255));
    s.set_active_layer("Second");
    rect(&mut s, 2.0, 0.0, 1.0, 1.0);
    let mut reused = DrawingPlane::with_value(4, 1, 7.0).unwrap();
    //Act
    let one = s.export_layer_mask("Ink", (4, 1), MaskMeaning::HardEdged).unwrap();
    s.export_mask_into(&mut reused, MaskMeaning::HardEdged).unwrap();
    //Assert
    assert_eq!(rows(&one), ["#..."]);
    assert_eq!(rows(&reused), ["#.#."]);
}

#[test]
fn export_mask_hard_edged_of_a_scaled_export_scales_the_geometry() {
    let mut s = session(4, 4);
    rect(&mut s, 1.0, 1.0, 2.0, 2.0);
    let mask = s.export_mask((8, 8), MaskMeaning::HardEdged).unwrap();
    assert_eq!(rows(&mask), ["........", "........", "..####..", "..####..", "..####..", "..####..", "........", "........"]);
}

#[test]
fn export_mask_refuses_the_meanings_measured_from_the_drawn_frame() {
    let mut s = session(4, 4);
    rect(&mut s, 1.0, 1.0, 2.0, 2.0);
    assert!(s.export_mask((4, 4), MaskMeaning::Coverage).unwrap_err().contains("HardEdged"));
    assert!(s.export_mask((4, 4), MaskMeaning::VisibleAlpha).is_err());
    assert!(s.export_mask((0, 4), MaskMeaning::HardEdged).is_err());
    assert!(s.export_layer_mask("Nope", (4, 4), MaskMeaning::HardEdged).is_err());
}

#[test]
fn export_mask_hard_edged_gives_only_zeros_and_ones_for_curves_lines_and_strokes() {
    let mut s = session(64, 48);
    s.draw_circle(30.0, 20.0, 13.3, None, None, true).unwrap();
    s.draw_line(3.0, 40.0, 60.0, 5.0, Some(3.5), None).unwrap();
    s.draw_shape(stroke(40.0, 30.0, 30.0, 9.0, 34.0, 0.1, 4.0));
    let first = s.export_mask((64, 48), MaskMeaning::HardEdged).unwrap();
    let second = s.export_mask((64, 48), MaskMeaning::HardEdged).unwrap();
    assert_eq!(first.data(), second.data());
    assert!(first.data().iter().all(|&v| v == 0.0 || v == 1.0));
    assert!(first.data().iter().filter(|&&v| v == 1.0).count() > 600);
}

// ---- Ownership maps (C# DrawingOwnershipMapTests) -----------------------------------------------

// A map whose owned pixels are given as (x, y): one 1 x 1 square per pixel, on one layer
fn map_of(width: u32, height: u32, pixels: &[(u32, u32)]) -> DrawingOwnershipMap {
    let mut s = session(width, height);
    for &(x, y) in pixels {
        rect(&mut s, x as f32, y as f32, 1.0, 1.0);
    }
    s.export_ownership_map((width, height)).unwrap()
}

// The nearest owned pixel by brute force, ties to the smallest x then the smallest y
fn brute_force_fill(map: &DrawingOwnershipMap) -> Vec<i32> {
    let (w, h) = (map.width() as i64, map.height() as i64);
    let owners = map.owners();
    let mut result = vec![0; owners.len()];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if owners[i] >= 0 {
                result[i] = owners[i];
                continue;
            }
            let (mut best, mut best_owner) = (i64::MAX, N);
            for qx in 0..w {
                for qy in 0..h {
                    let owner = owners[(qy * w + qx) as usize];
                    if owner < 0 {
                        continue;
                    }
                    let d = (qx - x) * (qx - x) + (qy - y) * (qy - y);
                    if d < best {
                        best = d;
                        best_owner = owner;
                    }
                }
            }
            result[i] = best_owner;
        }
    }
    result
}

#[test]
fn export_ownership_map_gives_each_pixel_to_the_last_element_covering_its_centre() {
    //Arrange - two overlapping squares; the second one is painted later
    let mut s = session(5, 3);
    let first = s.draw_rectangle(0.0, 0.0, 3.0, 2.0, None, None, true, 0.0).unwrap();
    let second = s.draw_rectangle(2.0, 1.0, 2.0, 2.0, None, None, true, 0.0).unwrap();
    //Act
    let map = s.export_ownership_map((5, 3)).unwrap();
    //Assert
    assert_eq!(map.owners(), &[0, 0, 0, N, N, 0, 0, 1, 1, N, N, N, 1, 1, N]);
    assert_eq!((map.element_count(), map.unowned_count()), (2, 6));
    assert_eq!(map.get_element(0).unwrap(), first);
    assert_eq!(map.index_of(second), Some(1));
    assert_eq!((map.get_pixel_count(0).unwrap(), map.get_pixel_count(1).unwrap()), (5, 4));
    assert_eq!(map.get_bounds(0).unwrap(), Rectangle::new(0, 0, 3, 2));
    assert_eq!(map.get_bounds(1).unwrap(), Rectangle::new(2, 1, 2, 2));
    assert_eq!(map.get(3, 1).unwrap(), 1);
    assert!(map.get(5, 0).is_err() && map.get_element(2).is_err());
}

#[test]
fn export_ownership_map_numbers_every_element_across_layers_including_hidden_ones() {
    //Arrange
    let mut s = session(3, 1);
    rect(&mut s, 0.0, 0.0, 1.0, 1.0);
    let hidden = s.draw_shape(DrawingShape::from(filled_rect(1.0, 0.0, 1.0, 1.0)).with_time_window(None, Some(0.0)));
    s.add_layer("Top", Color::from_rgb(0, 0, 255));
    s.set_active_layer("Top");
    let upper = s.draw_rectangle(2.0, 0.0, 1.0, 1.0, None, None, true, 0.0).unwrap();
    //Act
    let all = s.export_ownership_map((3, 1)).unwrap();
    let only_top = s.export_layer_ownership_map("Top", (3, 1)).unwrap();
    //Assert
    assert_eq!(all.owners(), &[0, N, 2]);
    assert_eq!(all.index_of(hidden), Some(1));
    assert_eq!(all.get_pixel_count(1).unwrap(), 0);
    assert_eq!(all.get_bounds(1).unwrap(), Rectangle::default());
    assert_eq!(all.get_layer(0).unwrap(), "Ink");
    assert_eq!(all.get_layer(2).unwrap(), "Top");
    assert_eq!(only_top.owners(), &[N, N, 0]);
    assert_eq!(only_top.get_element(0).unwrap(), upper);
    assert_eq!(only_top.index_of(hidden), None);
}

#[test]
fn export_ownership_map_shares_an_edge_between_neighbours_without_gaps_or_overlaps() {
    //Arrange - two triangles splitting a square along its diagonal
    let mut s = session(8, 8);
    s.draw_polyline(&[(0.0, 0.0), (8.0, 0.0), (8.0, 8.0)], None, None, true, true).unwrap();
    s.draw_polyline(&[(0.0, 0.0), (8.0, 8.0), (0.0, 8.0)], None, None, true, true).unwrap();
    //Act
    let map = s.export_ownership_map((8, 8)).unwrap();
    //Assert
    assert_eq!(map.unowned_count(), 0);
    assert_eq!(map.get_pixel_count(0).unwrap() + map.get_pixel_count(1).unwrap(), 64);
    assert_eq!(map.get_pixel_count(0).unwrap(), 36);
    assert_eq!(map.get(3, 3).unwrap(), 0);
    assert_eq!(map.get(3, 4).unwrap(), 1);
}

#[test]
fn fill_unowned_takes_the_nearest_owner_with_ties_to_the_smallest_x() {
    let map = map_of(5, 1, &[(0, 0), (4, 0)]);
    let filled = map.fill_unowned();
    assert_eq!(filled.owners(), &[0, 0, 0, 1, 1]);
    assert_eq!(filled.unowned_count(), 0);
    assert_eq!(filled.get_pixel_count(0).unwrap(), 3);
    assert_eq!(map.owners(), &[0, N, N, N, 1]);
}

#[test]
fn fill_unowned_breaks_equal_x_ties_by_the_smallest_y() {
    let map = map_of(3, 3, &[(1, 0), (1, 2)]);
    assert_eq!(map.fill_unowned().owners(), &[0, 0, 0, 0, 0, 0, 1, 1, 1]);
}

#[test]
fn fill_unowned_matches_a_brute_force_search_on_a_scattered_map() {
    //Arrange - scattered single pixels from the crate's seeded random generator
    let mut random = crate::planes::DrawingRandom::new(31);
    let mut pixels = Vec::new();
    for _ in 0..23 {
        pixels.push((random.next_int(37).unwrap() as u32, random.next_int(29).unwrap() as u32));
    }
    let map = map_of(37, 29, &pixels);
    //Act
    let filled = map.fill_unowned();
    //Assert
    assert_eq!(filled.owners(), brute_force_fill(&map).as_slice());
    assert_eq!(filled.unowned_count(), 0);
}

#[test]
fn fill_unowned_of_a_map_with_nothing_owned_returns_it_unchanged() {
    let map = map_of(4, 4, &[]);
    let filled = map.fill_unowned();
    assert_eq!(filled, map);
    assert_eq!(filled.unowned_count(), 16);
}

#[test]
fn create_boundary_plane_marks_pixels_whose_left_or_upper_neighbour_differs() {
    //Arrange
    let mut s = session(4, 3);
    rect(&mut s, 0.0, 0.0, 2.0, 3.0);
    rect(&mut s, 2.0, 0.0, 2.0, 3.0);
    rect(&mut s, 1.0, 1.0, 1.0, 1.0);
    //Act
    let seams = s.export_ownership_map((4, 3)).unwrap().create_boundary_plane(None).unwrap();
    //Assert - owners 0 0 1 1 / 0 2 1 1 / 0 0 1 1
    assert_eq!(seams.data(), &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0]);
}

#[test]
fn create_boundary_plane_with_a_blur_is_the_hard_seams_blurred() {
    let mut s = session(30, 20);
    s.draw_circle(12.0, 9.0, 7.0, None, None, true).unwrap();
    let map = s.export_ownership_map((30, 20)).unwrap();
    let soft = map.create_boundary_plane(Some(1.3)).unwrap();
    let mut hard = map.create_boundary_plane(None).unwrap();
    hard.gaussian_blur(1.3).unwrap();
    assert_eq!(soft.data(), hard.data());
    assert!(map.create_boundary_plane(Some(-1.0)).is_err());
}

#[test]
fn get_mean_colors_averages_the_source_under_each_element() {
    //Arrange - element 0 owns x 0..1, element 1 owns x 2; the source is a red ramp
    let mut source = DrawingPlaneImage::new(4, 1).unwrap();
    source.fill_with(|x, _| [x as f32 * 0.25, 0.5, 0.0, 1.0]);
    let mut s = session(4, 1);
    rect(&mut s, 0.0, 0.0, 2.0, 1.0);
    rect(&mut s, 2.0, 0.0, 1.0, 1.0);
    rect(&mut s, 3.0, 0.0, 0.4, 1.0);
    let map = s.export_ownership_map((4, 1)).unwrap();
    //Act
    let means = map.get_mean_colors(&source);
    //Assert - (0 + 0.25) / 2; 0.5; nothing for the sliver (its centre is outside)
    assert_eq!(means, vec![[0.125, 0.5, 0.0, 1.0], [0.5, 0.5, 0.0, 1.0], [0.0; 4]]);
}

#[test]
fn get_mean_colors_reads_a_half_size_source_at_half_coordinates() {
    let mut s = session(4, 2);
    rect(&mut s, 0.0, 0.0, 1.0, 2.0);
    rect(&mut s, 1.0, 0.0, 3.0, 2.0);
    let map = s.export_ownership_map((4, 2)).unwrap();
    let mut source = DrawingPlaneImage::new(2, 1).unwrap();
    source.set_pixel(0, 0, [0.2, 0.0, 0.0, 1.0]).unwrap();
    source.set_pixel(1, 0, [0.8, 0.0, 0.0, 1.0]).unwrap();
    let means = map.get_mean_colors(&source);
    assert_eq!(means[0][0], 0.2);
    let sum: f64 = [0.2f32, 0.8, 0.8, 0.2, 0.8, 0.8].iter().map(|&v| v as f64).sum();
    assert_eq!(means[1][0], (sum / 6.0) as f32);
}

#[test]
fn get_mean_colors_weights_by_alpha_so_transparent_pixels_lend_no_colour() {
    let mut s = session(2, 1);
    rect(&mut s, 0.0, 0.0, 2.0, 1.0);
    let map = s.export_ownership_map((2, 1)).unwrap();
    let mut source = DrawingPlaneImage::new(2, 1).unwrap();
    source.set_pixel(0, 0, [0.0, 0.0, 1.0, 1.0]).unwrap();
    source.set_pixel(1, 0, [1.0, 0.0, 0.0, 0.0]).unwrap();
    let rgba = [0u8, 0, 255, 255, 255, 0, 0, 0];
    assert_eq!(map.get_mean_colors(&source)[0], [0.0, 0.0, 1.0, 0.5]);
    assert_eq!(map.get_mean_colors_bytes(&rgba, 2, 1).unwrap()[0], [0.0, 0.0, 1.0, 0.5]);
}

#[test]
fn get_mean_colors_of_bytes_uses_exact_integer_sums() {
    let mut s = session(3, 1);
    rect(&mut s, 0.0, 0.0, 3.0, 1.0);
    let map = s.export_ownership_map((3, 1)).unwrap();
    let rgba = [10u8, 20, 30, 255, 11, 21, 31, 255, 12, 22, 32, 255];
    let mean = map.get_mean_colors_bytes(&rgba, 3, 1).unwrap()[0];
    let c = |v: f64| (v * 255.0 / (255.0 * 765.0)) as f32;
    assert_eq!(mean, [c(33.0), c(63.0), c(93.0), 1.0]);
    assert!(map.get_mean_colors_bytes(&rgba[..8], 3, 1).is_err());
}

#[test]
fn apply_colors_paints_owned_pixels_and_leaves_the_rest() {
    //Arrange
    let map = map_of(3, 1, &[(0, 0), (2, 0)]);
    let mut image = DrawingPlaneImage::new(3, 1).unwrap();
    image.fill(0.1, 0.2, 0.3, Some(0.4));
    let mut rgba = [9u8; 12];
    //Act
    map.apply_colors(&mut image, &[[1.0, 0.0, 0.0, 1.0], [0.0, 0.0, 1.0, 0.0]]).unwrap();
    map.apply_colors_bytes(&mut rgba, &[Color::from_rgba(1, 2, 3, 4), Color::from_rgba(5, 6, 7, 8)]).unwrap();
    //Assert
    assert_eq!(image.get_pixel(0, 0).unwrap(), [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(image.get_pixel(1, 0).unwrap(), [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(image.get_pixel(2, 0).unwrap(), [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(rgba, [1, 2, 3, 4, 9, 9, 9, 9, 5, 6, 7, 8]);
}

#[test]
fn apply_colors_checks_its_arguments() {
    let map = map_of(3, 1, &[(0, 0), (2, 0)]);
    assert!(map.apply_colors(&mut DrawingPlaneImage::new(2, 1).unwrap(), &[[1.0; 4], [1.0; 4]]).is_err());
    assert!(map.apply_colors(&mut DrawingPlaneImage::new(3, 1).unwrap(), &[[1.0; 4]]).is_err());
    assert!(map.apply_colors_bytes(&mut [0u8; 8], &[Color::WHITE, Color::WHITE]).is_err());
}

#[test]
fn apply_colors_with_float_colours_writes_bytes_by_the_planes_rounding_rule() {
    let map = map_of(3, 1, &[(0, 0), (2, 0)]);
    let mut rgba = [9u8; 12];
    map.apply_float_colors_bytes(&mut rgba, &[[0.5, 1.5, -0.25, f32::NAN], [1.0, 0.0, 0.2, 0.25]]).unwrap();
    assert_eq!(rgba, [128, 255, 0, 0, 9, 9, 9, 9, 255, 0, 51, 64]);
    assert!(map.apply_float_colors_bytes(&mut [0u8; 12], &[[1.0; 4]]).is_err());
}

#[test]
fn fill_by_element_measures_each_pixel_centre_in_its_owners_axes() {
    //Arrange - a rectangle whose axes are its centre (4, 2) along +x; a second one moved by a
    //  transform (its axes are in its own coordinates, so the move does not show)
    let mut s = session(10, 4);
    rect(&mut s, 1.0, 0.0, 6.0, 4.0);
    s.draw_shape(DrawingShape::from(filled_rect(0.0, 0.0, 2.0, 4.0)).with_transform(DrawingTransform::translate(8.0, 0.0)));
    let map = s.export_ownership_map((10, 4)).unwrap();
    let mut along = DrawingPlane::new(10, 4).unwrap();
    let mut across = DrawingPlane::new(10, 4).unwrap();
    //Act
    map.fill_by_element(&mut along, |p| p.along() as f32, Some(-99.0), None).unwrap();
    map.fill_by_element(&mut across, |p| p.normalized_across() as f32, None, None).unwrap();
    //Assert
    assert_eq!(along.get(0, 0).unwrap(), -99.0);
    assert_eq!(along.get(1, 0).unwrap(), -2.5);
    assert_eq!(along.get(6, 3).unwrap(), 2.5);
    assert_eq!(along.get(8, 1).unwrap(), -0.5);
    assert_eq!(along.get(9, 1).unwrap(), 0.5);
    assert_eq!(across.get(3, 0).unwrap(), -0.75);
    assert_eq!(across.get(3, 3).unwrap(), 0.75);
    assert_eq!(map.get_axes(1).unwrap(), ElementAxes::new(PointF::new(1.0, 2.0), 0.0, 2.0, 4.0).unwrap());
}

#[test]
fn fill_by_element_uses_a_brushstrokes_own_axes_and_accepts_overrides() {
    //Arrange - a vertical stroke: along runs down the screen, across to the left
    let mut s = session(20, 40);
    let id = s.draw_brush_stroke(10.0, 20.0, 30.0, 10.0, 90.0, BrushStrokeShapeOptions::default()).unwrap();
    let map = s.export_ownership_map((20, 40)).unwrap();
    let mut along = DrawingPlane::new(20, 40).unwrap();
    let mut overridden = DrawingPlane::new(20, 40).unwrap();
    let other = |_i: usize| Some(ElementAxes::new(PointF::new(0.0, 0.0), 0.0, 1.0, 1.0).unwrap());
    //Act
    map.fill_by_element(&mut along, |p| p.along() as f32, None, None).unwrap();
    map.fill_by_element(&mut overridden, |p| p.along() as f32, Some(0.0), Some(&other)).unwrap();
    let pixel = map.try_get_element_pixel(12, 30).unwrap().unwrap();
    //Assert
    let ShapeKind::BrushStroke(b) = s.find_element(id).unwrap().kind() else { panic!("not a brushstroke") };
    assert_eq!(map.get_axes(0).unwrap(), b.get_axes());
    assert!(close(along.get(10, 30).unwrap(), 10.5, 1e-4));
    assert!(close(along.get(10, 9).unwrap(), -10.5, 1e-4));
    assert_eq!(overridden.get(10, 30).unwrap(), 10.5);
    assert!((pixel.across() - -2.5).abs() < 1e-4);
    assert!((pixel.normalized_along() - 10.5 / 15.0).abs() < 1e-5);
    assert_eq!(map.try_get_element_pixel(0, 0).unwrap(), None);
}

#[test]
fn export_ownership_map_counts_a_group_as_one_element() {
    //Arrange
    let mut s = session(4, 1);
    let mut group = DrawingGroup::new(None);
    group.add(filled_rect(0.0, 0.0, 1.0, 1.0));
    group.add(DrawingShape::from(filled_rect(2.0, 0.0, 1.0, 1.0)).with_time_window(Some(1.0), None));
    s.draw_shape(group);
    //Act
    let early = s.export_ownership_map((4, 1)).unwrap();
    s.set_time(2.0);
    let late = s.export_ownership_map((4, 1)).unwrap();
    //Assert - the child's time window applies inside the group
    assert_eq!(early.owners(), &[0, N, N, N]);
    assert_eq!(late.owners(), &[0, N, 0, N]);
    assert_eq!(late.element_count(), 1);
}

#[test]
fn a_group_reports_its_childrens_areas_through_their_transforms() {
    let mut group = DrawingGroup::new(None);
    group.add(DrawingShape::from(filled_rect(0.0, 0.0, 1.0, 1.0)).with_transform(DrawingTransform::translate(3.0, 0.0)));
    let areas = DrawingShape::from(group).get_coverage_areas(0.1).unwrap();
    assert_eq!(areas.len(), 1);
    assert_eq!(areas[0].get_bounds(), RectangleF::new(3.0, 0.0, 1.0, 1.0));
}

// ---- Stroke areas (C#'s stroker, without its union) ---------------------------------------------

#[test]
fn a_stroke_area_covers_the_band_within_half_the_width_with_round_caps_and_joins() {
    //Arrange - an L-shaped polyline outlined 6 thick
    let mut s = session(40, 40);
    s.draw_polyline(&[(5.0, 5.0), (30.0, 5.0), (30.0, 30.0)], Some(6.0), None, false, false).unwrap();
    //Act
    let mask = s.export_mask((40, 40), MaskMeaning::HardEdged).unwrap();
    //Assert - every pixel centre: inside exactly when within 3 of the polyline (none sits
    //  closer than 1e-3 to that distance here)
    let d = |px: f64, py: f64| -> f64 {
        let seg = |ax: f64, ay: f64, bx: f64, by: f64| {
            let (dx, dy) = (bx - ax, by - ay);
            let t = (((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            ((px - ax - t * dx).powi(2) + (py - ay - t * dy).powi(2)).sqrt()
        };
        seg(5.0, 5.0, 30.0, 5.0).min(seg(30.0, 5.0, 30.0, 30.0))
    };
    let mut wrong = 0;
    for y in 0..40 {
        for x in 0..40 {
            let dist = d(x as f64 + 0.5, y as f64 + 0.5);
            let inside = mask.get(x, y).unwrap() == 1.0;
            // round arcs are chords within the 0.1-pixel tolerance: allow that band
            if (dist - 3.0).abs() > 0.1 && inside != (dist < 3.0) {
                wrong += 1;
            }
        }
    }
    assert_eq!(wrong, 0);
}

// ---- Keyframe players (C# KeyframePlayerTests, one thread) --------------------------------------

const W: u32 = 4;
const H: u32 = 2;

// Keyframe k: red = 10 k, green = pixel index, blue = 7, alpha as given
fn paint_into(k: i32, rgba: &mut [u8], alpha: u8) {
    for i in 0..(W * H) as usize {
        rgba[i * 4] = (10 * k) as u8;
        rgba[i * 4 + 1] = i as u8;
        rgba[i * 4 + 2] = 7;
        rgba[i * 4 + 3] = alpha;
    }
}

fn keyframe(k: i32, alpha: u8) -> Vec<u8> {
    let mut rgba = vec![0u8; (W * H * 4) as usize];
    paint_into(k, &mut rgba, alpha);
    rgba
}

fn expected(k: i32, fraction: f32, alpha: u8) -> Vec<u8> {
    let mut rgba = vec![0u8; (W * H * 4) as usize];
    DrawingTransition::dissolve().apply_bytes(&keyframe(k, alpha), &keyframe(k + 1, alpha), &mut rgba, W, H, fraction).unwrap();
    rgba
}

fn player(interval: f64, capacity: usize, times: Rc<RefCell<Vec<f64>>>) -> KeyframePlayer {
    KeyframePlayer::new(
        W,
        H,
        interval,
        move |k, seconds, rgba| {
            times.borrow_mut().push(seconds);
            paint_into(k, rgba, 255);
            Ok(())
        },
        KeyframePlayerOptions { cache_capacity: capacity, ..KeyframePlayerOptions::default() },
    )
    .unwrap()
}

#[test]
fn render_frame_dissolves_between_the_two_nearest_keyframes() {
    //Arrange
    let times = Rc::new(RefCell::new(Vec::new()));
    let mut p = player(2.0, 4, times.clone());
    let mut frame = vec![0u8; (W * H * 4) as usize];
    //Act
    p.render_frame(5.0, &mut frame).unwrap();
    //Assert - t 5 = keyframe 2 (time 4) to keyframe 3 (time 6) at u 0.5: red 25
    assert_eq!(frame, expected(2, 0.5, 255));
    assert_eq!(frame[0], 25);
    assert_eq!(p.keyframe_at(5.0).unwrap(), 2);
    assert_eq!(*times.borrow(), vec![4.0, 6.0]);
}

#[test]
fn render_frame_on_a_keyframe_is_that_keyframe_exactly() {
    let mut p = player(0.5, 4, Rc::new(RefCell::new(Vec::new())));
    let mut frame = vec![0u8; (W * H * 4) as usize + 3];
    frame[(W * H * 4) as usize + 2] = 99;
    p.render_frame(1.5, &mut frame).unwrap();
    assert_eq!(&frame[..(W * H * 4) as usize], keyframe(3, 255).as_slice());
    assert_eq!(frame[(W * H * 4) as usize + 2], 99);
    // on a keyframe only that keyframe is painted (no worker paints ahead)
    assert_eq!(p.paint_count(), 1);
}

#[test]
fn render_frame_after_seeks_matches_painting_on_the_spot_and_keeps_the_cache_bounded() {
    let mut p = player(1.0, 3, Rc::new(RefCell::new(Vec::new())));
    let mut frame = vec![0u8; (W * H * 4) as usize];
    for t in [0.25, 7.5, 3.75, 3.8, 0.1, 12.25, 11.9] {
        p.render_frame(t, &mut frame).unwrap();
        let k = t.floor() as i32;
        assert_eq!(frame, expected(k, (t - k as f64) as f32, 255), "t = {t}");
        assert!(p.cached_keyframe_count() <= 3);
    }
}

#[test]
fn prefetch_paints_one_keyframe_ahead_per_call_so_steady_playback_never_waits() {
    //Arrange
    let times = Rc::new(RefCell::new(Vec::new()));
    let mut p = player(1.0, 4, times.clone());
    let mut frame = vec![0u8; (W * H * 4) as usize];
    //Act - a frame, then a prefetch, as a frontend would each frame
    p.render_frame(0.5, &mut frame).unwrap();
    let painted_by_frame = p.paint_count();
    assert!(p.prefetch(0.5).unwrap());
    assert!(!p.prefetch(0.5).unwrap());
    let before = p.paint_count();
    assert!(p.is_ready(1.2).unwrap());
    p.render_frame(1.2, &mut frame).unwrap();
    //Assert - keyframes 0 and 1 for the first frame, 2 ahead, and nothing when crossing into 1..2
    assert_eq!(painted_by_frame, 2);
    assert_eq!(before, 3);
    assert_eq!(p.paint_count(), 3);
    assert_eq!(*times.borrow(), vec![0.0, 1.0, 2.0]);
    assert_eq!(frame, expected(1, (1.2f64 - 1.0) as f32, 255));
}

#[test]
fn render_frame_passes_on_a_paint_failure_and_paints_afresh_next_time() {
    let fail = Rc::new(RefCell::new(true));
    let f = fail.clone();
    let mut p = KeyframePlayer::new(
        W,
        H,
        1.0,
        move |k, _, rgba| {
            if *f.borrow() {
                return Err("boom".to_string());
            }
            paint_into(k, rgba, 255);
            Ok(())
        },
        KeyframePlayerOptions::default(),
    )
    .unwrap();
    let mut frame = vec![0u8; (W * H * 4) as usize];
    assert_eq!(p.render_frame(2.0, &mut frame), Err("boom".to_string()));
    assert_eq!(p.cached_keyframe_count(), 0);
    *fail.borrow_mut() = false;
    p.render_frame(2.0, &mut frame).unwrap();
    assert_eq!(frame, keyframe(2, 255));
    assert_eq!(p.paint_count(), 2);
}

#[test]
fn render_frame_dissolves_transparent_keyframes_in_premultiplied_space() {
    let mut p = KeyframePlayer::new(W, H, 1.0, |k, _, rgba| {
        paint_into(k, rgba, 128);
        Ok(())
    }, KeyframePlayerOptions::default())
    .unwrap();
    let mut frame = vec![0u8; (W * H * 4) as usize];
    p.render_frame(4.25, &mut frame).unwrap();
    assert_eq!(frame, expected(4, 0.25, 128));
}

#[test]
fn constructor_and_render_frame_check_their_arguments() {
    let paint = |_: i32, _: f64, _: &mut [u8]| Ok(());
    assert!(KeyframePlayer::new(0, 1, 1.0, paint, KeyframePlayerOptions::default()).is_err());
    assert!(KeyframePlayer::new(1, 1, 0.0, paint, KeyframePlayerOptions::default()).is_err());
    assert!(KeyframePlayer::new(1, 1, f64::INFINITY, paint, KeyframePlayerOptions::default()).is_err());
    assert!(KeyframePlayer::new(1, 1, 1.0, paint, KeyframePlayerOptions { cache_capacity: 2, ..KeyframePlayerOptions::default() }).is_err());
    let mut p = KeyframePlayer::new(2, 2, 1.0, paint, KeyframePlayerOptions::default()).unwrap();
    assert!(p.render_frame(0.0, &mut [0u8; 15]).is_err());
    assert!(p.render_frame(f64::NAN, &mut [0u8; 16]).is_err());
    assert!(p.render_frame(1e300, &mut [0u8; 16]).is_err());
}

#[test]
fn a_wipe_transition_can_replace_the_dissolve() {
    let wipe = DrawingTransition::wipe(DrawingTransitionDirection::Right, None).unwrap();
    let mut p = KeyframePlayer::new(W, H, 1.0, |k, _, rgba| {
        paint_into(k, rgba, 255);
        Ok(())
    }, KeyframePlayerOptions { transition: Some(wipe), ..KeyframePlayerOptions::default() })
    .unwrap();
    let mut frame = vec![0u8; (W * H * 4) as usize];
    p.render_frame(0.5, &mut frame).unwrap();
    let mut wanted = vec![0u8; (W * H * 4) as usize];
    wipe.apply_bytes(&keyframe(0, 255), &keyframe(1, 255), &mut wanted, W, H, 0.5).unwrap();
    assert_eq!(frame, wanted);
    assert_eq!(p.transition(), wipe);
}

#[test]
fn plane_keyframe_player_dissolves_float_keyframes() {
    //Arrange - keyframe k: every pixel (k / 10, 0.5, 2 (open-ended), 1)
    let mut p = PlaneKeyframePlayer::new(3, 2, 1.0, |k, _, image| {
        image.fill(k as f32 / 10.0, 0.5, 2.0, Some(1.0));
        Ok(())
    }, KeyframePlayerOptions::default())
    .unwrap();
    let mut frame = DrawingPlaneImage::new(3, 2).unwrap();
    //Act
    p.render_frame(2.25, &mut frame).unwrap();
    //Assert - the float transition of the two keyframes
    let mut a = DrawingPlaneImage::new(3, 2).unwrap();
    a.fill(0.2, 0.5, 2.0, Some(1.0));
    let mut b = DrawingPlaneImage::new(3, 2).unwrap();
    b.fill(0.3, 0.5, 2.0, Some(1.0));
    assert_eq!(frame, DrawingTransition::dissolve().apply(&a, &b, 0.25).unwrap());
    assert_eq!(frame.get_pixel(0, 0).unwrap()[2], 2.0);
}

#[test]
fn plane_keyframe_player_on_a_keyframe_copies_it_and_checks_the_destination_size() {
    let mut p = PlaneKeyframePlayer::new(3, 2, 0.5, |k, _, image| {
        // a fresh, transparent black image every time
        assert_eq!(image.get_pixel(1, 1).unwrap(), [0.0; 4]);
        image.set_pixel(1, 1, [k as f32, 0.0, 0.0, 1.0])?;
        Ok(())
    }, KeyframePlayerOptions { cache_capacity: 3, ..KeyframePlayerOptions::default() })
    .unwrap();
    let mut frame = DrawingPlaneImage::new(3, 2).unwrap();
    for t in [1.0, 5.0, 9.0, 13.0, 1.0] {
        p.render_frame(t, &mut frame).unwrap();
        assert_eq!(frame.get_pixel(1, 1).unwrap(), [(t * 2.0) as f32, 0.0, 0.0, 1.0]);
    }
    assert!(p.render_frame(1.0, &mut DrawingPlaneImage::new(2, 2).unwrap()).is_err());
    assert_eq!(p.paint_count(), 5);
}

#[test]
fn the_cache_drops_the_keyframes_farthest_ahead_of_the_shown_one_last() {
    //Arrange - capacity 3; show 5, then 0 (behind is farther than any ahead)
    let times = Rc::new(RefCell::new(Vec::new()));
    let mut p = player(1.0, 3, times.clone());
    let mut frame = vec![0u8; (W * H * 4) as usize];
    for t in [5.5, 6.5, 0.5] {
        p.render_frame(t, &mut frame).unwrap();
    }
    // keyframes painted: 5, 6, 7, 0, 1; after showing 0 the kept ones are 0, 1 and the nearest ahead
    p.render_frame(6.0, &mut frame).unwrap();
    assert_eq!(*times.borrow(), vec![5.0, 6.0, 7.0, 0.0, 1.0, 6.0]);
}
