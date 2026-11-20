//! `DrawingParticle`: one live particle at one moment - pure data (C# `DrawingParticle`).

use crate::Color;

/// One live particle at one moment, as [`DrawingParticleSystem::get_particles`](crate::planes::DrawingParticleSystem::get_particles)
/// returns it - pure data, nothing drawn (C# `DrawingParticle`). Positions and sizes are in the
/// units the emitters use (a plane's pixels, or calibrated drawing units when a session draws
/// the particles).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawingParticle {
    emitter_index: usize,
    index: i64,
    birth_time: f64,
    age: f64,
    lifetime: f64,
    x: f32,
    y: f32,
    size: f32,
    opacity: f32,
    color: Color,
}

impl DrawingParticle {
    /// A particle snapshot (C# `DrawingParticle(emitterIndex, index, birthTime, age, lifetime, x,
    /// y, size, opacity, color)`).
    #[allow(clippy::too_many_arguments)]
    pub fn new(emitter_index: usize, index: i64, birth_time: f64, age: f64, lifetime: f64, x: f32, y: f32, size: f32, opacity: f32, color: Color) -> DrawingParticle {
        DrawingParticle { emitter_index, index, birth_time, age, lifetime, x, y, size, opacity, color }
    }

    /// The index of the emitter in the system (C# `EmitterIndex`).
    pub fn emitter_index(&self) -> usize {
        self.emitter_index
    }

    /// The birth number within its emitter: burst particles 0 .. burst - 1, then continuous ones
    /// in birth order; stable across queries, so it can key per-particle extras (C# `Index`).
    pub fn index(&self) -> i64 {
        self.index
    }

    /// When it was born, seconds (C# `BirthTime`).
    pub fn birth_time(&self) -> f64 {
        self.birth_time
    }

    /// How long it has lived at the queried time, seconds (C# `Age`).
    pub fn age(&self) -> f64 {
        self.age
    }

    /// How long it lives in total, seconds (C# `Lifetime`).
    pub fn lifetime(&self) -> f64 {
        self.lifetime
    }

    /// `age / lifetime`, 0 (just born) .. 1 (about to die) (C# `LifeFraction`).
    pub fn life_fraction(&self) -> f64 {
        if self.lifetime > 0.0 {
            self.age / self.lifetime
        } else {
            0.0
        }
    }

    /// The centre's x (C# `X`).
    pub fn x(&self) -> f32 {
        self.x
    }

    /// The centre's y (C# `Y`).
    pub fn y(&self) -> f32 {
        self.y
    }

    /// The size (diameter) (C# `Size`).
    pub fn size(&self) -> f32 {
        self.size
    }

    /// The opacity, 0..1 - multiply the colour's alpha by it when drawing (C# `Opacity`).
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// The colour at this point of its life (C# `Color`).
    pub fn color(&self) -> Color {
        self.color
    }
}
