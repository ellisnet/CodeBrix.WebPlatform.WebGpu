//! Host tests: the drawing half - a Lottie element in a session, its draw list mapped command by
//! command onto the renderer's scene, the element's transform and opacity, image pixels supplied
//! per asset id, the time, and markers as the clock's named points.

use std::sync::Arc;

use kurbo::{Affine, Point};
use peniko::Brush;

use crate::color::Color;
use crate::geometry::RectangleF;
use crate::images::ImageSource;
use crate::lottie::lottie_element::encode_list;
use crate::lottie::test_support::*;
use crate::lottie::*;
use crate::models::DrawingTransform;
use crate::rendering::VectorScene;
use crate::shapes::{DrawingShape, ShapeKind};
use crate::{DrawingSession, DrawingSessionOptions};

const BASICS: &str = include_str!("test_files/basics.json");
const MASKS_MATTES: &str = include_str!("test_files/masks_mattes.json");
const SOLID_IMAGE: &str = include_str!("test_files/solid_image.json");

/// The letters `VectorScene::item_kinds` gives for a draw list mapped one command to one call.
fn kinds_of(list: &LottieDrawList) -> String {
    list.commands
        .iter()
        .map(|c| match c {
            LottieDrawCommand::PushGroup { .. } => 'L',
            LottieDrawCommand::PushClip { .. } => 'C',
            LottieDrawCommand::Fill { .. } | LottieDrawCommand::Image { .. } => 'F',
            LottieDrawCommand::Stroke { .. } => 'S',
            LottieDrawCommand::Pop => 'P',
        })
        .collect()
}

/// A session of 200 x 200 calibrated units with one layer, rendered at 200 x 200 pixels.
fn session() -> DrawingSession {
    let mut session = DrawingSession::new(DrawingSessionOptions { calibration_size: (200, 200), ..DrawingSessionOptions::default() });
    session.add_layer("Lottie", Color::WHITE);
    session
}

fn render(session: &DrawingSession) -> VectorScene {
    let mut scene = VectorScene::new();
    session.render(&mut scene, 200, 200);
    scene
}

/// The brushes of a scene's fills and strokes, in drawing order.
fn brushes(scene: &VectorScene) -> Vec<Brush> {
    scene.brushes.clone()
}

/// The transforms of a list's draws.
fn transforms(list: &LottieDrawList) -> Vec<Affine> {
    draws(list).into_iter().map(transform_of).collect()
}

#[test]
fn every_command_becomes_one_renderer_call_in_order() {
    //Arrange
    let lottie = load(MASKS_MATTES);
    let list = lottie.draw_list(12.0, Affine::IDENTITY, 1.0, true);
    let mut scene = VectorScene::new();
    //Act
    encode_list(&mut scene, &list, &Default::default());
    //Assert
    assert!(list.group_count() >= 2 && list.clip_count() >= 2, "{:?}", list.commands.len());
    assert_eq!(scene.item_kinds(), kinds_of(&list));
}

#[test]
fn draw_lottie_puts_an_element_on_the_active_layer_that_draws_its_list_at_the_session_time() {
    //Arrange
    let lottie = Arc::new(load(BASICS));
    let mut session = session();
    //Act
    let id = session.draw_lottie(lottie.clone(), RectangleF::new(0.0, 0.0, 200.0, 200.0), DrawingLottieAspect::Meet, LottiePlayOptions::default()).unwrap();
    session.set_time(0.5);
    let scene = render(&session);
    //Assert - the same commands as the shape's own list at 0.5 s (frame 5), under the
    // calibration transform (1:1 here)
    let element = session.find_element(id).unwrap();
    assert!(matches!(element.kind(), ShapeKind::Lottie(_)));
    let mut shape = element.as_lottie().unwrap().clone();
    let list = shape.draw_list(0.5);
    assert!(scene.item_kinds().contains(&kinds_of(&list)), "{} / {}", scene.item_kinds(), kinds_of(&list));
    assert_eq!(scene.draw_count(), list.draw_count());
}

#[test]
fn the_element_transform_prefixes_every_draw_and_its_opacity_multiplies_the_paint() {
    //Arrange - basics.json at frame 5: the rectangle's fill at 0.5 (layer opacity 50 %)
    let lottie = Arc::new(load(BASICS));
    let mut session = session();
    let id = session.draw_lottie(lottie, RectangleF::new(0.0, 0.0, 100.0, 100.0), DrawingLottieAspect::None, LottiePlayOptions::default()).unwrap();
    session.set_time(0.5);
    let plain = brushes(&render(&session));
    let shape = session.find_element(id).unwrap().as_lottie().unwrap().clone();
    //Act
    let element = session.find_element_mut(id).unwrap();
    element.set_transform(DrawingTransform::translate(30.0, 40.0));
    element.set_opacity(0.5);
    let faded = brushes(&render(&session));
    let moved = shape.list_at(5.0, Affine::translate((30.0, 40.0)), 0.5);
    //Assert - the encoding's list: every draw moved by the element transform
    let still = shape.list_at(5.0, Affine::IDENTITY, 1.0);
    assert!(!plain.is_empty());
    assert_eq!(plain.len(), faded.len());
    for (t0, t1) in transforms(&still).iter().zip(&transforms(&moved)) {
        let (p0, p1) = (*t0 * Point::new(1.0, 2.0), *t1 * Point::new(1.0, 2.0));
        assert!(near(p1.x - p0.x, 30.0, 1e-9) && near(p1.y - p0.y, 40.0, 1e-9), "{p0:?} {p1:?}");
    }
    // and every solid paint at half the alpha
    for (b0, b1) in plain.iter().zip(&faded) {
        if let (Brush::Solid(c0), Brush::Solid(c1)) = (b0, b1) {
            assert!(near(c1.components[3] as f64, c0.components[3] as f64 * 0.5, 1e-6), "{c0:?} {c1:?}");
        }
    }
}

