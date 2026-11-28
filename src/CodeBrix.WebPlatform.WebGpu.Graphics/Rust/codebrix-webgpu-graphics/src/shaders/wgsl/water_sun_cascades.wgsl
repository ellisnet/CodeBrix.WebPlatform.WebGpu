// The water surface's sun map with CASCADES (Not in C#; after water_surface.wgsl in
// WaterShaders::SURFACE_CASCADED): one layer per cascade, the nearest first.

@group(3) @binding(0) var sun_map: texture_depth_2d_array;

// One filtered comparison of cascade `layer` of the sun's map at `uv` against `depth`.
fn sun_compare(uv: vec2<f32>, layer: i32, depth: f32) -> f32 {
    return textureSampleCompareLevel(sun_map, shadow_sampler, uv, layer, depth);
}
