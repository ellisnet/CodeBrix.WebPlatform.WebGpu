//! Host tests of the water's CPU arithmetic (Not in C#): the wave equation's step against hand
//! computed cells, energy and damping, ring speed, walls, absorbing edges, wrapping, masks, the
//! fixed-rate clock and interpolation, the disturbances, the picture, the uniform block, rain,
//! masks from pictures, stirring, and the Gerstner waves against their formulas.

use codebrix_webgpu::{CanvasInput, DecodedImage, ModifierKeys, PointerButton, PointerButtons, PointerKind};
use glam::{Vec2, Vec3, Vec4};

use crate::water::gerstner_math;
use crate::water::wave_math::{self, check_options, coefficients, disturb_grid, path_profile, publish_grid, step_count, step_cell, step_grid, Coefficients, WaveCore, WaveParams, PARAMS_SIZE};
use crate::water::wave_rain::{rain_drop, RainState};
use crate::water::{CpuWaveField, GerstnerWave, OpenWaterWaves, WaveDrop, WaveEdge, WaveFieldOptions, WaveMask, WaveRain, WaveStir, WaveWake};

/// Options of a small test grid.
fn options(columns: u32, rows: u32, cell: f32, speed: f32) -> WaveFieldOptions {
    WaveFieldOptions { columns, rows, size: Vec2::new(columns as f32 * cell, rows as f32 * cell), wave_speed: speed, damping: 1.0, ..WaveFieldOptions::default() }
}

/// A grid run by the test itself (two states, the mask, the numbers).
struct Grid {
    columns: u32,
    rows: u32,
    c: Coefficients,
    size: Vec2,
    current: Vec<f32>,
    previous: Vec<f32>,
    mask: Option<Vec<f32>>,
}

impl Grid {
    fn new(options: &WaveFieldOptions) -> Grid {
        check_options(options).unwrap();
        let cells = (options.columns * options.rows) as usize;
        Grid { columns: options.columns, rows: options.rows, c: coefficients(options), size: options.size, current: vec![0.0; cells], previous: vec![0.0; cells], mask: None }
    }

    fn drop(&mut self, position: Vec2, radius: f32, depth: f32) {
        disturb_grid(&mut self.current, &mut self.previous, self.mask.as_deref(), self.columns, self.rows, &self.c, self.size, &[Vec4::new(position.x, position.y, radius, depth)], &[]);
    }

    /// A plane pulse `amplitude * exp(-((x - x0) / width)^2)` moving toward +x at the wave speed.
    fn plane_pulse(&mut self, x0: f32, width: f32, amplitude: f32, speed: f32) {
        for j in 0..self.rows {
            for i in 0..self.columns {
                let x = (i as f32 + 0.5) * self.c.cell.x;
                let index = (j * self.columns + i) as usize;
                self.current[index] = amplitude * (-((x - x0) / width).powi(2)).exp();
                self.previous[index] = amplitude * (-((x + speed * self.c.dt - x0) / width).powi(2)).exp();
            }
        }
    }

    fn run(&mut self, steps: u32) {
        for _ in 0..steps {
            step_grid(&self.current, &mut self.previous, self.mask.as_deref(), self.columns, self.rows, &self.c);
            std::mem::swap(&mut self.current, &mut self.previous);
        }
    }

    fn at(&self, i: u32, j: u32) -> f32 {
        self.current[(j * self.columns + i) as usize]
    }

    /// The discrete energy the undamped leapfrog keeps: kinetic `sum (h - p)^2` plus the
    /// potential `wx sum dx(h) dx(p) + wy sum dy(h) dy(p)` over the neighbouring pairs inside
    /// the grid (the walls add none).
    fn energy(&self) -> f64 {
        let (h, p) = (&self.current, &self.previous);
        let (w, rows) = (self.columns as usize, self.rows as usize);
        let mut kinetic = 0.0f64;
        let mut potential = 0.0f64;
        for j in 0..rows {
            for i in 0..w {
                let k = j * w + i;
                kinetic += ((h[k] - p[k]) as f64).powi(2);
                if i + 1 < w {
                    potential += self.c.wx as f64 * ((h[k + 1] - h[k]) as f64) * ((p[k + 1] - p[k]) as f64);
                }
                if j + 1 < rows {
                    potential += self.c.wy as f64 * ((h[k + w] - h[k]) as f64) * ((p[k + w] - p[k]) as f64);
                }
            }
        }
        kinetic + potential
    }
}

// ---- the step ---------------------------------------------------------------------------------

