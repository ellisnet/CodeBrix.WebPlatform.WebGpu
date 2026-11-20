//! Host tests: the scene as data, parts, overrides, the walk and the report (test_files/parts.svg).

use std::sync::Arc;

use kurbo::{Affine, Point};

use crate::animation::{EasingKind, FloatTrack};
use crate::geometry::{PointF, RectangleF};
use crate::models::AnimatedProperty;
use crate::svg::*;

fn parts() -> DrawingSvg {
    DrawingSvg::from_svg(include_str!("test_files/parts.svg"), &DrawingSvgOptions::default()).expect("parts.svg loads")
}

fn close(a: Point, b: (f64, f64)) -> bool {
    (a.x - b.0).abs() < 1e-6 && (a.y - b.1).abs() < 1e-6
}

fn has(svg: &DrawingSvg, kind: DrawingSvgWarningKind) -> bool {
    svg.warnings().iter().any(|w| w.kind == kind)
}

#[test]
fn from_svg_reads_size_view_box_and_parts() {
    //Arrange
    let svg = parts();
    //Act
    let ids = svg.get_part_ids();
    //Assert
    assert_eq!(svg.bounds(), RectangleF::new(0.0, 0.0, 200.0, 100.0));
    assert_eq!(svg.scene().view_box(), kurbo::Rect::new(0.0, 0.0, 400.0, 200.0));
    for id in ["gear", "body", "moved", "clipped", "masked", "t1", "t2", "patterned", "blurred", "hand", "photo"] {
        assert!(ids.contains(&id.to_string()), "missing part {id}: {ids:?}");
    }
    assert!(!svg.has_part("sky"), "paint servers are not parts");
    assert!(!svg.has_part("tooth"), "usvg does not keep ids inside <use>d content (a difference from C#): animate the <use> by its own id");
}

#[test]
fn scene_carries_paints_strokes_groups_clips_masks_and_images() {
    //Arrange
    let svg = parts();
    let scene = svg.scene();
    let node = |id: &str| &scene.nodes()[scene.find_nodes(id)[0]];
    //Act
    let body = match &node("body").kind {
        SvgNodeKind::Path(p) => p.clone(),
        other => panic!("body is {other:?}"),
    };
    //Assert
    let fill = body.fill.expect("body is filled");
    match fill.paint {
        SvgPaint::Linear(g) => {
            assert_eq!(g.stops.len(), 2);
            assert_eq!(g.stops[1].color, crate::Color::from_rgba(0, 0, 255, 128));
        }
        other => panic!("body paint {other:?}"),
    }
    let stroke = body.stroke.expect("body is stroked");
    assert_eq!(stroke.width, 4.0);
    assert_eq!(stroke.dash_array, Some(vec![5.0, 3.0]));
    match &node("gear").kind {
        SvgNodeKind::Group(g) => assert_eq!(g.opacity, 0.5),
        other => panic!("gear {other:?}"),
    }
    match &node("clipped").kind {
        SvgNodeKind::Group(g) => assert!(g.clip.is_some()),
        other => panic!("clipped {other:?}"),
    }
    match &node("masked").kind {
        SvgNodeKind::Group(g) => assert_eq!(g.mask.as_ref().map(|m| m.kind), Some(SvgMaskKind::Alpha)),
        other => panic!("masked {other:?}"),
    }
    let blurred = scene.part("blurred").unwrap().owners[0];
    let blurred_group = scene.nodes()[blurred].parent.unwrap();
    match &scene.nodes()[blurred_group].kind {
        SvgNodeKind::Group(g) => assert_eq!(g.blur_std_deviation, Some(3.0)),
        other => panic!("blurred group {other:?}"),
    }
    let image = scene.nodes().iter().find(|n| matches!(n.kind, SvgNodeKind::Image(_))).expect("the image record");
    match &image.kind {
        SvgNodeKind::Image(i) => {
            assert_eq!(i.source, SvgImageSource::Href("pictures/photo.png".to_string()));
            assert_eq!(i.size, (64.0, 48.0));
        }
        _ => unreachable!(),
    }
    let moved = scene.part("moved").unwrap();
    assert_eq!(moved.transform_attribute, Affine::translate((5.0, 0.0)));
    assert!(matches!(scene.nodes()[moved.owners[0]].kind, SvgNodeKind::Path(_)), "a part nothing animates is its path (part overrides wrap it)");
}

