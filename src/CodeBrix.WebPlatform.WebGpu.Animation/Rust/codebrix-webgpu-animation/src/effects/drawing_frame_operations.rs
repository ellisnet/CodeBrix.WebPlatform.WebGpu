//! `DrawingFrameOperations`: the GPU versions of the planes' whole-frame operations, applied to a
//! layer as it is drawn (no C# type: C# applies them to a plane image on the CPU).

/// A vignette (the planes' `apply_vignette`): the colour multiplied by
/// `clamp(1 - strength * (dx^2 + dy^2), minimum, 1)` with `dx = (x - center_x) / radius_x` and
/// `dy = (y - center_y) / radius_y` - here in the session's calibrated units, at each pixel's
/// centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameVignette {
    /// The centre, calibrated units.
    pub center_x: f32,
    /// The centre, calibrated units.
    pub center_y: f32,
    /// The horizontal radius, calibrated units (more than 0).
    pub radius_x: f32,
    /// The vertical radius, calibrated units (more than 0).
    pub radius_y: f32,
    /// How strongly the edges darken.
    pub strength: f32,
    /// The least the colour is multiplied by (0 by default in C#).
    pub minimum: f32,
}

/// A bloom (the planes' `create_bloom` added back onto the frame): the layer's content blurred by
/// a Gaussian of `sigma` calibrated units, multiplied by `gain`, ADDED to the content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameBloom {
    /// The blur's standard deviation, calibrated units (more than 0).
    pub sigma: f32,
    /// What the blurred light is multiplied by before it is added.
    pub gain: f32,
}

/// The whole-frame operations of the pixel planes as GPU PASSES over a layer: a tone curve, a
/// gamma, a vignette and a bloom, with the planes' formulas and parameters (values within float
/// rounding of the CPU's: the GPU works in 32-bit floats on the layer as it is drawn, the CPU in
/// a plane). Set them on a layer with
/// [`DrawingLayer::set_frame_operations`](crate::models::DrawingLayer::set_frame_operations); a
/// real-time scene uses these instead of the CPU plane operations, which cost milliseconds to tens
/// of milliseconds per whole frame. The colour operations apply to the STRAIGHT sRGB-encoded
/// colour (the values a plane image holds), alpha left alone, in the order tone curve, gamma,
/// vignette; the bloom is added after them. The layer is drawn offscreen once for them (as for
/// effects); on a layer that also has a glow, the bloom is not drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawingFrameOperations {
    /// `v / (1 + k v) * gain` (the planes' `apply_tone_curve(k, gain)`), or None.
    pub tone_curve: Option<(f32, f32)>,
    /// `pow(v, exponent)` for v above 0 (the planes' `apply_gamma`), or None.
    pub gamma: Option<f32>,
    /// A vignette, or None.
    pub vignette: Option<FrameVignette>,
    /// A bloom, or None.
    pub bloom: Option<FrameBloom>,
}

impl DrawingFrameOperations {
    /// No operation.
    pub const NONE: DrawingFrameOperations = DrawingFrameOperations { tone_curve: None, gamma: None, vignette: None, bloom: None };

    /// A copy with a tone curve `v / (1 + k v) * gain`.
    pub fn with_tone_curve(mut self, k: f32, gain: f32) -> DrawingFrameOperations {
        self.tone_curve = Some((k, gain));
        self
    }

    /// A copy with a gamma (the error says what to do with an exponent that is not positive).
    pub fn with_gamma(mut self, exponent: f32) -> Result<DrawingFrameOperations, String> {
        if !(exponent > 0.0) || !exponent.is_finite() {
            return Err(format!("A gamma exponent must be a finite number above 0 (it was {exponent})."));
        }
        self.gamma = Some(exponent);
        Ok(self)
    }

    /// A copy with a vignette (the error says what to do with a radius that is not positive).
    pub fn with_vignette(mut self, vignette: FrameVignette) -> Result<DrawingFrameOperations, String> {
        if !(vignette.radius_x > 0.0) || !(vignette.radius_y > 0.0) {
            return Err("A vignette's radii must be above 0.".to_string());
        }
        self.vignette = Some(vignette);
        Ok(self)
    }

    /// A copy with a bloom (the error says what to do with a sigma that is not positive).
    pub fn with_bloom(mut self, sigma: f32, gain: f32) -> Result<DrawingFrameOperations, String> {
        if !(sigma > 0.0) || !sigma.is_finite() {
            return Err(format!("A bloom's sigma must be a finite number above 0 (it was {sigma})."));
        }
        self.bloom = Some(FrameBloom { sigma, gain });
        Ok(self)
    }

    /// True when nothing would change the layer.
    pub fn is_empty(&self) -> bool {
        self.tone_curve.is_none() && self.gamma.is_none() && self.vignette.is_none() && self.bloom.is_none()
    }
}
