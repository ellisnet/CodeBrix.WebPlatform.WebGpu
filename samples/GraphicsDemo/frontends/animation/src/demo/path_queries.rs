//! "path-queries" (AnimationDemoApp's PathQueriesScene): asking a path about points. Left, a star
//! with a hole (a path SUBTRACT); a field of dots behind it coloured by the signed distance (blue
//! inside, warm outside, banded every 20 units); probes wander over it, each green inside and red
//! outside (contains), with a ring whose radius is its distance to the edge and a line to the
//! nearest point. Right, a circle orbits a rounded hexagon and the gap between the two areas is
//! distance_with_offset - the circle is one fixed geometry asked about at an offset - drawn as a
//! line between their nearest points, 0 when they touch. The hexagon is
//! PathGeometry::rounded_polygon (C#'s PathGeometry.RoundedPolygon).

use std::f64::consts::PI;

use codebrix_webgpu_animation::paths::PathGeometry;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{CircleShape, LineShape, LineShapeOptions, PathShape, ShapeOptions, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF};

use crate::demo::kit::{disc, ring};
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Path queries: inside, edge distance, nearest point, gap";
const DESCRIPTION: &str = "PathGeometry.Contains, DistanceToEdge, GetSignedDistance, GetNearestPoint and Distance(other, offset) - shared arithmetic, identical in both packages";
const STAR_CENTER: PointF = PointF { x: 320.0, y: 405.0 };
const HEX_CENTER: PointF = PointF { x: 960.0, y: 405.0 };
const INSIDE: Color = Color::from_rgb(110, 220, 140);
const OUTSIDE: Color = Color::from_rgb(240, 90, 100);
const ORBIT_RADIUS: f32 = 50.0;

pub struct PathQueriesScene {
    session: DrawingSession,
    fonts: FontRegistry,
    star: PathGeometry,
    hexagon: PathGeometry,
    orbiter: PathGeometry,
}

fn outlined(geometry: PathGeometry, thickness: f32, color: Color) -> PathShape {
    PathShape::new(geometry, ShapeOptions { stroke_thickness: thickness, color: Some(color), is_filled: false }).unwrap()
}

impl PathQueriesScene {
    pub fn new(fonts: &FontRegistry) -> PathQueriesScene {
        let star = PathGeometry::star(STAR_CENTER, 220.0, 100.0, 5, 0.0).unwrap().subtract(&PathGeometry::circle(STAR_CENTER, 46.0), None).unwrap();
        let corners: Vec<PointF> = (0..6).map(|i| {
            let a = i as f64 * PI / 3.0 + PI / 6.0;
            PointF::new(HEX_CENTER.x + (115.0 * a.cos()) as f32, HEX_CENTER.y + (115.0 * a.sin()) as f32)
        }).collect();
        let hexagon = PathGeometry::rounded_polygon(&corners, 18.0).unwrap();
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let field = session.add_layer("Field", DIM);
        let mut y = 120.0;
        while y <= 690.0 {
            let mut x = 30.0;
            while x <= 610.0 {
                let d = star.get_signed_distance(PointF::new(x, y), None).unwrap();
                let band = (d.abs() / 20.0).floor() as i32;
                let fade = (1.0 - band as f32 * 0.16).max(0.15);
                let color = if d < 0.0 { Color::from_rgba(90, 150, 255, (200.0 * fade) as u8) } else { Color::from_rgba(255, 150, 80, (150.0 * fade) as u8) };
                field.add_shape(disc(x, y, if band % 2 == 0 { 2.4 } else { 1.4 }, Some(color)));
                x += 15.0;
            }
            y += 15.0;
        }
        field.add_shape(outlined(star.clone(), 2.0, INK));
        field.add_shape(PathShape::with_fill_color(hexagon.clone(), Color::from_rgb(70, 96, 160)));
        field.add_shape(outlined(hexagon.clone(), 2.0, Color::from_rgb(140, 170, 240)));
        label(field, fonts, "Contains (green inside / red outside), DistanceToEdge (ring), GetNearestPoint (line); dots: GetSignedDistance", 30.0, 118.0, 13.0, DIM, TextAlign::Left);
        label(field, fonts, "hexagon.Distance(circle, offset): the gap between the two areas", 700.0, 118.0, 13.0, DIM, TextAlign::Left);
        session.add_layer("Now", INK);
        PathQueriesScene { session, fonts: fonts.clone(), star, hexagon, orbiter: PathGeometry::circle(PointF::new(0.0, 0.0), ORBIT_RADIUS) }
    }
}

