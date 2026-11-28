// ---- strokes.wgsl: stroke sets (BrushStrokesShape), ONE instanced draw per set. Appended after
// common.wgsl and noise.wgsl. Not in C#.
//
// Each instance is one record of the set's storage buffer (uploaded once per set; layout in
// strokes/stroke_gpu.rs). The vertex shader works out the stroke at the draw's time - how much is
// painted, the boil's jitter, the flow's turn, its two anchors blended - and emits an 8-vertex
// HULL (a triangle strip of four stations along the painted part, each as wide as the stroke
// around it, grown by one device pixel); a stroke not yet started is a degenerate strip. The
// fragment shader draws the ANALYTIC lozenge of BrushStrokeShape's formula (centre line
// bend x length x (4 s^2 - 1), half width w/2 x sqrt(max(0, 1 - |2 s|^profile))) with its edge
// smoothed over a device pixel, cut at the brush's head (straight, or faded by the head
// softness), times the brush tip's coverage, colour linear premultiplied. The CPU mirrors of the
// hull and the coverage are in strokes/stroke_gpu.rs (keep them equal).
//
// PICKED COLOUR (StrokeColor::Picked): the vertex shader samples the colour source K times along
// the stroke's centre line (in the set's coordinates, after boil, flow and anchors), averages in
// linear light weighted by alpha, applies the colour rules in Oklab (palette, broken colour) and
// mixes with the record's own colour by the own weight (the draw's times the stroke's own: word
// 13's top byte holds 255 minus its 8-bit weight, so 0 there - a set without own weights - is
// weight 1 exactly). The CPU mirrors of this arithmetic are in strokes/stroke_color_gpu.rs (keep
// them equal; they are tested against painting/color_rules.rs and painting/color_mix.rs).
//
// PROGRESS is exactly 1 from a stroke's end on (a comparison, not a division): the CPU knows
// exactly which strokes the GPU draws whole, which the checkpoint cache relies on (a checkpoint
// holds only whole strokes; strokes/stroke_checkpoints.rs).
//
// A STROKE-LEVEL TRANSITION (draw flag STROKE_MORPH; strokes/brush_stroke_morph.rs): `records`
// holds each pair's first stroke and `targets` its second (slot k of both is one pair; in a
// target word 11 is the pair's delay under the spread and word 13's flags what it does: 0
// travels, 1 paints off, 2 paints on). The pair's own fraction is clamp((fraction - spread x
// delay) / (1 - spread), 0, 1); at 0 it is exactly its first stroke, at 1 exactly its second;
// between, morph_record interpolates (centre, sizes, the angle the shorter way using the stroke's
// 180-degree symmetry with the bend flipped, the colour in Oklab; tip, seed and anchors switch at
// halfway; the own weight linearly, a stroke painting off or on keeping its own). A plain set
// binds its records as the targets too (never read).
//
// HEIGHT (a layer's SurfaceFinish): fs_stroke_height draws the same coverage into the layer's
// R16Float height record, additively: tip height x coverage x load x opacity x the layer's
// impasto.

struct StrokeRecord {
    a: vec4<f32>,  // centre x, y, angle (radians), length
    b: vec4<f32>,  // width, bend, profile, load
    c: vec4<f32>,  // start, end, anchor blend, 0
    d: vec4<u32>,  // rgba8, tip | flags << 16 | (255 - own weight) << 24, anchor0 | anchor1 << 16, seed
};

