//! `LottieStroke`: a stroke of the draw list with everything kurbo's `Stroke` takes.

use crate::lottie::lottie_paint::LottiePaint;

/// How a stroke's ends look (Lottie `lc`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LottieLineCap {
    /// Lottie 1.
    #[default]
    Butt,
    /// Lottie 2.
    Round,
    /// Lottie 3.
    Square,
}

/// How a stroke's corners look (Lottie `lj`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LottieLineJoin {
    /// Lottie 1.
    #[default]
    Miter,
    /// Lottie 2.
    Round,
    /// Lottie 3.
    Bevel,
}

/// A stroke: paint, opacity, width, caps, joins and miter limit. Lottie stroke dashes are NOT
/// here: velato does not apply them (reported as
/// [`StrokeDashesIgnored`](crate::lottie::DrawingLottieWarningKind::StrokeDashesIgnored)).
#[derive(Clone, Debug, PartialEq)]
pub struct LottieStroke {
    /// The paint.
    pub paint: LottiePaint,
    /// The opacity, 0..1 (1 for gradients: their opacity is in the stops).
    pub opacity: f32,
    /// The width, in the path's own units.
    pub width: f64,
    /// The caps.
    pub cap: LottieLineCap,
    /// The joins.
    pub join: LottieLineJoin,
    /// The miter limit.
    pub miter_limit: f64,
}

impl LottieStroke {
    /// The stroke as kurbo's `Stroke` (what the renderer's `stroke` takes).
    pub fn to_kurbo(&self) -> kurbo::Stroke {
        kurbo::Stroke::new(self.width)
            .with_caps(match self.cap {
                LottieLineCap::Butt => kurbo::Cap::Butt,
                LottieLineCap::Round => kurbo::Cap::Round,
                LottieLineCap::Square => kurbo::Cap::Square,
            })
            .with_join(match self.join {
                LottieLineJoin::Miter => kurbo::Join::Miter,
                LottieLineJoin::Round => kurbo::Join::Round,
                LottieLineJoin::Bevel => kurbo::Join::Bevel,
            })
            .with_miter_limit(self.miter_limit)
    }

    /// How far the stroke can reach beyond its path, in the path's own units (half the width,
    /// times the miter limit for mitred joins and the square-cap diagonal otherwise).
    pub(crate) fn reach(&self) -> f64 {
        let factor = match self.join {
            LottieLineJoin::Miter => self.miter_limit.max(std::f64::consts::SQRT_2),
            _ => std::f64::consts::SQRT_2,
        };
        self.width * 0.5 * factor
    }
}
