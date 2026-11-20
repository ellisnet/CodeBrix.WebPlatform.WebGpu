//! Host tests of the renderer's pixel-plane and painterly drawing: noise as a paint evaluated on the GPU,
//! particles as ONE instanced operation, a layer's whole-frame operations and transitions riding on
//! the effect composite, slides and pushes as whole-pixel shifts, brushstrokes as fills.

use kurbo::Affine;

use crate::effects::effect_sizes::frame_params;
use crate::effects::{DrawingFrameOperations, FrameVignette};
use crate::planes::{DrawingNoiseKind, DrawingParticleEmitter, DrawingParticleSystem, DrawingTransition, DrawingTransitionDirection};
use crate::rendering::core::build::{Builder, Op};
use crate::rendering::core::paint::KIND_NOISE;
use crate::rendering::VectorScene;
use crate::shapes::{BrushStrokeShape, BrushStrokeShapeOptions, NoiseShape, NoiseShapeOptions, ParticlesShape};
use crate::{Color, DrawingSession};

fn session_with(shape: impl Into<crate::shapes::DrawingShape>) -> VectorScene {
    let mut s = DrawingSession::default();
    s.add_layer("A", Color::WHITE).add_shape(shape);
    let mut scene = VectorScene::new();
    s.render(&mut scene, 640, 360);
    scene
}

#[test]
fn a_noise_shape_is_one_fill_with_the_noise_paint() {
    //Arrange
    let noise = NoiseShape::new(10.0, 10.0, 200.0, 100.0, DrawingNoiseKind::Gradient, 42, 0.02, NoiseShapeOptions::default()).unwrap().with_z(1.5);
    let scene = session_with(noise);
    let mut b = Builder::default();
    //Act
    b.build(&scene, 640, 360);
    //Assert
    assert!(scene.item_kinds().contains('N'));
    let p = b.paints.iter().find(|p| p.info[0] == KIND_NOISE).expect("a noise paint");
    assert_eq!(p.info[1] & 3, 3, "gradient and 3D");
    assert_eq!(p.info[1] >> 8, 1, "one octave");
    assert_eq!(p.color[1], 1.5);
    assert!(NoiseShape::new(0.0, 0.0, 0.0, 10.0, DrawingNoiseKind::Value, 1, 0.1, NoiseShapeOptions::default()).is_err());
}

#[test]
fn particles_are_one_instanced_operation() {
    //Arrange
    let mut system = DrawingParticleSystem::new(3);
    let e = system.add_emitter(DrawingParticleEmitter::new());
    e.set_x(100.0);
    e.set_y(100.0);
    e.set_burst(50);
    e.set_lifetime(5.0).unwrap();
    e.set_start_size(8.0);
    e.set_end_size(8.0);
    let shape = ParticlesShape::new(&system, 0.5, Some(0.5)).unwrap();
    let count = shape.count();
    let scene = session_with(shape);
    let mut b = Builder::default();
    //Act
    b.build(&scene, 640, 360);
    //Assert
    assert!(count > 0);
    let ops: Vec<&Op> = b.ops.iter().filter(|o| matches!(o, Op::Particles { .. })).collect();
    assert_eq!(ops.len(), 1);
    assert_eq!(b.particles.len(), count);
    assert!(b.particles.iter().all(|p| p.softness == 0.5 && p.color[3] > 0.0));
}

#[test]
fn frame_operations_and_transitions_become_the_composites_parameters() {
    //Arrange
    let ops = DrawingFrameOperations::NONE.with_tone_curve(0.15, 1.08).with_gamma(2.0).unwrap().with_bloom(4.0, 0.5).unwrap().with_vignette(FrameVignette { center_x: 50.0, center_y: 50.0, radius_x: 10.0, radius_y: 20.0, strength: 0.4, minimum: 0.2 }).unwrap();
    let wipe = DrawingTransition::wipe(DrawingTransitionDirection::Down, Some(10.0)).unwrap();
    //Act
    let f = frame_params(Some(&ops), Some((&wipe, 0.25)), Affine::scale(2.0), [0.0, 0.0, 200.0, 100.0]).unwrap();
    let slide = frame_params(None, Some((&DrawingTransition::slide(DrawingTransitionDirection::Left), 0.5)), Affine::IDENTITY, [0.0; 4]);
    //Assert
    assert_eq!(f.flags & 15, 15);
    assert_eq!((f.flags >> 8) & 15, 2, "a wipe");
    assert_eq!((f.flags >> 12) & 3, 3, "down");
    assert_eq!(f.vignette, [100.0, 100.0, 20.0, 40.0]);
    assert_eq!(f.bloom_sigma, 8.0);
    assert_eq!(f.transition, [0.25, 20.0]);
    assert!(slide.is_none(), "a slide is a transform, not a pass");
    assert!(DrawingFrameOperations::NONE.with_gamma(0.0).is_err());
}

