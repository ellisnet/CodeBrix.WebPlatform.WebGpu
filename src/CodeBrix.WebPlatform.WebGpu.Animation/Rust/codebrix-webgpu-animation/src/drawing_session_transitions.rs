//! Transitions between two LAYERS of a session, drawn on the GPU (the planes' `DrawingTransition`
//! as a pass of the renderer instead of a CPU mix of two plane images).

use kurbo::Affine;

use crate::planes::{DrawingTransition, DrawingTransitionDirection, DrawingTransitionKind};
use crate::DrawingSession;

/// A transition between two layers of a session at a fraction (0 = the outgoing layer, 1 = the
/// incoming one).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayerTransition {
    pub(crate) outgoing: String,
    pub(crate) incoming: String,
    pub(crate) transition: DrawingTransition,
    pub(crate) fraction: f32,
}

impl LayerTransition {
    /// How the layer `name` takes part, for a frame of `width` x `height` device pixels: the
    /// whole-pixel shift of a slide or push (device pixels), and - for the incoming layer of a
    /// dissolve, wipe or fade through a colour - the transition and fraction its pass applies.
    pub(crate) fn part(&self, name: &str, width: f64, height: f64) -> (Option<(f64, f64)>, Option<(&DrawingTransition, f32)>) {
        let incoming = name == self.incoming;
        let outgoing = name == self.outgoing;
        if !incoming && !outgoing {
            return (None, None);
        }
        let kind = self.transition.kind();
        if matches!(kind, DrawingTransitionKind::Slide | DrawingTransitionKind::Push) {
            let horizontal = matches!(self.transition.direction(), DrawingTransitionDirection::Left | DrawingTransitionDirection::Right);
            let extent = if horizontal { width } else { height };
            // The planes' whole-pixel offset: floor(fraction x extent + 0.5).
            let offset = (self.fraction as f64 * extent + 0.5).floor().clamp(0.0, extent);
            let (sign, along) = match self.transition.direction() {
                DrawingTransitionDirection::Right | DrawingTransitionDirection::Down => (1.0, offset - extent),
                DrawingTransitionDirection::Left | DrawingTransitionDirection::Up => (-1.0, extent - offset),
            };
            let shift = if incoming {
                Some(along)
            } else if kind == DrawingTransitionKind::Push {
                Some(sign * offset)
            } else {
                None
            };
            return (shift.map(|d| if horizontal { (d, 0.0) } else { (0.0, d) }), None);
        }
        if incoming {
            (None, Some((&self.transition, self.fraction)))
        } else {
            (None, None)
        }
    }
}

impl DrawingSession {
    /// Runs a transition between two of the session's layers, drawn on the GPU: the INCOMING
    /// layer over the OUTGOING one at `fraction` (0 = only the outgoing layer, 1 = only the
    /// incoming one; clamped) - compute the fraction from the frame's time, for example by easing a
    /// track, and set it every frame. The planes' formulas and parameters (C#'s
    /// `DrawingTransition.Apply` on two frames): a dissolve weighs the incoming layer by the
    /// fraction; a wipe by the hard or soft edge moving from the side it starts at (softness in
    /// calibrated units); slides and pushes move the layers by whole device pixels,
    /// `floor(fraction x extent + 0.5)` (a slide moves the incoming layer in over the outgoing one, a
    /// push moves both); a fade through a colour shows the colour over the outgoing layer up to
    /// 0.5 and fades from it to the incoming layer after it. Exact for OPAQUE layers (the planes
    /// transition whole frames); a translucent incoming layer is composited over the outgoing one
    /// rather than mixed with it. The extent is the session's frame as drawn (its aspect-fitted
    /// rectangle). The error says what to do with an unknown layer name or a NaN fraction.
    pub fn set_layer_transition(&mut self, outgoing: &str, incoming: &str, transition: DrawingTransition, fraction: f32) -> Result<(), String> {
        if fraction.is_nan() {
            return Err("A transition's fraction must be a number from 0 to 1 (it was NaN).".to_string());
        }
        for name in [outgoing, incoming] {
            if self.get_layer(name).is_none() {
                return Err(format!("The session has no layer named '{name}': add it first."));
            }
        }
        self.transition = Some(LayerTransition { outgoing: outgoing.to_string(), incoming: incoming.to_string(), transition, fraction: fraction.clamp(0.0, 1.0) });
        Ok(())
    }

    /// Ends the transition between layers: both layers are drawn as they are again.
    pub fn clear_layer_transition(&mut self) {
        self.transition = None;
    }

    /// The device shift a slide or push gives the layer `name` within the frame `frame`
    /// (device pixels), and the transition its pass applies (crate-internal).
    pub(crate) fn transition_part(&self, name: &str, frame: kurbo::Rect) -> (Affine, Option<(&DrawingTransition, f32)>, bool) {
        match &self.transition {
            Some(t) => {
                let (shift, pass) = t.part(name, frame.width(), frame.height());
                let moved = shift.is_some();
                (shift.map_or(Affine::IDENTITY, |(x, y)| Affine::translate((x, y))), pass, moved)
            }
            None => (Affine::IDENTITY, None, false),
        }
    }
}

impl DrawingSession {
    /// The particles of `system` at `time` (the frame's time, passed in) as a
    /// [`ParticlesShape`](crate::shapes::ParticlesShape) on the active layer, drawn with `softness`
    /// (None: 1) - C# `DrawParticles(system, time, softness = 1)`; returns its id. The shape holds
    /// the particles of THAT time: for moving particles draw a new one per frame (or use an
    /// [`AnimatedShape`](crate::shapes::AnimatedShape)). The error is the system's.
    pub fn draw_particles(&mut self, system: &crate::planes::DrawingParticleSystem, time: f64, softness: Option<f32>) -> Result<crate::models::ElementId, String> {
        let shape = crate::shapes::ParticlesShape::new(system, time, softness)?;
        Ok(self.draw_shape(shape))
    }
}
