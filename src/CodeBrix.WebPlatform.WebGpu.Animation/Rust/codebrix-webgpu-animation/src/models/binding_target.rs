//! `BindingTarget`: what a binding drives - an element, a layer or the camera (C#'s `Bind`
//! overloads take the element, layer or camera object itself).

use crate::models::ElementId;

/// What an [`AnimationBinding`](crate::models::AnimationBinding) drives: an element (by its id,
/// on any layer or inside a group), a layer (by its name), or the session's camera.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BindingTarget {
    /// A shape or group on one of the session's layers.
    Element(ElementId),
    /// The session's layer of this name.
    Layer(String),
    /// The session's camera (set one with `DrawingSession::set_camera` first).
    Camera,
}

impl BindingTarget {
    /// The layer of this name.
    pub fn layer(name: &str) -> BindingTarget {
        BindingTarget::Layer(name.to_string())
    }
}

impl From<ElementId> for BindingTarget {
    fn from(id: ElementId) -> BindingTarget {
        BindingTarget::Element(id)
    }
}
