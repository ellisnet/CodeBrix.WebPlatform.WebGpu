//! `DrawingAnimations`: the bindings of a session - keyframe tracks and time functions bound to
//! its elements, layers and camera, applied whenever the session's time is set (C#
//! `DrawingAnimations`).

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::effects::DrawingEffects;
use crate::geometry::PointF;
use crate::models::binding_combine::{CombineSlot, RestEntry, SessionDriver};
use crate::models::bound_track::BoundValue;
use crate::models::{AnimatedProperty, AnimationBinding, BindingTarget, BoundTrack, DrawingCamera, DrawingLayer, DrawingTransform, PathTravelOptions};
use crate::paths::PathGeometry;
use crate::shapes::{DrawingShape, ShapeKind};

/// Where one binding's effect comes from.
#[derive(Clone)]
pub(crate) enum BindingSource {
    /// A track or value function for a property.
    Track(BoundTrack),
    /// A function called with the time (C# `Bind(Action<double>)`).
    Callback(Rc<RefCell<dyn FnMut(f64)>>),
    /// The element placed on a path at the fraction a track gives (C# `BindAlongPath`).
    AlongPath { path: PathGeometry, progress: BoundTrack, options: PathTravelOptions },
    /// A float or point track (or function) combined into numeric slots of its target (C#'s
    /// `BindSlots`: the `Bind(..., BindingMode)` overloads).
    Combine { track: BoundTrack, slots: Vec<CombineSlot> },
    /// Something that places elements of the session itself at a time: a roll or overshoot along
    /// a path, a train, a formation.
    Driver(Rc<dyn SessionDriver>),
}

#[derive(Clone)]
pub(crate) struct BindingEntry {
    pub(crate) binding: AnimationBinding,
    pub(crate) source: BindingSource,
}

/// The bindings of a [`DrawingSession`](crate::DrawingSession) (C# `session.Animations`), in the
/// order they were made. Make them through
/// [`DrawingSession::animations_mut`](crate::DrawingSession::animations_mut); setting the
/// session's time applies them all, in order (a later binding of the same property wins), so a
/// frame is only `session.set_time(t)` - a function of the time.
#[derive(Clone, Default)]
pub struct DrawingAnimations {
    pub(crate) entries: Vec<BindingEntry>,
    pub(crate) next_id: u64,
    /// The resting value of every slot a combining binding drives (C#'s `_rests`).
    pub(crate) rests: Vec<RestEntry>,
}

impl DrawingAnimations {
    /// The number of bindings (C# `Count`).
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// The bindings, in the order they were made (C# `GetBindings`).
    pub fn get_bindings(&self) -> Vec<AnimationBinding> {
        self.entries.iter().map(|e| e.binding.clone()).collect()
    }
}

impl fmt::Debug for DrawingAnimations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawingAnimations").field("bindings", &self.get_bindings()).finish()
    }
}

