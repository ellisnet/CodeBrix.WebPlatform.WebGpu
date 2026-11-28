//! Host tests of the session's render-time looks (Not in C#): the session blend mode and
//! opacity, what the scene records for them, and the nesting level they cost in the renderer.

use kurbo::{Affine, Shape};
use peniko::{BlendMode, Compose, Mix};

use crate::color::Color;
use crate::drawing_session::{DrawingSession, SessionRenderOptions};
use crate::drawing_session_blend::SessionBlendMode;
use crate::drawing_session_options::DrawingSessionOptions;
use crate::effects::DrawingEffects;
use crate::drawing_session_paint::{LinePaint, SessionPaint};
use crate::effects::{DrawingFrameOperations, DrawingMask};
use crate::fills::{ColorStop, GradientSpread, LinearGradientFill};
use crate::geometry::{PointF, RectangleF};
use crate::images::{ImageSampling, ImageSource};
use crate::models::DrawingTransform;
use crate::paths::{PathGeometry, PathStrokeStyle, PathStrokeStyleOptions, StrokeWidthMode};
use crate::rendering::{SceneItem as Item, TestBuilder, VectorScene};
use crate::shapes::{CircleShape, DrawingGroup, DrawingShape, LineShape, LineShapeOptions, OwnershipShape, OwnershipShapeOptions, RectangleShape, RectangleShapeOptions, ShapeOptions};

pub(crate) fn session() -> DrawingSession {
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (100, 100), layer_opacity: 255, background_fill_color: Color::BLACK, ..DrawingSessionOptions::default() });
    s.add_layer("A", Color::WHITE);
    s.draw_circle(50.0, 50.0, 10.0, None, None, true).unwrap();
    s
}

pub(crate) fn record(s: &DrawingSession, options: &SessionRenderOptions) -> VectorScene {
    let mut scene = VectorScene::new();
    s.render_with(&mut scene, 100, 100, options);
    scene
}

/// The blend and alpha of every recorded group, in order.
fn groups(scene: &VectorScene) -> Vec<(BlendMode, f32)> {
    scene.items.iter().filter_map(|i| if let Item::PushLayer { blend, alpha, .. } = i { Some((*blend, *alpha)) } else { None }).collect()
}

#[test]
fn the_default_render_options_draw_the_session_as_before() {
    //Arrange
    let s = session();
    //Act
    let scene = record(&s, &SessionRenderOptions::default());
    //Assert
    assert_eq!(scene.item_kinds(), "LFP");
    assert_eq!(SessionRenderOptions::default().blend, SessionBlendMode::Normal);
    assert_eq!(SessionRenderOptions::default().opacity, 1.0);
}

#[test]
fn a_blended_session_is_one_group_in_its_blend_mode_around_the_background_and_the_layers() {
    //Arrange
    let s = session();
    let modes = [
        (SessionBlendMode::Multiply, BlendMode::new(Mix::Multiply, Compose::SrcOver)),
        (SessionBlendMode::Screen, BlendMode::new(Mix::Screen, Compose::SrcOver)),
        (SessionBlendMode::Overlay, BlendMode::new(Mix::Overlay, Compose::SrcOver)),
        (SessionBlendMode::Darken, BlendMode::new(Mix::Darken, Compose::SrcOver)),
        (SessionBlendMode::Lighten, BlendMode::new(Mix::Lighten, Compose::SrcOver)),
        (SessionBlendMode::Add, BlendMode::new(Mix::Normal, Compose::Plus)),
    ];
    for (mode, expected) in modes {
        //Act
        let scene = record(&s, &SessionRenderOptions { include_background: true, blend: mode, ..SessionRenderOptions::default() });
        //Assert
        assert_eq!(scene.item_kinds(), "LFLFPP", "{mode:?}: the background inside the session's group");
        assert_eq!(groups(&scene)[0], (expected, 1.0), "{mode:?}");
    }
}

#[test]
fn the_render_opacity_fades_the_background_too_and_multiplies_the_session_opacity() {
    //Arrange
    let mut s = session();
    s.set_opacity(0.5);
    //Act
    let with_background = record(&s, &SessionRenderOptions { include_background: true, opacity: 0.4, ..SessionRenderOptions::default() });
    let without = record(&s, &SessionRenderOptions { opacity: 0.4, ..SessionRenderOptions::default() });
    //Assert: with the background the session's own opacity stays a group of its own inside (the
    // background is not faded by it); without one the two fold into one group.
    assert_eq!(with_background.item_kinds(), "LFLLFPPP");
    let g = groups(&with_background);
    assert!((g[0].1 - 0.4).abs() < 1e-6 && (g[1].1 - 0.5).abs() < 1e-6, "{g:?}");
    assert_eq!(without.item_kinds(), "LLFPP");
    assert!((groups(&without)[0].1 - 0.2).abs() < 1e-6);
}

