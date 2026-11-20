//! `IArrivalPath`: how a formation member travels to its target (C# `IArrivalPath`).

use crate::formations::FormationPose;

/// A member's way to its target (C# `IArrivalPath`): its target pose, when it sets off, when it
/// arrives, and its pose at any time - a PURE function of the time (at or after the arrival, the
/// target). Implement it for a way of your own.
pub trait IArrivalPath {
    /// The pose it comes to rest in (C# `Target`).
    fn target(&self) -> FormationPose;
    /// When it sets off, seconds (C# `StartTime`).
    fn start_time(&self) -> f64;
    /// When it arrives, seconds (C# `ArrivalTime`).
    fn arrival_time(&self) -> f64;
    /// Its pose at a time (C# `GetPoseAt`).
    fn get_pose_at(&self, seconds: f64) -> FormationPose;
}
