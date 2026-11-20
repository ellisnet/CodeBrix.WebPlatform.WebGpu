//! "additive" (AnimationDemoApp's AdditiveScene): motions that add up (bind_with_mode). Left, a
//! pendulum's rotation is a swing track (Replace) plus a flutter function (Add) - the flutter
//! bound BEFORE the swing, which the combining order makes no matter; middle, a star's scale is a
//! grow track (Replace) times a pulse (Multiply); right, a box goes round a loop (bind_along_path)
//! with a shake added on top (Add). Ghosts show each base alone; small plots show base, part and
//! result over the scene, with the play head; read-outs give the sums.

use std::f64::consts::PI;

use codebrix_webgpu_animation::animation::{EasingKind, FloatTrack, TrackLoopMode};
use codebrix_webgpu_animation::models::{AnimatedProperty, BindingMode, BoundTrack, DrawingTransform, ElementId, PathTravelOptions};
use codebrix_webgpu_animation::models::DrawingLayer;
use codebrix_webgpu_animation::paths::{PathGeometry, PathStrokeStyle, PathStrokeStyleOptions};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{DrawingGroup, DrawingShape, PathShape, PolylineShape, PolylineShapeOptions, ShapeOptions, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::kit::{disc, fill_rect, floats, line, outline_rect, L};
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Additive bindings: motions that add up";
const DESCRIPTION: &str = "BindingMode Replace / Multiply / Add - (base x factors) + additions, in any binding order; tracks and functions of time";
const PLOT_TOP: f32 = 470.0;
const PLOT_HEIGHT: f32 = 90.0;
const PLOT_WIDTH: f32 = 360.0;
const PANEL_LEFT: [f32; 3] = [40.0, 460.0, 880.0];
const BASE_COLOR: Color = Color::from_rgb(132, 138, 164);
const PART_COLOR: Color = Color::from_rgb(120, 180, 255);
const PIVOT: PointF = PointF { x: 220.0, y: 185.0 };
const STAR_CENTER: PointF = PointF { x: 640.0, y: 290.0 };

pub struct AdditiveScene {
    session: DrawingSession,
    fonts: FontRegistry,
    swing: FloatTrack,
    grow: FloatTrack,
    lap: FloatTrack,
    path: PathGeometry,
    pendulum: ElementId,
    star: ElementId,
    square: ElementId,
}

/// A quick flutter that swells and fades twice over the scene.
fn flutter(t: f64) -> f32 {
    (7.0 * (2.0 * PI * 3.2 * t).sin() * (PI * t / 4.0).sin().powi(2)) as f32
}

/// A heartbeat-like pulse about 1.
fn pulse(t: f64) -> f32 {
    (1.0 + 0.16 * (2.0 * PI * 1.5 * t).sin()) as f32
}

/// Two bursts of shaking, each rising and dying away over a second.
fn shake(t: f64) -> PointF {
    let envelope = if (2.0..3.0).contains(&t) { (PI * (t - 2.0)).sin() } else if (5.0..6.0).contains(&t) { (PI * (t - 5.0)).sin() } else { 0.0 };
    PointF::new((10.0 * envelope * (2.0 * PI * 13.0 * t).sin()) as f32, (10.0 * envelope * (2.0 * PI * 17.0 * t).cos()) as f32)
}

/// A value with its sign, rounded to one decimal (a rounded negative zero shows as 0.0).
fn signed(value: f32) -> String {
    let r = (value as f64 * 10.0).round() / 10.0 + 0.0;
    if r > 0.0 {
        format!("+{r:.1}")
    } else if r < 0.0 {
        format!("-{:.1}", -r)
    } else {
        "0.0".to_string()
    }
}

fn pendulum(rod: Color, bob: Color) -> DrawingShape {
    let mut group = DrawingGroup::new(None);
    group.add(line(0.0, 0.0, 0.0, 178.0, 3.0, Some(rod)));
    group.add(disc(0.0, 200.0, 24.0, Some(bob)));
    DrawingShape::from(group).with_transform(DrawingTransform::translate(PIVOT.x, PIVOT.y))
}

fn dashed(width: f32, dash: f32, gap: f32) -> PathStrokeStyle {
    PathStrokeStyle::new(width, PathStrokeStyleOptions { dash_pattern: Some(vec![dash, gap]), ..PathStrokeStyleOptions::default() }).unwrap()
}

/// A small plot of a base curve, a part and their combination.
#[allow(clippy::too_many_arguments)]
fn plot(layer: &mut DrawingLayer, fonts: &FontRegistry, panel: usize, base_at: &dyn Fn(f64) -> f32, part_at: &dyn Fn(f64) -> f32, combine: &dyn Fn(f32, f32) -> f32, low: f32, high: f32, names: [&str; 3]) {
    let left = PANEL_LEFT[panel];
    let y = |v: f32| PLOT_TOP + PLOT_HEIGHT - PLOT_HEIGHT * (v - low) / (high - low);
    layer.add_shape(fill_rect(left, PLOT_TOP, PLOT_WIDTH, PLOT_HEIGHT, Some(Color::from_rgb(28, 31, 44)), 4.0));
    let (mut base, mut part, mut result) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..=480 {
        let t = i as f64 * 8.0 / 480.0;
        let x = left + PLOT_WIDTH * i as f32 / 480.0;
        let (b, p) = (base_at(t), part_at(t));
        base.push(PointF::new(x, y(b)));
        part.push(PointF::new(x, y(p)));
        result.push(PointF::new(x, y(combine(b, p))));
    }
    let poly = |points: &[PointF], thickness: f32, color: Color| PolylineShape::new(points, PolylineShapeOptions { stroke_thickness: thickness, color: Some(color), is_closed: false, is_filled: false }).unwrap();
    if !names[0].is_empty() {
        layer.add_shape(poly(&base, 1.5, BASE_COLOR));
    }
    layer.add_shape(poly(&part, 1.5, PART_COLOR));
    if !names[2].is_empty() {
        layer.add_shape(poly(&result, 2.0, ACCENT));
    }
    let key = PLOT_TOP + PLOT_HEIGHT + 22.0;
    let mut x0 = left;
    if !names[0].is_empty() {
        label(layer, fonts, names[0], x0, key, 13.0, BASE_COLOR, TextAlign::Left);
        x0 += 70.0;
    }
    label(layer, fonts, names[1], x0, key, 13.0, PART_COLOR, TextAlign::Left);
    x0 += 80.0;
    if !names[2].is_empty() {
        label(layer, fonts, names[2], x0, key, 13.0, ACCENT, TextAlign::Left);
    }
}

impl AdditiveScene {
    pub fn new(fonts: &FontRegistry) -> AdditiveScene {
        let swing = floats(&[(0.0, -26.0, L), (1.3, 26.0, EasingKind::SineInOut), (2.6, -26.0, EasingKind::SineInOut)]).with_loop_mode(TrackLoopMode::Loop);
        let grow = floats(&[(0.0, 0.6, L), (4.0, 1.25, EasingKind::SineInOut), (8.0, 0.6, EasingKind::SineInOut)]);
        let lap = floats(&[(0.0, 0.0, L), (8.0, 1.0, L)]);
        let path = PathGeometry::ellipse(PointF::new(1060.0, 290.0), 150.0, 110.0);
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        {
            let notes = session.add_layer("Notes", DIM);
            label(notes, fonts, "rotation: swing (Replace) + flutter (Add)", PANEL_LEFT[0], 128.0, 14.0, INK, TextAlign::Left);
            label(notes, fonts, "the flutter is bound before the swing", PANEL_LEFT[0], 148.0, 12.0, DIM, TextAlign::Left);
            label(notes, fonts, "scale: grow track (Replace) x pulse (Multiply)", PANEL_LEFT[1], 128.0, 14.0, INK, TextAlign::Left);
            label(notes, fonts, "the ghost outline is the track alone", PANEL_LEFT[1], 148.0, 12.0, DIM, TextAlign::Left);
            label(notes, fonts, "position: BindAlongPath + shake (Add)", PANEL_LEFT[2], 128.0, 14.0, INK, TextAlign::Left);
            label(notes, fonts, "the ghost box is the path alone", PANEL_LEFT[2], 148.0, 12.0, DIM, TextAlign::Left);
            notes.add_shape(PathShape::with_style(path.clone(), dashed(1.5, 6.0, 6.0), Some(Color::from_rgba(132, 138, 164, 110)), None, None));
            notes.add_shape(disc(PIVOT.x, PIVOT.y, 5.0, Some(DIM)));
            let (s, g) = (swing.clone(), grow.clone());
            plot(notes, fonts, 0, &move |t| s.evaluate(t).unwrap_or(0.0), &flutter, &|b, p| b + p, -40.0, 40.0, ["swing", "flutter", "sum"]);
            plot(notes, fonts, 1, &move |t| g.evaluate(t).unwrap_or(0.0), &pulse, &|b, p| b * p, 0.4, 1.6, ["grow", "pulse", "product"]);
            plot(notes, fonts, 2, &|_| 0.0, &|t| shake(t).x, &|_, p| p, -12.0, 12.0, ["", "shake x", ""]);
        }
        session.add_layer("Ghosts", INK);
        session.add_layer("Things", INK);
        // Left: a pendulum drawn hanging from (0, 0) and placed at the pivot.
        let ghost = session.get_layer_mut("Ghosts").unwrap().add_element(pendulum(Color::from_rgba(132, 138, 164, 70), Color::from_rgba(132, 138, 164, 70)));
        session.animations_mut().bind(ghost, AnimatedProperty::Rotation, swing.clone()).unwrap();
        let swinging = session.get_layer_mut("Things").unwrap().add_element(pendulum(INK, ACCENT));
        session.animations_mut().bind_with_mode(swinging, AnimatedProperty::Rotation, BoundTrack::float_fn(flutter), BindingMode::Add).unwrap();
        session.animations_mut().bind(swinging, AnimatedProperty::Rotation, swing.clone()).unwrap();
        // Middle: a star scaled about its centre.
        let star_path = PathGeometry::star(STAR_CENTER, 100.0, 44.0, 5, 0.0).unwrap();
        let about_centre = DrawingTransform::IDENTITY.with_anchor(STAR_CENTER);
        let ghost_star = DrawingShape::from(PathShape::with_style(star_path.clone(), dashed(2.0, 6.0, 5.0), Some(Color::from_rgba(132, 138, 164, 160)), None, None)).with_transform(about_centre);
        let ghost_star = session.get_layer_mut("Ghosts").unwrap().add_element(ghost_star);
        session.animations_mut().bind(ghost_star, AnimatedProperty::Scale, grow.clone()).unwrap();
        let star = DrawingShape::from(PathShape::new(star_path, ShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgb(255, 176, 72)), is_filled: true }).unwrap()).with_transform(about_centre);
        let star = session.get_layer_mut("Things").unwrap().add_element(star);
        session.animations_mut().bind(star, AnimatedProperty::Scale, grow.clone()).unwrap();
        session.animations_mut().bind_with_mode(star, AnimatedProperty::Scale, BoundTrack::float_fn(pulse), BindingMode::Multiply).unwrap();
        // Right: a box (drawn round (0, 0)) going round the loop; shakes added on top.
        let ghost_box = session.get_layer_mut("Ghosts").unwrap().add_element(outline_rect(-22.0, -22.0, 44.0, 44.0, 2.0, Some(Color::from_rgba(132, 138, 164, 150)), 6.0));
        session.animations_mut().bind_along_path(ghost_box, &path, lap.clone(), PathTravelOptions::default()).unwrap();
        let square = session.get_layer_mut("Things").unwrap().add_element(fill_rect(-22.0, -22.0, 44.0, 44.0, Some(Color::from_rgb(120, 180, 255)), 6.0));
        session.animations_mut().bind_with_mode(square, AnimatedProperty::Position, BoundTrack::point_fn(shake), BindingMode::Add).unwrap();
        session.animations_mut().bind_along_path(square, &path, lap.clone(), PathTravelOptions::default()).unwrap();
        session.add_layer("Now", INK);
        AdditiveScene { session, fonts: fonts.clone(), swing, grow, lap, path, pendulum: swinging, star, square }
    }
}

