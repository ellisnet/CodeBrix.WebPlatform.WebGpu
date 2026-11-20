//! "shapes": the renderer's feature sheet (not a scene of the C# demo). Every fill rule, stroke
//! style, gradient kind and extend mode, mix mode, compose mode, clip, group opacity and image
//! paint the renderer has, one per cell, laid out so that a person can see each one - with a
//! few things moving (turning stars, marching dashes, a turning sweep, a turning image) so that
//! the scene is visibly on the clock. The basic shapes go through the drawing session; the rest
//! through the VectorScene, the renderer's low-level door. The scene draws no backdrop of its own
//! (the canvas shows its clear colour), so that its sample points can tell "drawn" from "empty".

use codebrix_webgpu_animation::kurbo::{Affine, BezPath, Cap, Circle, Join, Line, Point, Rect, RoundedRect, Stroke};
use codebrix_webgpu_animation::paths::{PathGeometryBuilder, PathStrokeCap, PathStrokeStyle, PathStrokeStyleOptions};
use codebrix_webgpu_animation::peniko::color::{AlphaColor, Srgb};
use codebrix_webgpu_animation::peniko::{Blob, BlendMode, Brush, ColorStop, Compose, Extend, Fill, Gradient, ImageAlphaType, ImageBrush, ImageData, ImageFormat, ImageQuality, Mix};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{EllipseShape, CircleShape, PathShape, PolylineShape, PolylineShapeOptions, RectangleShape, RectangleShapeOptions, ShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions, PointF};

use crate::demo::{add_header, label, DemoScene, Expect, SamplePoint, DIM, FRAME_HEIGHT, FRAME_WIDTH, INK};

const TITLE: &str = "Shapes, strokes, gradients, blends and clips";
const DESCRIPTION: &str = "Every fill rule, stroke style, gradient kind, mix and compose mode, clip, group opacity and image paint of the renderer, one per cell";

const LEFT: f32 = 32.0;
const CELL_W: f32 = 150.0;
const STEP: f32 = 154.0;
const ROWS: [f32; 6] = [114.0, 222.0, 330.0, 440.0, 530.0, 622.0];
const CELL_H: f32 = 84.0;
const SMALL_H: f32 = 60.0;

const MIX_NAMES: [&str; 16] = ["Normal", "Multiply", "Screen", "Overlay", "Darken", "Lighten", "ColorDodge", "ColorBurn", "HardLight", "SoftLight", "Difference", "Exclusion", "Hue", "Saturation", "Color", "Luminosity"];
const MIXES: [Mix; 16] = [Mix::Normal, Mix::Multiply, Mix::Screen, Mix::Overlay, Mix::Darken, Mix::Lighten, Mix::ColorDodge, Mix::ColorBurn, Mix::HardLight, Mix::SoftLight, Mix::Difference, Mix::Exclusion, Mix::Hue, Mix::Saturation, Mix::Color, Mix::Luminosity];
const COMPOSE_NAMES: [&str; 14] = ["Clear", "Copy", "Dest", "SrcOver", "DestOver", "SrcIn", "DestIn", "SrcOut", "DestOut", "SrcAtop", "DestAtop", "Xor", "Plus", "PlusLighter"];
const COMPOSES: [Compose; 14] = [Compose::Clear, Compose::Copy, Compose::Dest, Compose::SrcOver, Compose::DestOver, Compose::SrcIn, Compose::DestIn, Compose::SrcOut, Compose::DestOut, Compose::SrcAtop, Compose::DestAtop, Compose::Xor, Compose::Plus, Compose::PlusLighter];
/// Which of (overlap, source only, destination only) each compose leaves drawn.
const COMPOSE_DRAWN: [(bool, bool, bool); 14] = [
    (false, false, false), (true, true, false), (true, false, true), (true, true, true), (true, true, true), (true, false, false), (true, false, false),
    (false, true, false), (false, false, true), (true, false, true), (true, true, false), (false, true, true), (true, true, true), (true, true, true),
];
const MIX_STEP: f32 = 76.0;
const COMPOSE_STEP: f32 = 87.0;

fn rgb(r: u8, g: u8, b: u8) -> Brush {
    Brush::Solid(AlphaColor::<Srgb>::new([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]))
}