struct StrokeDraw {
    m: vec4<f32>,          // element -> device: a b c d
    e: vec4<f32>,          // e f, the draw's time, the opacity
    p: vec4<f32>,          // head softness, impasto (height scale; 0 off a finished layer)
    flags: vec4<u32>,      // flags, anchor count, boil redraw number (i32 bits), boil seed
    boil: vec4<f32>,       // offset, turn (radians), size
    flow: vec4<f32>,       // frequency, speed, turn (radians)
    flow_seed: vec4<u32>,  // seed
    place: vec4<f32>,      // the source in set units: x, y, 1 / width, 1 / height
    pick: vec4<u32>,       // colour flags (PICK_*), texels (0 sRGB straight, 1 sRGB premultiplied, 2 linear premultiplied), taps, pigments
    rules: vec4<f32>,      // own weight, palette snap, broken hue (radians), broken chroma
    rules2: vec4<f32>,     // broken lightness
    rules_seed: vec4<u32>, // broken colour's seed
    pigments: array<vec4<f32>, 32>,  // the palette in Oklab (L, a, b, 0)
    anchors: array<vec4<f32>, 512>,  // per anchor: (a b c d), (e f 0 0)
};

const STROKE_TIPS: u32 = 1u;
const STROKE_MIRROR: u32 = 2u;
const STROKE_BOIL: u32 = 4u;
const STROKE_FLOW: u32 = 8u;
const STROKE_MORPH: u32 = 16u;
const PICK_ON: u32 = 1u;
const PICK_PALETTE: u32 = 2u;
const PICK_MIX_OF_TWO: u32 = 4u;
const PICK_BROKEN: u32 = 8u;

@group(1) @binding(0) var tips: texture_2d_array<f32>;
@group(1) @binding(1) var tip_sampler: sampler;
@group(1) @binding(2) var source_tex: texture_2d<f32>;
@group(1) @binding(3) var source_sampler: sampler;
@group(2) @binding(0) var<storage, read> records: array<StrokeRecord>;
@group(2) @binding(1) var<storage, read> targets: array<StrokeRecord>;
@group(3) @binding(0) var<uniform> draw: StrokeDraw;

struct StrokeOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,                       // s (-0.5 .. 0.5 along), across (set units)
    @location(1) @interpolate(flat) color: vec4<f32>,   // linear premultiplied, alphas applied
    @location(2) @interpolate(flat) shape: vec4<f32>,   // half width, bow, profile, the cut's s
    @location(3) @interpolate(flat) look: vec4<f32>,    // head softness, tip layer (-1: none), mirrored (+ 2: reversed along), cut on
    @location(4) @interpolate(flat) height: f32,        // the height pass's scale: opacity x load x thin x impasto
};

// A hash as a value in -1..1 (stroke_boil.rs hash_unit).
fn hash_unit(h: u32) -> f32 {
    return f32(h) / 4294967295.0 * 2.0 - 1.0;
}

// One half of an anchor's transform (k = 0: a b c d, k = 1: e f); the identity past the list.
fn anchor_part(i: u32, k: u32) -> vec4<f32> {
    if (i >= draw.flags.y) {
        return select(vec4<f32>(1.0, 0.0, 0.0, 1.0), vec4<f32>(0.0), k == 1u);
    }
    return draw.anchors[2u * i + k];
}

fn stroke_centre(bow: f32, s: f32) -> f32 {
    return bow * (4.0 * s * s - 1.0);
}

fn stroke_half(half: f32, profile: f32, s: f32) -> f32 {
    return half * sqrt(max(0.0, 1.0 - pow(abs(2.0 * s), profile)));
}

// The lowest and highest across-axis edges over a..b (stroke_gpu.rs extent).
fn stroke_extent(bow: f32, half: f32, profile: f32, a: f32, b: f32) -> vec2<f32> {
    let ca = stroke_centre(bow, a);
    let cb = stroke_centre(bow, b);
    var lo = min(ca, cb);
    var hi = max(ca, cb);
    if (a <= 0.0 && b >= 0.0) {
        lo = min(lo, -bow);
        hi = max(hi, -bow);
    }
    let h = stroke_half(half, profile, clamp(0.0, a, b));
    return vec2<f32>(lo - h, hi + h);
}

fn apply2(m: vec4<f32>, v: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(m.x * v.x + m.z * v.y, m.y * v.x + m.w * v.y);
}

