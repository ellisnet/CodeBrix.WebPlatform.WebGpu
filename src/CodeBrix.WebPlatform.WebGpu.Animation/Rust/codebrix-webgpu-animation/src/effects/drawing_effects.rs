//! `DrawingEffects`: blur, glow and drop shadow (C# `DrawingEffects`).

use crate::color::Color;
use crate::effects::ShadowDirection;
use crate::geometry::PointF;

/// Pixel effects for an element, a group or a whole layer: a Gaussian BLUR of the content, an
/// outer GLOW behind it and a DROP SHADOW behind that (C# `DrawingEffects`). A value with
/// equality; build one with [`DrawingEffects::blur`], [`DrawingEffects::drop_shadow`] or
/// [`DrawingEffects::glow`] and combine with the `with_*` methods. Set it on an element
/// ([`DrawingShape::set_effects`](crate::shapes::DrawingShape::set_effects)) or a layer
/// ([`DrawingLayer::set_effects`](crate::models::DrawingLayer::set_effects)); animate the
/// parameters with bindings (`AnimatedProperty::BlurRadius`, `ShadowOffset`, `ShadowBlurRadius`,
/// `ShadowColor`, `GlowRadius`, `GlowColor`).
///
/// HOW THEY ARE MADE (C#'s order): the content is drawn into an offscreen target; (1) it is
/// blurred by `blur_radius`; (2) the glow is its coverage blurred by `glow_radius`, times
/// `glow_strength` (capped at full), in `glow_color`; (3) the shadow is its coverage blurred by
/// `shadow_blur_radius`, moved by `shadow_offset`, in `shadow_color`; the result is the content
/// over the glow over the shadow (each colour's alpha is that effect's opacity). A blur radius
/// follows the CSS convention: the Gaussian's standard deviation is HALF the radius.
///
/// SIZES FOLLOW THE CONTENT: radii and the shadow offset are in the element's OWN units - they
/// grow and shrink with its scale, its groups' and its layer's, the camera's zoom and the
/// canvas size, as its outline does (a radius by the square root of the area scale). For a
/// layer's effects the layer's transform (under the camera) plays the element's part. See
/// [`ShadowDirection`] for which way the shadow falls when the element turns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawingEffects {
    blur_radius: f32,
    shadow_color: Color,
    shadow_offset: PointF,
    shadow_direction: ShadowDirection,
    shadow_blur_radius: f32,
    glow_color: Color,
    glow_radius: f32,
    glow_strength: f32,
}

fn radius(value: f32, what: &str) -> Result<f32, String> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(format!("The {what} must be a finite number of 0 or more (it was {value})."))
    }
}

impl Default for DrawingEffects {
    fn default() -> DrawingEffects {
        DrawingEffects::NONE
    }
}

impl DrawingEffects {
    /// No effect at all (C# `None`).
    pub const NONE: DrawingEffects = DrawingEffects { blur_radius: 0.0, shadow_color: Color::TRANSPARENT, shadow_offset: PointF::new(0.0, 0.0), shadow_direction: ShadowDirection::FixedInScene, shadow_blur_radius: 0.0, glow_color: Color::TRANSPARENT, glow_radius: 0.0, glow_strength: 1.0 };

    /// A Gaussian blur of the content (C# `Blur(radius)`), the radius in the element's own units.
    pub fn blur(radius: f32) -> Result<DrawingEffects, String> {
        DrawingEffects::NONE.with_blur_radius(radius)
    }

    /// A drop shadow behind the content (C# `DropShadow(offset, blurRadius, color)`), fixed in
    /// the scene.
    pub fn drop_shadow(offset: PointF, blur_radius: f32, color: Color) -> Result<DrawingEffects, String> {
        DrawingEffects::NONE.with_shadow(offset, blur_radius, color)
    }

    /// A drop shadow with a choice of how its direction follows the element (C#
    /// `DropShadow(offset, blurRadius, color, direction)`).
    pub fn drop_shadow_with_direction(offset: PointF, blur_radius: f32, color: Color, direction: ShadowDirection) -> Result<DrawingEffects, String> {
        Ok(DrawingEffects::NONE.with_shadow(offset, blur_radius, color)?.with_shadow_direction(direction))
    }

    /// An outer glow behind the content (C# `Glow(radius, color, strength = 1)`).
    pub fn glow(radius: f32, color: Color, strength: f32) -> Result<DrawingEffects, String> {
        DrawingEffects::NONE.with_glow(radius, color, strength)
    }

    /// The content blur radius, element units (0 = sharp).
    pub fn blur_radius(&self) -> f32 {
        self.blur_radius
    }

