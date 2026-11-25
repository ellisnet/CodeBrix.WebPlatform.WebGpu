//! [`WaterShaders`]: the WGSL sources of the water (Not in C#) - the wave field's compute steps,
//! the water surface and the water's scene capture.
//! Compiled into the crate as `.wgsl` files like the other built-in shaders.

/// The water's shaders (Not in C#).
pub struct WaterShaders;

impl WaterShaders {
    /// The wave field: entry points `disturb_field`, `step_field` and `publish_field` (8 x 8
    /// workgroups).
    pub const WAVE_FIELD: &'static str = include_str!("wgsl/wave_field.wgsl");
    /// The water surface: entry points `vs_main` (the grid from the vertex index, moved by the
    /// wave field and the open-water waves) and `fs_main`.
    pub const SURFACE: &'static str = include_str!("wgsl/water_surface.wgsl");
    /// The water's scene capture: `vs_main` (a full-screen triangle) with `fs_single` (a 1-sample
    /// depth) and `fs_multi` (sample 0 of a 4-sample depth).
    pub const CAPTURE: &'static str = include_str!("wgsl/water_capture.wgsl");
    /// Every water shader by name, for a page that compiles each built-in shader once.
    pub fn all() -> Vec<(&'static str, &'static str)> {
        vec![("wave field", WaterShaders::WAVE_FIELD), ("water surface", WaterShaders::SURFACE), ("water capture", WaterShaders::CAPTURE)]
    }
}

#[cfg(test)]
mod tests {
    use super::WaterShaders;

    #[test]
    fn every_water_shader_has_its_entry_points() {
        for entry in ["fn disturb_field(", "fn step_field(", "fn publish_field("] {
            assert!(WaterShaders::WAVE_FIELD.contains(entry), "{entry}");
        }
        assert!(WaterShaders::SURFACE.contains("fn vs_main(") && WaterShaders::SURFACE.contains("fn fs_main("));
        for entry in ["fn vs_main(", "fn fs_single(", "fn fs_multi("] {
            assert!(WaterShaders::CAPTURE.contains(entry), "{entry}");
        }
        assert_eq!(WaterShaders::all().len(), 3);
    }
}
