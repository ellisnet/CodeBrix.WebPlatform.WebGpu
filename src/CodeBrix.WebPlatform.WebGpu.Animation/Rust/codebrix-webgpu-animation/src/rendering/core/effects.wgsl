// The effect passes (blur, glow, drop shadow - C#'s EffectProcessor on the GPU), appended to
// common.wgsl. A group with effects is drawn into an offscreen multisampled target; then:
//   fs_resolve    averages its samples into a float texture (transparent outside the content),
//   fs_down       box-filters a texture into a coarser level (wide blurs run at 1/2, 1/4 ...),
//   fs_blur       one direction of a separable Gaussian (the planes' kernel, from a buffer),
//   fs_composite  puts down shadow, glow and content (C#'s order: content over glow over
//                 shadow) into the target below, at the group's alpha - with a layer's
//                 whole-frame operations (tone curve, gamma, vignette on the straight sRGB
//                 colour; a bloom added) and its weight in a transition (dissolve, wipe, fade
//                 through a colour) applied to the content first.
// Everything is LINEAR and PREMULTIPLIED (the renderer's convention); the passes read only the
// texels they know to be valid (zero elsewhere), as C#'s planes see transparency outside the
// region it processes. The parts below the "//@part" lines are separate shader modules (each
// with this shared head), so that their group-1 bindings never meet.
//
// A layer's SURFACE FINISH (Not in C#; effects/surface_finish.rs) is lit in fs_composite from
// the layer's height record (its stroke sets' paint, drawn by strokes.wgsl fs_stroke_height just
// before) and the canvas weave computed here; the CPU mirrors of weave_noise, weave_height and
// finish_shade are in effects/surface_finish.rs (keep them equal). The "picture" part is a
// picture target's final pass: the layer's samples resolved and stored as premultiplied
// sRGB-encoded bytes.
//
// A layer's WATER (Not in C#; src/water/) is drawn in fs_composite too: a REFRACTION reads the
// content where the water's slope bends the view and shades it (glints, a caustic light from
// noise.wgsl's caustics_once, which the composite module carries); a REFLECTION puts the content
// mirrored about a waterline under it, broken up by the water and fading. The water is a ripple
// field evaluated per pixel from the frame's `ripples` records (ripple_ring / ripple_water: the
// transcription of src/water/ripple_math.rs `evaluate`, the CPU reference; keep them equal) or a
// height picture (4 bilinear taps around the point and the centre). Every texture read in the
// water code is textureSampleLevel (no implicit derivatives), so it may sit in branches and loops.

struct FxPass {
    v0: vec4<f32>,
    v1: vec4<f32>,
    v2: vec4<f32>,
    v3: vec4<f32>,
    v4: vec4<f32>,
    u0: vec4<u32>,
    v5: vec4<f32>,
    v6: vec4<f32>,
    v7: vec4<f32>,
    v8: vec4<f32>,
    v9: vec4<f32>,
    u1: vec4<u32>,
    v10: vec4<f32>,  // finish: the light (towards it), relief
    v11: vec4<f32>,  // finish: gloss, weave depth, weave spacing (layer units), irregularity
    v12: vec4<f32>,  // finish: device -> layer a b c d
    v13: vec4<f32>,  // finish: e f, weave angle (radians), device pixels per layer unit
    u2: vec4<u32>,   // finish flags (1 on, 2 weave), weave seed
    v14: vec4<f32>,  // water: device -> water a b c d
    v15: vec4<f32>,  // water: e f, caustics z, caustics frequency
    v16: vec4<f32>,  // water: water -> device linear a b c d
    v17: vec4<f32>,  // refraction strength, shading, glint amount, glint sharpness
    v18: vec4<f32>,  // light x y z (towards it), caustic strength
    v19: vec4<f32>,  // glint colour (linear rgb), caustic focus
    v20: vec4<f32>,  // caustic colour (linear rgb), the refraction's largest offset (device px)
    v21: vec4<f32>,  // slot A's place x0 y0 x1 y1
    v22: vec4<f32>,  // slot A picture scale, bias; slot B scale, bias
    v23: vec4<f32>,  // waterline: device point x y, unit normal into the water x y
    v24: vec4<f32>,  // reflection opacity, fade (device px), streak (device px), water strength
    v25: vec4<f32>,  // reflection tint (linear rgb), amount
    v26: vec4<f32>,  // slot B's place x0 y0 x1 y1
    v27: vec4<f32>,  // the mirror's gap, the reflection water's largest offset (device px)
    u3: vec4<u32>,   // slot A ripple header: record offset, drops, swell waves, flags (1 walls)
    u4: vec4<u32>,   // water flags (1 refraction, 2 glints, 4 caustics, 8 reflection, 16 B = A), kind A, kind B, caustic seed
    u5: vec4<u32>,   // slot B ripple header
};