#[test]
fn one_step_matches_hand_computed_cells() {
    // 8 x 8, cell 0.5, speed 24, 120 steps/s: wx = wy = (24 / 120 / 0.5)^2 = 0.16.
    let o = options(8, 8, 0.5, 24.0);
    let mut g = Grid::new(&o);
    assert!((g.c.wx - 0.16).abs() < 1e-6 && (g.c.wy - 0.16).abs() < 1e-6);
    assert_eq!((g.c.kept, g.c.level), (1.0, 1.0));
    g.current[3 * 8 + 3] = 1.0;
    g.previous[3 * 8 + 3] = 1.0;
    g.run(1);
    // The centre: 1 + 0 + 0.16 * (0 + 0 - 2) * 2 = 0.36; its four neighbours: 0.16 * 1 each.
    assert!((g.at(3, 3) - 0.36).abs() < 1e-6, "{}", g.at(3, 3));
    for (i, j) in [(2, 3), (4, 3), (3, 2), (3, 4)] {
        assert!((g.at(i, j) - 0.16).abs() < 1e-6);
    }
    assert_eq!(g.at(2, 2), 0.0);
    // A corner cell (Reflect: its missing neighbours are itself): next = 2h - p + 0.16 (R - h) + 0.16 (D - h).
    let c = g.c;
    assert!((step_cell(1.0, 0.5, 1.0, 0.0, 1.0, 0.2, 0.0, 0.0, &c) - (2.0 - 0.5 + 0.16 * (1.0 + 0.0 - 2.0) + 0.16 * (1.0 + 0.2 - 2.0))).abs() < 1e-6);
    // A solid cell writes 0; shallows and the absorbing band damp the velocity (h - p).
    assert_eq!(step_cell(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, &c), 0.0);
    let half = step_cell(1.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.5, 0.0, &c);
    assert!((half - (1.0 + 0.5 * (1.0 - 0.5 * c.shore))).abs() < 1e-6);
    let band = step_cell(1.0, 0.5, 1.0, 1.0, 1.0, 1.0, 0.0, 0.25, &c);
    assert!((band - (1.0 + 0.5 * 0.75)).abs() < 1e-6);
}

#[test]
fn the_neighbour_rule_follows_the_edge_and_the_mask() {
    let values = [1.0f32, 2.0, 3.0, 4.0];
    let value = |i: usize| values[i];
    // 2 x 2: outside a Reflect grid the centre stands in, a wrapping grid uses the opposite edge.
    assert_eq!(wave_math::neighbour(&value, None, 2, 2, false, -1, 0, 9.0), 9.0);
    assert_eq!(wave_math::neighbour(&value, None, 2, 2, true, -1, 0, 9.0), 2.0);
    assert_eq!(wave_math::neighbour(&value, None, 2, 2, true, 0, 2, 9.0), 1.0);
    let mask = [0.0, 1.0, 0.5, 0.0];
    assert_eq!(wave_math::neighbour(&value, Some(&mask), 2, 2, false, 1, 0, 9.0), 9.0);
    assert_eq!(wave_math::neighbour(&value, Some(&mask), 2, 2, false, 0, 1, 9.0), 3.0);
}

#[test]
fn creation_refuses_a_speed_too_fast_for_the_rate_and_names_the_rate() {
    let defaults = WaveFieldOptions::default();
    let c = coefficients(&defaults);
    assert!((c.wx + c.wy - 0.091).abs() < 0.001, "{}", c.wx + c.wy);
    assert!(check_options(&defaults).is_ok());
    let fast = WaveFieldOptions { wave_speed: 3.0, ..defaults.clone() };
    let message = check_options(&fast).unwrap_err().message().to_string();
    // wx + wy = 9 * 1311 / 14400 = 0.82 > 0.5: the lowest rate is 3 * sqrt(2 * 1310.72) = 153.6 -> 154.
    assert!(message.contains("at least 154"), "{message}");
    assert!(message.contains("at most 2.344"), "{message}");
    let mut core = WaveCore::new(&defaults).unwrap();
    assert!(core.set_wave_speed(3.0).is_err());
    assert_eq!(core.options().wave_speed, 1.0);
    assert!(core.set_wave_speed(2.3).is_ok());
    assert!(check_options(&WaveFieldOptions { columns: 4, ..defaults.clone() }).is_err());
    assert!(check_options(&WaveFieldOptions { damping: 1.5, ..defaults.clone() }).is_err());
    assert!(check_options(&WaveFieldOptions { max_steps_per_frame: 0, ..defaults }).is_err());
}

// ---- energy and damping -----------------------------------------------------------------------

#[test]
fn undamped_energy_is_kept_over_two_thousand_steps() {
    let o = options(48, 48, 0.1, 5.0);
    let mut g = Grid::new(&o);
    g.drop(Vec2::new(2.0, 2.6), 0.3, 0.1);
    let start = g.energy();
    assert!(start > 0.0);
    let mut worst = 0.0f64;
    for _ in 0..20 {
        g.run(100);
        worst = worst.max((g.energy() / start - 1.0).abs());
    }
    assert!(worst < 0.01, "energy drifted by {worst}");
}

#[test]
fn damping_leaves_the_stated_fraction_of_a_wave_after_each_second() {
    let undamped = options(64, 64, 0.1, 1.0);
    let damped = WaveFieldOptions { damping: 0.6, ..undamped.clone() };
    let (mut a, mut b) = (Grid::new(&undamped), Grid::new(&damped));
    a.drop(Vec2::new(3.2, 3.2), 0.3, 0.1);
    b.drop(Vec2::new(3.2, 3.2), 0.3, 0.1);
    for second in 1..=3 {
        a.run(120);
        b.run(120);
        // The wave's height from its energy (kinetic and potential together, so the phase of
        // each mode does not matter).
        let ratio = (b.energy() / a.energy()).sqrt();
        let expected = 0.6f64.powi(second);
        assert!((ratio / expected - 1.0).abs() < 0.02, "after {second} s: {ratio} against {expected}");
    }
}

#[test]
fn a_central_drop_makes_a_ring_at_the_wave_speed() {
    let o = options(128, 128, 0.1, 1.0);
    let mut g = Grid::new(&o);
    g.drop(Vec2::new(6.45, 6.45), 0.2, 0.1);
    let mut elapsed = 0.0;
    for t in [1.0f32, 2.0, 3.0, 4.0] {
        g.run(((t - elapsed) * 120.0) as u32);
        elapsed = t;
        // The leading trough of the ring (the drop pressed down) along the row through the
        // centre (cell 64), outward from it.
        let ring = (64 + 3..128).min_by(|&a, &b| g.at(a, 64).total_cmp(&g.at(b, 64))).unwrap();
        let radius = (ring as f32 + 0.5) * 0.1 - 6.45;
        assert!((radius - t).abs() <= 0.2, "at {t} s the ring is at {radius}");
    }
}

