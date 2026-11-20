//! `RandomArrivalOrder`: targets fill in a seeded random order (C# `RandomArrivalOrder`).

use crate::formations::{FormationArea, IArrivalOrder};
use crate::planes::DrawingRandom;

/// A seeded random order (C# `RandomArrivalOrder`): each key is the next number of a
/// [`DrawingRandom`] seeded with `seed ^ 0x5DEECE66D`.
#[derive(Clone, Debug, Default)]
pub struct RandomArrivalOrder {
    random: Option<DrawingRandom>,
}

impl RandomArrivalOrder {
    /// A random order.
    pub fn new() -> RandomArrivalOrder {
        RandomArrivalOrder::default()
    }
}

impl IArrivalOrder for RandomArrivalOrder {
    fn prepare(&mut self, _area: &FormationArea, seed: i64) {
        self.random = Some(DrawingRandom::new(seed ^ 0x5DEECE66D));
    }

    /// The next random number (NaN - the target dropped - before `prepare`).
    fn get_key(&mut self, _x: f32, _y: f32, _clearance: f32) -> f32 {
        self.random.as_mut().map_or(f32::NAN, |r| r.next_single())
    }
}