#[test]
fn a_session_at_render_opacity_zero_draws_nothing_and_nan_counts_as_zero() {
    //Arrange
    let s = session();
    //Act
    let zero = record(&s, &SessionRenderOptions { include_background: true, opacity: 0.0, ..SessionRenderOptions::default() });
    let nan = record(&s, &SessionRenderOptions { opacity: f32::NAN, ..SessionRenderOptions::default() });
    //Assert
    assert_eq!(zero.item_kinds(), "");
    assert_eq!(nan.item_kinds(), "");
}

#[test]
fn a_blended_session_drawn_into_a_part_of_the_canvas_is_blended_inside_the_clip() {
    //Arrange
    let s = session();
    //Act
    let scene = record(&s, &SessionRenderOptions { target: Some(crate::geometry::RectangleF::new(10.0, 10.0, 50.0, 50.0)), blend: SessionBlendMode::Screen, ..SessionRenderOptions::default() });
    //Assert
    assert_eq!(scene.item_kinds(), "CLLFPPP");
}

#[test]
fn a_blended_session_costs_one_offscreen_level_and_an_over_deep_scene_is_counted_unsupported() {
    //Arrange: an element with a blur inside a faded group inside a faded layer: three levels.
    let mut s = session();
    let mut group = DrawingGroup::new(None);
    group.add(DrawingShape::from(CircleShape::new(30.0, 30.0, 5.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap()).with_effects(DrawingEffects::NONE.with_blur_radius(3.0).unwrap()));
    group.add(CircleShape::new(60.0, 30.0, 5.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap());
    s.draw_shape(DrawingShape::from(group).with_opacity(0.5));
    s.get_layer_mut("A").unwrap().set_opacity(0.5);
    let build = |options: &SessionRenderOptions| {
        let mut b = TestBuilder::default();
        b.build(&record(&s, options), 100, 100);
        b.stats
    };
    //Act
    let plain = build(&SessionRenderOptions::default());
    let multiplied = build(&SessionRenderOptions { blend: SessionBlendMode::Multiply, ..SessionRenderOptions::default() });
    //Assert: the plain scene fits; the blend's group is one more level, so the deepest (the
    // blur's) no longer fits and is counted - the renderer's usual report of an over-deep scene.
    assert_eq!(plain.unsupported, 0);
    assert_eq!(multiplied.offscreen + multiplied.unsupported, plain.offscreen + 1 + plain.unsupported);
    assert_eq!(multiplied.unsupported, 1);
}

// ---- Animation 2: the render-time paint override, the effects switch, per-layer exceptions ----

fn line_paint(width: f32, mode: StrokeWidthMode) -> LinePaint {
    LinePaint::new(PathStrokeStyle::new(width, PathStrokeStyleOptions { width_mode: mode, ..PathStrokeStyleOptions::default() }).unwrap(), Color::from_rgb(20, 30, 40))
}

fn line(width: f32) -> SessionRenderOptions {
    SessionRenderOptions { paint: SessionPaint::Line(line_paint(width, StrokeWidthMode::ScalesWithElement)), ..SessionRenderOptions::default() }
}

/// The solid colour of every recorded fill or stroke (None for a gradient or image brush).
fn solids(scene: &VectorScene) -> Vec<Option<[f32; 4]>> {
    scene
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Fill { brush, .. } | Item::Stroke { brush, .. } => Some(match &scene.brushes[*brush as usize] {
                peniko::Brush::Solid(c) => Some(c.components),
                _ => None,
            }),
            _ => None,
        })
        .collect()
}

fn widths(scene: &VectorScene) -> Vec<f64> {
    scene.items.iter().filter_map(|i| if let Item::Stroke { stroke, .. } = i { Some(scene.strokes[*stroke as usize].width) } else { None }).collect()
}

#[test]
fn line_mode_outlines_every_fill_and_redraws_every_stroke_in_the_line_style_without_changing_the_scene() {
    //Arrange: a filled circle and a 6-wide red line.
    let mut s = session();
    s.draw_shape(LineShape::new(10.0, 10.0, 90.0, 10.0, LineShapeOptions { stroke_thickness: 6.0, color: Some(Color::from_rgb(255, 0, 0)) }).unwrap());
    let before = record(&s, &SessionRenderOptions::default());
    //Act
    let lines = record(&s, &line(2.0));
    let after = record(&s, &SessionRenderOptions::default());
    //Assert
    assert_eq!(before.item_kinds(), "LFSP");
    assert_eq!(lines.item_kinds(), "LSSP", "the fill became its outline");
    assert_eq!(widths(&lines), vec![2.0, 2.0]);
    let c = Color::from_rgb(20, 30, 40).to_peniko(1.0).components;
    assert!(solids(&lines).iter().all(|x| *x == Some(c)));
    assert_eq!(after.item_kinds(), before.item_kinds(), "the session is unchanged: the next render is as authored");
    assert!(lines.look.is_none(), "the look ends with the session's render");
}

