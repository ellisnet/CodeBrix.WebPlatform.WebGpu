//! Tests of the ripple field: rings, decay, superposition, the swell, walls, rain as a function
//! of time, the live-drop cap, seeking exactness and the GPU packing.

use std::collections::BTreeSet;

use crate::water::ripple_math::{self, records_to_bytes, RippleGpuHeader, FLAG_WALLS};
use crate::water::{RippleDrop, RippleField, RippleOptions, RippleRain, RippleSwell, SwellWave};
use crate::{PointF, RectangleF};

fn pool() -> RectangleF {
    RectangleF::new(0.0, 0.0, 800.0, 600.0)
}

fn field() -> RippleField {
    RippleField::new(pool(), RippleOptions::default()).unwrap()
}

fn one_drop(x: f32, y: f32, time: f64) -> RippleField {
    field().with_drop(RippleDrop::new(PointF::new(x, y), time)).unwrap()
}

fn key(d: &RippleDrop) -> (u64, u32, u32, u32, u32) {
    (d.time.to_bits(), d.center.x.to_bits(), d.center.y.to_bits(), d.strength.to_bits(), d.size.to_bits())
}

#[test]
fn ripple_height_is_zero_before_a_drop_lands_and_after_its_life() {
    //Arrange
    let f = one_drop(400.0, 300.0, 2.0);
    let life = f.drop_life();
    //Act
    let mut before = 0.0f32;
    let mut after = 0.0f32;
    let mut during = 0.0f32;
    for i in 0..400 {
        let x = 400.0 + i as f32;
        before = before.max(f.height_at(x, 300.0, 1.999).abs());
        after = after.max(f.height_at(x, 300.0, 2.0 + life + 0.001).abs());
        during = during.max(f.height_at(x, 300.0, 3.0).abs());
    }
    //Assert
    assert_eq!(before, 0.0);
    assert_eq!(after, 0.0);
    assert!(during > 0.1, "{during}");
    assert!((life - 1.8f32 as f64 * 100f64.ln()).abs() < 1e-12);
}

#[test]
fn ripple_ring_radius_is_speed_times_age_within_the_life() {
    //Arrange
    let f = field();
    let d = RippleDrop::new(PointF::new(10.0, 10.0), 1.0);
    //Act / Assert
    assert_eq!(f.ring_radius(&d, 0.99), None);
    assert_eq!(f.ring_radius(&d, 1.0), Some(0.0));
    assert_eq!(f.ring_radius(&d, 2.5), Some(135.0));
    assert_eq!(f.ring_radius(&d, 1.0 + f.drop_life()), None);
}

#[test]
fn ripple_ring_front_trough_and_crest_found_numerically_match_ring_radius() {
    //Arrange
    let f = one_drop(0.0, 0.0, 0.0);
    let t = 1.5;
    let radius = f.ring_radius(&f.drops()[0], t).unwrap();
    let lambda = 16.0f32;
    //Act: walk outward along +x in 1/100 unit steps.
    let (mut trough, mut trough_h) = (0.0f32, f32::MAX);
    let (mut crest, mut crest_h) = (0.0f32, f32::MIN);
    let mut zero = None;
    let mut previous = f.height_at(radius - 3.0 * lambda, 0.0, t);
    let mut x = radius - 3.0 * lambda;
    while x < radius + 2.0 * lambda {
        x += 0.01;
        let h = f.height_at(x, 0.0, t);
        if h < trough_h {
            (trough, trough_h) = (x, h);
        }
        if x > radius - lambda / 8.0 && h > crest_h {
            (crest, crest_h) = (x, h);
        }
        if previous < 0.0 && h >= 0.0 && (x - radius).abs() < lambda / 4.0 {
            zero = Some(x);
        }
        previous = h;
    }
    //Assert
    let zero = zero.expect("a rising zero crossing near the front");
    assert!((zero - radius).abs() < lambda / 10.0, "front {zero} vs {radius}");
    assert!((trough - (radius - lambda / 4.0)).abs() < lambda / 10.0, "trough {trough} vs {}", radius - lambda / 4.0);
    assert!((crest - (radius + 0.15 * lambda)).abs() < lambda / 10.0, "crest {crest} vs {}", radius + 0.15 * lambda);
}

