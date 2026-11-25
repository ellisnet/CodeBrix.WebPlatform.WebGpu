//! The arithmetic of the effects (crate-internal): C#'s `EffectProcessor.Measure` (how big the
//! effects are on the device), the planes' Gaussian kernel (`PlaneKernels.GaussianKernel`), how
//! far an effect reaches, and the level of detail a blur is computed at on the GPU; and the group
//! a layer's surface finish rides on (effects/surface_finish.rs); and (no C# equivalent) the WATER
//! part of a layer's effect group - a refraction and a reflection on the device, their region
//! growth and their words of the composite's pass data.

use kurbo::Affine;

use crate::effects::{DrawingEffects, ShadowDirection};
use crate::rendering::core_paint_srgb_to_linear as srgb_to_linear;

/// The most taps a GPU blur pass reads on each side of a pixel. A blur whose radius would read
/// more is computed at half (a quarter, ...) the resolution and stretched back (a Gaussian of
/// more than two texels' deviation loses nothing visible that way, and a full-resolution blur of
/// a large canvas is what costs: measured, see the maintainer notes).
pub(crate) const MAX_KERNEL_RADIUS: i32 = 8;
/// The coarsest level a blur is computed at (one pixel of it = this many canvas pixels).
pub(crate) const MAX_LEVEL: u32 = 16;

/// Effect sizes on the device: the factor for radii and the shadow offset in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EffectSizes {
    pub(crate) radius_scale: f32,
    pub(crate) offset_x: f32,
    pub(crate) offset_y: f32,
}

/// How big the effects are on the device (C#'s `EffectProcessor.Measure`). `device` is the
/// content's map from its own units to device pixels; `scene_rotation` the scene's turn (the
/// camera's roll), radians. Radii scale by sqrt|det|. A scene-fixed offset v becomes P (R v): R
/// turns it with the scene, P = sqrt(F^T F) (the map's stretch along the scene's axes with its
/// rotation taken out); an offset that turns with the element goes through the map itself.
pub(crate) fn measure(effects: &DrawingEffects, device: Affine, scene_rotation: f64) -> EffectSizes {
    let c = device.as_coeffs();
    let (a, b, cc, d) = (c[0], c[1], c[2], c[3]);
    let det = (a * d - b * cc).abs();
    let radius_scale = det.sqrt() as f32;
    let (ox, oy) = (effects.shadow_offset().x as f64, effects.shadow_offset().y as f64);
    if ox == 0.0 && oy == 0.0 {
        return EffectSizes { radius_scale, offset_x: 0.0, offset_y: 0.0 };
    }
    if effects.shadow_direction() == ShadowDirection::RotatesWithElement {
        return EffectSizes { radius_scale, offset_x: (ox * a + oy * cc) as f32, offset_y: (ox * b + oy * d) as f32 };
    }
    let (sin, cos) = scene_rotation.sin_cos();
    let (vx, vy) = if scene_rotation == 0.0 { (ox, oy) } else { (ox * cos - oy * sin, ox * sin + oy * cos) };
    // F^T F and its square root by the 2 x 2 closed form (M + sqrt(det M) I) / sqrt(tr M + 2 sqrt(det M)).
    let (m11, m12, m22) = (a * a + cc * cc, a * b + cc * d, b * b + d * d);
    let t = (m11 + m22 + 2.0 * det).sqrt();
    if t <= 0.0 {
        return EffectSizes { radius_scale, offset_x: 0.0, offset_y: 0.0 };
    }
    let (p11, p12, p22) = ((m11 + det) / t, m12 / t, (m22 + det) / t);
    EffectSizes { radius_scale, offset_x: (p11 * vx + p12 * vy) as f32, offset_y: (p12 * vx + p22 * vy) as f32 }
}

/// The planes' Gaussian kernel (C#'s `PlaneKernels.GaussianKernel`): radius (int)(4 sigma + 0.5),
/// weights exp(-0.5 i^2 / sigma^2) in double, normalised by their sum, rounded once to f32.
pub(crate) fn gaussian_kernel(sigma: f32) -> (Vec<f32>, i32) {
    crate::planes::plane_kernels::gaussian_kernel(sigma)
}

