struct Sky {
    zenith: vec4<f32>,
    horizon: vec4<f32>,
    ground: vec4<f32>,
    sun: vec4<f32>,
    sun_color: vec4<f32>,
    glow: vec4<f32>,
    up: vec4<f32>,
};

@group(0) @binding(0) var<uniform> bake: Bake;
@group(0) @binding(1) var<uniform> sky: Sky;

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let d = face_direction(bake.face, position.xy / f32(bake.size));
    let h = dot(d, sky.up.xyz);
    let above = max(h, 0.0);
    let band = pow(1.0 - above, sky.horizon.w);
    let gradient = mix(sky.zenith.rgb, sky.horizon.rgb, band);
    let horizon_blend = smoothstep(-0.04, 0.01, h);
    let below = mix(sky.horizon.rgb * 0.6, sky.ground.rgb, smoothstep(0.0, 0.25, -h));
    var color = mix(below, gradient, horizon_blend);

    let cos_sun = dot(d, sky.sun.xyz);
    let c = max(cos_sun, 0.0);
    let glow = sky.glow.x * pow(c, sky.glow.y) + sky.glow.z * pow(c, sky.glow.w);
    color += sky.sun_color.rgb * glow * mix(0.15, 1.0, horizon_blend);
    color *= sky.up.w;

    // The disc, anti-aliased over about one texel (angle in radians).
    let texel = 1.5707963 / f32(bake.size);
    let angle = acos(clamp(cos_sun, -1.0, 1.0));
    let coverage = 1.0 - smoothstep(sky.sun.w - texel * 0.5, sky.sun.w + texel * 0.5, angle);
    color += sky.sun_color.rgb * sky.sun_color.w * coverage;
    return vec4<f32>(min(color, vec3<f32>(60000.0)), 1.0);
}
