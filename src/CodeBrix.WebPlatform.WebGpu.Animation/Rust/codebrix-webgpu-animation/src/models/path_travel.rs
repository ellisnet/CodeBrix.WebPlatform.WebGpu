//! Travel along a path with ROLLING and overshoot past the ends (internal): the binding C#'s
//! `BindAlongPath(element, path, progress, PathTravelOptions)` makes when a roll radius or
//! `ExtendBeyondEnds` is set.
//!
//! THE ARITHMETIC (C#'s): the element's anchor rides the path at the progress's fraction of the
//! length; the roll is `(p - UprightAtProgress) x (length / radius x 180 / pi)` degrees, and it
//! ADDS to the facing rotation (`angle + offset`) when the element also faces along the path.
//! For a float TRACK the roll is evaluated key by key on the converted key values - exactly a
//! rotation track mirrored from the progress track, each segment eased alike; for a value
//! function (a Rust addition) it is the roll of the value. Without extending, a progress outside
//! 0..1 rolls as the clamped progress.

use crate::animation::{FloatTrack, KeyframeTrack};
use crate::drawing_session::DrawingSession;
use crate::geometry::PointF;
use crate::models::binding_combine::SessionDriver;
use crate::models::bound_track::BoundValue;
use crate::models::{BoundTrack, ElementId, PathTravelOptions};
use crate::paths::PathGeometry;

/// The roll: degrees per fraction of the path, the upright progress, and the progress track's
/// keys converted to rotations.
pub(crate) struct Roll {
    pub(crate) upright: f32,
    pub(crate) degrees_per_fraction: f64,
    pub(crate) mapped: Option<FloatTrack>,
}

impl Roll {
    pub(crate) fn new(path: &PathGeometry, radius: f32, upright: f32, progress: &BoundTrack) -> Roll {
        let length = path.get_length();
        let degrees_per_fraction = (length / radius) as f64 * 180.0 / std::f64::consts::PI;
        let mut roll = Roll { upright, degrees_per_fraction, mapped: None };
        if let BoundTrack::Float(track) = progress {
            let mut mapped: FloatTrack = KeyframeTrack::new().with_loop_mode(track.loop_mode);
            for key in track.get_keyframes() {
                mapped.add_key(key.with_value(roll.roll(*key.value())));
            }
            roll.mapped = Some(mapped);
        }
        roll
    }

    /// The roll at a progress value (C#'s `roll` lambda).
    pub(crate) fn roll(&self, value: f32) -> f32 {
        ((value - self.upright) as f64 * self.degrees_per_fraction) as f32
    }

    /// The roll at a time (C#'s `rollAt`).
    pub(crate) fn at(&self, p: f32, seconds: f64, extend: bool) -> f32 {
        if p.is_nan() {
            return self.roll(0.0);
        }
        if !extend && !(0.0..=1.0).contains(&p) {
            return self.roll(p.clamp(0.0, 1.0));
        }
        match &self.mapped {
            Some(track) => track.evaluate(seconds).unwrap_or(0.0),
            None => self.roll(p),
        }
    }
}

pub(crate) struct TravelDriver {
    pub(crate) element: ElementId,
    pub(crate) path: PathGeometry,
    pub(crate) progress: BoundTrack,
    pub(crate) options: PathTravelOptions,
    pub(crate) roll: Option<Roll>,
}

impl SessionDriver for TravelDriver {
    fn apply(&self, session: &mut DrawingSession, seconds: f64) {
        let Some(BoundValue::Float(p)) = self.progress.value_at(seconds) else { return };
        let extend = self.options.extend_beyond_ends;
        let point = self.path.get_point_at_with(p, extend);
        let Some(element) = session.find_element_mut(self.element) else { return };
        let current = element.transform();
        let translation = PointF::new(point.position.x - current.anchor().x, point.position.y - current.anchor().y);
        let mut next = current.with_translation(translation);
        let offset = self.options.rotation_offset;
        if self.options.orient_to_path {
            let facing = point.angle() + offset;
            next = next.with_rotation(match &self.roll {
                Some(r) => facing + r.at(p, seconds, extend),
                None => facing,
            });
        } else if let Some(r) = &self.roll {
            let rolled = r.at(p, seconds, extend);
            next = next.with_rotation(if offset != 0.0 { rolled + offset } else { rolled });
        }
        element.set_transform(next);
    }
}
