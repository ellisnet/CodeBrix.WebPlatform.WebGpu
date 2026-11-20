//! `FormationTarget`: one place a formation member comes to rest (C# `FormationTarget`).

use crate::geometry::PointF;

/// One target point of a formation (C# `FormationTarget`): where it is, its arrival-order key
/// (smaller arrives first) and its clearance (distance to the area's edge).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormationTarget {
    /// Where the member rests, calibrated units.
    pub position: PointF,
    /// The arrival-order key: smaller arrives first.
    pub key: f32,
    /// The distance to the area's edge.
    pub clearance: f32,
}

impl FormationTarget {
    /// A target (C#'s constructor).
    pub fn new(position: PointF, key: f32, clearance: f32) -> FormationTarget {
        FormationTarget { position, key, clearance }
    }
}
