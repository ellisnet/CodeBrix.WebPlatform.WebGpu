//! Host tests of the animation model (tracks, transforms, the camera, bindings, groups, time
//! windows, animated shapes) and of the path step (measuring, trims, morphs, motion along a
//! path), against values worked out by hand from NoSkia's formulas.

use crate::animation::{BoolTrack, ColorTrack, Easing, EasingKind, FloatTrack, Keyframe, PointTrack, TrackLoopMode};
use crate::color::Color;
use crate::geometry::{PointF, RectangleF};
use crate::models::{AnimatedProperty, BindingTarget, BoundTrack, DrawingCamera, DrawingTransform, DrawingTransformOptions, PathTravelOptions};
use crate::paths::{PathGeometry, PathGeometryBuilder, PathMorph, PathStrokeStyle, PathStrokeStyleOptions};
use crate::rendering::VectorScene;
use crate::shapes::{AnimatedShape, CircleShape, DrawingGroup, DrawingShape, PathShape, RectangleShape, RectangleShapeOptions, ShapeOptions};
use crate::{DrawingSession, DrawingSessionOptions};

fn near(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

fn track(keys: &[(f64, f32, EasingKind)]) -> FloatTrack {
    FloatTrack::from_keys(keys).unwrap()
}

#[test]
fn a_float_track_interpolates_each_segment_with_the_later_keys_easing() {
    let t = track(&[(0.0, 0.0, EasingKind::Linear), (2.0, 10.0, EasingKind::Linear), (4.0, 20.0, EasingKind::CubicInOut)]);
    assert_eq!(t.evaluate(-1.0), Some(0.0), "before the first key: its value");
    assert_eq!(t.evaluate(1.0), Some(5.0));
    // CubicInOut at u = 0.25 is 4 u^3 = 0.0625: 10 + 10 x 0.0625.
    assert!(near(t.evaluate(2.5).unwrap(), 10.625, 1e-5));
    assert_eq!(t.evaluate(9.0), Some(20.0), "after the last key: its value");
    assert_eq!(t.evaluate(f64::NAN), Some(0.0), "NaN counts as the first key's time");
    assert_eq!(FloatTrack::new().evaluate(1.0), None, "no keys (C# throws)");
    assert_eq!((t.count(), t.start_time(), t.end_time(), t.duration()), (3, 0.0, 4.0, 4.0));
}

#[test]
fn keys_are_kept_in_time_order_and_a_hold_key_jumps() {
    let mut t = FloatTrack::new();
    t.add(2.0, 8.0, EasingKind::Linear).unwrap();
    t.add(0.0, 0.0, EasingKind::Linear).unwrap();
    t.add_hold(3.0, 100.0).unwrap();
    assert_eq!(t.get_keyframes().iter().map(|k| k.time()).collect::<Vec<_>>(), vec![0.0, 2.0, 3.0]);
    assert_eq!(t.evaluate(2.9), Some(8.0), "up to a hold key: the earlier value");
    assert_eq!(t.evaluate(3.0), Some(100.0));
    assert!(t.add(f64::INFINITY, 1.0, EasingKind::Linear).is_err());
    let custom = Keyframe::with_curve(1.0, 4.0f32, |u| u * u).unwrap();
    let mut c = FloatTrack::new();
    c.add_key(Keyframe::new(0.0, 0.0, EasingKind::Linear).unwrap()).add_key(custom);
    assert_eq!(c.evaluate(0.5), Some(1.0), "a custom curve: 4 x 0.5^2");
}

#[test]
fn loop_and_ping_pong_map_the_time_back_into_the_keys() {
    let base = track(&[(1.0, 0.0, EasingKind::Linear), (3.0, 20.0, EasingKind::Linear)]);
    let looped = base.clone().with_loop_mode(TrackLoopMode::Loop);
    // span 2: 4.5 -> 1 + (3.5 % 2) = 2.5 -> 15; 0.5 -> 1 + (-0.5 + 2) = 2.5.
    assert_eq!(looped.evaluate(4.5), Some(15.0));
    assert_eq!(looped.evaluate(0.5), Some(15.0));
    let ping = base.with_loop_mode(TrackLoopMode::PingPong);
    // 4.5: doubled = 3.5 % 4 = 3.5 > 2 -> 4 - 3.5 = 0.5 -> 1.5 -> 5.
    assert_eq!(ping.evaluate(4.5), Some(5.0));
    assert_eq!(ping.evaluate(2.0), Some(10.0));
}

#[test]
fn point_colour_and_bool_tracks_interpolate_as_csharp() {
    let p = PointTrack::from_keys(&[(0.0, PointF::new(0.0, 10.0), EasingKind::Linear), (1.0, PointF::new(10.0, 30.0), EasingKind::Linear)]).unwrap();
    assert_eq!(p.evaluate(0.25), Some(PointF::new(2.5, 15.0)));
    let c = ColorTrack::from_keys(&[(0.0, Color::from_rgba(0, 0, 0, 0), EasingKind::Linear), (1.0, Color::from_rgba(255, 1, 100, 255), EasingKind::Linear)]).unwrap();
    // ColorRamp.Channel rounds half away from zero: 127.5 -> 128, 0.5 -> 1, 50 -> 50.
    assert_eq!(c.evaluate(0.5), Some(Color::from_rgba(128, 1, 50, 128)));
    let b = BoolTrack::from_keys(&[(0.0, false, EasingKind::Linear), (1.0, true, EasingKind::Linear)]).unwrap();
    assert_eq!((b.evaluate(0.99), b.evaluate(1.0)), (Some(false), Some(true)));
    // The eased fraction is the curve's: BackOut overshoots past the end value.
    let over = track(&[(0.0, 0.0, EasingKind::Linear), (1.0, 100.0, EasingKind::BackOut)]);
    assert!(near(over.evaluate(0.7).unwrap(), (100.0 * Easing::back_out(0.7)) as f32, 1e-4) && over.evaluate(0.7).unwrap() > 100.0);
}

#[test]
fn a_transform_scales_and_rotates_about_its_anchor_then_moves() {
    let t = DrawingTransform::new(PointF::new(10.0, 20.0), DrawingTransformOptions { rotation: 90.0, scale_x: 2.0, scale_y: 3.0, anchor: PointF::new(5.0, 5.0) }).unwrap();
    // (6, 5): minus the anchor (1, 0); scaled (2, 0); turned 90 degrees clockwise (y down) (0, 2);
    // plus anchor + translation (15, 25) -> (15, 27).
    let p = t.transform_point(PointF::new(6.0, 5.0));
    assert!(near(p.x, 15.0, 1e-4) && near(p.y, 27.0, 1e-4), "{p:?}");
    // The box of (5,5)-(7,6): corners (5,5)->(15,25), (7,5)->(15,29), (5,6)->(12,25), (7,6)->(12,29).
    let b = t.transform_bounds(RectangleF::new(5.0, 5.0, 2.0, 1.0));
    assert!(near(b.x, 12.0, 1e-4) && near(b.y, 25.0, 1e-4) && near(b.width, 3.0, 1e-4) && near(b.height, 4.0, 1e-4), "{b:?}");
    assert!(DrawingTransform::rotate(45.0, PointF::new(1.0, 1.0)).with_rotation(0.0).is_identity(), "the anchor alone changes nothing");
    assert!(DrawingTransform::new(PointF::new(f32::NAN, 0.0), DrawingTransformOptions::default()).is_err());
}

#[test]
fn the_camera_pans_zooms_and_rolls_about_its_pivot_scaled_by_parallax() {
    let mut camera = DrawingCamera::new();
    camera.set_pan(PointF::new(100.0, 0.0)).unwrap();
    camera.set_zoom(2.0).unwrap();
    // Pivot = the centre (500, 500): (600, 500) - pivot - pan = (0, 0) -> stays at the pivot.
    let v = camera.to_view(PointF::new(600.0, 500.0), (1000, 1000), 1.0);
    assert!(near(v.x, 500.0, 1e-3) && near(v.y, 500.0, 1e-3));
    // (700, 500): (100, 0) x 2 + pivot = (700, 500).
    assert!(near(camera.to_view(PointF::new(700.0, 500.0), (1000, 1000), 1.0).x, 700.0, 1e-3));
    // Parallax 0.5: pan 50, zoom 2^0.5. (550, 500) -> (0, 0) -> pivot.
    let half = camera.to_view(PointF::new(550.0, 500.0), (1000, 1000), 0.5);
    assert!(near(half.x, 500.0, 1e-3));
    let x = camera.to_view(PointF::new(650.0, 500.0), (1000, 1000), 0.5).x;
    assert!(near(x, 500.0 + 100.0 * 2f32.sqrt(), 1e-2), "{x}");
    // Parallax 0: fixed to the frame. Roll 90: the scene turns the other way on screen.
    assert_eq!(camera.to_view(PointF::new(10.0, 20.0), (1000, 1000), 0.0), PointF::new(10.0, 20.0));
    let mut roll = DrawingCamera::new();
    roll.set_rotation(90.0).unwrap();
    let r = roll.to_view(PointF::new(600.0, 500.0), (1000, 1000), 1.0);
    assert!(near(r.x, 500.0, 1e-3) && near(r.y, 400.0, 1e-3), "{r:?}");
    let back = roll.to_scene(r, (1000, 1000), 1.0);
    assert!(near(back.x, 600.0, 1e-3) && near(back.y, 500.0, 1e-3));
    assert!(camera.set_zoom(0.0).is_err());
}

fn session() -> DrawingSession {
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (1000, 1000), layer_opacity: 255, ..DrawingSessionOptions::default() });
    s.add_layer("A", Color::WHITE);
    s
}

