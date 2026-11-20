//! Host tests of the model side of combined bindings, motion and formations - COMBINED bindings (BindingMode and the binding-order
//! rule), ROLLING and overshooting along a path, TRAINS, FORMATION bindings and LAYER VISIBILITY -
//! ported from NoSkia's DrawingAnimationsCombineTests, PathTravelTests, FormationTests and
//! DrawingLayerVisibilityTests with their stated values.

use crate::animation::{BoolTrack, EasingKind, FloatTrack, Keyframe, PointTrack};
use crate::color::Color;
use crate::formations::{FormationMember, FormationPose, IArrivalPath, StraightArrival};
use crate::geometry::PointF;
use crate::models::{AnimatedProperty, BindingMode, BindingTarget, BoundTrack, DrawingCamera, DrawingTransform, DrawingTransformOptions, ElementId, PathTrainOptions, PathTravelOptions, StaggerOrder};
use crate::paths::{PathGeometry, PathGeometryBuilder};
use crate::shapes::{CircleShape, DrawingShape, PathShape, ShapeOptions};
use crate::{DrawingSession, DrawingSessionOptions};

fn session() -> DrawingSession {
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (1000, 1000), layer_opacity: 255, ..DrawingSessionOptions::default() });
    s.add_layer("A", Color::WHITE);
    s
}

fn circle(s: &mut DrawingSession, radius: f32) -> ElementId {
    s.draw_shape(CircleShape::new(0.0, 0.0, radius, ShapeOptions::default()).unwrap())
}

fn ramp(from: f32, to: f32) -> FloatTrack {
    FloatTrack::from_keys(&[(0.0, from, EasingKind::Linear), (1.0, to, EasingKind::Linear)]).unwrap()
}

fn constant(value: f32) -> FloatTrack {
    FloatTrack::from_keys(&[(0.0, value, EasingKind::Linear)]).unwrap()
}

fn point(value: PointF) -> PointTrack {
    PointTrack::from_keys(&[(0.0, value, EasingKind::Linear)]).unwrap()
}

fn line(a: (f32, f32), b: (f32, f32)) -> PathGeometry {
    PathGeometryBuilder::new().move_to(a.0, a.1).line_to(b.0, b.1).build()
}

fn rotation(s: &DrawingSession, id: ElementId) -> f32 {
    s.find_element(id).unwrap().transform().rotation()
}

fn translation(s: &DrawingSession, id: ElementId) -> PointF {
    s.find_element(id).unwrap().transform().translation()
}

// ---- COMBINED BINDINGS (DrawingAnimationsCombineTests) ----

#[test]
fn an_add_binding_adds_to_the_replace_binding_whatever_the_binding_order() {
    //Arrange - the same two bindings made in both orders
    let mut s = session();
    let first = circle(&mut s, 5.0);
    let second = circle(&mut s, 5.0);
    s.animations_mut().bind(first, AnimatedProperty::Rotation, ramp(0.0, 90.0)).unwrap();
    s.animations_mut().bind_with_mode(first, AnimatedProperty::Rotation, ramp(10.0, 20.0), BindingMode::Add).unwrap();
    s.animations_mut().bind_with_mode(second, AnimatedProperty::Rotation, ramp(10.0, 20.0), BindingMode::Add).unwrap();
    s.animations_mut().bind(second, AnimatedProperty::Rotation, ramp(0.0, 90.0)).unwrap();
    //Act
    s.set_time(0.5);
    //Assert
    assert_eq!(rotation(&s, first), 45.0 + 15.0);
    assert_eq!(rotation(&s, second), rotation(&s, first));
}

#[test]
fn applying_again_does_not_accumulate_an_addition_without_a_replace_binding() {
    //Arrange - only an add binding: it adds to the resting value, never to its own last result
    let mut s = session();
    let shape = s.draw_shape(DrawingShape::from(CircleShape::new(0.0, 0.0, 5.0, ShapeOptions::default()).unwrap()).with_transform(DrawingTransform::new(PointF::new(0.0, 0.0), DrawingTransformOptions { rotation: 30.0, ..DrawingTransformOptions::default() }).unwrap()));
    s.animations_mut().bind_with_mode(shape, AnimatedProperty::Rotation, ramp(0.0, 10.0), BindingMode::Add).unwrap();
    //Act
    s.set_time(1.0);
    s.set_time(1.0);
    s.set_time(0.5);
    //Assert
    assert_eq!(rotation(&s, shape), 35.0);
}

