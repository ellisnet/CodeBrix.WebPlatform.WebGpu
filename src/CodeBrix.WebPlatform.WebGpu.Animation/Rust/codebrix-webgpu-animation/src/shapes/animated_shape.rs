//! `AnimatedShape`: an element rebuilt for each moment by a function of the time (C#
//! `AnimatedShape`).

use std::fmt;
use std::rc::Rc;

use crate::geometry::RectangleF;
use crate::shapes::DrawingShape;

/// An element whose shape is made, for each moment, by a FACTORY of the session time (C#
/// `AnimatedShape`): the session calls [`AnimatedShape::update`] whenever its time is set, and
/// draws what the factory returned (None draws nothing). The factory must be a pure function of
/// the time it is given - it never reads a clock.
#[derive(Clone)]
pub struct AnimatedShape {
    factory: Rc<dyn Fn(f64) -> Option<DrawingShape>>,
    current: Option<Box<DrawingShape>>,
    time: f64,
}

impl AnimatedShape {
    /// An animated shape, made at `initial_time` at once (C# `AnimatedShape(factory,
    /// initialTime = 0)`).
    pub fn new(factory: impl Fn(f64) -> Option<DrawingShape> + 'static, initial_time: f64) -> AnimatedShape {
        let mut shape = AnimatedShape { factory: Rc::new(factory), current: None, time: initial_time };
        shape.update(initial_time);
        shape
    }

    /// The shape made for the last time it was updated to (C# `Current`).
    pub fn current(&self) -> Option<&DrawingShape> {
        self.current.as_deref()
    }

    /// The time it was last made for (C# `Time`).
    pub fn time(&self) -> f64 {
        self.time
    }

    /// Makes the shape for a time (C# `Update`).
    pub fn update(&mut self, seconds: f64) {
        self.time = seconds;
        self.current = (self.factory)(seconds).map(Box::new);
    }

    /// The current shape's transformed bounds (an empty rectangle when there is none; C#
    /// `GetBounds`).
    pub fn get_bounds(&self) -> Option<RectangleF> {
        match &self.current {
            None => Some(RectangleF::default()),
            Some(c) => c.get_transformed_bounds(),
        }
    }
}

impl PartialEq for AnimatedShape {
    fn eq(&self, other: &AnimatedShape) -> bool {
        Rc::ptr_eq(&self.factory, &other.factory) && self.current == other.current && self.time == other.time
    }
}

impl fmt::Debug for AnimatedShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnimatedShape").field("time", &self.time).field("current", &self.current).finish()
    }
}