#[test]
fn line_mode_with_a_paper_colour_fills_every_filled_shape_with_it_under_its_outline() {
    //Arrange
    let s = session();
    let paper = Color::from_rgb(250, 248, 240);
    //Act
    let scene = record(&s, &SessionRenderOptions { paint: SessionPaint::Line(line_paint(1.5, StrokeWidthMode::ScalesWithElement).with_fill(paper)), ..SessionRenderOptions::default() });
    //Assert
    assert_eq!(scene.item_kinds(), "LFSP");
    assert_eq!(solids(&scene)[0], Some(paper.to_peniko(1.0).components));
}

#[test]
fn a_fixed_width_line_style_keeps_one_line_weight_under_the_elements_scaling() {
    //Arrange
    let mut s = session();
    let mut group = DrawingGroup::new(None);
    group.add(CircleShape::new(10.0, 10.0, 4.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap());
    s.draw_shape(DrawingShape::from(group).with_transform(DrawingTransform::IDENTITY.with_scale_xy(3.0, 7.0)));
    let fixed = SessionRenderOptions { paint: SessionPaint::Line(line_paint(2.0, StrokeWidthMode::Fixed)), ..SessionRenderOptions::default() };
    //Act
    let scene = record(&s, &fixed);
    //Assert: both outlines are recorded in the drawing's space (the identity here).
    let transforms: Vec<kurbo::Affine> = scene.items.iter().filter_map(|i| if let Item::Stroke { transform, .. } = i { Some(*transform) } else { None }).collect();
    assert_eq!(transforms.len(), 2);
    assert!(transforms.iter().all(|t| *t == kurbo::Affine::IDENTITY), "{transforms:?}");
}

#[test]
fn a_css_pixel_line_style_is_width_times_the_pixel_ratio_whatever_the_zoom_or_the_elements_scaling() {
    //Arrange: drawing units of 2 canvas pixels (100 x 100 drawn at 200 x 200), a camera zoomed
    // 1.5 times, a circle and a scaled group; a 1-CSS-pixel line look at a 2x pixel ratio.
    let mut s = session();
    let mut group = DrawingGroup::new(None);
    group.add(CircleShape::new(10.0, 10.0, 4.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap());
    s.draw_shape(DrawingShape::from(group).with_transform(DrawingTransform::IDENTITY.with_scale_xy(3.0, 7.0)));
    let mut camera = crate::models::DrawingCamera::new();
    camera.set_zoom(1.5).unwrap();
    s.set_camera(Some(camera));
    let look = |ratio: f64| SessionRenderOptions { paint: SessionPaint::Line(line_paint(1.0, StrokeWidthMode::CssPixels)), pixel_ratio: ratio, ..SessionRenderOptions::default() };
    let record_at = |options: &SessionRenderOptions| {
        let mut scene = VectorScene::new();
        s.render_with(&mut scene, 200, 200, options);
        scene
    };
    //Act
    let two = record_at(&look(2.0));
    let one = record_at(&look(1.0));
    //Assert: every outline is expanded in the CSS pixel - recorded under the ratio's scale, its
    // width in CSS pixels - so it is 2 device pixels wide at 2x and 1 at 1x.
    for (scene, ratio) in [(&two, 2.0), (&one, 1.0)] {
        let strokes: Vec<(kurbo::Affine, f64)> = scene.items.iter().filter_map(|i| if let Item::Stroke { transform, stroke, .. } = i { Some((*transform, scene.strokes[*stroke as usize].width)) } else { None }).collect();
        assert_eq!(strokes.len(), 2);
        for (t, w) in strokes {
            assert_eq!(t, kurbo::Affine::scale(ratio), "expanded in the CSS pixel at {ratio}x");
            assert_eq!(w, 1.0);
        }
    }
}

#[test]
fn flat_mode_paints_a_gradient_its_average_colour_and_keeps_outlines_as_authored() {
    //Arrange: a black-to-white gradient rectangle and a red outline.
    let mut s = session();
    let stops = [ColorStop::new(0.0, Color::BLACK).unwrap(), ColorStop::new(1.0, Color::WHITE).unwrap()];
    let gradient = LinearGradientFill::new(PointF::new(0.0, 0.0), PointF::new(100.0, 0.0), &stops, GradientSpread::Pad).unwrap();
    s.draw_shape(RectangleShape::with_fill(0.0, 0.0, 100.0, 50.0, gradient.into(), 0.0).unwrap());
    s.draw_rectangle(10.0, 60.0, 30.0, 30.0, Some(3.0), Some(Color::from_rgb(255, 0, 0)), false, 0.0).unwrap();
    //Act
    let scene = record(&s, &SessionRenderOptions { paint: SessionPaint::Flat, ..SessionRenderOptions::default() });
    //Assert
    assert_eq!(scene.item_kinds(), "LFFSP");
    let colours = solids(&scene);
    let grey = colours[1].expect("the gradient became a solid colour");
    assert!((grey[0] - 0.5).abs() < 0.01 && (grey[1] - 0.5).abs() < 0.01 && (grey[3] - 1.0).abs() < 1e-6, "{grey:?}");
    assert_eq!(colours[2], Some(Color::from_rgb(255, 0, 0).to_peniko(1.0).components), "the outline as authored");
    assert_eq!(widths(&scene), vec![3.0]);
}

#[test]
fn a_picture_is_left_out_of_a_line_drawing_and_becomes_its_average_colour_in_flat_mode() {
    //Arrange: a 2 x 1 picture, one opaque red pixel and one transparent pixel.
    let mut s = session();
    let picture = ImageSource::from_rgba_vec(vec![255, 0, 0, 255, 0, 0, 255, 0], 2, 1).unwrap();
    s.draw_image(&picture, RectangleF::new(0.0, 0.0, 40.0, 20.0), ImageSampling::Linear).unwrap();
    //Act
    let lines = record(&s, &line(1.0));
    let flat = record(&s, &SessionRenderOptions { paint: SessionPaint::Flat, ..SessionRenderOptions::default() });
    //Assert: the circle's outline alone; flat: pure red (the transparent pixel adds no colour) at half alpha.
    assert_eq!(lines.item_kinds(), "LSP");
    assert_eq!(flat.item_kinds(), "LFFP");
    let c = solids(&flat)[1].unwrap();
    assert!((c[0] - 1.0).abs() < 1e-6 && c[1].abs() < 1e-6 && (c[3] - 0.5).abs() < 0.01, "{c:?}");
    assert_eq!(flat.averages.len(), 1, "the average is kept for the next frame");
}

#[test]
fn an_ownership_map_is_left_out_of_a_line_drawing_and_flat_is_its_owners_colours_by_area() {
    //Arrange: the left half of the map owned by element 0 (blue), the right half nobody's (white).
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (8, 4), layer_opacity: 255, ..DrawingSessionOptions::default() });
    s.add_layer("Paint", Color::WHITE).add_shape(RectangleShape::new(0.0, 0.0, 4.0, 4.0, RectangleShapeOptions { is_filled: true, ..Default::default() }).unwrap());
    let map = s.export_layer_ownership_map("Paint", (8, 4)).unwrap();
    let shape = OwnershipShape::new(&map, &[Color::from_rgb(0, 0, 255)], RectangleF::new(0.0, 0.0, 8.0, 4.0), OwnershipShapeOptions { unowned: Color::WHITE, ..Default::default() }).unwrap();
    let mut other = DrawingSession::new(s.options().clone());
    other.add_layer("Map", Color::WHITE).add_shape(shape);
    //Act
    let lines = record(&other, &line(1.0));
    let flat = record(&other, &SessionRenderOptions { paint: SessionPaint::Flat, ..SessionRenderOptions::default() });
    //Assert: half blue, half white -> (0.5, 0.5, 1.0).
    assert_eq!(lines.item_kinds(), "LP");
    assert_eq!(flat.item_kinds(), "LFP");
    let c = solids(&flat)[0].unwrap();
    assert!((c[0] - 0.5).abs() < 0.01 && (c[1] - 0.5).abs() < 0.01 && (c[2] - 1.0).abs() < 0.01, "{c:?}");
}

