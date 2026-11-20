//! Short constructors the scenes share (the C# scenes write `new
//! RectangleShape(x, y, w, h, 1, color, isFilled: true, cornerRadius: 4)` and the like inline;
//! these say the same with the crate's option structs).

use codebrix_webgpu_animation::animation::{EasingKind, FloatTrack, PointTrack};
use codebrix_webgpu_animation::shapes::{CircleShape, LineShape, LineShapeOptions, PolylineShape, PolylineShapeOptions, RectangleShape, RectangleShapeOptions, ShapeOptions};
use codebrix_webgpu_animation::{Color, PointF};

/// A filled rectangle (C# `new RectangleShape(x, y, w, h, 1, color, isFilled: true, cornerRadius: r)`).
pub fn fill_rect(x: f32, y: f32, w: f32, h: f32, color: Option<Color>, corner_radius: f32) -> RectangleShape {
    RectangleShape::new(x, y, w, h, RectangleShapeOptions { stroke_thickness: 1.0, color, is_filled: true, corner_radius }).unwrap()
}

/// An outlined rectangle.
pub fn outline_rect(x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Option<Color>, corner_radius: f32) -> RectangleShape {
    RectangleShape::new(x, y, w.max(0.001), h.max(0.001), RectangleShapeOptions { stroke_thickness: thickness, color, is_filled: false, corner_radius }).unwrap()
}

/// A filled circle.
pub fn disc(x: f32, y: f32, r: f32, color: Option<Color>) -> CircleShape {
    CircleShape::new(x, y, r, ShapeOptions { stroke_thickness: 1.0, color, is_filled: true }).unwrap()
}

/// An outlined circle.
pub fn ring(x: f32, y: f32, r: f32, thickness: f32, color: Option<Color>) -> CircleShape {
    CircleShape::new(x, y, r, ShapeOptions { stroke_thickness: thickness, color, is_filled: false }).unwrap()
}

/// A line.
pub fn line(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Option<Color>) -> LineShape {
    LineShape::new(x1, y1, x2, y2, LineShapeOptions { stroke_thickness: thickness, color }).unwrap()
}

/// A filled closed polygon.
pub fn polygon(points: &[PointF], color: Option<Color>) -> PolylineShape {
    PolylineShape::new(points, PolylineShapeOptions { stroke_thickness: 1.0, color, is_closed: true, is_filled: true }).unwrap()
}

/// A float track from (time, value, easing) keys.
pub fn floats(keys: &[(f64, f32, EasingKind)]) -> FloatTrack {
    FloatTrack::from_keys(keys).unwrap()
}

/// A point track from (time, x, y, easing) keys.
pub fn points(keys: &[(f64, f32, f32, EasingKind)]) -> PointTrack {
    let keys: Vec<(f64, PointF, EasingKind)> = keys.iter().map(|(t, x, y, e)| (*t, PointF::new(*x, *y), *e)).collect();
    PointTrack::from_keys(&keys).unwrap()
}

/// Linear easing, for short key lists.
pub const L: EasingKind = EasingKind::Linear;

/// A checkerboard of 20-unit cells over a rectangle (DemoScene.CreateCheckerboard's look), on a
/// layer.
pub fn checkerboard(layer: &mut codebrix_webgpu_animation::models::DrawingLayer, x: f32, y: f32, w: f32, h: f32) {
    layer.add_shape(fill_rect(x, y, w, h, Some(Color::from_rgb(70, 72, 84)), 0.0));
    let mut row = 0.0;
    while row < h {
        let mut col = ((row / 20.0) as i32 % 2) as f32 * 20.0;
        while col < w {
            layer.add_shape(fill_rect(x + col, y + row, 20.0f32.min(w - col), 20.0f32.min(h - row), Some(Color::from_rgb(110, 112, 124)), 0.0));
            col += 40.0;
        }
        row += 20.0;
    }
}

/// The canvas-pixel rectangle of a box in a frame session's units (where a second session is
/// drawn into the frame, as the C# scenes copy an exported session's pixels there).
pub fn part(frame: &codebrix_webgpu_animation::DrawingSession, x: f32, y: f32, w: f32, h: f32, width: u32, height: u32) -> codebrix_webgpu_animation::RectangleF {
    let to_canvas = frame.calibration_transform(width, height);
    let a = to_canvas * codebrix_webgpu_animation::kurbo::Point::new(x as f64, y as f64);
    let b = to_canvas * codebrix_webgpu_animation::kurbo::Point::new((x + w) as f64, (y + h) as f64);
    codebrix_webgpu_animation::RectangleF::new(a.x as f32, a.y as f32, (b.x - a.x) as f32, (b.y - a.y) as f32)
}