#[test]
fn ripple_slope_is_the_derivative_of_the_height() {
    //Arrange
    let f = one_drop(300.0, 260.0, 0.0).with_drop(RippleDrop { center: PointF::new(420.0, 330.0), time: 0.4, strength: 0.7, size: 1.2 }).unwrap().with_swell(RippleSwell::gentle(3, 120.0, 0.6)).unwrap();
    let t = 1.3;
    let step = 0.05f32;
    let mut worst = 0.0f32;
    //Act
    for j in 0..40 {
        for i in 0..40 {
            let (x, y) = (250.0 + i as f32 * 6.3, 200.0 + j as f32 * 5.7);
            let (sx, sy) = f.slope_at(x, y, t);
            let nx = (f.height_at(x + step, y, t) - f.height_at(x - step, y, t)) / (2.0 * step);
            let ny = (f.height_at(x, y + step, t) - f.height_at(x, y - step, t)) / (2.0 * step);
            worst = worst.max((sx - nx).abs()).max((sy - ny).abs());
        }
    }
    //Assert
    assert!(worst < 1e-3, "slope against the numeric derivative: {worst}");
}

#[test]
fn ripple_focus_is_the_laplacian_of_the_height() {
    //Arrange
    let f = one_drop(300.0, 260.0, 0.0).with_swell(RippleSwell::gentle(8, 140.0, 0.4)).unwrap();
    let t = 1.1;
    let step = 0.25f32;
    let (mut worst, mut largest) = (0.0f32, 0.0f32);
    //Act
    for j in 0..30 {
        for i in 0..30 {
            let (x, y) = (300.0 + i as f32 * 4.9, 260.0 + j as f32 * 4.3);
            let c = f.height_at(x, y, t);
            let numeric = (f.height_at(x + step, y, t) + f.height_at(x - step, y, t) + f.height_at(x, y + step, t) + f.height_at(x, y - step, t) - 4.0 * c) / (step * step);
            let focus = f.focus_at(x, y, t);
            worst = worst.max((focus - numeric).abs());
            largest = largest.max(focus.abs());
        }
    }
    //Assert: exact per ring, so within the numeric Laplacian's own rounding and truncation.
    assert!(largest > 0.02, "{largest}");
    assert!(worst < 0.02 * largest + 2e-3, "focus against the numeric Laplacian: {worst} (largest {largest})");
}

#[test]
fn ripple_decay_divides_the_amplitude_by_e_every_decay_seconds() {
    //Arrange
    let f = one_drop(100.0, 100.0, 0.0);
    let d = f.drops()[0];
    //Act
    let a1 = f.amplitude(&d, 1.0);
    let a2 = f.amplitude(&d, 1.0 + 1.8f32 as f64);
    //Assert
    assert!((a2 / a1 - (-1.0f64).exp()).abs() < 1e-12);
    // The ease: nothing at landing, full after one period (wavelength / speed).
    assert_eq!(f.amplitude(&d, 0.0), 0.0);
    assert!(f.amplitude(&d, 16.0 / 90.0 / 2.0) < f.amplitude(&d, 16.0 / 90.0));
    assert!((f.amplitude(&d, 16.0 / 90.0) - (-(16.0f64 / 90.0) / 1.8f32 as f64).exp()).abs() < 1e-9);
    assert!(f.live_drops(0.0).is_empty(), "a drop with no amplitude yet is not sent");
}