#[test]
fn report_lists_what_is_not_drawn() {
    //Arrange
    let svg = parts();
    //Act
    let report = svg.report();
    //Assert
    assert!(has(&svg, DrawingSvgWarningKind::NoFontsRegistered));
    assert!(has(&svg, DrawingSvgWarningKind::PatternNotDrawn));
    assert!(has(&svg, DrawingSvgWarningKind::UnsupportedFilterPrimitive));
    assert!(has(&svg, DrawingSvgWarningKind::ImageNeedsPixels));
    assert!(report.iter().any(|l| l.starts_with("PatternNotDrawn: ")));
}

#[test]
fn walk_places_paths_in_document_coordinates_and_skips_undrawable_paints() {
    //Arrange
    let svg = parts();
    //Act
    let list = svg.walk(None, &[], Affine::IDENTITY);
    //Assert
    let body = svg.scene().find_nodes("body")[0];
    let body_path = match &svg.scene().nodes()[body].kind {
        SvgNodeKind::Path(p) => p.path.clone(),
        _ => unreachable!(),
    };
    let fill = list.commands.iter().find_map(|c| if let SvgDrawCommand::Fill { path, transform, .. } = c { if Arc::ptr_eq(path, &body_path) { Some(*transform) } else { None } } else { None }).expect("body fill drawn");
    // viewBox 400x200 into 200x100 (x 0.5), then translate(100 50): (-20,-20) -> (40, 15).
    assert!(close(fill * Point::new(-20.0, -20.0), (40.0, 15.0)));
    let patterned = svg.scene().find_nodes("patterned")[0];
    let pat_path = match &svg.scene().nodes()[patterned].kind {
        SvgNodeKind::Path(p) => p.path.clone(),
        _ => unreachable!(),
    };
    assert!(!list.commands.iter().any(|c| matches!(c, SvgDrawCommand::Fill { path, .. } if Arc::ptr_eq(path, &pat_path))), "a pattern fill is drawn as nothing");
    let pushes = list.commands.iter().filter(|c| matches!(c, SvgDrawCommand::PushGroup { .. } | SvgDrawCommand::PushClip { .. })).count();
    let pops = list.commands.iter().filter(|c| matches!(c, SvgDrawCommand::Pop)).count();
    assert_eq!(pushes, pops);
    assert!(list.commands.iter().any(|c| matches!(c, SvgDrawCommand::MaskBegin { kind: SvgMaskKind::Alpha, .. })));
    assert!(list.commands.iter().any(|c| matches!(c, SvgDrawCommand::PushGroup { blur_std_deviation: Some(b), .. } if *b == 3.0)));
    assert!(list.commands.iter().any(|c| matches!(c, SvgDrawCommand::Image { .. })));
}

#[test]
fn part_rotation_turns_about_a_document_pivot_and_trim_cuts_strokes() {
    //Arrange
    let svg = Arc::new(parts());
    let mut shape = DrawingSvgShape::new(svg.clone(), RectangleF::new(0.0, 0.0, 200.0, 100.0), DrawingSvgAspect::None);
    {
        let hand = shape.get_part_mut("hand").unwrap();
        hand.pivot = Some(PointF::new(100.0, 50.0));
        hand.rotation = 90.0;
        hand.trim_end = 0.5;
    }
    //Act
    let list = shape.draw_list(0.0);
    //Assert
    let (path, transform) = list.commands.iter().find_map(|c| if let SvgDrawCommand::Stroke { path, transform, stroke, .. } = c { if stroke.width == 3.0 { Some((path.clone(), *transform)) } else { None } } else { None }).expect("hand stroked");
    // The hand runs (200,100) -> (200,20) in viewBox units = (100,50) -> (100,10) in the document;
    // turned 90 degrees clockwise about (100,50), its tip lands at (140,50).
    assert!(close(transform * Point::new(200.0, 20.0), (140.0, 50.0)));
    let len = crate::paths::PathGeometry::from_bez_path((*path).clone(), crate::paths::PathFillRule::NonZero).get_length();
    assert!((len - 40.0).abs() < 1e-3, "trimmed to half of 80: {len}");
    let b = shape.get_part_bounds("hand", 0.0).expect("hand bounds");
    assert!((b.x - 100.0).abs() < 1e-3 && (b.right() - 120.0).abs() < 1e-3 && b.height.abs() < 1e-3, "the trimmed stroke's box: {b:?}");
}

