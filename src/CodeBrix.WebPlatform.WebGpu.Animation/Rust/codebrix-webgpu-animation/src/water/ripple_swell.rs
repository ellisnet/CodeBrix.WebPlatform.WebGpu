//! `RippleSwell` and `SwellWave`: the slow travelling waves under a ripple field's drops (no C#
//! equivalent).

use crate::planes::DrawingRandom;

/// One travelling wave of a swell (no C# equivalent): `height sin(k (d . p) - k speed t + phase)`
/// with `k = 2 pi / wavelength` and `d` the direction it travels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwellWave {
    /// Where it travels, in degrees clockwise from +x (y points down, so 90 travels down the
    /// screen). Default 0.
    pub angle: f32,
    /// Crest to crest, calibrated units; more than 0. Default 160.
    pub wavelength: f32,
    /// The crest height, calibrated units. Default 0.5.
    pub height: f32,
    /// Calibrated units per second the crests travel (0 = a standing pattern that does not
    /// move). Default 40.
    pub speed: f32,
    /// The phase at session time 0, radians. Default 0.
    pub phase: f32,
}

impl Default for SwellWave {
    fn default() -> SwellWave {
        SwellWave { angle: 0.0, wavelength: 160.0, height: 0.5, speed: 40.0, phase: 0.0 }
    }
}

/// A slow swell of travelling waves under a ripple field's drops (no C# equivalent): at most
/// [`RippleSwell::MAX_WAVES`] [`SwellWave`]s summed. A pure function of the time like the rest
/// of the field.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RippleSwell {
    /// The waves, at most [`RippleSwell::MAX_WAVES`].
    pub waves: Vec<SwellWave>,
}

impl RippleSwell {
    /// The most waves a swell holds (the GPU loops over them per pixel).
    pub const MAX_WAVES: usize = 8;

    /// A swell of the given waves (no C# equivalent).
    pub fn new(waves: Vec<SwellWave>) -> RippleSwell {
        RippleSwell { waves }
    }

    /// A gentle swell of FOUR seeded waves (no C# equivalent) around a main direction drawn from
    /// the seed: wave 0 travels that way with `wavelength`; waves 1 to 3 turn up to 50 degrees
    /// either side and have 0.55 to 1.45 times the wavelength. Each travels at a quarter of its
    /// own wavelength per second times `sqrt(its wavelength / wavelength)` (longer waves are
    /// faster, as on water), with a seeded phase; the heights are shared out in proportion to
    /// the wavelengths so that the four together never exceed `height`. The draws, from
    /// `DrawingRandom::new(seed)`, in order: the main angle (0..360), then per wave 1..3 its turn,
    /// its wavelength factor; then the four phases.
    pub fn gentle(seed: i64, wavelength: f32, height: f32) -> RippleSwell {
        let mut random = DrawingRandom::new(seed);
        let main = random.uniform(0.0, 360.0);
        let mut angles = [main; 4];
        let mut factors = [1.0f64; 4];
        for i in 1..4 {
            angles[i] = main + random.uniform(-50.0, 50.0);
            factors[i] = random.uniform(0.55, 1.45);
        }
        let total: f64 = factors.iter().sum();
        let waves = (0..4)
            .map(|i| {
                let lambda = wavelength as f64 * factors[i];
                SwellWave {
                    angle: angles[i].rem_euclid(360.0) as f32,
                    wavelength: lambda as f32,
                    height: (height as f64 * factors[i] / total) as f32,
                    speed: (0.25 * lambda * factors[i].sqrt()) as f32,
                    phase: random.uniform(0.0, std::f64::consts::TAU) as f32,
                }
            })
            .collect();
        RippleSwell { waves }
    }

    /// Checks the swell; the error names the wave and says what is wrong.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.waves.len() > RippleSwell::MAX_WAVES {
            return Err(format!("A swell holds at most {} waves (it has {}); combine or drop some.", RippleSwell::MAX_WAVES, self.waves.len()));
        }
        for (i, w) in self.waves.iter().enumerate() {
            if !(w.wavelength.is_finite() && w.wavelength > 0.0) {
                return Err(format!("Swell wave {i}: the wavelength must be a finite number above 0 (it is {}).", w.wavelength));
            }
            if !(w.angle.is_finite() && w.height.is_finite() && w.speed.is_finite() && w.phase.is_finite()) {
                return Err(format!("Swell wave {i}: the angle, height, speed and phase must be finite numbers."));
            }
        }
        Ok(())
    }

    /// The waves' GPU records at a time (`[k dx, k dy, height, phase now]`, phase in double
    /// precision reduced to 0..2 pi).
    pub(crate) fn records(&self, seconds: f64) -> impl Iterator<Item = [f32; 4]> + '_ {
        self.waves.iter().map(move |w| {
            let k = std::f64::consts::TAU / w.wavelength as f64;
            let angle = (w.angle as f64).to_radians();
            let phase = (w.phase as f64 - k * w.speed as f64 * seconds).rem_euclid(std::f64::consts::TAU);
            [(k * angle.cos()) as f32, (k * angle.sin()) as f32, w.height, phase as f32]
        })
    }
}