#[test]
fn ripple_drops_superpose_linearly() {
    //Arrange
    let a = RippleDrop::new(PointF::new(300.0, 300.0), 0.2);
    let b = RippleDrop { center: PointF::new(360.0, 280.0), time: 0.5, strength: 0.6, size: 0.8 };
    let fa = field().with_drop(a).unwrap();
    let fb = field().with_drop(b).unwrap();
    let both = field().with_drops(vec![b, a]).unwrap();
    //Act / Assert
    for i in 0..200 {
        let (x, y) = (250.0 + i as f32 * 0.9, 240.0 + i as f32 * 0.4);
        let sum = fa.height_at(x, y, 1.2) + fb.height_at(x, y, 1.2);
        assert!((both.height_at(x, y, 1.2) - sum).abs() < 1e-5);
        let (s1, s2) = (fa.slope_at(x, y, 1.2), fb.slope_at(x, y, 1.2));
        let s = both.slope_at(x, y, 1.2);
        assert!((s.0 - s1.0 - s2.0).abs() < 1e-5 && (s.1 - s1.1 - s2.1).abs() < 1e-5);
    }
    assert_eq!(both.drops()[0], a, "drops are kept sorted by time");
}

#[test]
fn ripple_swell_waves_travel_at_their_speed() {
    //Arrange
    let wave = SwellWave { angle: 30.0, wavelength: 100.0, height: 0.8, speed: 25.0, phase: 0.4 };
    let f = field().with_swell(RippleSwell::new(vec![wave])).unwrap();
    let (dx, dy) = (30f32.to_radians().cos(), 30f32.to_radians().sin());
    let dt = 0.7f64;
    //Act / Assert: h(p, t) = h(p - v dt d, t - dt)
    for i in 0..100 {
        let (x, y) = (100.0 + i as f32 * 3.1, 120.0 + i as f32 * 1.7);
        let now = f.height_at(x, y, 5.0);
        let earlier = f.height_at(x - 25.0 * dt as f32 * dx, y - 25.0 * dt as f32 * dy, 5.0 - dt);
        assert!((now - earlier).abs() < 2e-4, "{now} vs {earlier}");
    }
    // At time 0 the wave is its formula.
    let h0 = f.height_at(10.0, 20.0, 0.0);
    let k = std::f32::consts::TAU / 100.0;
    assert!((h0 - 0.8 * (k * (dx * 10.0 + dy * 20.0) + 0.4).sin()).abs() < 1e-5);
}

#[test]
fn ripple_gentle_swell_is_four_seeded_waves_within_the_height() {
    //Arrange
    let a = RippleSwell::gentle(42, 120.0, 0.9);
    let b = RippleSwell::gentle(42, 120.0, 0.9);
    let c = RippleSwell::gentle(43, 120.0, 0.9);
    //Act
    let total: f32 = a.waves.iter().map(|w| w.height).sum();
    //Assert
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.waves.len(), 4);
    assert_eq!(a.waves[0].wavelength, 120.0);
    assert!((total - 0.9).abs() < 1e-5);
    assert!(a.waves.iter().all(|w| w.speed > 0.0 && (0.0..360.0).contains(&w.angle)));
}

#[test]
fn ripple_walls_reflect_so_the_normal_slope_at_a_wall_is_zero() {
    //Arrange: a drop near the left wall; at 1 s its ring has crossed the wall.
    let options = RippleOptions { walls: true, ..RippleOptions::default() };
    let walled = RippleField::new(pool(), options).unwrap().with_drop(RippleDrop::new(PointF::new(40.0, 300.0), 0.0)).unwrap();
    let open = one_drop(40.0, 300.0, 0.0);
    let (mut normal, mut along, mut open_normal) = (0.0f32, 0.0f32, 0.0f32);
    //Act
    for j in 0..300 {
        let y = 150.0 + j as f32;
        let (sx, sy) = walled.slope_at(0.0, y, 1.0);
        normal = normal.max(sx.abs());
        along = along.max(sy.abs());
        open_normal = open_normal.max(open.slope_at(0.0, y, 1.0).0.abs());
    }
    //Assert
    assert!(open_normal > 0.01, "without walls the ring crosses the edge: {open_normal}");
    assert!(along > 0.01, "{along}");
    assert!(normal < 1e-4 * open_normal.max(1.0) + 1e-5, "normal slope at the wall: {normal}");
    // The walled field is the open field of the drop and its eight mirror images.
    let images: Vec<RippleDrop> = [-40.0f32, 40.0, 1560.0].iter().flat_map(|&x| [-300.0f32, 300.0, 900.0].map(move |y| RippleDrop::new(PointF::new(x, y), 0.0))).collect();
    let mirrored = field().with_drops(images).unwrap();
    for i in 0..100 {
        let (x, y) = (i as f32 * 2.0, 200.0 + i as f32);
        assert!((walled.height_at(x, y, 1.0) - mirrored.height_at(x, y, 1.0)).abs() < 1e-5);
    }
}

