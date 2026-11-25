//! [`WaveRain`]: seeded rain on a wave field (Not in C#), and the CPU arithmetic that turns it
//! into drops - an integer PCG hash of (seed, drop number, dimension), so the same seed and the
//! same sequence of deltas give the same rain on every machine.

use glam::Vec2;

use crate::water::wave_field::WaveDrop;

/// Rain on a wave field (Not in C#): `rate` drops per simulated second at seeded places, sizes
/// and depths. The drops are numbered from the field's creation or last reset; drop `n`'s place,
/// radius and depth are a pure function of (`seed`, `n`). A long frame never saves up a downpour
/// (at most one second's worth at once).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveRain {
    /// Drops per simulated second (0: none). Default 1.5.
    pub rate: f32,
    /// The smallest and largest drop radius, field units. Default (0.04, 0.08).
    pub radius: (f32, f32),
    /// The smallest and largest depth, field units. Default (0.01, 0.03).
    pub depth: (f32, f32),
    /// Where drops fall: (min, max) corners in field units; None = the whole field.
    pub area: Option<(Vec2, Vec2)>,
    /// The seed. Default 1.
    pub seed: u32,
}

impl Default for WaveRain {
    fn default() -> WaveRain {
        WaveRain { rate: 1.5, radius: (0.04, 0.08), depth: (0.01, 0.03), area: None, seed: 1 }
    }
}

/// The PCG hash (a permuted congruential step of one 32-bit word; the particle system's).
pub(crate) fn pcg(value: u32) -> u32 {
    let state = value.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

/// The random number in 0..1 of drop `number` for draw `dimension` under `seed`.
pub(crate) fn random(seed: u32, number: u32, dimension: u32) -> f32 {
    let h = pcg(pcg(number ^ pcg(seed)) ^ dimension.wrapping_mul(0x9E37_79B9));
    (h >> 8) as f32 * (1.0 / 16_777_216.0)
}

/// Drop `number` of `rain` on a field of `size`.
pub(crate) fn rain_drop(rain: &WaveRain, number: u32, size: Vec2) -> WaveDrop {
    let (low, high) = rain.area.unwrap_or((Vec2::ZERO, size));
    let r = |dimension| random(rain.seed, number, dimension);
    let between = |range: (f32, f32), t: f32| range.0 + (range.1 - range.0) * t;
    WaveDrop { position: low + (high - low) * Vec2::new(r(0), r(1)), radius: between(rain.radius, r(2)).max(0.0), depth: between(rain.depth, r(3)) }
}

/// How far the rain has got: the next drop's number and the fraction of a drop carried over.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RainState {
    number: u32,
    carried: f64,
}

impl RainState {
    /// The drops of `seconds` of simulated time (the rate's share plus the carried fraction; at
    /// most one second's worth).
    pub(crate) fn take(&mut self, rain: &WaveRain, seconds: f64, size: Vec2) -> Vec<WaveDrop> {
        let rate = rain.rate.max(0.0) as f64;
        let total = self.carried + rate * seconds.max(0.0);
        let whole = total.floor();
        self.carried = total - whole;
        let count = whole.min(rate.ceil().max(1.0)) as u32;
        let drops = (0..count).map(|i| rain_drop(rain, self.number.wrapping_add(i), size)).collect();
        self.number = self.number.wrapping_add(count);
        drops
    }
}
