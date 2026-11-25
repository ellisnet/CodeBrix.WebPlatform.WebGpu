//! [`OpenWaterWaves`]: a sum of travelling trochoidal (Gerstner) waves, an exact function of
//! time, with [`GerstnerWave`] and the GPU record [`GerstnerWaveRecord`] (Not in C#). The
//! arithmetic is in `gerstner_math.rs`.

use glam::{Vec2, Vec3, Vec4};

use crate::context::GraphicsException;
use crate::water::gerstner_math::{self, ResolvedWave};
use crate::water::wave_rain::random;

/// One travelling (trochoidal, Gerstner) wave (Not in C#).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GerstnerWave {
    /// Where it travels: radians about +Y, from +X toward +Z. Default 0.
    pub direction: f32,
    /// Crest to crest, scene units (above 0). Default 8.
    pub wavelength: f32,
    /// Crest height above still water, scene units. Default 0.15.
    pub amplitude: f32,
    /// 0 (a sine wave) to 1 (a trochoid: water moves on circles, the crests sharpen): the
    /// horizontal radius of the water's motion is `steepness * amplitude`. Default 0.5.
    pub steepness: f32,
    /// The phase at time 0, radians. Default 0.
    pub phase: f32,
    /// The speed of the crests, scene units per second; None: deep water,
    /// `sqrt(g * wavelength / (2 pi))`. Default None.
    pub speed: Option<f32>,
}

impl Default for GerstnerWave {
    fn default() -> GerstnerWave {
        GerstnerWave { direction: 0.0, wavelength: 8.0, amplitude: 0.15, steepness: 0.5, phase: 0.0, speed: None }
    }
}

/// One wave as a shader reads it (Not in C#): 32 bytes, two vec4s, recomputed each frame by
/// [`OpenWaterWaves::records`] so the phase is exact for hours of page time (an f32 time on the
/// GPU would lose precision).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GerstnerWaveRecord {
    /// cos(direction), sin(direction), the wavenumber `k = 2 pi / wavelength`, the amplitude.
    pub direction: Vec4,
    /// The horizontal radius `steepness * amplitude`, the CURRENT phase `P = (omega t + phase)
    /// mod 2 pi` (radians), the wavelength, 0. The argument at a still-water point `(x, z)` is
    /// `k * dot(direction.xy, (x, z)) - P`.
    pub motion: Vec4,
}

const _: () = assert!(std::mem::size_of::<GerstnerWaveRecord>() == 32);
// SAFETY: #[repr(C)]; two Vec4 (16 bytes each, 4-byte aligned), no padding (size asserted
// above); any bits are valid.
unsafe impl bytemuck::Zeroable for GerstnerWaveRecord {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for GerstnerWaveRecord {}

/// OPEN-WATER WAVES (Not in C#): a sum of up to [`OpenWaterWaves::MAX_WAVES`] travelling
/// trochoidal waves that moves a water surface (its vertices) and gives its normals (per pixel),
/// an EXACT FUNCTION OF TIME (pause, speed and seeks all show the right waves); usable alone
/// (lakes, the sea) or on top of a [`crate::WaveField`] (heights and slopes add). The CPU twin
/// ([`OpenWaterWaves::height_at`], [`OpenWaterWaves::normal_at`]) floats things on it without any
/// GPU readback.
///
/// LIMITS: repetition shows with few waves (8 to 16 seeded waves hide it at normal viewing); no
/// breaking, whitecaps, foam or spray; deep-water dispersion only (waves do not slow or steepen
/// near a shore); they pass THROUGH islands and walls (a formula: a wave field is what reflects);
/// not an FFT ocean.
#[derive(Clone, Debug, PartialEq)]
pub struct OpenWaterWaves {
    waves: Vec<GerstnerWave>,
    resolved: Vec<ResolvedWave>,
    gravity: f32,
}

impl OpenWaterWaves {
    /// The most waves in one sum.
    pub const MAX_WAVES: usize = 16;
    /// Gravity in scene units per second squared when the scene unit is a metre.
    pub const EARTH_GRAVITY: f32 = 9.81;
    /// A wave shorter than this many grid cells of the surface that draws it is left out of the
    /// vertex displacement (it would alias between vertices) and kept in the normals.
    pub const DISPLACEMENT_CELLS: f32 = 4.0;

