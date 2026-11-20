//! `LottieBlendMode`: how a group of the draw list is composited onto what is below it.

use peniko::{BlendMode, Compose, Mix};

/// How a [`LottieDrawCommand::PushGroup`](crate::lottie::LottieDrawCommand::PushGroup) is
/// composited onto what was drawn before it inside its parent group.
///
/// `Normal` is source-over. The sixteen separable and non-separable modes are a top-level layer's
/// `bm` (Lottie numbers 0 to 15); `Add` is Lottie's "add" (16, linear dodge: the colours summed).
/// `SourceIn` and `SourceOut` are what a track matte asks for: the group's content kept only
/// where (SourceIn) or only where not (SourceOut) the matte drawn just before it covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LottieBlendMode {
    /// Source-over (Lottie 0).
    #[default]
    Normal,
    /// Lottie 1.
    Multiply,
    /// Lottie 2.
    Screen,
    /// Lottie 3.
    Overlay,
    /// Lottie 4.
    Darken,
    /// Lottie 5.
    Lighten,
    /// Lottie 6.
    ColorDodge,
    /// Lottie 7.
    ColorBurn,
    /// Lottie 8.
    HardLight,
    /// Lottie 9.
    SoftLight,
    /// Lottie 10.
    Difference,
    /// Lottie 11.
    Exclusion,
    /// Lottie 12.
    Hue,
    /// Lottie 13.
    Saturation,
    /// Lottie 14.
    Color,
    /// Lottie 15.
    Luminosity,
    /// Lottie 16 ("add", linear dodge): source plus destination, clamped.
    Add,
    /// A track matte (alpha, and luma drawn as alpha): the content where the matte covers.
    SourceIn,
    /// An inverted track matte: the content where the matte does not cover.
    SourceOut,
}

impl LottieBlendMode {
    /// The mode as peniko's blend mode (what the renderer's `push_layer` takes).
    pub fn to_peniko(self) -> BlendMode {
        match self {
            LottieBlendMode::Normal => Mix::Normal.into(),
            LottieBlendMode::Multiply => Mix::Multiply.into(),
            LottieBlendMode::Screen => Mix::Screen.into(),
            LottieBlendMode::Overlay => Mix::Overlay.into(),
            LottieBlendMode::Darken => Mix::Darken.into(),
            LottieBlendMode::Lighten => Mix::Lighten.into(),
            LottieBlendMode::ColorDodge => Mix::ColorDodge.into(),
            LottieBlendMode::ColorBurn => Mix::ColorBurn.into(),
            LottieBlendMode::HardLight => Mix::HardLight.into(),
            LottieBlendMode::SoftLight => Mix::SoftLight.into(),
            LottieBlendMode::Difference => Mix::Difference.into(),
            LottieBlendMode::Exclusion => Mix::Exclusion.into(),
            LottieBlendMode::Hue => Mix::Hue.into(),
            LottieBlendMode::Saturation => Mix::Saturation.into(),
            LottieBlendMode::Color => Mix::Color.into(),
            LottieBlendMode::Luminosity => Mix::Luminosity.into(),
            LottieBlendMode::Add => Compose::Plus.into(),
            LottieBlendMode::SourceIn => Compose::SrcIn.into(),
            LottieBlendMode::SourceOut => Compose::SrcOut.into(),
        }
    }

    /// The mode of a layer's `bm` number; None for 17 (hard mix, which has no compositing
    /// equivalent here) and for numbers Lottie does not define.
    pub(crate) fn from_lottie(bm: u64) -> Option<LottieBlendMode> {
        Some(match bm {
            0 => LottieBlendMode::Normal,
            1 => LottieBlendMode::Multiply,
            2 => LottieBlendMode::Screen,
            3 => LottieBlendMode::Overlay,
            4 => LottieBlendMode::Darken,
            5 => LottieBlendMode::Lighten,
            6 => LottieBlendMode::ColorDodge,
            7 => LottieBlendMode::ColorBurn,
            8 => LottieBlendMode::HardLight,
            9 => LottieBlendMode::SoftLight,
            10 => LottieBlendMode::Difference,
            11 => LottieBlendMode::Exclusion,
            12 => LottieBlendMode::Hue,
            13 => LottieBlendMode::Saturation,
            14 => LottieBlendMode::Color,
            15 => LottieBlendMode::Luminosity,
            16 => LottieBlendMode::Add,
            _ => return None,
        })
    }

    /// The mode of a peniko blend mode velato asked for; None when it is none of these.
    pub(crate) fn from_peniko(blend: BlendMode) -> Option<LottieBlendMode> {
        match blend.compose {
            Compose::SrcIn => return Some(LottieBlendMode::SourceIn),
            Compose::SrcOut => return Some(LottieBlendMode::SourceOut),
            Compose::Plus => return Some(LottieBlendMode::Add),
            Compose::SrcOver => {}
            _ => return None,
        }
        Some(match blend.mix {
            Mix::Normal => LottieBlendMode::Normal,
            Mix::Multiply => LottieBlendMode::Multiply,
            Mix::Screen => LottieBlendMode::Screen,
            Mix::Overlay => LottieBlendMode::Overlay,
            Mix::Darken => LottieBlendMode::Darken,
            Mix::Lighten => LottieBlendMode::Lighten,
            Mix::ColorDodge => LottieBlendMode::ColorDodge,
            Mix::ColorBurn => LottieBlendMode::ColorBurn,
            Mix::HardLight => LottieBlendMode::HardLight,
            Mix::SoftLight => LottieBlendMode::SoftLight,
            Mix::Difference => LottieBlendMode::Difference,
            Mix::Exclusion => LottieBlendMode::Exclusion,
            Mix::Hue => LottieBlendMode::Hue,
            Mix::Saturation => LottieBlendMode::Saturation,
            Mix::Color => LottieBlendMode::Color,
            Mix::Luminosity => LottieBlendMode::Luminosity,
        })
    }
}
