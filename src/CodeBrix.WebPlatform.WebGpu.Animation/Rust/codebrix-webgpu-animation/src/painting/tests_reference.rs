//! BIT-FOR-BIT parity with NoSkia: every case builds the same inputs as a reference console run
//! against CodeBrix.Imaging.Drawing.NoSkia (the cached package's own DLLs; numbers only, no
//! images) and asserts the FNV-1a 64 hash of the bits of every value NoSkia produced: brushstroke
//! outlines, line axes, hard-edged masks of every kind of shape (filled, outlined with round
//! caps and joins, stroked paths with square caps and miter / bevel joins, transformed, grouped,
//! letterboxed), the demo's ownership maps with their filled form, seams, a bristle texture,
//! colour pickup and painting, and both keyframe players. A hash that differs means at least one
//! value differs in at least one bit. The math-library cases (sin / cos / pow / atan2) hold on
//! the host; a browser's math library may differ in the last bit.

use crate::color::Color;
use crate::geometry::PointF;
use crate::mask_meaning::MaskMeaning;
use crate::models::{DrawingTransform, DrawingTransformOptions};
use crate::painting::{DrawingOwnershipMap, KeyframePlayer, KeyframePlayerOptions, PlaneKeyframePlayer};
use crate::paths::{PathGeometry, PathGeometryBuilder, PathStrokeCap, PathStrokeJoin, PathStrokeStyle, PathStrokeStyleOptions};
use crate::planes::{DrawingPlane, DrawingPlaneImage, DrawingRandom, DrawingTransition, DrawingTransitionDirection};
use crate::shapes::{BrushStrokeShape, BrushStrokeShapeOptions, CircleShape, DrawingGroup, DrawingShape, LineShape, LineShapeOptions, PathShape, RectangleShape, RectangleShapeOptions, ShapeOptions};
use crate::{DrawingSession, DrawingSessionOptions};

struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(14695981039346656037)
    }
    fn b(&mut self, b: u8) {
        self.0 ^= b as u64;
        self.0 = self.0.wrapping_mul(1099511628211);
    }
    fn u32(&mut self, v: u32) {
        v.to_le_bytes().iter().for_each(|b| self.b(*b));
    }
    fn i32(&mut self, v: i32) {
        self.u32(v as u32);
    }
    fn f(&mut self, v: f32) {
        self.u32(v.to_bits());
    }
    fn plane(&mut self, p: &DrawingPlane) {
        self.u32(p.width());
        self.u32(p.height());
        p.data().iter().for_each(|v| self.f(*v));
    }
    fn image(&mut self, i: &DrawingPlaneImage) {
        self.plane(i.r());
        self.plane(i.g());
        self.plane(i.b());
        self.plane(i.a());
    }
    fn owners(&mut self, map: &DrawingOwnershipMap) {
        map.owners().iter().for_each(|o| self.i32(*o));
    }
}

fn check(name: &str, h: &Fnv, expected: u64) {
    assert_eq!(h.0, expected, "{name}: 0x{:016X} instead of NoSkia's 0x{expected:016X}", h.0);
}

fn session(w: u32, h: u32) -> DrawingSession {
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (w, h), layer_opacity: 255, background_fill_color: Color::TRANSPARENT, ..DrawingSessionOptions::default() });
    s.add_layer("Ink", Color::from_rgb(255, 0, 0));
    s
}

fn brush(cx: f64, cy: f64, length: f64, width: f64, degrees: f64, bend: f64) -> BrushStrokeShape {
    BrushStrokeShape::new(cx as f32, cy as f32, length as f32, width as f32, degrees as f32, BrushStrokeShapeOptions { bend: bend as f32, ..BrushStrokeShapeOptions::default() }).unwrap()
}

fn flow(u: f64, v: f64) -> f64 {
    (0.5 * ((u * 4.0) + (v * 2.5)).sin()) + (0.35 * ((v * 5.0) - (u * 1.5)).cos())
}

