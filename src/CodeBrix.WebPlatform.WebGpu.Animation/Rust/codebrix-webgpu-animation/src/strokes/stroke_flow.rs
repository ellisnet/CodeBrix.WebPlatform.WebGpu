//! `StrokeFlow`: an animated swirl of a stroke set's directions, evaluated per frame on the GPU
//! from the session time (no C# equivalent).

/// Flow of a [`BrushStrokesShape`](crate::shapes::BrushStrokesShape): every stroke is turned
/// about its centre by `turn` degrees times the GPU's 3D gradient noise (one octave, -1..1) at
/// the stroke's centre (set units times `frequency`) and z = time x `speed` - so neighbouring
/// strokes turn together and the pattern drifts with the time. It is the GPU noise pattern
/// (`NoiseShape`'s), NOT `DrawingNoise`'s; an exact function of the session time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeFlow {
    /// The noise's seed. Default 0.
    pub seed: u32,
    /// The noise's frequency per set unit (larger = smaller swirls). Default 0.01.
    pub frequency: f32,
    /// How fast the pattern changes: noise z per second. Default 0.2.
    pub speed: f32,
    /// The turn at noise +-1, degrees. Default 30.
    pub turn: f32,
}

impl Default for StrokeFlow {
    fn default() -> StrokeFlow {
        StrokeFlow { seed: 0, frequency: 0.01, speed: 0.2, turn: 30.0 }
    }
}

impl StrokeFlow {
    /// The checks a shape makes when drawing: frequency, speed and turn finite, frequency not
    /// negative.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !self.frequency.is_finite() || self.frequency < 0.0 || !self.speed.is_finite() || !self.turn.is_finite() {
            return Err("A stroke flow's frequency must be finite and not negative, and its speed and turn finite.".to_string());
        }
        Ok(())
    }
}
