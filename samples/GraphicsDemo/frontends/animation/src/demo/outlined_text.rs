//! "outlined-text" (AnimationDemoApp's OutlinedTextScene): glyphs as paths. Top, outlined text (a
//! stroke only, outside the glyphs) and gradient-filled text with an outline; middle, the glyph
//! outlines taken into a PathShape and drawn on by trimming, then filled in; bottom, a line of
//! text set along a wave (each glyph turned to the path's direction), sliding along it.

use codebrix_webgpu_animation::animation::EasingKind;
use codebrix_webgpu_animation::fills::LinearGradientFill;
use codebrix_webgpu_animation::models::AnimatedProperty;
use codebrix_webgpu_animation::paths::{PathGeometry, PathStrokeCap, PathStrokeJoin, PathStrokeStyle, PathStrokeStyleOptions};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{AnimatedShape, DrawingShape, PathShape, TextBlockShape, TextBlockStrokeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign, TextStrokePlacement};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::kit::{floats, L};
use crate::demo::pictures::ping_pong;
use crate::demo::{create_frame_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Outlined, gradient and drawn-on text";
const DESCRIPTION: &str = "Glyph outlines as PathGeometry: stroke and gradient fill, draw-on by trimming, text along a path";
const WAVE: &str = "M 40 620 C 220 480, 420 480, 640 600 S 1060 720, 1240 560";

pub struct OutlinedTextScene {
    session: DrawingSession,
}

fn style(width: f32) -> PathStrokeStyle {
    PathStrokeStyle::new(width, PathStrokeStyleOptions::default()).unwrap()
}

impl OutlinedTextScene {
    pub fn new(fonts: &FontRegistry) -> OutlinedTextScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = session.add_layer("Notes", DIM);
        label(notes, fonts, "a stroke only, StrokePlacement.Outside (no overlap lines)", 40.0, 128.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "a LinearGradientFill inside a 4 px outside outline", 680.0, 128.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "GetOutline() into a PathShape: TrimEnd 0 -> 1 traces every contour, then the fill fades in", 40.0, 300.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "GetOutlineAlongPath(wave, distance): glyphs turned by the path's tangent", 40.0, 470.0, 13.0, DIM, TextAlign::Left);
        session.add_layer("Text", INK);
        let outline = session.layout_text("OUTLINE", 100.0, None, None).unwrap();
        let mut stroked = TextBlockShape::with_stroke(outline, 40.0, 136.0, Some(style(5.0)), TextBlockStrokeOptions { stroke_color: Some(INK), ..TextBlockStrokeOptions::default() }).unwrap();
        stroked.set_stroke_placement(TextStrokePlacement::Outside);
        session.draw_shape(stroked);
        let gradient = session.layout_text("Gradient", 100.0, None, None).unwrap();
        let fill = LinearGradientFill::from_colors(PointF::new(680.0, 150.0), PointF::new(1120.0, 250.0), Color::from_rgb(255, 176, 72), Color::from_rgb(236, 90, 160)).unwrap();
        let round = PathStrokeStyle::new(8.0, PathStrokeStyleOptions { cap: PathStrokeCap::Round, join: PathStrokeJoin::Round, ..PathStrokeStyleOptions::default() }).unwrap();
        let mut filled = TextBlockShape::with_stroke(gradient, 680.0, 136.0, Some(round), TextBlockStrokeOptions { stroke_color: Some(Color::from_rgb(250, 240, 255)), fill_color: None, fill: Some(fill.into()) }).unwrap();
        filled.set_stroke_placement(TextStrokePlacement::Outside);
        session.draw_shape(filled);
        // Draw-on: the outlines are paths, so the trim writes them out.
        let draw_on = session.layout_text("Draw on, then fill", 92.0, None, None).unwrap();
        let mut strokes = PathShape::with_style(draw_on.get_outline_at(40.0, 304.0), style(2.0), Some(Color::from_rgb(140, 200, 255)), None, None);
        strokes.set_trim_end(0.0).unwrap();
        let strokes = session.draw_path(strokes);
        session.animations_mut().bind(strokes, AnimatedProperty::TrimEnd, floats(&[(0.3, 0.0, L), (3.8, 1.0, EasingKind::SineInOut)])).unwrap();
        let block = DrawingShape::from(TextBlockShape::new(draw_on, 40.0, 304.0, Some(Color::from_rgb(140, 200, 255))).unwrap()).with_opacity(0.0);
        let block = session.draw_shape(block);
        session.animations_mut().bind(block, AnimatedProperty::Opacity, floats(&[(3.6, 0.0, L), (4.6, 1.0, EasingKind::CubicOut)])).unwrap();
        // Text on a path, sliding along it.
        let wave = PathGeometry::parse(WAVE).unwrap();
        session.draw_path(PathShape::with_style(wave.clone(), style(1.5), Some(Color::from_rgba(132, 138, 164, 90)), None, None));
        let along = session.layout_text("letters ride the curve, each one turned to follow it", 30.0, None, None).unwrap();
        let travel = wave.get_length() - along.width() - 20.0;
        let along_fill = LinearGradientFill::from_colors(PointF::new(40.0, 0.0), PointF::new(1240.0, 0.0), Color::from_rgb(140, 230, 160), Color::from_rgb(255, 176, 72)).unwrap();
        session.draw_shape(AnimatedShape::new(
            move |t| {
                let outline = along.get_outline_along_path(&wave, 10.0 + (ping_pong(t, 8.0) as f32 * travel), -6.0).ok()?;
                Some(PathShape::with_fill(outline, along_fill.clone().into()).into())
            },
            0.0,
        ));
        OutlinedTextScene { session }
    }
}

impl DemoScene for OutlinedTextScene {
    fn name(&self) -> &'static str {
        "outlined-text"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
