//! The arithmetic of the effects (crate-internal): C#'s `EffectProcessor.Measure` (how big the
//! effects are on the device), the planes' Gaussian kernel (`PlaneKernels.GaussianKernel`), how
//! far an effect reaches, and the level of detail a blur is computed at on the GPU.

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
        (reach(self.blur_sigma) + outer) as f64
    }
}
