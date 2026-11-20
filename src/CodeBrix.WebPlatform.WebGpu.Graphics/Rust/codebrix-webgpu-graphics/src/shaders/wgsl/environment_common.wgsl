const PI: f32 = 3.14159265358979;

struct Bake {
    face: u32,
    count: u32,
    level: u32,
    size: u32,
    a: vec4<f32>,
    b: vec4<f32>,
    c: vec4<f32>,
    d: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
}

// The direction through a point of a cube face (uv 0..1, v down), WebGPU / Vulkan / D3D cube conventions.
fn face_direction(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let s = uv.x * 2.0 - 1.0;
    let t = uv.y * 2.0 - 1.0;
    var d: vec3<f32>;
    switch face {
        case 0u: { d = vec3<f32>(1.0, -t, -s); }
        case 1u: { d = vec3<f32>(-1.0, -t, s); }
        case 2u: { d = vec3<f32>(s, 1.0, t); }
        case 3u: { d = vec3<f32>(s, -1.0, -t); }
        case 4u: { d = vec3<f32>(s, -t, 1.0); }
        default: { d = vec3<f32>(-s, -t, -1.0); }
    }
    return normalize(d);
}

fn hammersley(i: u32, count: u32) -> vec2<f32> {
    return vec2<f32>(f32(i) / f32(count), f32(reverseBits(i)) * 2.3283064365386963e-10);
}

// A GGX half vector around n for a uniform 2D sample (alpha = roughness squared).
fn importance_sample_ggx(xi: vec2<f32>, n: vec3<f32>, alpha: f32) -> vec3<f32> {
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt((1.0 - xi.y) / (1.0 + (alpha * alpha - 1.0) * xi.y));
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let h = vec3<f32>(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);
    let up = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), abs(n.z) < 0.999);
    let tangent = normalize(cross(up, n));
    let bitangent = cross(n, tangent);
    return normalize(tangent * h.x + bitangent * h.y + n * h.z);
}

fn distribution_ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let alpha2 = alpha * alpha;
    let f = n_dot_h * n_dot_h * (alpha2 - 1.0) + 1.0;
    return alpha2 / (PI * f * f);
}
