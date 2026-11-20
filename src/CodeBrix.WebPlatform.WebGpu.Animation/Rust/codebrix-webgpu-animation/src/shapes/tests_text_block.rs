//! Host tests of the DRAWING half of text blocks and the safe-area overlay: what a text block, a
//! reveal, an outline, merged overlaps, captions and the overlay record into the scene, and the
//! session's text bindings (reveals, staggers, text along a path). Fonts as the text tests.

use crate::animation::{EasingKind, FloatTrack};
use crate::captions::{CaptionPlacement, CaptionPresenter, CaptionTrack};
use crate::color::Color;
use crate::drawing_session::DrawingSession;
use crate::drawing_session_options::DrawingSessionOptions;
use crate::layout::{DrawingLayout, DrawingLayoutGuideOptions};
use crate::models::{AnimatedProperty, PathTrainOptions, StaggerOptions};
use crate::paths::{PathGeometry, PathStrokeStyle, PathStrokeStyleOptions};
use crate::rendering::VectorScene;
use crate::safe_areas::{SafeAreaGuides, SafeAreaOverlay};
use crate::shapes::{ShapeKind, TextBlockShape, TextBlockStrokeOptions};
use crate::text::layout_tests::fonts;
use crate::text::{TextStrokePlacement, TextUnit};

fn session() -> Option<DrawingSession> {
    let fonts = fonts()?;
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (1000, 500), layer_opacity: 255, fonts: Some(fonts), ..DrawingSessionOptions::default() });
    s.add_layer("Text", Color::WHITE);
    Some(s)
}

fn kinds(s: &DrawingSession) -> String {
    let mut scene = VectorScene::new();
    s.render(&mut scene, 1000, 500);
    scene.item_kinds()
}

fn fills(s: &DrawingSession) -> usize {
    kinds(s).chars().filter(|c| *c == 'F').count()
}

fn track(points: &[(f64, f32)]) -> FloatTrack {
    let mut t = FloatTrack::new();
    for (time, value) in points {
        t.add(*time, *value, EasingKind::Linear).unwrap();
    }
    t
}

#[test]
fn a_text_block_draws_one_cached_fill_per_glyph_with_ink() {
    let Some(mut s) = session() else { return };
    let layout = s.layout_text("Hi there", 40.0, None, None).unwrap();
    s.draw_text_block(layout, 10.0, 10.0, None).unwrap();
    // 7 letters with ink, the space has none; one layer group around them.
    assert_eq!(kinds(&s), format!("L{}P", "F".repeat(7)));
}

#[test]
fn a_reveal_binding_shows_the_block_unit_by_unit_with_the_session_time() {
    let Some(mut s) = session() else { return };
    let layout = s.layout_text("abcd efgh", 40.0, None, None).unwrap();
    let id = s.draw_text_block(layout, 0.0, 0.0, None).unwrap();
    s.animations_mut().bind(id, AnimatedProperty::RevealFraction, track(&[(0.0, 0.0), (1.0, 1.0)])).unwrap();
    s.set_time(0.0);
    assert_eq!(fills(&s), 0);
    s.set_time(0.5);
    let half = fills(&s);
    assert!(half > 0 && half < 8, "{half}");
    s.set_time(1.0);
    assert_eq!(fills(&s), 8);
    s.set_time(0.5);
    assert_eq!(fills(&s), half, "the same time draws the same");
    // RevealCount by words.
    let mut s2 = session().unwrap();
    let layout = s2.layout_text("one two three", 40.0, None, None).unwrap();
    let mut block = TextBlockShape::new(layout, 0.0, 0.0, None).unwrap();
    block.set_reveal_unit(TextUnit::Word);
    let id = s2.draw_shape(block);
    s2.animations_mut().bind(id, AnimatedProperty::RevealCount, track(&[(0.0, 0.0), (3.0, 3.0)])).unwrap();
    s2.set_time(1.0);
    assert_eq!(fills(&s2), 3, "one word: o n e");
    s2.set_time(2.0);
    assert_eq!(fills(&s2), 6);
}

