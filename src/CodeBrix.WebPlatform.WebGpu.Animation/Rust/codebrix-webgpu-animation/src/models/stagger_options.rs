//! `StaggerOptions`: the settings of a staggered entrance (C# `StaggerOptions`).

use crate::animation::EasingKind;
use crate::geometry::PointF;
use crate::models::StaggerOrder;

/// The settings of a staggered entrance (C# `StaggerOptions`, the same members and defaults):
/// each element (in [`order`](StaggerOptions::order)) starts `interval` seconds after the one
/// before and takes `duration` seconds to go from the "from" state (opacity, offset, scale,
/// rotation) to `to_opacity` with no offset, scale 1 and rotation 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StaggerOptions {
    /// When the first element starts, seconds (default 0).
    pub start_time: f64,
    /// Seconds between one element's start and the next's (default 0.05).
    pub interval: f64,
    /// Seconds each element takes (default 0.4).
    pub duration: f64,
    /// The easing of every element's change (default CubicOut).
    pub easing: EasingKind,
    /// The order the elements start in (default Forward).
    pub order: StaggerOrder,
    /// The opacity each element starts at, 0..1 (default 0).
    pub from_opacity: f32,
    /// The opacity each element ends at, 0..1 (default 1).
    pub to_opacity: f32,
    /// The offset each element starts at and moves back from (default none).
    pub from_offset: PointF,
    /// The scale each element starts at (default 1: no scaling).
    pub from_scale: f32,
    /// The rotation each element starts at, degrees (default 0).
    pub from_rotation: f32,
    /// True (the default) to move each element's transform anchor to the centre of its bounds
    /// first, so it scales and turns about its own centre.
    pub anchor_at_center: bool,
}

impl Default for StaggerOptions {
    fn default() -> StaggerOptions {
        StaggerOptions {
            start_time: 0.0,
            interval: 0.05,
            duration: 0.4,
            easing: EasingKind::CubicOut,
            order: StaggerOrder::Forward,
            from_opacity: 0.0,
            to_opacity: 1.0,
            from_offset: PointF::new(0.0, 0.0),
            from_scale: 1.0,
            from_rotation: 0.0,
            anchor_at_center: true,
        }
    }
}

impl StaggerOptions {
    /// Rejects values no stagger can honour, naming the member at fault (C# `Validate`).
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !self.start_time.is_finite() {
            return Err("A stagger's start_time must be a finite time in seconds.".to_string());
        }
        if !self.interval.is_finite() || self.interval < 0.0 {
            return Err(format!("A stagger's interval must be 0 or a positive, finite number of seconds (it was {}).", self.interval));
        }
        if !self.duration.is_finite() || self.duration < 0.0 {
            return Err(format!("A stagger's duration must be 0 or a positive, finite number of seconds (it was {}).", self.duration));
        }
        for (name, value) in [("from_opacity", self.from_opacity), ("to_opacity", self.to_opacity)] {
            if value.is_nan() || !(0.0..=1.0).contains(&value) {
                return Err(format!("A stagger's {name} must be between 0 and 1 (it was {value})."));
            }
        }
        if !self.from_offset.x.is_finite() || !self.from_offset.y.is_finite() {
            return Err("A stagger's from_offset must be finite numbers (calibrated units).".to_string());
        }
        if !self.from_scale.is_finite() {
            return Err("A stagger's from_scale must be a finite number (1 for no scaling).".to_string());
        }
        if !self.from_rotation.is_finite() {
            return Err("A stagger's from_rotation must be a finite number of degrees.".to_string());
        }
        Ok(())
    }
}