@group(2) @binding(0) var<storage, read> fx: array<FxPass>;
@group(2) @binding(1) var<storage, read> fx_kernel: array<f32>;

// Down and blur passes draw into a texture of a coarser level: device pixels / level, mapped
// onto that texture (v4.xy is its size in texels). The effect textures cover the frame's target
// window (common.wgsl): a level's texel j is the canvas's texel j + origin / level (the origin is
// a multiple of every level, target_window.rs), so the passes' valid texels (canvas texels) are
// checked at the texel plus that shift.
@vertex
fn vs_fx(@location(0) p: vec2<f32>, @location(1) pass_index: u32) -> VsOut {
    let fp = fx[pass_index];
    let q = (p - view.size.zw) / fp.v1.y;
    var out: VsOut;
    out.position = vec4<f32>(q.x / fp.v4.x * 2.0 - 1.0, 1.0 - q.y / fp.v4.y * 2.0, 0.0, 1.0);
    out.paint = pass_index;
    return out;
}

fn inside(c: vec2<f32>, r: vec4<f32>) -> bool {
    return c.x >= r.x && c.y >= r.y && c.x < r.z && c.y < r.w;
}

//@part resolve
@group(1) @binding(0) var fx_ms: texture_multisampled_2d<f32>;

@fragment
fn fs_resolve(in: VsOut) -> @location(0) vec4<f32> {
    let fp = fx[in.paint];
    let xy = in.position.xy;
    if (!inside(canvas_xy(xy), fp.v0)) {
        return vec4<f32>(0.0);
    }
    let c = vec2<i32>(xy);
    return (textureLoad(fx_ms, c, 0) + textureLoad(fx_ms, c, 1) + textureLoad(fx_ms, c, 2) + textureLoad(fx_ms, c, 3)) * 0.25;
}

//@part filter
@group(1) @binding(0) var fx_src: texture_2d<f32>;

// The texel c of the source (at `level`) where it is valid, zero elsewhere.
fn load_valid(c: vec2<i32>, valid: vec4<f32>, level: f32) -> vec4<f32> {
    if (!inside(vec2<f32>(c) + view.size.zw / level + vec2<f32>(0.5), valid)) {
        return vec4<f32>(0.0);
    }
    return textureLoad(fx_src, c, 0);
}

@fragment
fn fs_down(in: VsOut) -> @location(0) vec4<f32> {
    let fp = fx[in.paint];
    let ratio = fp.v1.y / fp.v1.x;
    let texel = vec2<i32>(in.position.xy);
    if (ratio <= 1.0) {
        // A finer target (rare): the source texel under this one.
        return load_valid(vec2<i32>(in.position.xy * ratio), fp.v0, fp.v1.x);
    }
    // The exact box of source texels under this texel (zero outside the valid ones).
    let n = i32(ratio + 0.5);
    let origin = texel * n;
    var sum = vec4<f32>(0.0);
    for (var y = 0; y < n; y++) {
        for (var x = 0; x < n; x++) {
            sum += load_valid(origin + vec2<i32>(x, y), fp.v0, fp.v1.x);
        }
    }
    return sum / f32(n * n);
}

