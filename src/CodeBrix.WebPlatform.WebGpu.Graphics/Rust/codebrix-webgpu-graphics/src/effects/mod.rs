//! Effects (Not in C#): the glow layer (soft additive sprites in the 3D scene), bloom on the HDR
//! path and the GPU particle system (drawn with the glow layer's drawing), with their context
//! factories.

mod bloom_renderer;
mod glow_drawing;
mod glow_layer;
mod graphics_context_effects;
mod particle_math;
mod particle_system;

pub use bloom_renderer::{BloomRenderer, BloomSettings};
pub use glow_drawing::GlowBlend;
pub use glow_layer::{Glow, GlowLayer, GlowPulse};
pub use particle_system::{EmitterShape, ParticleEmitter, ParticleSettings, ParticleSystem, ParticleSystemOptions};

use crate::presentation::RenderTargetView;

/// The sample count of a pass's target, the third part of the effect renderers' pipeline keys
/// beside the colour and depth formats: 1, or 4 on a multisampled target (the glow layer and the
/// particles then draw with their 4-sample pipelines; bloom's own chain is always 1).
pub(crate) fn target_sample_count(target: &RenderTargetView) -> u32 {
    target.sample_count()
}
