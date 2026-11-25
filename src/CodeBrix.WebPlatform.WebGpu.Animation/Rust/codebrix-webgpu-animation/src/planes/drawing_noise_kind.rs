//! `DrawingNoiseKind`: the kinds of coherent noise (C# `DrawingNoiseKind`, plus the caustics kind,
//! which is not in C#).

/// The kinds of coherent noise [`DrawingNoise`](crate::planes::DrawingNoise) produces (C#
/// `DrawingNoiseKind`; `Caustics` is not in C#).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrawingNoiseKind {
    /// Value noise: a random value at every whole-number lattice point, blended smoothly
    /// between them. Blobby and soft: clouds, mottling, slow flicker.
    Value = 0,
    /// Gradient (Perlin-style) noise: a random direction at every lattice point - finer, more
    /// natural detail, exactly 0 at the lattice points.
    Gradient = 1,
    /// Caustics (Not in C#): the light net of a pool floor - bright curved lines along the edges of
    /// a lattice of cells whose points circle slowly as z grows (Worley F2 - F1 edges, the space
    /// warped by two sines; [`DrawingNoise::caustics`](crate::planes::DrawingNoise::caustics)).
    /// -1 is the dark floor, 1 the brightest line; z is the time (times a speed), passed in. UNLIKE
    /// the two kinds above it is THE SAME PATTERN on the CPU and the GPU (within float rounding):
    /// it has no C# original to stay bit-exact with, so the CPU uses the GPU's 32-bit hash. A
    /// plane filled with it and a [`NoiseShape`](crate::shapes::NoiseShape) of it show the same
    /// net. Two octaves, lacunarity 1.7, gain 0.6 look like real pool light.
    Caustics = 2,
}
