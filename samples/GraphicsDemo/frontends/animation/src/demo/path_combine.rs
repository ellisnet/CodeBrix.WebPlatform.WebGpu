//! "path-combine" (AnimationDemoApp's PathCombineScene): paths as areas, worked out again on every
//! frame. Top, a circle slides across a star and the four boolean combinations of the two are
//! filled; middle left, a stroke turned into a fillable outline while it draws on (filled with a
//! gradient, its polygon edge traced in white); middle right, a star inflated and deflated over
//! fixed offset rings; bottom, outlined Roboto text with a centred stroke - as the font draws it
//! (the lines where a glyph's strokes overlap show) and with merged overlaps (clean outlines).
//! Combining is i_overlay's integer overlay on NoSkia's grid: the same areas as C#, contours that
//! start at other points.

use codebrix_webgpu_animation::fills::LinearGradientFill;
use codebrix_webgpu_animation::paths::{PathGeometry, PathInflateOptions, PathStrokeCap, PathStrokeJoin, PathStrokeStyle, PathStrokeStyleOptions};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{AnimatedShape, DrawingGroup, DrawingShape, PathShape, ShapeOptions, TextBlockShape, TextBlockStrokeOptions, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::pictures::ping_pong;
use crate::demo::{create_frame_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Combining paths, stroke to outline, inflate";
const DESCRIPTION: &str = "PathGeometry Union / Intersect / Subtract / ExclusiveOr, ToStrokeOutline, Inflate / Deflate and merged glyph outlines - exact integer polygon arithmetic";
const SIGNATURE: &str = "M 60 520 C 110 400, 210 390, 250 470 S 360 570, 420 470 Q 470 390 530 460 T 610 470";
const OPERATIONS: [&str; 4] = ["Union", "Intersect", "Subtract", "ExclusiveOr"];
const RESULT: Color = Color::from_rgb(255, 176, 72);
const INPUT: Color = Color::from_rgba(200, 205, 230, 150);

pub struct PathCombineScene {
    session: DrawingSession,
}

fn outlined(geometry: PathGeometry, thickness: f32, color: Color) -> PathShape {
    PathShape::new(geometry, ShapeOptions { stroke_thickness: thickness, color: Some(color), is_filled: false }).unwrap()
}

fn filled(geometry: PathGeometry, color: Color) -> PathShape {
    PathShape::new(geometry, ShapeOptions { stroke_thickness: 1.0, color: Some(color), is_filled: true }).unwrap()
}

/// Grows from -16 (deflated) to +26 and back over the scene.
fn distance(t: f64) -> f32 {
    -16.0 + 42.0 * ping_pong(t, 8.0) as f32
}

/// One boolean combination of a star and a circle that slides across it.
fn combined(operation: usize, cx: f32, t: f64) -> Option<DrawingShape> {
    let star = PathGeometry::star(PointF::new(cx, 226.0), 92.0, 40.0, 5, 0.0).ok()?;
    let offset = -72.0 + 144.0 * ping_pong(t, 4.0) as f32;
    let circle = PathGeometry::circle(PointF::new(cx + offset, 226.0), 56.0);
    let result = match operation {
        0 => star.union(&circle, None),
        1 => star.intersect(&circle, None),
        2 => star.subtract(&circle, None),
        _ => star.exclusive_or(&circle, None),
    }
    .ok()?;
    let mut group = DrawingGroup::new(None);
    if !result.is_empty() {
        group.add(filled(result, RESULT));
    }
    group.add(outlined(star, 1.5, INPUT));
    group.add(outlined(circle, 1.5, INPUT));
    Some(group.into())
}

impl PathCombineScene {
    pub fn new(fonts: &FontRegistry) -> PathCombineScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        {
            let notes = session.add_layer("Notes", DIM);
            for (i, op) in OPERATIONS.iter().enumerate() {
                let cx = 40.0 + i as f32 * 305.0 + 140.0;
                label(notes, fonts, &format!("star.{op}(circle)"), cx, 350.0, 15.0, INK, TextAlign::Center);
            }
            label(notes, fonts, "Trim(0, t).ToStrokeOutline(26 px, round caps, miter joins) - a fill, its polygon edge in white", 40.0, 580.0, 14.0, INK, TextAlign::Left);
            label(notes, fonts, "rings: Inflate -14, 14, 30", 1088.0, 490.0, 13.0, DIM, TextAlign::Left);
            label(notes, fonts, "a centred stroke on Roboto: overlap lines show", 40.0, 610.0, 14.0, DIM, TextAlign::Left);
            label(notes, fonts, "MergeOverlaps = true: each glyph's contours merged", 680.0, 610.0, 14.0, DIM, TextAlign::Left);
        }
        session.add_layer("Art", INK);
        for i in 0..OPERATIONS.len() {
            let cx = 40.0 + i as f32 * 305.0 + 140.0;
            session.draw_shape(AnimatedShape::new(move |t| combined(i, cx, t), 0.0));
        }
        // Stroke to outline while it draws on.
        let signature = PathGeometry::parse(SIGNATURE).unwrap();
        session.draw_path(outlined(signature.clone(), 1.2, Color::from_rgba(132, 138, 164, 120)));
        let gradient = LinearGradientFill::from_colors(PointF::new(60.0, 400.0), PointF::new(610.0, 560.0), Color::from_rgb(120, 200, 255), Color::from_rgb(236, 90, 160)).unwrap();
        let style = PathStrokeStyle::new(26.0, PathStrokeStyleOptions { cap: PathStrokeCap::Round, join: PathStrokeJoin::Miter, ..PathStrokeStyleOptions::default() }).unwrap();
        session.draw_shape(AnimatedShape::new(
            move |t| {
                let end = ((t % 4.0) / 3.0).clamp(0.02, 1.0) as f32;
                let outline = signature.trim(0.0, end, 0.0).ok()?.to_stroke_outline(&style, None).ok()?;
                let mut group = DrawingGroup::new(None);
                group.add(PathShape::with_fill(outline.clone(), gradient.clone().into()));
                group.add(outlined(outline, 1.2, Color::from_rgba(255, 255, 255, 220)));
                Some(group.into())
            },
            0.0,
        ));
        // Inflate and deflate.
        let star = PathGeometry::star(PointF::new(960.0, 470.0), 72.0, 34.0, 6, 0.0).unwrap();
        for ring in [-14.0, 14.0, 30.0] {
            session.draw_path(outlined(star.inflate(ring, PathInflateOptions::default()).unwrap(), 1.2, Color::from_rgba(132, 138, 164, 130)));
        }
        let text_fonts = fonts.clone();
        session.draw_shape(AnimatedShape::new(
            move |t| {
                let d = distance(t);
                let mut group = DrawingGroup::new(None);
                let grown = star.inflate(d, PathInflateOptions { join: PathStrokeJoin::Miter, miter_limit: 2.0, ..PathInflateOptions::default() }).ok()?;
                if !grown.is_empty() {
                    group.add(filled(grown, Color::from_rgba(255, 176, 72, 230)));
                }
                group.add(outlined(star.clone(), 2.0, INK));
                let sign = if d > 0.0 { "+" } else if d < 0.0 { "-" } else { "" };
                group.add(TextShape::new(&format!("Inflate({sign}{:.1}, Miter, 2)", d.abs()), 1088.0, 466.0, 15.0, &text_fonts, TextShapeOptions { color: Some(RESULT), ..TextShapeOptions::default() }).ok()?);
                Some(group.into())
            },
            0.0,
        ));
        // Outlined text: the font's overlapping contours, and merged.
        let text = session.layout_text("HEAT WAVE", 70.0, None, None).unwrap();
        let stroke = || PathStrokeStyle::new(3.0, PathStrokeStyleOptions::default()).unwrap();
        session.draw_shape(TextBlockShape::with_stroke(text.clone(), 40.0, 618.0, Some(stroke()), TextBlockStrokeOptions { stroke_color: Some(INK), ..TextBlockStrokeOptions::default() }).unwrap());
        let mut merged = TextBlockShape::with_stroke(text, 680.0, 618.0, Some(stroke()), TextBlockStrokeOptions { stroke_color: Some(INK), ..TextBlockStrokeOptions::default() }).unwrap();
        merged.set_merge_overlaps(true);
        session.draw_shape(merged);
        PathCombineScene { session }
    }
}

impl DemoScene for PathCombineScene {
    fn name(&self) -> &'static str {
        "path-combine"
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
