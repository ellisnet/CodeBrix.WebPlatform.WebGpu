//! Host tests of the DRAWING half of SVG documents: what an SVG element records into the
//! renderer's scene (groups, the union of a several-shape clip, masks, gradients, the blur,
//! supplied pictures), SVG text through usvg with a supplied font, and the session API. Fonts
//! as the text tests (CODEBRIX_ANIMATION_TEST_FONTS, or the test project's folder).

use std::path::PathBuf;
use std::sync::Arc;

use crate::animation::{EasingKind, FloatTrack};
use crate::color::Color;
use crate::drawing_session::DrawingSession;
use crate::drawing_session_options::DrawingSessionOptions;
use crate::geometry::RectangleF;
use crate::images::ImageSource;
use crate::models::AnimatedProperty;
use crate::rendering::VectorScene;
use crate::svg::*;

fn session() -> DrawingSession {
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (400, 200), layer_opacity: 255, ..DrawingSessionOptions::default() });
    s.add_layer("Svg", Color::WHITE);
    s
}

fn kinds(s: &DrawingSession) -> String {
    let mut scene = VectorScene::new();
    s.render(&mut scene, 400, 200);
    scene.item_kinds()
}

fn load(markup: &str) -> Arc<DrawingSvg> {
    Arc::new(DrawingSvg::from_svg(markup, &DrawingSvgOptions::default()).expect("loads"))
}

fn roboto() -> Option<Vec<u8>> {
    let root = match std::env::var("CODEBRIX_ANIMATION_TEST_FONTS") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/Fonts"),
    };
    let path = root.join("Roboto").join("Roboto-Regular.ttf");
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(_) if std::env::var("CODEBRIX_ANIMATION_TEST_FONTS").is_ok() => panic!("CODEBRIX_ANIMATION_TEST_FONTS is set, but {} cannot be read", path.display()),
        Err(_) => {
            eprintln!("SKIPPED: no test font at {} (set CODEBRIX_ANIMATION_TEST_FONTS)", path.display());
            None
        }
    }
}

const DEST: RectangleF = RectangleF::new(0.0, 0.0, 400.0, 200.0);

