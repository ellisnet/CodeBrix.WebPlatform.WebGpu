//! Combined bindings (internal): the numeric SLOTS a combining binding drives, their resting
//! values, and the order in which a session applies its bindings (C# `DrawingAnimations.Apply`,
//! `BindSlots`, `Combine`, `TransformSlots` and the rest registry).
//!
//! THE BINDING-ORDER RULE (C#'s, exactly): with only Replace bindings, every binding applies in
//! the order it was made (a later binding of the same property wins). When an Add or Multiply
//! binding exists, every slot such a binding combines into is first put back to its RESTING
//! value (its value when it was first bound that way), then every Replace binding and callback
//! applies in order, then every Multiply binding, then every Add binding - so the result never
//! depends on the order the bindings were made in, and showing a time twice gives the same
//! values.

use crate::drawing_session::DrawingSession;
use crate::geometry::PointF;
use crate::models::bound_track::BoundValue;
use crate::models::drawing_animations::{BindingEntry, BindingSource, DrawingAnimations};
use crate::models::{BindingMode, BindingTarget, BoundTrack};

/// One numeric part of a target that a combining binding drives (C#'s slot names).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombineSlot {
    TranslationX,
    TranslationY,
    Rotation,
    ScaleX,
    ScaleY,
    Opacity,
    PanX,
    PanY,
    Zoom,
}

/// A slot's resting value and how many combining bindings use it (C#'s `RestEntry`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RestEntry {
    pub(crate) target: BindingTarget,
    pub(crate) slot: CombineSlot,
    pub(crate) rest: f32,
    pub(crate) count: usize,
}

/// Something that places elements of the session at a time (a roll or overshoot along a path, a
/// train, a formation). It must be a pure function of the time.
pub(crate) trait SessionDriver {
    /// Places what it drives for this time.
    fn apply(&self, session: &mut DrawingSession, seconds: f64);
}

