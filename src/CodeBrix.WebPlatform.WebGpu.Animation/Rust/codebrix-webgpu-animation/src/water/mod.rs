//! WATER for 2D scenes (no C# equivalent): a [`RippleField`] - the water's height and slope over
//! a rectangle at any session time, FROM A FORMULA: [`RippleDrop`]s (and [`RippleRain`]) making
//! rings that spread and fade, over an optional [`RippleSwell`] of [`SwellWave`]s, inside a
//! rectangle whose edges can reflect them ([`RippleOptions::walls`]).
//!
//! Like everything this crate draws, the water is a FUNCTION OF WHAT IS PASSED IN, the time
//! included: nothing is simulated step by step, rain is drawn per session second from a seed,
//! so pause freezes it, seeking to any time shows exactly the water that playing to it shows,
//! and frames may be drawn in any order. (The simulated, pointer-stirred water is the Graphics
//! package's wave field; a picture of it can be handed to this crate by the application.)
//!
//! The field's height, slope and focus (the Laplacian, where light gathers) are readable on the
//! CPU ([`RippleField::height_at`], [`RippleField::slope_at`], [`RippleField::focus_at`]) with the
//! same single-precision arithmetic the GPU uses, so things can float on the water without any
//! readback. A caustic light pattern for pool floors is the noise kind
//! [`DrawingNoiseKind::Caustics`](crate::planes::DrawingNoiseKind).
//!
//! A layer's (or a whole session's) picture seen THROUGH that water: [`WaterRefraction`] (the
//! picture bent by the water's slope, shaded, with [`WaterGlints`] and a [`CausticLight`]); and a
//! layer MIRRORED below a waterline, broken up by the water and fading with distance:
//! [`WaterReflection`]. Both are drawn from the layer's own offscreen picture in its effect
//! group's composite (nothing is drawn twice; no extra nesting level when the layer already has an
//! effect group). The water is a [`WaterSource`]: a ripple field (evaluated per pixel on the GPU,
//! the same arithmetic as its CPU twin), or a height picture - decoded pixels, a picture target
//! drawn this frame, or an application's GPU texture (the Graphics package's simulated wave field,
//! for example; such a scene cannot be rewound).

mod ripple_field;
pub(crate) mod ripple_math;
mod ripple_rain;
mod ripple_swell;
pub(crate) mod water_reflection;
pub(crate) mod water_refraction;
pub(crate) mod water_source;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_water_gpu;

pub use ripple_field::{RippleDrop, RippleField, RippleOptions};
pub use ripple_rain::RippleRain;
pub use ripple_swell::{RippleSwell, SwellWave};
pub use water_reflection::WaterReflection;
pub use water_refraction::{CausticLight, WaterGlints, WaterRefraction};
pub use water_source::WaterSource;