// ---- edges and masks --------------------------------------------------------------------------

#[test]
fn a_reflect_wall_sends_a_pulse_back_with_the_same_sign() {
    let o = options(128, 8, 0.1, 4.0);
    let mut g = Grid::new(&o);
    g.plane_pulse(3.2, 0.3, 1.0, 4.0);
    // To the wall at 12.8 and back to 3.2: 19.2 units at 4 per second = 4.8 s.
    g.run(576);
    let peak = (0..128).max_by(|&a, &b| g.at(a, 4).abs().total_cmp(&g.at(b, 4).abs())).unwrap();
    let x = (peak as f32 + 0.5) * 0.1;
    assert!((x - 3.2).abs() < 0.3, "the reflected pulse is at {x}");
    assert!(g.at(peak, 4) > 0.5, "{}", g.at(peak, 4));
}

#[test]
fn an_absorbing_edge_returns_less_than_five_per_cent() {
    let reflect = options(128, 128, 0.1, 2.0);
    let absorb = WaveFieldOptions { edge: WaveEdge::ABSORB, ..reflect.clone() };
    let (mut a, mut b) = (Grid::new(&reflect), Grid::new(&absorb));
    a.drop(Vec2::new(6.4, 6.4), 0.25, 0.1);
    b.drop(Vec2::new(6.4, 6.4), 0.25, 0.1);
    let start = b.energy();
    // Out to the edges and back past the centre: 6.4 s at 2 units per second.
    a.run(768);
    b.run(768);
    assert!((a.energy() / start - 1.0).abs() < 0.01);
    let returned = b.energy() / start;
    assert!(returned < 0.05, "{returned}");
}

#[test]
fn a_wrapping_field_lets_a_pulse_leave_right_and_enter_left() {
    let o = WaveFieldOptions { edge: WaveEdge::Wrap, ..options(128, 8, 0.1, 4.0) };
    let mut g = Grid::new(&o);
    g.plane_pulse(10.0, 0.3, 1.0, 4.0);
    g.run(120);
    let peak = (0..128).max_by(|&a, &b| g.at(a, 4).abs().total_cmp(&g.at(b, 4).abs())).unwrap();
    let x = (peak as f32 + 0.5) * 0.1;
    assert!((x - 1.2).abs() < 0.3, "the pulse is at {x}");
    assert!(g.at(peak, 4) > 0.8);
}

#[test]
fn a_solid_disc_reflects_a_pulse_and_stays_dry() {
    let o = options(128, 128, 0.1, 2.0);
    let mask = WaveMask::from_fn(128, 128, |u, v| if Vec2::new(u, v).distance(Vec2::splat(0.5)) < 1.0 / 12.8 { 1.0 } else { 0.0 });
    let (mut open, mut walled) = (Grid::new(&o), Grid::new(&o));
    walled.mask = Some(mask.cells().to_vec());
    open.plane_pulse(2.0, 0.3, 1.0, 2.0);
    walled.plane_pulse(2.0, 0.3, 1.0, 2.0);
    // The pulse front reaches x = 8.4 (just behind the disc, which spans 5.4 to 7.4) after 3.2 s.
    for _ in 0..384 {
        open.run(1);
        walled.run(1);
        for (value, solid) in walled.current.iter().zip(mask.cells()) {
            if *solid >= 1.0 {
                assert!(value.abs() < 1e-6);
            }
        }
    }
    let behind_open = open.at(84, 64).abs();
    let behind_walled = walled.at(84, 64).abs();
    assert!(behind_walled < 0.5 * behind_open, "behind the disc: {behind_walled} against {behind_open}");
    let in_front = |g: &Grid| (30..52).map(|i| g.at(i, 64).abs()).fold(0.0f32, f32::max);
    assert!(in_front(&walled) > 0.1 && in_front(&walled) > 3.0 * in_front(&open), "reflected: {} against {}", in_front(&walled), in_front(&open));
}

#[test]
fn shallows_damp_waves() {
    let o = options(64, 64, 0.1, 2.0);
    let mask = WaveMask::from_fn(64, 64, |u, _| ((u - 0.75) * 8.0).clamp(0.0, 0.98));
    let (mut open, mut shore) = (Grid::new(&o), Grid::new(&o));
    shore.mask = Some(mask.cells().to_vec());
    open.drop(Vec2::new(2.0, 3.2), 0.25, 0.1);
    shore.drop(Vec2::new(2.0, 3.2), 0.25, 0.1);
    open.run(1200);
    shore.run(1200);
    assert!(shore.energy() < 0.5 * open.energy(), "{} against {}", shore.energy(), open.energy());
}

// ---- the clock --------------------------------------------------------------------------------