    /// The shadow's colour; its alpha is the shadow's opacity (transparent = no shadow).
    pub fn shadow_color(&self) -> Color {
        self.shadow_color
    }

    /// How far the shadow is moved, element units (positive = right and down).
    pub fn shadow_offset(&self) -> PointF {
        self.shadow_offset
    }

    /// Whether the shadow's direction is fixed in the scene or turns with the element.
    pub fn shadow_direction(&self) -> ShadowDirection {
        self.shadow_direction
    }

    /// How soft the shadow is, element units (0 = a hard-edged copy).
    pub fn shadow_blur_radius(&self) -> f32 {
        self.shadow_blur_radius
    }

    /// The glow's colour; its alpha is the glow's opacity.
    pub fn glow_color(&self) -> Color {
        self.glow_color
    }

    /// How far the glow spreads, element units (0 = no glow).
    pub fn glow_radius(&self) -> f32 {
        self.glow_radius
    }

    /// The multiplier on the glow's blurred coverage, capped at full (1 by default).
    pub fn glow_strength(&self) -> f32 {
        self.glow_strength
    }

    /// True when the content is blurred.
    pub fn has_blur(&self) -> bool {
        self.blur_radius > 0.0
    }

    /// True when a shadow is drawn (its colour is not fully transparent).
    pub fn has_shadow(&self) -> bool {
        self.shadow_color.a > 0
    }

    /// True when a glow is drawn (a radius above 0, a colour not fully transparent, a strength
    /// above 0).
    pub fn has_glow(&self) -> bool {
        self.glow_radius > 0.0 && self.glow_strength > 0.0 && self.glow_color.a > 0
    }

    /// True when nothing would change the content.
    pub fn is_empty(&self) -> bool {
        !self.has_blur() && !self.has_shadow() && !self.has_glow()
    }

    /// The same effects with another content blur radius.
    pub fn with_blur_radius(&self, radius_value: f32) -> Result<DrawingEffects, String> {
        Ok(DrawingEffects { blur_radius: radius(radius_value, "blur radius")?, ..*self })
    }

    /// The same effects with another shadow (offset, softness, colour; the direction kept).
    pub fn with_shadow(&self, offset: PointF, blur_radius: f32, color: Color) -> Result<DrawingEffects, String> {
        if !(offset.x.is_finite() && offset.y.is_finite()) {
            return Err("The shadow offset must be finite.".to_string());
        }
        Ok(DrawingEffects { shadow_offset: offset, shadow_blur_radius: radius(blur_radius, "shadow blur radius")?, shadow_color: color, ..*self })
    }

    /// The same effects with the shadow's direction rule.
    pub fn with_shadow_direction(&self, direction: ShadowDirection) -> DrawingEffects {
        DrawingEffects { shadow_direction: direction, ..*self }
    }

    /// The same effects with another shadow offset.
    pub fn with_shadow_offset(&self, offset: PointF) -> Result<DrawingEffects, String> {
        self.with_shadow(offset, self.shadow_blur_radius, self.shadow_color)
    }

    /// The same effects with another shadow softness.
    pub fn with_shadow_blur_radius(&self, blur_radius: f32) -> Result<DrawingEffects, String> {
        self.with_shadow(self.shadow_offset, blur_radius, self.shadow_color)
    }

    /// The same effects with another shadow colour.
    pub fn with_shadow_color(&self, color: Color) -> DrawingEffects {
        DrawingEffects { shadow_color: color, ..*self }
    }

    /// The same effects with another glow (radius, colour, strength).
    pub fn with_glow(&self, radius_value: f32, color: Color, strength: f32) -> Result<DrawingEffects, String> {
        if !(strength.is_finite() && strength >= 0.0) {
            return Err(format!("The glow strength must be a finite number of 0 or more (it was {strength})."));
        }
        Ok(DrawingEffects { glow_radius: radius(radius_value, "glow radius")?, glow_color: color, glow_strength: strength, ..*self })
    }

    /// The same effects with another glow radius.
    pub fn with_glow_radius(&self, radius_value: f32) -> Result<DrawingEffects, String> {
        self.with_glow(radius_value, self.glow_color, self.glow_strength)
    }

    /// The same effects with another glow colour.
    pub fn with_glow_color(&self, color: Color) -> DrawingEffects {
        DrawingEffects { glow_color: color, ..*self }
    }

    /// The same effects with another glow strength.
    pub fn with_glow_strength(&self, strength: f32) -> Result<DrawingEffects, String> {
        self.with_glow(self.glow_radius, self.glow_color, strength)
    }
}
