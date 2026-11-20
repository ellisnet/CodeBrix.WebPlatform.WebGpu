//! `LottiePaint` and its gradients: what a fill or a stroke of the draw list paints with.

use std::sync::Arc;

use crate::color::Color;

/// One gradient stop: offset 0..1 and a straight-alpha sRGB colour. The alpha carries the stop's
/// own opacity TIMES the opacity of everything above the draw (layers, groups, the fill or
/// stroke), because velato multiplies them into the gradient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LottieGradientStop {
    /// The offset, 0..1.
    pub offset: f32,
    /// The colour with its alpha.
    pub color: Color,
}

/// A linear gradient in the painted path's own coordinates (pad at both ends).
#[derive(Clone, Debug, PartialEq)]
pub struct LottieLinearGradient {
    /// The start point (Lottie `s`).
    pub start: (f64, f64),
    /// The end point (Lottie `e`).
    pub end: (f64, f64),
    /// The stops.
    pub stops: Vec<LottieGradientStop>,
}

/// A radial gradient in the painted path's own coordinates: a circle about `center` whose
/// radius is the distance from Lottie's start point to its end point (pad beyond it).
#[derive(Clone, Debug, PartialEq)]
pub struct LottieRadialGradient {
    /// The centre (Lottie `s`).
    pub center: (f64, f64),
    /// The radius.
    pub radius: f64,
    /// The stops.
    pub stops: Vec<LottieGradientStop>,
}

/// What a fill or a stroke paints with.
#[derive(Clone, Debug, PartialEq)]
pub enum LottiePaint {
    /// A solid colour, opaque (alpha 255): its opacity is the command's `opacity`.
    Solid(Color),
    /// A linear gradient (the command's `opacity` is 1: the opacity is in the stops).
    Linear(Arc<LottieLinearGradient>),
    /// A radial gradient (the command's `opacity` is 1: the opacity is in the stops).
    Radial(Arc<LottieRadialGradient>),
}
