//! Host tests of the formations, ported from NoSkia's FormationTests with their stated values,
//! plus values worked out from NoSkia's formulas.

use std::rc::Rc;

use crate::animation::EasingKind;
use crate::formations::{
    ArrivalRateProfile, ArrivalSchedule, CenterOutArrivalOrder, CurvedArrival, CurvedArrivalOptions, CustomArrivalOrder, DirectionalArrivalOrder, FormationArea, FormationPose, FormationTargetOptions, FormationTargets, HeapArrivalOrder, IArrivalPath,
    SpiralArrival, SpiralArrivalOptions, StraightArrival, SwayingFallArrival,
};
use crate::geometry::{PointF, RectangleF};
use crate::paths::{PathGeometry, PathGeometryBuilder};
use crate::planes::DrawingRandom;

fn disc() -> PathGeometry {
    PathGeometry::circle(PointF::new(200.0, 200.0), 100.0)
}

/// A thick "U": two legs on a ground line at y = 300 joined at the bottom.
fn cup() -> PathGeometry {
    let points = [(100.0, 100.0), (150.0, 100.0), (150.0, 250.0), (250.0, 250.0), (250.0, 100.0), (300.0, 100.0), (300.0, 300.0), (100.0, 300.0)].map(|(x, y)| PointF::new(x, y));
    PathGeometryBuilder::new().add_polygon(&points, true).build()
}

fn opts(spacing: f32, edge: f32, seed: i64) -> FormationTargetOptions {
    FormationTargetOptions { spacing, edge_clearance: edge, seed, ..FormationTargetOptions::default() }
}

#[test]
fn fill_gives_the_same_spots_for_the_same_seed_inside_and_clear_of_the_edge() {
    //Act
    let a = FormationTargets::fill_geometry(&disc(), opts(12.0, 5.0, 7)).unwrap();
    let b = FormationTargets::fill_geometry(&disc(), opts(12.0, 5.0, 7)).unwrap();
    let other = FormationTargets::fill_geometry(&disc(), opts(12.0, 5.0, 8)).unwrap();
    //Assert
    assert!(a.len() > 150, "{}", a.len());
    assert_eq!(a.iter().map(|t| t.position).collect::<Vec<_>>(), b.iter().map(|t| t.position).collect::<Vec<_>>());
    assert_ne!(a.iter().map(|t| t.position).collect::<Vec<_>>(), other.iter().map(|t| t.position).collect::<Vec<_>>());
    for t in &a {
        assert!(disc().contains(t.position, None).unwrap());
        assert!(t.clearance >= 5.0);
    }
    assert!(a.windows(2).all(|w| w[0].key <= w[1].key));
}

#[test]
fn the_first_target_follows_csharps_grid_and_jitter_draws_exactly() {
    // The bottom row starts at bounds.bottom - rowStep / 2, x from bounds.left; each candidate
    // draws x's jitter then y's from DrawingRandom(seed), kept or not - so the first kept target
    // of a square is computable by hand.
    let square = PathGeometryBuilder::new().add_rectangle(RectangleF::new(0.0, 0.0, 100.0, 100.0)).build();
    let targets = FormationTargets::fill_geometry(&square, FormationTargetOptions { spacing: 10.0, jitter: 0.25, edge_clearance: 0.0, seed: 3, order: Some(Box::new(DirectionalArrivalOrder::new(90.0, 0.0).unwrap())) }).unwrap();
    let mut r = DrawingRandom::new(3);
    let (row_step, jitter) = (10.0f32 * 0.866, 10.0f32 * 0.25);
    let x = 0.0f32 + r.uniform(-jitter as f64, jitter as f64) as f32;
    let y = (100.0f32 - (row_step * 0.5)) + r.uniform(-jitter as f64, jitter as f64) as f32;
    // The order sweeps top to bottom, so the first candidate (bottom left) is among the last.
    assert!(targets.iter().any(|t| t.position == PointF::new(x, y)), "({x}, {y})");
    let rows = ((100.0f32 - row_step * 0.5) / row_step).floor() as usize + 1;
    assert!(targets.len() <= rows * 11 && targets.len() > rows * 8, "{} for {rows} rows", targets.len());
}