#[test]
fn hidden_and_transparent_parts_draw_nothing() {
    //Arrange
    let svg = Arc::new(parts());
    let mut shape = DrawingSvgShape::new(svg, RectangleF::new(0.0, 0.0, 200.0, 100.0), DrawingSvgAspect::Meet);
    let all = shape.draw_list(0.0).draw_count();
    //Act
    shape.get_part_mut("hand").unwrap().is_visible = false;
    shape.get_part_mut("moved").unwrap().opacity = 0.0;
    let fewer = shape.draw_list(0.0).draw_count();
    shape.reset_parts();
    let again = shape.draw_list(0.0).draw_count();
    //Assert
    assert!(fewer < all, "{fewer} < {all}");
    assert_eq!(shape.get_part_bounds("hand", 0.0).is_some(), true, "reset shows the hand again");
    assert_eq!(again, all);
}

#[test]
fn aspect_fits_the_document_and_maps_points() {
    //Arrange
    let svg = Arc::new(parts());
    //Act
    let meet = DrawingSvgShape::new(svg.clone(), RectangleF::new(0.0, 0.0, 400.0, 400.0), DrawingSvgAspect::Meet);
    let slice = DrawingSvgShape::new(svg.clone(), RectangleF::new(0.0, 0.0, 400.0, 400.0), DrawingSvgAspect::Slice);
    let stretch = DrawingSvgShape::new(svg, RectangleF::new(10.0, 0.0, 400.0, 400.0), DrawingSvgAspect::Stretch);
    //Assert
    // 200 x 100 into 400 x 400: Meet scale 2, centred vertically (+100).
    assert_eq!(meet.map_to_calibrated(PointF::new(100.0, 50.0)), PointF::new(200.0, 200.0));
    assert_eq!(meet.content_bounds(), RectangleF::new(0.0, 100.0, 400.0, 200.0));
    // Slice scale 4, centred horizontally (-200); its bounds are the destination.
    assert_eq!(slice.map_to_calibrated(PointF::new(0.0, 0.0)), PointF::new(-200.0, 0.0));
    assert_eq!(slice.get_bounds(), RectangleF::new(0.0, 0.0, 400.0, 400.0));
    assert_eq!(stretch.map_to_calibrated(PointF::new(200.0, 100.0)), PointF::new(410.0, 400.0));
}

#[test]
fn bind_part_drives_a_part_from_a_track_and_checks_its_arguments() {
    //Arrange
    let svg = Arc::new(parts());
    let mut shape = DrawingSvgShape::new(svg, RectangleF::new(0.0, 0.0, 200.0, 100.0), DrawingSvgAspect::None);
    let track = FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (2.0, 180.0, EasingKind::Linear)]).unwrap();
    //Act
    shape.bind_part("gear", AnimatedProperty::Rotation, track.clone()).unwrap();
    shape.apply_bindings(1.0);
    //Assert
    assert_eq!(shape.get_part("gear").unwrap().rotation, 90.0);
    assert!(shape.get_part("gear").unwrap().has_overrides());
    assert!(shape.bind_part("nope", AnimatedProperty::Rotation, track.clone()).is_err());
    assert!(shape.bind_part("gear", AnimatedProperty::Color, track).is_err());
    assert!(shape.get_part("nope").unwrap_err().contains("get_part_ids"));
}

#[test]
fn from_svg_errors_say_what_to_do() {
    //Arrange / Act
    let blank = DrawingSvg::from_svg("  ", &DrawingSvgOptions::default());
    let broken = DrawingSvg::from_svg("<svg", &DrawingSvgOptions::default());
    let not_svg = DrawingSvg::from_svg("<html/>", &DrawingSvgOptions::default());
    //Assert
    assert!(blank.unwrap_err().contains("pass the document"));
    assert!(broken.unwrap_err().contains("not well-formed"));
    assert!(not_svg.unwrap_err().contains("not <svg>"));
}

fn clock() -> Arc<DrawingSvg> {
    Arc::new(DrawingSvg::from_svg(include_str!("test_files/clock.svg"), &DrawingSvgOptions::default()).expect("clock.svg loads"))
}

