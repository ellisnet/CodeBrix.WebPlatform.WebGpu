//! Host tests of the FIXED-width (non-scaling) stroke: the stroke style's setting, what a path,
//! an outlined text block and an SVG path with `vector-effect="non-scaling-stroke"` record, and
//! the width the recorded stroke has on the canvas whatever the element's scaling (expanded with
//! kurbo's stroker exactly as the renderer expands it). Not in C#.

use std::sync::Arc;

use kurbo::{Affine, Rect, Shape, StrokeOpts};

use crate::color::Color;
use crate::drawing_session::DrawingSession;
use crate::drawing_session_options::DrawingSessionOptions;
use crate::geometry::{PointF, RectangleF};
use crate::models::{AnimatedProperty, DrawingCamera, DrawingTransform};
use crate::paths::{PathGeometryBuilder, PathStrokeCap, PathStrokeStyle, PathStrokeStyleOptions, StrokeWidthMode};
use crate::rendering::SceneItem as Item;
use crate::rendering::VectorScene;
use crate::shapes::{PathShape, TextBlockShape, TextBlockStrokeOptions};
use crate::svg::{DrawingSvg, DrawingSvgAspect, DrawingSvgOptions};

/// A session whose drawing units are canvas pixels (100 x 100 drawn at 100 x 100).
fn session() -> DrawingSession {
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (100, 100), layer_opacity: 255, ..DrawingSessionOptions::default() });
    s.add_layer("A", Color::WHITE);
    s
}

/// A horizontal line from (10, 50) to (60, 50), butt caps, stroked in `mode`.
fn line(width: f32, mode: StrokeWidthMode) -> PathShape {
    let mut b = PathGeometryBuilder::new();
    b.move_to(10.0, 50.0).line_to(60.0, 50.0);
    let style = PathStrokeStyle::new(width, PathStrokeStyleOptions { cap: PathStrokeCap::Butt, width_mode: mode, ..PathStrokeStyleOptions::default() }).unwrap();
    PathShape::with_style(b.build(), style, Some(Color::BLACK), None, None)
}

/// The canvas-pixel bounds of every recorded stroke, expanded as the renderer expands it.
fn stroke_boxes(scene: &VectorScene) -> Vec<Rect> {
    scene
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Stroke { stroke, transform, path, .. } => {
                let els: Vec<kurbo::PathEl> = scene.elements[path.start as usize..path.end as usize].to_vec();
                let outline = kurbo::stroke(els, &scene.strokes[*stroke as usize], &StrokeOpts::default(), 1e-4);
                Some((*transform * outline).bounding_box())
            }
            _ => None,
        })
        .collect()
}