    /// The sum of `waves` under Earth's gravity. Err for more than [`OpenWaterWaves::MAX_WAVES`],
    /// a wavelength that is not above 0, a negative amplitude, a steepness outside 0 to 1, a
    /// speed that is not above 0, or waves that together would loop over themselves (the sum of
    /// `steepness x k x amplitude` above 1; the message gives the factor to scale the steepness
    /// by).
    pub fn new(waves: Vec<GerstnerWave>) -> Result<OpenWaterWaves, GraphicsException> {
        if waves.len() > OpenWaterWaves::MAX_WAVES {
            return Err(GraphicsException::new(format!("Open-water waves take at most {} waves; {} were given.", OpenWaterWaves::MAX_WAVES, waves.len())));
        }
        for (i, w) in waves.iter().enumerate() {
            if !(w.wavelength > 0.0 && w.wavelength.is_finite()) {
                return Err(GraphicsException::new(format!("Wave {i}'s wavelength must be above 0; it is {}.", w.wavelength)));
            }
            if !(w.amplitude >= 0.0 && w.amplitude.is_finite()) {
                return Err(GraphicsException::new(format!("Wave {i}'s amplitude must be 0 or more; it is {}.", w.amplitude)));
            }
            if !(0.0..=1.0).contains(&w.steepness) {
                return Err(GraphicsException::new(format!("Wave {i}'s steepness must be 0 to 1; it is {}.", w.steepness)));
            }
            if let Some(speed) = w.speed {
                if !(speed > 0.0 && speed.is_finite()) {
                    return Err(GraphicsException::new(format!("Wave {i}'s speed must be above 0 (or None for deep water); it is {speed}.")));
                }
            }
        }
        let sea = OpenWaterWaves::unchecked(waves, OpenWaterWaves::EARTH_GRAVITY);
        let factor = gerstner_math::loop_factor(&sea.resolved);
        if factor > 1.0 {
            return Err(GraphicsException::new(format!(
                "These waves would loop over themselves: the sum of steepness x k x amplitude (k = 2 pi / wavelength) is {factor:.3}, above 1. Scale every steepness by at most {:.3}, or lower the amplitudes.",
                1.0 / factor
            )));
        }
        Ok(sea)
    }

    fn unchecked(waves: Vec<GerstnerWave>, gravity: f32) -> OpenWaterWaves {
        let resolved = waves.iter().map(|w| gerstner_math::resolve(w, gravity)).collect();
        OpenWaterWaves { waves, resolved, gravity }
    }

    /// A SEEDED WIND SEA: `count` waves (1 to 16; 8 is a good start) travelling within +-60
    /// degrees of the wind's `direction` (radians about +Y, from +X toward +Z), with frequencies
    /// spread over 0.8 to 2.4 times the Pierson-Moskowitz peak for `wind_speed` (metres per
    /// second; the peak wavelength is about `0.83 * wind_speed^2` metres) and amplitudes from that
    /// spectrum, steepness 0.7 scaled down so nothing loops. Scene units are metres (gravity
    /// 9.81; [`OpenWaterWaves::with_gravity`] changes how fast they travel). A wind of 0 or less
    /// gives flat water. The same arguments give the same sea.
    pub fn from_wind(direction: f32, wind_speed: f32, count: u32, seed: u32) -> OpenWaterWaves {
        let g = OpenWaterWaves::EARTH_GRAVITY as f64;
        if !(wind_speed > 0.0 && wind_speed.is_finite()) {
            return OpenWaterWaves::unchecked(Vec::new(), OpenWaterWaves::EARTH_GRAVITY);
        }
        let count = count.clamp(1, OpenWaterWaves::MAX_WAVES as u32);
        let peak = 0.877 * g / wind_speed as f64;
        let band = 1.6 * peak / count as f64;
        let spread = 60f32.to_radians();
        let mut waves: Vec<GerstnerWave> = (0..count)
            .map(|i| {
                let r = |dimension| random(seed, i, dimension) as f64;
                let u = (i as f64 + 0.5 + 0.8 * (r(0) - 0.5)) / count as f64;
                let omega = peak * (0.8 + 1.6 * u);
                let spectrum = 0.0081 * g * g / omega.powi(5) * (-1.25 * (peak / omega).powi(4)).exp();
                GerstnerWave {
                    direction: direction + (2.0 * r(1) as f32 - 1.0) * spread,
                    wavelength: (std::f64::consts::TAU * g / (omega * omega)) as f32,
                    amplitude: (2.0 * spectrum * band).sqrt() as f32,
                    steepness: 0.7,
                    phase: (std::f64::consts::TAU * r(2)) as f32,
                    speed: None,
                }
            })
            .collect();
        let sea = OpenWaterWaves::unchecked(waves.clone(), OpenWaterWaves::EARTH_GRAVITY);
        let factor = gerstner_math::loop_factor(&sea.resolved);
        if factor > 0.9 {
            for w in &mut waves {
                w.steepness *= 0.9 / factor;
            }
        }
        OpenWaterWaves::unchecked(waves, OpenWaterWaves::EARTH_GRAVITY)
    }

