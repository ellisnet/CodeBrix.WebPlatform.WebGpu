//! Host tests of the pixel-plane additions to masks: plane masks (their checks, their 8-bit copy and
//! how they are recorded) and the report of an unknown matte layer name (once, not every frame).

use crate::effects::{DrawingMask, DrawingMaskKind};
use crate::planes::DrawingPlane;
use crate::rendering::VectorScene;
use crate::shapes::CircleShape;
use crate::{Color, DrawingSession, RectangleF};

#[test]
fn a_plane_mask_copies_the_values_as_8_bit_alpha_and_checks_its_rectangle() {
    //Arrange
    let plane = DrawingPlane::from_data(2, 1, vec![0.0, 1.0]).unwrap();
    //Act
    let mask = DrawingMask::from_plane(&plane, RectangleF::new(10.0, 20.0, 200.0, 100.0), false).unwrap();
    let origin = DrawingMask::from_plane_at_origin(&plane, true);
    let bad = DrawingMask::from_plane(&plane, RectangleF::new(0.0, 0.0, 0.0, 10.0), false).unwrap_err();
    let nan = DrawingMask::from_plane(&plane, RectangleF::new(f32::NAN, 0.0, 5.0, 10.0), false);
    //Assert
    assert_eq!(mask.kind(), DrawingMaskKind::Plane);
    let (picture, size) = mask.plane().unwrap();
    assert_eq!(size, (2, 1));
    assert_eq!(picture.to_rgba(), vec![255, 255, 255, 0, 255, 255, 255, 255]);
    assert_eq!(mask.destination(), RectangleF::new(10.0, 20.0, 200.0, 100.0));
    assert_eq!(origin.destination(), RectangleF::new(0.0, 0.0, 2.0, 1.0));
    assert!(origin.is_inverted());
    assert!(bad.contains("positive width"));
    assert!(nan.is_err());
}

#[test]
fn a_plane_mask_is_recorded_as_a_destination_in_picture_over_its_rectangle() {
    //Arrange
    let mut session = DrawingSession::default();
    let plane = DrawingPlane::with_value(4, 4, 0.5).unwrap();
    let layer = session.add_layer("Content", Color::WHITE);
    layer.add_shape(CircleShape::new(100.0, 100.0, 50.0, crate::shapes::ShapeOptions { is_filled: true, ..Default::default() }).unwrap());
    layer.set_mask(Some(DrawingMask::from_plane(&plane, RectangleF::new(0.0, 0.0, 200.0, 200.0), false).unwrap()));
    let mut scene = VectorScene::new();
    //Act
    session.render(&mut scene, 640, 360);
    //Assert: the layer's group, the circle, the destination-in group with the picture
    assert_eq!(scene.item_kinds(), "LFLFPP");
}

#[test]
fn an_unknown_matte_name_is_reported_once_and_covers_nothing() {
    //Arrange
    let mut session = DrawingSession::default();
    let layer = session.add_layer("Content", Color::WHITE);
    layer.add_shape(CircleShape::new(100.0, 100.0, 50.0, crate::shapes::ShapeOptions { is_filled: true, ..Default::default() }).unwrap());
    layer.set_mask(Some(DrawingMask::from_layer("Mate", false)));
    session.add_layer("Matte", Color::WHITE).add_shape(CircleShape::new(90.0, 90.0, 20.0, crate::shapes::ShapeOptions { is_filled: true, ..Default::default() }).unwrap());
    let mut scene = VectorScene::new();
    //Act
    session.render(&mut scene, 640, 360);
    let first = session.take_reports();
    scene.reset();
    session.render(&mut scene, 640, 360);
    let second = session.take_reports();
    //Assert
    assert_eq!(first.len(), 1);
    assert!(first[0].contains("\"Mate\"") && first[0].contains("from_layer"), "{}", first[0]);
    assert!(second.is_empty());
}