#[test]
fn the_effects_switch_drops_blur_glow_and_shadow_but_keeps_a_layers_frame_operations() {
    //Arrange
    let mut s = session();
    s.draw_shape(DrawingShape::from(CircleShape::new(30.0, 30.0, 5.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap()).with_effects(DrawingEffects::NONE.with_blur_radius(3.0).unwrap()));
    let no_effects = SessionRenderOptions { effects: false, ..SessionRenderOptions::default() };
    //Act
    let with = record(&s, &SessionRenderOptions::default());
    let without = record(&s, &no_effects);
    s.get_layer_mut("A").unwrap().set_frame_operations(Some(DrawingFrameOperations::NONE.with_gamma(1.5).unwrap()));
    let framed = record(&s, &no_effects);
    //Assert
    assert_eq!(with.item_kinds(), "LFEFPP");
    assert_eq!(without.item_kinds(), "LFLFPP", "the blurred element is an ordinary group");
    let fx: Vec<_> = framed.items.iter().filter_map(|i| if let Item::PushEffects { params, .. } = i { Some(*params) } else { None }).collect();
    assert_eq!(fx.len(), 1, "the layer's frame group stays");
    assert!(fx[0].frame.is_some() && fx[0].blur_sigma == 0.0 && !fx[0].has_glow && !fx[0].has_shadow);
}

#[test]
fn a_layer_named_as_an_exception_is_drawn_as_authored() {
    //Arrange
    let mut s = session();
    s.add_layer("Keep", Color::WHITE);
    s.draw_circle(20.0, 20.0, 5.0, None, None, true).unwrap();
    //Act
    let scene = record(&s, &SessionRenderOptions { authored_layers: vec!["Keep".to_string()], ..line(1.0) });
    //Assert: layer A outlined, layer Keep filled.
    assert_eq!(scene.item_kinds(), "LSPLFP");
}

#[test]
fn a_mask_keeps_its_coverage_in_every_look() {
    //Arrange: a layer masked outside a rectangle (content, then the path destination-out) and a
    // layer matte (content, then the matte's elements destination-in).
    let mut s = session();
    s.get_layer_mut("A").unwrap().set_mask(Some(DrawingMask::outside(PathGeometry::rectangle(RectangleF::new(0.0, 0.0, 20.0, 20.0)))));
    //Act
    let scene = record(&s, &line(1.0));
    //Assert: the content became an outline, the mask's fill stayed a fill.
    assert_eq!(scene.item_kinds(), "LSLFPP");
    assert_eq!(solids(&scene)[1], Some(Color::WHITE.to_peniko(1.0).components));
}

#[test]
fn line_mode_draws_text_as_outlined_letters() {
    //Arrange
    let Some(fonts) = crate::text::layout_tests::fonts() else { return };
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (1000, 500), layer_opacity: 255, fonts: Some(fonts), ..DrawingSessionOptions::default() });
    s.add_layer("Text", Color::WHITE);
    s.draw_text("Hi", 10.0, 100.0, 80.0, Default::default()).unwrap();
    let mut scene = VectorScene::new();
    //Act
    s.render_with(&mut scene, 1000, 500, &SessionRenderOptions { paint: SessionPaint::Line(line_paint(2.0, StrokeWidthMode::ScalesWithElement)), ..SessionRenderOptions::default() });
    //Assert: two glyphs, two outlines in the text's space (the identity calibration here).
    assert_eq!(scene.item_kinds(), "LSSP");
    assert_eq!(widths(&scene), vec![2.0, 2.0]);
}

