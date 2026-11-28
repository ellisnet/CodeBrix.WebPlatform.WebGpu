//! Host tests of the cull rectangle (Not in C#): what a session leaves out of the scene with
//! `SessionRenderOptions::cull`, that it never leaves out anything that would show inside the
//! rectangle (the bounds are conservative), the effects, groups and water that grow the test,
//! and the opt-in culling of a two-session wipe.

use kurbo::{Rect, Shape, StrokeOpts};

use crate::color::Color;
use crate::drawing_session::{DrawingSession, SessionRenderOptions};
use crate::drawing_session_options::DrawingSessionOptions;
use crate::drawing_session_paint::{LinePaint, SessionPaint};
use crate::drawing_session_wipe::{SessionWipe, WipeEdge};
use crate::effects::DrawingEffects;
use crate::geometry::{PointF, RectangleF};
use crate::models::{DrawingCamera, DrawingTransform};
use crate::paths::{PathGeometryBuilder, PathStrokeCap, PathStrokeJoin, PathStrokeStyle, PathStrokeStyleOptions, StrokeWidthMode};
use crate::rendering::SceneItem as Item;
use crate::rendering::VectorScene;
use crate::shapes::{ArrowShape, ArrowShapeOptions, CircleShape, DrawingGroup, DrawingShape, EllipseShape, LineShape, LineShapeOptions, PathShape, PolylineShape, PolylineShapeOptions, RectangleShape, RectangleShapeOptions, ShapeOptions};
use crate::water::{RippleDrop, RippleField, RippleOptions, WaterReflection, WaterRefraction, WaterSource};

/// A session of 100 x 100 drawing units with one layer "A".
fn session() -> DrawingSession {
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (100, 100), layer_opacity: 255, ..DrawingSessionOptions::default() });
    s.add_layer("A", Color::WHITE);
    s
}

fn record(s: &DrawingSession, size: u32, options: &SessionRenderOptions) -> VectorScene {
    let mut scene = VectorScene::new();
    s.render_with(&mut scene, size, size, options);
    scene
}

fn culled(c: RectangleF) -> SessionRenderOptions {
    SessionRenderOptions { cull: Some(c), ..SessionRenderOptions::default() }
}

/// The number of fills and strokes recorded.
fn draws(scene: &VectorScene) -> usize {
    scene.item_kinds().chars().filter(|c| *c == 'F' || *c == 'S').count()
}

/// The exact canvas-pixel box of every recorded fill and stroke (a stroke expanded as the
/// renderer expands it).
fn drawn_boxes(scene: &VectorScene) -> Vec<Rect> {
    scene
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Fill { transform, path, .. } => {
                let els: kurbo::BezPath = scene.elements[path.start as usize..path.end as usize].iter().copied().collect();
                Some((*transform * els).bounding_box())
            }
            Item::Stroke { stroke, transform, path, .. } => {
                let els: Vec<kurbo::PathEl> = scene.elements[path.start as usize..path.end as usize].to_vec();
                let outline = kurbo::stroke(els, &scene.strokes[*stroke as usize], &StrokeOpts::default(), 1e-4);
                Some((*transform * outline).bounding_box())
            }
            _ => None,
        })
        .collect()
}

fn overlaps(a: Rect, c: RectangleF) -> bool {
    a.x1 > c.x as f64 && a.x0 < c.right() as f64 && a.y1 > c.y as f64 && a.y0 < c.bottom() as f64
}

/// A small deterministic generator (the tests read no randomness from anywhere else).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32) / ((1u64 << 24) as f32)
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next()
    }
}

