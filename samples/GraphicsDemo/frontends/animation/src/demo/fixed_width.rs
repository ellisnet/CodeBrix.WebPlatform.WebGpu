//! "fixed-width" (a feature of the second GO, Not in C#): the same drawing scaled and squashed,
//! with strokes that scale with the element (top row, the default) beside fixed-width strokes
//! (bottom row, `StrokeWidthMode::Fixed`): a star, a dashed circle squashed by an uneven scale, a
//! text block's outline and a square whose top edge the browser check measures; on the right an
//! SVG with a 10 x 10 viewBox drawn large, one of its two lines marked
//! `vector-effect="non-scaling-stroke"`. Every shape's scale is a bound track (0.5 -> the chosen
//! amount -> 0.5 every 4 s); the camera's zoom (a control) scales BOTH rows alike - it is the
//! view, not the drawing.

use std::sync::Arc;

use codebrix_webgpu_animation::animation::{EasingKind, FloatTrack, TrackLoopMode};
use codebrix_webgpu_animation::models::{AnimatedProperty, DrawingCamera, DrawingTransform};
use codebrix_webgpu_animation::paths::{PathGeometry, PathStrokeStyle, PathStrokeStyleOptions, StrokeWidthMode};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{DrawingShape, PathShape, TextBlockShape, TextBlockStrokeOptions};
use codebrix_webgpu_animation::svg::{DrawingSvg, DrawingSvgAspect, DrawingSvgOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::{create_frame_session, label, Control, DemoScene, Readout, Settings, ACCENT, DIM, INK};

const TITLE: &str = "Fixed-width strokes";
const DESCRIPTION: &str = "Top row: strokes scale with the element (default). Bottom row: StrokeWidthMode::Fixed - the same width in drawing units whatever the element's scale";
/// The rows' centres (frame units).
pub const ROWS: [f32; 2] = [262.0, 522.0];
/// The measured square's centre x and half size; its stroke is 4 units wide.
pub const SQUARE_X: f32 = 860.0;
const SQUARE_HALF: f32 = 22.0;
const WIDTH: f32 = 4.0;

const CONTROLS: &[Control] = &[
    Control::Slider { key: "scale", label: "Largest scale", min: 1.0, max: 4.0, step: 0.25, default: 3.0 },
    Control::Toggle { key: "swing", label: "Scale swings with the clock", default: true },
    Control::Slider { key: "still-scale", label: "Scale when not swinging", min: 0.25, max: 4.0, step: 0.25, default: 2.0 },
    Control::Slider { key: "zoom", label: "Camera zoom (the view)", min: 0.5, max: 2.0, step: 0.1, default: 1.0 },
];

const LEGEND: &[&str] = &[
    "Bottom row: the lines stay the same thickness (and the dashes the same length) while the shapes grow, shrink and stretch.",
    "Top row: the lines thicken and thin with the shape and go uneven under the squashed circle.",
    "Camera zoom thickens BOTH rows alike (it is the view of the drawing).",
    "Right: the SVG's marked line (vector-effect=\"non-scaling-stroke\") stays thin; the other scales with the viewBox.",
];

const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
<rect x="0.5" y="0.5" width="9" height="9" fill="#1c2236" stroke="#5a6488" stroke-width="0.1"/>
<line x1="1" y1="3" x2="9" y2="3" stroke="#ffb048" stroke-width="0.6"/>
<line x1="1" y1="7" x2="9" y2="7" stroke="#8fd3ff" stroke-width="0.6" vector-effect="non-scaling-stroke"/>
</svg>"##;

pub struct FixedWidthScene {
    session: DrawingSession,
    fonts: FontRegistry,
    settings: Settings,
    scale_now: f32,
}

fn style(mode: StrokeWidthMode, dashes: bool) -> PathStrokeStyle {
    PathStrokeStyle::new(WIDTH, PathStrokeStyleOptions { width_mode: mode, dash_pattern: dashes.then(|| vec![10.0, 6.0]), ..Default::default() }).unwrap()
}

/// The swing of the scale track: 0.5 -> `most` -> 0.5 every 4 s, looping.
fn swing(most: f32) -> FloatTrack {
    FloatTrack::from_keys(&[(0.0, 0.5, EasingKind::Linear), (2.0, most, EasingKind::SineInOut), (4.0, 0.5, EasingKind::SineInOut)]).unwrap().with_loop_mode(TrackLoopMode::Loop)
}

fn constant(value: f32) -> FloatTrack {
    FloatTrack::from_keys(&[(0.0, value, EasingKind::Linear)]).unwrap()
}

/// The scale the tracks give at a time (for the read-out).
fn scale_at(settings: &Settings, seconds: f64) -> f32 {
    if !settings.on("swing") {
        return settings.get("still-scale");
    }
    swing(settings.get("scale")).evaluate(seconds).unwrap_or(1.0)
}

fn build(fonts: &FontRegistry, settings: &Settings) -> DrawingSession {
    let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
    let labels = session.add_layer("Labels", DIM);
    label(labels, fonts, "ScalesWithElement (the default)", 40.0, ROWS[0] - 120.0, 15.0, INK, TextAlign::Left);
    label(labels, fonts, "StrokeWidthMode::Fixed", 40.0, ROWS[1] - 120.0, 15.0, ACCENT, TextAlign::Left);
    for (x, text) in [(190.0, "star"), (420.0, "dashed circle, uneven scale"), (640.0, "text outline"), (SQUARE_X, "square (measured)")] {
        label(labels, fonts, text, x, ROWS[1] + 128.0, 13.0, DIM, TextAlign::Center);
    }
    label(labels, fonts, "SVG, viewBox 10 x 10 drawn 240 x 240", 1110.0, 168.0, 13.0, DIM, TextAlign::Center);
    label(labels, fonts, "orange: scales with the viewBox", 1110.0, 440.0, 13.0, DIM, TextAlign::Center);
    label(labels, fonts, "blue: vector-effect=\"non-scaling-stroke\"", 1110.0, 460.0, 13.0, DIM, TextAlign::Center);
    let ink = Color::from_rgb(236, 236, 244);
    session.add_layer("Shapes", ink);
    let layout = session.layout_text("Fixed", 40.0, None, None).unwrap();
    let mut elements = Vec::new();
    for (row, mode) in [(0usize, StrokeWidthMode::ScalesWithElement), (1, StrokeWidthMode::Fixed)] {
        let y = ROWS[row];
        let color = if row == 0 { ink } else { ACCENT };
        let star = PathGeometry::star(PointF::new(190.0, y), 30.0, 13.0, 5, 0.0).unwrap();
        let star = session.draw_shape(DrawingShape::from(PathShape::with_style(star, style(mode, false), Some(color), None, None)).with_transform(DrawingTransform::scale(1.0, PointF::new(190.0, y))));
        let circle = PathGeometry::circle(PointF::new(420.0, y), 28.0);
        let circle = session.draw_shape(DrawingShape::from(PathShape::with_style(circle, style(mode, true), Some(color), None, None)).with_transform(DrawingTransform::scale(1.0, PointF::new(420.0, y))));
        let text = TextBlockShape::with_stroke(layout.clone(), 640.0 - layout.width() / 2.0, y - layout.height() / 2.0, style(mode, false).with_width(1.5).ok(), TextBlockStrokeOptions { stroke_color: Some(color), ..Default::default() }).unwrap();
        let text = session.draw_shape(DrawingShape::from(text).with_transform(DrawingTransform::scale(1.0, PointF::new(640.0, y))));
        let square = PathGeometry::rectangle(RectangleF::new(SQUARE_X - SQUARE_HALF, y - SQUARE_HALF, 2.0 * SQUARE_HALF, 2.0 * SQUARE_HALF));
        let square = session.draw_shape(DrawingShape::from(PathShape::with_style(square, style(mode, false), Some(color), None, None)).with_transform(DrawingTransform::scale(1.0, PointF::new(SQUARE_X, y))));
        elements.push((star, circle, text, square));
    }
    let track = || if settings.on("swing") { swing(settings.get("scale")) } else { constant(settings.get("still-scale")) };
    for (star, circle, text, square) in elements {
        let mut animations = session.animations_mut();
        animations.bind(star, AnimatedProperty::Scale, track()).unwrap();
        animations.bind(text, AnimatedProperty::Scale, track()).unwrap();
        animations.bind(square, AnimatedProperty::Scale, track()).unwrap();
        // The circle is squashed: its width follows the swing, its height stays 1.
        animations.bind(circle, AnimatedProperty::ScaleX, track()).unwrap();
        animations.bind(circle, AnimatedProperty::ScaleY, constant(1.0)).unwrap();
    }
    session.add_layer("Svg", ink);
    let svg = Arc::new(DrawingSvg::from_svg(SVG, &DrawingSvgOptions::default()).unwrap());
    session.draw_svg(&svg, RectangleF::new(990.0, 180.0, 240.0, 240.0), DrawingSvgAspect::Meet);
    let zoom = settings.get("zoom");
    if (zoom - 1.0).abs() > 1e-3 {
        let mut camera = DrawingCamera::new();
        camera.set_zoom(zoom).unwrap();
        session.set_camera(Some(camera));
    }
    session
}

impl FixedWidthScene {
    pub fn new(fonts: &FontRegistry) -> FixedWidthScene {
        let settings = Settings::from_controls(CONTROLS);
        FixedWidthScene { session: build(fonts, &settings), fonts: fonts.clone(), settings, scale_now: 0.5 }
    }
}

impl DemoScene for FixedWidthScene {
    fn name(&self) -> &'static str {
        "fixed-width"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        12.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.scale_now = scale_at(&self.settings, seconds);
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }

    fn controls(&self) -> &'static [Control] {
        CONTROLS
    }

    fn apply_settings(&mut self, settings: &Settings) {
        self.settings = settings.clone();
        self.session = build(&self.fonts, settings);
    }

    fn legend(&self) -> &'static [&'static str] {
        LEGEND
    }

    fn readouts(&self) -> Vec<Readout> {
        vec![
            Readout::new("scale-now", "Element scale now", format!("{:.3}", self.scale_now)),
            Readout::new("expected", "Square's top edge, frame units (scaling / fixed)", format!("{:.2} / {:.2}", WIDTH * self.scale_now, WIDTH)),
            Readout::new("square", "Measured square: centre x, rows' centre y (frame units)", format!("{SQUARE_X} / {} {}", ROWS[0], ROWS[1])),
        ]
    }
}