@fragment
fn fs_blur(in: VsOut) -> @location(0) vec4<f32> {
    let fp = fx[in.paint];
    let texel = vec2<i32>(in.position.xy);
    let dir = vec2<i32>(fp.v1.zw);
    let offset = fp.u0.x;
    let radius = i32(fp.u0.y);
    // C#'s order of summation: from -radius to +radius.
    var sum = vec4<f32>(0.0);
    for (var i = -radius; i <= radius; i++) {
        sum += fx_kernel[offset + u32(i + radius)] * load_valid(texel + dir * i, fp.v0, fp.v1.x);
    }
    return sum;
}

//@part picture
@group(1) @binding(0) var picture_ms: texture_multisampled_2d<f32>;

// A picture target's final pass: the mean of the layer's 4 samples (linear premultiplied) as
// premultiplied sRGB-encoded values (the picture's Rgba8Unorm texels). The picture pixel (drawn
// by vs_final) reads the target at the pixel less the window's origin.
@fragment
fn fs_picture(in: VsOut) -> @location(0) vec4<f32> {
    let c = vec2<i32>(in.position.xy - view.size.zw);
    let linear = (textureLoad(picture_ms, c, 0) + textureLoad(picture_ms, c, 1) + textureLoad(picture_ms, c, 2) + textureLoad(picture_ms, c, 3)) * 0.25;
    return premul_linear_to_srgb(linear);
}

//@part composite
@group(1) @binding(0) var fx_content: texture_2d<f32>;
@group(1) @binding(1) var fx_glow: texture_2d<f32>;
@group(1) @binding(2) var fx_shadow: texture_2d<f32>;
@group(1) @binding(3) var fx_linear: sampler;
@group(1) @binding(4) var fx_height: texture_2d<f32>;
@group(1) @binding(5) var fx_water_a: texture_2d<f32>;
@group(1) @binding(6) var fx_water_b: texture_2d<f32>;
@group(2) @binding(2) var<storage, read> ripples: array<vec4<f32>>;

// ---- The water (src/water/ripple_math.rs evaluate is the CPU reference of the ripples) ----

struct Water {
    height: f32,
    slope: vec2<f32>,
    focus: f32,
};

// One source of rings (a drop or one of its mirror images) at the water point q.
fn ripple_ring(w: ptr<function, Water>, q: vec2<f32>, cx: f32, cy: f32, a: vec4<f32>, b: vec4<f32>) {
    let front = a.z;
    let amplitude = a.w;
    let lambda = b.x;
    let width = b.y;
    let dx = q.x - cx;
    let dy = q.y - cy;
    let r = sqrt(dx * dx + dy * dy);
    let u = r - front;
    if (u > 1.0 * lambda || u < -3.0 * width) {
        return;
    }
    let k = 6.28318530717958647692 / lambda;
    let c = select(0.25 * lambda, width, u < 0.0);
    let qq = u / c;
    let e = exp(-(qq * qq));
    let e1 = -2.0 * qq / c * e;
    let e2 = (4.0 * qq * qq - 2.0) / (c * c) * e;
    let g = 1.0 + r / lambda;
    let s = 1.0 / sqrt(g);
    let s1 = -0.5 / lambda * s / g;
    let s2 = 0.75 / (lambda * lambda) * s / (g * g);
    let sn = sin(k * u);
    let cs = cos(k * u);
    let p1 = k * cs;
    let p2 = -k * k * sn;
    let h = amplitude * s * e * sn;
    let h1 = amplitude * (s1 * e * sn + s * e1 * sn + s * e * p1);
    let h2 = amplitude * (s2 * e * sn + s * e2 * sn + s * e * p2 + 2.0 * (s1 * e1 * sn + s1 * e * p1 + s * e1 * p1));
    (*w).height += h;
    if (r > 0.0) {
        (*w).slope += vec2<f32>(h1 * dx / r, h1 * dy / r);
    }
    (*w).focus += h2 + h1 / max(r, 0.25 * lambda);
}

