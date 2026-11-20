//! `DrawingLottieAspect`: how a document is fitted into its destination (the same four fits as
//! the `svg` module's aspect).

use kurbo::Affine;

use crate::geometry::RectangleF;

/// How a document is fitted into its destination rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DrawingLottieAspect {
    /// Fit inside, centred (the default).
    #[default]
    Meet,
    /// Cover, centred, clipped to the destination.
    Slice,
    /// Each axis to the destination.
    Stretch,
    /// One document unit = one calibrated unit, at the destination's corner.
    None,
}

impl DrawingLottieAspect {
    /// The matrix from document coordinates (a `width` x `height` document) to the destination.
    pub fn fit(self, width: f64, height: f64, destination: RectangleF) -> Affine {
        let (x, y, dw, dh) = (destination.x as f64, destination.y as f64, destination.width as f64, destination.height as f64);
        if !(width > 0.0 && height > 0.0) {
            return Affine::translate((x, y));
        }
        match self {
            DrawingLottieAspect::None => Affine::translate((x, y)),
            DrawingLottieAspect::Stretch => Affine::translate((x, y)) * Affine::scale_non_uniform(dw / width, dh / height),
            DrawingLottieAspect::Meet | DrawingLottieAspect::Slice => {
                let s = if self == DrawingLottieAspect::Meet { (dw / width).min(dh / height) } else { (dw / width).max(dh / height) };
                Affine::translate((x + (dw - width * s) / 2.0, y + (dh - height * s) / 2.0)) * Affine::scale(s)
            }
        }
    }
}