#[test]
fn clock_of_the_svg_parts_scene_turns_its_hands_and_draws_its_ticks_on() {
    //Arrange
    let svg = clock();
    let mut shape = DrawingSvgShape::new(svg.clone(), RectangleF::new(0.0, 0.0, 300.0, 300.0), DrawingSvgAspect::Meet);
    for hand in ["hour", "minute", "second"] {
        shape.get_part_mut(hand).unwrap().pivot = Some(PointF::new(150.0, 150.0));
    }
    shape.bind_part("second", AnimatedProperty::Rotation, FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (8.0, 720.0, EasingKind::Linear)]).unwrap()).unwrap();
    shape.bind_part("ticks", AnimatedProperty::TrimEnd, FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (2.0, 1.0, EasingKind::Linear)]).unwrap()).unwrap();
    //Act
    let list = shape.draw_list(1.0);
    //Assert
    assert!(svg.warnings().iter().any(|w| w.kind == DrawingSvgWarningKind::NoFontsRegistered), "the clock's label is text");
    for id in ["face", "ticks", "hour", "minute", "second"] {
        assert!(svg.has_part(id), "{id}");
    }
    // At 1 s: the second hand has turned 90 degrees about (150,150): its tip (150,40) is at (260,150).
    let second = list.commands.iter().find_map(|c| if let SvgDrawCommand::Stroke { transform, stroke, .. } = c { if stroke.width == 3.0 { Some(*transform) } else { None } } else { None }).unwrap();
    assert!(close(second * Point::new(150.0, 40.0), (260.0, 150.0)));
    // Every tick (24 or 12 long) is trimmed to half its length, each on its own.
    let ticks: Vec<f32> = list.commands.iter().filter_map(|c| if let SvgDrawCommand::Stroke { path, stroke, .. } = c { if stroke.width == 7.0 || stroke.width == 4.0 { Some(crate::paths::PathGeometry::from_bez_path((**path).clone(), crate::paths::PathFillRule::NonZero).get_length()) } else { None } } else { None }).collect();
    assert_eq!(ticks.len(), 12);
    for (i, len) in ticks.iter().enumerate() {
        let full = if i % 3 == 0 { 24.0 } else { 12.0 };
        assert!((len - full / 2.0).abs() < 1e-3, "tick {i}: {len}");
    }
}

#[test]
fn ids_inside_used_content_do_not_reach_the_scene_but_the_use_ids_do() {
    //Arrange / Act
    let svg = parts();
    //Assert: usvg clears ids inside <use>d content (a difference from C#, which reaches every instance).
    assert!(!svg.has_part("tooth"));
    assert!(!svg.has_part("tooth-body"));
    assert!(svg.has_part("t1") && svg.has_part("t2"));
}

#[test]
fn radial_gradients_luminance_masks_and_embedded_images() {
    //Arrange
    let svg = parts();
    let scene = svg.scene();
    //Act
    let lit = scene.find_nodes("lit").into_iter().find_map(|i| if let SvgNodeKind::Path(p) = &scene.nodes()[i].kind { Some(p.clone()) } else { None }).expect("lit path");
    let embedded = scene.nodes().iter().find_map(|n| if let SvgNodeKind::Image(i) = &n.kind { if let SvgImageSource::Encoded { format, bytes } = &i.source { Some((*format, bytes.len(), i.size)) } else { None } } else { None });
    let list = svg.walk(None, &[], Affine::IDENTITY);
    let placed = list.commands.iter().find_map(|c| if let SvgDrawCommand::Image { source: SvgImageSource::Encoded { .. }, transform, .. } = c { Some(*transform) } else { None }).expect("the embedded image is drawn as a record");
    //Assert
    match lit.fill.unwrap().paint {
        SvgPaint::Radial(g) => {
            assert_eq!(g.spread, SvgSpread::Reflect);
            assert_ne!(g.focal, g.center, "the focal point is kept");
            // objectBoundingBox units folded into the transform: the unit circle maps onto the 40 x 40 box.
            assert!(close(g.transform * Point::new(g.center.0, g.center.1), (350.0, 170.0)));
        }
        other => panic!("lit paint {other:?}"),
    }
    assert!(svg.warnings().iter().any(|w| w.kind == DrawingSvgWarningKind::LuminanceMaskApproximated));
    // The record carries the picture's own size (4 x 2 pixels); the transform places it in its 8 x 4 box at
    // (250,150) of the viewBox, i.e. (125,75) .. (129,77) in the document.
    assert_eq!(embedded, Some(("png", 33, (4.0, 2.0))));
    assert!(close(placed * Point::new(0.0, 0.0), (125.0, 75.0)));
    assert!(close(placed * Point::new(4.0, 2.0), (129.0, 77.0)));
}
