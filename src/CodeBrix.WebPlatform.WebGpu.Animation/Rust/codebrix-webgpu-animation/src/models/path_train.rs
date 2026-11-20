//! `PathTrain`: several elements sent along ONE path one behind the other (C# `PathTrain`, made
//! by `BindTrainAlongPath`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::drawing_session::DrawingSession;
use crate::geometry::PointF;
use crate::models::binding_combine::{clamp_opacity, SessionDriver};
use crate::models::bound_track::BoundValue;
use crate::models::{AnimationBinding, BoundTrack, ElementId, PathTrainCar, PathTrainOptions};
use crate::paths::{PathGeometry, PathPoint};

/// What a train's binding holds (shared with its [`PathTrain`] handle, so a car's gap, delay or
/// roll can change after binding, as in C#).
pub(crate) struct TrainState {
    pub(crate) path: PathGeometry,
    pub(crate) progress: BoundTrack,
    pub(crate) options: PathTrainOptions,
    pub(crate) cars: Vec<PathTrainCar>,
    pub(crate) length: f64,
    pub(crate) rest_scale: f32,
}

impl TrainState {
    pub(crate) fn new(path: PathGeometry, progress: BoundTrack, options: PathTrainOptions, cars: Vec<PathTrainCar>) -> TrainState {
        let length = path.length_value();
        let rest_scale = match &options.spacing_scale {
            Some(track) if track.count() > 0 => *track.get_keyframes()[track.count() - 1].value(),
            _ => 1.0,
        };
        TrainState { path, progress, options, cars, length, rest_scale }
    }

    fn spacing_scale_at(&self, seconds: f64) -> f32 {
        match &self.options.spacing_scale {
            Some(track) if track.count() > 0 => {
                let value = track.evaluate(seconds).unwrap_or(1.0);
                if value.is_finite() {
                    value
                } else {
                    1.0
                }
            }
            _ => 1.0,
        }
    }

    fn distance(&self, car: &PathTrainCar, seconds: f64, spacing_scale: f32) -> f64 {
        let mut progress = match self.progress.value_at(seconds - car.delay) {
            Some(BoundValue::Float(v)) => v,
            _ => 0.0,
        };
        if progress.is_nan() {
            progress = 0.0;
        }
        let distance = (progress as f64 * self.length) - (car.gap as f64 * spacing_scale as f64);
        if self.options.extend_beyond_ends {
            distance
        } else {
            distance.clamp(0.0, self.length)
        }
    }
}

struct TrainDriver(Rc<RefCell<TrainState>>);

impl SessionDriver for TrainDriver {
    fn apply(&self, session: &mut DrawingSession, seconds: f64) {
        let state = self.0.borrow();
        let options = &state.options;
        let scale_now = state.spacing_scale_at(seconds);
        for car in &state.cars {
            let distance = state.distance(car, seconds, scale_now);
            let point = state.path.point_at_distance_extended(distance);
            let Some(element) = session.find_element_mut(car.element) else { continue };
            let current = element.transform();
            let reference = car.reference.unwrap_or(current.anchor());
            let translation = PointF::new(point.position.x - reference.x, point.position.y - reference.y);
            let mut next = current.with_translation(translation);
            let rolls = car.roll_radius != 0.0;
            if options.orient_to_path || rolls || options.rotation_offset != 0.0 {
                let mut rotation = options.rotation_offset as f64;
                if options.orient_to_path {
                    rotation += point.angle() as f64;
                }
                if rolls {
                    let upright = (options.upright_at_progress as f64 * state.length) - (car.gap as f64 * state.rest_scale as f64);
                    rotation += (distance - upright) / car.roll_radius as f64 * (180.0 / std::f64::consts::PI);
                }
                next = next.with_rotation(rotation as f32);
            }
            let fraction = if state.length > 0.0 { distance / state.length } else { 0.0 };
            if let Some(track) = options.scale_along_path.as_ref().filter(|t| t.count() > 0) {
                if let Some(scale) = track.evaluate(fraction) {
                    next = next.with_scale(scale);
                }
            }
            element.set_transform(next);
            if let Some(track) = options.opacity_along_path.as_ref().filter(|t| t.count() > 0) {
                if let Some(opacity) = track.evaluate(fraction) {
                    element.set_opacity(clamp_opacity(opacity));
                }
            }
        }
    }
}

/// A train of elements along one path (C# `PathTrain`): made by
/// [`DrawingAnimationsMut::bind_train_along_path`](crate::models::DrawingAnimationsMut::bind_train_along_path),
/// placed by ONE binding - a pure function of the time. Keep it to read where each car is, or to
/// change a car's gap, delay or roll later.
#[derive(Clone)]
pub struct PathTrain {
    pub(crate) state: Rc<RefCell<TrainState>>,
    pub(crate) binding: AnimationBinding,
}

impl PathTrain {
    pub(crate) fn driver(state: &Rc<RefCell<TrainState>>) -> Rc<dyn SessionDriver> {
        Rc::new(TrainDriver(state.clone()))
    }

    /// The path (C# `Path`).
    pub fn path(&self) -> PathGeometry {
        self.state.borrow().path.clone()
    }

    /// The lead's progress (C# `Progress`).
    pub fn progress(&self) -> BoundTrack {
        self.state.borrow().progress.clone()
    }

    /// A copy of the settings it was made with (C# `Options`).
    pub fn options(&self) -> PathTrainOptions {
        self.state.borrow().options.clone()
    }

    /// The cars, in the order of the elements given (C# `Cars`).
    pub fn cars(&self) -> Vec<PathTrainCar> {
        self.state.borrow().cars.clone()
    }

    /// The elements, in the order given (C# `Elements`).
    pub fn elements(&self) -> Vec<ElementId> {
        self.state.borrow().cars.iter().map(|c| c.element).collect()
    }

    /// The binding that places them (C# `Binding`).
    pub fn binding(&self) -> &AnimationBinding {
        &self.binding
    }

    /// Replaces car `index`'s gap, delay and roll radius with those of `car` (C#'s settable
    /// `PathTrainCar.Gap`, `Delay`, `RollRadius`; set them on a copy from [`cars`](PathTrain::cars)
    /// with its validated setters). The error says what to do with an index out of range.
    pub fn set_car(&self, index: usize, car: PathTrainCar) -> Result<(), String> {
        let mut state = self.state.borrow_mut();
        let count = state.cars.len();
        let target = state.cars.get_mut(index).ok_or_else(|| format!("The train has {count} cars: the index must be below that."))?;
        target.gap = car.gap;
        target.delay = car.delay;
        target.roll_radius = car.roll_radius;
        Ok(())
    }

    /// Car `index`'s distance along the path at a time (C# `GetDistance`).
    pub fn get_distance(&self, index: usize, seconds: f64) -> Result<f32, String> {
        let state = self.state.borrow();
        let car = state.cars.get(index).ok_or_else(|| "The car index is out of range.".to_string())?;
        Ok(state.distance(car, seconds, state.spacing_scale_at(seconds)) as f32)
    }

    /// Car `index`'s point on the path at a time (continued past the ends when extending) (C#
    /// `GetPoint`).
    pub fn get_point(&self, index: usize, seconds: f64) -> Result<PathPoint, String> {
        let state = self.state.borrow();
        let car = state.cars.get(index).ok_or_else(|| "The car index is out of range.".to_string())?;
        Ok(state.path.point_at_distance_extended(state.distance(car, seconds, state.spacing_scale_at(seconds))))
    }
}

impl std::fmt::Debug for PathTrain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PathTrain").field("cars", &self.cars()).field("binding", &self.binding).finish()
    }
}
