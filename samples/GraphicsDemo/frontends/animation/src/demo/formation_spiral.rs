//! "formation-spiral" (AnimationDemoApp's FormationSpiralScene): a formation spiralling in. A
//! word's glyph outline is covered with target spots (FormationTargets::fill_geometry, filling from
//! the centre outwards with CenterOutArrivalOrder); each spot gets an arrival time from an
//! ArrivalSchedule (a lazy start, a busy middle and a long taper, done by 6.4 s) and a
//! SpiralArrival; bind_formation turns them into resting dots on one layer and travelling dots on
//! another. Below, the schedule's rate with the play head, and a count of the dots.

use std::rc::Rc;

use codebrix_webgpu_animation::formations::{ArrivalRateProfile, ArrivalSchedule, CenterOutArrivalOrder, FormationElements, FormationMember, FormationPose, FormationTargetOptions, FormationTargets, IArrivalPath, SpiralArrival, SpiralArrivalOptions};
use codebrix_webgpu_animation::planes::DrawingRandom;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{DrawingShape, LineShape, LineShapeOptions, PathShape, PolylineShape, PolylineShapeOptions, ShapeOptions, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::kit::disc;
use crate::demo::{add_header, create_session, label, DemoScene, ACCENT, BACKDROP, DIM, FRAME_HEIGHT, FRAME_WIDTH, INK};

const TITLE: &str = "Formation: dots spiral in to form a word";
const DESCRIPTION: &str = "FormationTargets over a glyph outline, CenterOutArrivalOrder, ArrivalSchedule, SpiralArrival, DrawingAnimations.BindFormation";
const WORD: &str = "BLOOM";
const RADIUS: f32 = 3.0;
const SEED: i64 = 11;
const FIRST_ARRIVAL: f64 = 1.6;
const LAST_ARRIVAL: f64 = 6.4;
const GRAPH: (f32, f32, f32, f32) = (240.0, 1040.0, 590.0, 650.0);
const PALETTE: [Color; 5] = [Color::from_rgb(255, 176, 72), Color::from_rgb(255, 128, 72), Color::from_rgb(240, 84, 110), Color::from_rgb(255, 214, 110), Color::from_rgb(250, 150, 96)];

pub struct FormationSpiralScene {
    session: DrawingSession,
    formation: FormationElements,
    times: Vec<f64>,
    fonts: FontRegistry,
}

fn graph_x(seconds: f64) -> f32 {
    GRAPH.0 + (GRAPH.1 - GRAPH.0) * (seconds / 8.0) as f32
}

fn color_of(k: usize) -> Color {
    PALETTE[DrawingRandom::for_key(SEED + 1, k as i64).next_int(PALETTE.len() as i32).unwrap_or(0) as usize]
}

impl FormationSpiralScene {
    pub fn new(fonts: &FontRegistry) -> FormationSpiralScene {
        let mut session = create_session(FRAME_WIDTH, FRAME_HEIGHT, BACKDROP, fonts);
        session.add_layer("Notes", DIM);
        let layout = session.layout_text(WORD, 300.0, None, None).unwrap();
        let left = (FRAME_WIDTH as f32 - layout.width()) / 2.0;
        let outline = layout.get_outline_at(left, 150.0);
        session.draw_shape(PathShape::new(outline.clone(), ShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgba(132, 138, 164, 60)), is_filled: false }).unwrap());
        let targets = FormationTargets::fill_geometry(&outline, FormationTargetOptions { spacing: 7.5, jitter: 0.25, edge_clearance: 2.5, seed: SEED, order: Some(Box::new(CenterOutArrivalOrder::new(false))) }).unwrap();
        let profile = ArrivalRateProfile::default();
        let times = ArrivalSchedule::new(FIRST_ARRIVAL, LAST_ARRIVAL, Some(profile.clone())).unwrap().get_times(targets.len());
        let members = FormationMember::from_targets(&targets, &times, |target, arrival, k| {
            let mut random = DrawingRandom::for_key(SEED, k as i64);
            let travel = random.uniform(1.2, 1.7);
            // C#'s order of the draws: the start radius, then the start angle.
            let start_radius = random.uniform(240.0, 460.0) as f32;
            let options = SpiralArrivalOptions { start_angle: random.uniform(0.0, 360.0) as f32, turns: 1.25, start_scale: 1.8, ..SpiralArrivalOptions::default() };
            let path: Rc<dyn IArrivalPath> = Rc::new(SpiralArrival::new(FormationPose::at(target.position.x, target.position.y), arrival - travel, arrival, start_radius, options).unwrap());
            path
        })
        .unwrap();
        session.add_layer("Resting", ACCENT);
        session.add_layer("Travelling", ACCENT);
        let spots: Vec<PointF> = targets.iter().map(|t| t.position).collect();
        let mut travelling = |k: usize| vec![DrawingShape::from(disc(0.0, 0.0, RADIUS, Some(color_of(k))))];
        let formation = session.animations_mut().bind_formation(&members, "Resting", "Travelling", |k| DrawingShape::from(disc(spots[k].x, spots[k].y, RADIUS, Some(color_of(k)))), Some(&mut travelling)).unwrap();
        // The schedule's rate over the scene, with the span of the arrivals marked.
        session.set_active_layer("Notes");
        let (gl, gr, gt, gb) = GRAPH;
        let mut rate: Vec<PointF> = (0..=200).map(|i| {
            let u = i as f64 / 200.0;
            PointF::new(graph_x(FIRST_ARRIVAL + u * (LAST_ARRIVAL - FIRST_ARRIVAL)), gb - (gb - gt) * profile.get_rate(u) as f32)
        }).collect();
        let curve = rate.clone();
        rate.push(PointF::new(graph_x(LAST_ARRIVAL), gb));
        rate.push(PointF::new(graph_x(FIRST_ARRIVAL), gb));
        session.draw_shape(PolylineShape::new(&rate, PolylineShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgba(255, 176, 72, 60)), is_closed: true, is_filled: true }).unwrap());
        session.draw_shape(PolylineShape::new(&curve, PolylineShapeOptions { stroke_thickness: 2.0, color: Some(Color::from_rgba(255, 176, 72, 200)), is_closed: false, is_filled: false }).unwrap());
        let rule = |x1: f32, y1: f32, x2: f32, y2: f32| LineShape::new(x1, y1, x2, y2, LineShapeOptions { stroke_thickness: 1.0, color: Some(DIM) }).unwrap();
        session.draw_shape(rule(gl, gb, gr, gb));
        let notes = session.get_layer_mut("Notes").unwrap();
        for s in 0..=8 {
            notes.add_shape(rule(graph_x(s as f64), gb, graph_x(s as f64), gb + 5.0));
            label(notes, fonts, &format!("{s} s"), graph_x(s as f64), gb + 20.0, 12.0, DIM, TextAlign::Center);
        }
        label(notes, fonts, "arrival rate: ArrivalSchedule(1.6 s, 6.4 s, new ArrivalRateProfile()) - ramp, hold, taper", gl, gt - 12.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "FormationTargets.Fill(outline, spacing 7.5, CenterOutArrivalOrder)  +  SpiralArrival  +  BindFormation", 640.0, 140.0, 14.0, DIM, TextAlign::Center);
        // The header goes on top (with an opaque band), so the dots fly behind it.
        add_header(&mut session, TITLE, DESCRIPTION, true);
        session.add_layer("Now", INK);
        FormationSpiralScene { session, formation, times, fonts: fonts.clone() }
    }
}

impl DemoScene for FormationSpiralScene {
    fn name(&self) -> &'static str {
        "formation-spiral"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        let arrived = self.times.iter().take_while(|t| **t <= seconds).count();
        let readout = format!("spots {}    resting {}    spiralling in {}", self.formation.count(), arrived, self.formation.get_travelling_count(seconds));
        let fonts = self.fonts.clone();
        let now = self.session.get_layer_mut("Now").unwrap();
        now.clear();
        now.add_shape(LineShape::new(graph_x(seconds), GRAPH.2 - 4.0, graph_x(seconds), GRAPH.3, LineShapeOptions { stroke_thickness: 2.0, color: Some(INK) }).unwrap());
        now.add_shape(TextShape::new(&readout, 1248.0, 46.0, 16.0, &fonts, TextShapeOptions { color: Some(ACCENT), align: TextAlign::Right, ..TextShapeOptions::default() }).unwrap());
        self.session.render_with_background(scene, width, height);
    }
}
