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
    /// wave field and the open-water waves), `fs_main` and `fs_masked` (shaped by a coverage
    /// mask, group 2 binding 2); the sun's map is one `texture_depth_2d` (group 3 binding 0).
    pub const SURFACE: &'static str = concat!(include_str!("wgsl/water_surface.wgsl"), "\n", include_str!("wgsl/water_sun_map.wgsl"));
    /// The water surface for the sun's CASCADES (Not in C#): [`WaterShaders::SURFACE`] with the
    /// sun's map a `texture_depth_2d_array` of one layer per cascade.
    pub const SURFACE_CASCADED: &'static str = concat!(include_str!("wgsl/water_surface.wgsl"), "\n", include_str!("wgsl/water_sun_cascades.wgsl"));
    /// The water's scene capture: `vs_main` (a full-screen triangle) with `fs_single` (a 1-sample
    /// depth) and `fs_multi` (sample 0 of a 4-sample depth).
    pub const CAPTURE: &'static str = include_str!("wgsl/water_capture.wgsl");
    /// Every water shader by name, for a page that compiles each built-in shader once.
    pub fn all() -> Vec<(&'static str, &'static str)> {
        vec![("wave field", WaterShaders::WAVE_FIELD), ("water surface", WaterShaders::SURFACE), ("water surface cascaded", WaterShaders::SURFACE_CASCADED), ("water capture", WaterShaders::CAPTURE)]
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
        for surface in [WaterShaders::SURFACE, WaterShaders::SURFACE_CASCADED] {
            assert!(surface.contains("fn vs_main(") && surface.contains("fn fs_main(") && surface.contains("fn fs_masked("));
            assert_eq!(surface.matches("@group(3) @binding(0) var sun_map:").count(), 1);
        }
        assert!(WaterShaders::SURFACE.contains("var sun_map: texture_depth_2d;") && WaterShaders::SURFACE_CASCADED.contains("var sun_map: texture_depth_2d_array;"));
        for entry in ["fn vs_main(", "fn fs_single(", "fn fs_multi("] {
            assert!(WaterShaders::CAPTURE.contains(entry), "{entry}");
        }
        assert_eq!(WaterShaders::all().len(), 4);
    }
}
