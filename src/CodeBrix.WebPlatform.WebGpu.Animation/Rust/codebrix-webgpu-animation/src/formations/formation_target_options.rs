//! `FormationTargetOptions`: how a formation's targets are laid out (C# `FormationTargetOptions`).

use crate::formations::IArrivalOrder;

/// How [`FormationTargets::fill`](crate::formations::FormationTargets::fill) lays targets out
/// (C# `FormationTargetOptions`, the same defaults): a hexagonal grid of `spacing` (rows
/// `spacing x 0.866` apart, every other row shifted half a step), each point moved by up to
/// `jitter x spacing` either way, points closer than `edge_clearance` to the edge dropped, the
/// rest ranked by `order` (None: a [`RandomArrivalOrder`](crate::formations::RandomArrivalOrder)).
pub struct FormationTargetOptions {
    /// The grid spacing, calibrated units (16).
    pub spacing: f32,
    /// The jitter, a fraction of the spacing (0.33).
    pub jitter: f32,
    /// The least distance from the edge, calibrated units (4).
    pub edge_clearance: f32,
    /// The seed of the jitter and of the order (1).
    pub seed: i64,
    /// The arrival order (None: random).
    pub order: Option<Box<dyn IArrivalOrder>>,
}

impl Default for FormationTargetOptions {
    fn default() -> FormationTargetOptions {
        FormationTargetOptions { spacing: 16.0, jitter: 0.33, edge_clearance: 4.0, seed: 1, order: None }
    }
}

impl FormationTargetOptions {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !self.spacing.is_finite() || self.spacing <= 0.0 {
            return Err("The spacing must be a positive, finite distance.".to_string());
        }
        if !self.jitter.is_finite() || self.jitter < 0.0 {
            return Err("The jitter must be a finite fraction, 0 or more.".to_string());
        }
        if !self.edge_clearance.is_finite() {
            return Err("The edge clearance must be a finite distance.".to_string());
        }
        Ok(())
    }
}