// ---- Oklab (the crate's own single-precision form: painting/color_mix.rs) ----

fn mul3(r0: vec3<f32>, r1: vec3<f32>, r2: vec3<f32>, v: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(r0.x * v.x + r0.y * v.y + r0.z * v.z, r1.x * v.x + r1.y * v.y + r1.z * v.z, r2.x * v.x + r2.y * v.y + r2.z * v.z);
}

// The sign-preserving cube root (pow never sees 0: some GPUs give NaN for pow(0, y)).
fn cbrt3(v: vec3<f32>) -> vec3<f32> {
    return sign(v) * pow(max(abs(v), vec3<f32>(1e-30)), vec3<f32>(1.0 / 3.0));
}

// Linear light (sRGB primaries) to Oklab [L, a, b].
fn linear_to_oklab(c: vec3<f32>) -> vec3<f32> {
    let lms = cbrt3(mul3(vec3<f32>(0.41222146, 0.53633255, 0.051445995), vec3<f32>(0.2119035, 0.6806995, 0.10739696), vec3<f32>(0.08830246, 0.28171885, 0.6299787), c));
    return mul3(vec3<f32>(0.21045426, 0.7936178, -0.004072047), vec3<f32>(1.9779985, -2.4285922, 0.4505937), vec3<f32>(0.025904037, 0.78277177, -0.80867577), lms);
}

// Oklab to linear light (not clamped).
fn oklab_to_linear(lab: vec3<f32>) -> vec3<f32> {
    let l = mul3(vec3<f32>(1.0, 0.39633778, 0.21580376), vec3<f32>(1.0, -0.105561346, -0.06385417), vec3<f32>(1.0, -0.08948418, -1.2914855), lab);
    let lms = l * l * l;
    return mul3(vec3<f32>(4.0767417, -3.3077116, 0.23096994), vec3<f32>(-1.268438, 2.6097574, -0.34131938), vec3<f32>(-0.0041960863, -0.7034186, 1.7076147), lms);
}

fn dist2(a: vec3<f32>, b: vec3<f32>) -> f32 {
    let d = a - b;
    return dot(d, d);
}

// The point of the palette a colour is pulled onto (painting/color_rules.rs palette_target):
// the nearest pigment, or with MixOfTwo the nearest point on any segment between two pigments;
// ties go to the earlier pigment (pair).
fn palette_target(lab: vec3<f32>) -> vec3<f32> {
    let n = min(draw.pick.w, 32u);
    var best = draw.pigments[0].xyz;
    var best_d = dist2(lab, best);
    for (var i = 1u; i < n; i++) {
        let p = draw.pigments[i].xyz;
        let d = dist2(lab, p);
        if (d < best_d) {
            best = p;
            best_d = d;
        }
    }
    if ((draw.pick.x & PICK_MIX_OF_TWO) != 0u) {
        for (var i = 0u; i < n; i++) {
            for (var j = i + 1u; j < n; j++) {
                let a = draw.pigments[i].xyz;
                let ab = draw.pigments[j].xyz - a;
                let len2 = dot(ab, ab);
                var q = a;
                if (len2 > 0.0) {
                    q = a + ab * clamp(dot(lab - a, ab) / len2, 0.0, 1.0);
                }
                let d = dist2(lab, q);
                if (d < best_d) {
                    best = q;
                    best_d = d;
                }
            }
        }
    }
    return best;
}

