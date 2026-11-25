//! `ShapeFill`: what paints a filled shape instead of a flat colour (C# `ShapeFill`, whose
//! derived classes are the gradient fills).

use peniko::{Brush, ColorStops, Gradient, InterpolationAlphaSpace};

use crate::fills::linear_gradient_fill::LinearGradientFill;
use crate::fills::radial_gradient_fill::RadialGradientFill;

/// A fill that paints a shape (C# `ShapeFill`; its kinds are the gradient fills).
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeFill {
    /// A linear gradient.
    Linear(LinearGradientFill),
    /// A radial gradient.
    Radial(RadialGradientFill),
}

impl From<LinearGradientFill> for ShapeFill {
    fn from(fill: LinearGradientFill) -> ShapeFill {
        ShapeFill::Linear(fill)
    }
}

impl From<RadialGradientFill> for ShapeFill {
    fn from(fill: RadialGradientFill) -> ShapeFill {
        ShapeFill::Radial(fill)
    }
}

impl ShapeFill {
    /// The renderer's brush for this fill in calibrated coordinates (the shape's transform maps
    /// them to the canvas), with the fill's interpolation (straight sRGB, NoSkia's, unless the
    /// fill chose another; carried as the gradient's `interpolation_cs`, which `make_ramp`
    /// honours) and `opacity` applied.
    pub(crate) fn to_brush(&self, opacity: f32) -> Brush {
        let (gradient, stops, space) = match self {
            ShapeFill::Linear(l) => (Gradient::new_linear((l.start().x as f64, l.start().y as f64), (l.end().x as f64, l.end().y as f64)).with_extend(l.spread().to_extend()), l.stops(), l.interpolation()),
            ShapeFill::Radial(r) => (Gradient::new_radial((r.center().x as f64, r.center().y as f64), r.radius()).with_extend(r.spread().to_extend()), r.stops(), r.interpolation()),
        };
        let gradient = gradient.with_interpolation_cs(space.to_color_space_tag());
        let mut list = ColorStops::new();
        for stop in stops {
            list.0.push(peniko::ColorStop::from((stop.position(), stop.color().to_peniko(opacity))));
        }
        Brush::Gradient(Gradient { stops: list, interpolation_alpha_space: InterpolationAlphaSpace::Unpremultiplied, ..gradient })
    }
}
