//! `StrokeBoil` and `boil_epoch`: a stroke set redrawn a few times a second with every stroke
//! jittered a little - the "boiling" line of hand-drawn film - as a function of the session time
//! (no C# equivalent). The one timing rule is shared with the sketch helper
//! ([`SketchOptions::boiled`](crate::paths::SketchOptions::boiled)).

/// Boiling of a [`BrushStrokesShape`](crate::shapes::BrushStrokesShape): `rate` times a second
/// (of the session time) every stroke is drawn again moved, turned and resized by a little, each
/// stroke differently (by its seed), the same at the same time always - pausing freezes the boil,
/// a seek shows the same jitter. The jitter is evaluated on the GPU.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeBoil {
    /// Redraws per second (8 to 12 looks hand-drawn); 0 = never (no jitter at all). Default 10.
    pub rate: f32,
    /// How far each stroke may move per redraw, set units, either way on each axis. Default 1.
    pub offset: f32,
    /// How far each stroke may turn per redraw, degrees either way. Default 3.
    pub turn: f32,
    /// How much each stroke's length and width may change per redraw, a fraction either way.
    /// Default 0.05.
    pub size: f32,
    /// The boil's own seed (two shapes with different seeds boil differently). Default 0.
    pub seed: u32,
}

impl Default for StrokeBoil {
    fn default() -> StrokeBoil {
        StrokeBoil { rate: 10.0, offset: 1.0, turn: 3.0, size: 0.05, seed: 0 }
    }
}

impl StrokeBoil {
    /// The checks a shape makes when drawing: every value finite, rate, offset, turn and size not
    /// negative, size below 1.
    pub(crate) fn validate(&self) -> Result<(), String> {
        let ok = |v: f32| v.is_finite() && v >= 0.0;
        if !ok(self.rate) || !ok(self.offset) || !ok(self.turn) || !ok(self.size) || self.size >= 1.0 {
            return Err("A stroke boil's rate, offset, turn and size must be finite and not negative, and its size below 1.".to_string());
        }
        Ok(())
    }
}

/// The redraw number at a time: `floor(seconds x rate)` - 0 when the rate is 0 or below or not
/// finite, or the time not finite. The one boil timing of the crate: the sketch helper's
/// `boiled` and a stroke set's [`StrokeBoil`] both use it.
pub fn boil_epoch(seconds: f64, rate: f32) -> i64 {
    if !seconds.is_finite() || !rate.is_finite() || rate <= 0.0 {
        return 0;
    }
    (seconds * rate as f64).floor() as i64
}

#[cfg(test)]
/// One stroke's jitter for a redraw, as the stroke shader computes it: four numbers in -1..1
/// from the GPU's 32-bit hash of (the boil's seed; the stroke's seed, the redraw number, k) for
/// k = 0..3 - the offset x and y (times `offset`), the turn (times `turn`), the size (times
/// `size`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoilJitter {
    pub(crate) dx: f32,
    pub(crate) dy: f32,
    pub(crate) turn_degrees: f32,
    pub(crate) size: f32,
}

#[cfg(test)]
/// A hash as a value in -1..1 (the shader's `hash_unit`).
pub(crate) fn hash_unit(h: u32) -> f32 {
    ((h as f32 / 4_294_967_295.0) * 2.0) - 1.0
}

#[cfg(test)]
/// The CPU mirror of the stroke shader's boil jitter.
pub(crate) fn boil_jitter(boil: &StrokeBoil, stroke_seed: u32, epoch: i64) -> BoilJitter {
    let e = epoch as i32;
    let j = |k: i32| hash_unit(crate::painting::color_rules::noise_hash(boil.seed, stroke_seed as i32, e, k));
    BoilJitter { dx: boil.offset * j(0), dy: boil.offset * j(1), turn_degrees: boil.turn * j(2), size: 1.0 + boil.size * j(3) }
}