// The colour rules on a straight linear colour (ColorRules::apply_float): the palette (lerp in
// Oklab by the snap), broken colour (hue turned, chroma scaled, lightness moved by three hashes of
// the rules' seed and the stroke's seed), clamped to 0..1.
fn colour_rules(c: vec3<f32>, stroke_seed: u32) -> vec3<f32> {
    var lab = linear_to_oklab(c);
    if ((draw.pick.x & PICK_PALETTE) != 0u && draw.pick.w > 0u) {
        lab = mix(lab, palette_target(lab), draw.rules.y);
    }
    if ((draw.pick.x & PICK_BROKEN) != 0u) {
        let seed = i32(stroke_seed);
        let u_hue = hash_unit(noise_hash(draw.rules_seed.x, seed, 0, 0));
        let u_chroma = hash_unit(noise_hash(draw.rules_seed.x, seed, 1, 0));
        let u_light = hash_unit(noise_hash(draw.rules_seed.x, seed, 2, 0));
        let chroma = length(lab.yz) * max(1.0 + draw.rules.w * u_chroma, 0.0);
        let hue = atan2(lab.z, lab.y) + draw.rules.z * u_hue;
        lab = vec3<f32>(clamp(lab.x + draw.rules2.x * u_light, 0.0, 1.0), chroma * cos(hue), chroma * sin(hue));
    }
    return clamp(oklab_to_linear(lab), vec3<f32>(0.0), vec3<f32>(1.0));
}

// One texel of the source as linear premultiplied colour.
fn source_texel(t: vec4<f32>) -> vec4<f32> {
    let c = clamp(t, vec4<f32>(0.0), vec4<f32>(1.0));
    let kind = draw.pick.y;
    if (kind == 2u) {
        return c;
    }
    if (kind == 1u) {
        return premul_srgb_to_linear(c);
    }
    return vec4<f32>(srgb_to_linear(c.rgb) * c.a, c.a);
}

// A stroke's own weight from its record's word 13 (strokes/stroke_gpu.rs own_weight_of_word): the
// top byte is 255 minus the 8-bit weight; 255 is exactly 1 (and 0 exactly 0).
fn stroke_own_weight(word: u32) -> f32 {
    let q = 255u - (word >> 24u);
    return select(f32(q) / 255.0, 1.0, q == 255u);
}