fn distance_from_centre(p: PointF) -> f32 {
    (((p.x - 200.0) * (p.x - 200.0)) + ((p.y - 200.0) * (p.y - 200.0))).sqrt()
}

#[test]
fn center_out_fills_from_the_middle_and_reversed_from_the_outside() {
    let outward = FormationTargets::fill_geometry(&disc(), FormationTargetOptions { order: Some(Box::new(CenterOutArrivalOrder::new(false))), ..FormationTargetOptions::default() }).unwrap();
    let inward = FormationTargets::fill_geometry(&disc(), FormationTargetOptions { order: Some(Box::new(CenterOutArrivalOrder::new(true))), ..FormationTargetOptions::default() }).unwrap();
    assert!(distance_from_centre(outward[0].position) < 15.0);
    assert!(distance_from_centre(outward[outward.len() - 1].position) > 80.0);
    assert!(distance_from_centre(inward[0].position) > 80.0);
}

#[test]
fn heap_order_grows_from_the_ground_up_each_leg() {
    let mut heap = HeapArrivalOrder::new(300.0).unwrap();
    heap.scatter = 0.0;
    heap.lumpiness = 0.0;
    let spots = FormationTargets::fill_geometry(&cup(), FormationTargetOptions { spacing: 10.0, order: Some(Box::new(heap)), ..FormationTargetOptions::default() }).unwrap();
    let tenth = spots.len() / 10;
    let first: f32 = spots[..tenth].iter().map(|t| t.position.y).sum::<f32>() / tenth as f32;
    let last: f32 = spots[spots.len() - tenth..].iter().map(|t| t.position.y).sum::<f32>() / tenth as f32;
    assert!(first > 260.0, "{first}");
    assert!(last < 150.0, "{last}");
}

#[test]
fn directional_and_custom_orders_sort_as_their_keys_say() {
    let left_to_right = FormationTargets::fill_geometry(&disc(), FormationTargetOptions { order: Some(Box::new(DirectionalArrivalOrder::new(0.0, 0.0).unwrap())), ..FormationTargetOptions::default() }).unwrap();
    let deep_first = FormationTargets::fill_geometry(&disc(), FormationTargetOptions { order: Some(Box::new(CustomArrivalOrder::new(|_, _, clearance| -clearance))), ..FormationTargetOptions::default() }).unwrap();
    assert!(left_to_right.windows(2).all(|w| w[0].position.x <= w[1].position.x));
    assert!(deep_first.windows(2).all(|w| w[0].clearance >= w[1].clearance));
    assert!(DirectionalArrivalOrder::new(f32::NAN, 0.0).is_err());
}

#[test]
fn a_mask_area_matches_its_pixels() {
    //Arrange - a 40 x 20 plane with a 10 x 10 block at (10, 5), stretched x2 over (100, 0)
    let mut plane = vec![0.0f32; 40 * 20];
    for y in 5..15 {
        for x in 10..20 {
            plane[(y * 40) + x] = 1.0;
        }
    }
    //Act
    let area = FormationArea::from_mask_data(40, 20, &plane, RectangleF::new(100.0, 0.0, 80.0, 40.0), None).unwrap();
    //Assert
    assert_eq!(area.bounds(), RectangleF::new(120.0, 10.0, 20.0, 20.0));
    assert!(area.contains(130.0, 20.0));
    assert!(!area.contains(119.0, 20.0));
    assert!(area.geometry().is_none());
    assert_eq!(area.distance_to_edge(131.0, 21.0), 10.0);
    assert!(FormationTargets::fill(&area, FormationTargetOptions { spacing: 4.0, edge_clearance: 2.0, ..FormationTargetOptions::default() }).unwrap().len() > 10);
    assert!(FormationArea::from_mask_data(2, 2, &[0.0; 3], RectangleF::new(0.0, 0.0, 1.0, 1.0), None).is_err());
}