#[test]
fn ripple_rain_found_directly_equals_rain_found_by_stepping() {
    //Arrange
    let f = field().with_rain(RippleRain { rate: 6.0, seed: 17, ..RippleRain::default() }).unwrap();
    let life = f.drop_life();
    //Act: every drop seen alive while stepping 0..12 in 1/60 s, then kept if alive at 12.
    let mut seen = BTreeSet::new();
    let mut seen_drops = Vec::new();
    for n in 0..=720 {
        for d in f.live_drops(n as f64 / 60.0) {
            if seen.insert(key(&d)) {
                seen_drops.push(d);
            }
        }
    }
    let stepped: BTreeSet<_> = seen_drops.iter().filter(|d| 12.0 - d.time < life && d.time <= 12.0).map(key).collect();
    let direct: BTreeSet<_> = f.live_drops(12.0).iter().map(key).collect();
    //Assert
    assert!(direct.len() > 20, "{}", direct.len());
    assert_eq!(direct, stepped);
}

#[test]
fn ripple_rain_drop_from_one_window_is_still_alive_in_the_next() {
    //Arrange
    let f = field().with_rain(RippleRain { rate: 3.0, seed: 5, ..RippleRain::default() }).unwrap();
    let at_4 = f.live_drops(4.999);
    let late = at_4.iter().copied().filter(|d| d.time >= 4.0 && d.time < 4.999).max_by(|a, b| a.time.total_cmp(&b.time)).expect("a drop in window 4");
    //Act
    let at_5 = f.live_drops(5.001);
    let at_12 = f.live_drops(4.0 + f.drop_life() + 1.0);
    //Assert
    assert!(at_5.iter().any(|d| key(d) == key(&late)), "the drop of window 4 lives on in window 5");
    assert!(at_12.iter().all(|d| key(d) != key(&late)), "and is gone after its life");
    let r = f.ring_radius(&late, 5.3).unwrap();
    assert!(f.height_at(late.center.x + r - 4.0, late.center.y, 5.3).abs() > 0.0);
}

#[test]
fn ripple_rain_mean_count_per_window_approaches_the_rate() {
    //Arrange
    let rain = RippleRain { rate: 2.5, seed: 99, ..RippleRain::default() };
    let mut out = Vec::new();
    //Act
    for k in 0..4000 {
        rain.window(k, pool(), &mut out);
    }
    //Assert
    let mean = out.len() as f64 / 4000.0;
    assert!((mean - 2.5).abs() < 0.1, "{mean}");
    assert!(out.iter().all(|d| (0.0..800.0).contains(&d.center.x) && (0.0..600.0).contains(&d.center.y)));
    assert!(out.iter().all(|d| (0.4..=1.0).contains(&d.strength) && (0.7..=1.3).contains(&d.size)));
}