fn rgba(r: u8, g: u8, b: u8, a: f32) -> Brush {
    Brush::Solid(AlphaColor::<Srgb>::new([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a]))
}

fn cell(row: usize, col: usize) -> (f32, f32) {
    (LEFT + col as f32 * STEP, ROWS[row])
}

/// A five-pointed star drawn as ONE self-crossing contour (so the fill rule matters).
fn star(cx: f64, cy: f64, r: f64, turn: f64) -> BezPath {
    let mut p = BezPath::new();
    for k in 0..5 {
        let a = turn - std::f64::consts::FRAC_PI_2 + (k * 2 % 5) as f64 * std::f64::consts::TAU / 5.0;
        let pt = Point::new(cx + r * a.cos(), cy + r * a.sin());
        if k == 0 {
            p.move_to(pt);
        } else {
            p.line_to(pt);
        }
    }
    p.close_path();
    p
}

/// Two circles wound the same way (a donut under even-odd, a disc under non-zero).
fn donut(cx: f64, cy: f64, outer: f64, inner: f64) -> BezPath {
    let mut p = BezPath::new();
    p.extend(codebrix_webgpu_animation::kurbo::Shape::path_elements(&Circle::new((cx, cy), outer), 0.01));
    p.extend(codebrix_webgpu_animation::kurbo::Shape::path_elements(&Circle::new((cx, cy), inner), 0.01));
    p
}

fn gradient_stops(g: Gradient, colors: &[(f32, [u8; 3])]) -> Gradient {
    let mut g = g;
    for (offset, c) in colors {
        g.stops.0.push(ColorStop::from((*offset, AlphaColor::<Srgb>::new([c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0]))));
    }
    g
}

fn make_image(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> ImageData {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            data.extend_from_slice(&pixel(x, y));
        }
    }
    ImageData { data: Blob::from(data), format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::Alpha, width, height }
}

/// The cell labels: text, x, baseline, size.
fn cell_labels() -> Vec<(&'static str, f32, f32, f32)> {
    let names: [[&str; 8]; 4] = [
        ["Star, non-zero", "Star, even-odd", "Two circles, even-odd", "Two circles, non-zero", "Rounded rectangle", "Ellipse and circle", "Polygon and polyline", "Path: cubic, dashed"],
        ["Butt cap", "Round cap", "Square cap", "Miter join", "Round join", "Bevel join", "Dashes (marching)", "Thin: 0.35 0.7 1 2 4"],
        ["Linear, pad", "Linear, repeat", "Linear, reflect", "Radial, pad", "Radial, repeat", "Radial, reflect", "Two-point radial", "Sweep, repeat (turning)"],
        ["Clip: rectangle", "Clip: circle", "Clips nested", "Clip: even-odd", "Group at 50%", "Each shape at 50%", "Image paint, repeat", "Image drawn, turning"],
    ];
    let mut labels = Vec::new();
    for (r, row) in [0usize, 1, 2, 5].iter().enumerate() {
        for col in 0..8 {
            let (x, y) = cell(*row, col);
            labels.push((names[r][col], x + 2.0, y - 4.0, 11.0));
        }
    }
    for (k, name) in MIX_NAMES.iter().enumerate() {
        labels.push((*name, LEFT + k as f32 * MIX_STEP + 1.0, ROWS[3] - 4.0, 10.0));
    }
    for (k, name) in COMPOSE_NAMES.iter().enumerate() {
        labels.push((*name, LEFT + k as f32 * COMPOSE_STEP + 1.0, ROWS[4] - 4.0, 10.0));
    }
    labels
}

pub struct ShapesScene {
    session: DrawingSession,
    checker: ImageBrush,
    picture: ImageBrush,
}

