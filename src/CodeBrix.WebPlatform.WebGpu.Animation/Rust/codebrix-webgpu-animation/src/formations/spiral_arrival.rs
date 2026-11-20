//! `SpiralArrival`: a member spirals in onto its target (C# `SpiralArrival`).

use crate::animation::EasingKind;
use crate::formations::arrival_math::{check_times, eased};
use crate::formations::{FormationPose, IArrivalPath};

/// The optional arguments of [`SpiralArrival::new`] (C#'s defaults).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpiralArrivalOptions {
    /// The angle it starts at round the target, degrees (0).
    pub start_angle: f32,
    /// How many turns it makes on the way in (1.5).
    pub turns: f32,
    /// Its scale at the start, as a factor of the target's (1).
    pub start_scale: f32,
    /// True to spin with the path, ending at the target's rotation (false).
    pub spins_with_path: bool,
    /// The easing (CubicOut).
    pub easing: EasingKind,
}

impl Default for SpiralArrivalOptions {
    fn default() -> SpiralArrivalOptions {
        SpiralArrivalOptions { start_angle: 0.0, turns: 1.5, start_scale: 1.0, spins_with_path: false, easing: EasingKind::CubicOut }
    }
}

/// A spiral in onto the target from `start_radius` away (C# `SpiralArrival(target, startTime,
/// arrivalTime, startRadius, startAngle = 0, turns = 1.5, startScale = 1, spinsWithPath = false,
/// easing = CubicOut)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpiralArrival {
    target: FormationPose,
    start_time: f64,
    arrival_time: f64,
    start_radius: f32,
    options: SpiralArrivalOptions,
}

impl SpiralArrival {
    /// The path. The error says which value is wrong.
    pub fn new(target: FormationPose, start_time: f64, arrival_time: f64, start_radius: f32, options: SpiralArrivalOptions) -> Result<SpiralArrival, String> {
        target.check("target")?;
        check_times(start_time, arrival_time)?;
        if !start_radius.is_finite() || start_radius < 0.0 {
            return Err("The start radius must be a finite distance, 0 or more.".to_string());
        }
        if !options.start_angle.is_finite() || !options.turns.is_finite() || !options.start_scale.is_finite() {
            return Err("The start angle, turns and start scale must be finite numbers.".to_string());
        }
        Ok(SpiralArrival { target, start_time, arrival_time, start_radius, options })
    }

    /// The start radius (C# `StartRadius`).
    pub fn start_radius(&self) -> f32 {
        self.start_radius
    }

    /// The optional settings it was made with (C#'s `StartAngle`, `Turns`, `StartScale`,
    /// `SpinsWithPath`, `Easing`).
    pub fn options(&self) -> SpiralArrivalOptions {
        self.options
    }
}

impl IArrivalPath for SpiralArrival {
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
        let o = self.options;
        let t = self.target;
        let e = eased(seconds, self.start_time, self.arrival_time, o.easing);
        let left = 1.0 - e;
        let angle_degrees = o.start_angle as f64 + (o.turns as f64 * 360.0 * e);
        let radians = angle_degrees * (std::f64::consts::PI / 180.0);
        let radius = self.start_radius as f64 * left;
        let x = t.x as f64 + (radius * radians.cos());
        let y = t.y as f64 + (radius * radians.sin());
        let rotation = t.rotation as f64 - if o.spins_with_path { o.turns as f64 * 360.0 * left } else { 0.0 };
        let scale = o.start_scale as f64 + ((1.0 - o.start_scale as f64) * e);
        FormationPose::new(x as f32, y as f32, rotation as f32, (t.scale_x as f64 * scale) as f32, (t.scale_y as f64 * scale) as f32)
    }
}
