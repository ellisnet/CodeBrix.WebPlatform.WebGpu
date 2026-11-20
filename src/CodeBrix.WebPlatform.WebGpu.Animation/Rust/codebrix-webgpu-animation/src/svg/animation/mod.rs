//! The document's own animations: SMIL timing and values, the CSS subset, and their evaluation
//! into per-part overrides (our own code: no crate evaluates SMIL or CSS animations).

pub(crate) mod css_animation;
pub(crate) mod smil_animation;
pub(crate) mod smil_timing;
pub(crate) mod svg_animations;
pub(crate) mod svg_value;

pub use svg_animations::SvgAnimations;