#[test]
fn every_arrival_style_starts_where_it_should_and_ends_exactly_on_its_target() {
    //Arrange
    let target = FormationPose::new(50.0, 60.0, 30.0, 0.5, 0.8);
    let start = FormationPose::new(-100.0, -50.0, -90.0, 2.0, 2.0);
    let styles: Vec<Rc<dyn IArrivalPath>> = vec![
        Rc::new(StraightArrival::new(start, target, 1.0, 3.0, None).unwrap()),
        Rc::new(CurvedArrival::new(start, target, 1.0, 3.0, CurvedArrivalOptions { bend: 0.5, ..CurvedArrivalOptions::default() }).unwrap()),
        Rc::new(SpiralArrival::new(target, 1.0, 3.0, 120.0, SpiralArrivalOptions { start_angle: 45.0, turns: 2.0, start_scale: 3.0, spins_with_path: true, ..SpiralArrivalOptions::default() }).unwrap()),
        Rc::new(SwayingFallArrival::new(&mut DrawingRandom::new(5), target, 3.0, -40.0, None).unwrap()),
    ];
    //Assert
    for path in &styles {
        assert_eq!(path.get_pose_at(path.arrival_time()), target);
        assert_eq!(path.get_pose_at(path.arrival_time() + 10.0), target);
        assert!((path.get_pose_at(path.arrival_time() - 0.001).x - 50.0).abs() <= 1.0);
    }
    assert_eq!(styles[0].get_pose_at(1.0), start);
    assert_eq!(styles[1].get_pose_at(0.0).position(), start.position());
    let s = styles[2].get_pose_at(1.0).position();
    assert!(((((s.x - 50.0) * (s.x - 50.0)) + ((s.y - 60.0) * (s.y - 60.0))).sqrt() - 120.0).abs() <= 1e-3);
    assert!((styles[3].get_pose_at(styles[3].start_time()).y - -40.0).abs() <= 0.01);
    assert!(styles[3].start_time() < 3.0 - 0.5);
    assert!(StraightArrival::new(start, target, 2.0, 1.0, None).is_err(), "the arrival must be after the start");
}

#[test]
fn a_swaying_fall_draws_its_character_from_the_random_in_csharps_order() {
    let target = FormationPose::at(0.0, 0.0);
    let fall = SwayingFallArrival::new(&mut DrawingRandom::new(9), target, 5.0, -100.0, None).unwrap();
    let mut r = DrawingRandom::new(9);
    assert_eq!(fall.sink_speed, r.uniform(185.0, 250.0));
    assert_eq!(fall.sway, r.uniform(16.0, 74.0));
    assert_eq!(fall.swing_period, r.uniform(1.5, 2.8));
    assert_eq!(fall.bob, r.uniform(0.3, 0.62));
    let rock = r.uniform(14.0, 38.0);
    assert_eq!(fall.rock, rock * if r.next_double() < 0.5 { -1.0 } else { 1.0 });
}

#[test]
fn a_curved_arrival_bows_to_the_right_of_its_direction() {
    let path = CurvedArrival::new(FormationPose::at(0.0, 0.0), FormationPose::at(100.0, 0.0), 0.0, 1.0, CurvedArrivalOptions { bend: 0.5, easing: EasingKind::Linear }).unwrap();
    // The control point is (50, 50): at e = 0.5 the quadratic gives y = 2 x 0.25 x 50 = 25.
    assert_eq!(path.get_pose_at(0.5).y, 25.0);
}

#[test]
fn the_schedule_starts_and_ends_on_time_and_crowds_its_middle() {
    let schedule = ArrivalSchedule::new(10.0, 40.0, None).unwrap();
    let times = schedule.get_times(301);
    assert_eq!(times[0], 10.0);
    assert_eq!(times[300], 40.0);
    assert!(times.windows(2).all(|w| w[1] >= w[0]));
    assert!(times[180] - times[150] < times[30] - times[0]);
    assert!(schedule.get_times(0).is_empty());
}