/// One element of a varied kind, place, stroke and transform.
fn element(g: &mut Lcg, i: usize) -> DrawingShape {
    let (x, y) = (g.range(-20.0, 120.0), g.range(-20.0, 120.0));
    let (w, h) = (g.range(0.5, 25.0), g.range(0.5, 25.0));
    let thick = g.range(0.5, 9.0);
    let filled = g.next() < 0.5;
    let shape: DrawingShape = match i % 9 {
        0 => RectangleShape::new(x, y, w, h, RectangleShapeOptions { stroke_thickness: thick, color: None, is_filled: filled, corner_radius: if g.next() < 0.5 { 3.0 } else { 0.0 } }).unwrap().into(),
        1 => CircleShape::new(x, y, w * 0.5, ShapeOptions { stroke_thickness: thick, color: None, is_filled: filled }).unwrap().into(),
        2 => EllipseShape::new(x, y, w * 0.5, h * 0.5, ShapeOptions { stroke_thickness: thick, color: None, is_filled: filled }).unwrap().into(),
        3 => LineShape::new(x, y, x + w, y - h, LineShapeOptions { stroke_thickness: thick, color: None }).unwrap().into(),
        4 => PolylineShape::new(&[PointF::new(x, y), PointF::new(x + w, y + 2.0), PointF::new(x + 3.0, y + h)], PolylineShapeOptions { stroke_thickness: thick, color: None, is_closed: true, is_filled: filled }).unwrap().into(),
        5 => ArrowShape::new(x, y, x + w, y + h, ArrowShapeOptions { stroke_thickness: thick, color: None, head_length: Some(6.0) }).unwrap().into(),
        _ => {
            // A curved path, its curve bulging well past its end points, stroked in one of the
            // three width modes with a miter join (the widest reach).
            let mut b = PathGeometryBuilder::new();
            b.move_to(x, y).cubic_to(PointF::new(x + w * 2.0, y - h), PointF::new(x - w, y + h * 2.0), PointF::new(x + w, y + h)).line_to(x + w * 0.5, y + h * 1.5);
            let mode = [StrokeWidthMode::ScalesWithElement, StrokeWidthMode::Fixed, StrokeWidthMode::CssPixels][i % 3];
            let style = PathStrokeStyle::new(thick, PathStrokeStyleOptions { cap: PathStrokeCap::Square, join: PathStrokeJoin::Miter, width_mode: mode, ..PathStrokeStyleOptions::default() }).unwrap();
            PathShape::with_style(b.build(), style, Some(Color::BLACK), if filled { Some(Color::WHITE) } else { None }, None).into()
        }
    };
    let transform = DrawingTransform::IDENTITY.with_rotation(g.range(-180.0, 180.0)).with_scale_xy(g.range(0.3, 3.0), g.range(0.3, 3.0)).with_anchor(PointF::new(x, y)).with_translation(PointF::new(g.range(-30.0, 30.0), g.range(-30.0, 30.0)));
    shape.with_transform(transform)
}

#[test]
fn without_a_cull_rectangle_everything_is_encoded_and_with_one_only_what_can_reach_it() {
    //Arrange: a 10 x 10 grid of filled squares, 6 units each, every 10 units.
    let mut s = session();
    for j in 0..10 {
        for i in 0..10 {
            s.draw_rectangle(2.0 + 10.0 * i as f32, 2.0 + 10.0 * j as f32, 6.0, 6.0, None, None, true, 0.0).unwrap();
        }
    }
    //Act
    let all = record(&s, 100, &SessionRenderOptions::default());
    let quarter = record(&s, 100, &culled(RectangleF::new(0.0, 0.0, 50.0, 50.0)));
    let one = record(&s, 100, &culled(RectangleF::new(43.0, 43.0, 4.0, 4.0)));
    let nothing = record(&s, 100, &culled(RectangleF::new(500.0, 500.0, 10.0, 10.0)));
    //Assert
    assert_eq!(SessionRenderOptions::default().cull, None);
    assert_eq!(draws(&all), 100);
    assert_eq!(draws(&quarter), 25, "the squares of the first five rows and columns");
    assert_eq!(draws(&one), 1, "the one square under the rectangle");
    assert_eq!(draws(&nothing), 0);
    assert_eq!(nothing.item_kinds(), "LP", "the layer's group is still pushed");
}

#[test]
fn nothing_that_draws_inside_the_cull_rectangle_is_ever_left_out() {
    //Arrange: hundreds of varied elements, one per session, under a turned and zoomed camera,
    // a scaled layer and drawing units of 2 canvas pixels; a 2x pixel ratio.
    let mut g = Lcg(7);
    let cull = RectangleF::new(60.0, 50.0, 80.0, 70.0);
    let options = SessionRenderOptions { pixel_ratio: 2.0, ..culled(cull) };
    let plain = SessionRenderOptions { pixel_ratio: 2.0, ..SessionRenderOptions::default() };
    let (mut visible, mut dropped) = (0, 0);
    for i in 0..600 {
        let mut s = session();
        let mut camera = DrawingCamera::new();
        camera.set_zoom(1.25).unwrap();
        camera.set_rotation(15.0).unwrap();
        s.set_camera(Some(camera));
        s.get_layer_mut("A").unwrap().set_transform(DrawingTransform::IDENTITY.with_scale_xy(1.0, 0.8));
        let shape = element(&mut g, i);
        if i % 5 == 0 {
            // Some inside a transformed group.
            let mut group = DrawingGroup::new(None);
            group.add(shape);
            s.draw_shape(DrawingShape::from(group).with_transform(DrawingTransform::IDENTITY.with_rotation(-30.0)));
        } else {
            s.draw_shape(shape);
        }
        //Act
        let full = record(&s, 200, &plain);
        let cut = record(&s, 200, &options);
        //Assert
        if drawn_boxes(&full).iter().any(|b| overlaps(*b, cull)) {
            visible += 1;
            assert_eq!(cut.item_kinds(), full.item_kinds(), "element {i} draws inside the rectangle but was left out");
        } else if draws(&cut) < draws(&full) {
            dropped += 1;
        }
    }
    assert!(visible > 50 && dropped > 50, "the cases cover both sides: {visible} visible, {dropped} left out");
}

