//! `TrackValue`: what a keyframe track can hold and how it is interpolated (the C# track classes'
//! `Interpolate` overrides: FloatTrack, PointTrack, ColorTrack, BoolTrack).

use crate::animation::color_ramp::channel;
use crate::animation::Easing;
use crate::color::Color;
use crate::geometry::PointF;

/// A value a [`KeyframeTrack`](crate::animation::KeyframeTrack) holds, with the C# track's
/// interpolation between two keys (`fraction` is already eased, and may pass 0..1 for an
/// overshooting curve).
pub trait TrackValue: Clone + std::fmt::Debug {
    /// The value `fraction` of the way from `from` to `to`.
    fn interpolate(from: &Self, to: &Self, fraction: f64) -> Self;
}

impl TrackValue for f32 {
    /// C# `FloatTrack`: `Easing.Lerp(from, to, fraction)`.
    fn interpolate(from: &f32, to: &f32, fraction: f64) -> f32 {
        Easing::lerp_f32(*from, *to, fraction)
    }
}

impl TrackValue for PointF {
    /// C# `PointTrack`: X and Y each by `Easing.Lerp`.
    fn interpolate(from: &PointF, to: &PointF, fraction: f64) -> PointF {
        PointF::new(Easing::lerp_f32(from.x, to.x, fraction), Easing::lerp_f32(from.y, to.y, fraction))
    }
}

impl TrackValue for Color {
    /// C# `ColorTrack`: each 8-bit channel (straight sRGB, alpha too) by `ColorRamp.Channel`
    /// (interpolated, rounded half away from zero, clamped).
    fn interpolate(from: &Color, to: &Color, fraction: f64) -> Color {
        Color::from_rgba(channel(from.r, to.r, fraction), channel(from.g, to.g, fraction), channel(from.b, to.b, fraction), channel(from.a, to.a, fraction))
    }
}

impl TrackValue for bool {
    /// C# `BoolTrack`: a step - the earlier key's value until the next key.
    fn interpolate(from: &bool, _to: &bool, _fraction: f64) -> bool {
        *from
    }
}