#[test]
fn reveal_bindings_are_refused_on_anything_but_a_text_block() {
    let Some(mut s) = session() else { return };
    let dot = s.draw_circle(0.0, 0.0, 5.0, None, None, true).unwrap();
    let error = s.animations_mut().bind(dot, AnimatedProperty::RevealFraction, track(&[(0.0, 0.0)])).unwrap_err();
    assert!(error.contains("text block"), "{error}");
}

#[test]
fn an_outside_outline_is_a_layer_with_the_glyphs_cut_out_then_the_fill() {
    let Some(mut s) = session() else { return };
    let layout = s.layout_text("O", 80.0, None, None).unwrap();
    let style = PathStrokeStyle::new(6.0, PathStrokeStyleOptions::default()).unwrap();
    let mut block = TextBlockShape::with_stroke(layout.clone(), 0.0, 0.0, Some(style.clone()), TextBlockStrokeOptions { stroke_color: Some(Color::BLACK), fill_color: Some(Color::WHITE), fill: None }).unwrap();
    block.set_stroke_placement(TextStrokePlacement::Outside);
    s.draw_shape(block);
    assert_eq!(kinds(&s), "LLSLFPPFP", "stroke in a layer, the glyphs cut (destination-out), then the fill");
    let mut s2 = session().unwrap();
    s2.draw_shape(TextBlockShape::with_stroke(layout, 0.0, 0.0, Some(style), TextBlockStrokeOptions { stroke_color: None, fill_color: Some(Color::WHITE), fill: None }).unwrap());
    assert_eq!(kinds(&s2), "LFSP", "centred: the fill, then the stroke over it");
}

#[test]
fn merge_overlaps_draws_each_glyph_simplified_into_lines() {
    let Some(mut s) = session() else { return };
    let layout = s.layout_text("ox", 80.0, None, None).unwrap();
    let mut block = TextBlockShape::new(layout, 0.0, 0.0, None).unwrap();
    assert!(block.get_outline().bez_path().elements().iter().any(|e| matches!(e, kurbo::PathEl::QuadTo(..) | kurbo::PathEl::CurveTo(..))));
    block.set_merge_overlaps(true);
    let outline = block.get_outline();
    assert!(!outline.is_empty());
    assert!(outline.bez_path().elements().iter().all(|e| !matches!(e, kurbo::PathEl::QuadTo(..) | kurbo::PathEl::CurveTo(..))), "merged outlines are polygons");
    s.draw_shape(block);
    assert_eq!(kinds(&s), "LFP", "one batch of merged outlines");
}

#[test]
fn stagger_binds_each_piece_and_binds_nothing_for_bad_options() {
    let Some(mut s) = session() else { return };
    let layout = s.layout_text("abc", 40.0, None, None).unwrap();
    let block = TextBlockShape::new(layout, 0.0, 0.0, None).unwrap();
    let ids: Vec<_> = block.create_pieces(TextUnit::Character).into_iter().map(|p| s.draw_shape(p)).collect();
    let options = StaggerOptions { interval: 0.5, duration: 0.5, from_opacity: 0.0, ..StaggerOptions::default() };
    let bindings = s.animations_mut().bind_stagger(&ids, Some(&options)).unwrap();
    assert!(bindings.len() >= 3);
    s.set_time(0.25);
    let opacity = |s: &DrawingSession, i: usize| s.find_element(ids[i]).unwrap().opacity();
    assert!(opacity(&s, 0) > 0.0 && opacity(&s, 0) < 1.0);
    assert_eq!(opacity(&s, 2), 0.0, "the last starts at 1 s");
    s.set_time(5.0);
    assert_eq!(opacity(&s, 2), 1.0);
    let before = s.animations().count();
    let bad = StaggerOptions { duration: -1.0, ..StaggerOptions::default() };
    assert!(s.animations_mut().bind_stagger(&ids, Some(&bad)).is_err());
    assert_eq!(s.animations().count(), before, "nothing bound");
}