#[test]
fn a_schedule_with_a_flat_rate_spaces_evenly_and_the_profile_follows_its_formula() {
    let times = ArrivalSchedule::with_rate(0.0, 10.0, |_| 1.0).unwrap().get_times(11);
    for (i, t) in times.iter().enumerate() {
        assert!((t - i as f64).abs() <= 1e-9);
    }
    let p = ArrivalRateProfile::default();
    assert_eq!(p.get_rate(0.15), 0.06 + (0.94 * 0.5f64.powf(1.6)));
    assert_eq!(p.get_rate(0.5), 1.0);
    assert_eq!(p.get_rate(0.86), (1.0f64 - ((0.86 - 0.72) / (1.0 - 0.72))).powf(2.1) + 0.012);
    assert!(ArrivalSchedule::with_rate(0.0, 1.0, |_| 0.0).is_err());
    assert!(ArrivalSchedule::new(0.0, 1.0, Some(ArrivalRateProfile { taper_from: 0.1, ..ArrivalRateProfile::default() })).is_err());
}

// ---- EXACT AGREEMENT WITH A NoSkia RUN ----
// The expected numbers below were printed by NoSkia itself (CodeBrix.Imaging.Drawing.NoSkia
// 1.0.317.314, the package with these features) running the same calls on the same inputs: an
// FNV-1a 64 hash over the bit patterns of every value, in order, and a few values as bits. They
// pin the Rust port to C#'s arithmetic BIT FOR BIT on the host.

struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(14695981039346656037)
    }
    fn bytes(&mut self, b: &[u8]) {
        for x in b {
            self.0 ^= *x as u64;
            self.0 = self.0.wrapping_mul(1099511628211);
        }
    }
    fn f(&mut self, v: f32) {
        self.bytes(&v.to_bits().to_le_bytes());
    }
    fn d(&mut self, v: f64) {
        self.bytes(&v.to_bits().to_le_bytes());
    }
}

fn targets_hash(targets: &[crate::formations::FormationTarget]) -> u64 {
    let mut h = Fnv::new();
    for t in targets {
        h.f(t.position.x);
        h.f(t.position.y);
        h.f(t.key);
        h.f(t.clearance);
    }
    h.0
}

#[test]
fn targets_equal_a_noskia_run_bit_for_bit_for_every_order() {
    let check = |name: &str, area: &PathGeometry, options: FormationTargetOptions, count: usize, hash: u64, first: (u32, u32, u32)| {
        let t = FormationTargets::fill_geometry(area, options).unwrap();
        assert_eq!((t.len(), targets_hash(&t)), (count, hash), "{name}");
        assert_eq!((t[0].position.x.to_bits(), t[0].position.y.to_bits(), t[0].key.to_bits()), first, "{name}");
    };
    check("random seed 7", &disc(), opts(12.0, 5.0, 7), 233, 0xB25682CA4F82FDB0, (0x42E00962, 0x4350A638, 0x39205000));
    check("center out", &disc(), FormationTargetOptions { order: Some(Box::new(CenterOutArrivalOrder::new(false))), ..FormationTargetOptions::default() }, 129, 0xACA44D835976DC54, (0x4344D358, 0x434D877E, 0x40CC05EC));
    check("directional 30 scatter 2", &disc(), FormationTargetOptions { order: Some(Box::new(DirectionalArrivalOrder::new(30.0, 2.0).unwrap())), ..FormationTargetOptions::default() }, 129, 0x622A3260C51682AE, (0x4307A74D, 0x430C7182, 0x433C9367));
    check("heap default", &cup(), FormationTargetOptions { spacing: 10.0, order: Some(Box::new(HeapArrivalOrder::new(300.0).unwrap())), ..FormationTargetOptions::default() }, 238, 0x5DD3DFF28EBBE96D, (0x43313B23, 0x4393CE76, 0xC0D49841));
    // No jitter: whole rows tie on their key, so this pins .NET's introspective sort too.
    check("ties no jitter", &cup(), FormationTargetOptions { spacing: 10.0, jitter: 0.0, order: Some(Box::new(DirectionalArrivalOrder::new(90.0, 0.0).unwrap())), ..FormationTargetOptions::default() }, 259, 0xACCFFEC713EE4CA9, (0x43910000, 0x42D24CC4, 0x42D24CC4));
}