#[test]
fn multiply_applies_before_add_and_both_combine_with_the_base() {
    //Arrange
    let mut s = session();
    let shape = circle(&mut s, 5.0);
    s.animations_mut().bind_with_mode(shape, AnimatedProperty::Scale, constant(0.1), BindingMode::Add).unwrap();
    s.animations_mut().bind(shape, AnimatedProperty::Scale, constant(2.0)).unwrap();
    s.animations_mut().bind_with_mode(shape, AnimatedProperty::Scale, constant(1.5), BindingMode::Multiply).unwrap();
    //Act
    s.set_time(0.0);
    //Assert - (2 x 1.5) + 0.1
    let t = s.find_element(shape).unwrap().transform();
    assert_eq!((t.scale_x(), t.scale_y()), (3.1, 3.1));
}

#[test]
fn a_position_offset_adds_to_a_path_binding() {
    //Arrange
    let mut s = session();
    let shape = circle(&mut s, 5.0);
    s.animations_mut().bind_with_mode(shape, AnimatedProperty::Position, point(PointF::new(3.0, -4.0)), BindingMode::Add).unwrap();
    s.animations_mut().bind_along_path(shape, &line((0.0, 0.0), (100.0, 0.0)), ramp(0.0, 1.0), PathTravelOptions::default()).unwrap();
    //Act
    s.set_time(0.25);
    //Assert
    assert_eq!(translation(&s, shape), PointF::new(28.0, -4.0));
}

#[test]
fn opacity_combines_then_clamps() {
    //Arrange
    let mut s = session();
    let shape = circle(&mut s, 5.0);
    s.animations_mut().bind(shape, AnimatedProperty::Opacity, constant(0.8)).unwrap();
    s.animations_mut().bind_with_mode(shape, AnimatedProperty::Opacity, constant(0.5), BindingMode::Add).unwrap();
    //Act
    s.set_time(0.0);
    //Assert
    assert_eq!(s.find_element(shape).unwrap().opacity(), 1.0);
}

#[test]
fn a_function_binding_adds_a_computed_wave() {
    //Arrange
    let mut s = session();
    let shape = circle(&mut s, 5.0);
    s.animations_mut().bind_with_mode(shape, AnimatedProperty::Rotation, BoundTrack::float_fn(|t| (10.0 * t) as f32), BindingMode::Add).unwrap();
    s.animations_mut().bind(shape, AnimatedProperty::Rotation, ramp(0.0, 100.0)).unwrap();
    s.animations_mut().bind(shape, AnimatedProperty::ScaleY, BoundTrack::float_fn(|_| 2.0)).unwrap();
    //Act
    s.set_time(0.5);
    //Assert
    let t = s.find_element(shape).unwrap().transform();
    assert_eq!((t.rotation(), t.scale_y(), t.scale_x()), (55.0, 2.0, 1.0));
}

#[test]
fn layer_and_camera_bindings_combine() {
    //Arrange
    let mut s = session();
    s.set_camera(Some(DrawingCamera::new()));
    let layer = BindingTarget::layer("A");
    s.animations_mut().bind_with_mode(layer.clone(), AnimatedProperty::Opacity, constant(0.5), BindingMode::Multiply).unwrap();
    s.animations_mut().bind_with_mode(layer, AnimatedProperty::Rotation, constant(5.0), BindingMode::Add).unwrap();
    s.animations_mut().bind(BindingTarget::Camera, AnimatedProperty::Position, point(PointF::new(100.0, 0.0))).unwrap();
    s.animations_mut().bind_with_mode(BindingTarget::Camera, AnimatedProperty::Position, point(PointF::new(1.0, 2.0)), BindingMode::Add).unwrap();
    s.animations_mut().bind_with_mode(BindingTarget::Camera, AnimatedProperty::Scale, constant(2.0), BindingMode::Multiply).unwrap();
    //Act
    s.set_time(0.0);
    s.set_time(0.0);
    //Assert
    let l = s.get_layer("A").unwrap();
    assert_eq!((l.opacity(), l.transform().rotation()), (0.5, 5.0));
    let c = s.camera().unwrap();
    assert_eq!((c.pan(), c.zoom()), (PointF::new(101.0, 2.0), 2.0));
}