#[test]
fn the_fixed_rate_counts_steps_by_clock_time() {
    assert_eq!(step_count(0.0, 120.0, 8).0, 0);
    assert_eq!(step_count(1.0 / 60.0, 120.0, 8).0, 2);
    assert_eq!(step_count(2.0 / 60.0, 120.0, 8).0, 4);
    let (n, rest) = step_count(2.0, 120.0, 8);
    assert_eq!((n, rest < 1.0 / 120.0), (8, true));
    let (n, rest) = step_count(0.0125, 120.0, 8);
    assert_eq!(n, 1);
    assert!((rest - (0.0125 - 1.0 / 120.0)).abs() < 1e-12);
    // Through the core: delta 0 plans nothing; slow motion steps every other frame with a fraction.
    let mut core = WaveCore::new(&WaveFieldOptions::default()).unwrap();
    assert!(core.plan(0.0).is_none());
    assert_eq!(core.last_step_count(), 0);
    let first = core.plan(1.0 / 240.0).unwrap();
    let second = core.plan(1.0 / 240.0).unwrap();
    assert_eq!((first.steps, second.steps), (0, 1));
    assert!((first.fraction - 0.5).abs() < 1e-4 && second.fraction < 1e-4);
    assert_eq!(core.plan(5.0).unwrap().steps, 8);
    assert!((core.simulated_time() - 9.0 / 120.0).abs() < 1e-9);
    assert_eq!(core.steps_taken(), 9);
}

#[test]
fn every_frame_rate_gives_the_same_field_over_the_same_clock_time() {
    let o = WaveFieldOptions { damping: 0.6, ..options(48, 48, 0.1, 1.0) };
    let run = |rate: f64| {
        let mut field = CpuWaveField::new(&o).unwrap();
        field.drop(WaveDrop { position: Vec2::new(2.4, 2.4), radius: 0.3, depth: 0.1 });
        for _ in 0..(rate as u32) {
            field.update(1.0 / rate);
        }
        (field.steps_taken(), field.heights().to_vec())
    };
    let (s60, h60) = run(60.0);
    for rate in [30.0, 75.0, 144.0] {
        let (steps, heights) = run(rate);
        assert_eq!(steps, s60, "{rate} Hz");
        assert_eq!(heights, h60, "{rate} Hz");
    }
    assert_eq!(s60, 120);
}

#[test]
fn the_picture_is_interpolated_between_the_last_two_steps() {
    let o = options(16, 16, 0.1, 1.0);
    let c = coefficients(&o);
    let current: Vec<f32> = (0..256).map(|i| i as f32).collect();
    let previous: Vec<f32> = (0..256).map(|i| 2.0 * i as f32).collect();
    let mut out = vec![[0.0; 4]; 256];
    publish_grid(&current, &previous, None, 16, 16, &c, 0.25, &mut out);
    for (i, texel) in out.iter().enumerate() {
        assert!((texel[0] - 1.75 * i as f32).abs() < 1e-4);
    }
    // Slow motion (a quarter of the speed at 60 Hz): the picture still changes every frame.
    let mut field = CpuWaveField::new(&o).unwrap();
    field.drop(WaveDrop { position: Vec2::new(0.8, 0.8), radius: 0.2, depth: 0.1 });
    field.update(1.0 / 60.0);
    let mut last = field.surface()[8 * 16 + 9][0];
    for _ in 0..8 {
        field.update(0.25 / 60.0);
        let now = field.surface()[8 * 16 + 9][0];
        assert!(now != last);
        last = now;
    }
}

// ---- disturbances -----------------------------------------------------------------------------

#[test]
fn a_drop_presses_its_volume() {
    let o = options(64, 64, 0.05, 1.0);
    let mut g = Grid::new(&o);
    g.drop(Vec2::new(1.6, 1.6), 0.3, 0.02);
    let volume: f32 = g.current.iter().sum::<f32>() * 0.05 * 0.05;
    let expected = -0.02 * std::f32::consts::PI * 0.09;
    assert!((volume / expected - 1.0).abs() < 0.01, "{volume} against {expected}");
    assert_eq!(g.current, g.previous);
}

#[test]
fn a_wake_presses_its_path_whatever_the_frame_rate() {
    // A long path presses 1 along its middle; consecutive stretches add up to the whole.
    let (a, b, c) = (Vec2::new(0.0, 0.0), Vec2::new(0.4, 0.0), Vec2::new(1.0, 0.0));
    assert!((path_profile(Vec2::new(0.5, 0.0), a, c, 0.05) - 1.0).abs() < 1e-5);
    for p in [Vec2::new(0.3, 0.02), Vec2::new(0.4, 0.0), Vec2::new(0.9, -0.05)] {
        let whole = path_profile(p, a, c, 0.15);
        let parts = path_profile(p, a, b, 0.15) + path_profile(p, b, c, 0.15);
        assert!((whole - parts).abs() < 1e-5, "{whole} against {parts}");
    }
    assert_eq!(path_profile(Vec2::new(0.1, 0.0), a, a, 0.15), 0.0);
    // Through the core: a wake that does not move presses nothing; one that moves one unit
    // presses `strength` along its middle, in one update or in ten.
    let o = options(64, 32, 0.05, 1.0);
    let mut core = WaveCore::new(&o).unwrap();
    core.set_wake(0, Some(WaveWake { position: Vec2::new(1.0, 0.8), radius: 0.1, ..WaveWake::default() })).unwrap();
    assert!(core.plan(1.0 / 60.0).unwrap().is_calm());
    let press = |pieces: u32| {
        let mut core = WaveCore::new(&o).unwrap();
        let mut g = Grid::new(&o);
        for k in 0..=pieces {
            let x = 1.0 + k as f32 / pieces as f32;
            core.set_wake(0, Some(WaveWake { position: Vec2::new(x, 0.8), radius: 0.1, strength: 0.05, ..WaveWake::default() })).unwrap();
            let plan = core.plan(1.0 / 60.0).unwrap();
            disturb_grid(&mut g.current, &mut g.previous, None, 64, 32, &g.c, g.size, &plan.drops, &plan.wakes);
        }
        g.current
    };
    let (once, tenfold) = (press(1), press(10));
    let middle = (15 * 64 + 29) as usize; // x = 1.475, y = 0.775
    assert!((once[middle] + 0.05 * (-(0.025f32 / 0.1).powi(2)).exp()).abs() < 1e-3, "{}", once[middle]);
    for (x, y) in once.iter().zip(&tenfold) {
        assert!((x - y).abs() < 1e-5);
    }
}