/// The pixels a Gaussian of this sigma reaches (C#'s `Reach`: the kernel radius, plus one).
pub(crate) fn reach(sigma: f32) -> i32 {
    if sigma > 0.0 {
        (4.0 * sigma + 0.5) as i32 + 1
    } else {
        0
    }
}

/// The level (1, 2, 4 ... MAX_LEVEL canvas pixels per pixel) a blur of this sigma (canvas
/// pixels) is computed at, so that its kernel reads at most MAX_KERNEL_RADIUS taps a side.
pub(crate) fn level_for(sigma: f32) -> u32 {
    let mut level = 1u32;
    while level < MAX_LEVEL && (4.0 * sigma / level as f32 + 0.5) as i32 > MAX_KERNEL_RADIUS {
        level *= 2;
    }
    level
}

/// One element's (group's, layer's) effects as the renderer takes them: device pixels, linear
/// premultiplied colours.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EffectParams {
    pub(crate) blur_sigma: f32,
    pub(crate) glow_sigma: f32,
    pub(crate) glow_strength: f32,
    pub(crate) glow_color: [f32; 4],
    pub(crate) shadow_sigma: f32,
    pub(crate) shadow_offset: [f32; 2],
    pub(crate) shadow_color: [f32; 4],
    pub(crate) has_glow: bool,
    pub(crate) has_shadow: bool,
    /// The whole-frame operations and the transition weight of a layer (device pixels).
    pub(crate) frame: Option<FrameParams>,
    /// A layer's surface finish (Not in C#): the composite lights the content from the layer's
    /// height record and the canvas weave.
    pub(crate) finish: Option<crate::effects::surface_finish::FinishParams>,
    /// A layer's (or a session's) refraction and reflection (no C# equivalent): the composite
    /// bends and mirrors the content through water.
    pub(crate) water: Option<WaterParams>,
}

/// A layer's whole-frame operations ([`DrawingFrameOperations`](crate::effects::DrawingFrameOperations))
/// and its part in a transition, on the device. `flags`: 1 tone curve, 2 gamma, 4 vignette,
/// 8 bloom; the transition kind + 1 in bits 8-11 (0: none), its direction in bits 12-13.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FrameParams {
    pub(crate) flags: u32,
    pub(crate) tone: [f32; 2],
    pub(crate) gamma: f32,
    /// centre x, centre y, radius x, radius y (device pixels).
    pub(crate) vignette: [f32; 4],
    /// strength, minimum.
    pub(crate) vignette_k: [f32; 2],
    pub(crate) bloom_sigma: f32,
    pub(crate) bloom_gain: f32,
    /// The transition's fraction and softness (device pixels).
    pub(crate) transition: [f32; 2],
    /// The transition's colour, linear premultiplied.
    pub(crate) transition_color: [f32; 4],
    /// The frame the transition runs over (device x0 y0 x1 y1).
    pub(crate) extent: [f32; 4],
}

pub(crate) fn linear_premultiplied(c: crate::color::Color) -> [f32; 4] {
    let a = c.a as f32 / 255.0;
    [srgb_to_linear(c.r as f32 / 255.0) * a, srgb_to_linear(c.g as f32 / 255.0) * a, srgb_to_linear(c.b as f32 / 255.0) * a, a]
}

/// The scale a transform applies to a length (its larger axis).
fn length_scale(t: Affine) -> f32 {
    let c = t.as_coeffs();
    (c[0] * c[0] + c[1] * c[1]).sqrt().max((c[2] * c[2] + c[3] * c[3]).sqrt()) as f32
}