    /// The waves.
    pub fn waves(&self) -> &[GerstnerWave] {
        &self.waves
    }

    /// The gravity in scene units per second squared (9.81 unless changed).
    pub fn gravity(&self) -> f32 {
        self.gravity
    }

    /// The same waves under another gravity (scene units per second squared; for a scene whose
    /// unit is not a metre): deep-water waves travel at `sqrt(g * wavelength / (2 pi))`. A value
    /// that is not above 0 is ignored.
    pub fn with_gravity(self, gravity: f32) -> OpenWaterWaves {
        if gravity > 0.0 && gravity.is_finite() {
            OpenWaterWaves::unchecked(self.waves, gravity)
        } else {
            self
        }
    }

    /// Where the still-water point (x, z) is at a time (`frame.info.time`): its horizontal and
    /// vertical displacement (x, height, z).
    pub fn displacement_at(&self, x: f32, z: f32, seconds: f64) -> Vec3 {
        let phases = gerstner_math::phases(&self.resolved, seconds);
        gerstner_math::displacement(&self.resolved, &phases, Vec2::new(x, z))
    }

    /// The unit normal of the surface ABOVE the horizontal point (x, z) at a time (as
    /// [`OpenWaterWaves::height_at`]) - to tilt a floating thing.
    pub fn normal_at(&self, x: f32, z: f32, seconds: f64) -> Vec3 {
        let phases = gerstner_math::phases(&self.resolved, seconds);
        let still = gerstner_math::still_point(&self.resolved, &phases, Vec2::new(x, z));
        gerstner_math::normal(&self.resolved, &phases, still)
    }

    /// The surface's height ABOVE the horizontal point (x, z) at a time (the displacement
    /// inverted: the still-water point that moves to (x, z) is found first) - to float things.
    pub fn height_at(&self, x: f32, z: f32, seconds: f64) -> f32 {
        let phases = gerstner_math::phases(&self.resolved, seconds);
        let still = gerstner_math::still_point(&self.resolved, &phases, Vec2::new(x, z));
        gerstner_math::displacement(&self.resolved, &phases, still).y
    }

    /// The highest the surface can rise (the sum of the amplitudes).
    pub fn max_height(&self) -> f32 {
        self.waves.iter().map(|w| w.amplitude).sum()
    }

    /// The waves as a shader reads them at a time ([`GerstnerWaveRecord`]: one per wave, the
    /// phases computed in f64 here). Write them into a uniform each frame.
    pub fn records(&self, seconds: f64) -> Vec<GerstnerWaveRecord> {
        self.resolved
            .iter()
            .map(|w| GerstnerWaveRecord {
                direction: Vec4::new(w.direction.x, w.direction.y, w.k, w.amplitude),
                motion: Vec4::new(w.horizontal, gerstner_math::current_phase(w, seconds) as f32, w.wavelength, 0.0),
            })
            .collect()
    }

    /// True when a wave of `wavelength` moves the vertices of a surface grid whose cells are
    /// `grid_cell` scene units ([`OpenWaterWaves::DISPLACEMENT_CELLS`] cells or longer); shorter
    /// waves only shade the normals.
    pub fn displaces_grid(wavelength: f32, grid_cell: f32) -> bool {
        wavelength >= OpenWaterWaves::DISPLACEMENT_CELLS * grid_cell
    }

    /// How much of a wave's normal is kept where one pixel covers `pixel_footprint` scene units:
    /// 0 for a wavelength of two pixels or less, 1 from four pixels, smooth between (no shimmer
    /// far away).
    pub fn normal_fade(wavelength: f32, pixel_footprint: f32) -> f32 {
        if pixel_footprint <= 0.0 {
            return 1.0;
        }
        let x = ((wavelength / pixel_footprint - 2.0) / 2.0).clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    }
}