#[test]
fn removing_an_add_binding_stops_it_and_leaves_the_base() {
    //Arrange
    let mut s = session();
    let shape = circle(&mut s, 5.0);
    s.animations_mut().bind(shape, AnimatedProperty::Rotation, ramp(0.0, 100.0)).unwrap();
    let wobble = s.animations_mut().bind_with_mode(shape, AnimatedProperty::Rotation, constant(7.0), BindingMode::Add).unwrap();
    s.set_time(1.0);
    assert_eq!(rotation(&s, shape), 107.0);
    //Act
    let removed = s.animations_mut().remove(&wobble);
    s.set_time(1.0);
    //Assert
    assert!(removed);
    assert_eq!(wobble.mode(), BindingMode::Add);
    assert_eq!(rotation(&s, shape), 100.0);
    assert!(s.animations().rests.is_empty(), "the slot is forgotten with its last combining binding");
}

#[test]
fn replace_mode_is_the_plain_binding_and_later_wins_as_before() {
    //Arrange
    let mut s = session();
    let shape = circle(&mut s, 5.0);
    let plain = s.animations_mut().bind(shape, AnimatedProperty::Rotation, ramp(0.0, 10.0)).unwrap();
    s.animations_mut().bind_with_mode(shape, AnimatedProperty::Rotation, ramp(50.0, 60.0), BindingMode::Replace).unwrap();
    //Act
    s.set_time(1.0);
    //Assert
    assert_eq!(plain.mode(), BindingMode::Replace);
    assert_eq!(rotation(&s, shape), 60.0);
}

#[test]
fn properties_that_cannot_combine_are_rejected() {
    //Arrange
    let mut s = session();
    let shape = s.draw_shape(PathShape::new(PathGeometry::circle(PointF::new(0.0, 0.0), 4.0), Default::default()).unwrap());
    //Act
    let trim = s.animations_mut().bind_with_mode(shape, AnimatedProperty::TrimEnd, ramp(0.0, 1.0), BindingMode::Add);
    let anchor = s.animations_mut().bind_with_mode(shape, AnimatedProperty::Anchor, point(PointF::new(1.0, 1.0)), BindingMode::Add);
    //Assert
    assert!(trim.is_err());
    assert!(anchor.is_err());
    assert_eq!(s.animations().count(), 0);
}

// ---- ROLLING AND OVERSHOOT ALONG A PATH (PathTravelTests) ----

fn road() -> PathGeometry {
    line((0.0, 50.0), (200.0, 50.0))
}

fn progress_track() -> FloatTrack {
    let mut t = FloatTrack::new();
    t.add(0.0, 0.0, EasingKind::Linear).unwrap();
    t.add(2.0, 0.6, EasingKind::CubicIn).unwrap();
    t.add_with_curve(3.0, 1.05, |u| u * (2.0 - u)).unwrap();
    t.add(4.0, 1.0, EasingKind::SineInOut).unwrap();
    t
}

#[test]
fn a_roll_turns_by_distance_over_radius_and_is_upright_at_rest() {
    //Arrange
    let mut s = session();
    let wheel = circle(&mut s, 20.0);
    s.animations_mut().bind_roll_along_path(wheel, &road(), ramp(0.0, 1.0), 20.0, None, false).unwrap();
    //Act
    s.set_time(0.0);
    let start = rotation(&s, wheel);
    s.set_time(1.0);
    let end = rotation(&s, wheel);
    //Assert - 200 units on a radius of 20: 10 radians, clockwise, so -10 rad at the start
    assert!((start - (-10.0 * 180.0 / std::f64::consts::PI) as f32).abs() <= 1e-3);
    assert_eq!(end, 0.0);
    assert_eq!(translation(&s, wheel), PointF::new(200.0, 50.0));
}