#[test]
fn a_bobbing_wake_rocks_the_water_under_it() {
    let o = options(32, 32, 0.1, 1.0);
    let mut core = WaveCore::new(&o).unwrap();
    core.set_wake(1, Some(WaveWake { position: Vec2::splat(1.6), bob: 0.02, bob_rate: 1.0, ..WaveWake::default() })).unwrap();
    let plan = core.plan(0.25).unwrap();
    // 8 steps of 1/120 s: the bob moved from sin(0) to sin(2 pi / 15) - it rose, so the water under it rises.
    let expected = -(0.02 * (std::f32::consts::TAU * 8.0 / 120.0).sin());
    assert_eq!(plan.wakes.len(), 1);
    assert!((plan.wakes[0][1].z - expected).abs() < 1e-6);
    assert!(core.set_wake(8, None).is_err());
}

#[test]
fn drops_wait_beyond_the_per_update_limit_and_the_queue_is_capped() {
    let mut core = WaveCore::new(&WaveFieldOptions::default()).unwrap();
    for i in 0..100 {
        core.drop(WaveDrop::at(Vec2::splat(i as f32 * 0.01)));
    }
    assert_eq!(core.plan(1.0 / 60.0).unwrap().drops.len(), 32);
    assert_eq!(core.plan(1.0 / 60.0).unwrap().drops.len(), 32);
    assert_eq!(core.plan(1.0 / 60.0).unwrap().drops.len(), 0);
    // A non-stepping update keeps them waiting.
    core.drop(WaveDrop::default());
    assert!(core.plan(1.0 / 1000.0).unwrap().drops.is_empty());
    assert_eq!(core.plan(1.0 / 60.0).unwrap().drops.len(), 1);
}

// ---- the picture and the uniform block --------------------------------------------------------

#[test]
fn the_picture_has_the_slopes_of_a_plane_and_the_laplacian_of_a_bowl() {
    let o = options(16, 16, 0.25, 1.0);
    let c = coefficients(&o);
    let centre = |i: u32| (i as f32 + 0.5) * 0.25;
    let plane: Vec<f32> = (0..256u32).map(|k| 0.3 * centre(k % 16) - 0.7 * centre(k / 16)).collect();
    let bowl: Vec<f32> = (0..256u32).map(|k| centre(k % 16).powi(2) + centre(k / 16).powi(2)).collect();
    let mut out = vec![[0.0; 4]; 256];
    publish_grid(&plane, &plane, None, 16, 16, &c, 0.0, &mut out);
    let t = out[5 * 16 + 7];
    assert!((t[1] - 0.3).abs() < 1e-4 && (t[2] + 0.7).abs() < 1e-4 && t[3].abs() < 1e-3, "{t:?}");
    publish_grid(&bowl, &bowl, None, 16, 16, &c, 1.0, &mut out);
    assert!((out[8 * 16 + 4][3] - 4.0).abs() < 1e-3, "{:?}", out[8 * 16 + 4]);
    // A solid cell shows 0.
    let mut mask = vec![0.0; 256];
    mask[3] = 1.0;
    publish_grid(&bowl, &bowl, Some(&mask), 16, 16, &c, 1.0, &mut out);
    assert_eq!(out[3], [0.0; 4]);
}

#[test]
fn the_uniform_block_is_laid_out_as_the_shader_reads_it() {
    assert_eq!(std::mem::size_of::<WaveParams>(), PARAMS_SIZE as usize);
    assert_eq!(std::mem::offset_of!(WaveParams, counts), 16);
    assert_eq!(std::mem::offset_of!(WaveParams, weights), 32);
    assert_eq!(std::mem::offset_of!(WaveParams, cells), 48);
    assert_eq!(std::mem::offset_of!(WaveParams, extra), 64);
    assert_eq!(std::mem::offset_of!(WaveParams, drops), 80);
    assert_eq!(std::mem::offset_of!(WaveParams, wakes), 592);
    let o = WaveFieldOptions { edge: WaveEdge::Absorb { width: 500 }, ..WaveFieldOptions::default() };
    let mut core = WaveCore::new(&o).unwrap();
    core.drop(WaveDrop { position: Vec2::new(1.0, 2.0), radius: 0.1, depth: 0.03 });
    core.set_mask(Some(&WaveMask::new(256, 256))).unwrap();
    assert!(core.set_mask(Some(&WaveMask::new(128, 256))).unwrap_err().message().contains("128 x 256"));
    let plan = core.plan(1.0 / 60.0).unwrap();
    let p = core.params(&plan);
    assert_eq!((p.grid.x, p.grid.y, p.grid.z, p.grid.w), (256, 256, 1, 1));
    assert_eq!((p.counts.x, p.counts.y, p.counts.z), (1, 0, 128));
    assert_eq!(p.drops[0], Vec4::new(1.0, 2.0, 0.1, 0.03));
    assert_eq!((p.extra.y, p.extra.z), (10.0, 10.0));
}

