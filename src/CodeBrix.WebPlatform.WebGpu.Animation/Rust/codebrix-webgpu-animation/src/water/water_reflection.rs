//! `WaterReflection`: a layer mirrored below a waterline from its own offscreen picture (no C#
//! equivalent).

use crate::water::WaterSource;
use crate::Color;

/// A mirrored copy of a layer below a WATERLINE, broken up by water and fading with distance (no
/// C# equivalent). Drawn FROM THE LAYER'S OWN PICTURE in its composite pass: nothing is recorded
/// or drawn twice. Set it with
/// [`DrawingLayer::set_reflection`](crate::models::DrawingLayer::set_reflection).
///
/// HOW IT IS DRAWN: through the layer's effect group, like a
/// [`WaterRefraction`](crate::water::WaterRefraction) (no extra nesting level when the layer has
/// one, one otherwise; refraction and reflection share it). The waterline `y = line` is in the
/// layer's DRAWING space (calibrated units under the camera at the layer's parallax - not the
/// layer's own transform), so a camera that pans or rolls moves and tilts it with the layer; the
/// mirror is the line `y = line + gap`. For a pixel `p` BELOW the waterline: the layer's content
/// is read at `mirror(p)` moved by the water (`J (water_strength x slope)` at `p`, as a
/// refraction), as the mean of 6 taps along the mirrored vertical over `streak` units, times
/// `opacity x (1 - smoothstep(0, fade_distance, distance below the line))`, its colour multiplied
/// by `mix(1, tint, tint alpha)`; the reflection goes UNDER the layer's content (what the layer
/// draws below the line covers its reflection). It mirrors the CONTENT, not the layer's glow or
/// shadow. The group's region grows by the mirrored part of the content box (the part above the
/// line), clipped to the canvas.
///
/// EXACT IN TIME as its water (see [`WaterSource`]).
///
/// LIMITS: a flat mirror of the layer's 2D picture - no parallax (the reflection does not show the
/// underside of things), no reflection of OTHER layers (put what must be reflected on one layer,
/// or give each layer a reflection with the same line); the waterline is horizontal in the scene
/// (tilted only by the camera's roll).
#[derive(Clone, Debug, PartialEq)]
pub struct WaterReflection {
    /// The waterline's y, in the layer's drawing space (calibrated units).
    pub line: f32,
    /// The mirror sits this much LOWER than the waterline (things set back from the shore), units.
    /// Default 0.
    pub gap: f32,
    /// The reflection's opacity at the waterline, 0..1. Default 0.6.
    pub opacity: f32,
    /// Units below the line where it has faded out, above 0. Default 300.
    pub fade_distance: f32,
    /// Multiplied into the reflection's colour; its alpha is how much (the water's colour).
    /// Default (40, 70, 90, 90).
    pub tint: Color,
    /// A vertical smear, units (still water 0; a breeze 6), at least 0. Default 3.
    pub streak: f32,
    /// What breaks the mirror up: ripples or a height picture; None (the default) for a still
    /// mirror.
    pub water: Option<WaterSource>,
    /// Water units the mirrored view moves per unit of slope. Default 10.
    pub water_strength: f32,
}

impl WaterReflection {
    /// A reflection about the waterline `line` with the defaults (gap 0, opacity 0.6, fade 300,
    /// tint (40, 70, 90, 90), streak 3, no water: a still mirror, water strength 10).
    pub fn new(line: f32) -> WaterReflection {
        WaterReflection { line, gap: 0.0, opacity: 0.6, fade_distance: 300.0, tint: Color::from_rgba(40, 70, 90, 90), streak: 3.0, water: None, water_strength: 10.0 }
    }

    /// Checks every value; the error names the one to change.
    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [("line", self.line), ("gap", self.gap), ("water_strength", self.water_strength)] {
            if !value.is_finite() {
                return Err(format!("A reflection's `{name}` must be a finite number (it is {value})."));
            }
        }
        if !(self.opacity >= 0.0 && self.opacity <= 1.0) {
            return Err(format!("A reflection's `opacity` must be 0..1 (it is {}).", self.opacity));
        }
        if !(self.fade_distance.is_finite() && self.fade_distance > 0.0) {
            return Err(format!("A reflection's `fade_distance` must be a finite number above 0 (it is {}).", self.fade_distance));
        }
        if !(self.streak.is_finite() && self.streak >= 0.0) {
            return Err(format!("A reflection's `streak` must be a finite number of at least 0 (it is {}).", self.streak));
        }
        if let Some(w) = &self.water {
            w.validate()?;
        }
        Ok(())
    }
}