#[test]
fn the_roll_equals_a_rotation_track_mirrored_from_the_progress_track_bit_for_bit() {
    //Arrange - the hand-made way: the progress keys copied into a rotation track
    let progress = progress_track();
    let (radius, upright) = (13.0f32, 0.95f32);
    let degrees_per_fraction = (road().get_length() / radius) as f64 * 180.0 / std::f64::consts::PI;
    let mut mirrored = FloatTrack::new();
    for key in progress.get_keyframes() {
        mirrored.add_key(key.with_value(((key.value() - upright) as f64 * degrees_per_fraction) as f32));
    }
    let mut s = session();
    let wheel = circle(&mut s, radius);
    s.animations_mut().bind_along_path(wheel, &road(), progress, PathTravelOptions { roll_radius: radius, upright_at_progress: upright, extend_beyond_ends: true, ..PathTravelOptions::default() }).unwrap();
    //Assert
    let mut t = 0.0f64;
    while t <= 4.0 {
        s.set_time(t);
        assert_eq!(rotation(&s, wheel), mirrored.evaluate(t).unwrap(), "at {t}");
        t += 1.0 / 30.0;
    }
}

#[test]
fn facing_and_rolling_add() {
    //Arrange - a road heading straight down: facing is 90 degrees
    let down = line((0.0, 0.0), (0.0, 100.0));
    let mut s = session();
    let plain = circle(&mut s, 10.0);
    let both = circle(&mut s, 10.0);
    s.animations_mut().bind_roll_along_path(plain, &down, constant(0.5), 10.0, None, false).unwrap();
    s.animations_mut().bind_roll_along_path(both, &down, constant(0.5), 10.0, None, true).unwrap();
    //Act
    s.set_time(0.0);
    //Assert
    assert!((rotation(&s, both) - (90.0 + rotation(&s, plain))).abs() <= 1e-3);
}

#[test]
fn extending_beyond_the_end_carries_an_overshoot_past_the_path() {
    //Arrange
    let mut s = session();
    let clamped = circle(&mut s, 5.0);
    let extended = circle(&mut s, 5.0);
    s.animations_mut().bind_along_path(clamped, &road(), constant(1.1), PathTravelOptions::default()).unwrap();
    s.animations_mut().bind_along_path(extended, &road(), constant(1.1), PathTravelOptions { extend_beyond_ends: true, ..PathTravelOptions::default() }).unwrap();
    //Act
    s.set_time(0.0);
    //Assert
    assert_eq!(translation(&s, clamped), PointF::new(200.0, 50.0));
    assert!((translation(&s, extended).x - 220.0).abs() <= 1e-3);
    // And before the start, backwards along the first direction.
    assert_eq!(road().get_point_at_with(-0.25, true).position, PointF::new(-50.0, 50.0));
    assert_eq!(road().get_point_at_distance_with(250.0, true).distance, 250.0);
}

#[test]
fn travel_options_reject_values_that_are_not_finite() {
    //Arrange
    let mut s = session();
    let shape = circle(&mut s, 5.0);
    //Act
    let roll = s.animations_mut().bind_along_path(shape, &road(), progress_track(), PathTravelOptions { roll_radius: f32::NAN, ..PathTravelOptions::default() });
    let zero = s.animations_mut().bind_roll_along_path(shape, &road(), progress_track(), 0.0, None, false);
    //Assert
    assert!(roll.is_err());
    assert!(zero.is_err());
}

// ---- TRAINS (PathTravelTests) ----

#[test]
fn a_train_keeps_its_spacing_behind_the_lead() {
    //Arrange - three markers 30 apart, the first leading
    let mut s = session();
    let cars: Vec<ElementId> = (0..3).map(|_| circle(&mut s, 4.0)).collect();
    let train = s.animations_mut().bind_train_along_path(&cars, &road(), constant(0.5), PathTrainOptions { spacing: 30.0, ..PathTrainOptions::default() }).unwrap();
    //Act
    s.set_time(0.0);
    //Assert
    assert_eq!(translation(&s, cars[0]), PointF::new(100.0, 50.0));
    assert_eq!(translation(&s, cars[1]), PointF::new(70.0, 50.0));
    assert_eq!(translation(&s, cars[2]), PointF::new(40.0, 50.0));
    assert_eq!(train.get_distance(2, 0.0).unwrap(), 40.0);
    assert_eq!(train.cars()[2].rank(), 2);
    assert_eq!(train.get_point(1, 0.0).unwrap().position, PointF::new(70.0, 50.0));
}