#[test]
fn ripple_rain_same_seed_same_drops_and_start_end_respected() {
    //Arrange
    let base = RippleRain { rate: 4.0, seed: 3, ..RippleRain::default() };
    let windowed = RippleRain { start: 2.5, end: Some(6.25), ..base };
    let other = RippleRain { seed: 4, ..base };
    let (mut a, mut b, mut c, mut w) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    //Act
    for k in -2..10 {
        base.window(k, pool(), &mut a);
        base.window(k, pool(), &mut b);
        other.window(k, pool(), &mut c);
        windowed.window(k, pool(), &mut w);
    }
    //Assert
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert!(a.iter().all(|d| d.time >= 0.0), "the default start is 0");
    let kept: Vec<RippleDrop> = a.iter().copied().filter(|d| d.time >= 2.5 && d.time < 6.25).collect();
    assert_eq!(w, kept, "start and end leave the other drops where they were");
}

#[test]
fn ripple_rain_area_is_respected() {
    //Arrange
    let area = RectangleF::new(100.0, 50.0, 20.0, 10.0);
    let rain = RippleRain { rate: 10.0, area: Some(area), seed: 1, ..RippleRain::default() };
    let mut out = Vec::new();
    //Act
    for k in 0..50 {
        rain.window(k, pool(), &mut out);
    }
    //Assert
    assert!(!out.is_empty());
    assert!(out.iter().all(|d| (100.0..120.0).contains(&d.center.x) && (50.0..60.0).contains(&d.center.y)));
}

#[test]
fn ripple_live_drops_are_capped_at_the_strongest() {
    //Arrange: 300 drops, strengths rising with the index, all landed at 0.
    let drops: Vec<RippleDrop> = (0..300).map(|i| RippleDrop { center: PointF::new(i as f32, 10.0), time: 0.0, strength: 0.001 * (i + 1) as f32, size: 1.0 }).collect();
    let f = field().with_drops(drops).unwrap();
    //Act
    let (live, skipped) = f.select_live(1.0);
    //Assert
    assert_eq!(live.len(), RippleField::MAX_LIVE_DROPS);
    assert_eq!(skipped, 300 - RippleField::MAX_LIVE_DROPS);
    assert!(live.iter().all(|d| d.strength > 0.001 * 44.5), "the weakest 44 are the ones left out");
    assert!(live.windows(2).all(|w| w[0].center.x < w[1].center.x), "the kept ones keep their order");
    assert_eq!(f.pack(1.0, 0).skipped, 44);
}

#[test]
fn ripple_field_at_a_time_is_the_same_however_it_is_reached() {
    //Arrange
    let f = one_drop(200.0, 200.0, 1.0).with_drop(RippleDrop::new(PointF::new(500.0, 350.0), 9.5)).unwrap().with_rain(RippleRain::new(5.0, 8)).unwrap().with_swell(RippleSwell::gentle(1, 150.0, 0.5)).unwrap();
    let points: Vec<(f32, f32)> = (0..50).map(|i| (120.0 + i as f32 * 11.0, 90.0 + i as f32 * 8.0)).collect();
    let direct: Vec<u32> = points.iter().map(|&(x, y)| f.height_at(x, y, 12.0).to_bits()).collect();
    //Act: play forward to 12, then come back to 12 from later, and on a clone.
    for n in 0..=144 {
        let _ = f.height_at(300.0, 300.0, n as f64 / 12.0);
    }
    let played: Vec<u32> = points.iter().map(|&(x, y)| f.height_at(x, y, 12.0).to_bits()).collect();
    let _ = f.height_at(1.0, 1.0, 30.0);
    let copy = f.clone();
    let seeked: Vec<u32> = points.iter().map(|&(x, y)| copy.height_at(x, y, 12.0).to_bits()).collect();
    //Assert
    assert_eq!(direct, played);
    assert_eq!(direct, seeked);
    assert!(direct.iter().any(|&b| f32::from_bits(b) != 0.0));
}

