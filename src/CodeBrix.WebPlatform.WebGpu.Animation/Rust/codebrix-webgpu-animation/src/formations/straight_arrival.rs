//! `StraightArrival`: a member travels in a straight line, easing in (C# `StraightArrival`).

use crate::animation::EasingKind;
use crate::formations::arrival_math::{check_times, eased, lerp};
use crate::formations::{FormationPose, IArrivalPath};

/// From a start pose to the target in a straight line, every part of the pose eased alike (C#
/// `StraightArrival(start, target, startTime, arrivalTime, easing = CubicOut)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StraightArrival {
    start: FormationPose,
    target: FormationPose,
    start_time: f64,
    arrival_time: f64,
    easing: EasingKind,
}

impl StraightArrival {
    /// The path; `easing` None is CubicOut. The error says which value is wrong.
    pub fn new(start: FormationPose, target: FormationPose, start_time: f64, arrival_time: f64, easing: Option<EasingKind>) -> Result<StraightArrival, String> {
        start.check("start")?;
        target.check("target")?;
        check_times(start_time, arrival_time)?;
        Ok(StraightArrival { start, target, start_time, arrival_time, easing: easing.unwrap_or(EasingKind::CubicOut) })
    }

    /// The start pose (C# `Start`).
    pub fn start(&self) -> FormationPose {
        self.start
    }

    /// The easing (C# `Easing`).
    pub fn easing(&self) -> EasingKind {
        self.easing
    }
}

impl IArrivalPath for StraightArrival {
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
        let e = eased(seconds, self.start_time, self.arrival_time, self.easing);
        let (s, t) = (self.start, self.target);
        FormationPose::new(lerp(s.x, t.x, e), lerp(s.y, t.y, e), lerp(s.rotation, t.rotation, e), lerp(s.scale_x, t.scale_x, e), lerp(s.scale_y, t.scale_y, e))
    }
}
