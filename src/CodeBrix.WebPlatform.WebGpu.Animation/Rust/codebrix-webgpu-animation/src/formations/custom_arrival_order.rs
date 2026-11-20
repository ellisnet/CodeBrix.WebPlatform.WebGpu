//! `CustomArrivalOrder`: an arrival order from a function of your own (C# `CustomArrivalOrder`).

use crate::formations::{FormationArea, IArrivalOrder};

/// An order whose key is a function of (x, y, clearance) (C# `CustomArrivalOrder(Func<float,
/// float, float, float>)`). Keep it a pure function so the formation is the same every time.
pub struct CustomArrivalOrder {
    key: Box<dyn Fn(f32, f32, f32) -> f32>,
}

impl CustomArrivalOrder {
    /// The order.
    pub fn new(key: impl Fn(f32, f32, f32) -> f32 + 'static) -> CustomArrivalOrder {
        CustomArrivalOrder { key: Box::new(key) }
    }
}

impl IArrivalOrder for CustomArrivalOrder {
    fn prepare(&mut self, _area: &FormationArea, _seed: i64) {}

    fn get_key(&mut self, x: f32, y: f32, clearance: f32) -> f32 {
        (self.key)(x, y, clearance)
    }
}