impl ShapesScene {
    pub fn new(fonts: &FontRegistry) -> ShapesScene {
        // The basic shapes through the drawing session (row 1, cells 4 to 7).
        let mut session = DrawingSession::new(DrawingSessionOptions { calibration_size: (FRAME_WIDTH, FRAME_HEIGHT), layer_opacity: 255, fonts: Some(fonts.clone()), ..DrawingSessionOptions::default() });
        // The header and the cell labels, drawn by the crate (no backdrop: the sample points tell
        // drawn from empty in the layer's own pixels).
        add_header(&mut session, TITLE, DESCRIPTION, false);
        let labels = session.add_layer("Labels", DIM);
        for (text, x, y, size) in cell_labels() {
            label(labels, fonts, text, x, y, size, DIM, TextAlign::Left);
        }
        let layer = session.add_layer("Session shapes", INK);
        let (x, y) = cell(0, 4);
        layer.add_shape(RectangleShape::new(x + 25.0, y + 12.0, 100.0, 60.0, RectangleShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgb(90, 160, 120)), is_filled: true, corner_radius: 20.0 }).unwrap());
        layer.add_shape(RectangleShape::new(x + 25.0, y + 12.0, 100.0, 60.0, RectangleShapeOptions { stroke_thickness: 4.0, color: Some(INK), corner_radius: 20.0, ..Default::default() }).unwrap());
        let (x, y) = cell(0, 5);
        layer.add_shape(EllipseShape::new(x + 75.0, y + 42.0, 60.0, 30.0, ShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgb(200, 120, 220)), is_filled: true }).unwrap());
        layer.add_shape(CircleShape::new(x + 75.0, y + 42.0, 26.0, ShapeOptions { stroke_thickness: 3.0, color: Some(INK), is_filled: false }).unwrap());
        let (x, y) = cell(0, 6);
        let hexagon: Vec<PointF> = (0..6).map(|k| {
            let a = k as f32 * std::f32::consts::TAU / 6.0;
            PointF::new(x + 75.0 + 34.0 * a.cos(), y + 42.0 + 34.0 * a.sin())
        }).collect();
        layer.add_shape(PolylineShape::new(&hexagon, PolylineShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgb(230, 170, 60)), is_filled: true, ..Default::default() }).unwrap());
        layer.add_shape(PolylineShape::new(&hexagon[..4], PolylineShapeOptions { stroke_thickness: 3.0, color: Some(INK), ..Default::default() }).unwrap());
        let (x, y) = cell(0, 7);
        let mut builder = PathGeometryBuilder::new();
        builder.move_to(x + 20.0, y + 70.0).cubic_to(PointF::new(x + 40.0, y - 10.0), PointF::new(x + 110.0, y + 90.0), PointF::new(x + 130.0, y + 15.0)).line_to(x + 130.0, y + 70.0).close();
        let style = PathStrokeStyle::new(3.0, PathStrokeStyleOptions { dash_pattern: Some(vec![8.0, 4.0]), cap: PathStrokeCap::Butt, ..Default::default() }).unwrap();
        layer.add_shape(PathShape::with_style(builder.build(), style, Some(INK), Some(Color::from_rgb(70, 130, 220)), None));

        let checker = ImageBrush::new(make_image(16, 16, |x, y| if (x / 8 + y / 8) % 2 == 0 { [240, 240, 240, 255] } else { [40, 120, 200, 255] })).with_extend(Extend::Repeat).with_quality(ImageQuality::Low);
        let picture = ImageBrush::new(make_image(32, 32, |x, y| [(x * 8) as u8, (y * 8) as u8, 160, 255])).with_quality(ImageQuality::Medium);
        ShapesScene { session, checker, picture }
    }

    fn draw(&self, s: &mut VectorScene, t: f64, to_canvas: Affine) {
        let at = |x: f32, y: f32| to_canvas * Affine::translate((x as f64, y as f64));
        let frame = Stroke::new(1.0);
        let dim = rgba(132, 138, 164, 0.35);
        // Cell frames (thin outlines; no sample point lies on one).
        for row in [0usize, 1, 2, 5] {
            for col in 0..8 {
                let (x, y) = cell(row, col);
                s.stroke(&frame, at(x, y), &dim, None, &RoundedRect::new(0.0, 0.0, CELL_W as f64, CELL_H as f64, 6.0));
            }
        }
        let turn = t * 0.6;
        // Row 1: fill rules.
        let (x, y) = cell(0, 0);
        s.fill(Fill::NonZero, at(x, y), &rgb(230, 90, 80), None, &star(75.0, 44.0, 38.0, turn));
        let (x, y) = cell(0, 1);
        s.fill(Fill::EvenOdd, at(x, y), &rgb(230, 90, 80), None, &star(75.0, 44.0, 38.0, turn));
        let (x, y) = cell(0, 2);
        s.fill(Fill::EvenOdd, at(x, y), &rgb(80, 180, 230), None, &donut(75.0, 42.0, 36.0, 18.0));
        let (x, y) = cell(0, 3);
        s.fill(Fill::NonZero, at(x, y), &rgb(80, 180, 230), None, &donut(75.0, 42.0, 36.0, 18.0));
        // Row 2: strokes - caps, joins, the miter limit's bevel, dashes, thin lines.
        for (col, cap) in [(0, Cap::Butt), (1, Cap::Round), (2, Cap::Square)] {
            let (x, y) = cell(1, col);
            s.stroke(&Stroke::new(16.0).with_caps(cap), at(x, y), &rgb(240, 200, 90), None, &Line::new((30.0, 42.0), (110.0, 42.0)));
            s.stroke(&Stroke::new(1.0), at(x, y), &rgb(60, 60, 70), None, &Line::new((30.0, 42.0), (110.0, 42.0)));
        }
        for (col, join) in [(3, Join::Miter), (4, Join::Round), (5, Join::Bevel)] {
            let (x, y) = cell(1, col);
            let mut v = BezPath::new();
            v.move_to((20.0, 70.0));
            v.line_to((75.0, 20.0));
            v.line_to((130.0, 70.0));
            s.stroke(&Stroke::new(14.0).with_join(join).with_miter_limit(10.0), at(x, y), &rgb(150, 220, 120), None, &v);
        }
        let (x, y) = cell(1, 6);
        s.stroke(&Stroke::new(6.0).with_caps(Cap::Butt).with_dashes(0.0, [20.0, 20.0]), at(x, y), &rgb(240, 140, 200), None, &Line::new((20.0, 20.0), (130.0, 20.0)));
        s.stroke(&Stroke::new(4.0).with_caps(Cap::Round).with_dashes(t * 24.0, [10.0, 6.0]), at(x, y), &rgb(240, 140, 200), None, &Circle::new((75.0, 56.0), 22.0));
        let (x, y) = cell(1, 7);
        for (k, w) in [0.35f64, 0.7, 1.0, 2.0, 4.0].iter().enumerate() {
            let yy = 14.0 + k as f64 * 14.0;
            s.stroke(&Stroke::new(*w), at(x, y), &rgb(230, 230, 240), None, &Line::new((20.0, yy - 6.0), (130.0, yy + 6.0)));
        }
        // Row 3: gradients.
        let stops3 = [(0.0, [230, 60, 60]), (0.5, [240, 220, 80]), (1.0, [60, 90, 230])];
        for (col, extend) in [(0, Extend::Pad), (1, Extend::Repeat), (2, Extend::Reflect)] {
            let (x, y) = cell(2, col);
            let g = gradient_stops(Gradient::new_linear((40.0, 0.0), (110.0, 0.0)), &stops3).with_extend(extend);
            s.fill(Fill::NonZero, at(x, y), &Brush::Gradient(g), None, &Rect::new(8.0, 10.0, 142.0, 74.0));
        }
        for (col, extend) in [(3, Extend::Pad), (4, Extend::Repeat), (5, Extend::Reflect)] {
            let (x, y) = cell(2, col);
            let g = gradient_stops(Gradient::new_radial((75.0, 42.0), 25.0), &stops3).with_extend(extend);
            s.fill(Fill::NonZero, at(x, y), &Brush::Gradient(g), None, &Rect::new(8.0, 4.0, 142.0, 80.0));
        }
        let (x, y) = cell(2, 6);
        let g = gradient_stops(Gradient::new_two_point_radial((55.0, 34.0), 4.0, (75.0, 42.0), 38.0), &[(0.0, [255, 255, 255]), (0.4, [120, 200, 255]), (1.0, [20, 30, 90])]);
        s.fill(Fill::NonZero, at(x, y), &Brush::Gradient(g), None, &Circle::new((75.0, 42.0), 38.0));
        let (x, y) = cell(2, 7);
        let a0 = (t * 0.8) as f32;
        // Repeat: the angles before the (turning) start angle wrap round (pad would hold the first
        // colour there).
        let g = gradient_stops(Gradient::new_sweep((75.0, 42.0), a0, a0 + std::f32::consts::TAU), &[(0.0, [230, 60, 60]), (0.33, [240, 220, 80]), (0.66, [60, 200, 120]), (1.0, [230, 60, 60])]).with_extend(Extend::Repeat);
        s.fill(Fill::NonZero, at(x, y), &Brush::Gradient(g), None, &Circle::new((75.0, 42.0), 38.0));
        // Row 4: mix modes - a red backdrop, a blue disc in a group with the mode.
        for (k, mix) in MIXES.iter().enumerate() {
            let (x, y) = (LEFT + k as f32 * MIX_STEP, ROWS[3]);
            let place = at(x, y);
            s.fill(Fill::NonZero, place, &rgb(200, 60, 40), None, &Rect::new(0.0, 0.0, 72.0, SMALL_H as f64));
            s.fill(Fill::NonZero, place, &rgb(240, 220, 200), None, &Rect::new(0.0, 0.0, 72.0, 16.0));
            s.push_layer(*mix, 1.0, place, &Rect::new(0.0, 0.0, 72.0, SMALL_H as f64));
            s.fill(Fill::NonZero, place, &rgb(60, 170, 220), None, &Circle::new((36.0, 32.0), 24.0));
            s.fill(Fill::NonZero, place, &rgba(250, 250, 120, 0.8), None, &Circle::new((50.0, 44.0), 10.0));
            s.pop_layer();
        }
        // Row 5: compose modes - a blue disc (destination), then an orange square (source) in a
        // group with the mode, both inside an isolated group.
        for (k, compose) in COMPOSES.iter().enumerate() {
            let (x, y) = (LEFT + k as f32 * COMPOSE_STEP, ROWS[4]);
            let place = at(x, y);
            let area = Rect::new(0.0, 0.0, 82.0, SMALL_H as f64);
            s.push_layer(BlendMode::default(), 1.0, place, &area);
            s.fill(Fill::NonZero, place, &rgb(60, 110, 230), None, &Circle::new((32.0, 26.0), 20.0));
            s.push_layer(*compose, 1.0, place, &area);
            s.fill(Fill::NonZero, place, &rgb(240, 150, 50), None, &Rect::new(36.0, 22.0, 74.0, 56.0));
            s.fill(Fill::NonZero, place, &rgb(240, 150, 50), None, &Rect::new(36.0, 22.0, 37.0, 23.0));
            s.pop_layer();
            s.pop_layer();
        }
        // Row 6: clips, group opacity, images.
        let (x, y) = cell(5, 0);
        s.push_clip_layer(at(x, y), &Rect::new(50.0, 27.0, 100.0, 57.0));
        s.fill(Fill::NonZero, at(x, y), &rgb(230, 120, 60), None, &Circle::new((75.0, 42.0), 40.0));
        s.pop_layer();
        let (x, y) = cell(5, 1);
        s.push_clip_layer(at(x, y), &Circle::new((75.0, 42.0), 30.0));
        for k in 0..8 {
            s.fill(Fill::NonZero, at(x, y), &(if k % 2 == 0 { rgb(240, 240, 240) } else { rgb(120, 80, 200) }), None, &Rect::new(10.0 + k as f64 * 16.0, 0.0, 26.0 + k as f64 * 16.0, 84.0));
        }
        s.pop_layer();
        let (x, y) = cell(5, 2);
        s.push_clip_layer(at(x, y), &Rect::new(30.0, 10.0, 120.0, 60.0));
        s.push_clip_layer(at(x, y), &Circle::new((75.0, 50.0), 34.0));
        s.fill(Fill::NonZero, at(x, y), &rgb(90, 200, 160), None, &Rect::new(0.0, 0.0, 150.0, 84.0));
        s.pop_layer();
        s.pop_layer();
        let (x, y) = cell(5, 3);
        s.push_clip_layer_with_rule(Fill::EvenOdd, at(x, y), &donut(75.0, 42.0, 36.0, 18.0));
        s.fill(Fill::NonZero, at(x, y), &Brush::Gradient(gradient_stops(Gradient::new_linear((0.0, 0.0), (150.0, 0.0)), &stops3)), None, &Rect::new(0.0, 0.0, 150.0, 84.0));
        s.pop_layer();
        let (x, y) = cell(5, 4);
        s.push_layer(BlendMode::default(), 0.5, at(x, y), &Rect::new(0.0, 0.0, CELL_W as f64, CELL_H as f64));
        s.fill(Fill::NonZero, at(x, y), &rgb(230, 70, 70), None, &Circle::new((60.0, 42.0), 28.0));
        s.fill(Fill::NonZero, at(x, y), &rgb(70, 110, 230), None, &Circle::new((90.0, 42.0), 28.0));
        s.pop_layer();
        let (x, y) = cell(5, 5);
        s.fill(Fill::NonZero, at(x, y), &rgba(230, 70, 70, 0.5), None, &Circle::new((60.0, 42.0), 28.0));
        s.fill(Fill::NonZero, at(x, y), &rgba(70, 110, 230, 0.5), None, &Circle::new((90.0, 42.0), 28.0));
        let (x, y) = cell(5, 6);
        s.fill(Fill::NonZero, at(x, y), &Brush::Image(self.checker.clone()), Some(Affine::scale(1.5)), &RoundedRect::new(10.0, 8.0, 140.0, 76.0, 12.0));
        let (x, y) = cell(5, 7);
        s.draw_image(&self.picture, at(x, y) * Affine::translate((75.0, 42.0)) * Affine::rotate(t * 0.5) * Affine::scale(1.6) * Affine::translate((-16.0, -16.0)));
    }
}

