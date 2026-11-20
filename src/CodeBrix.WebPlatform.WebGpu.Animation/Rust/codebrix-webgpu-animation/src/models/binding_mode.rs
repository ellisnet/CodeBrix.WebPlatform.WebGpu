//! `BindingMode`: how a binding's value combines with the property it drives (C# `BindingMode`).

/// How a binding's value combines with its property (C# `BindingMode`, the same numbers).
/// Replace is the plain binding (a later one of the same property wins); Add adds the value to
/// whatever the other bindings set (a sway on top of a turn, a shake on top of a pan); Multiply
/// multiplies by it (a pulse on top of a scale, a flicker on top of a fade). See
/// [`DrawingAnimationsMut::bind_with_mode`](crate::models::DrawingAnimationsMut::bind_with_mode)
/// for the order the session applies them in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BindingMode {
    /// The value replaces the property (the plain binding).
    #[default]
    Replace = 0,
    /// The value is added to the property.
    Add = 1,
    /// The property is multiplied by the value.
    Multiply = 2,
}
