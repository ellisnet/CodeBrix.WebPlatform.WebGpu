//! `IArrivalOrder`: what decides which target of a formation fills first (C# `IArrivalOrder`).

use crate::formations::FormationArea;

/// Ranks a formation's targets (C# `IArrivalOrder`): prepared once with the area and the seed,
/// then asked for each target's key - smaller arrives first; an infinite or NaN key drops the
/// target. Implement it for an order of your own, or use [`CustomArrivalOrder`](crate::formations::CustomArrivalOrder).
pub trait IArrivalOrder {
    /// Prepares the order for an area (C# `Prepare(area, seed)`).
    fn prepare(&mut self, area: &FormationArea, seed: i64);

    /// The key of a target at (x, y) with its clearance from the edge (C# `GetKey`).
    fn get_key(&mut self, x: f32, y: f32, clearance: f32) -> f32;
}
