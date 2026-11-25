//! `WaterRefraction`, `WaterGlints` and `CausticLight`: a layer's (or a whole session's) picture
//! seen THROUGH WATER (no C# equivalent).

use crate::water::WaterSource;
use crate::Color;

/// A picture seen THROUGH WATER (no C# equivalent): every pixel of the layer (or of the session,
/// [`SessionRenderOptions::refraction`](crate::SessionRenderOptions::refraction)) is taken from
/// where the water's slope bends the view, shaded by the slope, with optional glints and a
/// caustic light. Set it with
/// [`DrawingLayer::set_refraction`](crate::models::DrawingLayer::set_refraction).
///
/// HOW IT IS DRAWN: through the layer's EFFECT GROUP - the one its effects, frame operations,
/// transition and surface finish use: no extra nesting level when the layer already has one, one
/// level when it has none. In that group's composite, per pixel `p` (device pixels): the water's
/// slope `s` at the pixel's place in the water's coordinates (see [`WaterSource`]); the content is
/// read at `p + J (strength s)` (`J` the water-to-device map; the offset is limited to the
/// group's grown region, at most 128 device pixels), then
/// - shading `1 + shading (dot(n, l) - l.z)` with `n = normalize(-s, 1)` and `l` towards the light
///   (flat water: exactly 1);
/// - glints `amount pow(max(dot(reflect(-l, n), (0, 0, 1)), 0), sharpness)` of their colour,
///   added where the content is drawn;
/// - the caustic light: the [caustics](crate::planes::DrawingNoiseKind::Caustics) pattern at the
///   water point times the frequency and at `z = time x speed`, as `(v + 1) / 2`, gathered by the
///   water's curvature (`max(0, 1 - focus x strength x laplacian)`: light concentrates under
///   crests), lighting the content's own colour (`rgb += rgb x colour x caustic strength x light`);
///
/// then the layer's surface finish, reflection, frame operations, bloom, glow, shadow, transition
/// and alpha as without water. The group's region grows by the largest offset (the strength times
/// an upper bound of the water's slope) so bent content near its edge is not cut.
///
/// EXACT IN TIME with a ripple field or a picture (see [`WaterSource`]); with an application's
/// texture fed by a simulation the scene CANNOT BE REWOUND.
///
/// LIMITS: a screen-space bend of the layer's own picture - nothing outside the layer is seen
/// through it (a session's refraction sees only that session); not a lens (no magnification by
/// depth); layers and sessions only (a group of elements to bend goes on its own layer). It stays
/// under the LINE and FLAT paint looks and with effects switched off (it is a pass, like the frame
/// operations).
#[derive(Clone, Debug, PartialEq)]
pub struct WaterRefraction {
    /// The water.
    pub source: WaterSource,
    /// Water units the view moves per unit of slope (0: no bend). Default 12.
    pub strength: f32,
    /// How much slopes facing the light brighten and the others darken (0: no shading). Default
    /// 0.3.
    pub shading: f32,
    /// Degrees clockwise from +x on the screen the light comes FROM (225 = upper left). Default
    /// 225.
    pub light_angle: f32,
    /// Degrees above the water, in (0, 90]. Default 45.
    pub light_elevation: f32,
    /// Sun glints on the slopes; None (the default) for none.
    pub glints: Option<WaterGlints>,
    /// The caustic light on what is seen through the water; None (the default) for none.
    pub caustics: Option<CausticLight>,
}

/// Sun glints on a refraction's water (no C# equivalent).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterGlints {
    /// How bright, at least 0. Default 0.8.
    pub amount: f32,
    /// The highlight's exponent (higher: smaller, sharper glints), above 0. Default 80.
    pub sharpness: f32,
    /// Their colour (its alpha is not used). Default warm white (255, 245, 225).
    pub color: Color,
}

impl Default for WaterGlints {
    fn default() -> WaterGlints {
        WaterGlints { amount: 0.8, sharpness: 80.0, color: Color::from_rgb(255, 245, 225) }
    }
}

