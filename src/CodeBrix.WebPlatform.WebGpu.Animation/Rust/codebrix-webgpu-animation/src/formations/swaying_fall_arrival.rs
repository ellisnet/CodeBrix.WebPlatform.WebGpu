//! `SwayingFallArrival`: a member falls like a leaf - swaying, rocking, fluttering, landing
//! softly (C# `SwayingFallArrival`).

use std::f64::consts::PI;
use std::rc::Rc;

use crate::formations::{FormationPose, IArrivalPath};
use crate::planes::DrawingRandom;

const SETTLE_TIME: f64 = 1.1;
const SOFT_LANDING: f64 = 0.42;

/// A swaying fall onto the target, arriving at the end of a swing (C# `SwayingFallArrival(random,
/// target, arrivalTime, startY, airOffset = null)`): its character - sink speed, sway, swing
/// period, bob, rock, spin, flip, drift, flutter - is drawn from `random` (in C#'s order, so the
/// same random gives the same fall), and its start time is found so that it starts at `start_y`
/// (searched by halving). `air_offset` is an optional horizontal wind as a function of the time.
#[derive(Clone)]
pub struct SwayingFallArrival {
    air_offset: Option<Rc<dyn Fn(f64) -> f64>>,
    target: FormationPose,
    arrival_time: f64,
    start_time: f64,
    /// Units per second it sinks (C# `SinkSpeed`).
    pub sink_speed: f64,
    /// How far it sways either way (C# `Sway`).
    pub sway: f64,
    /// Seconds per swing (C# `SwingPeriod`).
    pub swing_period: f64,
    /// How much it bobs up at the end of a swing (C# `Bob`).
    pub bob: f64,
    /// Degrees it rocks (C# `Rock`).
    pub rock: f64,
    /// Degrees per second it spins (C# `Spin`; 0 for most).
    pub spin: f64,
    /// How fast it flips over (C# `FlipRate`).
    pub flip_rate: f64,
    /// The phase of its flipping (C# `FlipPhase`).
    pub flip_phase: f64,
    /// How much the wind moves it (C# `Drift`).
    pub drift: f64,
    /// Degrees it flutters as it lands (C# `Flutter`).
    pub flutter: f64,
    /// How fast it flutters (C# `FlutterRate`).
    pub flutter_rate: f64,
    /// The phase of its swing (C# `Phase`).
    pub phase: f64,
}

impl SwayingFallArrival {
    /// The fall. The error says which value is not finite.
    pub fn new(random: &mut DrawingRandom, target: FormationPose, arrival_time: f64, start_y: f32, air_offset: Option<Rc<dyn Fn(f64) -> f64>>) -> Result<SwayingFallArrival, String> {
        target.check("target")?;
        if !arrival_time.is_finite() {
            return Err("The arrival time must be a finite number of seconds.".to_string());
        }
        if !start_y.is_finite() {
            return Err("The start y must be a finite number.".to_string());
        }
        let sink_speed = random.uniform(185.0, 250.0);
        let sway = random.uniform(16.0, 74.0);
        let swing_period = random.uniform(1.5, 2.8);
        let bob = random.uniform(0.3, 0.62);
        let rock = random.uniform(14.0, 38.0) * if random.next_double() < 0.5 { -1.0 } else { 1.0 };
        let spin = if random.next_double() < 0.32 {
            let s = random.uniform(100.0, 250.0);
            s * if random.next_double() < 0.5 { -1.0 } else { 1.0 }
        } else {
            0.0
        };
        let flip_rate = random.uniform(0.7, 2.3);
        let flip_phase = random.uniform(0.0, PI * 2.0);
        let drift = random.uniform(0.6, 1.3);
        let flutter = random.uniform(6.0, 15.0);
        let flutter_rate = random.uniform(26.0, 40.0);
        // Arrive at the end of a swing: the swing's phase there is +-90 degrees.
        let omega = 2.0 * PI / swing_period;
        let phase = ((if random.next_double() < 0.5 { 0.5 } else { -0.5 }) * PI) - (omega * arrival_time);
        let mut fall = SwayingFallArrival {
            air_offset,
            target,
            arrival_time,
            start_time: arrival_time,
            sink_speed,
            sway,
            swing_period,
            bob,
            rock,
            spin,
            flip_rate,
            flip_phase,
            drift,
            flutter,
            flutter_rate,
            phase,
        };
        // How long it falls: from start_y down to the target (the height only grows going back).
        let (mut lo, mut hi) = (0.5f64, 40.0f64);
        for _ in 0..50 {
            let mid = (lo + hi) / 2.0;
            if fall.get_pose_at(arrival_time - mid).y > start_y {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        fall.start_time = arrival_time - hi;
        Ok(fall)
    }
}

fn smooth(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - (2.0 * u))
}

/// C#'s `Math.Sign` (0 for 0).
fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

impl IArrivalPath for SwayingFallArrival {
    fn target(&self) -> FormationPose {
        self.target
    }

    fn start_time(&self) -> f64 {
        self.start_time
    }

    fn arrival_time(&self) -> f64 {
        self.arrival_time
    }

    fn get_pose_at(&self, seconds: f64) -> FormationPose {
        if seconds >= self.arrival_time {
            return self.target;
        }
        let t = self.target;
        let to_go = (self.arrival_time - seconds).max(0.0);
        let ease = smooth(to_go / SETTLE_TIME);
        let omega = 2.0 * PI / self.swing_period;
        let swing = (omega * seconds) + self.phase;
        let swing_at_rest = (omega * self.arrival_time) + self.phase;
        let air = match &self.air_offset {
            Some(f) => f(seconds) - f(self.arrival_time),
            None => 0.0,
        };
        let x = t.x as f64 + (self.sway * (swing.sin() - swing_at_rest.sin())) + (self.drift * air * ease);
        let sink = to_go - (SOFT_LANDING * (1.0 - (-to_go / SOFT_LANDING).exp()));
        let bob = self.sink_speed * self.bob * ((2.0 * swing).sin() - (2.0 * swing_at_rest).sin()) / (2.0 * omega);
        let y = t.y as f64 - (self.sink_speed * sink) + (bob * ease);
        let flutter_window = to_go / 0.55 * (1.0 - (to_go / 0.55)).exp();
        let angle = t.rotation as f64 + (self.rock * swing.cos() * ease) + (self.spin * (seconds - self.arrival_time) * ease) + (self.flutter * (self.flutter_rate * to_go).sin() * flutter_window);
        let flip = ((self.flip_rate * seconds) + self.flip_phase).cos();
        let flip = sign(flip) * flip.abs().max(0.16);
        let scale_x = t.scale_x as f64 + ((flip - t.scale_x as f64) * smooth(to_go / 1.4));
        let scale_y = t.scale_y as f64 + ((0.92 + (0.08 * swing.sin()) - t.scale_y as f64) * ease);
        FormationPose::new(x as f32, y as f32, angle as f32, scale_x as f32, scale_y as f32)
    }
}