#[test]
fn an_svg_document_gets_the_look_too() {
    //Arrange
    let svg = std::sync::Arc::new(crate::svg::DrawingSvg::from_svg(r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect width="10" height="10" fill="#f00"/></svg>"##, &crate::svg::DrawingSvgOptions::default()).unwrap());
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (40, 20), layer_opacity: 255, ..DrawingSessionOptions::default() });
    s.add_layer("Svg", Color::WHITE);
    s.draw_svg(&svg, RectangleF::new(0.0, 0.0, 40.0, 20.0), crate::svg::DrawingSvgAspect::Meet);
    //Act
    let mut scene = VectorScene::new();
    s.render_with(&mut scene, 40, 20, &line(1.0));
    //Assert
    assert_eq!(scene.item_kinds(), "LSP");
}

// ---- Animation 3: the wipe between two renders ----

use crate::drawing_session_wipe::{SessionWipe, WipeEdge};

/// A first render (one red fill) and a second (one blue fill), each a group the wipe can see.
fn red(scene: &mut VectorScene) {
    scene.fill(peniko::Fill::NonZero, Affine::IDENTITY, &peniko::Brush::Solid(Color::from_rgb(255, 0, 0).to_peniko(1.0)), None, &kurbo::Rect::new(0.0, 0.0, 100.0, 100.0));
}

fn blue(scene: &mut VectorScene) {
    scene.fill(peniko::Fill::NonZero, Affine::IDENTITY, &peniko::Brush::Solid(Color::from_rgb(0, 0, 255).to_peniko(1.0)), None, &kurbo::Rect::new(0.0, 0.0, 100.0, 100.0));
}

fn wiped(wipe: SessionWipe) -> VectorScene {
    let mut scene = VectorScene::new();
    wipe.render(&mut scene, RectangleF::new(0.0, 0.0, 100.0, 100.0), red, blue);
    scene
}

#[test]
fn a_wipe_at_its_ends_draws_only_one_render() {
    //Arrange
    let edge = WipeEdge::Line { angle_degrees: 0.0 };
    //Act
    let start = wiped(SessionWipe::new(0.0, edge, 20.0));
    let end = wiped(SessionWipe::new(1.0, edge, 20.0));
    let nan = wiped(SessionWipe::new(f32::NAN, edge, 20.0));
    //Assert: clipped to the area, one fill each.
    assert_eq!(start.item_kinds(), "CFP");
    assert_eq!(solids(&start)[0], Some(Color::from_rgb(255, 0, 0).to_peniko(1.0).components));
    assert_eq!(end.item_kinds(), "CFP");
    assert_eq!(solids(&end)[0], Some(Color::from_rgb(0, 0, 255).to_peniko(1.0).components));
    assert_eq!(nan.item_kinds(), "CFP");
}