#[test]
fn setting_the_time_applies_bindings_to_elements_layers_and_the_camera() {
    let mut s = session();
    let ball = s.draw_circle(0.0, 0.0, 10.0, None, None, true).unwrap();
    s.set_camera(Some(DrawingCamera::new()));
    let position = PointTrack::from_keys(&[(0.0, PointF::new(0.0, 0.0), EasingKind::Linear), (2.0, PointF::new(100.0, 50.0), EasingKind::Linear)]).unwrap();
    let mut animations = s.animations_mut();
    animations.bind(ball, AnimatedProperty::Position, position).unwrap();
    animations.bind(ball, AnimatedProperty::Opacity, track(&[(0.0, 0.0, EasingKind::Linear), (1.0, 2.0, EasingKind::Linear)])).unwrap();
    animations.bind(BindingTarget::layer("A"), AnimatedProperty::Rotation, track(&[(0.0, 0.0, EasingKind::Linear), (2.0, 90.0, EasingKind::Linear)])).unwrap();
    animations.bind(BindingTarget::Camera, AnimatedProperty::Scale, track(&[(0.0, 1.0, EasingKind::Linear), (2.0, 3.0, EasingKind::Linear)])).unwrap();
    animations.bind(BindingTarget::layer("A"), AnimatedProperty::IsVisible, BoolTrack::from_keys(&[(0.0, true, EasingKind::Linear), (1.5, false, EasingKind::Linear)]).unwrap()).unwrap();
    s.set_time(1.0);
    let e = s.find_element(ball).unwrap();
    assert_eq!(e.transform().translation(), PointF::new(50.0, 25.0));
    assert_eq!(e.opacity(), 1.0, "opacity clamped to 0..1");
    assert_eq!(s.get_layer("A").unwrap().transform().rotation(), 45.0);
    assert_eq!(s.camera().unwrap().zoom(), 2.0);
    assert!(s.get_layer("A").unwrap().is_visible());
    s.set_time(1.6);
    assert!(!s.get_layer("A").unwrap().is_visible());
    // Back to 1.0: the same values again (a function of the time).
    s.set_time(1.0);
    assert_eq!(s.find_element(ball).unwrap().transform().translation(), PointF::new(50.0, 25.0));
    assert_eq!(s.animations().count(), 5);
    // A later binding of the same property wins.
    s.animations_mut().bind(ball, AnimatedProperty::Position, BoundTrack::point_fn(|t| PointF::new(t as f32, 7.0))).unwrap();
    s.set_time(1.0);
    assert_eq!(s.find_element(ball).unwrap().transform().translation(), PointF::new(1.0, 7.0));
}

