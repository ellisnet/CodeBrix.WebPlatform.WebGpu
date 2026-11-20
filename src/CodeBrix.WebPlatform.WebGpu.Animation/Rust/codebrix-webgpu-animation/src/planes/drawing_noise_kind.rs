//! `DrawingNoiseKind`: the kinds of coherent noise (C# `DrawingNoiseKind`).

/// The kinds of coherent noise [`DrawingNoise`](crate::planes::DrawingNoise) produces (C#
/// `DrawingNoiseKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrawingNoiseKind {
    /// Value noise: a random value at every whole-number lattice point, blended smoothly
    /// between them. Blobby and soft: clouds, mottling, slow flicker.
    Value = 0,
    /// Gradient (Perlin-style) noise: a random direction at every lattice point - finer, more
    /// natural detail, exactly 0 at the lattice points.
    Gradient = 1,
}
