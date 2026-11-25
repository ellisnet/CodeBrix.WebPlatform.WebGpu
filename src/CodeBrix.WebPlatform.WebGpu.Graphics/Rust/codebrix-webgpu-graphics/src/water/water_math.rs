//! The water surface's arithmetic on the CPU (Not in C#): the uniform record the water shader
//! reads (offsets asserted), the per-frame phases computed in f64, the detail ripples' waves,
//! and the CPU twins of the shader's formulas - Schlick's Fresnel, absorption and scattering by
//! the path through the water, the soft shore, depth reconstruction, the refraction fallback, the
//! screen-space reflection's march and its fades, the caustic net, the bottom's normal from three
//! depth points and the sheen's colour curve (the twins are compiled for the host tests only). `water_surface.wgsl` uses the same formulas: keep
//! the two in step (tests in `water::tests`).

use glam::{Mat4, Vec3, Vec4};
#[cfg(test)]
use glam::Vec2;

use crate::context::GraphicsException;

/// The water's reflectance looking straight down (Schlick's F0 for water, ior 1.33).
#[cfg(test)]
pub(crate) const WATER_F0: f32 = 0.02;
/// The steps of the screen-space reflection's march, and its refinement steps.
#[cfg(test)]
pub(crate) const MARCH_STEPS: u32 = 24;
#[cfg(test)]
pub(crate) const REFINE_STEPS: u32 = 4;
/// The detail ripples per surface.
pub(crate) const DETAIL_WAVES: usize = 4;
/// Each detail ripple's wavelength as a share of the longest, and its share of the slope.
pub(crate) const DETAIL_LENGTHS: [f32; DETAIL_WAVES] = [1.0, 0.71, 0.53, 0.37];
pub(crate) const DETAIL_WEIGHTS: [f32; DETAIL_WAVES] = [0.4, 0.3, 0.2, 0.1];
/// The sheen's band phase turns this many times faster than its drift.
pub(crate) const SHEEN_BAND_RATE: f64 = 6.0;
/// The caustic net's second motion runs at this share of the first (no visible repeat).
pub(crate) const CAUSTIC_SECOND_RATE: f64 = 0.618_034;

/// The water shader's uniforms (one 256-byte-aligned slot per surface drawn). Layout in
/// `water_surface.wgsl` (`struct Water`), every field a vec4 / mat4 / array of vec4.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WaterUniforms {
    pub(crate) view_projection: Mat4,
    pub(crate) inverse_view_projection: Mat4,
    /// xyz: the camera's position; w: the lighting's exposure.
    pub(crate) camera: Vec4,
    /// The target's width, height, 1 / width, 1 / height (pixels).
    pub(crate) viewport: Vec4,
    /// xyz: the direction the key light travels (unit); w: 1 when the key light is lit.
    pub(crate) key_direction: Vec4,
    /// rgb: the key light's colour times its intensity.
    pub(crate) key_color: Vec4,
    /// x: 1 for an environment (0 the hemisphere); y: its intensity; z: its last prefiltered mip
    /// level; w: its rotation about +Y.
    pub(crate) environment: Vec4,
    pub(crate) ambient_sky: Vec4,
    pub(crate) ambient_ground: Vec4,
    pub(crate) ambient_up: Vec4,
    /// xyz: the still surface's centre; w: its rotation about +Y.
    pub(crate) surface: Vec4,
    /// x, y: the extent along local X and Z; z: quads per side; w: one quad's side (the larger).
    pub(crate) extent: Vec4,
    /// x: 1 with a wave field; y: its height scale; z: 1 when it moves the vertices.
    pub(crate) field: Vec4,
    /// x, y: the field's extent in field units.
    pub(crate) field_extent: Vec4,
    /// rgb: absorption per unit; w: turbidity.
    pub(crate) absorption: Vec4,
    /// rgb: the scattering colour; w: refraction.
    pub(crate) scattering: Vec4,
    /// reflectivity, roughness, sun glint, soft shore.
    pub(crate) look: Vec4,
    /// caustics strength (0: none), scale, focus, depth fade.
    pub(crate) caustics: Vec4,
    /// x, y: the caustic net's two motion phases (radians).
    pub(crate) caustic_phase: Vec4,
    /// The current phase of each detail ripple (radians).
    pub(crate) detail_phase: Vec4,
    /// Each detail ripple: direction (x, z), wavenumber, slope amplitude.
    pub(crate) detail_waves: [Vec4; DETAIL_WAVES],
    /// strength, scale, slope gain, height gain.
    pub(crate) sheen: Vec4,
    /// x: the film's drift (0..1 cycles); y: the band phase (radians).
    pub(crate) sheen_phase: Vec4,
    /// x: screen reflections; y: local-light highlights; z: shadows received; w: shader-side sRGB
    /// encoding (an 8-bit linear target).
    pub(crate) switches: [u32; 4],
    /// x: open-water waves; y: local lights; z: the sun map's filter grid per side (0: no map); w:
    /// the caustic net's seed.
    pub(crate) counts: [u32; 4],
    pub(crate) sun_matrix: Mat4,
    /// strength, texel (1 / size), depth bias, normal offset.
    pub(crate) sun_shadow: Vec4,
    /// Two vec4 per open-water wave (`GerstnerWaveRecord`).
    pub(crate) waves: [Vec4; 32],
    /// Four vec4 per local light, as the model shader's.
    pub(crate) local_lights: [Vec4; 64],
}

