@group(0) @binding(0) var<uniform> bake: Bake;
@group(0) @binding(1) var radiance: texture_cube<f32>;
@group(0) @binding(2) var radiance_sampler: sampler;

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let n = face_direction(bake.face, position.xy / f32(bake.size));
    let roughness = bake.a.x;
    if (roughness <= 0.0) {
        return vec4<f32>(textureSampleLevel(radiance, radiance_sampler, n, bake.a.w).rgb, 1.0);
    }
    let alpha = roughness * roughness;
    let texel_solid_angle = 4.0 * PI / (6.0 * bake.a.y * bake.a.y);
    var sum = vec3<f32>(0.0);
    var weight = 0.0;
    for (var i = 0u; i < bake.count; i++) {
        let h = importance_sample_ggx(hammersley(i, bake.count), n, alpha);
        let n_dot_h = max(dot(n, h), 0.0);
        let l = 2.0 * n_dot_h * h - n;
        let n_dot_l = dot(n, l);
        if (n_dot_l > 0.0) {
            // pdf(l) = D * n.h / (4 v.h) = D / 4 with v = n.
            let pdf = distribution_ggx(n_dot_h, alpha) * 0.25;
            let sample_solid_angle = 1.0 / (f32(bake.count) * pdf + 1e-6);
            let lod = clamp(0.5 * log2(sample_solid_angle / texel_solid_angle) + 1.0, 0.0, bake.a.z);
            sum += textureSampleLevel(radiance, radiance_sampler, l, lod).rgb * n_dot_l;
            weight += n_dot_l;
        }
    }
    return vec4<f32>(sum / max(weight, 1e-8), 1.0);
}
