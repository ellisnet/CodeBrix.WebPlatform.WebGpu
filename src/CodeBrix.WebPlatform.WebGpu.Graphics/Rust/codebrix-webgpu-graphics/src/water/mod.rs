//! Water (Not in C#): the simulated wave field ([`WaveField`], its CPU twin [`CpuWaveField`],
//! [`WaveMask`], [`WaveRain`]), the open-water waves ([`OpenWaterWaves`]) and the water surface
//! for 3D scenes ([`WaterRenderer`], [`WaterSurface`], [`WaterLook`] and its parts, [`WaterMask`]
//! and [`WaterCoverage`]), with their context factories.

mod gerstner_math;
mod graphics_context_water;
mod open_water_waves;
mod scene_capture;
mod wave_field;
mod wave_mask;
mod wave_math;
mod wave_rain;
mod water_look;
mod water_mask;
mod water_math;
mod water_renderer;
mod water_surface;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_surface;
#[cfg(test)]
mod tests_shape_flow;

pub use open_water_waves::{GerstnerWave, GerstnerWaveRecord, OpenWaterWaves};
pub use wave_field::{WaveDrop, WaveEdge, WaveField, WaveFieldOptions, WaveStir, WaveWake};
pub use wave_mask::WaveMask;
pub use wave_math::CpuWaveField;
pub use wave_rain::WaveRain;
pub use water_look::{WaterCaustics, WaterDetail, WaterLook, WaterSheen};
pub use water_mask::{WaterCoverage, WaterMask};
pub use water_renderer::{WaterFrame, WaterRenderStats, WaterRenderer};
pub use water_surface::{WaterField, WaterSurface};
