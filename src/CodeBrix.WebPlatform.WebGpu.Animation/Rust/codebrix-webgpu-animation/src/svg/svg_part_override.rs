//! `SvgPartOverride` and `SvgAnimationFrame`: what a document's OWN animations (SMIL and the CSS
//! subset) say about each part at one time - replacements for the element's attributes, applied
//! while the scene is walked. The loaded scene is never changed.

use std::collections::HashMap;

use kurbo::Affine;

use crate::color::Color;

/// The animated values of one element at one time; None everywhere = as loaded.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SvgPartOverride {
    /// Replaces the element's own `transform` attribute (in its parent's coordinates):
    /// `animateTransform` and the CSS `transform` property.
    pub transform: Option<Affine>,
    /// `animateMotion`'s supplemental transform, applied on top of (outside) the element's
    /// transform: parent x motion x transform.
    pub motion: Option<Affine>,
    /// Replaces the element's `opacity`.
    pub opacity: Option<f32>,
    /// Replaces the fill paint of the element's paths: Some(None) = `none`.
    pub fill: Option<Option<Color>>,
    /// Replaces `fill-opacity`.
    pub fill_opacity: Option<f32>,
    /// Replaces the stroke paint: Some(None) = `none`.
    pub stroke: Option<Option<Color>>,
    /// Replaces `stroke-opacity`.
    pub stroke_opacity: Option<f32>,
    /// Replaces `stroke-width`.
    pub stroke_width: Option<f32>,
    /// Replaces `stroke-dashoffset`.
    pub stroke_dash_offset: Option<f32>,
    /// Replaces `stroke-dasharray`: Some(None) = `none`.
    pub stroke_dash_array: Option<Option<Vec<f32>>>,
    /// `display`: Some(false) hides the element and everything in it (nothing inside can show
    /// itself again); Some(true) undoes nothing that was not hidden by an animation.
    pub visible: Option<bool>,
    /// `visibility` (an INHERITED property, as SVG has it): Some(false) hides the element's paths
    /// and images and those of its descendants that do not set their own visibility; Some(true)
    /// shows them, also those loaded hidden. A descendant that sets `visibility` itself keeps it.
    pub visibility: Option<bool>,
}

impl SvgPartOverride {
    /// True when nothing is overridden.
    pub fn is_empty(&self) -> bool {
        *self == SvgPartOverride::default()
    }
}

/// Every part's override at one time (the result of
/// [`DrawingSvg::evaluate_animations`](crate::svg::DrawingSvg::evaluate_animations)).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SvgAnimationFrame {
    /// The document time the frame was evaluated at, in seconds.
    pub time: f64,
    /// The overrides by part id (parts with nothing animated are absent).
    pub parts: HashMap<String, SvgPartOverride>,
}

impl SvgAnimationFrame {
    /// The override of one part, if it has one.
    pub fn get(&self, id: &str) -> Option<&SvgPartOverride> {
        self.parts.get(id)
    }
}