fn render(s: &DrawingSession, width: u32, height: u32) -> VectorScene {
    let mut scene = VectorScene::new();
    s.render(&mut scene, width, height);
    scene
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn a_stroke_style_scales_with_the_element_by_default_and_keeps_its_width_mode_through_its_copies() {
    //Arrange
    let plain = PathStrokeStyle::new(2.0, PathStrokeStyleOptions::default()).unwrap();
    //Act
    let fixed = plain.with_width_mode(StrokeWidthMode::Fixed);
    //Assert
    assert_eq!(PathStrokeStyleOptions::default().width_mode, StrokeWidthMode::ScalesWithElement);
    assert_eq!(StrokeWidthMode::default(), StrokeWidthMode::ScalesWithElement);
    assert!(!plain.is_fixed_width());
    assert!(fixed.is_fixed_width());
    assert_eq!(fixed.with_width(5.0).unwrap().width_mode(), StrokeWidthMode::Fixed);
    assert_eq!(fixed.with_dash_offset(3.0).width_mode(), StrokeWidthMode::Fixed);
    assert_eq!(fixed.width(), 2.0);
}

#[test]
fn a_fixed_width_stroke_keeps_its_width_under_an_uneven_element_scale_where_a_scaling_one_grows() {
    //Arrange: the element scaled 3 x 4 about the origin.
    let scale = DrawingTransform::IDENTITY.with_scale_xy(3.0, 4.0);
    let mut scaling = session();
    scaling.draw_shape(crate::shapes::DrawingShape::from(line(2.0, StrokeWidthMode::ScalesWithElement)).with_transform(scale));
    let mut fixed = session();
    fixed.draw_shape(crate::shapes::DrawingShape::from(line(2.0, StrokeWidthMode::Fixed)).with_transform(scale));
    //Act
    let grows = stroke_boxes(&render(&scaling, 100, 100));
    let stays = stroke_boxes(&render(&fixed, 100, 100));
    //Assert: the line is 4 x 50 = 200 tall; a scaling stroke is 2 x 4 = 8 thick, a fixed one 2.
    assert_eq!(grows.len(), 1);
    assert!(close(grows[0].height(), 8.0), "{:?}", grows[0]);
    assert_eq!(stays.len(), 1);
    assert!(close(stays[0].height(), 2.0), "{:?}", stays[0]);
    assert!(close(stays[0].x0, 30.0) && close(stays[0].x1, 180.0), "the path itself still follows the element: {:?}", stays[0]);
    assert!(close(stays[0].center().y, 200.0));
}

#[test]
fn a_fixed_width_stroke_is_in_drawing_units_so_the_calibration_and_the_camera_still_scale_it() {
    //Arrange: drawing units are 2 canvas pixels each (100 x 100 drawn at 200 x 200), the
    // element scaled 5 times, the camera zoomed 1.5 times.
    let mut s = session();
    s.draw_shape(crate::shapes::DrawingShape::from(line(2.0, StrokeWidthMode::Fixed)).with_transform(DrawingTransform::scale(5.0, PointF::new(0.0, 0.0))));
    //Act
    let plain = stroke_boxes(&render(&s, 200, 200));
    let mut camera = DrawingCamera::new();
    camera.set_zoom(1.5).unwrap();
    s.set_camera(Some(camera));
    let zoomed = stroke_boxes(&render(&s, 200, 200));
    //Assert: 2 units x 2 pixels; then x 1.5 by the camera; the element's 5 never applies.
    assert!(close(plain[0].height(), 4.0), "{:?}", plain[0]);
    assert!(close(zoomed[0].height(), 6.0), "{:?}", zoomed[0]);
}

#[test]
fn a_fixed_width_stroke_ignores_the_layer_transform_and_an_animated_scale() {
    //Arrange
    let mut s = session();
    let id = s.draw_shape(line(3.0, StrokeWidthMode::Fixed));
    s.get_layer_mut("A").unwrap().set_transform(DrawingTransform::IDENTITY.with_scale_xy(1.0, 6.0));
    let mut track = crate::animation::FloatTrack::new();
    track.add(0.0, 1.0, crate::animation::EasingKind::Linear).unwrap();
    track.add(1.0, 9.0, crate::animation::EasingKind::Linear).unwrap();
    s.animations_mut().bind(id, AnimatedProperty::Scale, track).unwrap();
    //Act
    let mut heights = Vec::new();
    for t in [0.0, 0.5, 1.0] {
        s.set_time(t);
        heights.push(stroke_boxes(&render(&s, 100, 100))[0].height());
    }
    //Assert
    for h in heights {
        assert!(close(h, 3.0), "{h}");
    }
}

#[test]
fn the_dashes_of_a_fixed_width_stroke_follow_the_width_not_the_element() {
    //Arrange: dashes 10 on, 10 off along a 50 long line scaled 4 times (200 on the canvas).
    let mut b = PathGeometryBuilder::new();
    b.move_to(0.0, 10.0).line_to(50.0, 10.0);
    let style = PathStrokeStyle::new(2.0, PathStrokeStyleOptions { cap: PathStrokeCap::Butt, dash_pattern: Some(vec![10.0, 10.0]), width_mode: StrokeWidthMode::Fixed, ..PathStrokeStyleOptions::default() }).unwrap();
    let mut s = session();
    s.draw_shape(crate::shapes::DrawingShape::from(PathShape::with_style(b.build(), style, Some(Color::BLACK), None, None)).with_transform(DrawingTransform::scale(4.0, PointF::new(0.0, 0.0))));
    let scene = render(&s, 100, 100);
    //Act: the dashes as the renderer cuts them.
    let Some(Item::Stroke { stroke, transform, path, .. }) = scene.items.iter().find(|i| matches!(i, Item::Stroke { .. })) else { panic!("no stroke") };
    let els: Vec<kurbo::PathEl> = scene.elements[path.start as usize..path.end as usize].to_vec();
    let outline = *transform * kurbo::stroke(els, &scene.strokes[*stroke as usize], &StrokeOpts::default(), 1e-4);
    let dashes = outline.elements().iter().filter(|e| matches!(e, kurbo::PathEl::MoveTo(_))).count();
    //Assert: 200 canvas units of 10 + 10 = 10 dashes (a scaling stroke would give 50 / 20, 3).
    assert_eq!(dashes, 10);
}

#[test]
fn an_element_scaled_to_nothing_draws_no_fixed_width_stroke() {
    //Arrange
    let mut s = session();
    s.draw_shape(crate::shapes::DrawingShape::from(line(2.0, StrokeWidthMode::Fixed)).with_transform(DrawingTransform::IDENTITY.with_scale_xy(0.0, 1.0)));
    //Act
    let boxes = stroke_boxes(&render(&s, 100, 100));
    //Assert
    assert!(boxes.is_empty());
}

#[test]
fn vector_scene_stroke_fixed_width_places_a_gradient_brush_with_the_shape() {
    //Arrange
    let mut scene = VectorScene::new();
    let gradient = peniko::Gradient::new_linear((0.0, 0.0), (10.0, 0.0));
    let element = Affine::scale_non_uniform(2.0, 3.0);
    //Act
    scene.stroke_fixed_width(&kurbo::Stroke::new(1.0), element, Affine::IDENTITY, &peniko::Brush::Gradient(gradient), None, &kurbo::Line::new((0.0, 0.0), (10.0, 0.0)));
    //Assert
    let Some(Item::Stroke { transform, brush_transform, .. }) = scene.items.first() else { panic!() };
    assert_eq!(*transform, Affine::IDENTITY);
    assert_eq!(*brush_transform, Some(element));
}

#[test]
fn an_outlined_text_block_with_a_fixed_width_outline_records_it_in_drawing_units() {
    //Arrange
    let Some(fonts) = crate::text::layout_tests::fonts() else { return };
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (1000, 500), layer_opacity: 255, fonts: Some(fonts), ..DrawingSessionOptions::default() });
    s.add_layer("Text", Color::WHITE);
    let layout = s.layout_text("H", 100.0, None, None).unwrap();
    let style = PathStrokeStyle::new(4.0, PathStrokeStyleOptions { width_mode: StrokeWidthMode::Fixed, ..PathStrokeStyleOptions::default() }).unwrap();
    let block = TextBlockShape::with_stroke(layout.clone(), 0.0, 0.0, Some(style.clone()), TextBlockStrokeOptions::default()).unwrap();
    let scaling = TextBlockShape::with_stroke(layout, 0.0, 0.0, Some(style.with_width_mode(StrokeWidthMode::ScalesWithElement)), TextBlockStrokeOptions::default()).unwrap();
    let mut other = DrawingSession::new(s.options().clone());
    other.add_layer("Text", Color::WHITE);
    let scale = DrawingTransform::IDENTITY.with_scale_xy(1.0, 5.0);
    s.draw_shape(crate::shapes::DrawingShape::from(block).with_transform(scale));
    other.draw_shape(crate::shapes::DrawingShape::from(scaling).with_transform(scale));
    //Act
    let fixed = render(&s, 1000, 500);
    let grows = render(&other, 1000, 500);
    //Assert: the H's stems are vertical, so the outline's horizontal reach shows the width: a
    // fixed outline reaches 2 units past the glyph either side whatever the y scale; the fixed
    // and the scaling one differ only where the scale is uneven.
    let (f, g) = (stroke_boxes(&fixed)[0], stroke_boxes(&grows)[0]);
    assert!(close(f.width(), g.width()), "x is not scaled: the same horizontal reach ({f:?} / {g:?})");
    assert!(g.height() - f.height() > 15.0, "y is scaled 5 times: the scaling outline reaches 5 x 2 units, the fixed one 2 ({f:?} / {g:?})");
}

