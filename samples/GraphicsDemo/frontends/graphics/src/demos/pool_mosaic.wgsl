// The pool floor of the "Water pond" demo: the pastel glass MOSAIC of an eight-petalled ROSETTE
// (petals with pale gold outlines, a gold ring, a cream heart, sea glass around it, the glass
// tesserae with their grout). The functions `hash2`, `petal`, `rosette` and `tesserae`, and the way
// the floor colour is made from them (the glass shaded by its piece, then the grout mixed in), are
// carried over unchanged from the repository owner's own palm visualizer sample (its Ripple pool's
// surface shader); only the frame around them is this demo's: the picture is drawn ONCE into a
// texture of the pool floor's shape (POOL_ASPECT wide, 1 high, in the palm pool's units: the floor
// point p runs 0..POOL_ASPECT across and 0..1 down, the rosette centred), 2 x 2 samples per texel,
// and written to an sRGB texture so the texture holds the palm shader's colour values as they
// were shown. The water (refraction, caustics, absorption, reflection, glints, sheen) is the
// graphics crate's water surface, not part of this shader.

// The texture's size in texels and the floor's shape (width / height): kept in step with
// FLOOR_TEXTURE and POOL_SIZE in water_pond.rs.
const RESOLUTION = vec2<f32>(2800.0, 1600.0);
const POOL_ASPECT = 1.75;

const PI = 3.14159265;

fn hash2(p: vec2<f32>) -> vec2<f32> {
    let q = vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)), dot(p, vec2<f32>(269.5, 183.3)));
    return fract(sin(q) * 43758.5453);
}

// Signed distance to one petal of the rosette: a pointed lens (two circular arcs) running
// from near the heart, at 0.06, out to the rim, at 0.44, along angle a
fn petal(q: vec2<f32>, a: f32) -> f32 {
    let along = vec2<f32>(cos(a), sin(a));
    let p = abs(vec2<f32>(dot(q, vec2<f32>(-along.y, along.x)), dot(q, along) - 0.25));
    // Half-length 0.19, half-width 0.10: arcs of radius 0.2305 centred 0.1305 to either side
    let r = 0.2305;
    let d = 0.1305;
    let b = 0.19;
    if ((p.y - b) * d > p.x * b) {
        return length(p - vec2<f32>(0.0, b));
    }
    return length(p - vec2<f32>(-d, 0.0)) - r;
}

// The rosette's design at a point (relative to its centre): eight pastel glass petals with
// pale gold outlines, a cream heart, a gold ring holding it all, sea glass around it
fn rosette(q: vec2<f32>) -> vec3<f32> {
    var palette = array<vec3<f32>, 4>(
        vec3<f32>(0.92, 0.50, 0.64), vec3<f32>(0.98, 0.64, 0.36),
        vec3<f32>(0.60, 0.48, 0.90), vec3<f32>(0.40, 0.80, 0.66));
    let gold = vec3<f32>(0.80, 0.58, 0.18);
    let r = length(q);

    var col = mix(vec3<f32>(0.45, 0.74, 0.84), vec3<f32>(0.62, 0.58, 0.88), 0.5 + 0.5 * sin(q.x * 4.0 + q.y * 3.0));
    if (r < 0.47) {
        col = mix(col, vec3<f32>(0.84, 0.88, 0.95), 0.15);
    }

    var fill = vec3<f32>(0.0);
    var count = 0.0;
    var edge = 1000.0;
    for (var k = 0; k < 8; k = k + 1) {
        let a = (f32(k) + 0.5) * PI / 4.0;
        let dk = petal(q, a);
        edge = min(edge, abs(dk));
        if (dk < 0.0) {
            // Lighter toward the petal's tip
            let tip = clamp((dot(q, vec2<f32>(cos(a), sin(a))) - 0.06) / 0.38, 0.0, 1.0);
            let base = palette[k % 4];
            fill = fill + mix(base * 0.92, mix(base, vec3<f32>(1.0), 0.15), tip);
            count = count + 1.0;
        }
    }
    if (count > 0.0) {
        // Where two petals overlap, the glass layers into a lighter blend of both
        col = mix(fill / count, vec3<f32>(1.0), 0.06 * (count - 1.0));
    }
    if (edge < 0.0075 || abs(r - 0.47) < 0.00975) {
        col = gold;
    }

    // The heart: a solid cream highlight whose rim softens into the petals
    col = mix(col, vec3<f32>(0.98, 0.88, 0.66), 1.0 - smoothstep(0.045, 0.072, r));
    return col;
}

// The glass tesserae: (distance to the piece's edge, two random values for the piece)
fn tesserae(p: vec2<f32>) -> vec3<f32> {
    let cells = p * 34.0;
    let i = floor(cells);
    let f = fract(cells);
    var nearest = 8.0;
    var second = 8.0;
    var id = vec2<f32>(0.0);
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let g = vec2<f32>(f32(x), f32(y));
            let offset = g + hash2(i + g) * 0.8 + 0.1 - f;
            let dist = dot(offset, offset);
            if (dist < nearest) {
                second = nearest;
                nearest = dist;
                id = i + g;
            } else if (dist < second) {
                second = dist;
            }
        }
    }
    return vec3<f32>((sqrt(second) - sqrt(nearest)) * 0.5 / 34.0, hash2(id + 7.3));
}

// The floor's colour at a floor point p, exactly as the palm pool's surface shader makes it
// before the water: the rosette's glass, shaded per piece, and the grout.
fn floor_color(p: vec2<f32>) -> vec3<f32> {
    let pieces = tesserae(p);
    var glass = rosette(p - vec2<f32>(POOL_ASPECT * 0.5, 0.5));
    glass = glass * (0.94 + 0.10 * pieces.y) + (pieces.z - 0.5) * 0.03;
    let groutWidth = 0.0022;
    let grout = 1.0 - smoothstep(groutWidth * 0.5, groutWidth * 1.5, pieces.x);
    return mix(glass, vec3<f32>(0.82, 0.82, 0.80), grout * 0.55);
}

// The palm pool wrote its colours straight to the screen; the texture is sRGB, so the shader
// hands it the linear value whose sRGB encoding is the palm colour.
fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let v = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    return select(pow((v + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4)), v / 12.92, v <= vec3<f32>(0.04045));
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    // One triangle that covers the whole target
    var corners = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(corners[index], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) fragCoord: vec4<f32>) -> @location(0) vec4<f32> {
    // 2 x 2 samples per texel, averaged as the palm pool's colours, then made linear.
    var sum = vec3<f32>(0.0);
    for (var k = 0; k < 4; k = k + 1) {
        let offset = vec2<f32>(f32(k % 2) - 0.5, f32(k / 2) - 0.5) * 0.5;
        let uv = (fragCoord.xy + offset) / RESOLUTION;
        sum = sum + floor_color(vec2<f32>(uv.x * POOL_ASPECT, uv.y));
    }
    return vec4<f32>(to_linear(sum * 0.25), 1.0);
}