/// The device form of a layer's whole-frame operations and of its part in a transition (as the
/// INCOMING layer, at `fraction`) over the frame `extent` (device x0 y0 x1 y1); `calibration` maps
/// calibrated units to device pixels. None when neither changes anything on the GPU (slides and
/// pushes are transforms of the layers, not passes).
pub(crate) fn frame_params(operations: Option<&crate::effects::DrawingFrameOperations>, transition: Option<(&crate::planes::DrawingTransition, f32)>, calibration: Affine, extent: [f32; 4]) -> Option<FrameParams> {
    use crate::planes::DrawingTransitionKind as K;
    let mut f = FrameParams { extent, ..FrameParams::default() };
    let scale = length_scale(calibration);
    if let Some(o) = operations {
        if let Some((k, gain)) = o.tone_curve {
            f.flags |= 1;
            f.tone = [k, gain];
        }
        if let Some(g) = o.gamma {
            f.flags |= 2;
            f.gamma = g;
        }
        if let Some(v) = o.vignette {
            f.flags |= 4;
            let c = calibration * kurbo::Point::new(v.center_x as f64, v.center_y as f64);
            f.vignette = [c.x as f32, c.y as f32, v.radius_x * scale, v.radius_y * scale];
            f.vignette_k = [v.strength, v.minimum];
        }
        if let Some(b) = o.bloom {
            f.flags |= 8;
            f.bloom_sigma = b.sigma * scale;
            f.bloom_gain = b.gain;
        }
    }
    if let Some((t, fraction)) = transition {
        if matches!(t.kind(), K::Dissolve | K::Wipe | K::FadeThroughColor) {
            f.flags |= ((t.kind() as u32 + 1) << 8) | ((t.direction() as u32) << 12);
            f.transition = [fraction.clamp(0.0, 1.0), t.softness() * scale];
            f.transition_color = linear_premultiplied(t.color());
        }
    }
    if f.flags == 0 {
        None
    } else {
        Some(f)
    }
}

impl EffectParams {
    /// The device form of effects drawn through `device` under a scene turned by
    /// `scene_rotation`; None when nothing would change the content.
    pub(crate) fn new(effects: &DrawingEffects, device: Affine, scene_rotation: f64) -> Option<EffectParams> {
        if effects.is_empty() {
            return None;
        }
        let sizes = measure(effects, device, scene_rotation);
        let s = sizes.radius_scale;
        Some(EffectParams {
            blur_sigma: effects.blur_radius() * 0.5 * s,
            glow_sigma: effects.glow_radius() * 0.5 * s,
            glow_strength: effects.glow_strength(),
            glow_color: linear_premultiplied(effects.glow_color()),
            shadow_sigma: effects.shadow_blur_radius() * 0.5 * s,
            shadow_offset: [sizes.offset_x, sizes.offset_y],
            shadow_color: linear_premultiplied(effects.shadow_color()),
            has_glow: effects.has_glow(),
            has_shadow: effects.has_shadow(),
            frame: None,
            finish: None,
            water: None,
        })
    }

    /// How far, in device pixels, the result reaches beyond the content (C#'s region growth:
    /// the blur's reach plus the larger of the glow's and the offset shadow's).
    pub(crate) fn reach(&self) -> f64 {
        let mut outer = 0;
        if let Some(f) = &self.frame {
            if f.flags & 8 != 0 {
                outer = outer.max(reach(f.bloom_sigma));
            }
        }
        if self.has_glow {
            outer = outer.max(reach(self.glow_sigma));
        }
        if self.has_shadow {
            outer = outer.max(reach(self.shadow_sigma) + self.shadow_offset[0].abs().max(self.shadow_offset[1].abs()).ceil() as i32);
        }
        // A refraction reads its content up to its largest offset away (whole pixels).
        let water = self.water.and_then(|w| w.refraction).map_or(0, |r| r.reach.ceil() as i32);
        (reach(self.blur_sigma) + outer + water) as f64
    }
}

// ---- WATER (no C# equivalent): a layer's refraction and reflection on the device ---------------

/// The largest offset, device pixels, a refraction or a reflection's water moves a pixel by (the
/// shader clamps to it, so the region grown by it holds everything the composite reads).
pub(crate) const MAX_WATER_OFFSET: f32 = 128.0;

/// A layer's (or a session's) water as the composite reads it (crate-internal). `draw` names the
/// scene's [`WaterDraw`](crate::rendering::WaterDraw) holding the sources this frame. The water's
/// coordinates are the layer's DRAWING space; `to_water` maps device pixels into it (a b c d e f:
/// x' = a x + c y + e), `to_device` is the drawing map's linear part (water units -> device).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WaterParams {
    pub(crate) draw: u32,
    pub(crate) to_water: [f32; 6],
    pub(crate) to_device: [f32; 4],
    pub(crate) refraction: Option<RefractionParams>,
    pub(crate) reflection: Option<ReflectionParams>,
}