// A ripple field at the water point q: its drops (with their 8 mirror images when the place's
// walls reflect), then its swell waves.
fn ripple_water(words: vec4<u32>, place: vec4<f32>, q: vec2<f32>) -> Water {
    var w = Water(0.0, vec2<f32>(0.0), 0.0);
    let offset = words.x;
    let drops = words.y;
    let waves = words.z;
    let walls = (words.w & 1u) != 0u;
    for (var i = 0u; i < drops; i++) {
        let a = ripples[offset + i * 2u];
        let b = ripples[offset + i * 2u + 1u];
        if (walls) {
            let xs = vec3<f32>(2.0 * place.x - a.x, a.x, 2.0 * place.z - a.x);
            let ys = vec3<f32>(2.0 * place.y - a.y, a.y, 2.0 * place.w - a.y);
            for (var j = 0; j < 3; j++) {
                for (var m = 0; m < 3; m++) {
                    ripple_ring(&w, q, xs[m], ys[j], a, b);
                }
            }
        } else {
            ripple_ring(&w, q, a.x, a.y, a, b);
        }
    }
    let base = offset + drops * 2u;
    for (var j = 0u; j < waves; j++) {
        let wave = ripples[base + j];
        let phase = wave.x * q.x + wave.y * q.y + wave.w;
        let sn = sin(phase);
        let cs = cos(phase);
        let h = wave.z * sn;
        w.height += h;
        w.slope += vec2<f32>(wave.z * cs * wave.x, wave.z * cs * wave.y);
        w.focus -= (wave.x * wave.x + wave.y * wave.y) * h;
    }
    return w;
}

// A height picture's height at a water point: (red - bias) x scale, the picture stretched over
// place (bilinear, clamped to its edge).
fn picture_height(t: texture_2d<f32>, place: vec4<f32>, scale: f32, bias: f32, q: vec2<f32>) -> f32 {
    let uv = (q - place.xy) / (place.zw - place.xy);
    return (textureSampleLevel(t, fx_linear, uv, 0.0).r - bias) * scale;
}

// A height picture's water at q: the slope by central differences one texel either side, the
// Laplacian from the same taps and the centre.
fn picture_water(t: texture_2d<f32>, place: vec4<f32>, scale: f32, bias: f32, q: vec2<f32>) -> Water {
    let texel = (place.zw - place.xy) / vec2<f32>(textureDimensions(t));
    let h0 = picture_height(t, place, scale, bias, q);
    let hl = picture_height(t, place, scale, bias, q - vec2<f32>(texel.x, 0.0));
    let hr = picture_height(t, place, scale, bias, q + vec2<f32>(texel.x, 0.0));
    let hu = picture_height(t, place, scale, bias, q - vec2<f32>(0.0, texel.y));
    let hd = picture_height(t, place, scale, bias, q + vec2<f32>(0.0, texel.y));
    let slope = vec2<f32>((hr - hl) / (2.0 * texel.x), (hd - hu) / (2.0 * texel.y));
    let focus = (hl + hr - 2.0 * h0) / (texel.x * texel.x) + (hu + hd - 2.0 * h0) / (texel.y * texel.y);
    return Water(h0, slope, focus);
}

// A device pixel's point in the water's coordinates.
fn water_point(fp: FxPass, p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(fp.v14.x * p.x + fp.v14.z * p.y + fp.v15.x, fp.v14.y * p.x + fp.v14.w * p.y + fp.v15.y);
}

// The device offset of a water slope moved by `strength` (water units per unit of slope), at
// most `reach` device pixels long.
fn water_offset(fp: FxPass, slope: vec2<f32>, strength: f32, reach: f32) -> vec2<f32> {
    let d = strength * slope;
    let v = vec2<f32>(fp.v16.x * d.x + fp.v16.z * d.y, fp.v16.y * d.x + fp.v16.w * d.y);
    let len = length(v);
    if (len > reach) {
        return v * (reach / len);
    }
    return v;
}