#[test]
fn text_along_a_path_adds_the_pieces_to_the_layer_and_settles_them_in_place() {
    let Some(mut s) = session() else { return };
    let layout = s.layout_text("go on", 40.0, None, None).unwrap();
    let block = TextBlockShape::new(layout, 100.0, 100.0, None).unwrap();
    let route = PathGeometry::parse("M -500 200 L 150 120").unwrap();
    s.animations_mut().bind_text_along_path(&block, TextUnit::Character, "Text", &route, track(&[(0.0, 0.0), (1.0, 1.0)]), PathTrainOptions::default()).unwrap();
    assert_eq!(s.get_layer("Text").unwrap().get_elements().len(), 4, "the space is left out");
    s.set_time(1.0);
    let settled = s.find_element(s.get_layer("Text").unwrap().get_elements()[0].id()).unwrap().transform().translation();
    assert!(settled.x.abs() < 0.01 && settled.y.abs() < 0.01, "at the end each piece is at its own place: {settled:?}");
    assert!(s.animations_mut().bind_text_along_path(&block, TextUnit::Word, "None", &route, track(&[(0.0, 0.0)]), PathTrainOptions::default()).unwrap_err().contains("no layer"));
}

#[test]
fn captions_show_their_cues_at_the_session_time_against_the_frame_by_default() {
    let Some(mut s) = session() else { return };
    let track = CaptionTrack::parse("1\n00:00:01,000 --> 00:00:02,000\nHello there\n").unwrap();
    let id = s.draw_captions(track.clone(), None, None).unwrap();
    let children = |s: &DrawingSession| match s.find_element(id).unwrap().kind() {
        ShapeKind::Animated(a) => a.current().and_then(|c| c.as_group()).map_or(0, |g| g.children.len()),
        _ => panic!("captions are an animated shape"),
    };
    s.set_time(0.5);
    assert_eq!(children(&s), 0);
    s.set_time(1.5);
    assert_eq!(children(&s), 2, "the box, then the text");
    let bounds = s.find_element(id).unwrap().get_bounds().unwrap();
    assert!(bounds.bottom() < 500.0 && bounds.bottom() > 400.0, "near the frame's bottom edge: {bounds:?}");
    // The opt-in: inside a safe-area guide (YouTube Shorts keeps a deep band at the bottom).
    let guide = SafeAreaGuides::youtube_shorts_ad();
    let placement = CaptionPlacement::from_guide((1000, 500), &guide, DrawingLayoutGuideOptions::default()).unwrap();
    let presenter = CaptionPresenter::new(track, s.fonts(), DrawingLayout::new((1000, 500)).unwrap(), None).unwrap().with_placement(placement);
    let inside = s.draw_captions_with(presenter);
    s.set_time(1.5);
    let guided = s.find_element(inside).unwrap().get_bounds().unwrap();
    assert!(guided.bottom() < bounds.bottom(), "the guide lifts them: {guided:?} vs {bounds:?}");
}

#[test]
fn the_safe_area_overlay_draws_over_the_layers_in_groups_of_its_own() {
    let Some(mut s) = session() else { return };
    s.draw_circle(500.0, 250.0, 10.0, None, None, true).unwrap();
    assert_eq!(kinds(&s), "LFP");
    s.set_safe_area_overlay(Some(SafeAreaOverlay::new(SafeAreaGuides::broadcast_title_safe())));
    let k = kinds(&s);
    assert!(k.starts_with("LFP") && k.len() > 3, "{k}");
    assert_eq!(k.matches('L').count(), 3, "the layer, the mask, the ring: {k}");
    assert!(k.ends_with("FFFFP"), "the ring's four bands: {k}");
}