#[test]
fn effects_strokes_and_the_line_look_grow_the_test_by_their_reach() {
    //Arrange: a 10-unit square 12 canvas pixels to the right of the rectangle.
    let cull = RectangleF::new(0.0, 0.0, 50.0, 100.0);
    let square = |effects: Option<DrawingEffects>| {
        let mut s = session();
        let shape = DrawingShape::from(RectangleShape::new(62.0, 40.0, 10.0, 10.0, RectangleShapeOptions { stroke_thickness: 1.0, color: None, is_filled: true, corner_radius: 0.0 }).unwrap());
        s.draw_shape(match effects {
            Some(e) => shape.with_effects(e),
            None => shape,
        });
        s
    };
    //Act
    let bare = draws(&record(&square(None), 100, &culled(cull)));
    let blurred = draws(&record(&square(Some(DrawingEffects::blur(20.0).unwrap())), 100, &culled(cull)));
    let shadowed = draws(&record(&square(Some(DrawingEffects::drop_shadow(PointF::new(-15.0, 0.0), 0.0, Color::BLACK).unwrap())), 100, &culled(cull)));
    let lined = SessionRenderOptions { paint: SessionPaint::Line(LinePaint::new(PathStrokeStyle::new(30.0, PathStrokeStyleOptions::default()).unwrap(), Color::BLACK)), ..culled(cull) };
    let outlined = draws(&record(&square(None), 100, &lined));
    let mut layer_blur = square(None);
    layer_blur.get_layer_mut("A").unwrap().set_effects(Some(DrawingEffects::blur(20.0).unwrap()));
    let layer_blurred = draws(&record(&layer_blur, 100, &culled(cull)));
    let mut group = DrawingGroup::new(None);
    group.add(RectangleShape::new(62.0, 40.0, 10.0, 10.0, RectangleShapeOptions { stroke_thickness: 1.0, color: None, is_filled: true, corner_radius: 0.0 }).unwrap());
    let mut grouped = session();
    grouped.draw_shape(DrawingShape::from(group).with_effects(DrawingEffects::glow(20.0, Color::WHITE, 1.0).unwrap()));
    let glowing_group = draws(&record(&grouped, 100, &culled(cull)));
    //Assert
    assert_eq!(bare, 0, "12 pixels away with no reach: left out");
    assert_eq!((blurred, shadowed, outlined, layer_blurred, glowing_group), (1, 1, 1, 1, 1), "each reaches into the rectangle");
}

#[test]
fn a_reflection_keeps_what_it_mirrors_into_the_rectangle_and_a_refraction_what_it_bends_into_it() {
    //Arrange: a waterline at y = 50; a square at y 10..20 (its reflection lies at 80..90), and
    // the rectangle over the reflection only.
    let mut s = session();
    s.draw_rectangle(40.0, 10.0, 10.0, 10.0, None, None, true, 0.0).unwrap();
    let cull = culled(RectangleF::new(0.0, 78.0, 100.0, 15.0));
    //Act
    let without = draws(&record(&s, 100, &cull));
    s.get_layer_mut("A").unwrap().set_reflection(Some(WaterReflection::new(50.0))).unwrap();
    let with_reflection = draws(&record(&s, 100, &cull));
    let mut far = session();
    far.draw_rectangle(40.0, 10.0, 10.0, 10.0, None, None, true, 0.0).unwrap();
    far.get_layer_mut("A").unwrap().set_reflection(Some(WaterReflection::new(5.0))).unwrap();
    let mirrored_away = draws(&record(&far, 100, &cull));
    //Assert
    assert_eq!(without, 0);
    assert_eq!(with_reflection, 1, "the square is drawn: its reflection shows inside the rectangle");
    assert_eq!(mirrored_away, 0, "a waterline above the square mirrors nothing into the rectangle");
}

