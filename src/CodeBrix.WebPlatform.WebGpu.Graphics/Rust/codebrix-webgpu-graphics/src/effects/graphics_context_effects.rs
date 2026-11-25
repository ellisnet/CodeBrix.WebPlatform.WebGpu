//! The effect factories of [`GraphicsContext`] (Not in C#): the glow layer, the bloom renderer
//! and the particle system.

use crate::context::{GraphicsContext, GraphicsException};
use crate::effects::{BloomRenderer, GlowLayer, ParticleSystem, ParticleSystemOptions};

impl GraphicsContext {
    /// A [`GlowLayer`] with room for [`GlowLayer::DEFAULT_CAPACITY`] glows per frame before its
    /// buffer grows (Not in C#).
    pub fn create_glow_layer(&self) -> Result<GlowLayer, GraphicsException> {
        GlowLayer::new(self)
    }

    /// A [`BloomRenderer`] with [`crate::BloomSettings`] defaults (Not in C#).
    pub fn create_bloom_renderer(&self) -> Result<BloomRenderer, GraphicsException> {
        BloomRenderer::new(self)
    }

    /// A [`ParticleSystem`] of `options.capacity` particles with the default emitter and
    /// settings (Not in C#). Err for a capacity outside 1 to [`ParticleSystem::MAX_CAPACITY`].
    pub fn create_particle_system(&self, options: &ParticleSystemOptions) -> Result<ParticleSystem, GraphicsException> {
        ParticleSystem::new(self, options)
    }
}