#[test]
fn a_binding_is_checked_against_what_it_names() {
    let mut s = session();
    let ball = s.draw_circle(0.0, 0.0, 10.0, None, None, true).unwrap();
    let line = s.draw_line(0.0, 0.0, 10.0, 0.0, None, None).unwrap();
    let mut a = s.animations_mut();
    let f = || track(&[(0.0, 0.0, EasingKind::Linear)]);
    assert!(a.bind(ball, AnimatedProperty::TrimEnd, f()).unwrap_err().contains("path shape"));
    assert!(a.bind(line, AnimatedProperty::Color, ColorTrack::new()).unwrap_err().contains("group"));
    assert!(a.bind(ball, AnimatedProperty::Position, f()).is_err(), "Position takes a point track");
    assert!(a.bind(BindingTarget::Camera, AnimatedProperty::Scale, f()).unwrap_err().contains("set_camera"));
    assert!(a.bind(BindingTarget::layer("none"), AnimatedProperty::Opacity, f()).unwrap_err().contains("no layer"));
    assert!(a.bind(ball, AnimatedProperty::GlowRadius, f()).is_ok(), "effects bind on any element");
    assert!(a.bind(ball, AnimatedProperty::GlowColor, f()).unwrap_err().contains("color track"));
    assert!(a.bind(ball, AnimatedProperty::FrameIndex, f()).unwrap_err().contains("sprite"));
    assert!(a.bind(ball, AnimatedProperty::Tint, ColorTrack::new()).unwrap_err().contains("picture"));
    assert!(a.bind(ball, AnimatedProperty::RevealFraction, f()).unwrap_err().contains("text block"));
    let lonely = DrawingShape::from(CircleShape::new(0.0, 0.0, 1.0, ShapeOptions::default()).unwrap()).id();
    assert!(a.bind(lonely, AnimatedProperty::Opacity, f()).unwrap_err().contains("add it"));
    let path = PathShape::new(PathGeometry::parse("M 0 0 L 10 0").unwrap(), ShapeOptions::default()).unwrap();
    drop(a);
    let id = s.draw_path(path);
    assert!(s.animations_mut().bind(id, AnimatedProperty::MorphFraction, f()).unwrap_err().contains("PathMorph"));
    assert!(s.animations_mut().bind(id, AnimatedProperty::TrimEnd, f()).is_ok());
    let b = s.animations_mut().bind(id, AnimatedProperty::TrimStart, f()).unwrap();
    assert!(s.animations_mut().remove(&b) && !s.animations_mut().remove(&b));
}

