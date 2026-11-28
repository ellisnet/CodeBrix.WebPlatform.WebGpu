// The water surface's sun map when it is ONE map (Not in C#; after water_surface.wgsl in
// WaterShaders::SURFACE): the single map, or the last cascade's layer for a receiver of one map.

@group(3) @binding(0) var sun_map: texture_depth_2d;

// One filtered comparison of the sun's map at `uv` against `depth` (`layer` is always 0 here).
fn sun_compare(uv: vec2<f32>, layer: i32, depth: f32) -> f32 {
    return textureSampleCompareLevel(sun_map, shadow_sampler, uv, depth);
}