#[test]
fn a_trains_gaps_close_with_the_spacing_scale_and_followers_lag_by_the_stagger() {
    //Arrange - reversed: the last element leads
    let mut s = session();
    let cars: Vec<ElementId> = (0..2).map(|_| circle(&mut s, 4.0)).collect();
    let progress = FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (2.0, 1.0, EasingKind::Linear)]).unwrap();
    let scale = FloatTrack::from_keys(&[(0.0, 2.0, EasingKind::Linear), (2.0, 0.0, EasingKind::Linear)]).unwrap();
    s.animations_mut().bind_train_along_path(&cars, &road(), progress, PathTrainOptions { spacing: 20.0, spacing_scale: Some(scale), stagger: 0.5, order: StaggerOrder::Reverse, ..PathTrainOptions::default() }).unwrap();
    //Act
    s.set_time(1.0);
    //Assert - lead (index 1) at 100; the follower follows the track at 0.5 s (50) less 20 x 1
    assert_eq!(translation(&s, cars[1]), PointF::new(100.0, 50.0));
    assert_eq!(translation(&s, cars[0]), PointF::new(30.0, 50.0));
}

#[test]
fn a_train_scales_and_fades_each_element_by_its_own_progress() {
    //Arrange
    let mut s = session();
    let cars: Vec<ElementId> = (0..2).map(|_| circle(&mut s, 4.0)).collect();
    let scale = ramp(1.0, 3.0);
    let opacity = FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (0.5, 1.0, EasingKind::Linear)]).unwrap();
    s.animations_mut().bind_train_along_path(&cars, &road(), constant(1.0), PathTrainOptions { spacing: 100.0, scale_along_path: Some(scale), opacity_along_path: Some(opacity), ..PathTrainOptions::default() }).unwrap();
    //Act
    s.set_time(0.0);
    //Assert
    assert_eq!(s.find_element(cars[0]).unwrap().transform().scale_x(), 3.0);
    assert_eq!(s.find_element(cars[1]).unwrap().transform().scale_x(), 2.0);
    assert_eq!(s.find_element(cars[0]).unwrap().opacity(), 1.0);
    assert_eq!(s.find_element(cars[1]).unwrap().opacity(), 1.0);
}

#[test]
fn train_elements_roll_upright_where_they_rest() {
    //Arrange
    let mut s = session();
    let cars: Vec<ElementId> = (0..2).map(|_| circle(&mut s, 10.0)).collect();
    s.animations_mut().bind_train_along_path(&cars, &road(), ramp(0.0, 1.0), PathTrainOptions { spacing: 40.0, roll_radius: 10.0, orient_to_path: true, ..PathTrainOptions::default() }).unwrap();
    //Act
    s.set_time(1.0);
    //Assert
    assert_eq!(rotation(&s, cars[0]), 0.0);
    assert_eq!(rotation(&s, cars[1]), 0.0);
    assert_eq!(translation(&s, cars[1]), PointF::new(160.0, 50.0));
}

#[test]
fn a_train_that_settles_in_place_ends_every_car_untransformed_and_changes_a_car_later() {
    //Arrange - two dots drawn where they should end; the route ends at (100, 0)
    let mut s = session();
    let a = s.draw_shape(CircleShape::new(300.0, 300.0, 4.0, ShapeOptions::default()).unwrap());
    let b = s.draw_shape(CircleShape::new(340.0, 300.0, 4.0, ShapeOptions::default()).unwrap());
    let route = line((0.0, 0.0), (100.0, 0.0));
    let train = s.animations_mut().bind_train_along_path(&[a, b], &route, ramp(0.0, 1.0), PathTrainOptions { settle_in_place: true, ..PathTrainOptions::default() }).unwrap();
    //Act
    s.set_time(1.0);
    let rest = (translation(&s, a), translation(&s, b));
    s.set_time(0.5);
    let flight = translation(&s, b);
    let mut car = train.cars()[1];
    car.set_delay(0.5).unwrap();
    train.set_car(1, car).unwrap();
    s.set_time(1.0);
    //Assert - at rest exactly 0; halfway moved back by half the route; a later delay holds it back
    assert_eq!(rest, (PointF::new(0.0, 0.0), PointF::new(0.0, 0.0)));
    assert_eq!(flight, PointF::new(-50.0, 0.0));
    assert_eq!(translation(&s, b), PointF::new(-50.0, 0.0));
    assert!(car.set_gap(-1.0).is_err());
    assert!(train.set_car(9, car).is_err());
}

