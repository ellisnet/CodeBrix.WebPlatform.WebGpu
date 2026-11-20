//! "morph" (AnimationDemoApp's MorphScene): one PathMorph per step - circle, star, heart, rounded
//! square, circle - each a PathShape with a radial gradient fill and an outline whose
//! MorphFraction is bound to a track, shown in its own time window; and a countdown 3, 2, 1, 0
//! from the GLYPH OUTLINES of the bundled font (TextShape::get_outline) morphing into each other,
//! an extra contour growing from its centre, with the in-between's resampled points as dots.

use std::rc::Rc;

use codebrix_webgpu_animation::animation::EasingKind;
use codebrix_webgpu_animation::fills::RadialGradientFill;
use codebrix_webgpu_animation::models::AnimatedProperty;
use codebrix_webgpu_animation::paths::{PathGeometry, PathMorph, PathStrokeStyle, PathStrokeStyleOptions};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{AnimatedShape, DrawingGroup, DrawingShape, PathShape, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::kit::{disc, floats, L};
use crate::demo::{create_frame_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Path morphing";
const DESCRIPTION: &str = "PathMorph (contours paired, resampled by length, best start and direction), PathShape.MorphFraction, TextShape.GetOutline";
const STEP: f64 = 2.0;

pub struct MorphScene {
    session: DrawingSession,
}

fn window(i: usize, last: usize) -> (Option<f64>, Option<f64>) {
    (if i == 0 { None } else { Some(i as f64 * STEP) }, if i == last { None } else { Some((i + 1) as f64 * STEP) })
}

impl MorphScene {
    pub fn new(fonts: &FontRegistry) -> MorphScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = session.add_layer("Notes", DIM);
        label(notes, fonts, "one PathMorph per step; MorphFraction bound to a track", 330.0, 650.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "glyph outlines morphing; an extra contour grows from its centre", 950.0, 650.0, 14.0, DIM, TextAlign::Center);
        let center = PointF::new(330.0, 390.0);
        let circle = PathGeometry::circle(center, 170.0);
        let star = PathGeometry::star(center, 200.0, 85.0, 5, 0.0).unwrap();
        let heart = PathGeometry::parse("M 330 560 C 120 430, 130 230, 260 230 C 300 230, 320 260, 330 285 C 340 260, 360 230, 400 230 C 530 230, 540 430, 330 560 Z").unwrap();
        let square = PathGeometry::rounded_rectangle(RectangleF::new(170.0, 230.0, 320.0, 320.0), 40.0);
        let fill = RadialGradientFill::from_colors(PointF::new(290.0, 330.0), 280.0, Color::from_rgb(255, 214, 120), Color::from_rgb(214, 70, 110)).unwrap();
        let outline = PathStrokeStyle::new(4.0, PathStrokeStyleOptions::default()).unwrap();
        session.add_layer("Shapes", INK);
        let chain = [circle.clone(), star, heart, square, circle];
        for i in 0..chain.len() - 1 {
            let morph = PathMorph::new(chain[i].clone(), chain[i + 1].clone(), 0).unwrap();
            let shape = PathShape::with_morph(morph, 0.0, outline.clone(), Some(Color::from_rgb(255, 240, 220)), None, Some(fill.clone().into())).unwrap();
            let (start, end) = window(i, chain.len() - 2);
            let id = session.draw_shape(DrawingShape::from(shape).with_time_window(start, end));
            session.animations_mut().bind(id, AnimatedProperty::MorphFraction, floats(&[((i as f64 * STEP) + 0.4, 0.0, L), (((i + 1) as f64 * STEP) - 0.2, 1.0, EasingKind::CubicInOut)])).unwrap();
        }
        // The countdown, from the glyph outlines of the bundled font, centred on (950, 420).
        session.add_layer("Digits", Color::from_rgb(120, 180, 255));
        let glyphs: Vec<PathGeometry> = ["3", "2", "1", "0"].iter().map(|t| TextShape::new(t, 950.0, 540.0, 360.0, fonts, TextShapeOptions { align: TextAlign::Center, ..TextShapeOptions::default() }).unwrap().get_outline()).collect();
        for i in 0..glyphs.len() - 1 {
            let morph = Rc::new(PathMorph::new(glyphs[i].clone(), glyphs[i + 1].clone(), 0).unwrap());
            let shape = PathShape::with_morph((*morph).clone(), 0.0, PathStrokeStyle::new(5.0, PathStrokeStyleOptions::default()).unwrap(), Some(Color::from_rgb(120, 180, 255)), None, None).unwrap();
            let (start, end) = window(i, glyphs.len() - 2);
            let id = session.draw_shape(DrawingShape::from(shape).with_time_window(start, end));
            let progress = floats(&[((i as f64 * STEP) + 0.6, 0.0, L), (((i + 1) as f64 * STEP) - 0.1, 1.0, EasingKind::QuartInOut)]);
            session.animations_mut().bind(id, AnimatedProperty::MorphFraction, progress.clone()).unwrap();
            // The resampled points of this step's in-between, as small dots (shown while it morphs).
            let dots = AnimatedShape::new(
                move |t| {
                    let fraction = progress.evaluate(t).unwrap_or(0.0);
                    if fraction <= 0.0 || fraction >= 1.0 {
                        return None;
                    }
                    let mut group = DrawingGroup::new(Some(Color::from_rgba(255, 176, 72, 200)));
                    let in_between = morph.get_path(fraction);
                    for (k, el) in in_between.bez_path().elements().iter().enumerate() {
                        if k % 20 == 0 {
                            if let Some(end) = el.end_point() {
                                group.add(disc(end.x as f32, end.y as f32, 4.0, None));
                            }
                        }
                    }
                    Some(group.into())
                },
                0.0,
            );
            session.draw_shape(DrawingShape::from(dots).with_time_window(start, end));
        }
        MorphScene { session }
    }
}

impl DemoScene for MorphScene {
    fn name(&self) -> &'static str {
        "morph"
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
