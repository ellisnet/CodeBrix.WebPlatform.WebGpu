@group(0) @binding(0) var<uniform> bake: Bake;
@group(0) @binding(1) var radiance: texture_2d_array<f32>;

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let n = face_direction(bake.face, position.xy / f32(bake.size));
    let source = bake.count;
    let inverse = 1.0 / f32(source);
    var sum = vec3<f32>(0.0);
    var weight = 0.0;
    for (var face = 0u; face < 6u; face++) {
        for (var y = 0u; y < source; y++) {
            for (var x = 0u; x < source; x++) {
                let uv = (vec2<f32>(f32(x), f32(y)) + vec2<f32>(0.5)) * inverse;
                let st = uv * 2.0 - vec2<f32>(1.0);
                let l = face_direction(face, uv);
                let cosine = dot(n, l);
                if (cosine > 0.0) {
                    // The solid angle of a texel at (s, t) on a unit-distance face: dA / (1 + s^2 + t^2)^(3/2).
                    let r2 = 1.0 + dot(st, st);
                    let w = cosine / (r2 * sqrt(r2));
                    sum += textureLoad(radiance, vec2<i32>(i32(x), i32(y)), i32(face), i32(bake.level)).rgb * w;
                    weight += w;
                }
            }
        }
    }
    return vec4<f32>(sum / max(weight, 1e-8), 1.0);
}
