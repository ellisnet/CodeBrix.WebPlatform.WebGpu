//! `BoundTrack`: the source of a binding's values - a keyframe track of one of the four kinds, or
//! a function of the time (C#'s `Bind(..., KeyframeTrack<T>)` and `Bind(..., Func<double, T>)`).

use std::fmt;
use std::rc::Rc;

use crate::animation::{BoolTrack, ColorTrack, FloatTrack, PointTrack};
use crate::color::Color;
use crate::geometry::PointF;

/// Where a binding's value at a time comes from.
#[derive(Clone)]
pub enum BoundTrack {
    /// A float track.
    Float(FloatTrack),
    /// A point track.
    Point(PointTrack),
    /// A colour track.
    Color(ColorTrack),
    /// A bool track.
    Bool(BoolTrack),
    /// A float function of the time (C# `Func<double, float> valueAt`).
    FloatFn(Rc<dyn Fn(f64) -> f32>),
    /// A point function of the time (C# `Func<double, PointF> valueAt`).
    PointFn(Rc<dyn Fn(f64) -> PointF>),
    /// A colour function of the time (no C# equivalent): binds wherever a colour track binds -
    /// a layer's colour, a group's content colour, a picture's or sprite's tint, a glow's or
    /// shadow's colour - without a keyframe track (for example `ColorMix::mix(a, b, ease(t),
    /// ColorInterpolation::Oklab)`).
    ColorFn(Rc<dyn Fn(f64) -> Color>),
}

/// A binding's value at one time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum BoundValue {
    Float(f32),
    Point(PointF),
    Color(Color),
    Bool(bool),
}

impl BoundTrack {
    /// A float function of the time (it must not read a clock: the time is given).
    pub fn float_fn(value_at: impl Fn(f64) -> f32 + 'static) -> BoundTrack {
        BoundTrack::FloatFn(Rc::new(value_at))
    }

    /// A point function of the time.
    pub fn point_fn(value_at: impl Fn(f64) -> PointF + 'static) -> BoundTrack {
        BoundTrack::PointFn(Rc::new(value_at))
    }

    /// A colour function of the time (no C# equivalent; it must not read a clock: the time is
    /// given) - see [`BoundTrack::ColorFn`].
    pub fn color_fn(value_at: impl Fn(f64) -> Color + 'static) -> BoundTrack {
        BoundTrack::ColorFn(Rc::new(value_at))
    }

    /// "float", "point", "color" or "bool" (for messages).
    pub(crate) fn kind_name(&self) -> &'static str {
        match self {
            BoundTrack::Float(_) | BoundTrack::FloatFn(_) => "float",
            BoundTrack::Point(_) | BoundTrack::PointFn(_) => "point",
            BoundTrack::Color(_) | BoundTrack::ColorFn(_) => "color",
            BoundTrack::Bool(_) => "bool",
        }
    }

    /// The value at a time (None for a track with no keys).
    pub(crate) fn value_at(&self, seconds: f64) -> Option<BoundValue> {
        match self {
            BoundTrack::Float(t) => t.evaluate(seconds).map(BoundValue::Float),
            BoundTrack::Point(t) => t.evaluate(seconds).map(BoundValue::Point),
            BoundTrack::Color(t) => t.evaluate(seconds).map(BoundValue::Color),
            BoundTrack::Bool(t) => t.evaluate(seconds).map(BoundValue::Bool),
            BoundTrack::FloatFn(f) => Some(BoundValue::Float(f(seconds))),
            BoundTrack::PointFn(f) => Some(BoundValue::Point(f(seconds))),
            BoundTrack::ColorFn(f) => Some(BoundValue::Color(f(seconds))),
        }
    }
}

impl From<FloatTrack> for BoundTrack {
    fn from(t: FloatTrack) -> BoundTrack {
        BoundTrack::Float(t)
    }
}

impl From<PointTrack> for BoundTrack {
    fn from(t: PointTrack) -> BoundTrack {
        BoundTrack::Point(t)
    }
}

impl From<ColorTrack> for BoundTrack {
    fn from(t: ColorTrack) -> BoundTrack {
        BoundTrack::Color(t)
    }
}

impl From<BoolTrack> for BoundTrack {
    fn from(t: BoolTrack) -> BoundTrack {
        BoundTrack::Bool(t)
    }
}

impl fmt::Debug for BoundTrack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoundTrack::Float(t) => f.debug_tuple("Float").field(t).finish(),
            BoundTrack::Point(t) => f.debug_tuple("Point").field(t).finish(),
            BoundTrack::Color(t) => f.debug_tuple("Color").field(t).finish(),
            BoundTrack::Bool(t) => f.debug_tuple("Bool").field(t).finish(),
            BoundTrack::FloatFn(_) => f.write_str("FloatFn"),
            BoundTrack::PointFn(_) => f.write_str("PointFn"),
            BoundTrack::ColorFn(_) => f.write_str("ColorFn"),
        }
    }
}
