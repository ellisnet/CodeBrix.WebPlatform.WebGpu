//! The `Planes` namespace of CodeBrix.Imaging.Drawing, as CALCULATION ON THE CPU with the same
//! API and the same values: pixel planes ([`DrawingPlane`], [`DrawingPlaneImage`]) with their
//! arithmetic, tone, vignette, blur, resampling, bloom and compositing; [`DrawingTransition`]s as
//! functions of a fraction; seeded randomness ([`DrawingRandom`]) and noise ([`DrawingNoise`],
//! also over planes); particles as a function of time ([`DrawingParticleSystem`]); motion blur by
//! accumulation ([`DrawingFrameAccumulator`], [`DrawingMotionBlur`]).
//!
//! SINGLE-THREADED: a browser gives a frontend one thread, so every operation runs on it (C#
//! shares rows out across threads; its results never depend on that, so the values are the
//! same). Whole-frame operations at video sizes cost milliseconds to tens of milliseconds each
//! here - see the crate's documents for the measured times - so a real-time scene uses the GPU
//! versions of the named whole-frame operations, and keeps planes for masks, data and small
//! images. Raw pixel access (`data`, `to_rgba`) is in memory only: there are no file encoders.

mod drawing_frame_accumulator;
mod drawing_motion_blur;
mod drawing_noise;
mod drawing_noise_fields;
mod drawing_noise_kind;
mod drawing_noise_options;
mod drawing_particle;
mod drawing_particle_emitter;
mod drawing_particle_system;
mod drawing_plane;
mod drawing_plane_bloom_options;
mod drawing_plane_image;
mod drawing_plane_resampling;
mod drawing_random;
mod drawing_transition;
mod drawing_transition_direction;
mod drawing_transition_kind;
pub(crate) mod plane_kernels;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_planes;
#[cfg(test)]
mod tests_reference;

pub use drawing_frame_accumulator::DrawingFrameAccumulator;
pub use drawing_motion_blur::DrawingMotionBlur;
pub use drawing_noise::DrawingNoise;
pub use drawing_noise_kind::DrawingNoiseKind;
pub use drawing_noise_options::{DrawingNoiseFillOptions, DrawingNoiseFractalOptions};
pub use drawing_particle::DrawingParticle;
pub use drawing_particle_emitter::DrawingParticleEmitter;
pub use drawing_particle_system::DrawingParticleSystem;
pub use drawing_plane::DrawingPlane;
pub use drawing_plane_bloom_options::DrawingPlaneBloomOptions;
pub use drawing_plane_image::DrawingPlaneImage;
pub use drawing_plane_resampling::DrawingPlaneResampling;
pub use drawing_random::DrawingRandom;
pub use drawing_transition::DrawingTransition;
pub use drawing_transition_direction::DrawingTransitionDirection;
pub use drawing_transition_kind::DrawingTransitionKind;