/// An opacity from a track: clamped to 0..1, NaN as 0 (C#'s ClampOpacity).
pub(crate) fn clamp_opacity(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// The slots a property stands for on a target, in value order (C#'s `TransformSlots` and the
/// camera's): a point fills two slots with X and Y, a float scale both scale slots with the same
/// value. The error says what can be combined instead.
pub(crate) fn slots_for(target: &BindingTarget, property: crate::models::AnimatedProperty, track: &BoundTrack) -> Result<Vec<CombineSlot>, String> {
    use crate::models::AnimatedProperty as P;
    use CombineSlot as S;
    let kind = track.kind_name();
    match target {
        BindingTarget::Camera => match (property, kind) {
            (P::Rotation, "float") => Ok(vec![S::Rotation]),
            (P::Scale, "float") => Ok(vec![S::Zoom]),
            (P::Position, "point") => Ok(vec![S::PanX, S::PanY]),
            _ => Err(format!("{property:?} cannot be driven by a combined {kind} track on a camera: combine Rotation or Scale (zoom) with a float track, or Position (pan) with a point track.")),
        },
        BindingTarget::Element(_) | BindingTarget::Layer(_) => match (property, kind) {
            (P::Position, "point") => Ok(vec![S::TranslationX, S::TranslationY]),
            (P::Scale, "point") | (P::Scale, "float") => Ok(vec![S::ScaleX, S::ScaleY]),
            (P::Rotation, "float") => Ok(vec![S::Rotation]),
            (P::ScaleX, "float") => Ok(vec![S::ScaleX]),
            (P::ScaleY, "float") => Ok(vec![S::ScaleY]),
            (P::Opacity, "float") => Ok(vec![S::Opacity]),
            _ => Err(format!("{property:?} cannot be driven by a combined {kind} track: combine Rotation, Scale, ScaleX, ScaleY or Opacity with a float track, or Position or Scale with a point track.")),
        },
    }
}

/// A slot's value now (None when the target is gone).
pub(crate) fn get_slot(session: &DrawingSession, target: &BindingTarget, slot: CombineSlot) -> Option<f32> {
    use CombineSlot as S;
    match target {
        BindingTarget::Camera => {
            let c = session.camera()?;
            match slot {
                S::Rotation => Some(c.rotation()),
                S::Zoom => Some(c.zoom()),
                S::PanX => Some(c.pan().x),
                S::PanY => Some(c.pan().y),
                _ => None,
            }
        }
        BindingTarget::Element(id) => {
            let e = session.find_element(*id)?;
            transform_slot(e.transform(), e.opacity(), slot)
        }
        BindingTarget::Layer(name) => {
            let l = session.get_layer(name)?;
            transform_slot(l.transform(), l.opacity(), slot)
        }
    }
}

fn transform_slot(t: crate::models::DrawingTransform, opacity: f32, slot: CombineSlot) -> Option<f32> {
    use CombineSlot as S;
    match slot {
        S::TranslationX => Some(t.translation().x),
        S::TranslationY => Some(t.translation().y),
        S::Rotation => Some(t.rotation()),
        S::ScaleX => Some(t.scale_x()),
        S::ScaleY => Some(t.scale_y()),
        S::Opacity => Some(opacity),
        _ => None,
    }
}

fn with_slot(t: crate::models::DrawingTransform, slot: CombineSlot, value: f32) -> crate::models::DrawingTransform {
    use CombineSlot as S;
    match slot {
        S::TranslationX => t.with_translation(PointF::new(value, t.translation().y)),
        S::TranslationY => t.with_translation(PointF::new(t.translation().x, value)),
        S::Rotation => t.with_rotation(value),
        S::ScaleX => t.with_scale_xy(value, t.scale_y()),
        S::ScaleY => t.with_scale_xy(t.scale_x(), value),
        _ => t,
    }
}

/// Sets a slot (opacity clamped to 0..1; a camera value it refuses - a zoom of 0 - leaves the
/// camera as it was, where C# would throw).
pub(crate) fn set_slot(session: &mut DrawingSession, target: &BindingTarget, slot: CombineSlot, value: f32) {
    use CombineSlot as S;
    match target {
        BindingTarget::Camera => {
            if let Some(c) = session.camera_mut() {
                let _ = match slot {
                    S::Rotation => c.set_rotation(value),
                    S::Zoom => c.set_zoom(value),
                    S::PanX => {
                        let y = c.pan().y;
                        c.set_pan(PointF::new(value, y))
                    }
                    S::PanY => {
                        let x = c.pan().x;
                        c.set_pan(PointF::new(x, value))
                    }
                    _ => Ok(()),
                };
            }
        }
        BindingTarget::Element(id) => {
            if let Some(e) = session.find_element_mut(*id) {
                if slot == S::Opacity {
                    e.set_opacity(clamp_opacity(value));
                } else {
                    e.set_transform(with_slot(e.transform(), slot, value));
                }
            }
        }
        BindingTarget::Layer(name) => {
            if let Some(l) = session.get_layer_mut(name) {
                if slot == S::Opacity {
                    l.set_opacity(clamp_opacity(value));
                } else {
                    l.set_transform(with_slot(l.transform(), slot, value));
                }
            }
        }
    }
}

fn combine(session: &mut DrawingSession, target: &BindingTarget, slot: CombineSlot, value: f32, mode: BindingMode) {
    let next = match mode {
        BindingMode::Add => match get_slot(session, target, slot) {
            Some(current) => current + value,
            None => return,
        },
        BindingMode::Multiply => match get_slot(session, target, slot) {
            Some(current) => current * value,
            None => return,
        },
        BindingMode::Replace => value,
    };
    set_slot(session, target, slot, next);
}

/// Applies one combining binding at a time (C#'s `BindSlots` apply).
pub(crate) fn apply_combine(session: &mut DrawingSession, target: &BindingTarget, track: &BoundTrack, slots: &[CombineSlot], mode: BindingMode, seconds: f64) {
    match track.value_at(seconds) {
        Some(BoundValue::Point(p)) if slots.len() == 2 => {
            combine(session, target, slots[0], p.x, mode);
            combine(session, target, slots[1], p.y, mode);
        }
        Some(BoundValue::Float(v)) => {
            for slot in slots {
                combine(session, target, *slot, v, mode);
            }
        }
        _ => {}
    }
}

impl DrawingAnimations {
    /// Registers the resting value of each slot a combining binding drives: taken at the FIRST
    /// combining bind of that slot, counted for every later one (C#'s rest registry).
    pub(crate) fn register_rests(&mut self, target: &BindingTarget, slots: &[CombineSlot], current: &[f32]) {
        for (slot, value) in slots.iter().zip(current) {
            match self.rests.iter_mut().find(|r| r.target == *target && r.slot == *slot) {
                Some(r) => r.count += 1,
                None => self.rests.push(RestEntry { target: target.clone(), slot: *slot, rest: *value, count: 1 }),
            }
        }
    }

    /// Releases the resting values a removed binding held (the slot is forgotten when no
    /// combining binding uses it any more).
    pub(crate) fn release_rests(&mut self, entry: &BindingEntry) {
        if entry.binding.mode == BindingMode::Replace {
            return;
        }
        let (BindingSource::Combine { slots, .. }, Some(target)) = (&entry.source, &entry.binding.target) else { return };
        for slot in slots {
            if let Some(i) = self.rests.iter().position(|r| r.target == *target && r.slot == *slot) {
                self.rests[i].count -= 1;
                if self.rests[i].count == 0 {
                    self.rests.remove(i);
                }
            }
        }
    }
}

/// Applies every binding at a time, by the binding-order rule (module docs).
pub(crate) fn apply_animations(session: &mut DrawingSession, seconds: f64) {
    if session.animations.entries.is_empty() {
        return;
    }
    let entries = std::mem::take(&mut session.animations.entries);
    let combining = entries.iter().any(|e| e.binding.mode != BindingMode::Replace);
    if !combining {
        for entry in &entries {
            session.apply_binding(entry, seconds);
        }
    } else {
        let rests = session.animations.rests.clone();
        for r in &rests {
            set_slot(session, &r.target, r.slot, r.rest);
        }
        for mode in [BindingMode::Replace, BindingMode::Multiply, BindingMode::Add] {
            for entry in entries.iter().filter(|e| e.binding.mode == mode) {
                session.apply_binding(entry, seconds);
            }
        }
    }
    // Bindings made while applying (none can be: the callbacks cannot reach the session).
    let mut entries = entries;
    entries.append(&mut session.animations.entries);
    session.animations.entries = entries;
}
