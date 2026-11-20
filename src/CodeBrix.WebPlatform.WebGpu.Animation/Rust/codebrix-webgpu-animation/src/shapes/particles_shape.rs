//! `ParticlesShape`: the particles of a particle system at one time, drawn as soft discs in ONE
//! instanced GPU draw (C# `ParticlesShape`).

use crate::color::Color;
use crate::geometry::RectangleF;
use crate::paths::{PathGeometry, PathGeometryBuilder};
use crate::planes::{DrawingParticle, DrawingParticleSystem};

/// One drawn particle: centre, diameter and colour (its alpha already scaled by the particle's
/// opacity, C#'s `ScaleAlpha`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DrawnParticle {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) size: f32,
    pub(crate) color: Color,
}

/// The particles of a [`DrawingParticleSystem`] at ONE time, computed when the shape is made
/// (C# `ParticlesShape(system, time, softness = 1)`): each a disc of its size (a diameter, in
/// calibrated units) at its position, in its colour at its opacity, drawn in the system's order
/// (later over earlier). `softness` 0 draws solid discs, 1 a radial fade from the colour to
/// transparent, between the two the colour held to `1 - softness` of the radius and then fading.
/// The renderer draws ALL of them in one instanced draw of soft discs (not a path per particle).
/// For particles that move, make a new shape per frame - for example from an
/// [`AnimatedShape`](crate::shapes::AnimatedShape) factory at the session's time, or with
/// [`DrawingSession::draw_particles`](crate::DrawingSession::draw_particles) - the system is a pure
/// function of the time, so frames can be drawn in any order.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticlesShape {
    pub(crate) particles: Vec<DrawnParticle>,
    pub(crate) softness: f32,
}

/// C#'s `ScaleAlpha(color.A, clamp(opacity, 0, 1))`: the alpha byte times the opacity, rounded half away from zero.
fn scale_alpha(alpha: u8, opacity: f32) -> u8 {
    (alpha as f32 * opacity.clamp(0.0, 1.0) + 0.5).floor().clamp(0.0, 255.0) as u8
}

impl ParticlesShape {
    /// The particles of `system` at `time` seconds (the frame's time, passed in), drawn with
    /// `softness` (None: 1, C#'s default; clamped to 0..1). The error is the system's (a time
    /// that is not finite).
    pub fn new(system: &DrawingParticleSystem, time: f64, softness: Option<f32>) -> Result<ParticlesShape, String> {
        let list = system.get_particles(time)?;
        Ok(ParticlesShape::from_particles(&list, softness))
    }

    /// Particles already queried (C# builds the shape from the system; this takes the list).
    pub fn from_particles(particles: &[DrawingParticle], softness: Option<f32>) -> ParticlesShape {
        let drawn = particles
            .iter()
            .filter_map(|p| {
                let c = p.color();
                let a = scale_alpha(c.a, p.opacity());
                if a == 0 || !(p.size() > 0.0) {
                    return None;
                }
                Some(DrawnParticle { x: p.x(), y: p.y(), size: p.size(), color: Color::from_rgba(c.r, c.g, c.b, a) })
            })
            .collect();
        ParticlesShape { particles: drawn, softness: softness.unwrap_or(1.0).clamp(0.0, 1.0) }
    }

    /// How many particles are drawn (those with a size and some alpha).
    pub fn count(&self) -> usize {
        self.particles.len()
    }

    /// The softness (0 solid discs .. 1 a radial fade).
    pub fn softness(&self) -> f32 {
        self.softness
    }

    /// The bounds of the drawn discs, calibrated units (empty when none is drawn).
    pub fn get_bounds(&self) -> RectangleF {
        let mut it = self.particles.iter();
        let Some(first) = it.next() else {
            return RectangleF::default();
        };
        let r = first.size / 2.0;
        let (mut l, mut t, mut rr, mut b) = (first.x - r, first.y - r, first.x + r, first.y + r);
        for p in it {
            let r = p.size / 2.0;
            l = l.min(p.x - r);
            t = t.min(p.y - r);
            rr = rr.max(p.x + r);
            b = b.max(p.y + r);
        }
        RectangleF::new(l, t, rr - l, b - t)
    }

    /// The discs as ONE geometry: a circle per drawn particle (C#'s `GetOutline` and
    /// `GetCoverageAreas`).
    pub fn get_outline(&self) -> PathGeometry {
        let mut builder = PathGeometryBuilder::new();
        for p in &self.particles {
            builder.add_circle(crate::PointF::new(p.x, p.y), p.size / 2.0);
        }
        builder.build()
    }
}