#[test]
fn a_refraction_of_the_layer_or_of_the_session_grows_the_test_by_its_largest_offset() {
    //Arrange: a square 20 pixels outside the rectangle; rippling water bending the view strongly.
    let water = || {
        let field = RippleField::new(RectangleF::new(0.0, 0.0, 100.0, 100.0), RippleOptions::default()).unwrap().with_drop(RippleDrop::new(PointF::new(50.0, 50.0), 0.0)).unwrap();
        WaterRefraction { strength: 1000.0, ..WaterRefraction::new(WaterSource::Ripples(field)) }
    };
    let mut s = session();
    s.draw_rectangle(70.0, 40.0, 10.0, 10.0, None, None, true, 0.0).unwrap();
    s.set_time(0.5);
    let cull = culled(RectangleF::new(0.0, 0.0, 50.0, 100.0));
    //Act
    let dry = draws(&record(&s, 100, &cull));
    let session_wet = draws(&record(&s, 100, &SessionRenderOptions { refraction: Some(water()), ..cull.clone() }));
    s.get_layer_mut("A").unwrap().set_refraction(Some(water())).unwrap();
    let layer_wet = draws(&record(&s, 100, &cull));
    //Assert
    assert_eq!((dry, session_wet, layer_wet), (0, 1, 1));
}

#[test]
fn a_culled_wipe_encodes_each_session_only_where_it_shows_and_draws_the_same_picture_parts() {
    //Arrange: two sessions with a square at each side of the canvas.
    let make = || {
        let mut s = session();
        s.draw_rectangle(5.0, 45.0, 10.0, 10.0, None, None, true, 0.0).unwrap();
        s.draw_rectangle(85.0, 45.0, 10.0, 10.0, None, None, true, 0.0).unwrap();
        s
    };
    let (a, b) = (make(), make());
    let wipe = SessionWipe::new(0.5, WipeEdge::Line { angle_degrees: 0.0 }, 10.0);
    let options = SessionRenderOptions::default();
    //Act
    let mut plain = VectorScene::new();
    wipe.render_sessions(&mut plain, 100, 100, (&a, &options), (&b, &options));
    let mut cut = VectorScene::new();
    wipe.render_sessions_culled(&mut cut, 100, 100, (&a, &options), (&b, &options));
    let mut iris = VectorScene::new();
    SessionWipe::new(0.2, WipeEdge::Circle { center: PointF::new(90.0, 50.0) }, 0.0).render_sessions_culled(&mut iris, 100, 100, (&a, &options), (&b, &options));
    //Assert: the first shows on the right (its left square wiped away), the second on the left.
    assert_eq!(draws(&plain), 4 + 1, "both sessions in full, and the band's ramp");
    assert_eq!(draws(&cut), 2 + 1, "one square of each session");
    assert_eq!(draws(&iris), 2 + 1, "the first in full, the second's square inside the small circle");
}

#[test]
fn the_cull_rectangle_leaves_a_camera_panned_picture_correct_at_its_edge() {
    //Arrange: a long row of squares, the camera panned along it; the rectangle is the canvas.
    let mut s = session();
    for i in 0..50 {
        s.draw_rectangle(10.0 * i as f32, 45.0, 6.0, 6.0, None, None, true, 0.0).unwrap();
    }
    let mut camera = DrawingCamera::new();
    camera.set_pan(PointF::new(200.0, 0.0)).unwrap();
    s.set_camera(Some(camera));
    let canvas = culled(RectangleF::new(0.0, 0.0, 100.0, 100.0));
    //Act
    let full = record(&s, 100, &SessionRenderOptions::default());
    let cut = record(&s, 100, &canvas);
    //Assert: every square on the canvas is kept; most of the row is not encoded.
    let on_canvas = drawn_boxes(&full).iter().filter(|b| overlaps(**b, RectangleF::new(0.0, 0.0, 100.0, 100.0))).count();
    assert!((9..=12).contains(&on_canvas), "{on_canvas}");
    assert!(draws(&cut) >= on_canvas && draws(&cut) <= on_canvas + 2, "{} for {on_canvas}", draws(&cut));
    let kept: Vec<Rect> = drawn_boxes(&cut);
    for b in drawn_boxes(&full).iter().filter(|b| overlaps(**b, RectangleF::new(0.0, 0.0, 100.0, 100.0))) {
        assert!(kept.contains(b), "{b:?} left out");
    }
}