#[test]
fn image_layers_draw_the_pixels_the_application_hands_over_and_nothing_without_them() {
    //Arrange - solid_image.json: a 64 x 32 box at (10, 20) at 40 %, over a green solid layer
    let lottie = Arc::new(load(SOLID_IMAGE));
    let mut session = session();
    let id = session.draw_lottie(lottie, RectangleF::new(0.0, 0.0, 120.0, 80.0), DrawingLottieAspect::None, LottiePlayOptions::default()).unwrap();
    let without = brushes(&render(&session));
    //Act
    let picture = ImageSource::from_rgba(&[255u8; 128 * 64 * 4], 128, 64).unwrap();
    session.find_element_mut(id).and_then(|e| e.as_lottie_mut()).unwrap().set_image("picture", picture);
    let with = brushes(&render(&session));
    let list = session.find_element(id).unwrap().as_lottie().unwrap().list_at(0.0, Affine::IDENTITY, 1.0);
    //Assert - one more fill, an image brush at 40 %, the 128 x 64 picture fitted into the 64 x 32 box
    assert_eq!(with.len(), without.len() + 1);
    assert!(without.iter().all(|b| !matches!(b, Brush::Image(_))));
    let Some(Brush::Image(image)) = with.iter().find(|b| matches!(b, Brush::Image(_))) else { panic!("an image brush") };
    assert!(near(image.sampler.alpha as f64, 0.4, 1e-6));
    let command = list.commands.iter().find(|c| matches!(c, LottieDrawCommand::Image { .. })).unwrap();
    let t = command.image_transform(128, 64).unwrap();
    assert!(at(t * Point::ORIGIN, 10.0, 20.0) && at(t * Point::new(128.0, 64.0), 74.0, 52.0), "{t:?}");
}

#[test]
fn draw_lottie_refuses_an_unknown_marker() {
    //Arrange
    let lottie = Arc::new(load(BASICS));
    let mut session = session();
    //Act
    let result = session.draw_lottie(lottie, RectangleF::new(0.0, 0.0, 100.0, 100.0), DrawingLottieAspect::Meet, LottiePlayOptions { range: LottieRange::Marker("nowhere".to_string()), ..LottiePlayOptions::default() });
    //Assert
    assert!(result.unwrap_err().contains("nowhere"));
}

#[test]
fn bounds_are_the_fitted_box_and_clones_compare_equal() {
    //Arrange
    let lottie = Arc::new(load(BASICS));
    let shape = DrawingLottieShape::new(lottie, RectangleF::new(0.0, 0.0, 200.0, 100.0), DrawingLottieAspect::Meet, LottiePlayOptions::default()).unwrap();
    //Act
    let element = DrawingShape::from(shape.clone());
    //Assert
    assert_eq!(element.get_bounds(), Some(RectangleF::new(50.0, 0.0, 100.0, 100.0)));
    assert_eq!(element.as_lottie(), Some(&shape));
}

#[test]
fn a_held_animation_draws_the_same_list_after_its_end() {
    //Arrange
    let lottie = Arc::new(load(BASICS));
    let mut session = session();
    session.draw_lottie(lottie, RectangleF::new(0.0, 0.0, 100.0, 100.0), DrawingLottieAspect::Meet, LottiePlayOptions { playback: LottiePlayback::Hold, ..LottiePlayOptions::default() }).unwrap();
    //Act
    session.set_time(3.0);
    let a = render(&session);
    session.set_time(9.0);
    let b = render(&session);
    //Assert - the same items, geometry and paint
    assert_eq!(a.item_kinds(), b.item_kinds());
    assert_eq!(a.fill_keys(), b.fill_keys());
    assert_eq!(brushes(&a), brushes(&b));
}

