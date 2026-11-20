//! `DirectionalArrivalOrder`: targets fill in a direction, a sweep across the area (C#
//! `DirectionalArrivalOrder`).

use crate::formations::{FormationArea, IArrivalOrder};
use crate::planes::DrawingRandom;

/// A sweep (C# `DirectionalArrivalOrder(angle, scatter = 0)`): the key is the target's distance
/// along the direction `angle` (degrees clockwise on screen from +x: 0 fills left to right, 90 top
/// to bottom), plus a seeded scatter of up to `scatter` either way.
#[derive(Clone, Debug)]
pub struct DirectionalArrivalOrder {
    angle: f32,
    scatter: f32,
    cos: f64,
    sin: f64,
    random: Option<DrawingRandom>,
}

impl DirectionalArrivalOrder {
    /// The order. The error says what to do with a non-finite angle or a negative scatter.
    pub fn new(angle: f32, scatter: f32) -> Result<DirectionalArrivalOrder, String> {
        if !angle.is_finite() {
            return Err("The angle must be a finite number of degrees.".to_string());
        }
        if !scatter.is_finite() || scatter < 0.0 {
            return Err("The scatter must be a finite distance, 0 or more.".to_string());
        }
        let radians = angle as f64 * (std::f64::consts::PI / 180.0);
        Ok(DirectionalArrivalOrder { angle, scatter, cos: radians.cos(), sin: radians.sin(), random: None })
    }

    /// The direction, degrees (C# `Angle`).
    pub fn angle(&self) -> f32 {
        self.angle
    }

    /// The scatter (C# `Scatter`).
    pub fn scatter(&self) -> f32 {
        self.scatter
    }
}

impl IArrivalOrder for DirectionalArrivalOrder {
    fn prepare(&mut self, _area: &FormationArea, seed: i64) {
        self.random = Some(DrawingRandom::new(seed ^ 0x1B873593));
    }

    fn get_key(&mut self, x: f32, y: f32, _clearance: f32) -> f32 {
        let mut key = (x as f64 * self.cos) + (y as f64 * self.sin);
        if self.scatter > 0.0 {
            if let Some(r) = self.random.as_mut() {
                key += r.uniform(-self.scatter as f64, self.scatter as f64);
            }
        }
        key as f32
    }
}
