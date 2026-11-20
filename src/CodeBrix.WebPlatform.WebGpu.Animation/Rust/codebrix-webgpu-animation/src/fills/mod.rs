//! The fills of CodeBrix.Imaging.Drawing's `Fills` namespace: colour stops and gradient fills.

pub(crate) mod color_stop;
mod gradient_spread;
mod linear_gradient_fill;
mod radial_gradient_fill;
mod shape_fill;

pub use color_stop::ColorStop;
pub use gradient_spread::GradientSpread;
pub use linear_gradient_fill::LinearGradientFill;
pub use radial_gradient_fill::RadialGradientFill;
pub use shape_fill::ShapeFill;
