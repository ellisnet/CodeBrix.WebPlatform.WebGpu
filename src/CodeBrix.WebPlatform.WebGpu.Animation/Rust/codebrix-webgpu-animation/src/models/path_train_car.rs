//! `PathTrainCar`: one car of a train - its element, rank, gap, delay and roll (C# `PathTrainCar`).

use crate::geometry::PointF;
use crate::models::ElementId;

/// One car of a [`PathTrain`](crate::models::PathTrain) (C# `PathTrainCar`): its element, its
/// index in the list given, its rank (by the train's order), its gap behind the lead (rank x
/// spacing), its delay (rank x stagger) and its rolling radius. Change a car's gap, delay or roll
/// through [`PathTrain::set_car`](crate::models::PathTrain::set_car).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathTrainCar {
    pub(crate) element: ElementId,
    pub(crate) index: usize,
    pub(crate) rank: usize,
    pub(crate) gap: f32,
    pub(crate) delay: f64,
    pub(crate) roll_radius: f32,
    /// The point of the path that maps to the element's anchor: None = the anchor itself (the
    /// anchor rides the path), or the path's end (the element travels a copy of the path ending
    /// at its anchor).
    pub(crate) reference: Option<PointF>,
}

impl PathTrainCar {
    /// The element (C# `Element`).
    pub fn element(&self) -> ElementId {
        self.element
    }

    /// Its index in the list of elements given (C# `Index`).
    pub fn index(&self) -> usize {
        self.index
    }

    /// Its rank by the train's order; rank 0 leads (C# `Rank`).
    pub fn rank(&self) -> usize {
        self.rank
    }

    /// Its gap behind the lead, calibrated units along the path (C# `Gap`).
    pub fn gap(&self) -> f32 {
        self.gap
    }

    /// Sets the gap; the error says what to do with a negative or non-finite value.
    pub fn set_gap(&mut self, value: f32) -> Result<(), String> {
        if !value.is_finite() || value < 0.0 {
            return Err("A car's gap must be a finite distance, 0 or more.".to_string());
        }
        self.gap = value;
        Ok(())
    }

    /// Seconds it follows the progress later than the lead (C# `Delay`).
    pub fn delay(&self) -> f64 {
        self.delay
    }

    /// Sets the delay; the error says what to do with a negative or non-finite value.
    pub fn set_delay(&mut self, value: f64) -> Result<(), String> {
        if !value.is_finite() || value < 0.0 {
            return Err("A car's delay must be a finite number of seconds, 0 or more.".to_string());
        }
        self.delay = value;
        Ok(())
    }

    /// Its rolling radius; 0 rolls not at all (C# `RollRadius`).
    pub fn roll_radius(&self) -> f32 {
        self.roll_radius
    }

    /// Sets the rolling radius; the error says what to do with a non-finite value.
    pub fn set_roll_radius(&mut self, value: f32) -> Result<(), String> {
        if !value.is_finite() {
            return Err("A car's roll radius must be a finite number (0 for none).".to_string());
        }
        self.roll_radius = value;
        Ok(())
    }
}