#[test]
fn halves_read_back_from_the_gpu_are_decoded() {
    assert_eq!(wave_math::half_to_f32(0x3C00), 1.0);
    assert_eq!(wave_math::half_to_f32(0xC000), -2.0);
    assert_eq!(wave_math::half_to_f32(0x0001), 2f32.powi(-24));
    let field = CpuWaveField::new(&options(8, 8, 0.1, 1.0)).unwrap();
    assert_eq!(field.height_difference(&vec![0u8; 8 * 64]).unwrap(), 0.0);
    assert!(field.height_difference(&[0u8; 8]).is_err());
}

// ---- rain, masks, stirring --------------------------------------------------------------------

#[test]
fn rain_is_the_same_for_a_seed() {
    let rain = WaveRain { rate: 20.0, ..WaveRain::default() };
    let size = Vec2::new(10.0, 10.0);
    let run = |rain: &WaveRain| {
        let mut state = RainState::default();
        (0..600).flat_map(|_| state.take(rain, 1.0 / 60.0, size)).collect::<Vec<WaveDrop>>()
    };
    let (a, b) = (run(&rain), run(&rain));
    assert_eq!(a, b);
    assert!((a.len() as i32 - 200).abs() <= 1, "{}", a.len());
    assert_ne!(a, run(&WaveRain { seed: 2, ..rain }));
    for d in &a {
        assert!(d.position.x >= 0.0 && d.position.x <= 10.0 && d.position.y >= 0.0 && d.position.y <= 10.0);
        assert!((0.04..=0.08).contains(&d.radius) && (0.01..=0.03).contains(&d.depth));
    }
    let area = WaveRain { area: Some((Vec2::new(2.0, 3.0), Vec2::new(4.0, 5.0))), ..rain };
    let d = rain_drop(&area, 17, size);
    assert!((2.0..=4.0).contains(&d.position.x) && (3.0..=5.0).contains(&d.position.y));
    // A long frame does not save up a downpour.
    assert!(RainState::default().take(&rain, 60.0, size).len() <= 20);
}

#[test]
fn rain_on_the_core_starts_over_after_a_reset() {
    let mut core = WaveCore::new(&WaveFieldOptions::default()).unwrap();
    core.set_rain(Some(WaveRain { rate: 60.0, ..WaveRain::default() }));
    let first: Vec<Vec4> = (0..10).flat_map(|_| core.plan(1.0 / 60.0).unwrap().drops).collect();
    core.reset();
    let again: Vec<Vec4> = (0..10).flat_map(|_| core.plan(1.0 / 60.0).unwrap().drops).collect();
    assert_eq!(first, again);
    assert_eq!(first.len(), 10);
}

#[test]
fn a_mask_is_made_from_a_picture_channel() {
    // 2 x 1 pixels: alpha 0 then 255; resampled to 4 x 2 cells the alpha ramps across.
    let picture = DecodedImage { width: 2, height: 1, rgba: vec![0, 0, 0, 0, 255, 0, 0, 255] };
    let mask = WaveMask::from_picture(&picture, 3, 4, 2).unwrap();
    let row: Vec<f32> = (0..4).map(|i| mask.get(i, 1)).collect();
    assert_eq!(row, vec![0.0, 0.25, 0.75, 1.0]);
    let red = WaveMask::from_picture(&picture, 0, 4, 2).unwrap();
    assert_eq!(red.get(3, 0), 1.0);
    assert!(WaveMask::from_picture(&picture, 4, 4, 2).is_err());
    assert!(WaveMask::from_picture(&DecodedImage { width: 2, height: 2, rgba: vec![0; 8] }, 0, 4, 2).is_err());
    let mut disc = WaveMask::new(10, 10);
    disc.add_disc(Vec2::splat(0.5), 0.2, 1.0);
    assert_eq!((disc.get(5, 5), disc.get(0, 0)), (1.0, 0.0));
    disc.set(0, 0, 7.0);
    assert_eq!(disc.get(0, 0), 1.0);
}

fn down(x: f64, button: PointerButton, id: i32) -> CanvasInput {
    let buttons = if button == PointerButton::Primary { PointerButtons::PRIMARY } else { PointerButtons::SECONDARY };
    CanvasInput::PointerDown { x, y: 0.0, pointer_id: id, button, buttons, kind: PointerKind::Mouse, modifiers: ModifierKeys::default(), tap_count: 1 }
}

fn moved(x: f64, id: i32) -> CanvasInput {
    CanvasInput::PointerMove { x, y: 0.0, dx: 1.0, dy: 0.0, pressed: true, pointer_id: id, buttons: PointerButtons::PRIMARY, kind: PointerKind::Mouse, modifiers: ModifierKeys::default() }
}