/// The caustic light a refraction throws on what is seen through it (no C# equivalent): the
/// [caustics](crate::planes::DrawingNoiseKind::Caustics) noise pattern - the same function, drawn
/// per pixel - moving with the time and gathered by the water's curvature.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CausticLight {
    /// How strongly it lights the content's colour, at least 0. Default 0.5.
    pub strength: f32,
    /// Pattern cells per water unit, above 0. Default 0.02 (one cell per 50 units).
    pub frequency: f32,
    /// How fast the net drifts (its z is the session time times this). Default 0.25.
    pub speed: f32,
    /// How much the water's curvature gathers the light under crests (0: the plain pattern), at
    /// least 0. Default 0.5.
    pub focus: f32,
    /// The pattern's seed. Default 0.
    pub seed: i64,
    /// The light's colour (its alpha is not used). Default pale (235, 248, 255).
    pub color: Color,
}

impl Default for CausticLight {
    fn default() -> CausticLight {
        CausticLight { strength: 0.5, frequency: 0.02, speed: 0.25, focus: 0.5, seed: 0, color: Color::from_rgb(235, 248, 255) }
    }
}

impl WaterRefraction {
    /// A refraction through `source` with the defaults (strength 12, shading 0.3, light from 225
    /// degrees at 45; glints and caustics off).
    pub fn new(source: WaterSource) -> WaterRefraction {
        WaterRefraction { source, strength: 12.0, shading: 0.3, light_angle: 225.0, light_elevation: 45.0, glints: None, caustics: None }
    }

    /// Checks every value; the error names the one to change.
    pub fn validate(&self) -> Result<(), String> {
        self.source.validate()?;
        for (name, value) in [("strength", self.strength), ("shading", self.shading), ("light_angle", self.light_angle)] {
            if !value.is_finite() {
                return Err(format!("A refraction's `{name}` must be a finite number (it is {value})."));
            }
        }
        if !(self.light_elevation > 0.0 && self.light_elevation <= 90.0) {
            return Err(format!("A refraction's `light_elevation` must be above 0 and at most 90 degrees (it is {}).", self.light_elevation));
        }
        if let Some(g) = &self.glints {
            if !(g.amount.is_finite() && g.amount >= 0.0) {
                return Err(format!("Water glints' `amount` must be a finite number of at least 0 (it is {}).", g.amount));
            }
            if !(g.sharpness.is_finite() && g.sharpness > 0.0) {
                return Err(format!("Water glints' `sharpness` must be a finite number above 0 (it is {}).", g.sharpness));
            }
        }
        if let Some(c) = &self.caustics {
            if !(c.strength.is_finite() && c.strength >= 0.0) {
                return Err(format!("A caustic light's `strength` must be a finite number of at least 0 (it is {}).", c.strength));
            }
            if !(c.frequency.is_finite() && c.frequency > 0.0) {
                return Err(format!("A caustic light's `frequency` must be a finite number above 0 (it is {}).", c.frequency));
            }
            if !c.speed.is_finite() {
                return Err(format!("A caustic light's `speed` must be a finite number (it is {}).", c.speed));
            }
            if !(c.focus.is_finite() && c.focus >= 0.0) {
                return Err(format!("A caustic light's `focus` must be a finite number of at least 0 (it is {}).", c.focus));
            }
        }
        Ok(())
    }
}

/// The direction TOWARDS a light that comes from `angle` degrees (clockwise from +x on the
/// screen, y down) at `elevation` degrees above the water (crate-internal; the finish's rule).
pub(crate) fn light_vector(angle: f32, elevation: f32) -> [f32; 3] {
    let (a, e) = ((angle as f64).to_radians(), (elevation as f64).to_radians());
    [(a.cos() * e.cos()) as f32, (a.sin() * e.cos()) as f32, e.sin() as f32]
}
