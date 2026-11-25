//! The water factories of [`GraphicsContext`] (Not in C#): the wave field.

use crate::context::{GraphicsContext, GraphicsException};
use crate::water::wave_field::{WaveField, WaveFieldOptions};

impl GraphicsContext {
    /// A calm [`WaveField`] of `options` (Not in C#). Err for a grid outside
    /// [`WaveField::GRID_RANGE`], a size that is not positive, a rate or step cap out of range,
    /// or a wave speed too fast for the rate (the message names the lowest `steps_per_second`
    /// that holds it).
    pub fn create_wave_field(&self, options: &WaveFieldOptions) -> Result<WaveField, GraphicsException> {
        WaveField::new(self, options)
    }
}