#[test]
fn the_pointer_stirs_with_a_drop_and_a_wake() {
    let mut core = WaveCore::new(&WaveFieldOptions::default()).unwrap();
    let stir = WaveStir::default();
    // 100 CSS pixels per field unit; nothing beyond x = 1000.
    let to_field = |x: f64, _y: f64| if x <= 1000.0 { Some(Vec2::new(x as f32 / 100.0, 5.0)) } else { None };
    core.stir(&down(100.0, PointerButton::Secondary, 1), &stir, &to_field);
    assert!(core.plan(1.0 / 60.0).unwrap().is_calm());
    core.stir(&down(100.0, PointerButton::Primary, 2), &stir, &to_field);
    core.stir(&down(300.0, PointerButton::Primary, 3), &stir, &to_field); // a second pointer: ignored
    core.stir(&moved(200.0, 2), &stir, &to_field);
    let plan = core.plan(1.0 / 60.0).unwrap();
    assert_eq!(plan.drops, vec![Vec4::new(1.0, 5.0, 0.1, 0.04)]);
    assert_eq!(plan.wakes.len(), 1);
    assert_eq!(plan.wakes[0][0], Vec4::new(1.0, 5.0, 2.0, 5.0));
    assert_eq!(plan.wakes[0][1].y, 0.08);
    // Off the water, the wake stops; back on, it starts where the pointer is.
    core.stir(&moved(2000.0, 2), &stir, &to_field);
    core.stir(&moved(500.0, 2), &stir, &to_field);
    assert!(core.plan(1.0 / 60.0).unwrap().is_calm());
    let up = CanvasInput::PointerUp { x: 500.0, y: 0.0, pointer_id: 2, button: PointerButton::Primary, buttons: PointerButtons::NONE, kind: PointerKind::Mouse, modifiers: ModifierKeys::default() };
    core.stir(&up, &stir, &to_field);
    core.stir(&moved(800.0, 2), &stir, &to_field);
    assert!(core.plan(1.0 / 60.0).unwrap().is_calm());
}

// ---- open-water waves -------------------------------------------------------------------------

fn sea() -> OpenWaterWaves {
    OpenWaterWaves::new(vec![
        GerstnerWave { direction: 0.3, wavelength: 8.0, amplitude: 0.2, steepness: 0.6, phase: 0.4, speed: None },
        GerstnerWave { direction: 1.9, wavelength: 3.0, amplitude: 0.05, steepness: 0.8, phase: 2.0, speed: None },
        GerstnerWave { direction: -0.8, wavelength: 13.0, amplitude: 0.3, steepness: 0.4, phase: 5.0, speed: Some(3.0) },
    ])
    .unwrap()
}

#[test]
fn zero_amplitude_is_flat_water() {
    let flat = OpenWaterWaves::new(vec![GerstnerWave { amplitude: 0.0, ..GerstnerWave::default() }]).unwrap();
    assert_eq!(flat.displacement_at(1.0, 2.0, 3.0), Vec3::ZERO);
    assert_eq!(flat.normal_at(1.0, 2.0, 3.0), Vec3::Y);
    assert_eq!(flat.height_at(1.0, 2.0, 3.0), 0.0);
    assert_eq!(OpenWaterWaves::from_wind(0.0, 0.0, 8, 1).waves().len(), 0);
}

#[test]
fn a_crest_travels_at_the_deep_water_speed() {
    let wave = OpenWaterWaves::new(vec![GerstnerWave { steepness: 0.0, ..GerstnerWave::default() }]).unwrap();
    let speed = (9.81f32 * 8.0 / std::f32::consts::TAU).sqrt();
    // The crest starts at x = 0 (phase 0) and is at x = speed * t a little later.
    for t in [0.1f64, 0.5, 1.0] {
        let x = speed * t as f32;
        assert!((wave.displacement_at(x, 0.0, t).y - 0.15).abs() < 1e-4, "at {t}: {}", wave.displacement_at(x, 0.0, t).y);
    }
    let slow = wave.clone().with_gravity(4.0);
    assert!((slow.displacement_at((4.0f32 * 8.0 / std::f32::consts::TAU).sqrt(), 0.0, 1.0).y - 0.15).abs() < 1e-4);
}

#[test]
fn a_still_water_point_moves_on_an_ellipse() {
    let wave = OpenWaterWaves::new(vec![GerstnerWave { amplitude: 0.3, steepness: 0.5, ..GerstnerWave::default() }]).unwrap();
    for i in 0..20 {
        let d = wave.displacement_at(1.3, 0.7, i as f64 * 0.17);
        assert!(d.z.abs() < 1e-6);
        let e = (d.x / 0.15).powi(2) + (d.y / 0.3).powi(2);
        assert!((e - 1.0).abs() < 1e-4, "{e}");
    }
}

#[test]
fn the_analytic_normal_equals_the_numeric_tangents() {
    let sea = sea();
    let resolved: Vec<_> = sea.waves().iter().map(|w| gerstner_math::resolve(w, 9.81)).collect();
    let phases = gerstner_math::phases(&resolved, 7.25);
    let moved = |p: Vec2| Vec3::new(p.x, 0.0, p.y) + gerstner_math::displacement(&resolved, &phases, p);
    for (x, z) in [(0.0f32, 0.0f32), (1.7, -2.3), (5.5, 4.1), (-3.0, 8.0)] {
        let p = Vec2::new(x, z);
        let e = 1e-2;
        let tx = (moved(p + Vec2::X * e) - moved(p - Vec2::X * e)) / (2.0 * e);
        let tz = (moved(p + Vec2::Y * e) - moved(p - Vec2::Y * e)) / (2.0 * e);
        let numeric = tz.cross(tx).normalize();
        let analytic = gerstner_math::normal(&resolved, &phases, p);
        assert!((numeric - analytic).length() < 1e-4, "{numeric} against {analytic}");
    }
}