#[test]
fn a_dissolve_draws_the_incoming_layer_through_an_effect_group_and_a_push_shifts_both() {
    //Arrange
    let mut s = DrawingSession::default();
    s.add_layer("A", Color::WHITE).add_shape(crate::shapes::CircleShape::new(100.0, 100.0, 40.0, crate::shapes::ShapeOptions { is_filled: true, ..Default::default() }).unwrap());
    s.add_layer("B", Color::WHITE).add_shape(crate::shapes::CircleShape::new(200.0, 100.0, 40.0, crate::shapes::ShapeOptions { is_filled: true, ..Default::default() }).unwrap());
    let mut scene = VectorScene::new();
    //Act
    s.set_layer_transition("A", "B", DrawingTransition::dissolve(), 0.5).unwrap();
    s.render(&mut scene, 640, 360);
    let dissolve = scene.item_kinds();
    scene.reset();
    s.set_layer_transition("A", "B", DrawingTransition::push(DrawingTransitionDirection::Right), 0.5).unwrap();
    s.render(&mut scene, 640, 360);
    let push = scene.item_kinds();
    //Assert
    assert_eq!(dissolve, "LFPEFP");
    assert_eq!(push, "CLFPPCLFPP", "each moved layer inside a clip of the frame");
    assert!(s.set_layer_transition("A", "None", DrawingTransition::dissolve(), 0.5).is_err());
    assert!(s.set_layer_transition("A", "B", DrawingTransition::dissolve(), f32::NAN).is_err());
}

#[test]
fn a_brushstroke_painting_on_is_a_fill_of_its_painted_part() {
    //Arrange
    let stroke = BrushStrokeShape::new(100.0, 100.0, 120.0, 30.0, 20.0, BrushStrokeShapeOptions { color: Some(Color::from_rgb(200, 40, 40)), ..Default::default() }).unwrap();
    //Act
    let whole = session_with(stroke.clone());
    let half = session_with(stroke.clone().with_progress(0.5));
    //Assert
    assert_eq!(whole.item_kinds(), "LFP");
    assert_eq!(half.item_kinds(), "LFP");
    assert!(stroke.clone().with_progress(0.5).get_bounds().width < stroke.get_bounds().width);
}

#[test]
fn an_ownership_shape_uploads_owner_numbers_and_colours_and_is_coloured_by_its_paint() {
    //Arrange
    let mut s = DrawingSession::new(crate::DrawingSessionOptions { calibration_size: (8, 4), ..crate::DrawingSessionOptions::default() });
    s.add_layer("Paint", Color::WHITE).add_shape(crate::shapes::RectangleShape::new(0.0, 0.0, 4.0, 4.0, crate::shapes::RectangleShapeOptions { is_filled: true, ..Default::default() }).unwrap());
    let map = s.export_layer_ownership_map("Paint", (8, 4)).unwrap();
    let colors = [Color::from_rgb(10, 20, 30)];
    //Act
    let shape = crate::shapes::OwnershipShape::new(&map, &colors, crate::RectangleF::new(0.0, 0.0, 80.0, 40.0), crate::shapes::OwnershipShapeOptions { seam_color: Some(Color::BLACK), ..Default::default() }).unwrap();
    let pixels = shape.picture.to_rgba();
    let scene = session_with(shape.clone());
    let mut b = Builder::default();
    b.build(&scene, 640, 360);
    //Assert: owner 0 -> number 1 on the left half, nobody (0) on the right; the colour row below
    assert_eq!(shape.picture.size(), (8, 5));
    assert_eq!(&pixels[0..4], &[1, 0, 0, 255]);
    assert_eq!(&pixels[7 * 4..8 * 4], &[0, 0, 0, 0]);
    assert_eq!(&pixels[32 * 4..33 * 4], &[10, 20, 30, 255]);
    assert!(b.paints.iter().any(|p| p.info[0] == crate::rendering::core::paint::KIND_OWNERS && p.info[3] == 1 && p.color[0] == 8.0));
    assert!(crate::shapes::OwnershipShape::new(&map, &[], crate::RectangleF::new(0.0, 0.0, 8.0, 4.0), Default::default()).is_err());
}
