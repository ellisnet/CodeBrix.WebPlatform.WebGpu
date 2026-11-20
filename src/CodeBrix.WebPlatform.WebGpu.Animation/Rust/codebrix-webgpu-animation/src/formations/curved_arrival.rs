//! `CurvedArrival`: a member travels on a curve bending to one side (C# `CurvedArrival`).

use crate::animation::EasingKind;
use crate::formations::arrival_math::{check_times, eased, lerp};
use crate::formations::{FormationPose, IArrivalPath};

/// The optional arguments of [`CurvedArrival::new`] (C#'s defaults).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurvedArrivalOptions {
    /// How far the curve bends, a fraction of the distance, to the RIGHT of the direction of
    /// travel on screen (negative: left) (0.35).
    pub bend: f32,
    /// The easing (CubicOut).
    pub easing: EasingKind,
}

impl Default for CurvedArrivalOptions {
    fn default() -> CurvedArrivalOptions {
        CurvedArrivalOptions { bend: 0.35, easing: EasingKind::CubicOut }
    }
}

/// From a start pose to the target along a quadratic curve whose control point lies `bend` x the
/// distance to the right of the midpoint (C# `CurvedArrival(start, target, startTime,
/// arrivalTime, bend = 0.35, easing = CubicOut)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurvedArrival {
    start: FormationPose,
    target: FormationPose,
    start_time: f64,
    arrival_time: f64,
    options: CurvedArrivalOptions,
    control_x: f64,
    control_y: f64,
}

impl CurvedArrival {
    /// The path. The error says which value is wrong.
    pub fn new(start: FormationPose, target: FormationPose, start_time: f64, arrival_time: f64, options: CurvedArrivalOptions) -> Result<CurvedArrival, String> {
        start.check("start")?;
        target.check("target")?;
        check_times(start_time, arrival_time)?;
        if !options.bend.is_finite() {
            return Err("The bend must be a finite fraction.".to_string());
        }
        let dx = target.x as f64 - start.x as f64;
        let dy = target.y as f64 - start.y as f64;
        // The right of the direction of travel, on screen (y down), is (-dy, dx).
        let control_x = ((start.x as f64 + target.x as f64) / 2.0) - (dy * options.bend as f64);
        let control_y = ((start.y as f64 + target.y as f64) / 2.0) + (dx * options.bend as f64);
        Ok(CurvedArrival { start, target, start_time, arrival_time, options, control_x, control_y })
    }

    /// The start pose (C# `Start`).
    pub fn start(&self) -> FormationPose {
        self.start
    }

    /// The bend (C# `Bend`).
    pub fn bend(&self) -> f32 {
        self.options.bend
    }

    /// The easing (C# `Easing`).
    pub fn easing(&self) -> EasingKind {
        self.options.easing
    }
}

impl IArrivalPath for CurvedArrival {
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
        let e = eased(seconds, self.start_time, self.arrival_time, self.options.easing);
        let a = (1.0 - e) * (1.0 - e);
        let b = 2.0 * (1.0 - e) * e;
        let c = e * e;
        let (s, t) = (self.start, self.target);
        let x = (a * s.x as f64) + (b * self.control_x) + (c * t.x as f64);
        let y = (a * s.y as f64) + (b * self.control_y) + (c * t.y as f64);
        FormationPose::new(x as f32, y as f32, lerp(s.rotation, t.rotation, e), lerp(s.scale_x, t.scale_x, e), lerp(s.scale_y, t.scale_y, e))
    }
}
