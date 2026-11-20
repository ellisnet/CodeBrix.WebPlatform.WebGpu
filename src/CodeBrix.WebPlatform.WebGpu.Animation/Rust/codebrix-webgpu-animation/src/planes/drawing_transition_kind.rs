//! `DrawingTransitionKind`: the kinds of frame-to-frame transition (C# `DrawingTransitionKind`).

/// The kinds of frame-to-frame transition a [`DrawingTransition`](crate::planes::DrawingTransition)
/// performs (C# `DrawingTransitionKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrawingTransitionKind {
    /// A cross-fade: every pixel mixes from the outgoing to the incoming frame.
    Dissolve = 0,
    /// A moving edge (hard or soft) replaces the outgoing frame with the incoming one; nothing moves.
    Wipe = 1,
    /// The incoming frame slides in over the outgoing frame, which stays still.
    Slide = 2,
    /// The incoming frame slides in and pushes the outgoing frame out ahead of it.
    Push = 3,
    /// The outgoing frame fades to a colour over the first half, then the colour fades to the
    /// incoming frame.
    FadeThroughColor = 4,
}