impl DemoScene for ShapesScene {
    fn name(&self) -> &'static str {
        "shapes"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        10.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let to_canvas = self.session.calibration_transform(width, height);
        self.session.set_time(seconds);
        // Rows 1-3 and 6 with VectorScene calls; the session's basic shapes in row 1.
        self.draw(scene, seconds, to_canvas);
        self.session.render(scene, width, height);
    }

    fn checks(&self) -> Vec<SamplePoint> {
        let mut p = Vec::new();
        let mut add = |name: &'static str, row: usize, col: usize, dx: f32, dy: f32, expect: Expect| {
            let (x, y) = cell(row, col);
            p.push(SamplePoint { name, x: x + dx, y: y + dy, expect });
            p.len() - 1
        };
        add("star non-zero: centre filled", 0, 0, 75.0, 44.0, Expect::Drawn);
        add("star even-odd: centre empty", 0, 1, 75.0, 44.0, Expect::Empty);
        add("circles even-odd: hole empty", 0, 2, 75.0, 42.0, Expect::Empty);
        add("circles even-odd: ring filled", 0, 2, 75.0 + 27.0, 42.0, Expect::Drawn);
        add("circles non-zero: centre filled", 0, 3, 75.0, 42.0, Expect::Drawn);
        add("rounded rectangle: centre", 0, 4, 75.0, 42.0, Expect::Drawn);
        add("rounded rectangle: cut corner", 0, 4, 28.0, 15.0, Expect::Empty);
        add("ellipse: inside", 0, 5, 75.0 + 45.0, 42.0, Expect::Drawn);
        add("polygon: centre", 0, 6, 75.0, 42.0, Expect::Drawn);
        add("path: filled", 0, 7, 124.0, 62.0, Expect::Drawn);
        add("butt cap: past the end empty", 1, 0, 115.0, 42.0, Expect::Empty);
        add("round cap: past the end drawn", 1, 1, 115.0, 42.0, Expect::Drawn);
        add("round cap: corner empty", 1, 1, 116.0, 48.5, Expect::Empty);
        add("square cap: corner drawn", 1, 2, 116.0, 48.5, Expect::Drawn);
        add("miter join: tip drawn", 1, 3, 75.0, 12.5, Expect::Drawn);
        add("round join: rounded tip drawn", 1, 4, 75.0, 14.0, Expect::Drawn);
        add("round join: beyond the round empty", 1, 4, 75.0, 12.0, Expect::Empty);
        add("bevel join: cut tip empty", 1, 5, 75.0, 13.5, Expect::Empty);
        add("dashes: first dash drawn", 1, 6, 30.0, 20.0, Expect::Drawn);
        add("dashes: first gap empty", 1, 6, 50.0, 20.0, Expect::Empty);
        add("thin lines: 4 wide drawn", 1, 7, 75.0, 70.0, Expect::Drawn);
        let pad = add("linear pad: past the end", 2, 0, 130.0, 42.0, Expect::Drawn);
        let repeat = add("linear repeat: past the end differs from pad", 2, 1, 130.0, 42.0, Expect::Differs(pad));
        add("linear reflect: past the end differs from repeat", 2, 2, 130.0, 42.0, Expect::Differs(repeat));
        let rpad = add("radial pad: outside", 2, 3, 75.0 + 35.0, 42.0, Expect::Drawn);
        let rrep = add("radial repeat: outside differs from pad", 2, 4, 75.0 + 35.0, 42.0, Expect::Differs(rpad));
        add("radial reflect: outside differs from repeat", 2, 5, 75.0 + 35.0, 42.0, Expect::Differs(rrep));
        let focal = add("two-point radial: near the focus", 2, 6, 56.0, 35.0, Expect::Drawn);
        add("two-point radial: edge differs", 2, 6, 75.0 + 32.0, 42.0, Expect::Differs(focal));
        let sweep = add("sweep: one side", 2, 7, 75.0 + 25.0, 42.0, Expect::Drawn);
        add("sweep: other side differs", 2, 7, 75.0 - 25.0, 42.0, Expect::Differs(sweep));
        let normal = p.len();
        p.push(SamplePoint { name: "mix Normal: disc", x: LEFT + 36.0, y: ROWS[3] + 32.0, expect: Expect::Drawn });
        for (k, name) in MIX_NAMES.iter().enumerate().skip(1) {
            p.push(SamplePoint { name, x: LEFT + k as f32 * MIX_STEP + 36.0, y: ROWS[3] + 32.0, expect: Expect::Differs(normal) });
        }
        for (k, name) in COMPOSE_NAMES.iter().enumerate() {
            let x = LEFT + k as f32 * COMPOSE_STEP;
            let (overlap, src, dst) = COMPOSE_DRAWN[k];
            let e = |drawn: bool| if drawn { Expect::Drawn } else { Expect::Empty };
            p.push(SamplePoint { name, x: x + 44.0, y: ROWS[4] + 30.0, expect: e(overlap) });
            p.push(SamplePoint { name, x: x + 66.0, y: ROWS[4] + 50.0, expect: e(src) });
            p.push(SamplePoint { name, x: x + 20.0, y: ROWS[4] + 20.0, expect: e(dst) });
        }
        let mut add = |name: &'static str, row: usize, col: usize, dx: f32, dy: f32, expect: Expect| {
            let (x, y) = cell(row, col);
            p.push(SamplePoint { name, x: x + dx, y: y + dy, expect });
            p.len() - 1
        };
        add("rectangle clip: inside", 5, 0, 75.0, 42.0, Expect::Drawn);
        add("rectangle clip: disc outside the clip empty", 5, 0, 75.0, 68.0, Expect::Empty);
        add("circle clip: inside", 5, 1, 75.0, 42.0, Expect::Drawn);
        add("circle clip: corner empty", 5, 1, 50.0, 16.0, Expect::Empty);
        add("nested clips: inside both", 5, 2, 75.0, 40.0, Expect::Drawn);
        add("nested clips: in the rectangle, outside the circle", 5, 2, 36.0, 14.0, Expect::Empty);
        add("even-odd clip: hole empty", 5, 3, 75.0, 42.0, Expect::Empty);
        add("even-odd clip: ring drawn", 5, 3, 75.0 + 27.0, 42.0, Expect::Drawn);
        let group = add("group at 50%: overlap is half covered", 5, 4, 75.0, 42.0, Expect::Translucent);
        add("each at 50%: overlap differs from the group's", 5, 5, 75.0, 42.0, Expect::Differs(group));
        let a = add("image paint: one square", 5, 6, 20.0, 18.0, Expect::Drawn);
        add("image paint: the next square differs", 5, 6, 20.0 + 12.0, 18.0, Expect::Differs(a));
        add("image drawn: centre", 5, 7, 75.0, 42.0, Expect::Drawn);
        p
    }
}