#[test]
fn a_soft_wipe_draws_the_first_clipped_and_the_second_in_one_group_kept_destination_in_by_the_band() {
    //Arrange: left to right over 0..100 with a 20 pixel band, half way: the band is 40..60.
    let wipe = SessionWipe::new(0.5, WipeEdge::Line { angle_degrees: 0.0 }, 20.0);
    //Act
    let scene = wiped(wipe);
    //Assert
    assert_eq!(scene.item_kinds(), "CCFPLFLFPPP");
    let Item::PushClip { transform, shape, .. } = &scene.items[1] else { panic!() };
    let clip = (*transform * kurbo::BezPath::from_vec(scene.elements[shape.start as usize..shape.end as usize].to_vec())).bounding_box();
    assert!((clip.x0 - 40.0).abs() < 1e-9, "the first shows from the band's start: {clip:?}");
    let g = groups(&scene);
    assert_eq!(g[0].0, BlendMode::default());
    assert_eq!(g[1].0.compose, Compose::DestIn);
    let Item::Fill { brush, .. } = &scene.items[7] else { panic!() };
    let peniko::Brush::Gradient(ramp) = &scene.brushes[*brush as usize] else { panic!("the band is a gradient") };
    let peniko::GradientKind::Linear(l) = ramp.kind else { panic!() };
    assert!((l.start.x - 40.0).abs() < 1e-9 && (l.end.x - 60.0).abs() < 1e-9, "{l:?}");
    let alphas: Vec<f32> = ramp.stops.iter().map(|s| s.color.to_alpha_color::<peniko::color::Srgb>().components[3]).collect();
    assert_eq!(alphas.first(), Some(&1.0));
    assert_eq!(alphas.last(), Some(&0.0));
    assert!((alphas[4] - 0.5).abs() < 1e-6, "smooth, symmetric: half way at the middle");
}

#[test]
fn a_wipe_at_an_angle_travels_across_the_whole_area_from_corner_to_corner() {
    //Arrange: 45 degrees over a 100 x 100 area: t runs 0..141.4 along the diagonal.
    let edge = WipeEdge::Line { angle_degrees: 45.0 };
    //Act
    let barely = wiped(SessionWipe::new(0.001, edge, 10.0));
    //Assert: the band starts just inside the near corner.
    let Item::Fill { brush, .. } = &barely.items[7] else { panic!() };
    let peniko::Brush::Gradient(ramp) = &barely.brushes[*brush as usize] else { panic!() };
    let peniko::GradientKind::Linear(l) = ramp.kind else { panic!() };
    let t = |p: kurbo::Point| (p.x + p.y) / 2f64.sqrt();
    assert!(t(l.start) < 0.0 && t(l.end) > 0.0 && (t(l.end) - t(l.start) - 10.0).abs() < 1e-6, "{l:?}");
}

#[test]
fn a_hard_wipe_only_clips_both_renders() {
    //Act
    let scene = wiped(SessionWipe::new(0.3, WipeEdge::Line { angle_degrees: 180.0 }, 0.0));
    //Assert: no group at all.
    assert_eq!(scene.item_kinds(), "CCFPCFPP");
}

#[test]
fn an_iris_grows_from_its_centre_with_the_second_render_inside() {
    //Arrange: from the centre of a 100 x 100 area (farthest corner 70.7 away), band 10.
    let edge = WipeEdge::Circle { center: PointF::new(50.0, 50.0) };
    //Act
    let early = wiped(SessionWipe::new(0.05, edge, 10.0));
    let half = wiped(SessionWipe::new(0.5, edge, 10.0));
    //Assert: early the band has not left the centre (the first fills the area); half way the
    // first is clipped to outside the inner circle (even-odd), and the ramp is radial.
    assert_eq!(early.item_kinds(), "CCFPLFLFPPP");
    let Item::PushClip { rule, .. } = &half.items[1] else { panic!() };
    assert_eq!(*rule, peniko::Fill::EvenOdd);
    let Item::Fill { brush, .. } = &half.items[7] else { panic!() };
    let peniko::Brush::Gradient(ramp) = &half.brushes[*brush as usize] else { panic!() };
    let peniko::GradientKind::Radial(r) = ramp.kind else { panic!("radial") };
    let inner = -10.0 + 0.5 * (50f64.hypot(50.0) + 10.0);
    assert!((r.end_radius as f64 - (inner + 10.0)).abs() < 1e-3, "{r:?}");
}