/// A refraction on the device (crate-internal).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RefractionParams {
    /// Water units moved per unit of slope.
    pub(crate) strength: f32,
    pub(crate) shading: f32,
    /// Towards the light (water axes: x right, y down, z up out of the water).
    pub(crate) light: [f32; 3],
    /// Glints: colour (linear), amount, sharpness.
    pub(crate) glints: Option<([f32; 3], f32, f32)>,
    pub(crate) caustics: Option<CausticParams>,
    /// The largest offset, device pixels (clamped to MAX_WATER_OFFSET).
    pub(crate) reach: f32,
}

/// A caustic light on the device (crate-internal): z = the session time x the speed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CausticParams {
    pub(crate) strength: f32,
    pub(crate) frequency: f32,
    pub(crate) z: f32,
    pub(crate) focus: f32,
    pub(crate) seed: u32,
    pub(crate) color: [f32; 3],
}

/// A reflection on the device (crate-internal): the waterline through `point` with the unit
/// `normal` pointing into the water (below the line), the mirror `gap` device pixels further.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ReflectionParams {
    pub(crate) point: [f32; 2],
    pub(crate) normal: [f32; 2],
    pub(crate) gap: f32,
    pub(crate) opacity: f32,
    /// Device pixels.
    pub(crate) fade: f32,
    /// Device pixels.
    pub(crate) streak: f32,
    pub(crate) water_strength: f32,
    /// Linear rgb, then how much.
    pub(crate) tint: [f32; 4],
    /// The water's largest offset, device pixels (0 for a still mirror).
    pub(crate) reach: f32,
    /// The reflection has water of its own (else a still mirror - or `same` as the refraction's).
    pub(crate) has_water: bool,
    /// Its water IS the refraction's (the composite evaluates it once).
    pub(crate) same: bool,
}

impl ReflectionParams {
    /// A device point mirrored about the mirror line (the waterline moved `gap` into the water).
    pub(crate) fn mirror(&self, p: [f64; 2]) -> [f64; 2] {
        let (n, m) = ([self.normal[0] as f64, self.normal[1] as f64], [self.point[0] as f64 + self.normal[0] as f64 * self.gap as f64, self.point[1] as f64 + self.normal[1] as f64 * self.gap as f64]);
        let d = (p[0] - m[0]) * n[0] + (p[1] - m[1]) * n[1];
        [p[0] - 2.0 * d * n[0], p[1] - 2.0 * d * n[1]]
    }

    /// How far below the waterline a device point is (negative above it), device pixels.
    pub(crate) fn depth(&self, p: [f64; 2]) -> f64 {
        (p[0] - self.point[0] as f64) * self.normal[0] as f64 + (p[1] - self.point[1] as f64) * self.normal[1] as f64
    }

    /// The reflection's weight at a depth below the line (0 above it): `opacity x (1 -
    /// smoothstep(0, fade, depth))` - the CPU mirror of effects.wgsl `with_reflection` (tests).
    #[cfg(test)]
    pub(crate) fn fade_at(&self, depth: f32) -> f32 {
        if depth <= 0.0 {
            return 0.0;
        }
        let t = (depth / self.fade).clamp(0.0, 1.0);
        self.opacity * (1.0 - t * t * (3.0 - 2.0 * t))
    }

    /// The device box the reflection of `content` covers: the part of the content box ABOVE the
    /// waterline, mirrored, grown by the streak and the water's reach (empty when the content lies
    /// wholly below the line).
    pub(crate) fn region(&self, content: kurbo::Rect) -> kurbo::Rect {
        if !(content.width() > 0.0 && content.height() > 0.0) {
            return kurbo::Rect::ZERO;
        }
        // The content box clipped to the half-plane above the line (one Sutherland-Hodgman step).
        let corners = [[content.x0, content.y0], [content.x1, content.y0], [content.x1, content.y1], [content.x0, content.y1]];
        let mut kept: Vec<[f64; 2]> = Vec::with_capacity(8);
        for i in 0..4 {
            let (a, b) = (corners[i], corners[(i + 1) % 4]);
            let (da, db) = (self.depth(a), self.depth(b));
            if da <= 0.0 {
                kept.push(a);
            }
            if (da < 0.0) != (db < 0.0) && da != db {
                let t = da / (da - db);
                kept.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            }
        }
        if kept.len() < 3 {
            return kurbo::Rect::ZERO;
        }
        let mut r: Option<kurbo::Rect> = None;
        for p in kept {
            let m = self.mirror(p);
            let q = kurbo::Rect::new(m[0], m[1], m[0], m[1]);
            r = Some(r.map_or(q, |r| r.union(q)));
        }
        let grow = (self.streak * 0.5 + self.reach) as f64 + 1.0;
        r.map_or(kurbo::Rect::ZERO, |r| r.inflate(grow, grow))
    }
}