// The stroke's picked colour (straight linear rgb, and the source's mean alpha): K taps along
// the centre line q(s) = m0 (centre + dir len s + nrm bow (4 s^2 - 1)) + m1 at s from -0.4 to
// 0.4, through the place onto the source, averaged premultiplied; then the rules, then the mix
// with the own colour in Oklab by the draw's own weight times the stroke's (`own_w`).
fn picked_colour(own: vec3<f32>, center: vec2<f32>, dir: vec2<f32>, nrm: vec2<f32>, len: f32, bow: f32, m0: vec4<f32>, m1: vec4<f32>, stroke_seed: u32, own_w: f32) -> vec4<f32> {
    let taps = clamp(draw.pick.z, 1u, 5u);
    var sum = vec4<f32>(0.0);
    for (var k = 0u; k < taps; k++) {
        var s = 0.0;
        if (taps > 1u) {
            s = -0.4 + 0.8 * f32(k) / f32(taps - 1u);
        }
        let q = apply2(m0, center + dir * (len * s) + nrm * stroke_centre(bow, s)) + m1.xy;
        let uv = (q - draw.place.xy) * draw.place.zw;
        sum += source_texel(textureSampleLevel(source_tex, source_sampler, uv, 0.0));
    }
    let mean = sum / f32(taps);
    var rgb = own;
    if (mean.a > 1e-6) {
        rgb = mean.rgb / mean.a;
    }
    if ((draw.pick.x & (PICK_PALETTE | PICK_BROKEN)) != 0u) {
        rgb = colour_rules(rgb, stroke_seed);
    }
    let w = draw.rules.x * own_w;
    if (w > 0.0) {
        rgb = clamp(oklab_to_linear(mix(linear_to_oklab(rgb), linear_to_oklab(own), w)), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    return vec4<f32>(rgb, mix(mean.a, 1.0, w));
}

// The angle from a to b the shorter way (radians, wrapped into -pi..pi) and 1 when that uses the
// stroke's 180-degree symmetry (more than 90 degrees apart: the other way round, bend negated).
fn shorter_turn(a: f32, b: f32) -> vec2<f32> {
    let tau = 6.2831853;
    let pi = 3.14159265;
    var d = b - a;
    d = d - tau * floor((d + pi) / tau);
    if (abs(d) > 0.5 * pi) {
        return vec2<f32>(d - sign(d) * pi, 1.0);
    }
    return vec2<f32>(d, 0.0);
}

// A pair of a transition between its two strokes at its own fraction f (0 < f < 1; the colour
// is mixed separately, in Oklab): strokes/brush_stroke_morph.rs morph_record.
fn morph_record(a: StrokeRecord, b: StrokeRecord, f: f32) -> StrokeRecord {
    let turn = shorter_turn(a.a.z, b.a.z);
    let late = f >= 0.5;
    var m: StrokeRecord;
    m.a = vec4<f32>(mix(a.a.xy, b.a.xy, f), a.a.z + turn.x * f, mix(a.a.w, b.a.w, f));
    m.b = vec4<f32>(mix(a.b.x, b.b.x, f), mix(a.b.y, select(b.b.y, -b.b.y, turn.y > 0.5), f), mix(a.b.z, b.b.z, f), mix(a.b.w, b.b.w, f));
    m.c = vec4<f32>(mix(a.c.x, b.c.x, f), mix(a.c.y, b.c.y, f), mix(a.c.z, b.c.z, f), 0.0);
    let side = select(a.d, b.d, late);
    m.d = vec4<u32>(a.d.x, side.y & 0xFFFFu, side.z, side.w);
    return m;
}

// The linear (straight) colour of a record's packed sRGB bytes.
fn record_colour(packed: u32) -> vec4<f32> {
    let c = vec4<f32>(f32(packed & 255u), f32((packed >> 8u) & 255u), f32((packed >> 16u) & 255u), f32(packed >> 24u)) / 255.0;
    return vec4<f32>(srgb_to_linear(c.rgb), c.a);
}

@vertex
fn vs_stroke(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> StrokeOut {
    var out: StrokeOut;
    out.position = vec4<f32>(2.0, 2.0, 0.0, 1.0);
    out.local = vec2<f32>(0.0);
    out.color = vec4<f32>(0.0);
    out.shape = vec4<f32>(0.0);
    out.look = vec4<f32>(0.0);
    out.height = 0.0;
    var r = records[ii];
    let t = draw.e.z;
    let flags = draw.flags.x;
    // A transition: the pair at its own fraction (exactly its first stroke at 0, its second at 1).
    var painted = 1.0;
    var own = record_colour(r.d.x);
    var own_w = stroke_own_weight(r.d.y);
    var reversed = false;
    if ((flags & STROKE_MORPH) != 0u) {
        let q = targets[ii];
        let kind = (q.d.y >> 16u) & 0xFFu;
        let f = clamp((draw.p.z - draw.p.w * q.c.w) / max(1.0 - draw.p.w, 1e-6), 0.0, 1.0);
        if (kind == 1u) {
            painted = 1.0 - f;
        } else if (kind == 2u) {
            painted = f;
        } else if (f >= 1.0) {
            r = q;
            r.d.y = q.d.y & 0xFFFFu;
            own = record_colour(r.d.x);
            own_w = stroke_own_weight(q.d.y);
        } else if (f > 0.0) {
            let ca = record_colour(r.d.x);
            let cb = record_colour(q.d.x);
            reversed = f >= 0.5 && shorter_turn(r.a.z, q.a.z).y > 0.5;
            own = vec4<f32>(clamp(oklab_to_linear(mix(linear_to_oklab(ca.rgb), linear_to_oklab(cb.rgb), f)), vec3<f32>(0.0), vec3<f32>(1.0)), mix(ca.a, cb.a, f));
            own_w = mix(own_w, stroke_own_weight(q.d.y), f);
            r = morph_record(r, q, f);
        }
    }
    // How much is painted (BrushStrokeRecord::progress_at): exactly 1 from the end on.
    var p = 0.0;
    if (t >= r.c.y) {
        p = 1.0;
    } else if (t >= r.c.x) {
        p = clamp((t - r.c.x) / (r.c.y - r.c.x), 0.0, 1.0);
    }
    p *= painted;
    if (!(p > 0.0)) {
        return out;
    }
    var center = r.a.xy;
    var angle = r.a.z;
    var len = r.a.w;
    var width = r.b.x;
    if ((flags & STROKE_BOIL) != 0u) {
        // stroke_boil.rs boil_jitter: four numbers from (boil seed; stroke seed, redraw, k).
        let epoch = bitcast<i32>(draw.flags.z);
        let seed = i32(r.d.w);
        center += draw.boil.x * vec2<f32>(hash_unit(noise_hash(draw.flags.w, seed, epoch, 0)), hash_unit(noise_hash(draw.flags.w, seed, epoch, 1)));
        angle += draw.boil.y * hash_unit(noise_hash(draw.flags.w, seed, epoch, 2));
        let k = 1.0 + draw.boil.z * hash_unit(noise_hash(draw.flags.w, seed, epoch, 3));
        len *= k;
        width *= k;
    }
    if ((flags & STROKE_FLOW) != 0u) {
        // One octave of 3D gradient noise at the stroke's centre, z the time.
        angle += draw.flow.z * noise_once(draw.flow_seed.x, vec3<f32>(r.a.xy * draw.flow.x, t * draw.flow.y), true, true);
    }
    // The two anchors blended, then the element's transform to the device.
    let i0 = r.d.z & 0xFFFFu;
    let i1 = r.d.z >> 16u;
    let blend = clamp(r.c.z, 0.0, 1.0);
    let m0 = mix(anchor_part(i0, 0u), anchor_part(i1, 0u), blend);
    let m1 = mix(anchor_part(i0, 1u), anchor_part(i1, 1u), blend);
    let dir = vec2<f32>(cos(angle), sin(angle));
    let nrm = vec2<f32>(-dir.y, dir.x);
    let ce = apply2(m0, center) + m1.xy;
    let cd = apply2(draw.m, ce) + draw.e.xy;
    let ud = apply2(draw.m, apply2(m0, dir * len));   // the device step per unit of s
    let nd = apply2(draw.m, apply2(m0, nrm));         // the device step per set unit across
    let ul = max(length(ud), 1e-6);
    let nl = max(length(nd), 1e-6);
    // The thin-stroke rule: narrower than a device pixel = one pixel wide at less alpha.
    var half = width * 0.5;
    var alpha = draw.e.w * r.b.w;
    let device_width = width * nl;
    if (device_width < 1.0) {
        alpha *= device_width;
        half = 0.5 / nl;
    }
    let bow = r.b.y * len;
    let profile = r.b.z;
    // The hull: station st of four (0, 1/4, 3/4, all of the painted part), low or high side.
    var k = array<f32, 4>(0.0, 0.25, 0.75, 1.0);
    let st = vi / 2u;
    let a = -0.5 + k[max(st, 1u) - 1u] * p;
    let b = -0.5 + k[min(st + 1u, 3u)] * p;
    let ext = stroke_extent(bow, half, profile, a, b);
    var s = -0.5 + k[st] * p;
    if (st == 0u) {
        s -= 1.0 / ul;
    }
    if (st == 3u) {
        s += 1.0 / ul;
    }
    let across = select(ext.x - 1.0 / nl, ext.y + 1.0 / nl, (vi & 1u) == 1u);
    let pos = cd + ud * s + nd * across;
    out.position = to_clip(pos);
    out.local = vec2<f32>(s, across);
    var rgb = own.rgb;
    var ca = own.a * alpha;
    if ((draw.pick.x & PICK_ON) != 0u) {
        let picked = picked_colour(rgb, center, dir, nrm, len, bow, m0, m1, r.d.w, own_w);
        rgb = picked.rgb;
        ca *= picked.a;
    }
    out.color = vec4<f32>(rgb * ca, ca);
    out.height = alpha * draw.p.y;
    out.shape = vec4<f32>(half, bow, profile, -0.5 + p);
    let tip = r.d.y & 0xFFFFu;
    let use_tip = (flags & STROKE_TIPS) != 0u && tip > 0u && tip <= textureNumLayers(tips);
    // A pair drawn reversed (the second stroke turned the other way round) samples its tip
    // reversed along and across, so it shows as the second stroke shows.
    let mirrored = ((flags & STROKE_MIRROR) != 0u && (r.d.w & 1u) == 1u) != reversed;
    out.look = vec4<f32>(draw.p.x, select(-1.0, f32(tip) - 1.0, use_tip), select(0.0, 1.0, mirrored) + select(0.0, 2.0, reversed), select(0.0, 1.0, p < 1.0));
    return out;
}

// The stroke's coverage at a fragment (outline x head x tip) and the tip's height there (1 for
// no tip): shared by the colour and the height pass. The one texture sample is outside branches.
fn stroke_cover(in: StrokeOut) -> vec2<f32> {
    let s = in.local.x;
    let a = in.local.y;
    let half = in.shape.x;
    let c = stroke_centre(in.shape.y, s);
    // The outline: the distance across to the edge, smoothed over a device pixel.
    let f = abs(a - c) - stroke_half(half, in.shape.z, s);
    let fw = max(length(vec2<f32>(dpdx(f), dpdy(f))), 1e-4);
    var cov = clamp(0.5 - f / fw, 0.0, 1.0);
    // The brush's head: a straight cut (smoothed over a pixel) or a fade over the head softness.
    let g = s - in.shape.w;
    let gw = max(length(vec2<f32>(dpdx(g), dpdy(g))), 1e-6);
    let hard = clamp(0.5 - g / gw, 0.0, 1.0);
    let soft = clamp(-g / max(in.look.x, 1e-6), 0.0, 1.0);
    cov *= select(1.0, select(hard, soft, in.look.x > 0.0), in.look.w > 0.5);
    // The tip: along = s + 0.5, across from the left of travel (0) to the right (1) over the
    // stroke's whole width, mirrored by seed.
    let reversed = in.look.z > 1.5;
    let mirrored = in.look.z - select(0.0, 2.0, reversed) > 0.5;
    var v = (a - c) / max(half, 1e-6) * 0.5 + 0.5;
    v = select(v, 1.0 - v, mirrored);
    let u = select(s + 0.5, 0.5 - s, reversed);
    let texel = textureSample(tips, tip_sampler, vec2<f32>(u, v), i32(max(in.look.y, 0.0)));
    let has_tip = in.look.y >= 0.0;
    cov *= select(1.0, texel.r, has_tip);
    return vec2<f32>(cov, select(1.0, texel.g, has_tip));
}

@fragment
fn fs_stroke(in: StrokeOut) -> @location(0) vec4<f32> {
    let cover = stroke_cover(in);
    // The hull's empty corners write nothing (no 4-sample blend for nothing: measured ~20 % of
    // a dense set's GPU time at device pixel ratio 2 on an integrated GPU).
    if (cover.x <= 0.0) {
        discard;
    }
    return in.color * cover.x;
}

// The height pass of a finished layer (single sample, additive into R16Float): paint piles up
// where strokes overlap.
@fragment
fn fs_stroke_height(in: StrokeOut) -> @location(0) vec4<f32> {
    let cover = stroke_cover(in);
    if (cover.x <= 0.0) {
        discard;
    }
    return vec4<f32>(cover.x * cover.y * in.height, 0.0, 0.0, 1.0);
}