impl DemoScene for PathQueriesScene {
    fn name(&self) -> &'static str {
        "path-queries"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        let (star, hexagon, orbiter, fonts) = (&self.star, &self.hexagon, &self.orbiter, &self.fonts);
        let text = |s: &str, x: f32, y: f32, size: f32, color: Color, align: TextAlign| TextShape::new(s, x, y, size, fonts, TextShapeOptions { color: Some(color), align, ..TextShapeOptions::default() }).unwrap();
        let line = |x1: f32, y1: f32, x2: f32, y2: f32, t: f32| LineShape::new(x1, y1, x2, y2, LineShapeOptions { stroke_thickness: t, color: Some(INK) }).unwrap();
        let now = self.session.get_layer_mut("Now").unwrap();
        now.clear();
        // The probes: Lissajous wanderers over the star.
        for i in 0..4 {
            let phase = seconds * 2.0 * PI / 8.0;
            let probe = PointF::new(STAR_CENTER.x + (215.0 * (phase * (1 + i) as f64 + i as f64 * 1.3).sin()) as f32, STAR_CENTER.y + (200.0 * (phase * (2 + i % 2) as f64 + i as f64 * 0.7).sin()) as f32);
            let inside = star.contains(probe, None).unwrap();
            let edge = star.distance_to_edge(probe, None).unwrap();
            let nearest = star.get_nearest_point(probe, None).unwrap();
            let color = if inside { INSIDE } else { OUTSIDE };
            if edge > 1.0 {
                now.add_shape(ring(probe.x, probe.y, edge, 1.5, Some(if inside { Color::from_rgba(110, 220, 140, 150) } else { Color::from_rgba(240, 90, 100, 150) })));
            }
            now.add_shape(line(probe.x, probe.y, nearest.x, nearest.y, 1.5));
            now.add_shape(disc(nearest.x, nearest.y, 3.5, Some(INK)));
            now.add_shape(disc(probe.x, probe.y, 7.0, Some(color)));
            let d = star.get_signed_distance(probe, None).unwrap();
            now.add_shape(text(&format!("{}{:.1}", if d < 0.0 { "-" } else { "+" }, d.abs()), probe.x + 11.0, probe.y - 9.0, 14.0, color, TextAlign::Left));
        }
        // The orbiting circle: one geometry round (0, 0), asked about at an offset.
        let angle = seconds * 2.0 * PI / 8.0;
        let reach = 190.0 + 75.0 * (seconds * 2.0 * PI / 2.6).sin() as f32;
        let offset = PointF::new(HEX_CENTER.x + reach * angle.cos() as f32, HEX_CENTER.y + 0.8 * reach * angle.sin() as f32);
        let gap = hexagon.distance_with_offset(orbiter, offset, None).unwrap();
        let touching = gap <= 0.0;
        now.add_shape(CircleShape::new(offset.x, offset.y, ORBIT_RADIUS, ShapeOptions { stroke_thickness: 1.0, color: Some(if touching { Color::from_rgba(240, 90, 100, 200) } else { Color::from_rgba(255, 176, 72, 170) }), is_filled: true }).unwrap());
        if !touching {
            // The nearest pair, found by stepping between the two shapes' nearest points (both convex).
            let mut on_circle = offset;
            let mut on_hex = hexagon.get_nearest_point(on_circle, None).unwrap();
            for _ in 0..8 {
                let local = orbiter.get_nearest_point(PointF::new(on_hex.x - offset.x, on_hex.y - offset.y), None).unwrap();
                on_circle = PointF::new(local.x + offset.x, local.y + offset.y);
                on_hex = hexagon.get_nearest_point(on_circle, None).unwrap();
            }
            now.add_shape(line(on_hex.x, on_hex.y, on_circle.x, on_circle.y, 2.0));
            now.add_shape(disc(on_hex.x, on_hex.y, 4.0, Some(INK)));
            now.add_shape(disc(on_circle.x, on_circle.y, 4.0, Some(INK)));
            let (dx, dy) = (on_circle.x - on_hex.x, on_circle.y - on_hex.y);
            let length = (dx * dx + dy * dy).sqrt().max(1.0);
            let (mut nx, mut ny) = (-dy / length, dx / length);
            let (mx, my) = ((on_hex.x + on_circle.x) / 2.0, (on_hex.y + on_circle.y) / 2.0);
            if nx * (mx - HEX_CENTER.x) + ny * (my - HEX_CENTER.y) < 0.0 {
                nx = -nx;
                ny = -ny;
            }
            now.add_shape(text(&format!("{gap:.1}"), mx + nx * 22.0, my + ny * 22.0 + 5.0, 15.0, INK, TextAlign::Center));
        }
        let readout = if touching { "Distance = 0 - the areas touch or overlap".to_string() } else { format!("Distance = {gap:.2}") };
        now.add_shape(text(&readout, 960.0, 690.0, 18.0, if touching { OUTSIDE } else { ACCENT }, TextAlign::Center));
        self.session.render_with_background(scene, width, height);
    }
}