#[test]
fn draw_svg_records_the_documents_paths_inside_the_layer() {
    //Arrange
    let mut s = session();
    let svg = load(r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect width="10" height="10" fill="#f00"/><circle cx="30" cy="10" r="5" fill="none" stroke="#00f" stroke-width="2"/></svg>"##);
    //Act
    let id = s.draw_svg(&svg, DEST, DrawingSvgAspect::Meet);
    //Assert
    assert_eq!(kinds(&s), "LFSP");
    assert!(s.get_svg(id).is_some());
    assert_eq!(s.find_element(id).unwrap().get_bounds(), Some(DEST));
}

#[test]
fn a_clip_of_several_shapes_is_their_union_and_one_shape_a_stencil_clip() {
    //Arrange
    let mut s = session();
    let one = load(r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><clipPath id="c"><rect width="10" height="10"/></clipPath><rect width="40" height="20" clip-path="url(#c)"/></svg>"##);
    let two = load(r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><clipPath id="c"><rect width="10" height="10"/><rect x="20" width="10" height="10"/></clipPath><rect width="40" height="20" clip-path="url(#c)"/></svg>"##);
    //Act
    s.draw_svg(&one, DEST, DrawingSvgAspect::Meet);
    let single = kinds(&s);
    let mut other = session();
    other.draw_svg(&two, DEST, DrawingSvgAspect::Meet);
    let union = kinds(&other);
    //Assert
    assert_eq!(single, "LCFPP");
    assert_eq!(union, "LLFLFFPPP", "content in a group, then both shapes drawn destination-in");
}

#[test]
fn masks_blend_modes_gradients_and_the_blur_become_groups_effects_and_gradient_fills() {
    //Arrange
    let mut s = session();
    let svg = load(r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20">
        <defs><linearGradient id="g"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient>
        <radialGradient id="r" fx="0.3"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#000"/></radialGradient>
        <mask id="m" mask-type="alpha"><rect width="20" height="20" fill="#fff"/></mask>
        <filter id="b"><feGaussianBlur stdDeviation="2"/></filter></defs>
        <rect width="20" height="20" fill="url(#g)" mask="url(#m)"/>
        <rect x="20" width="20" height="20" fill="url(#r)" style="mix-blend-mode:multiply"/>
        <circle cx="10" cy="10" r="5" filter="url(#b)"/></svg>"##);
    //Act
    s.draw_svg(&svg, DEST, DrawingSvgAspect::Meet);
    let k = kinds(&s);
    //Assert
    assert!(k.contains("LFLCFPP"), "masked: group, content, destination-in group clipped to the mask's region: {k}");
    assert!(k.contains("EFP"), "blurred: an effects group: {k}");
    assert!(svg.warnings().is_empty(), "{:?}", svg.report());
}

#[test]
fn a_picture_is_drawn_only_once_the_application_supplies_its_pixels() {
    //Arrange
    let mut s = session();
    let svg = load(r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><image href="pictures/cat.png" width="20" height="20"/></svg>"##);
    let mut shape = DrawingSvgShape::new(svg.clone(), DEST, DrawingSvgAspect::Meet);
    let requests = shape.image_requests();
    let mut later = session();
    //Act
    s.draw_svg_shape(shape.clone());
    let without = kinds(&s);
    let supplied = shape.set_image_href("pictures/cat.png", ImageSource::from_rgba(&[255u8; 16], 2, 2).unwrap());
    later.draw_svg_shape(shape);
    //Assert
    assert_eq!(requests, vec![SvgImageSource::Href("pictures/cat.png".to_string())]);
    assert!(supplied);
    assert_eq!(without, "LP");
    assert_eq!(kinds(&later), "LFP");
}

#[test]
fn part_bindings_follow_the_session_time_without_changing_the_shape() {
    //Arrange
    let mut s = session();
    let svg = load(r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect id="bar" width="10" height="10"/></svg>"##);
    let mut shape = DrawingSvgShape::new(svg, DEST, DrawingSvgAspect::Meet);
    shape.bind_part("bar", AnimatedProperty::Opacity, FloatTrack::from_keys(&[(0.0, 1.0, EasingKind::Linear), (1.0, 0.0, EasingKind::Linear)]).unwrap()).unwrap();
    let id = s.draw_svg_shape(shape);
    //Act
    s.set_time(0.0);
    let start = kinds(&s);
    s.set_time(2.0);
    let end = kinds(&s);
    //Assert
    assert_eq!(start, "LFP");
    assert_eq!(end, "LP", "opacity 0 at 1 s and after");
    assert_eq!(s.get_svg(id).unwrap().get_part("bar").unwrap().opacity, 1.0, "the element's own parts are not changed by drawing");
}

#[test]
fn svg_text_is_laid_out_into_paths_with_a_supplied_font_and_reported_without_one() {
    //Arrange
    let Some(font) = roboto() else { return };
    let markup = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="40"><text id="label" x="10" y="30" font-family="Roboto" font-size="20">PER-PART</text><text x="10" y="10" font-family="Garamond" font-size="8">x</text></svg>"##;
    //Act
    let none = DrawingSvg::from_svg(markup, &DrawingSvgOptions::default()).unwrap();
    let with = DrawingSvg::from_svg(markup, &DrawingSvgOptions::with_font(Arc::new(font))).unwrap();
    let mut shape = DrawingSvgShape::new(Arc::new(with.clone()), RectangleF::new(0.0, 0.0, 200.0, 40.0), DrawingSvgAspect::None);
    let list = shape.draw_list(0.0);
    //Assert
    assert!(none.warnings().iter().any(|w| w.kind == DrawingSvgWarningKind::NoFontsRegistered));
    assert!(!with.warnings().iter().any(|w| w.kind == DrawingSvgWarningKind::NoFontsRegistered));
    assert!(with.report().iter().any(|l| l.starts_with("FontSubstituted: ") && l.contains("'Garamond'") && l.contains("'Roboto'")), "{:?}", with.report());
    assert!(with.has_part("label"), "the text element's id is a part");
    assert!(list.draw_count() >= 2, "the glyph outlines are drawn as paths");
    let bounds = shape.get_part_bounds("label", 0.0).unwrap();
    assert!(bounds.x >= 9.0 && bounds.width > 60.0 && bounds.height > 10.0, "{bounds:?}");
}

#[test]
fn a_session_asked_to_reports_a_drawn_svgs_warnings_once_with_a_prefix() {
    //Arrange
    let svg = Arc::new(DrawingSvg::from_svg(include_str!("test_files/parts.svg"), &DrawingSvgOptions::default()).unwrap());
    let mut s = session();
    s.set_reports_document_warnings(true);
    s.draw_svg(&svg, DEST, DrawingSvgAspect::Meet);
    //Act
    kinds(&s);
    let first = s.take_reports();
    kinds(&s);
    let second = s.take_reports();
    //Assert
    let expected: Vec<String> = svg.report().iter().map(|l| format!("SVG document: {l}")).collect();
    assert!(!expected.is_empty());
    assert_eq!(first.into_iter().filter(|l| l.starts_with("SVG document: ")).collect::<Vec<_>>(), expected);
    assert!(second.is_empty(), "{second:?}");
}