// A device position kept inside the group's region (the content texture's valid texels).
fn in_region(fp: FxPass, p: vec2<f32>) -> vec2<f32> {
    return clamp(p, fp.v0.xy + vec2<f32>(0.5), fp.v0.zw - vec2<f32>(0.5));
}

// The refraction's light on the content (linear premultiplied): shading by the slope, glints,
// the caustic light. Each part is skipped when it is off, so a refraction with no shading, no
// glints and no caustics leaves the colour exactly as it was.
fn water_light(c: vec4<f32>, w: Water, q: vec2<f32>, fp: FxPass) -> vec4<f32> {
    if (c.a <= 0.0) {
        return c;
    }
    let flags = fp.u4.x;
    let l = fp.v18.xyz;
    let n = normalize(vec3<f32>(-w.slope, 1.0));
    var rgb = c.rgb;
    var lit = false;
    if (fp.v17.y != 0.0) {
        rgb = rgb * max(1.0 + fp.v17.y * (dot(n, l) - l.z), 0.0);
        lit = true;
    }
    if ((flags & 4u) != 0u) {
        let v = caustics_once(fp.u4.w, q * fp.v15.w, fp.v15.z);
        let gather = max(1.0 - fp.v19.w * fp.v17.x * w.focus, 0.0);
        rgb = rgb + rgb * fp.v20.rgb * (fp.v18.w * 0.5 * (v + 1.0) * gather);
        lit = true;
    }
    if ((flags & 2u) != 0u) {
        let glint = fp.v17.z * pow(max(dot(reflect(-l, n), vec3<f32>(0.0, 0.0, 1.0)), 1e-6), fp.v17.w);
        rgb = rgb + fp.v19.rgb * (glint * c.a);
        lit = true;
    }
    if (lit) {
        rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(c.a));
    }
    return vec4<f32>(rgb, c.a);
}

// The reflection under the content: below the waterline, the content mirrored about the mirror
// line, moved by the water, smeared along the mirrored vertical (6 taps), faded and tinted.
fn with_reflection(c: vec4<f32>, p: vec2<f32>, w: Water, fp: FxPass) -> vec4<f32> {
    let n = fp.v23.zw;
    let depth = dot(p - fp.v23.xy, n);
    if (depth <= 0.0 || c.a >= 1.0) {
        return c;
    }
    let t = clamp(depth / fp.v24.y, 0.0, 1.0);
    let weight = fp.v24.x * (1.0 - t * t * (3.0 - 2.0 * t));
    if (weight <= 0.0) {
        return c;
    }
    let m = fp.v23.xy + n * fp.v27.x;
    var mp = p - 2.0 * dot(p - m, n) * n;
    if ((fp.u4.z != 0u) || ((fp.u4.x & 16u) != 0u)) {
        mp = mp + water_offset(fp, w.slope, fp.v24.w, fp.v27.y);
    }
    var sum = vec4<f32>(0.0);
    for (var i = 0; i < 6; i++) {
        let at = mp + n * (fp.v24.z * (f32(i) / 5.0 - 0.5));
        sum += sample_level(fx_content, in_region(fp, at), fp.v1.x);
    }
    var r = sum / 6.0 * weight;
    r = vec4<f32>(r.rgb * mix(vec3<f32>(1.0), fp.v25.rgb, fp.v25.a), r.a);
    return c + r * (1.0 - c.a);
}

// ---- The surface finish (effects/surface_finish.rs mirrors these) ----