fn svg_scene(markup: &str) -> VectorScene {
    let svg = Arc::new(DrawingSvg::from_svg(markup, &DrawingSvgOptions::default()).expect("loads"));
    let mut s = session();
    s.draw_svg(&svg, RectangleF::new(0.0, 0.0, 100.0, 100.0), DrawingSvgAspect::Meet);
    render(&s, 100, 100)
}

#[test]
fn the_svg_reader_honours_vector_effect_non_scaling_stroke_from_an_attribute_a_style_or_a_rule() {
    //Arrange: a 10 x 10 viewBox drawn into 100 x 100 - every document unit is 10 drawing units.
    let doc = |attrs: &str, style: &str| format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><style>{style}</style><line x1="1" y1="5" x2="9" y2="5" stroke="#000" stroke-width="2" {attrs}/></svg>"##);
    //Act
    let scaled = stroke_boxes(&svg_scene(&doc("", "")));
    let by_attribute = stroke_boxes(&svg_scene(&doc(r#"vector-effect="non-scaling-stroke""#, "")));
    let by_style = stroke_boxes(&svg_scene(&doc(r#"style="vector-effect: non-scaling-stroke""#, "")));
    let by_rule = stroke_boxes(&svg_scene(&doc(r#"class="thin""#, ".thin { vector-effect: non-scaling-stroke; }")));
    let overridden = stroke_boxes(&svg_scene(&doc(r#"class="thin" style="vector-effect:none""#, ".thin { vector-effect: non-scaling-stroke; }")));
    //Assert: 2 document units = 20 canvas pixels when scaled; 2 drawing units = 2 pixels fixed.
    assert!(close(scaled[0].height(), 20.0), "{:?}", scaled[0]);
    assert!(close(by_attribute[0].height(), 2.0), "{:?}", by_attribute[0]);
    assert!(close(by_style[0].height(), 2.0), "{:?}", by_style[0]);
    assert!(close(by_rule[0].height(), 2.0), "{:?}", by_rule[0]);
    assert!(close(overridden[0].height(), 20.0), "the style attribute wins over the rule");
    assert!(close(by_attribute[0].x0, 10.0) && close(by_attribute[0].x1, 90.0), "the path still follows the viewBox");
}

#[test]
fn a_non_scaling_svg_stroke_keeps_its_id_and_is_marked_in_the_scene_data() {
    //Arrange
    let markup = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path id="p" d="M1 1 L9 9" stroke="#000" vector-effect="non-scaling-stroke"/><rect width="4" height="4" fill="none" stroke="#f00" vector-effect="non-scaling-stroke"/></svg>"##;
    //Act
    let svg = DrawingSvg::from_svg(markup, &DrawingSvgOptions::default()).unwrap();
    //Assert: both paths (the rect without an id of its own too) are marked.
    let marked = svg.scene().nodes.iter().filter(|n| matches!(&n.kind, crate::svg::SvgNodeKind::Path(p) if p.non_scaling_stroke && p.stroke.as_ref().map(|s| s.non_scaling).unwrap_or(false))).count();
    assert_eq!(marked, 2);
    assert!(svg.has_part("p"));
}
