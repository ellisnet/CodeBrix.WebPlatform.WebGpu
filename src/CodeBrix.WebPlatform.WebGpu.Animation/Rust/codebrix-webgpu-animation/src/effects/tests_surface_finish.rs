//! Host tests of a layer's surface finish (painterly P3): the checks, the composite's pass data,
//! the builder's plan (the finish on the layer's ONE effect group, the height pass listing
//! exactly the stroke draws of finished layers, the impasto in their blocks), no finish under the
//! line and flat looks, the nesting budget inside a wipe inside a blended session, and the CPU
//! mirrors of the shader's weave and shading.

use kurbo::Affine;

use crate::effects::surface_finish::{finish_params, finish_pass_data, finish_shade, smoothstep, weave_height, weave_noise, weave_showing};
use crate::effects::{CanvasWeave, DrawingFrameOperations, SurfaceFinish};
use crate::rendering::{TestBuilder as Builder, VectorScene};
use crate::shapes::{BrushStrokesOptions, BrushStrokesShape, CircleShape, DrawingShape, ShapeOptions};
use crate::strokes::{BrushStrokeRecord, BrushStrokeSet};
use crate::{Color, DrawingSession, DrawingSessionOptions, PointF, SessionBlendMode, SessionPaint, SessionRenderOptions, SessionWipe, WipeEdge};

fn set() -> BrushStrokeSet {
    BrushStrokeSet::new((0..20).map(|i| BrushStrokeRecord::new(PointF::new(40.0 + i as f32 * 25.0, 100.0), 40.0, 10.0, 0.0, Color::from_rgb(200, 80, 40))).collect()).unwrap()
}

fn strokes() -> BrushStrokesShape {
    BrushStrokesShape::new(set(), BrushStrokesOptions::default()).unwrap()
}

fn session() -> DrawingSession {
    DrawingSession::new(DrawingSessionOptions { calibration_size: (640, 360), layer_opacity: 255, ..DrawingSessionOptions::default() })
}

fn build(s: &DrawingSession, options: &SessionRenderOptions) -> Builder {
    let mut scene = VectorScene::new();
    s.render_with(&mut scene, 640, 360, options);
    let mut b = Builder::default();
    b.build(&scene, 640, 360);
    b
}

#[test]
fn a_finish_is_checked_and_a_layer_keeps_its_finish_when_a_new_one_is_unusable() {
    //Arrange
    let mut s = session();
    let layer = s.add_layer("Paint", Color::WHITE);
    //Act / Assert
    assert_eq!(layer.surface_finish(), None);
    layer.set_surface_finish(Some(SurfaceFinish::default())).unwrap();
    assert!(layer.set_surface_finish(Some(SurfaceFinish { light_elevation: 0.0, ..SurfaceFinish::default() })).unwrap_err().contains("elevation"));
    assert!(layer.set_surface_finish(Some(SurfaceFinish { gloss: 2.0, ..SurfaceFinish::default() })).unwrap_err().contains("gloss"));
    assert!(layer.set_surface_finish(Some(SurfaceFinish { impasto: -1.0, ..SurfaceFinish::default() })).unwrap_err().contains("impasto"));
    assert!(layer.set_surface_finish(Some(SurfaceFinish { weave: Some(CanvasWeave { spacing: 0.0, ..CanvasWeave::default() }), ..SurfaceFinish::default() })).unwrap_err().contains("spacing"));
    assert!(SurfaceFinish { weave: Some(CanvasWeave { irregularity: 1.5, ..CanvasWeave::default() }), ..SurfaceFinish::default() }.validate().unwrap_err().contains("irregularity"));
    assert_eq!(layer.surface_finish(), Some(SurfaceFinish::default()), "kept");
    layer.set_surface_finish(None).unwrap();
    assert_eq!(layer.surface_finish(), None);
}

