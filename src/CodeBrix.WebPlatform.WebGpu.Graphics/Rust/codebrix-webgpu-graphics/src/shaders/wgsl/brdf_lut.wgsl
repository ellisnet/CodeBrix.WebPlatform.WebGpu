const SAMPLES: u32 = 4096u;

@group(0) @binding(0) var<uniform> bake: Bake;

fn visibility_smith(n_dot_l: f32, n_dot_v: f32, alpha: f32) -> f32 {
    let alpha2 = alpha * alpha;
    let ggx_v = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - alpha2) + alpha2);
    let ggx_l = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - alpha2) + alpha2);
    let sum = ggx_v + ggx_l;
    return select(0.0, 0.5 / sum, sum > 0.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let n_dot_v = position.x / f32(bake.size);
    let roughness = position.y / f32(bake.size);
    let alpha = max(roughness * roughness, 0.0001);
    let v = vec3<f32>(sqrt(1.0 - n_dot_v * n_dot_v), 0.0, n_dot_v);
    let n = vec3<f32>(0.0, 0.0, 1.0);
    var scale = 0.0;
    var bias = 0.0;
    for (var i = 0u; i < SAMPLES; i++) {
        let h = importance_sample_ggx(hammersley(i, SAMPLES), n, alpha);
        let v_dot_h = dot(v, h);
        let l = 2.0 * v_dot_h * h - v;
        let n_dot_l = l.z;
        let n_dot_h = max(h.z, 1e-6);
        if (n_dot_l > 0.0 && v_dot_h > 0.0) {
            let g = visibility_smith(n_dot_l, n_dot_v, alpha) * 4.0 * n_dot_l * v_dot_h / n_dot_h;
            let fc = pow(1.0 - v_dot_h, 5.0);
            scale += (1.0 - fc) * g;
            bias += fc * g;
        }
    }
    return vec4<f32>(scale / f32(SAMPLES), bias / f32(SAMPLES), 0.0, 1.0);
}