fn weave_hash(seed: u32, x: i32, y: i32) -> u32 {
    var h = seed ^ 0x9E3779B9u;
    h = (h ^ u32(x)) * 0x85EBCA6Bu;
    h = (h ^ (h >> 13u) ^ u32(y)) * 0xC2B2AE35u;
    h = (h ^ (h >> 16u)) * 0x27D4EB2Fu;
    h = h ^ (h >> 15u);
    h = h * 0x2C1B3C6Du;
    return h ^ (h >> 12u);
}

// Value noise in -1..1 over the integer lattice, smoothstep between the corners.
fn weave_noise(seed: u32, p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = p - i;
    let ix = i32(i.x);
    let iy = i32(i.y);
    let c00 = f32(weave_hash(seed, ix, iy)) / 4294967295.0 * 2.0 - 1.0;
    let c10 = f32(weave_hash(seed, ix + 1, iy)) / 4294967295.0 * 2.0 - 1.0;
    let c01 = f32(weave_hash(seed, ix, iy + 1)) / 4294967295.0 * 2.0 - 1.0;
    let c11 = f32(weave_hash(seed, ix + 1, iy + 1)) / 4294967295.0 * 2.0 - 1.0;
    let s = f * f * (vec2<f32>(3.0) - 2.0 * f);
    let top = c00 + (c10 - c00) * s.x;
    let bottom = c01 + (c11 - c01) * s.x;
    return top + (bottom - top) * s.y;
}

// The plain weave's height at a point in layer units (0..depth, its thickness varied).
fn weave_height(fp: FxPass, q: vec2<f32>) -> f32 {
    let c = cos(fp.v13.z);
    let s = sin(fp.v13.z);
    let x0 = (c * q.x + s * q.y) / fp.v11.z;
    let y0 = (-s * q.x + c * q.y) / fp.v11.z;
    let irr = fp.v11.w;
    let seed = fp.u2.y;
    let x = x0 + irr * 0.3 * weave_noise(seed, vec2<f32>(x0, y0) * 0.25);
    let y = y0 + irr * 0.3 * weave_noise(seed + 1u, vec2<f32>(x0, y0) * 0.25);
    let a = sin(3.14159265 * x);
    let b = sin(3.14159265 * y);
    let warp = 0.5 * abs(a) + 0.5 * a * b;
    let weft = 0.5 * abs(b) - 0.5 * a * b;
    let thickness = 1.0 + irr * 0.25 * weave_noise(seed + 2u, vec2<f32>(x, y) * 0.5);
    return fp.v11.y * max(warp, weft) * thickness;
}

// The surface's whole height at a device position: the paint there plus the weave where the
// paint does not hide it (the weave faded out where its threads are under ~2 device pixels).
fn surface_height(fp: FxPass, device: vec2<f32>, paint: f32) -> f32 {
    var weave = 0.0;
    if ((fp.u2.x & 2u) != 0u) {
        let q = vec2<f32>(fp.v12.x * device.x + fp.v12.z * device.y + fp.v13.x, fp.v12.y * device.x + fp.v12.w * device.y + fp.v13.y);
        let fade = smoothstep(2.0, 4.0, fp.v11.z * fp.v13.w);
        weave = weave_height(fp, q) * fade;
    }
    return paint + weave * (1.0 - smoothstep(0.0, 0.6, paint));
}

// The shading factor and the highlight for a height gradient (height units per layer unit along
// screen x and y) with this much paint.
fn finish_shade(fp: FxPass, gradient: vec2<f32>, paint: f32) -> vec2<f32> {
    let l = fp.v10.xyz;
    let n = normalize(vec3<f32>(-gradient, 1.0));
    let shade = max(1.0 + fp.v10.w * (dot(n, l) - l.z), 0.0);
    let h = normalize(l + vec3<f32>(0.0, 0.0, 1.0));
    let highlight = max(fp.v11.x * (pow(max(dot(n, h), 0.0), 32.0) - pow(h.z, 32.0)), 0.0) * smoothstep(0.0, 0.3, paint);
    return vec2<f32>(shade, highlight);
}