#[test]
fn schedule_arrivals_and_mask_distances_equal_a_noskia_run_bit_for_bit() {
    let mut h = Fnv::new();
    for t in ArrivalSchedule::new(10.0, 40.0, None).unwrap().get_times(301) {
        h.d(t);
    }
    assert_eq!(h.0, 0x1A05069BEC7E22EA, "schedule");

    let target = FormationPose::new(50.0, 60.0, 30.0, 0.5, 0.8);
    let fall = SwayingFallArrival::new(&mut DrawingRandom::new(5), target, 3.0, -40.0, None).unwrap();
    let mut h = Fnv::new();
    h.d(fall.start_time());
    let mut t = fall.start_time();
    while t < 3.0 {
        let p = fall.get_pose_at(t);
        for v in [p.x, p.y, p.rotation, p.scale_x, p.scale_y] {
            h.f(v);
        }
        t += 0.05;
    }
    assert_eq!(fall.start_time().to_bits(), 0x4001CE57E9EFFC38, "fall start");
    assert_eq!(h.0, 0x1F7490D4A5244D4F, "fall poses (sin, cos, exp: host C library, as .NET's)");

    let spiral = SpiralArrival::new(target, 1.0, 3.0, 120.0, SpiralArrivalOptions { start_angle: 45.0, turns: 2.0, start_scale: 3.0, spins_with_path: true, ..SpiralArrivalOptions::default() }).unwrap();
    let curved = CurvedArrival::new(FormationPose::new(-100.0, -50.0, -90.0, 2.0, 2.0), target, 1.0, 3.0, CurvedArrivalOptions { bend: 0.5, ..CurvedArrivalOptions::default() }).unwrap();
    let mut h = Fnv::new();
    let mut t = 1.0f64;
    while t < 3.0 {
        let p = spiral.get_pose_at(t);
        let q = curved.get_pose_at(t);
        for v in [p.x, p.y, p.rotation, p.scale_x, q.x, q.y, q.rotation, q.scale_y] {
            h.f(v);
        }
        t += 0.1;
    }
    assert_eq!(h.0, 0x1928E311F688CC35, "spiral and curved poses");

    let mut plane = vec![0.0f32; 30 * 20];
    for y in 0..20i32 {
        for x in 0..30i32 {
            plane[(y * 30 + x) as usize] = if ((x - 12) * (x - 12)) + ((y - 9) * (y - 9)) < 50 { 1.0 } else { 0.0 };
        }
    }
    let mask = FormationArea::from_mask_data(30, 20, &plane, RectangleF::new(10.0, 20.0, 90.0, 50.0), None).unwrap();
    let mut h = Fnv::new();
    let mut y = 20.5f32;
    while y < 70.0 {
        let mut x = 10.5f32;
        while x < 100.0 {
            h.f(mask.distance_to_edge(x, y));
            x += 2.3;
        }
        y += 1.7;
    }
    assert_eq!(h.0, 0xDD29DC9F8CC5C7B7, "mask distances");
    assert_eq!((mask.bounds().x.to_bits(), mask.bounds().width.to_bits()), (0x41C80000, 0x42340000));

    let mut h = Fnv::new();
    for i in 0..200 {
        h.d(crate::planes::DrawingNoise::gradient(i as f64 * 0.137, 0.37, 4));
    }
    assert_eq!(h.0, 0xA9FA95A190107648, "gradient noise");
}
