//! `CenterOutArrivalOrder`: targets fill from the area's centre outwards, or inwards (C#
//! `CenterOutArrivalOrder`).

use crate::formations::{FormationArea, IArrivalOrder};
use crate::geometry::PointF;

/// From the centre of the area's bounds outwards (C# `CenterOutArrivalOrder(reverse = false)`):
/// the key is the distance from the centre (negated when reversed: from the rim inwards).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CenterOutArrivalOrder {
    is_reversed: bool,
    center: PointF,
}

impl CenterOutArrivalOrder {
    /// The order; `reverse` fills from the rim inwards.
    pub fn new(reverse: bool) -> CenterOutArrivalOrder {
        CenterOutArrivalOrder { is_reversed: reverse, center: PointF::default() }
    }

    /// True when it fills from the rim inwards (C# `IsReversed`).
    pub fn is_reversed(&self) -> bool {
        self.is_reversed
    }
}

impl IArrivalOrder for CenterOutArrivalOrder {
    fn prepare(&mut self, area: &FormationArea, _seed: i64) {
        let b = area.bounds();
        self.center = PointF::new(b.x + (b.width / 2.0), b.y + (b.height / 2.0));
    }

    fn get_key(&mut self, x: f32, y: f32, _clearance: f32) -> f32 {
        let c = self.center;
        let distance = (((x - c.x) * (x - c.x)) + ((y - c.y) * (y - c.y))).sqrt();
        if self.is_reversed {
            -distance
        } else {
            distance
        }
    }
}
