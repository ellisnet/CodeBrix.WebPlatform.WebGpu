//! `AnimationBinding`: one binding of a session's animations - what it drives (C#
//! `AnimationBinding`).

use crate::models::{AnimatedProperty, BindingMode, BindingTarget};

/// One binding made by the session's animations (C# `AnimationBinding`): its target and
/// property (None for a time callback or a motion along a path). Keep it to remove the binding
/// later. Applying is the session's: setting its time applies every binding in the order they
/// were made, so a later binding of the same property wins - unless a binding combines with a
/// [`BindingMode`] (see [`DrawingAnimationsMut::bind_with_mode`](crate::models::DrawingAnimationsMut::bind_with_mode)).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnimationBinding {
    pub(crate) id: u64,
    pub(crate) target: Option<BindingTarget>,
    pub(crate) property: Option<AnimatedProperty>,
    pub(crate) mode: BindingMode,
}

impl AnimationBinding {
    /// What it drives (None for a time callback).
    pub fn target(&self) -> Option<&BindingTarget> {
        self.target.as_ref()
    }

    /// The property it drives (None for a time callback or a motion along a path).
    pub fn property(&self) -> Option<AnimatedProperty> {
        self.property
    }

    /// How it combines with the property (C# `Mode`; Replace for every binding made without one).
    pub fn mode(&self) -> BindingMode {
        self.mode
    }
}
