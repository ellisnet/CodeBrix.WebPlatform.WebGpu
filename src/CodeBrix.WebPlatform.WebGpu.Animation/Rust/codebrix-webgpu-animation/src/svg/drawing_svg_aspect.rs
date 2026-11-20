//! `DrawingSvgAspect`: how a document is fitted into its destination (C# `DrawingSvgAspect`).

use kurbo::Affine;

use crate::geometry::RectangleF;

/// How a document is fitted into its destination rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DrawingSvgAspect {
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

impl DrawingSvgAspect {
    /// The matrix from document coordinates (a `width` x `height` document) to the destination.
    pub fn fit(self, width: f64, height: f64, destination: RectangleF) -> Affine {
        let (x, y, dw, dh) = (destination.x as f64, destination.y as f64, destination.width as f64, destination.height as f64);
        if !(width > 0.0 && height > 0.0) {
            return Affine::translate((x, y));
        }
        match self {
            DrawingSvgAspect::None => Affine::translate((x, y)),
            DrawingSvgAspect::Stretch => Affine::translate((x, y)) * Affine::scale_non_uniform(dw / width, dh / height),
            DrawingSvgAspect::Meet | DrawingSvgAspect::Slice => {
                let s = if self == DrawingSvgAspect::Meet { (dw / width).min(dh / height) } else { (dw / width).max(dh / height) };
                Affine::translate((x + (dw - width * s) / 2.0, y + (dh - height * s) / 2.0)) * Affine::scale(s)
            }
        }
    }
}