#[test]
fn markers_become_the_clocks_named_points_in_time() {
    //Arrange - basics.json's markers intro (frame 0), "second half" (10), flash (15) at 10 fps,
    // playing from session time 2 at speed 2
    let lottie = Arc::new(load(BASICS));
    let shape = DrawingLottieShape::new(lottie, RectangleF::new(0.0, 0.0, 100.0, 100.0), DrawingLottieAspect::Meet, LottiePlayOptions { begin_time: 2.0, speed: 2.0, ..LottiePlayOptions::default() }).unwrap();
    let clock = codebrix_webgpu::CanvasClock::new();
    //Act
    for (name, time) in shape.marker_times() {
        clock.define_point(&name, time);
    }
    //Assert
    let points = clock.points();
    assert_eq!(points.time("intro"), Some(2.0));
    assert_eq!(points.time("second half"), Some(2.5));
    assert_eq!(points.time("flash"), Some(2.75));
    assert!(points.reached("second half", 2.6) && !points.reached("flash", 2.6));
}

#[test]
fn the_samples_feature_file_draws_parenting_a_trim_path_a_gradient_and_a_matte_with_nothing_reported() {
    //Arrange - D_features.json (the GraphicsDemo sample's own file): a null layer turning an arm
    // that carries a star (parenting), a ring drawn on by a trim path, a gradient panel seen through
    // a moving disc (an alpha matte), and three markers
    let Some(json) = trial_file("D_features.json") else { return };
    let lottie = load(&json);
    //Act
    let at_0 = lottie.draw_list(0.0, Affine::IDENTITY, 1.0, true);
    let at_30 = lottie.draw_list(30.0, Affine::IDENTITY, 1.0, true);
    //Assert
    assert!(lottie.report().is_empty(), "{:?}", lottie.report());
    assert_eq!(lottie.markers().iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), vec!["build", "wave", "settle"]);
    // the ring's stroke is trimmed to nothing at frame 0 and drawn at frame 30
    assert_eq!(at_30.draw_count(), at_0.draw_count() + 1);
    // the matte's two groups (Normal, SourceIn) and a gradient paint inside them
    let blends: Vec<LottieBlendMode> = at_30.commands.iter().filter_map(|c| if let LottieDrawCommand::PushGroup { blend, .. } = c { Some(*blend) } else { None }).collect();
    assert!(blends.contains(&LottieBlendMode::SourceIn), "{blends:?}");
    assert!(at_30.commands.iter().any(|c| matches!(c, LottieDrawCommand::Fill { paint: LottiePaint::Linear(_), .. })));
    // the star (the null's grandchild) at frame 30: root turned 45 degrees, so the hand at 110 along it
    let star = draws(&at_30).into_iter().find(|c| matches!(c, LottieDrawCommand::Fill { paint: LottiePaint::Solid(color), .. } if *color == Color::from_rgb(255, 89, 153))).unwrap();
    let p = transform_of(star) * Point::ORIGIN;
    let a = 45f64.to_radians();
    assert!(near(p.x, 200.0 + 110.0 * a.cos(), 1e-6) && near(p.y, 150.0 + 110.0 * a.sin(), 1e-6), "{p:?}");
}

#[test]
fn a_session_puts_a_drawn_documents_warnings_into_its_reports_once_when_asked() {
    //Arrange
    let lottie = Arc::new(load(include_str!("test_files/not_drawn.json")));
    assert!(!lottie.report().is_empty());
    let mut off = session();
    off.draw_lottie(lottie.clone(), RectangleF::new(0.0, 0.0, 100.0, 100.0), DrawingLottieAspect::Meet, LottiePlayOptions::default()).unwrap();
    let mut on = session();
    on.set_reports_document_warnings(true);
    on.draw_lottie(lottie.clone(), RectangleF::new(0.0, 0.0, 100.0, 100.0), DrawingLottieAspect::Meet, LottiePlayOptions::default()).unwrap();
    on.draw_lottie(lottie.clone(), RectangleF::new(100.0, 0.0, 100.0, 100.0), DrawingLottieAspect::Meet, LottiePlayOptions::default()).unwrap();
    //Act
    render(&off);
    let off_reports = off.take_reports();
    render(&on);
    let first = on.take_reports();
    render(&on);
    let second = on.take_reports();
    //Assert
    assert!(!off.reports_document_warnings() && on.reports_document_warnings());
    assert!(off_reports.iter().all(|l| !l.starts_with("Lottie document: ")), "{off_reports:?}");
    let expected: Vec<String> = lottie.report().iter().map(|l| format!("Lottie document: {l}")).collect();
    let documents: Vec<String> = first.into_iter().filter(|l| l.starts_with("Lottie document: ")).collect();
    assert_eq!(documents, expected, "each line once, though the document is drawn twice");
    assert!(second.is_empty(), "{second:?}");
}

#[test]
fn an_application_adds_its_own_messages_to_the_session_reports_once_each() {
    //Arrange
    let s = session();
    //Act
    s.add_report("The score could not be read: drawn without it.");
    s.add_report("The score could not be read: drawn without it.".to_string());
    let first = s.take_reports();
    s.add_report("The score could not be read: drawn without it.");
    //Assert
    assert_eq!(first, vec!["The score could not be read: drawn without it.".to_string()]);
    assert!(s.take_reports().is_empty(), "a message is reported once in the session's life");
}
