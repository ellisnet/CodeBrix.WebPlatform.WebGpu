//! `StepPosition`: where the jumps of a `steps()` staircase fall (C# `StepPosition`, CSS's jump
//! terms).

/// Where the jumps of [`Easing::steps`](crate::animation::Easing::steps) fall: CSS's
/// `jump-end`, `jump-start`, `jump-none` and `jump-both`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum StepPosition {
    /// The jump is at the end of each step (CSS `jump-end`, the default).
    #[default]
    End = 0,
    /// The jump is at the start of each step (CSS `jump-start`).
    Start = 1,
    /// No jump at either end: the first step is 0 and the last 1 (CSS `jump-none`).
    None = 2,
    /// Jumps at both ends (CSS `jump-both`).
    Both = 3,
}