#[test]
fn the_composite_pass_data_carries_the_light_the_amounts_the_weave_and_the_layer_matrix() {
    //Arrange
    let finish = SurfaceFinish { light_angle: 225.0, light_elevation: 35.0, relief: 0.6, impasto: 1.5, gloss: 0.15, weave: Some(CanvasWeave { spacing: 3.0, depth: 0.35, irregularity: 0.3, angle: 90.0, seed: 4 }) };
    let layer = Affine::translate((10.0, 20.0)) * Affine::scale(2.0);
    //Act
    let params = finish_params(&finish, layer).unwrap();
    let data = finish_pass_data(Some(&params));
    let smooth = finish_pass_data(finish_params(&SurfaceFinish { weave: None, ..finish }, layer).as_ref());
    //Assert
    let (c, s) = (35f32.to_radians().cos(), 35f32.to_radians().sin());
    let light = [c * 225f32.to_radians().cos(), c * 225f32.to_radians().sin(), s];
    assert!(data.v10[..3].iter().zip(light).all(|(a, b)| (a - b).abs() < 1e-6), "towards the upper left (screen y down), 35 degrees up: {:?}", data.v10);
    assert!(data.v10[0] < 0.0 && data.v10[1] < 0.0);
    assert_eq!(data.v10[3], 0.6);
    assert_eq!(data.v11, [0.15, 0.35, 3.0, 0.3]);
    assert_eq!(data.v12, [0.5, 0.0, 0.0, 0.5], "device -> layer");
    assert_eq!((data.v13[0], data.v13[1], data.v13[3]), (-5.0, -10.0, 2.0));
    assert!((data.v13[2] - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    assert_eq!(data.u2, [3, 4, 0, 0], "finish on, weave on, the seed");
    assert_eq!(params.impasto, 1.5, "the impasto goes to the stroke draws, not the composite");
    assert_eq!((smooth.u2[0], smooth.v11[1]), (1, 0.0), "a smooth ground: no weave");
    assert_eq!(finish_pass_data(None), crate::rendering::core_fx_pass_data::default());
    assert!(finish_params(&finish, Affine::scale(0.0)).is_none(), "nothing is drawn through a flat transform");
}

#[test]
fn a_finished_layer_has_one_effect_group_whose_height_pass_lists_exactly_its_stroke_draws() {
    //Arrange: a finished layer with two stroke sets and a circle, under a plain layer's set.
    let mut s = session();
    s.add_layer("Plain", Color::WHITE).add_shape(strokes());
    let paint = s.add_layer("Paint", Color::WHITE);
    paint.set_surface_finish(Some(SurfaceFinish { impasto: 1.25, ..SurfaceFinish::default() })).unwrap();
    paint.add_shape(strokes());
    paint.add_shape(CircleShape::new(300.0, 200.0, 40.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap());
    paint.add_shape(strokes());
    //Act
    let b = build(&s, &SessionRenderOptions::default());
    //Assert
    assert_eq!(b.stats.effects, 1, "one effect group: the finished layer's");
    assert_eq!(b.effects.len(), 1);
    assert!(b.effects[0].params.finish.is_some());
    assert_eq!(b.effects[0].height_draws, vec![1, 2], "the finished layer's two sets, not the plain layer's");
    let impasto = |op: usize| f32::from_bits(b.stroke_blocks[b.stroke_ops[op].offset as usize / 4 + 9]);
    assert_eq!((impasto(0), impasto(1), impasto(2)), (0.0, 1.25, 1.25));
    let composite = b.fx_passes.last().unwrap();
    assert_eq!(composite.u2[0], 3, "the composite lights the layer");
}

#[test]
fn a_finish_shares_the_effect_group_of_the_layers_frame_operations_and_the_looks_draw_none() {
    //Arrange
    let mut s = session();
    let paint = s.add_layer("Paint", Color::WHITE);
    paint.set_surface_finish(Some(SurfaceFinish::default())).unwrap();
    paint.set_frame_operations(Some(DrawingFrameOperations::NONE.with_gamma(1.2).unwrap()));
    paint.add_shape(strokes());
    //Act
    let as_authored = build(&s, &SessionRenderOptions::default());
    let flat = build(&s, &SessionRenderOptions { paint: SessionPaint::Flat, ..SessionRenderOptions::default() });
    let authored_flat = build(&s, &SessionRenderOptions { paint: SessionPaint::Flat, authored_layers: vec!["Paint".to_string()], ..SessionRenderOptions::default() });
    //Assert
    assert_eq!((as_authored.stats.effects, as_authored.stats.offscreen), (1, 1), "ONE group for both");
    assert!(as_authored.effects[0].params.finish.is_some() && as_authored.effects[0].params.frame.is_some());
    assert!(flat.effects[0].params.finish.is_none(), "the flat look draws no finish");
    assert!(flat.effects[0].height_draws.is_empty());
    assert!(authored_flat.effects[0].params.finish.is_some(), "an authored layer keeps it");
}

#[test]
fn a_finished_layer_inside_a_wipe_inside_a_blended_session_uses_all_three_levels() {
    //Arrange
    let mut a = session();
    a.add_layer("A", Color::WHITE).add_shape(strokes());
    let mut b = session();
    let paint = b.add_layer("Paint", Color::WHITE);
    paint.set_surface_finish(Some(SurfaceFinish::default())).unwrap();
    paint.add_shape(strokes());
    let mut deeper = b.clone();
    deeper.get_layer_mut("Paint").unwrap().add_shape(DrawingShape::from(strokes()).with_opacity(0.5));
    let multiply = SessionRenderOptions { blend: SessionBlendMode::Multiply, ..SessionRenderOptions::default() };
    let wipe = SessionWipe::new(0.5, WipeEdge::Line { angle_degrees: 0.0 }, 20.0);
    let plan = |second: &DrawingSession| {
        let mut scene = VectorScene::new();
        // The blended session drawn inside the wipe's soft band (render_sessions would fold the
        // blend into the wipe's own group and save a level).
        wipe.render(&mut scene, crate::RectangleF::new(0.0, 0.0, 640.0, 360.0), |sc| a.render(sc, 640, 360), |sc| second.render_with(sc, 640, 360, &multiply));
        let mut builder = Builder::default();
        builder.build(&scene, 640, 360);
        builder
    };
    //Act
    let full = plan(&b);
    let over = plan(&deeper);
    //Assert
    assert_eq!(full.levels_needed, 4, "the canvas target and three offscreen levels");
    assert_eq!(full.stats.unsupported, 0);
    assert!(full.effects.iter().any(|e| e.params.finish.is_some() && e.height_draws.len() == 1));
    assert!(over.stats.unsupported >= 1, "a faded set inside it is past the limit: drawn as a clip");
}

#[test]
fn the_weave_is_a_plain_weave_of_period_two_threads_hidden_by_thick_paint() {
    //Arrange
    let w = CanvasWeave { spacing: 3.0, depth: 0.35, irregularity: 0.0, angle: 0.0, seed: 0 };
    let rough = CanvasWeave { irregularity: 1.0, seed: 3, ..w };
    //Act / Assert
    assert!((weave_height(&w, 1.5, 1.5) - 0.35).abs() < 1e-6, "a warp thread on top in the middle of its cell");
    assert!((weave_height(&w, 4.5, 1.5) - 0.35).abs() < 1e-6, "the weft on top in the next cell");
    assert!(weave_height(&w, 0.0, 0.0).abs() < 1e-6, "a gap where threads part");
    for (u, v) in [(0.3, 0.7), (2.2, 5.1), (7.9, 1.3)] {
        assert!((weave_height(&w, u, v) - weave_height(&w, u + 6.0, v)).abs() < 1e-5, "period: two threads");
        assert!((weave_height(&w, u, v) - weave_height(&w, u, v + 6.0)).abs() < 1e-5);
        let h = weave_height(&rough, u, v);
        assert!((0.0..=0.35 * 1.25 + 1e-6).contains(&h), "{h}");
    }
    let turned = CanvasWeave { angle: 90.0, ..w };
    assert!((weave_height(&turned, 1.5, -1.5) - weave_height(&w, -1.5, -1.5)).abs() < 1e-5, "turned 90 degrees");
    assert_eq!((weave_showing(0.0), weave_showing(0.6), weave_showing(2.0)), (1.0, 0.0, 0.0));
    assert!(weave_showing(0.3) > 0.0 && weave_showing(0.3) < 1.0);
}

#[test]
fn the_weave_noise_is_the_renderers_hash_at_the_lattice_and_smooth_between() {
    use crate::painting::color_rules::noise_hash;
    //Act / Assert
    for (x, y) in [(0, 0), (3, -2), (-7, 11)] {
        let corner = noise_hash(5, x, y, 0) as f32 / 4_294_967_295.0 * 2.0 - 1.0;
        assert!((weave_noise(5, x as f32, y as f32) - corner).abs() < 1e-6);
    }
    let mut previous = weave_noise(5, 0.0, 0.5);
    for i in 1..=100 {
        let v = weave_noise(5, i as f32 * 0.01, 0.5);
        assert!((-1.0..=1.0).contains(&v));
        assert!((v - previous).abs() < 0.06, "continuous");
        previous = v;
    }
}

#[test]
fn flat_areas_keep_their_colour_and_a_ridge_facing_the_light_is_brighter() {
    //Arrange
    let light = SurfaceFinish::default().light_vector();
    //Act
    let flat = finish_shade(light, 0.6, 0.15, [0.0, 0.0], 1.0);
    // The light from the upper left: a surface rising to the lower right (its slope tilted up
    // towards the light's side) faces it; one rising towards the upper left turns away.
    let facing = finish_shade(light, 0.6, 0.0, [0.5, 0.5], 1.0);
    let away = finish_shade(light, 0.6, 0.0, [-0.5, -0.5], 1.0);
    let no_relief = finish_shade(light, 0.0, 0.0, [0.5, 0.5], 1.0);
    let glossy = finish_shade(light, 0.6, 0.5, [0.3, 0.3], 1.0);
    let bare = finish_shade(light, 0.6, 0.5, [0.3, 0.3], 0.0);
    //Assert
    assert!((flat.0 - 1.0).abs() < 1e-6 && flat.1.abs() < 1e-6, "{flat:?}");
    assert!(facing.0 > 1.0, "{facing:?}");
    assert!(away.0 < 1.0, "{away:?}");
    assert_eq!(no_relief.0, 1.0);
    assert!(glossy.1 > 0.0 && bare.1 == 0.0, "a highlight on paint, none on the bare canvas: {glossy:?} {bare:?}");
    assert_eq!(smoothstep(0.0, 1.0, 0.5), 0.5);
}
