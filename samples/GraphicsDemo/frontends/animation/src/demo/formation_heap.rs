//! "formation-heap" (AnimationDemoApp's FormationHeapScene): a formation that heaps up. A heart and
//! an arch (a path UNION of a heart and an arch that is itself a SUBTRACT) stand on one ground
//! line; leaves fall into them and pile up from the ground (HeapArrivalOrder: the heart grows from
//! its tip, the arch rises up both legs and closes over the middle last). Each leaf drifts down with
//! a SwayingFallArrival in a light breeze and lands in a seeded resting pose; the arrivals follow an
//! ArrivalSchedule done by 7.4 s, and bind_formation keeps the resting leaves on one layer and the
//! falling ones on another. A read-out counts the falling and resting leaves.

use std::cell::RefCell;
use std::rc::Rc;

use codebrix_webgpu_animation::formations::{ArrivalSchedule, FormationElements, FormationMember, FormationPose, FormationTargetOptions, FormationTargets, HeapArrivalOrder, IArrivalPath, SwayingFallArrival};
use codebrix_webgpu_animation::paths::{PathGeometry, PathGeometryBuilder, PathStrokeStyle, PathStrokeStyleOptions};
use codebrix_webgpu_animation::planes::DrawingRandom;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{DrawingShape, PathShape, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::kit::{fill_rect, line};
use crate::demo::{add_header, create_session, label, DemoScene, ACCENT, BACKDROP, DIM, FRAME_HEIGHT, FRAME_WIDTH, INK};

const TITLE: &str = "Formation: leaves fall and heap up";
const DESCRIPTION: &str = "FormationTargets with HeapArrivalOrder (bottom-up from a ground line), SwayingFallArrival, ArrivalSchedule, BindFormation";
const GROUND: f32 = 600.0;
const SEED: i64 = 5;
const PALETTE: [Color; 6] = [Color::from_rgb(214, 96, 40), Color::from_rgb(232, 150, 50), Color::from_rgb(190, 60, 40), Color::from_rgb(240, 190, 70), Color::from_rgb(160, 84, 40), Color::from_rgb(226, 120, 64)];

pub struct FormationHeapScene {
    session: DrawingSession,
    formation: FormationElements,
    times: Vec<f64>,
    fonts: FontRegistry,
}

/// How far the air has moved sideways by a time: a steady breeze that gusts a little.
fn breeze(seconds: f64) -> f64 {
    14.0 * seconds + 10.0 * (0.7 * seconds).sin()
}

fn color_of(k: usize) -> Color {
    PALETTE[DrawingRandom::for_key(SEED + 1, k as i64).next_int(PALETTE.len() as i32).unwrap_or(0) as usize]
}

impl FormationHeapScene {
    pub fn new(fonts: &FontRegistry) -> FormationHeapScene {
        let mut session = create_session(FRAME_WIDTH, FRAME_HEIGHT, BACKDROP, fonts);
        session.add_layer("Stage", DIM);
        let p = PointF::new;
        let heart = PathGeometryBuilder::new()
            .move_to(330.0, GROUND - 2.0)
            .cubic_to(p(250.0, 520.0), p(140.0, 470.0), p(140.0, 380.0))
            .cubic_to(p(140.0, 315.0), p(190.0, 280.0), p(242.0, 280.0))
            .cubic_to(p(284.0, 280.0), p(314.0, 302.0), p(330.0, 334.0))
            .cubic_to(p(346.0, 302.0), p(376.0, 280.0), p(418.0, 280.0))
            .cubic_to(p(470.0, 280.0), p(520.0, 315.0), p(520.0, 380.0))
            .cubic_to(p(520.0, 470.0), p(410.0, 520.0), p(330.0, GROUND - 2.0))
            .close()
            .build();
        let arch = PathGeometry::rounded_rectangle(RectangleF::new(680.0, 280.0, 480.0, GROUND - 280.0), 36.0).subtract(&PathGeometry::rounded_rectangle(RectangleF::new(790.0, 390.0, 260.0, 300.0), 130.0), None).unwrap();
        let shape = heart.union(&arch, None).unwrap();
        session.draw_shape(fill_rect(0.0, GROUND, FRAME_WIDTH as f32, FRAME_HEIGHT as f32 - GROUND, Some(Color::from_rgb(34, 30, 34)), 0.0));
        session.draw_shape(line(40.0, GROUND, FRAME_WIDTH as f32 - 40.0, GROUND, 2.0, Some(Color::from_rgb(110, 92, 80))));
        let dashes = PathStrokeStyle::new(1.5, PathStrokeStyleOptions { dash_pattern: Some(vec![6.0, 6.0]), ..PathStrokeStyleOptions::default() }).unwrap();
        session.draw_shape(PathShape::with_style(shape.clone(), dashes, Some(Color::from_rgba(132, 138, 164, 120)), None, None));
        let stage = session.get_layer_mut("Stage").unwrap();
        label(stage, fonts, "HeapArrivalOrder(groundY: 600): bottom up - the heart from its tip, the arch up both legs, its middle last", 640.0, GROUND + 30.0, 14.0, DIM, TextAlign::Center);
        label(stage, fonts, "SwayingFallArrival (sway, rock, tumble, flip, flutter) in a breeze; leaves: fill-only PathShape(geometry, color)", 640.0, GROUND + 52.0, 14.0, DIM, TextAlign::Center);
        let targets = FormationTargets::fill_geometry(&shape, FormationTargetOptions { spacing: 12.0, jitter: 0.3, edge_clearance: 4.0, seed: SEED, order: Some(Box::new(HeapArrivalOrder::new(GROUND).unwrap())) }).unwrap();
        let times = ArrivalSchedule::new(3.0, 7.4, None).unwrap().get_times(targets.len());
        let leaf = PathGeometry::parse("M -10 0 C -5 -7 5 -7 10 0 C 5 7 -5 7 -10 0 Z").unwrap();
        let rests = Rc::new(RefCell::new(vec![FormationPose::at(0.0, 0.0); targets.len()]));
        let air: Rc<dyn Fn(f64) -> f64> = Rc::new(breeze);
        let members = FormationMember::from_targets(&targets, &times, |target, arrival, k| {
            let mut random = DrawingRandom::for_key(SEED, k as i64);
            let rotation = random.uniform(0.0, 180.0) as f32;
            let scale = random.uniform(0.85, 1.15) as f32;
            let rest = FormationPose::new(target.position.x, target.position.y, rotation, scale, scale);
            rests.borrow_mut()[k] = rest;
            let path: Rc<dyn IArrivalPath> = Rc::new(SwayingFallArrival::new(&mut random, rest, arrival, 40.0, Some(air.clone())).unwrap());
            path
        })
        .unwrap();
        session.add_layer("Resting", ACCENT);
        session.add_layer("Falling", ACCENT);
        let rests = rests.borrow().clone();
        let falling_leaf = leaf.clone();
        let mut falling = |k: usize| vec![DrawingShape::from(PathShape::with_fill_color(falling_leaf.clone(), color_of(k)))];
        let formation = session.animations_mut().bind_formation(&members, "Resting", "Falling", |k| DrawingShape::from(PathShape::with_fill_color(leaf.clone(), color_of(k))).with_transform(rests[k].to_transform()), Some(&mut falling)).unwrap();
        // The header goes on top (with an opaque band), so the leaves fall behind it.
        add_header(&mut session, TITLE, DESCRIPTION, true);
        session.add_layer("Now", INK);
        FormationHeapScene { session, formation, times, fonts: fonts.clone() }
    }
}

impl DemoScene for FormationHeapScene {
    fn name(&self) -> &'static str {
        "formation-heap"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        let landed = self.times.iter().take_while(|t| **t <= seconds).count();
        let readout = format!("falling {:>4}    resting {:>4} of {}", self.formation.get_travelling_count(seconds), landed, self.formation.count());
        let fonts = self.fonts.clone();
        let now = self.session.get_layer_mut("Now").unwrap();
        now.clear();
        now.add_shape(TextShape::new(&readout, 1248.0, 46.0, 16.0, &fonts, TextShapeOptions { color: Some(ACCENT), align: TextAlign::Right, ..TextShapeOptions::default() }).unwrap());
        self.session.render_with_background(scene, width, height);
    }
}
