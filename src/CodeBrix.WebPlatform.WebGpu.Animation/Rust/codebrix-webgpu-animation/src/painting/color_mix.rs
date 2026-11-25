//! `ColorMix` and `ColorInterpolation`: mixing two colours in a chosen space - NoSkia's straight
//! sRGB (the default everywhere), linear light, or the perceptual Oklab and Oklch (no C#
//! equivalent) - and the crate's own Oklab conversions, which the stroke shader mirrors.

use peniko::color::{AlphaColor, ColorSpace, ColorSpaceTag, HueDirection, LinearSrgb, Oklab, Oklch, Srgb};

use crate::color::Color;

/// The space two colours are mixed in (no C# equivalent). [`ColorInterpolation::Srgb`] is
/// NoSkia's rule and the default of everything that mixes colours; the others are opt-in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorInterpolation {
    /// Each 8-bit sRGB channel, alpha too, straight (not premultiplied), rounded half away from
    /// zero - exactly `ColorRamp` and `ColorTrack` (NoSkia's rule). The default.
    #[default]
    Srgb,
    /// Linear light (sRGB primaries), premultiplied by alpha: physically even blending of light;
    /// mid tones come out lighter than in sRGB.
    LinearRgb,
    /// Oklab, premultiplied by alpha: perceptually even steps; two bright colours do not pass
    /// through grey (blue to yellow keeps its colourfulness through the middle).
    Oklab,
    /// Oklch (lightness, chroma, hue), premultiplied by alpha, the hue along the SHORTER arc;
    /// a grey end (no chroma) takes the other end's hue. Keeps chroma through the mix, so it can
    /// pass through hues neither end has (blue to yellow through green or pink, by the arc).
    Oklch,
}

impl ColorInterpolation {
    /// The renderer's gradient tag for the space (peniko's `Gradient::interpolation_cs`).
    pub(crate) fn to_color_space_tag(self) -> ColorSpaceTag {
        match self {
            ColorInterpolation::Srgb => ColorSpaceTag::Srgb,
            ColorInterpolation::LinearRgb => ColorSpaceTag::LinearSrgb,
            ColorInterpolation::Oklab => ColorSpaceTag::Oklab,
            ColorInterpolation::Oklch => ColorSpaceTag::Oklch,
        }
    }

    /// The space a gradient's tag asks for; every tag but the three opt-in ones is Srgb (what the
    /// ramps did before: nothing else in the crate sets another tag).
    pub(crate) fn from_color_space_tag(tag: ColorSpaceTag) -> ColorInterpolation {
        match tag {
            ColorSpaceTag::LinearSrgb => ColorInterpolation::LinearRgb,
            ColorSpaceTag::Oklab => ColorInterpolation::Oklab,
            ColorSpaceTag::Oklch => ColorInterpolation::Oklch,
            _ => ColorInterpolation::Srgb,
        }
    }
}

/// Mixing two colours in a chosen space (no C# equivalent): what a perceptual ramp, a gradient
/// with an interpolation or a colour track made with a function uses.
pub struct ColorMix;

impl ColorMix {
    /// The colour `t` of the way from `a` to `b` (t clamped to 0..1; NaN counts as 0) in a space.
    /// Srgb is NoSkia's channel rule exactly; the others go through the vendored `color` crate
    /// (peniko's) - premultiplied by alpha as CSS Color 4 interpolates, so a transparent end takes
    /// on the other end's colour instead of darkening the mix - and are rounded to 8 bits, every
    /// channel clamped to 0..255.
    pub fn mix(a: Color, b: Color, t: f32, space: ColorInterpolation) -> Color {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        match space {
            ColorInterpolation::Srgb => {
                let u = t as f64;
                Color::from_rgba(channel(a.r, b.r, u), channel(a.g, b.g, u), channel(a.b, b.b, u), channel(a.a, b.a, u))
            }
            _ => {
                let [r, g, bl, al] = mix_float(to_float(a), to_float(b), t, space);
                Color::from_rgba(to_byte(r), to_byte(g), to_byte(bl), to_byte(al))
            }
        }
    }
}

/// One 8-bit channel interpolated and rounded half away from zero: the formula of `ColorRamp`'s
/// and `ColorTrack`'s channel (painting/tests_color.rs checks the two agree).
fn channel(from: u8, to: u8, u: f64) -> u8 {
    let value = from as f64 + ((to as f64 - from as f64) * u);
    value.round().clamp(0.0, 255.0) as u8
}

/// Below this Oklch chroma a colour is grey and its hue means nothing.
const POWERLESS_CHROMA: f32 = 1.0e-4;