pub(crate) const WATER_UNIFORM_SIZE: u64 = 2160;
const _: () = assert!(std::mem::size_of::<WaterUniforms>() == WATER_UNIFORM_SIZE as usize);
/// The bytes between two surfaces' uniforms: the size rounded up to the dynamic-offset alignment.
pub(crate) const WATER_SLOT_SIZE: u64 = WATER_UNIFORM_SIZE.div_ceil(256) * 256;
// SAFETY: #[repr(C)]; Mat4s, Vec4s and u32 quadruples at 16-byte offsets, no padding (size
// asserted above); every bit pattern is a valid value.
unsafe impl bytemuck::Zeroable for WaterUniforms {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for WaterUniforms {}

impl Default for WaterUniforms {
    fn default() -> WaterUniforms {
        bytemuck::Zeroable::zeroed()
    }
}

/// `x * TAU` wrapped into 0..TAU, from an f64 count of cycles (exact for hours of page time).
pub(crate) fn cycles_to_phase(cycles: f64) -> f32 {
    (cycles.rem_euclid(1.0) * std::f64::consts::TAU) as f32
}

/// The caustic net's two motion phases at `seconds` for `speed` cycles per second.
pub(crate) fn caustic_phases(seconds: f64, speed: f32) -> (f32, f32) {
    let cycles = seconds * speed as f64;
    (cycles_to_phase(cycles), cycles_to_phase(cycles * CAUSTIC_SECOND_RATE))
}

/// The sheen's drift (0..1 film cycles) and band phase (radians) at `seconds`.
pub(crate) fn sheen_phases(seconds: f64, drift: f32) -> (f32, f32) {
    let cycles = seconds * drift as f64;
    (cycles.rem_euclid(1.0) as f32, cycles_to_phase(cycles * SHEEN_BAND_RATE))
}

/// The 32-bit hash both sides use (a PCG step, then an output permutation).
pub(crate) fn hash_u32(x: u32) -> u32 {
    let state = x.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

/// Two numbers in 0..1 for a cell of the caustic net and a seed.
#[cfg(test)]
pub(crate) fn hash_cell(x: i32, y: i32, seed: u32) -> Vec2 {
    let h = hash_u32((x as u32) ^ hash_u32((y as u32).wrapping_add(seed.wrapping_mul(0x9E37_79B9))));
    let g = hash_u32(h ^ 0x68E3_1DA4);
    Vec2::new((h >> 8) as f32 / 16_777_216.0, (g >> 8) as f32 / 16_777_216.0)
}

/// The detail ripples of a seed: per wave its direction (x, z), wavenumber and slope amplitude.
pub(crate) fn detail_waves(strength: f32, wavelength: f32, seed: u32) -> [Vec4; DETAIL_WAVES] {
    let base = hash_u32(seed ^ 0x2545_F491) as f32 / 4_294_967_296.0 * std::f32::consts::TAU;
    let wavelength = wavelength.max(1e-3);
    let mut waves = [Vec4::ZERO; DETAIL_WAVES];
    for (i, wave) in waves.iter_mut().enumerate() {
        // Spread by the golden angle so no two run alike.
        let angle = base + i as f32 * 2.399_963;
        let k = std::f32::consts::TAU / (wavelength * DETAIL_LENGTHS[i]);
        *wave = Vec4::new(angle.cos(), angle.sin(), k, strength.max(0.0) * DETAIL_WEIGHTS[i]);
    }
    waves
}

/// The detail ripples' current phases at `seconds` (each travels at `speed` units per second).
pub(crate) fn detail_phases(waves: &[Vec4; DETAIL_WAVES], speed: f32, seconds: f64) -> Vec4 {
    let phase = |w: Vec4| cycles_to_phase(seconds * (w.z as f64 * speed as f64) / std::f64::consts::TAU);
    Vec4::new(phase(waves[0]), phase(waves[1]), phase(waves[2]), phase(waves[3]))
}

/// The slope (dh/dx, dh/dz) the detail ripples add at a world point, each faded by its
/// wavelength over `footprint` (scene units per pixel) like the open-water waves' normals.
#[cfg(test)]
pub(crate) fn detail_slope(waves: &[Vec4; DETAIL_WAVES], phases: Vec4, p: Vec2, footprint: f32) -> Vec2 {
    let mut slope = Vec2::ZERO;
    for (i, w) in waves.iter().enumerate() {
        let fade = crate::water::OpenWaterWaves::normal_fade(std::f32::consts::TAU / w.z.max(1e-6), footprint);
        let argument = w.z * (w.x * p.x + w.y * p.y) - phases[i];
        slope += Vec2::new(w.x, w.y) * (w.w * argument.cos() * fade);
    }
    slope
}

/// Schlick's Fresnel for water at `cosine` (the view against the normal), scaled by
/// `reflectivity` and kept in 0..1.
#[cfg(test)]
pub(crate) fn fresnel(cosine: f32, reflectivity: f32) -> f32 {
    let c = cosine.clamp(0.0, 1.0);
    ((WATER_F0 + (1.0 - WATER_F0) * (1.0 - c).powi(5)) * reflectivity).clamp(0.0, 1.0)
}

/// What is left of light after `path` units through water absorbing `absorption` per unit.
#[cfg(test)]
pub(crate) fn transmittance(absorption: Vec3, path: f32) -> Vec3 {
    (-absorption.max(Vec3::ZERO) * path.max(0.0)).exp()
}

/// The water's colour from the bottom's colour `bottom` seen through `path` units: absorbed, with
/// the scattered colour (`scattering` lit by `light`) taking over at the rate `turbidity`.
#[cfg(test)]
pub(crate) fn tint(bottom: Vec3, absorption: Vec3, scattering: Vec3, turbidity: f32, light: Vec3, path: f32) -> Vec3 {
    let path = path.max(0.0);
    bottom * transmittance(absorption, path) + scattering * light * (1.0 - (-turbidity.max(0.0) * path).exp())
}

/// How much of the water's own look shows where the path through it is `path`: 0 at the shore
/// (no water), 1 from `soft_shore` on (a smooth step between; 1 everywhere when 0).
#[cfg(test)]
pub(crate) fn shore(path: f32, soft_shore: f32) -> f32 {
    if soft_shore <= 0.0 {
        return 1.0;
    }
    let x = (path / soft_shore).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The world point at screen `uv` (0..1, y down) and depth `depth` (0..1).
#[cfg(test)]
pub(crate) fn reconstruct(inverse_view_projection: Mat4, uv: Vec2, depth: f32) -> Vec3 {
    let ndc = Vec4::new(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    let world = inverse_view_projection * ndc;
    world.truncate() / world.w
}

/// A world point's screen `uv` and depth; None behind the camera.
#[cfg(test)]
pub(crate) fn project(view_projection: Mat4, world: Vec3) -> Option<(Vec2, f32)> {
    let clip = view_projection * world.extend(1.0);
    if clip.w <= 1e-6 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    Some((Vec2::new(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5), ndc.z))
}

/// The refraction's fallback rule: the shifted view is used only when what it finds lies BEHIND
/// the water's own depth there (under the surface); something in front of the water (above it)
/// would leak into the refraction, so the unshifted view is used instead.
#[cfg(test)]
pub(crate) fn uses_shifted(shifted_scene_depth: f32, water_depth: f32) -> bool {
    shifted_scene_depth >= water_depth
}

/// The screen-edge fade of a reflection found at `uv`: 0 at the edge, 1 from a tenth of the
/// screen inwards.
#[cfg(test)]
pub(crate) fn edge_fade(uv: Vec2) -> f32 {
    let e = uv.min(Vec2::ONE - uv).min_element();
    let x = (e / 0.1).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The fade of a reflected ray `r` by how far it turns back toward the camera (`v`, from the
/// surface to the eye) and how far it dips under the horizon: 1 for a ray running away from the
/// camera, 0 for one coming straight back (what it would show is behind the camera) or pointing
/// down into the water.
#[cfg(test)]
pub(crate) fn facing_fade(r: Vec3, v: Vec3) -> f32 {
    let back = (1.0 - 2.0 * r.dot(v)).clamp(0.0, 1.0);
    let up = ((r.y + 0.05) / 0.1).clamp(0.0, 1.0);
    back * up
}

/// The screen-space reflection's march (the shader's twin): from `start` along `direction` (unit)
/// up to `max_distance`, in [`MARCH_STEPS`] steps growing with the square of the step number,
/// against the scene depth `depth_at(uv)`; a hit is where the ray passes behind a surface by less
/// than about two steps, refined by [`REFINE_STEPS`] halvings. Returns the hit's screen uv and its
/// distance along the ray; None for a miss (off the screen, behind the camera, or only sky).
#[cfg(test)]
pub(crate) fn march(depth_at: &dyn Fn(Vec2) -> f32, view_projection: Mat4, inverse_view_projection: Mat4, start: Vec3, direction: Vec3, max_distance: f32) -> Option<(Vec2, f32)> {
    let behind = |t: f32| -> Option<(Vec2, f32)> {
        let q = start + direction * t;
        let (uv, _) = project(view_projection, q)?;
        if uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0 {
            return None;
        }
        let scene = depth_at(uv);
        if scene >= 1.0 {
            return Some((uv, f32::NEG_INFINITY));
        }
        let eye = reconstruct(inverse_view_projection, uv, 0.0);
        let surface = reconstruct(inverse_view_projection, uv, scene);
        Some((uv, eye.distance(q) - eye.distance(surface)))
    };
    let mut previous = 0.0f32;
    for i in 1..=MARCH_STEPS {
        let s = i as f32 / MARCH_STEPS as f32;
        let t = max_distance * s * s;
        let (uv, depth_behind) = behind(t)?;
        let thickness = (2.0 * (t - previous)).max(0.05);
        if depth_behind > 0.0 && depth_behind < thickness {
            let (mut low, mut high) = (previous, t);
            let mut found = uv;
            for _ in 0..REFINE_STEPS {
                let middle = 0.5 * (low + high);
                match behind(middle) {
                    Some((at, d)) if d > 0.0 => {
                        high = middle;
                        found = at;
                    }
                    _ => low = middle,
                }
            }
            return Some((found, high));
        }
        previous = t;
    }
    None
}

/// The unit normal of the bottom from its point `b` and the points of the next pixel across
/// (`bx`) and down (`by`), turned to face `v` (the eye side).
#[cfg(test)]
pub(crate) fn bottom_normal(b: Vec3, bx: Vec3, by: Vec3, v: Vec3) -> Vec3 {
    let n = (by - b).cross(bx - b).normalize_or(Vec3::Y);
    if n.dot(v) < 0.0 {
        -n
    } else {
        n
    }
}

/// The caustic net at point `p` (in net cells) for the motion phases and a seed: bright thin cell
/// edges of a domain-warped field of moving points, 0..1.
#[cfg(test)]
pub(crate) fn caustic_net(p: Vec2, phase_a: f32, phase_b: f32, seed: u32) -> f32 {
    let q = p + 0.35 * Vec2::new((p.y * 1.7 + phase_b).sin(), (p.x * 1.3 - phase_b + 1.0).sin());
    let cell = q.floor();
    let f = q - cell;
    let (mut d1, mut d2) = (8.0f32, 8.0f32);
    for y in -1..=1 {
        for x in -1..=1 {
            let h = hash_cell(cell.x as i32 + x, cell.y as i32 + y, seed);
            let point = Vec2::new(x as f32, y as f32) + Vec2::splat(0.5) + 0.38 * Vec2::new((phase_a + std::f32::consts::TAU * h.x).sin(), (phase_b + std::f32::consts::TAU * h.y).sin());
            let d = point.distance(f);
            if d < d1 {
                d2 = d1;
                d1 = d;
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    let x = ((d2 - d1) / 0.18).clamp(0.0, 1.0);
    let line = 1.0 - x * x * (3.0 - 2.0 * x);
    line * line
}

/// The sheen's interference colour for a film thickness (in cycles; period 1).
#[cfg(test)]
pub(crate) fn sheen_color(film: f32) -> Vec3 {
    let t = std::f32::consts::TAU;
    Vec3::new(0.5 + 0.5 * (t * film).cos(), 0.5 + 0.5 * (t * (film + 0.33)).cos(), 0.5 + 0.5 * (t * (film + 0.67)).cos())
}

/// The sheen added to the water's colour: the colour around its middle (signed), stronger where
/// the surface slopes, times `strength` (0: nothing at all).
#[cfg(test)]
pub(crate) fn sheen_contribution(film: f32, steepness: f32, strength: f32) -> Vec3 {
    if strength <= 0.0 {
        return Vec3::ZERO;
    }
    (sheen_color(film) - Vec3::splat(0.45)) * ((0.10 + 0.9 * steepness.clamp(0.0, 0.6)) * 0.5 * strength)
}

/// How the scene capture reads the target's depth: a plain or a multisampled depth binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum DepthRead {
    /// `texture_depth_2d`, `textureLoad` (1 sample).
    Single,
    /// `texture_depth_multisampled_2d`, sample 0 (4 samples).
    Multisampled,
}

/// The capture's plan for a target of `sample_count` samples and depth format `depth`: Err for a
/// target without depth or with another depth format than `Depth32Float` (the message says how
/// to make one), or a sample count other than 1 and 4.
pub(crate) fn capture_plan(sample_count: u32, depth: Option<wgpu::TextureFormat>) -> Result<DepthRead, GraphicsException> {
    match depth {
        Some(wgpu::TextureFormat::Depth32Float) => {}
        Some(other) => {
            return Err(GraphicsException::new(format!(
                "The water's scene capture reads the target's depth, and this target's depth is {other:?}; make the target with HeadlessTargetOptions {{ with_depth: true, depth_format: Depth32Float, .. }}."
            )))
        }
        None => return Err(GraphicsException::new("The water's scene capture reads the target's depth, and this target has none; make the target with HeadlessTargetOptions { with_depth: true, depth_format: Depth32Float, .. }.")),
    }
    match sample_count {
        1 => Ok(DepthRead::Single),
        4 => Ok(DepthRead::Multisampled),
        other => Err(GraphicsException::new(format!("The water's scene capture reads targets of 1 or 4 samples; this one has {other}."))),
    }
}

/// Why a surface may not be drawn into a pass of frame `pass_frame`: no capture this frame
/// (`captured_frame` is the frame of the last capture), or a pass on another target than the
/// captured one. None when it may.
pub(crate) fn draw_refusal(captured_frame: Option<u64>, pass_frame: u64, same_target: bool) -> Option<&'static str> {
    if captured_frame != Some(pass_frame) {
        return Some("WaterRenderer::draw needs this frame's scene capture: call capture_scene(&mut frame, &target) after the pass with the opaque parts and the sky, then draw the water in a later pass on that target.");
    }
    if !same_target {
        return Some("WaterRenderer::draw was given a pass on another target than the one captured; draw the water into a pass on the HeadlessRenderTarget given to capture_scene (begin it with None, None to load its colour and depth).");
    }
    None
}

/// A local vector of the surface (before its rotation `angle` about +Y) in world axes (glam's
/// `Mat3::from_rotation_y`).
pub(crate) fn rotate_y(v: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    Vec3::new(c * v.x + s * v.z, v.y, -s * v.x + c * v.z)
}