/// The layer's (or session's) water for this frame: its sources go into the scene's water list
/// and the device parameters come back (None when there is nothing to draw: no refraction or
/// reflection, or a drawing map that cannot be inverted). `drawing` maps the water's coordinates
/// (calibrated units under the camera) to device pixels; `seconds` is the session time. A source
/// whose live drops exceed the cap is reported through `report` (once per message).
pub(crate) fn record_water(scene: &mut crate::rendering::VectorScene, refraction: Option<&crate::water::WaterRefraction>, reflection: Option<&crate::water::WaterReflection>, drawing: Affine, seconds: f64, report: &mut dyn FnMut(String)) -> Option<WaterParams> {
    use crate::water::water_source::WaterTap;
    if refraction.is_none() && reflection.is_none() {
        return None;
    }
    let det = drawing.determinant();
    if !(det.abs() > 1e-12) || !det.is_finite() {
        return None;
    }
    let scale = length_scale(drawing);
    let mut note = |tap: &WaterTap| {
        if tap.skipped() > 0 {
            report(format!("A ripple field has more than {} live drops at {seconds:.2} s ({} left out): the weakest at that time are not drawn (RippleField::MAX_LIVE_DROPS; RenderStats::ripples_skipped counts them per frame).", crate::water::RippleField::MAX_LIVE_DROPS, tap.skipped()));
        }
    };
    let refraction_tap = refraction.map(|r| WaterTap::new(&r.source, seconds));
    let same = matches!((refraction, reflection.and_then(|r| r.water.as_ref())), (Some(a), Some(b)) if a.source == *b);
    let reflection_tap = if same { None } else { reflection.and_then(|r| r.water.as_ref()).map(|w| WaterTap::new(w, seconds)) };
    for tap in refraction_tap.iter().chain(reflection_tap.iter()) {
        note(tap);
    }
    let reach_of = |strength: f32, tap: Option<&WaterTap>| tap.map_or(0.0, |t| (strength.abs() * t.slope_bound() * scale).min(MAX_WATER_OFFSET));
    let c = drawing.as_coeffs();
    let inverse = drawing.inverse().as_coeffs();
    let mut params = WaterParams { draw: 0, to_water: inverse.map(|v| v as f32), to_device: [c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32], refraction: None, reflection: None };
    if let Some(r) = refraction {
        let lin = |c: crate::Color| {
            let p = linear_premultiplied(crate::Color { a: 255, ..c });
            [p[0], p[1], p[2]]
        };
        params.refraction = Some(RefractionParams {
            strength: r.strength,
            shading: r.shading,
            light: crate::water::water_refraction::light_vector(r.light_angle, r.light_elevation),
            glints: r.glints.map(|g| (lin(g.color), g.amount, g.sharpness)),
            caustics: r.caustics.map(|k| CausticParams { strength: k.strength, frequency: k.frequency, z: (seconds * k.speed as f64) as f32, focus: k.focus, seed: k.seed as u32, color: lin(k.color) }),
            reach: reach_of(r.strength, refraction_tap.as_ref()),
        });
    }
    if let Some(r) = reflection {
        // The waterline on the device: two of its points, and one unit into the water.
        let (line, gap) = (r.line as f64, r.gap as f64);
        let a = drawing * kurbo::Point::new(0.0, line);
        let b = drawing * kurbo::Point::new(1.0, line);
        let below = drawing * kurbo::Point::new(0.0, line + 1.0);
        let t = b - a;
        let t = t / t.hypot();
        let v = below - a;
        let n = v - t * v.dot(t);
        let per_unit = n.hypot();
        if per_unit > 1e-9 {
            let n = n / per_unit;
            let tint = linear_premultiplied(crate::Color { a: 255, ..r.tint });
            let water = if same { refraction_tap.as_ref() } else { reflection_tap.as_ref() };
            params.reflection = Some(ReflectionParams {
                point: [a.x as f32, a.y as f32],
                normal: [n.x as f32, n.y as f32],
                gap: (gap * per_unit) as f32,
                opacity: r.opacity,
                fade: (r.fade_distance as f64 * per_unit) as f32,
                streak: (r.streak as f64 * per_unit) as f32,
                water_strength: r.water_strength,
                tint: [tint[0], tint[1], tint[2], r.tint.a as f32 / 255.0],
                reach: reach_of(r.water_strength, water),
                has_water: r.water.is_some(),
                same,
            });
        }
    }
    if params.refraction.is_none() && params.reflection.is_none() {
        return None;
    }
    params.draw = scene.water.len() as u32;
    scene.water.push(crate::rendering::WaterDraw { refraction: refraction_tap, reflection: reflection_tap });
    Some(params)
}

