//! "easing" (AnimationDemoApp's EasingScene): every easing curve at once - the named curves of
//! EasingKind, three Steps staircases and the four CSS keyword cubic-bezier curves - each drawn
//! as a graph with a dot running along it, and a second dot sliding along the bottom of its
//! panel at the eased position. The graphs are drawn once; only the dots move.

use codebrix_webgpu_animation::animation::{CubicBezierEasing, Easing, EasingKind, StepPosition};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{CircleShape, LineShape, LineShapeOptions, PolylineShape, PolylineShapeOptions, RectangleShape, RectangleShapeOptions, ShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::{create_frame_session, label, DemoScene, ACCENT, CONTENT_TOP, DIM, INK};

const TITLE: &str = "Easing curves";
const DESCRIPTION: &str = "Easing.Evaluate for every EasingKind, Easing.Steps and CubicBezierEasing: a dot on each graph, and one at the eased position below it";

const COLUMNS: usize = 8;
const PANEL_WIDTH: f32 = 146.0;
const PANEL_HEIGHT: f32 = 116.0;
const GAP: f32 = 6.0;
const LEFT: f32 = 32.0;
const GRAPH_LOW: f64 = -0.35;
const GRAPH_HIGH: f64 = 1.35;
const RUN_SECONDS: f64 = 2.0;
const PAUSE_SECONDS: f64 = 0.6;

type Curve = Box<dyn Fn(f64) -> f64>;

pub struct EasingScene {
    curves: Vec<(String, Curve)>,
    session: DrawingSession,
}

fn panel_origin(index: usize) -> (f32, f32) {
    (LEFT + ((index % COLUMNS) as f32 * (PANEL_WIDTH + GAP)), CONTENT_TOP + 6.0 + ((index / COLUMNS) as f32 * (PANEL_HEIGHT + GAP)))
}

fn graph_x(panel_x: f32, t: f64) -> f32 {
    Easing::lerp_f32(panel_x + 10.0, panel_x + PANEL_WIDTH - 10.0, t)
}

fn graph_y(panel_y: f32, value: f64) -> f32 {
    let top = panel_y + 22.0;
    let bottom = panel_y + PANEL_HEIGHT - 18.0;
    Easing::lerp_f32(bottom, top, (value - GRAPH_LOW) / (GRAPH_HIGH - GRAPH_LOW))
}

impl EasingScene {
    pub fn new(fonts: &FontRegistry) -> EasingScene {
        let mut curves: Vec<(String, Curve)> = Vec::new();
        for kind in EasingKind::ALL {
            curves.push((kind.name().to_string(), Box::new(Easing::get_function(kind))));
        }
        curves.push(("Steps(4, End)".to_string(), Box::new(|t| Easing::steps(t, 4, StepPosition::End).unwrap())));
        curves.push(("Steps(4, Start)".to_string(), Box::new(|t| Easing::steps(t, 4, StepPosition::Start).unwrap())));
        curves.push(("Steps(3, Both)".to_string(), Box::new(|t| Easing::steps(t, 3, StepPosition::Both).unwrap())));
        curves.push(("CSS ease".to_string(), Box::new(|t| CubicBezierEasing::EASE.evaluate(t))));
        curves.push(("CSS ease-in".to_string(), Box::new(|t| CubicBezierEasing::EASE_IN.evaluate(t))));
        curves.push(("CSS ease-out".to_string(), Box::new(|t| CubicBezierEasing::EASE_OUT.evaluate(t))));
        curves.push(("CSS ease-in-out".to_string(), Box::new(|t| CubicBezierEasing::EASE_IN_OUT.evaluate(t))));

        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let graphs = session.add_layer("Graphs", DIM);
        for (i, (name, curve)) in curves.iter().enumerate() {
            let (x, y) = panel_origin(i);
            label(graphs, fonts, name, x + 8.0, y + 15.0, 11.0, INK, TextAlign::Left);
            graphs.add_shape(RectangleShape::new(x, y, PANEL_WIDTH, PANEL_HEIGHT, RectangleShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgba(132, 138, 164, 60)), corner_radius: 6.0, ..Default::default() }).unwrap());
            for v in [0.0, 1.0] {
                graphs.add_shape(LineShape::new(x + 8.0, graph_y(y, v), x + PANEL_WIDTH - 8.0, graph_y(y, v), LineShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgba(132, 138, 164, 70)) }).unwrap());
            }
            let points: Vec<PointF> = (0..97).map(|s| {
                let t = s as f64 / 96.0;
                PointF::new(graph_x(x, t), graph_y(y, curve(t)))
            }).collect();
            graphs.add_shape(PolylineShape::new(&points, PolylineShapeOptions { stroke_thickness: 2.0, color: Some(Color::from_rgb(120, 170, 255)), ..Default::default() }).unwrap());
        }
        session.add_layer("Dots", ACCENT);
        EasingScene { curves, session }
    }
}

impl DemoScene for EasingScene {
    fn name(&self) -> &'static str {
        "easing"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let t = ((seconds % (RUN_SECONDS + PAUSE_SECONDS)) / RUN_SECONDS).min(1.0);
        let moving = self.session.get_layer_mut("Dots").unwrap();
        moving.clear();
        for (i, (_, curve)) in self.curves.iter().enumerate() {
            let (x, y) = panel_origin(i);
            let value = curve(t);
            moving.add_shape(CircleShape::new(graph_x(x, t), graph_y(y, value), 3.5, ShapeOptions { stroke_thickness: 1.0, color: Some(ACCENT), is_filled: true }).unwrap());
            let track_y = y + PANEL_HEIGHT - 9.0;
            moving.add_shape(LineShape::new(x + 8.0, track_y, x + PANEL_WIDTH - 8.0, track_y, LineShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgba(132, 138, 164, 60)) }).unwrap());
            moving.add_shape(CircleShape::new(Easing::lerp_f32(x + 10.0, x + PANEL_WIDTH - 10.0, value), track_y, 4.0, ShapeOptions { stroke_thickness: 1.0, color: Some(INK), is_filled: true }).unwrap());
        }
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
