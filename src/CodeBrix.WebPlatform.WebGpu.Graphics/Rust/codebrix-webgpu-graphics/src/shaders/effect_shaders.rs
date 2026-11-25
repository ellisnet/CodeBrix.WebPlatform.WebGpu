//! [`EffectShaders`]: the WGSL sources of the effect renderers - the glow layer (also the particle
//! system's drawing), bloom and the particle step (Not in C#). Compiled into the crate as `.wgsl`
//! files like the other built-in shaders.

/// The effect renderers' shaders (Not in C#).
pub struct EffectShaders;

impl EffectShaders {
    /// The glow layer's camera-facing instanced quads (also how particles are drawn): entry
    /// points `vs_main` and `fs_main`.
    pub const GLOW: &'static str = include_str!("wgsl/glow.wgsl");
    /// Bloom's steps: `vs_main` (a full-screen triangle) with `fs_bright`, `fs_down` and `fs_up`.
    pub const BLOOM: &'static str = include_str!("wgsl/bloom.wgsl");
    /// The particle system's compute step: entry point `step_particles`.
    pub const PARTICLES: &'static str = include_str!("wgsl/particles.wgsl");
    /// Every effect shader by name, for a page that compiles each built-in shader once.
    pub fn all() -> Vec<(&'static str, &'static str)> {
        vec![("glow", EffectShaders::GLOW), ("bloom", EffectShaders::BLOOM), ("particles", EffectShaders::PARTICLES)]
    }
}

#[cfg(test)]
mod tests {
    use super::EffectShaders;

    #[test]
    fn every_effect_shader_has_its_entry_points() {
        assert!(EffectShaders::GLOW.contains("fn vs_main(") && EffectShaders::GLOW.contains("fn fs_main("));
        for entry in ["fn vs_main(", "fn fs_bright(", "fn fs_down(", "fn fs_up("] {
            assert!(EffectShaders::BLOOM.contains(entry));
        }
        assert!(EffectShaders::PARTICLES.contains("fn step_particles("));
        assert_eq!(EffectShaders::all().len(), 3);
    }
}