/// One water slot of the composite's pass data (crate-internal): what kind of water (0 none, 1
/// ripples, 2 a height picture), the ripple header's words, the place, and how a picture's red
/// channel becomes height (`(red - bias) x scale`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WaterSlot {
    pub(crate) kind: u32,
    pub(crate) words: [u32; 4],
    pub(crate) place: [f32; 4],
    pub(crate) scale: f32,
    pub(crate) bias: f32,
}

/// Composite flags (u4.x) of the water part of the pass data.
pub(crate) const WATER_REFRACTION: u32 = 1;
/// Glints on.
pub(crate) const WATER_GLINTS: u32 = 2;
/// The caustic light on.
pub(crate) const WATER_CAUSTICS: u32 = 4;
/// A reflection.
pub(crate) const WATER_REFLECTION: u32 = 8;
/// The reflection's water is the refraction's (slot A).
pub(crate) const WATER_SAME: u32 = 16;

/// The water's words of the composite's pass data (v14 to v27, u3 to u5) into `fp`; `a` is the
/// refraction's water, `b` the reflection's own. See build.rs FxPassData for the layout.
pub(crate) fn water_pass_data(fp: &mut crate::rendering::core_fx_pass_data, w: &WaterParams, a: WaterSlot, b: WaterSlot) {
    let mut flags = 0u32;
    let m = w.to_water;
    fp.v14 = [m[0], m[1], m[2], m[3]];
    fp.v15 = [m[4], m[5], 0.0, 0.0];
    fp.v16 = w.to_device;
    if let Some(r) = &w.refraction {
        flags |= WATER_REFRACTION;
        fp.v17 = [r.strength, r.shading, 0.0, 1.0];
        fp.v18 = [r.light[0], r.light[1], r.light[2], 0.0];
        fp.v20[3] = r.reach;
        if let Some((color, amount, sharpness)) = r.glints {
            flags |= WATER_GLINTS;
            fp.v17[2] = amount;
            fp.v17[3] = sharpness;
            fp.v19 = [color[0], color[1], color[2], 0.0];
        }
        if let Some(k) = &r.caustics {
            flags |= WATER_CAUSTICS;
            fp.v15[2] = k.z;
            fp.v15[3] = k.frequency;
            fp.v18[3] = k.strength;
            fp.v19[3] = k.focus;
            fp.v20 = [k.color[0], k.color[1], k.color[2], r.reach];
            fp.u4[3] = k.seed;
        }
    }
    if let Some(r) = &w.reflection {
        flags |= WATER_REFLECTION;
        if r.same {
            flags |= WATER_SAME;
        }
        fp.v23 = [r.point[0], r.point[1], r.normal[0], r.normal[1]];
        fp.v24 = [r.opacity, r.fade, r.streak, r.water_strength];
        fp.v25 = r.tint;
        fp.v27 = [r.gap, r.reach, 0.0, 0.0];
    }
    fp.u3 = a.words;
    fp.v21 = a.place;
    fp.v22 = [a.scale, a.bias, b.scale, b.bias];
    fp.u5 = b.words;
    fp.v26 = b.place;
    fp.u4[0] = flags;
    fp.u4[1] = a.kind;
    fp.u4[2] = b.kind;
}