/// A seeded layout of brushstrokes in jittered rows, each turned along a gentle swirl, in a
/// shuffled painting order (the painterly demo scenes' layout, exactly).
pub(crate) fn layout(width: u32, height: u32, size: f32, seed: i64) -> Vec<BrushStrokeShape> {
    let mut random = DrawingRandom::new(seed);
    let mut strokes = Vec::new();
    let (s, w, h) = (size as f64, width as f64, height as f64);
    let mut y = -s * 0.2;
    while y < h + (s * 0.3) {
        let mut x = -s * 0.3 + random.uniform(0.0, s * 0.3);
        while x < w + (s * 0.4) {
            let cx = x + random.normal(0.0, s * 0.08);
            let cy = y + random.normal(0.0, s * 0.06);
            let angle = flow(cx / w, cy / h) + random.normal(0.0, 0.12);
            let length = s * random.uniform(0.75, 1.25);
            let width = length * random.uniform(0.28, 0.4);
            let bend = random.normal(0.0, 0.1);
            strokes.push(brush(cx, cy, length, width, angle * (180.0 / std::f64::consts::PI), bend));
            x += s * 0.5;
        }
        y += s * 0.28;
    }
    for i in (1..strokes.len()).rev() {
        let j = random.next_int(i as i32 + 1).unwrap() as usize;
        strokes.swap(i, j);
    }
    strokes
}

#[test]
fn brushstroke_outlines_are_noskias_bit_for_bit() {
    let mut r = DrawingRandom::new(5);
    let mut h = Fnv::new();
    for _ in 0..50 {
        let (cx, cy, length, width, angle) = (r.uniform(-50.0, 600.0), r.uniform(-50.0, 400.0), r.uniform(10.0, 200.0), r.uniform(2.0, 60.0), r.uniform(-720.0, 720.0));
        let (bend, profile) = (r.normal(0.0, 0.2), r.uniform(0.5, 9.0));
        let s = BrushStrokeShape::new(cx as f32, cy as f32, length as f32, width as f32, angle as f32, BrushStrokeShapeOptions { bend: bend as f32, profile: profile as f32, color: None }).unwrap();
        for p in s.get_points() {
            h.f(p.x);
            h.f(p.y);
        }
        let b = s.get_bounds();
        [b.x, b.y, b.width, b.height].iter().for_each(|v| h.f(*v));
    }
    check("strokes", &h, 0xE9B390456EAE036A);
}

#[test]
fn line_axes_are_noskias_bit_for_bit() {
    let mut r = DrawingRandom::new(6);
    let mut h = Fnv::new();
    for _ in 0..40 {
        let (x1, y1, x2, y2, t) = (r.uniform(-100.0, 100.0), r.uniform(-100.0, 100.0), r.uniform(-100.0, 100.0), r.uniform(-100.0, 100.0), r.uniform(0.5, 9.0));
        let line = DrawingShape::from(LineShape::new(x1 as f32, y1 as f32, x2 as f32, y2 as f32, LineShapeOptions { stroke_thickness: t as f32, color: None }).unwrap());
        let a = line.get_axes().unwrap();
        [a.center().x, a.center().y, a.angle(), a.length(), a.width()].iter().for_each(|v| h.f(*v));
    }
    check("line-axes", &h, 0xA62F35A6CFFDF075);
}

fn mask_case(output: (u32, u32), build: impl FnOnce(&mut DrawingSession)) -> Fnv {
    let mut s = session(64, 48);
    build(&mut s);
    let mask = s.export_mask(output, MaskMeaning::HardEdged).unwrap();
    let mut h = Fnv::new();
    h.plane(&mask);
    h
}