// The finish on the layer's content (linear premultiplied): its straight colour shaded and
// highlighted. The height record (the target window's size) is read at the pixel and one pixel
// either side.
fn finish_colour(c: vec4<f32>, p: vec2<f32>, fp: FxPass) -> vec4<f32> {
    let size = vec2<i32>(textureDimensions(fx_height)) - vec2<i32>(1);
    let t = clamp(vec2<i32>(p - view.size.zw), vec2<i32>(0), size);
    let h0 = textureLoad(fx_height, t, 0).r;
    let hl = textureLoad(fx_height, clamp(t - vec2<i32>(1, 0), vec2<i32>(0), size), 0).r;
    let hr = textureLoad(fx_height, clamp(t + vec2<i32>(1, 0), vec2<i32>(0), size), 0).r;
    let hu = textureLoad(fx_height, clamp(t - vec2<i32>(0, 1), vec2<i32>(0), size), 0).r;
    let hd = textureLoad(fx_height, clamp(t + vec2<i32>(0, 1), vec2<i32>(0), size), 0).r;
    if ((fp.u2.x & 1u) == 0u || c.a <= 0.0) {
        return c;
    }
    let gx = (surface_height(fp, p + vec2<f32>(1.0, 0.0), hr) - surface_height(fp, p - vec2<f32>(1.0, 0.0), hl)) * 0.5;
    let gy = (surface_height(fp, p + vec2<f32>(0.0, 1.0), hd) - surface_height(fp, p - vec2<f32>(0.0, 1.0), hu)) * 0.5;
    // Per device pixel -> per layer unit.
    let shaded = finish_shade(fp, vec2<f32>(gx, gy) * fp.v13.w, h0);
    let straight = c.rgb / c.a;
    return vec4<f32>(clamp(straight * shaded.x + vec3<f32>(shaded.y), vec3<f32>(0.0), vec3<f32>(1.0)) * c.a, c.a);
}

// A texture of a level sampled at a device position (bilinear between its texels; the texture
// covers the target window).
fn sample_level(t: texture_2d<f32>, device: vec2<f32>, level: f32) -> vec4<f32> {
    let size = vec2<f32>(textureDimensions(t));
    return textureSampleLevel(t, fx_linear, (device - view.size.zw) / level / size, 0.0);
}

// The planes' colour operations on a linear premultiplied colour: unpremultiplied and encoded
// to sRGB (the values a plane image holds), the tone curve, the gamma and the vignette (at the
// pixel's centre), then back. Alpha is left alone.
fn frame_colour(c: vec4<f32>, p: vec2<f32>, fp: FxPass) -> vec4<f32> {
    let flags = fp.u1.x;
    if ((flags & 7u) == 0u || c.a <= 0.0) {
        return c;
    }
    var s = linear_to_srgb(c.rgb / c.a);
    if ((flags & 1u) != 0u) {
        s = s / (vec3<f32>(1.0) + fp.v5.x * s) * fp.v5.y;
    }
    if ((flags & 2u) != 0u) {
        s = select(s, pow(max(s, vec3<f32>(1e-30)), vec3<f32>(fp.v5.z)), s > vec3<f32>(0.0));
    }
    if ((flags & 4u) != 0u) {
        let d = (p - fp.v6.xy) / fp.v6.zw;
        s = s * clamp(1.0 - fp.v7.x * dot(d, d), fp.v7.y, 1.0);
    }
    return vec4<f32>(srgb_to_linear(max(s, vec3<f32>(0.0))) * c.a, c.a);
}