#[test]
fn ripple_packing_records_and_header() {
    //Arrange
    let swell = RippleSwell::new(vec![SwellWave { angle: 90.0, wavelength: 50.0, height: 0.3, speed: 10.0, phase: 1.0 }]);
    let options = RippleOptions { walls: true, ..RippleOptions::default() };
    let f = RippleField::new(RectangleF::new(10.0, 20.0, 300.0, 200.0), options).unwrap().with_drop(RippleDrop { center: PointF::new(50.0, 60.0), time: 1.0, strength: 0.5, size: 2.0 }).unwrap().with_swell(swell).unwrap();
    //Act
    let packed = f.pack(2.0, 7);
    let bytes = records_to_bytes(&packed.records);
    let header = packed.header.to_bytes();
    //Assert
    assert_eq!(packed.header, RippleGpuHeader { place: [10.0, 20.0, 310.0, 220.0], words: [7, 1, 1, FLAG_WALLS] });
    assert_eq!(packed.records.len(), 3);
    assert_eq!(bytes.len(), 48, "a drop is 32 bytes, a swell wave 16");
    let amplitude = (0.5 * (-1.0f64 / 1.8).exp()) as f32;
    assert_eq!(packed.records[0], [50.0, 60.0, 90.0, amplitude]);
    assert_eq!(packed.records[1], [32.0, 3.0 * 32.0 / 2.0, 1.0, 0.0]);
    let k = std::f64::consts::TAU / 50.0;
    let phase = (1.0 - k * 10.0 * 2.0).rem_euclid(std::f64::consts::TAU) as f32;
    assert!((packed.records[2][0]).abs() < 1e-6 && (packed.records[2][1] - k as f32).abs() < 1e-6);
    assert_eq!((packed.records[2][2], packed.records[2][3]), (0.3, phase));
    assert_eq!(&bytes[12..16], &amplitude.to_le_bytes());
    assert_eq!(&bytes[32..36], &packed.records[2][0].to_le_bytes());
    assert_eq!(&header[0..4], &10f32.to_le_bytes());
    assert_eq!(&header[12..16], &220f32.to_le_bytes());
    assert_eq!(&header[16..20], &7u32.to_le_bytes());
    assert_eq!(&header[28..32], &FLAG_WALLS.to_le_bytes());
    // Evaluating the packed records at an offset into a larger buffer gives the field's values.
    let mut buffer = vec![[0.0f32; 4]; 7];
    buffer.extend_from_slice(&packed.records);
    let s = ripple_math::evaluate(&packed.header, &buffer, 120.0, 80.0);
    assert_eq!(s.height.to_bits(), f.height_at(120.0, 80.0, 2.0).to_bits());
}

#[test]
fn ripple_checks_say_what_is_wrong() {
    assert!(RippleField::new(RectangleF::new(0.0, 0.0, 0.0, 10.0), RippleOptions::default()).unwrap_err().contains("place"));
    assert!(RippleField::new(pool(), RippleOptions { speed: 0.0, ..RippleOptions::default() }).unwrap_err().contains("speed"));
    assert!(RippleField::new(pool(), RippleOptions { decay: f32::NAN, ..RippleOptions::default() }).unwrap_err().contains("decay"));
    assert!(field().with_drop(RippleDrop { size: 0.0, ..RippleDrop::default() }).unwrap_err().contains("size"));
    assert!(field().with_drops(vec![RippleDrop::default(), RippleDrop { strength: -1.0, ..RippleDrop::default() }]).unwrap_err().starts_with("Drop 1"));
    assert!(field().with_rain(RippleRain { rate: 500.0, ..RippleRain::default() }).unwrap_err().contains("rate"));
    assert!(field().with_rain(RippleRain { end: Some(-1.0), ..RippleRain::default() }).unwrap_err().contains("end"));
    assert!(field().with_rain(RippleRain { size: (0.0, 1.0), ..RippleRain::default() }).unwrap_err().contains("size"));
    assert!(field().with_swell(RippleSwell::new(vec![SwellWave::default(); 9])).unwrap_err().contains("at most 8"));
    assert!(field().with_swell(RippleSwell::new(vec![SwellWave { wavelength: -1.0, ..SwellWave::default() }])).unwrap_err().starts_with("Swell wave 0"));
}