impl DemoScene for AdditiveScene {
    fn name(&self) -> &'static str {
        "additive"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        let transform = |id: ElementId| self.session.find_element(id).map_or(DrawingTransform::IDENTITY, |e| e.transform());
        let (rotation, scale, at) = (transform(self.pendulum).rotation(), transform(self.star).scale_x(), transform(self.square).translation());
        let sh = shake(seconds);
        let on_path = self.path.get_point_at(self.lap.evaluate(seconds).unwrap_or(0.0)).position;
        let readouts = [
            format!("{} + {} = {} deg", signed(self.swing.evaluate(seconds).unwrap_or(0.0)), signed(flutter(seconds)), signed(rotation)),
            format!("{:.3} x {:.3} = {:.3}", self.grow.evaluate(seconds).unwrap_or(0.0), pulse(seconds), scale),
            format!("({:.1}, {:.1}) + ({}, {}) = ({:.1}, {:.1})", on_path.x, on_path.y, signed(sh.x), signed(sh.y), at.x, at.y),
        ];
        let fonts = self.fonts.clone();
        let now = self.session.get_layer_mut("Now").unwrap();
        now.clear();
        for (i, text) in readouts.iter().enumerate() {
            let x = PANEL_LEFT[i] + PLOT_WIDTH * (seconds / 8.0) as f32;
            now.add_shape(line(x, PLOT_TOP - 4.0, x, PLOT_TOP + PLOT_HEIGHT + 4.0, 2.0, Some(INK)));
            now.add_shape(TextShape::new(text, PANEL_LEFT[i], PLOT_TOP + PLOT_HEIGHT + 50.0, 16.0, &fonts, TextShapeOptions { color: Some(ACCENT), ..TextShapeOptions::default() }).unwrap());
        }
        self.session.render_with_background(scene, width, height);
    }
}