#[test]
fn a_group_fades_and_colours_as_one_and_holds_its_children_in_its_coordinates() {
    let mut s = session();
    let mut group = DrawingGroup::new(Some(Color::from_rgb(1, 2, 3)));
    let inner = group.add(RectangleShape::new(-5.0, -5.0, 10.0, 10.0, RectangleShapeOptions { is_filled: true, ..RectangleShapeOptions::default() }).unwrap());
    group.add(CircleShape::new(20.0, 0.0, 5.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap());
    let g = s.draw_shape(DrawingShape::from(group).with_transform(DrawingTransform::translate(100.0, 0.0)));
    assert!(s.find_element(inner).is_some(), "children are found by id");
    let bounds = s.find_element(g).unwrap().get_transformed_bounds().unwrap();
    assert_eq!(bounds, RectangleF::new(95.0, -5.0, 30.0, 10.0));
    s.animations_mut().bind(g, AnimatedProperty::Color, ColorTrack::from_keys(&[(0.0, Color::from_rgb(9, 9, 9), EasingKind::Linear)]).unwrap()).unwrap();
    s.animations_mut().bind(g, AnimatedProperty::Opacity, track(&[(0.0, 0.5, EasingKind::Linear)])).unwrap();
    s.set_time(0.0);
    assert_eq!(s.find_element(g).unwrap().as_group().unwrap().content_color(), Some(Color::from_rgb(9, 9, 9)));
    let mut scene = VectorScene::new();
    s.render(&mut scene, 1000, 1000);
    assert_eq!(scene.item_kinds(), "LLFFPP", "the layer, then the faded group as one layer");
}

#[test]
fn time_windows_and_animated_shapes_follow_the_session_time() {
    let mut s = session();
    let early = s.draw_shape(DrawingShape::from(CircleShape::new(0.0, 0.0, 1.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap()).with_time_window(None, Some(1.0)));
    s.draw_shape(AnimatedShape::new(|t| if t > 2.0 { None } else { Some(CircleShape::new(t as f32, 0.0, 1.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap().into()) }, 0.0));
    let mut scene = VectorScene::new();
    s.set_time(0.5);
    s.render(&mut scene, 100, 100);
    assert_eq!(scene.item_kinds(), "LFFP", "both shown at 0.5");
    assert!(s.find_element(early).unwrap().is_visible_at(0.5) && !s.find_element(early).unwrap().is_visible_at(1.0));
    s.set_time(1.5);
    scene.reset();
    s.render(&mut scene, 100, 100);
    assert_eq!(scene.item_kinds(), "LFP", "the early one is gone; the animated one remains");
    s.set_time(3.0);
    scene.reset();
    s.render(&mut scene, 100, 100);
    assert_eq!(scene.item_kinds(), "LP", "the factory gives nothing after 2 s");
    s.set_time(0.5);
    scene.reset();
    s.render(&mut scene, 100, 100);
    assert_eq!(scene.item_kinds(), "LFFP", "seeking back needs no rebuild");
}

#[test]
fn a_path_is_measured_along_its_length_from_where_csharp_starts_it() {
    let rect = PathGeometry::rectangle(RectangleF::new(0.0, 0.0, 10.0, 20.0));
    assert!(near(rect.get_length(), 60.0, 1e-4));
    let p = rect.get_point_at(0.25);
    assert!(near(p.position.x, 10.0, 1e-4) && near(p.position.y, 5.0, 1e-4) && near(p.angle(), 90.0, 1e-3), "{p:?}");
    let circle = PathGeometry::circle(PointF::new(50.0, 50.0), 10.0);
    let start = circle.get_point_at(0.0);
    assert!(near(start.position.x, 60.0, 1e-4) && near(start.position.y, 50.0, 1e-4) && near(start.angle(), 90.0, 1e-2), "a circle starts at its right, heading down (clockwise)");
    let quarter = circle.get_point_at(0.25);
    assert!(near(quarter.position.x, 50.0, 0.01) && near(quarter.position.y, 60.0, 0.01), "{quarter:?}");
    // NoSkia's four-cubic circle is within 0.03 % of 2 pi r.
    assert!(near(circle.get_length(), 62.83, 0.05), "{}", circle.get_length());
    let star = PathGeometry::star(PointF::new(0.0, 0.0), 10.0, 4.0, 5, 0.0).unwrap();
    let tip = star.get_point_at(0.0).position;
    assert!(near(tip.x, 0.0, 1e-4) && near(tip.y, -10.0, 1e-4), "the first tip straight up");
    let two = PathGeometryBuilder::new().move_to(0.0, 0.0).line_to(10.0, 0.0).move_to(0.0, 10.0).line_to(30.0, 10.0).build();
    let q = two.get_point_at(0.5);
    assert_eq!((q.contour_index, q.position), (1, PointF::new(10.0, 10.0)), "the contours count one after the other");
    assert_eq!(PathGeometry::empty().get_point_at(0.5).contour_index, -1);
}

#[test]
fn trimming_keeps_the_part_between_two_fractions_and_wraps_with_the_offset() {
    let line = PathGeometry::parse("M 0 0 L 100 0").unwrap();
    let mid = line.trim(0.25, 0.75, 0.0).unwrap();
    assert_eq!(mid.get_bounds(), RectangleF::new(25.0, 0.0, 50.0, 0.0));
    assert_eq!(line.trim(0.75, 0.25, 0.0).unwrap().get_bounds(), mid.get_bounds(), "reversed fractions swap");
    assert!(line.trim(0.5, 0.5, 0.0).unwrap().is_empty());
    // Open path, wrapping: from 0.9 for 0.3 -> 90..100 and 0..20: two pieces.
    let wrapped = line.trim(0.0, 0.3, 0.9).unwrap();
    assert_eq!(wrapped.contour_count(), 2);
    assert!(near(wrapped.get_length(), 30.0, 1e-3));
    // One closed contour: one piece across the start.
    let square = PathGeometry::rectangle(RectangleF::new(0.0, 0.0, 10.0, 10.0));
    let across = square.trim(0.0, 0.5, 0.75).unwrap();
    assert_eq!(across.contour_count(), 1);
    assert!(near(across.get_length(), 20.0, 1e-3));
    let s = across.get_point_at(0.0).position;
    assert!(near(s.x, 0.0, 1e-4) && near(s.y, 10.0, 1e-4), "starts 3/4 round: the bottom-left corner, {s:?}");
    assert!(line.trim(f32::NAN, 1.0, 0.0).is_err());
}

#[test]
fn a_path_shape_draws_its_trimmed_part_and_its_morph_at_the_fraction() {
    let mut shape = PathShape::with_style(PathGeometry::parse("M 0 0 L 100 0").unwrap(), PathStrokeStyle::new(10.0, PathStrokeStyleOptions::default()).unwrap(), None, None, None);
    shape.set_trim_end(0.5).unwrap();
    assert!(shape.is_trimmed());
    assert_eq!(shape.get_bounds(), RectangleF::new(-5.0, -5.0, 60.0, 10.0));
    shape.set_trim_end(0.0).unwrap();
    assert_eq!(shape.get_bounds(), RectangleF::new(0.0, 0.0, 0.0, 0.0), "nothing drawn: a point at the trim start");
    assert!(shape.set_trim_start(1.5).is_err());
    let a = PathGeometry::rectangle(RectangleF::new(0.0, 0.0, 10.0, 10.0));
    let b = PathGeometry::rectangle(RectangleF::new(100.0, 0.0, 10.0, 10.0));
    let morph = PathMorph::new(a.clone(), b, 0).unwrap();
    let mut m = PathShape::from_morph(morph, 0.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap();
    assert_eq!(m.geometry(), &a);
    m.set_morph_fraction(0.5).unwrap();
    let g = m.geometry().get_bounds();
    assert!(near(g.x, 50.0, 1e-3) && near(g.width, 10.0, 1e-3), "{g:?}");
    assert!(PathShape::new(a, ShapeOptions::default()).unwrap().set_morph_fraction(0.5).is_err());
}

#[test]
fn a_morph_pairs_contours_matches_start_and_direction_and_grows_missing_ones() {
    // The same square, the second run the other way round and from another corner: the morph
    // turns it back and finds the matching start, so the halfway path is the square itself.
    let a = PathGeometry::rectangle(RectangleF::new(0.0, 0.0, 10.0, 10.0));
    let b = PathGeometryBuilder::new().move_to(10.0, 10.0).line_to(10.0, 0.0).line_to(0.0, 0.0).line_to(0.0, 10.0).close().build();
    let morph = PathMorph::new(a.clone(), b, 40).unwrap();
    let half = morph.get_path(0.5);
    assert_eq!(half.get_bounds(), RectangleF::new(0.0, 0.0, 10.0, 10.0));
    assert!(near(half.get_length(), 40.0, 1e-3), "{}", half.get_length());
    // At 0 and 1 the end paths themselves; NaN counts as 0.
    assert_eq!(morph.get_path(0.0), a);
    assert_eq!(morph.get_path(f32::NAN), a);
    // A missing contour grows from the other's mean point.
    let two = PathGeometryBuilder::new().add_rectangle(RectangleF::new(0.0, 0.0, 10.0, 10.0)).add_rectangle(RectangleF::new(40.0, 40.0, 10.0, 10.0)).build();
    let grow = PathMorph::new(a, two, 0).unwrap();
    assert_eq!(grow.pair_count(), 2);
    // The first square stays; the second is half grown about its mean point (45, 45): 42.5..47.5.
    let both = grow.get_path(0.5).get_bounds();
    assert!(near(both.x, 0.0, 0.01) && near(both.right(), 47.5, 0.01) && near(both.bottom(), 47.5, 0.01), "{both:?}");
    assert!(PathMorph::new(PathGeometry::empty(), PathGeometry::empty(), 1).is_err());
}

#[test]
fn binding_along_a_path_places_the_anchor_on_it_and_can_face_along_it() {
    let mut s = session();
    let dart = s.draw_circle(0.0, 0.0, 3.0, None, None, true).unwrap();
    let down = PathGeometry::parse("M 10 0 L 10 100").unwrap();
    let progress = track(&[(0.0, 0.0, EasingKind::Linear), (1.0, 1.0, EasingKind::Linear)]);
    s.animations_mut().bind_along_path(dart, &down, progress.clone(), PathTravelOptions { orient_to_path: true, rotation_offset: 10.0, ..PathTravelOptions::default() }).unwrap();
    s.set_time(0.25);
    let t = s.find_element(dart).unwrap().transform();
    assert_eq!(t.translation(), PointF::new(10.0, 25.0));
    assert!(near(t.rotation(), 100.0, 1e-3), "heading down (90) plus the offset");
    // With an anchor the anchor lies on the path.
    s.find_element_mut(dart).unwrap().set_transform(DrawingTransform::IDENTITY.with_anchor(PointF::new(5.0, 5.0)));
    s.set_time(0.5);
    let t = s.find_element(dart).unwrap().transform();
    assert_eq!(t.translation(), PointF::new(5.0, 45.0));
    assert!(s.animations_mut().bind_along_path(dart, &down, ColorTrack::new(), PathTravelOptions::default()).is_err());
}

#[test]
fn effect_sprite_and_tint_bindings_apply_with_the_time() {
    //Arrange
    use crate::effects::DrawingEffects;
    use crate::images::{ImageSource, SpriteSheet};
    use crate::shapes::{SpriteShape, SpriteShapeOptions};
    use crate::RectangleF;
    let mut s = session();
    let ball = s.draw_circle(0.0, 0.0, 10.0, None, None, true).unwrap();
    let sheet = SpriteSheet::from_grid(&ImageSource::from_rgba(&[255; 64], 4, 4).unwrap(), 4, 1, 0).unwrap();
    let sprite = s.draw_shape(SpriteShape::new(sheet, RectangleF::new(0.0, 0.0, 4.0, 4.0), SpriteShapeOptions::default()).unwrap());
    let mut a = s.animations_mut();
    a.bind(ball, AnimatedProperty::BlurRadius, track(&[(0.0, 0.0, EasingKind::Linear), (2.0, -4.0, EasingKind::Linear)])).unwrap();
    a.bind(ball, AnimatedProperty::ShadowOffset, BoundTrack::point_fn(|t| PointF::new(t as f32, 2.0))).unwrap();
    a.bind(sprite, AnimatedProperty::FrameIndex, track(&[(0.0, 0.0, EasingKind::Linear), (4.0, 3.99, EasingKind::Linear)])).unwrap();
    a.bind(BindingTarget::layer("A"), AnimatedProperty::GlowRadius, track(&[(0.0, 6.0, EasingKind::Linear)])).unwrap();
    //Act
    s.set_time(3.0);
    //Assert
    let e = s.find_element(ball).unwrap().effects().unwrap();
    assert_eq!(e.blur_radius(), 0.0, "a negative radius from a track counts as 0 (C#'s Radius)");
    assert_eq!(e.shadow_offset(), PointF::new(3.0, 2.0));
    assert_eq!(e.shadow_color(), DrawingEffects::NONE.shadow_color(), "started from None");
    assert_eq!(s.find_element(sprite).unwrap().as_sprite().unwrap().frame_index(), 2, "2.9925 rounded down");
    assert_eq!(s.get_layer("A").unwrap().effects().unwrap().glow_radius(), 6.0);
}