#[test]
fn wind_seas_never_loop_and_stay_within_sixty_degrees() {
    for seed in 1..12 {
        for wind in [1.5f32, 5.0, 12.0] {
            let sea = OpenWaterWaves::from_wind(0.5, wind, 16, seed);
            assert_eq!(sea, OpenWaterWaves::from_wind(0.5, wind, 16, seed));
            assert_eq!(sea.waves().len(), 16);
            for w in sea.waves() {
                assert!((w.direction - 0.5).abs() <= 60f32.to_radians() + 1e-5);
                assert!(w.amplitude > 0.0 && w.wavelength > 0.0);
            }
            assert!(OpenWaterWaves::new(sea.waves().to_vec()).is_ok());
            let resolved: Vec<_> = sea.waves().iter().map(|w| gerstner_math::resolve(w, 9.81)).collect();
            let span = 0.83 * wind * wind;
            for t in [0.0f64, 3.3] {
                let phases = gerstner_math::phases(&resolved, t);
                for i in 0..40 {
                    for j in 0..40 {
                        let p = Vec2::new(i as f32, j as f32) * span / 20.0;
                        assert!(gerstner_math::jacobian(&resolved, &phases, p) > 0.0);
                    }
                }
            }
        }
    }
    assert_ne!(OpenWaterWaves::from_wind(0.5, 5.0, 8, 1), OpenWaterWaves::from_wind(0.5, 5.0, 8, 2));
    // The peak wavelength is about 0.83 U^2.
    let sea = OpenWaterWaves::from_wind(0.0, 10.0, 16, 3);
    let longest = sea.waves().iter().map(|w| w.wavelength).fold(0.0, f32::max);
    assert!(longest > 83.0 && longest < 140.0, "{longest}");
}

#[test]
fn looping_seas_and_bad_waves_are_refused() {
    let steep = vec![GerstnerWave { wavelength: 1.0, amplitude: 0.3, steepness: 1.0, ..GerstnerWave::default() }];
    let message = OpenWaterWaves::new(steep).unwrap_err().message().to_string();
    assert!(message.contains("1.885") && message.contains("0.531"), "{message}");
    assert!(OpenWaterWaves::new(vec![GerstnerWave::default(); 17]).is_err());
    assert!(OpenWaterWaves::new(vec![GerstnerWave { wavelength: 0.0, ..GerstnerWave::default() }]).is_err());
    assert!(OpenWaterWaves::new(vec![GerstnerWave { steepness: 1.5, ..GerstnerWave::default() }]).is_err());
    assert!(OpenWaterWaves::new(vec![GerstnerWave { speed: Some(0.0), ..GerstnerWave::default() }]).is_err());
}

#[test]
fn the_height_above_a_point_matches_a_brute_force_search() {
    let sea = sea();
    let resolved: Vec<_> = sea.waves().iter().map(|w| gerstner_math::resolve(w, 9.81)).collect();
    for (x, z, t) in [(0.0f32, 0.0f32, 0.0f64), (3.3, -1.2, 2.5), (-7.0, 4.4, 11.0), (12.0, 9.0, 100.0)] {
        let phases = gerstner_math::phases(&resolved, t);
        let target = Vec2::new(x, z);
        // Coarse to fine over still-water points around the target.
        let (mut best, mut reach) = (target, 1.0f32);
        for _ in 0..12 {
            let mut found = (f32::MAX, best);
            for i in -10..=10 {
                for j in -10..=10 {
                    let p = best + Vec2::new(i as f32, j as f32) * reach / 10.0;
                    let d = gerstner_math::displacement(&resolved, &phases, p);
                    let error = (p + Vec2::new(d.x, d.z) - target).length();
                    if error < found.0 {
                        found = (error, p);
                    }
                }
            }
            best = found.1;
            reach /= 4.0;
        }
        let brute = gerstner_math::displacement(&resolved, &phases, best).y;
        let height = sea.height_at(x, z, t);
        assert!((height - brute).abs() < 1e-3 * 0.3, "{height} against {brute}");
        let n = sea.normal_at(x, z, t);
        assert!((n.length() - 1.0).abs() < 1e-5 && n.y > 0.5);
    }
    assert!((sea.max_height() - 0.55).abs() < 1e-6);
}

#[test]
fn the_phase_is_computed_in_f64() {
    let sea = sea();
    let t = 1.0e5;
    let records = sea.records(t);
    assert_eq!(records.len(), 3);
    for (record, wave) in records.iter().zip(sea.waves()) {
        let k = std::f64::consts::TAU / wave.wavelength as f64;
        let speed = wave.speed.map_or((9.81f32 as f64 / k).sqrt(), |s| s as f64);
        let expected = (speed * k * t + wave.phase as f64).rem_euclid(std::f64::consts::TAU);
        assert!((record.motion.y as f64 - expected).abs() < 1e-5, "{} against {expected}", record.motion.y);
        assert!((record.direction.z as f64 - k).abs() < 1e-6);
        assert_eq!(record.motion.x, wave.steepness * wave.amplitude);
    }
}

#[test]
fn short_waves_fade_by_distance_and_stay_off_coarse_grids() {
    assert_eq!(OpenWaterWaves::normal_fade(1.0, 0.5), 0.0);
    assert_eq!(OpenWaterWaves::normal_fade(2.0, 0.5), 1.0);
    let mid = OpenWaterWaves::normal_fade(1.5, 0.5);
    assert!(mid > 0.0 && mid < 1.0);
    assert!(OpenWaterWaves::normal_fade(1.2, 0.5) < OpenWaterWaves::normal_fade(1.4, 0.5));
    assert!(OpenWaterWaves::displaces_grid(4.0, 1.0) && !OpenWaterWaves::displaces_grid(3.9, 1.0));
}