#[test]
fn a_soft_wipe_costs_one_nesting_level_for_the_second_render() {
    //Arrange: a session three levels deep (as in the blend test) as the second render.
    let mut s = session();
    let mut group = DrawingGroup::new(None);
    group.add(DrawingShape::from(CircleShape::new(30.0, 30.0, 5.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap()).with_effects(DrawingEffects::NONE.with_blur_radius(3.0).unwrap()));
    group.add(CircleShape::new(60.0, 30.0, 5.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap());
    s.draw_shape(DrawingShape::from(group).with_opacity(0.5));
    s.get_layer_mut("A").unwrap().set_opacity(0.5);
    let stats = |wipe: SessionWipe, deep_first: bool| {
        let mut scene = VectorScene::new();
        let render = |scene: &mut VectorScene| s.render(scene, 100, 100);
        if deep_first {
            wipe.render(&mut scene, RectangleF::new(0.0, 0.0, 100.0, 100.0), render, red);
        } else {
            wipe.render(&mut scene, RectangleF::new(0.0, 0.0, 100.0, 100.0), red, render);
        }
        let mut b = TestBuilder::default();
        b.build(&scene, 100, 100);
        b.stats
    };
    //Act
    let as_first = stats(SessionWipe::new(0.5, WipeEdge::Line { angle_degrees: 0.0 }, 20.0), true);
    let as_second = stats(SessionWipe::new(0.5, WipeEdge::Line { angle_degrees: 0.0 }, 20.0), false);
    //Assert: drawn first it fits; drawn second its deepest group is one level too deep.
    assert_eq!(as_first.unsupported, 0);
    assert_eq!(as_second.unsupported, 1);
}

#[test]
fn render_sessions_puts_the_second_sessions_blend_and_opacity_on_the_wipes_group() {
    //Arrange
    let a = session();
    let b = session();
    let multiplied = SessionRenderOptions { blend: SessionBlendMode::Multiply, opacity: 0.5, ..SessionRenderOptions::default() };
    //Act
    let mut scene = VectorScene::new();
    SessionWipe::new(0.5, WipeEdge::Line { angle_degrees: 90.0 }, 16.0).render_sessions(&mut scene, 100, 100, (&a, &SessionRenderOptions::default()), (&b, &multiplied));
    let mut hard = VectorScene::new();
    SessionWipe::new(0.5, WipeEdge::Line { angle_degrees: 90.0 }, 0.0).render_sessions(&mut hard, 100, 100, (&a, &SessionRenderOptions::default()), (&b, &multiplied));
    //Assert: the first session; the wipe's group multiplied at 0.5 holding the second session
    // drawn plainly; the band's destination-in.
    assert_eq!(scene.item_kinds(), "CCLFPPLLFPLFPPP");
    let g = groups(&scene);
    assert_eq!((g[1].0, g[1].1), (BlendMode::new(Mix::Multiply, Compose::SrcOver), 0.5));
    assert_eq!(g[2].0, BlendMode::default(), "the second session's own group is a plain one");
    // A hard edge has no wipe group: the session blends by itself.
    let gh = groups(&hard);
    assert!(gh.iter().any(|(b, a)| b.mix == Mix::Multiply && (*a - 0.5).abs() < 1e-6), "{gh:?}");
}

#[test]
fn one_session_can_be_wiped_between_two_looks() {
    //Arrange: as authored -> line drawing.
    let s = session();
    let lines = line(1.0);
    //Act
    let mut scene = VectorScene::new();
    SessionWipe::new(0.5, WipeEdge::Circle { center: PointF::new(50.0, 50.0) }, 10.0).render_sessions(&mut scene, 100, 100, (&s, &SessionRenderOptions::default()), (&s, &lines));
    //Assert: the circle filled under, outlined over.
    assert_eq!(scene.item_kinds(), "CCLFPPLLSPLFPPP");
}

// ---- BLEND MODES PER LAYER AND PER ELEMENT (Not in C#) ----

/// The alpha of every recorded effects group, in order.
fn effect_alphas(scene: &VectorScene) -> Vec<f32> {
    scene.items.iter().filter_map(|i| if let Item::PushEffects { alpha, .. } = i { Some(*alpha) } else { None }).collect()
}

#[test]
fn a_normal_element_or_layer_draws_as_before_with_no_extra_group() {
    //Arrange
    let mut s = session();
    let before = record(&s, &SessionRenderOptions::default());
    //Act
    s.get_layer_mut("A").unwrap().set_blend_mode(SessionBlendMode::Normal);
    let id = s.layers()[0].get_elements()[0].id();
    s.find_element_mut(id).unwrap().set_blend_mode(SessionBlendMode::Normal);
    let after = record(&s, &SessionRenderOptions::default());
    //Assert
    assert_eq!(s.get_layer("A").unwrap().blend_mode(), SessionBlendMode::Normal);
    assert_eq!(crate::models::DrawingLayer::new("B", Color::WHITE).blend_mode(), SessionBlendMode::Normal);
    assert_eq!(DrawingShape::from(CircleShape::new(1.0, 1.0, 1.0, ShapeOptions::default()).unwrap()).blend_mode(), SessionBlendMode::Normal);
    assert_eq!(after.item_kinds(), before.item_kinds());
    assert_eq!(groups(&after), groups(&before));
}

#[test]
fn a_blended_element_is_one_group_in_its_mode_at_its_opacity_and_drawn_inside_at_full_opacity() {
    //Arrange
    let mut s = session();
    s.draw_shape(DrawingShape::from(CircleShape::new(30.0, 30.0, 10.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap()).with_blend_mode(SessionBlendMode::Multiply).with_opacity(0.5));
    //Act
    let scene = record(&s, &SessionRenderOptions::default());
    //Assert: the layer's group, the first circle, the blend's group around the second.
    assert_eq!(scene.item_kinds(), "LFLFPP");
    let g = groups(&scene);
    assert_eq!(g[1], (BlendMode::new(Mix::Multiply, Compose::SrcOver), 0.5));
    let last = scene.brushes.last().unwrap();
    assert!(matches!(last, peniko::Brush::Solid(c) if (c.components[3] - 1.0).abs() < 1e-6), "drawn at full opacity inside: {last:?}");
}

#[test]
fn a_blended_element_with_effects_puts_the_blend_group_around_the_effects_group() {
    //Arrange
    let mut s = session();
    let id = s.draw_rectangle(10.0, 10.0, 20.0, 20.0, None, None, true, 0.0).unwrap();
    let e = s.find_element_mut(id).unwrap();
    e.set_effects(Some(DrawingEffects::blur(4.0).unwrap()));
    e.set_blend_mode(SessionBlendMode::Screen);
    e.set_opacity(0.25);
    //Act
    let scene = record(&s, &SessionRenderOptions::default());
    //Assert
    assert_eq!(scene.item_kinds(), "LFLEFPPP");
    assert_eq!(groups(&scene)[1], (BlendMode::new(Mix::Screen, Compose::SrcOver), 0.25));
    assert_eq!(effect_alphas(&scene), vec![1.0], "the effects at full opacity inside the blend");
}

#[test]
fn a_blended_faded_group_folds_its_fade_into_the_blend_group() {
    //Arrange
    let mut s = session();
    let mut group = DrawingGroup::new(None);
    group.add(RectangleShape::new(10.0, 10.0, 20.0, 20.0, RectangleShapeOptions { is_filled: true, ..RectangleShapeOptions::default() }).unwrap());
    group.add(RectangleShape::new(20.0, 20.0, 20.0, 20.0, RectangleShapeOptions { is_filled: true, ..RectangleShapeOptions::default() }).unwrap());
    s.draw_shape(DrawingShape::from(group).with_opacity(0.5).with_blend_mode(SessionBlendMode::Darken));
    //Act
    let scene = record(&s, &SessionRenderOptions::default());
    //Assert: one group (the blend's, at 0.5) - not a fade group inside it.
    assert_eq!(scene.item_kinds(), "LFLFFPP");
    assert_eq!(groups(&scene)[1], (BlendMode::new(Mix::Darken, Compose::SrcOver), 0.5));
}

#[test]
fn a_blended_layer_puts_its_own_group_down_in_its_mode_and_around_its_effects_with_them() {
    //Arrange
    let mut s = session();
    s.get_layer_mut("A").unwrap().set_blend_mode(SessionBlendMode::Add);
    //Act
    let plain = record(&s, &SessionRenderOptions::default());
    s.get_layer_mut("A").unwrap().set_effects(Some(DrawingEffects::glow(5.0, Color::WHITE, 1.0).unwrap()));
    s.get_layer_mut("A").unwrap().set_opacity(0.5);
    let with_effects = record(&s, &SessionRenderOptions::default());
    //Assert
    assert_eq!(plain.item_kinds(), "LFP", "no extra group: the layer's own is blended");
    assert_eq!(groups(&plain)[0].0, BlendMode::new(Mix::Normal, Compose::Plus));
    assert_eq!(with_effects.item_kinds(), "LEFPP");
    assert_eq!(groups(&with_effects)[0].0, BlendMode::new(Mix::Normal, Compose::Plus));
    assert!((groups(&with_effects)[0].1 - 0.5).abs() < 0.01, "the layer's alpha on the blend's group");
    assert_eq!(effect_alphas(&with_effects), vec![1.0]);
    let built = crate::models::DrawingLayer::new("B", Color::WHITE).with_blend_mode(SessionBlendMode::Lighten);
    assert_eq!(built.blend_mode(), SessionBlendMode::Lighten);
}