// The incoming layer of a transition, drawn over the outgoing one: its weight at this pixel
// (dissolve: the fraction; wipe: the planes' hard or soft edge from the side it starts at) - or,
// for a fade through a colour, the colour in front of the outgoing frame below 0.5 and mixed into
// the incoming one above it. Exact for opaque frames, as the planes' transitions are of frames.
fn transition_colour(c: vec4<f32>, p: vec2<f32>, fp: FxPass) -> vec4<f32> {
    let kind = (fp.u1.x >> 8u) & 15u;
    if (kind == 0u) {
        return c;
    }
    let f = clamp(fp.v7.z, 0.0, 1.0);
    let direction = (fp.u1.x >> 12u) & 3u;
    if (kind == 1u) {
        return c * f;
    }
    if (kind == 2u) {
        let horizontal = direction == 0u || direction == 1u;
        let extent = select(fp.v9.w - fp.v9.y, fp.v9.z - fp.v9.x, horizontal);
        // 0 Left, 1 Right, 2 Up, 3 Down: Right and Down start at the left and the top.
        var centre = 0.0;
        if (direction == 1u) { centre = p.x - fp.v9.x; }
        if (direction == 0u) { centre = fp.v9.z - p.x; }
        if (direction == 3u) { centre = p.y - fp.v9.y; }
        if (direction == 2u) { centre = fp.v9.w - p.y; }
        let s = fp.v7.w;
        var w = select(0.0, 1.0, centre < f * extent);
        if (s > 0.0) {
            w = clamp((f * (extent + s) - centre) / s, 0.0, 1.0);
        }
        return c * w;
    }
    if (kind == 5u) {
        if (f < 0.5) {
            return fp.v8 * (2.0 * f);
        }
        let w = 2.0 * f - 1.0;
        return c * w + fp.v8 * (1.0 - w);
    }
    return c;
}

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let fp = fx[in.paint];
    let p = canvas_xy(in.position.xy);
    let flags = u32(fp.v1.w);
    // The water under the pixel (slot A: the refraction's; slot B: the reflection's own).
    let wflags = fp.u4.x;
    let wq = water_point(fp, p);
    var wa = Water(0.0, vec2<f32>(0.0), 0.0);
    if (fp.u4.y == 1u) {
        wa = ripple_water(fp.u3, fp.v21, wq);
    } else if (fp.u4.y == 2u) {
        wa = picture_water(fx_water_a, fp.v21, fp.v22.x, fp.v22.y, wq);
    }
    var wb = wa;
    if (fp.u4.z == 1u) {
        wb = ripple_water(fp.u5, fp.v26, wq);
    } else if (fp.u4.z == 2u) {
        wb = picture_water(fx_water_b, fp.v26, fp.v22.z, fp.v22.w, wq);
    }
    // A refraction reads the content where the water bends the view.
    var at = p;
    if ((wflags & 1u) != 0u) {
        at = in_region(fp, p + water_offset(fp, wa.slope, fp.v17.x, fp.v20.w));
    }
    var content = finish_colour(sample_level(fx_content, at, fp.v1.x), p, fp);
    if ((wflags & 1u) != 0u) {
        content = water_light(content, wa, wq, fp);
    }
    if ((wflags & 8u) != 0u) {
        content = with_reflection(content, p, wb, fp);
    }
    content = frame_colour(content, p, fp);
    if ((fp.u1.x & 8u) != 0u) {
        // A bloom: the blurred content (in the glow's texture), times the gain, added.
        let light = sample_level(fx_glow, p, fp.v1.y) * fp.v5.w;
        content = vec4<f32>(content.rgb + light.rgb, clamp(content.a + light.a * (1.0 - content.a), 0.0, 1.0));
    }
    let q = p - fp.v4.xy;
    let shadow_cov = sample_level(fx_shadow, q, fp.v1.z).a;
    let glow_cov = sample_level(fx_glow, p, fp.v1.y).a;
    var result = vec4<f32>(0.0);
    if ((flags & 2u) != 0u && inside(q, fp.v0)) {
        result = fp.v3 * shadow_cov;
    }
    if ((flags & 1u) != 0u) {
        let glow = fp.v2 * min(glow_cov * fp.v4.z, 1.0);
        result = glow + result * (1.0 - glow.a);
    }
    result = content + result * (1.0 - content.a);
    return transition_colour(result, p, fp) * fp.v4.w;
}