fn mask_cases(output: (u32, u32)) -> Vec<(&'static str, Fnv)> {
    let style = |w: f32, cap, join, miter| PathStrokeStyle::new(w, PathStrokeStyleOptions { cap, join, miter_limit: miter, ..PathStrokeStyleOptions::default() }).unwrap();
    vec![
        ("circle", mask_case(output, |s| {
            s.draw_circle(30.0, 20.0, 13.3, None, None, true).unwrap();
        })),
        ("line", mask_case(output, |s| {
            s.draw_line(3.0, 40.0, 60.0, 5.0, Some(3.5), None).unwrap();
        })),
        ("stroke", mask_case(output, |s| {
            s.draw_shape(brush(40.0, 30.0, 30.0, 9.0, 34.0, 0.1f32 as f64));
        })),
        ("ring", mask_case(output, |s| {
            s.draw_circle(20.0, 30.0, 9.0, Some(2.5), None, false).unwrap();
        })),
        ("roundrect", mask_case(output, |s| {
            s.draw_rectangle(5.0, 5.0, 20.0, 12.0, Some(3.0), None, false, 4.0).unwrap();
        })),
        ("ellipse", mask_case(output, |s| {
            s.draw_ellipse(44.0, 14.0, 15.0, 7.5, Some(1.7), None, false).unwrap();
        })),
        ("arrow", mask_case(output, |s| {
            s.draw_arrow(50.0, 40.0, 60.0, 10.0, Some(2.0), None, Some(9.0)).unwrap();
        })),
        ("zigzag", mask_case(output, |s| {
            s.draw_polyline(&[(4.0, 44.0), (14.0, 30.0), (22.0, 41.0), (35.0, 26.0), (37.0, 44.0)], Some(2.2), None, false, false).unwrap();
        })),
        ("star", mask_case(output, |s| {
            s.draw_path(PathShape::new(PathGeometry::star(PointF::new(32.0, 24.0), 20.0, 8.0, 5, 10.0).unwrap(), ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap());
        })),
        ("path-miter", mask_case(output, |s| {
            let g = PathGeometryBuilder::new().move_to(6.0, 6.0).cubic_to(PointF::new(30.0, -10.0), PointF::new(40.0, 50.0), PointF::new(58.0, 20.0)).line_to(40.0, 30.0).build();
            s.draw_path(PathShape::with_style(g, style(4.0, PathStrokeCap::Square, PathStrokeJoin::Miter, 3.0), None, None, None));
        })),
        ("path-bevel", mask_case(output, |s| {
            let g = PathGeometryBuilder::new().move_to(10.0, 40.0).line_to(30.0, 8.0).line_to(50.0, 40.0).close().build();
            s.draw_path(PathShape::with_style(g, style(3.0, PathStrokeCap::Butt, PathStrokeJoin::Bevel, 4.0), None, Some(Color::from_rgb(0, 0, 255)), None));
        })),
        ("rotated", mask_case(output, |s| {
            let r = RectangleShape::new(20.0, 15.0, 24.0, 14.0, RectangleShapeOptions { is_filled: true, ..RectangleShapeOptions::default() }).unwrap();
            s.draw_shape(DrawingShape::from(r).with_transform(DrawingTransform::rotate(23.0, PointF::new(32.0, 22.0))));
        })),
        ("group", mask_case(output, |s| {
            let mut g = DrawingGroup::new(None);
            g.add(CircleShape::new(10.0, 10.0, 6.0, ShapeOptions { is_filled: true, ..ShapeOptions::default() }).unwrap());
            g.add(LineShape::new(0.0, 20.0, 20.0, 0.0, LineShapeOptions { stroke_thickness: 2.0, color: None }).unwrap());
            let t = DrawingTransform::new(PointF::new(12.0, 8.0), DrawingTransformOptions { rotation: 15.0, scale_x: 1.5, scale_y: 1.2, anchor: PointF::new(10.0, 10.0) }).unwrap();
            s.draw_shape(DrawingShape::from(g).with_transform(t));
        })),
    ]
}

#[test]
fn hard_edged_masks_of_every_kind_of_shape_are_noskias_bit_for_bit() {
    let expected: [(&str, u64, u64); 13] = [
        ("circle", 0x073167AF1E47C135, 0xCDE96D369D87C3FD),
        ("line", 0x826A93C5BE3A6E38, 0xF3EF8A2EEFC2ECBD),
        ("stroke", 0x7AEA1146AA4D94C8, 0x86A1B3546242547D),
        ("ring", 0x70925022C4D502D5, 0xD31BB959B0639AAD),
        ("roundrect", 0x6D79C5472711AA95, 0xD239BC41C514837D),
        ("ellipse", 0xC4DE658965FEFC75, 0xC770E8AE97BEE01D),
        ("arrow", 0xA175C7812FECBCC5, 0x80A7CA16FE36F01D),
        ("zigzag", 0x670B6B31BA3F1B25, 0xC164450F529A303D),
        ("star", 0x2CE6F09A46737CC8, 0x10B939576D3314A0),
        ("path-miter", 0x33EE86DCB84B8415, 0x6C55B15A026D34BD),
        ("path-bevel", 0xA576EEBF231F9475, 0x5519A09DFB12E450),
        ("rotated", 0x830B6BE9701C2B85, 0x9C90B99ED510965D),
        ("group", 0x214B766D116FC435, 0x9A1BEFAAF1591470),
    ];
    let mut failures = Vec::new();
    for ((name, h), (n2, at_size, _)) in mask_cases((64, 48)).iter().zip(expected.iter()) {
        assert_eq!(name, n2);
        if h.0 != *at_size {
            failures.push(format!("mask-{name}"));
        }
    }
    for ((name, h), (_, _, fit)) in mask_cases((100, 60)).iter().zip(expected.iter()) {
        if h.0 != *fit {
            failures.push(format!("mask-{name}-fit"));
        }
    }
    assert!(failures.is_empty(), "differ from NoSkia: {failures:?}");
}

#[test]
fn the_demos_ownership_map_and_everything_made_from_it_are_noskias_bit_for_bit() {
    //Arrange - the colour-pickup scene's layout
    let mut s = session(560, 420);
    for stroke in layout(560, 420, 46.0, 9) {
        s.get_layer_mut("Ink").unwrap().add_shape(stroke);
    }
    //Act and Assert, step by step as the scene works
    let map = s.export_layer_ownership_map("Ink", (560, 420)).unwrap();
    let mut h = Fnv::new();
    h.owners(&map);
    h.i32(map.element_count() as i32);
    h.i32(map.unowned_count() as i32);
    check("own-map", &h, 0x203B6DA783F717CC);
    let filled = map.fill_unowned();
    let mut h = Fnv::new();
    h.owners(&filled);
    check("own-filled", &h, 0xEF4EF82FF4B1914E);
    let mut h = Fnv::new();
    h.plane(&filled.create_boundary_plane(Some(0.7)).unwrap());
    check("own-seams", &h, 0x69EB386322D827FF);

    let mut pr = DrawingRandom::new(77);
    let phases: Vec<f32> = (0..filled.element_count()).map(|_| pr.uniform(0.0, 2.0 * std::f64::consts::PI) as f32).collect();
    let mut bristle = DrawingPlane::new(560, 420).unwrap();
    filled
        .fill_by_element(
            &mut bristle,
            |p| {
                let phase = phases[p.element_index()] as f64;
                (1.0 + (1.9 * ((0.07 * ((p.across() * 0.9) + phase).sin()) + (0.045 * ((p.across() * 0.35) + (2.0 * phase)).sin()))) + (0.07 * 1.9 * (p.along() / p.axes().length() as f64))) as f32
            },
            Some(1.0),
            None,
        )
        .unwrap();
    let mut h = Fnv::new();
    h.plane(&bristle);
    check("own-bristle", &h, 0x3C96DF0726C3B48D);

    let mut picture = DrawingPlaneImage::new(280, 210).unwrap();
    picture.fill_with(|x, y| {
        let (x, y) = (x as i32, y as i32);
        [(0.5 + (0.5 * (x as f64 / 17.0).sin())) as f32, (y as f64 / 210.0) as f32, (((x * y) % 7) as f64 / 7.0) as f32, (((x + y) % 5) as f64 / 4.0) as f32]
    });
    let means = filled.get_mean_colors(&picture);
    let mut h = Fnv::new();
    means.iter().for_each(|m| m.iter().for_each(|v| h.f(*v)));
    check("own-means", &h, 0x5E5542BD8AEDDECA);
    let bytes: Vec<u8> = (0..280 * 210 * 4).map(|i: i32| ((i * 31 + (i >> 7)) & 255) as u8).collect();
    let mut h = Fnv::new();
    filled.get_mean_colors_bytes(&bytes, 280, 210).unwrap().iter().for_each(|m| m.iter().for_each(|v| h.f(*v)));
    check("own-means-bytes", &h, 0x5BB6A09FD08C3DC7);

    let mut painting = DrawingPlaneImage::new(560, 420).unwrap();
    filled.apply_colors(&mut painting, &means).unwrap();
    let mut seams = filled.create_boundary_plane(Some(0.7)).unwrap();
    seams.multiply(0.9).clamp(0.0, 1.0).unwrap();
    painting.blend_color(Color::from_rgb(12, 14, 34), &seams).unwrap();
    let mut h = Fnv::new();
    h.image(&painting);
    check("own-painting", &h, 0x6A5DCA838103DC7C);

    let mut h = Fnv::new();
    for i in 0..filled.element_count() {
        let a = filled.get_axes(i).unwrap();
        [a.center().x, a.center().y, a.angle(), a.length(), a.width()].iter().for_each(|v| h.f(*v));
        let b = filled.get_bounds(i).unwrap();
        [b.x, b.y, b.width, b.height, filled.get_pixel_count(i).unwrap() as i32].iter().for_each(|v| h.i32(*v));
    }
    check("own-axes-bounds", &h, 0x29F8E00E22DFB850);
}

/// The demo's ownership scene at a time: forty drifting strokes and three circles.
pub(crate) fn ownership_scene(seconds: f64) -> DrawingSession {
    let (map_width, map_height) = (560.0, 420.0);
    let mut random = DrawingRandom::new(2024);
    let mut s = session(560, 420);
    let paint = s.get_layer_mut("Ink").unwrap();
    for _ in 0..40 {
        let (x, y, length, angle, speed, phase) = (random.uniform(0.0, map_width), random.uniform(0.0, map_height), random.uniform(60.0, 140.0), random.uniform(0.0, 180.0), random.uniform(-23.0, 23.0), random.uniform(0.0, 2.0 * std::f64::consts::PI));
        let drift = ((seconds * 0.6) + phase).sin();
        paint.add_shape(brush(x + (30.0 * drift), y + (14.0 * ((seconds * 0.5) + phase).cos()), length, length * 0.32, angle + (speed * seconds), 0.12 * drift));
    }
    for c in 0..3 {
        let a = (seconds * 0.5) + (c as f64 * 2.1);
        paint.add_shape(CircleShape::new((280.0 + (170.0 * a.cos())) as f32, (210.0 + (110.0 * a.sin())) as f32, 34.0, ShapeOptions { stroke_thickness: 1.0, color: None, is_filled: true }).unwrap());
    }
    s
}

#[test]
fn the_demos_ownership_scene_is_noskias_bit_for_bit_at_both_sizes() {
    let s = ownership_scene(1.3);
    let map = s.export_layer_ownership_map("Ink", (560, 420)).unwrap();
    let mut h = Fnv::new();
    h.owners(&map);
    check("scene-ownership-map", &h, 0xD236B3E841A3C062);
    let mut h = Fnv::new();
    h.owners(&map.fill_unowned());
    check("scene-ownership-filled", &h, 0x0F8AFD545AA8353B);
    let mut h = Fnv::new();
    h.owners(&s.export_layer_ownership_map("Ink", (1280, 720)).unwrap());
    check("scene-ownership-1280", &h, 0xB73A202B5BA178A0);
}

#[test]
fn both_keyframe_players_give_noskias_frames_bit_for_bit() {
    const W: u32 = 6;
    const H: u32 = 4;
    let times = [0.0, 0.3, 1.75, 0.6, 7.2, -0.4, 3.1];
    let mut player = KeyframePlayer::new(
        W,
        H,
        0.5,
        |k, seconds, rgba| {
            for (i, b) in rgba.iter_mut().enumerate() {
                *b = (((k * 37) + (i as i32 * 11) + (seconds * 8.0) as i32) & 255) as u8;
            }
            Ok(())
        },
        KeyframePlayerOptions::default(),
    )
    .unwrap();
    let mut frame = vec![0u8; (W * H * 4) as usize];
    let mut h = Fnv::new();
    for t in times {
        player.render_frame(t, &mut frame).unwrap();
        frame.iter().for_each(|b| h.b(*b));
    }
    check("keyframes-bytes", &h, 0x8499881893405EBD);

    let wipe = DrawingTransition::wipe(DrawingTransitionDirection::Down, Some(0.3)).unwrap();
    let mut planes = PlaneKeyframePlayer::new(
        W,
        H,
        0.75,
        |k, seconds, image| {
            image.fill_with(|x, y| {
                let (x, y) = (x as i32, y as i32);
                [(k as f64 * 0.1 + x as f64 * 0.05) as f32, (seconds - y as f64) as f32, 0.25f32 * x as f32, (((x + y + k) % 4) as f64 / 3.0) as f32]
            });
            Ok(())
        },
        KeyframePlayerOptions { transition: Some(wipe), ..KeyframePlayerOptions::default() },
    )
    .unwrap();
    let mut image = DrawingPlaneImage::new(W, H).unwrap();
    let mut h = Fnv::new();
    for t in times {
        planes.render_frame(t, &mut image).unwrap();
        h.image(&image);
    }
    check("keyframes-planes", &h, 0x1EF7B3D81FD764D9);
}