/// An opacity from a track: clamped to 0..1, NaN as 0 (C#'s ClampOpacity).
fn clamp_opacity(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// A radius from a track: finite and above 0, else 0 (C#'s Radius).
fn radius(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

/// A frame number from a float track: rounded down, 0 when not finite (C#'s FrameOf).
fn frame_of(value: f32) -> i32 {
    if value.is_finite() {
        (value as f64).floor().clamp(i32::MIN as f64, i32::MAX as f64) as i32
    } else {
        0
    }
}

/// Effects with one effect property set from a value (C#'s EffectFloatSetter / EffectColorSetter
/// and the ShadowOffset setter: from no effects, C#'s `None`); None when the property is not an
/// effect property for that kind of value.
fn set_effect(current: Option<DrawingEffects>, property: AnimatedProperty, value: BoundValue) -> Option<DrawingEffects> {
    use AnimatedProperty as P;
    let e = current.unwrap_or(DrawingEffects::NONE);
    let finite = |v: f32| if v.is_finite() { v } else { 0.0 };
    Some(match (property, value) {
        (P::BlurRadius, BoundValue::Float(v)) => e.with_blur_radius(radius(v)).unwrap_or(e),
        (P::ShadowBlurRadius, BoundValue::Float(v)) => e.with_shadow_blur_radius(radius(v)).unwrap_or(e),
        (P::GlowRadius, BoundValue::Float(v)) => e.with_glow_radius(radius(v)).unwrap_or(e),
        (P::ShadowOffset, BoundValue::Point(p)) => e.with_shadow_offset(PointF::new(finite(p.x), finite(p.y))).unwrap_or(e),
        (P::ShadowColor, BoundValue::Color(c)) => e.with_shadow_color(c),
        (P::GlowColor, BoundValue::Color(c)) => e.with_glow_color(c),
        _ => return None,
    })
}

/// A transform with one property set from a value (C#'s Float/PointTransformSetter); None when
/// the property is not a transform property for that kind of value.
fn set_transform(current: DrawingTransform, property: AnimatedProperty, value: BoundValue) -> Option<DrawingTransform> {
    use AnimatedProperty as P;
    Some(match (property, value) {
        (P::Rotation, BoundValue::Float(v)) => current.with_rotation(v),
        (P::Scale, BoundValue::Float(v)) => current.with_scale(v),
        (P::ScaleX, BoundValue::Float(v)) => current.with_scale_xy(v, current.scale_y()),
        (P::ScaleY, BoundValue::Float(v)) => current.with_scale_xy(current.scale_x(), v),
        (P::Position, BoundValue::Point(p)) => current.with_translation(p),
        (P::Anchor, BoundValue::Point(p)) => current.with_anchor(p),
        (P::Scale, BoundValue::Point(p)) => current.with_scale_xy(p.x, p.y),
        _ => return None,
    })
}

/// Applies one value to an element (the binding was checked when it was made).
pub(crate) fn apply_to_element(element: &mut DrawingShape, property: AnimatedProperty, value: BoundValue) {
    use AnimatedProperty as P;
    match (property, value) {
        (P::Opacity, BoundValue::Float(v)) => element.set_opacity(clamp_opacity(v)),
        (P::TrimStart, BoundValue::Float(v)) => {
            if let Some(p) = element.as_path_mut() {
                let _ = p.set_trim_start(clamp_opacity(v));
            }
        }
        (P::TrimEnd, BoundValue::Float(v)) => {
            if let Some(p) = element.as_path_mut() {
                let _ = p.set_trim_end(clamp_opacity(v));
            }
        }
        (P::TrimOffset, BoundValue::Float(v)) => {
            if let Some(p) = element.as_path_mut() {
                let _ = p.set_trim_offset(if v.is_finite() { v } else { 0.0 });
            }
        }
        (P::MorphFraction, BoundValue::Float(v)) => {
            if let Some(p) = element.as_path_mut() {
                let _ = p.set_morph_fraction(clamp_opacity(v));
            }
        }
        (P::Color, BoundValue::Color(c)) => {
            if let Some(g) = element.as_group_mut() {
                g.set_content_color(Some(c));
            }
        }
        (P::RevealFraction, BoundValue::Float(v)) => {
            if let Some(b) = element.as_text_block_mut() {
                b.apply_reveal_fraction(v);
            }
        }
        (P::RevealCount, BoundValue::Float(v)) => {
            if let Some(b) = element.as_text_block_mut() {
                b.apply_reveal_count(v);
            }
        }
        (P::FrameIndex, BoundValue::Float(v)) => {
            if let Some(s) = element.as_sprite_mut() {
                s.set_frame_index(frame_of(v));
            }
        }
        (P::Tint, BoundValue::Color(c)) => {
            if let Some(i) = element.as_image_mut() {
                i.set_tint(Some(c));
            } else if let Some(s) = element.as_sprite_mut() {
                s.set_tint(Some(c));
            }
        }
        _ if set_effect(None, property, value).is_some() => {
            let effects = set_effect(element.effects(), property, value);
            element.set_effects(effects);
        }
        _ => {
            if let Some(t) = set_transform(element.transform(), property, value) {
                element.set_transform(t);
            }
        }
    }
}

/// Applies one value to a layer.
pub(crate) fn apply_to_layer(layer: &mut DrawingLayer, property: AnimatedProperty, value: BoundValue) {
    use AnimatedProperty as P;
    match (property, value) {
        (P::Opacity, BoundValue::Float(v)) => layer.set_opacity(clamp_opacity(v)),
        (P::Color, BoundValue::Color(c)) => layer.set_color(c),
        (P::IsVisible, BoundValue::Bool(b)) => layer.set_is_visible(b),
        _ if set_effect(None, property, value).is_some() => {
            let effects = set_effect(layer.effects(), property, value);
            layer.set_effects(effects);
        }
        _ => {
            if let Some(t) = set_transform(layer.transform(), property, value) {
                layer.set_transform(t);
            }
        }
    }
}

/// Applies one value to a camera (an invalid value - a zoom of 0, say - leaves it as it was).
pub(crate) fn apply_to_camera(camera: &mut DrawingCamera, property: AnimatedProperty, value: BoundValue) {
    use AnimatedProperty as P;
    let _ = match (property, value) {
        (P::Rotation, BoundValue::Float(v)) => camera.set_rotation(v),
        (P::Scale, BoundValue::Float(v)) => camera.set_zoom(v),
        (P::Position, BoundValue::Point(p)) => camera.set_pan(p),
        (P::Anchor, BoundValue::Point(p)) => camera.set_pivot(Some(p)),
        _ => Ok(()),
    };
}

/// Whether a target takes a property with this kind of track (C#'s overloads and their
/// switches); the error says what does. `element` is the bound element, for the properties only
/// some kinds of element have.
pub(crate) fn check_binding(target: &BindingTarget, property: AnimatedProperty, track: &BoundTrack, element: Option<&DrawingShape>) -> Result<(), String> {
    use AnimatedProperty as P;
    let kind = track.kind_name();
    // The effect properties, on an element (any kind) or a layer (C#'s effect setters).
    let effect_track = match property {
        P::BlurRadius | P::ShadowBlurRadius | P::GlowRadius => Some("float"),
        P::ShadowOffset => Some("point"),
        P::ShadowColor | P::GlowColor => Some("color"),
        _ => None,
    };
    if let (Some(wanted), BindingTarget::Element(_) | BindingTarget::Layer(_)) = (effect_track, target) {
        return if kind == wanted { Ok(()) } else { Err(format!("{property:?} takes a {wanted} track, not a {kind} track.")) };
    }
    let wrong_track = |on: &str| Err(format!("{property:?} cannot be driven by a {kind} track on {on}."));
    match target {
        BindingTarget::Camera => match (property, kind) {
            (P::Rotation | P::Scale, "float") | (P::Position | P::Anchor, "point") => Ok(()),
            _ => wrong_track("a camera (Rotation and Scale take float tracks, Position and Anchor point tracks)"),
        },
        BindingTarget::Layer(_) => match (property, kind) {
            (P::Opacity | P::Rotation | P::Scale | P::ScaleX | P::ScaleY, "float") | (P::Position | P::Anchor | P::Scale, "point") | (P::Color, "color") | (P::IsVisible, "bool") => Ok(()),
            _ => wrong_track("a layer"),
        },
        BindingTarget::Element(_) => {
            let path = element.and_then(|e| e.as_path());
            match (property, kind) {
                (P::TrimStart | P::TrimEnd | P::TrimOffset, "float") => match path {
                    Some(_) => Ok(()),
                    None => Err(format!("{property:?} can only be bound on a path shape.")),
                },
                (P::MorphFraction, "float") => match path {
                    Some(p) if p.morph().is_some() => Ok(()),
                    _ => Err("MorphFraction can only be bound on a path shape made from a PathMorph (PathShape::from_morph or with_morph).".to_string()),
                },
                (P::Color, "color") => match element.map(|e| e.kind()) {
                    Some(ShapeKind::Group(_)) => Ok(()),
                    _ => Err("Color can only be bound on a group (its content colour).".to_string()),
                },
                (P::RevealFraction | P::RevealCount, "float") => match element.map(|e| e.kind()) {
                    Some(ShapeKind::TextBlock(_)) => Ok(()),
                    _ => Err(format!("{property:?} can only be bound on a text block (TextBlockShape).")),
                },
                (P::FrameIndex, "float") => match element.map(|e| e.kind()) {
                    Some(ShapeKind::Sprite(_)) => Ok(()),
                    _ => Err("FrameIndex can only be bound on a sprite (SpriteShape).".to_string()),
                },
                (P::Tint, "color") => match element.map(|e| e.kind()) {
                    Some(ShapeKind::Image(_) | ShapeKind::Sprite(_)) => Ok(()),
                    _ => Err("Tint can only be bound on a picture or a sprite (ImageShape, SpriteShape).".to_string()),
                },
                (P::Opacity | P::Rotation | P::Scale | P::ScaleX | P::ScaleY, "float") | (P::Position | P::Anchor | P::Scale, "point") => Ok(()),
                (P::IsVisible, _) => Err("IsVisible is bound on a layer (a bool track), not on an element.".to_string()),
                _ => wrong_track("an element"),
            }
        }
    }
}
