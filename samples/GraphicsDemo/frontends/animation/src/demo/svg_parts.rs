//! "svg-parts" (AnimationDemoApp's SvgPartsScene): SVG as a session element. Two documents - the
//! C# scene's gear train (gear outlines as SVG path data) and its clock, generated the same way and
//! served by the backend - are loaded with DrawingSvg and drawn with draw_svg_shape. Per-part
//! animation by element id: the gears turn at their gear ratios about their own centres, the
//! clock's hands turn about the clock's centre, the clock's tick marks draw on by trimming their
//! strokes, and the gear train's arrow fades in and draws on. Whole-element animation: the gear
//! train fades in and the clock slides and grows into place, as any element would. The clock's
//! "PER-PART" label is SVG text, laid out by usvg in the Roboto the backend serves.

use codebrix_webgpu_animation::animation::EasingKind;
use codebrix_webgpu_animation::models::AnimatedProperty;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::svg::{DrawingSvgAspect, DrawingSvgShape};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{DrawingSession, PointF, RectangleF};

use crate::demo::kit::{floats, points, L};
use crate::demo::svg_files::load_svg;
use crate::demo::{create_frame_session, label, DemoScene, DIM, INK};

const TITLE: &str = "SVG elements and parts";
const DESCRIPTION: &str = "DrawingSvgShape: a loaded SVG as an element; get_part(id) / bind_part turn, fade and trim parts at draw time";

pub struct SvgPartsScene {
    session: DrawingSession,
}

fn bind_gear(train: &mut DrawingSvgShape, id: &str, centre: PointF, degrees_per_second: f32) {
    train.get_part_mut(id).unwrap().pivot = Some(centre);
    train.bind_part(id, AnimatedProperty::Rotation, floats(&[(0.0, 0.0, L), (8.0, degrees_per_second * 8.0, L)])).unwrap();
}

impl SvgPartsScene {
    pub fn new(fonts: &FontRegistry) -> SvgPartsScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = session.add_layer("Notes", DIM);
        label(notes, fonts, "parts by id: gear1 / gear2 / gear3 turn at their ratios (pivot = gear centre); arrow fades in", 40.0, 128.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "whole element: the gear train fades in", 40.0, 680.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "parts: hands turn about the centre, ticks draw on (TrimEnd)", 820.0, 128.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "whole element: the clock slides in and grows", 820.0, 680.0, 13.0, DIM, TextAlign::Left);
        session.add_layer("Art", INK);

        let gears = load_svg("gear-train").expect("the gear train loads");
        let mut train = DrawingSvgShape::new(gears, RectangleF::new(40.0, 140.0, 720.0, 520.0), DrawingSvgAspect::Meet);
        bind_gear(&mut train, "gear1", PointF::new(180.0, 200.0), 45.0);
        bind_gear(&mut train, "gear2", PointF::new(340.0, 200.0), -45.0 * 100.0 / 60.0);
        bind_gear(&mut train, "gear3", PointF::new(410.71, 270.71), 45.0 * 100.0 / 40.0);
        train.bind_part("arrow", AnimatedProperty::Opacity, floats(&[(1.0, 0.0, L), (2.0, 1.0, L)])).unwrap();
        train.bind_part("arrow", AnimatedProperty::TrimEnd, floats(&[(1.0, 0.0, L), (2.5, 1.0, EasingKind::SineInOut)])).unwrap();
        let train = session.draw_svg_shape(train);
        session.animations_mut().bind(train, AnimatedProperty::Opacity, floats(&[(0.0, 0.0, L), (0.8, 1.0, EasingKind::CubicOut)])).unwrap();

        let clock = load_svg("clock").expect("the clock loads");
        let mut face = DrawingSvgShape::new(clock, RectangleF::new(840.0, 170.0, 380.0, 380.0), DrawingSvgAspect::Meet);
        let pivot = PointF::new(150.0, 150.0);
        for hand in ["hour", "minute", "second"] {
            face.get_part_mut(hand).unwrap().pivot = Some(pivot);
        }
        face.bind_part("second", AnimatedProperty::Rotation, floats(&[(0.0, 0.0, L), (8.0, 720.0, L)])).unwrap();
        face.bind_part("minute", AnimatedProperty::Rotation, floats(&[(0.0, 60.0, L), (8.0, 120.0, L)])).unwrap();
        face.bind_part("hour", AnimatedProperty::Rotation, floats(&[(0.0, 300.0, L), (8.0, 305.0, L)])).unwrap();
        face.bind_part("ticks", AnimatedProperty::TrimEnd, floats(&[(0.8, 0.0, L), (2.6, 1.0, EasingKind::SineInOut)])).unwrap();
        let face = session.draw_svg_shape(face);
        let mut a = session.animations_mut();
        a.bind(face, AnimatedProperty::Anchor, points(&[(0.0, 1030.0, 360.0, L)])).unwrap();
        a.bind(face, AnimatedProperty::Position, points(&[(0.0, 420.0, 0.0, L), (1.4, 0.0, 0.0, EasingKind::CubicOut)])).unwrap();
        a.bind(face, AnimatedProperty::Scale, floats(&[(0.0, 0.5, L), (1.6, 1.0, EasingKind::BackOut)])).unwrap();
        SvgPartsScene { session }
    }
}

impl DemoScene for SvgPartsScene {
    fn name(&self) -> &'static str {
        "svg-parts"
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