#[test]
fn a_train_of_an_element_not_on_the_session_is_refused() {
    let mut s = session();
    let lonely = DrawingShape::from(CircleShape::new(0.0, 0.0, 1.0, ShapeOptions::default()).unwrap()).id();
    assert!(s.animations_mut().bind_train_along_path(&[lonely], &road(), constant(0.5), PathTrainOptions::default()).is_err());
    assert!(s.animations_mut().bind_train_along_path(&[], &road(), constant(0.5), PathTrainOptions { spacing: -1.0, ..PathTrainOptions::default() }).is_err());
}

// ---- FORMATION BINDING (FormationTests' BindFormation part) ----

#[test]
fn bind_formation_shows_each_rest_from_its_arrival_and_poses_travellers_in_between() {
    //Arrange - two members: one travels 0..2 to (100, 0), one appears at 1
    let mut s = session();
    s.add_layer("travel", Color::WHITE);
    let path: std::rc::Rc<dyn IArrivalPath> = std::rc::Rc::new(StraightArrival::new(FormationPose::at(0.0, 0.0), FormationPose::at(100.0, 0.0), 0.0, 2.0, Some(EasingKind::Linear)).unwrap());
    let members = vec![FormationMember::with_path(path), FormationMember::new(1.0).unwrap()];
    let dot = || DrawingShape::from(CircleShape::new(0.0, 0.0, 3.0, ShapeOptions::default()).unwrap());
    let mut travelling = |_: usize| vec![dot()];
    //Act
    let f = s.animations_mut().bind_formation(&members, "A", "travel", |k| dot().with_transform(DrawingTransform::translate(10.0 * k as f32, 0.0)), Some(&mut travelling)).unwrap();
    s.set_time(1.0);
    //Assert - arrival order is by time (the appearing member first), resting from the arrival
    assert_eq!(f.arrival_order(), &[1, 0]);
    let rest0 = s.find_element(f.get_resting(0).unwrap()).unwrap();
    assert_eq!((rest0.start_time(), rest0.is_visible_at(1.0), rest0.is_visible_at(2.0)), (Some(2.0), false, true));
    let traveller = f.get_travelling(0).unwrap()[0];
    let t = s.find_element(traveller).unwrap();
    assert_eq!((t.start_time(), t.end_time()), (Some(0.0), Some(2.0)));
    assert_eq!(t.transform().translation(), PointF::new(50.0, 0.0));
    assert_eq!(f.get_travelling(1).unwrap().len(), 0);
    assert_eq!((f.get_travelling_count(1.0), f.get_travelling_count(2.0)), (1, 0));
    assert!(s.animations_mut().bind_formation(&members, "nope", "travel", |_| dot(), None).is_err());
}

// ---- LAYER VISIBILITY (DrawingLayerVisibilityTests: the model side) ----

#[test]
fn a_bool_track_bound_to_a_layer_shows_and_hides_it_exactly_at_its_keys() {
    let mut s = session();
    let mut track = BoolTrack::new();
    track.add_key(Keyframe::new(0.0, true, EasingKind::Linear).unwrap());
    track.add_key(Keyframe::new(1.0, false, EasingKind::Linear).unwrap());
    track.add_key(Keyframe::new(2.0, true, EasingKind::Linear).unwrap());
    s.animations_mut().bind(BindingTarget::layer("A"), AnimatedProperty::IsVisible, track).unwrap();
    let mut seen = Vec::new();
    for t in [0.5, 0.999, 1.0, 1.5, 2.0] {
        s.set_time(t);
        seen.push(s.get_layer("A").unwrap().is_visible());
    }
    assert_eq!(seen, vec![true, true, false, false, true]);
    assert!(s.animations_mut().bind(BindingTarget::layer("A"), AnimatedProperty::Opacity, BoolTrack::new()).is_err(), "a bool track drives IsVisible only");
}