/// Two straight-alpha colours of 0..1 sRGB-encoded channels mixed `t` of the way in a space
/// (t <= 0 gives `a` and t >= 1 gives `b`, exactly; so does a segment between equal colours):
/// Srgb straight and unrounded; the others through the `color` crate, premultiplied, Oklch along
/// the shorter arc with a grey end taking the other end's hue. Every channel of the result is
/// clamped to 0..1. What [`ColorMix::mix`] and the gradient ramps (`make_ramp`) use.
pub(crate) fn mix_float(a: [f32; 4], b: [f32; 4], t: f32, space: ColorInterpolation) -> [f32; 4] {
    // The ends, and a segment between equal colours, are exact (no round trip through a space).
    let mixed = if a == b || t <= 0.0 {
        a
    } else if t >= 1.0 {
        b
    } else {
        let (pa, pb) = (AlphaColor::<Srgb>::new(a), AlphaColor::<Srgb>::new(b));
        match space {
            ColorInterpolation::Srgb => [a[0] + ((b[0] - a[0]) * t), a[1] + ((b[1] - a[1]) * t), a[2] + ((b[2] - a[2]) * t), a[3] + ((b[3] - a[3]) * t)],
            ColorInterpolation::LinearRgb => mix_in::<LinearSrgb>(pa, pb, t),
            ColorInterpolation::Oklab => mix_in::<Oklab>(pa, pb, t),
            ColorInterpolation::Oklch => {
                let mut ca = pa.convert::<Oklch>();
                let mut cb = pb.convert::<Oklch>();
                // A hue with no chroma is powerless (CSS Color 4): take the other end's.
                if ca.components[1] < POWERLESS_CHROMA {
                    ca.components[2] = cb.components[2];
                }
                if cb.components[1] < POWERLESS_CHROMA {
                    cb.components[2] = ca.components[2];
                }
                ca.lerp(cb, t, HueDirection::Shorter).convert::<Srgb>().components
            }
        }
    };
    mixed.map(|v| if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) })
}

fn mix_in<CS: ColorSpace>(a: AlphaColor<Srgb>, b: AlphaColor<Srgb>, t: f32) -> [f32; 4] {
    a.convert::<CS>().lerp(b.convert::<CS>(), t, HueDirection::Shorter).convert::<Srgb>().components
}

fn to_float(c: Color) -> [f32; 4] {
    [c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, c.a as f32 / 255.0]
}

/// A 0..1 value as an 8-bit channel: clamped, rounded half away from zero (NaN counts as 0).
pub(crate) fn to_byte(v: f32) -> u8 {
    if v.is_nan() {
        return 0;
    }
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

// ---- The crate's own Oklab (single precision) -------------------------------------------------
//
// THE CPU MIRROR of the stroke shader's colour maths (painterly package P3 writes the WGSL from
// these lines): the sRGB transfer curve, the two 3 x 3 matrices of Bjorn Ottosson's Oklab with
// the cube root between them (sign-preserving), and back - the same constants as the vendored
// `color` crate's Oklab, so the two agree within float rounding (painting/tests_color.rs).

const SRGB_TO_LMS: [[f32; 3]; 3] = [[0.412_221_46, 0.536_332_55, 0.051_445_995], [0.211_903_5, 0.680_699_5, 0.107_396_96], [0.088_302_46, 0.281_718_85, 0.629_978_7]];
const LMS_TO_LAB: [[f32; 3]; 3] = [[0.210_454_26, 0.793_617_8, -0.004_072_047], [1.977_998_5, -2.428_592_2, 0.450_593_7], [0.025_904_037, 0.782_771_77, -0.808_675_77]];
const LAB_TO_LMS: [[f32; 3]; 3] = [[1.0, 0.396_337_78, 0.215_803_76], [1.0, -0.105_561_346, -0.063_854_17], [1.0, -0.089_484_18, -1.291_485_5]];
const LMS_TO_SRGB: [[f32; 3]; 3] = [[4.076_741_7, -3.307_711_6, 0.230_969_94], [-1.268_438, 2.609_757_4, -0.341_319_38], [-0.004_196_086_3, -0.703_418_6, 1.707_614_7]];

fn mul(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [(m[0][0] * v[0]) + (m[0][1] * v[1]) + (m[0][2] * v[2]), (m[1][0] * v[0]) + (m[1][1] * v[1]) + (m[1][2] * v[2]), (m[2][0] * v[0]) + (m[2][1] * v[1]) + (m[2][2] * v[2])]
}

/// The sRGB transfer curve: an encoded channel (0..1) to linear light (sign-preserving beyond).
pub(crate) fn srgb_to_linear(x: f32) -> f32 {
    if x.abs() <= 0.04045 {
        x / 12.92
    } else {
        ((x.abs() + 0.055) / 1.055).powf(2.4).copysign(x)
    }
}

/// Linear light to an encoded sRGB channel (sign-preserving beyond 0..1).
pub(crate) fn linear_to_srgb(x: f32) -> f32 {
    if x.abs() <= 0.003_130_8 {
        x * 12.92
    } else {
        ((1.055 * x.abs().powf(1.0 / 2.4)) - 0.055).copysign(x)
    }
}

/// Encoded sRGB (0..1 per channel, as `DrawingPlaneImage` and `get_mean_colors` hold colour) to
/// Oklab `[L, a, b]` (L 0..1).
pub(crate) fn srgb_to_oklab(rgb: [f32; 3]) -> [f32; 3] {
    let lms = mul(&SRGB_TO_LMS, rgb.map(srgb_to_linear)).map(f32::cbrt);
    mul(&LMS_TO_LAB, lms)
}

/// Oklab `[L, a, b]` to encoded sRGB (0..1 per channel; NOT clamped - a colour outside the sRGB
/// gamut comes back with channels outside 0..1).
pub(crate) fn oklab_to_srgb(lab: [f32; 3]) -> [f32; 3] {
    let lms = mul(&LAB_TO_LMS, lab).map(|x| x * x * x);
    mul(&LMS_TO_SRGB, lms).map(linear_to_srgb)
}

/// A colour's Oklab `[L, a, b]`.
pub(crate) fn color_to_oklab(c: Color) -> [f32; 3] {
    srgb_to_oklab([c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0])
}
