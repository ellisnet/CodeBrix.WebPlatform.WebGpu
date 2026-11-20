//! `EasingKind`: the named easing curves (C# `EasingKind`, same names and numeric values).

/// A named easing curve, for [`Easing::evaluate`](crate::animation::Easing::evaluate) and
/// [`Easing::get_function`](crate::animation::Easing::get_function). The numeric values are the
/// C# enum's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum EasingKind {
    /// No easing.
    Linear = 0,
    /// Smoothstep.
    SmoothStep = 1,
    /// Smootherstep.
    SmootherStep = 2,
    /// Quadratic in.
    QuadIn = 10,
    /// Quadratic out.
    QuadOut = 11,
    /// Quadratic in and out.
    QuadInOut = 12,
    /// Cubic in.
    CubicIn = 20,
    /// Cubic out.
    CubicOut = 21,
    /// Cubic in and out.
    CubicInOut = 22,
    /// Quartic in.
    QuartIn = 30,
    /// Quartic out.
    QuartOut = 31,
    /// Quartic in and out.
    QuartInOut = 32,
    /// Quintic in.
    QuintIn = 40,
    /// Quintic out.
    QuintOut = 41,
    /// Quintic in and out.
    QuintInOut = 42,
    /// Sine in.
    SineIn = 50,
    /// Sine out.
    SineOut = 51,
    /// Sine in and out.
    SineInOut = 52,
    /// Exponential in.
    ExpoIn = 60,
    /// Exponential out.
    ExpoOut = 61,
    /// Exponential in and out.
    ExpoInOut = 62,
    /// Circular in.
    CircIn = 70,
    /// Circular out.
    CircOut = 71,
    /// Circular in and out.
    CircInOut = 72,
    /// Back in (overshoots below 0 at the start).
    BackIn = 80,
    /// Back out (overshoots above 1 before the end).
    BackOut = 81,
    /// Back in and out.
    BackInOut = 82,
    /// Elastic in.
    ElasticIn = 90,
    /// Elastic out.
    ElasticOut = 91,
    /// Elastic in and out.
    ElasticInOut = 92,
    /// Bounce in.
    BounceIn = 100,
    /// Bounce out.
    BounceOut = 101,
    /// Bounce in and out.
    BounceInOut = 102,
}

impl EasingKind {
    /// Every kind, in the C# enum's declaration order (what `Enum.GetValues<EasingKind>()` gives).
    pub const ALL: [EasingKind; 33] = [
        EasingKind::Linear, EasingKind::SmoothStep, EasingKind::SmootherStep,
        EasingKind::QuadIn, EasingKind::QuadOut, EasingKind::QuadInOut,
        EasingKind::CubicIn, EasingKind::CubicOut, EasingKind::CubicInOut,
        EasingKind::QuartIn, EasingKind::QuartOut, EasingKind::QuartInOut,
        EasingKind::QuintIn, EasingKind::QuintOut, EasingKind::QuintInOut,
        EasingKind::SineIn, EasingKind::SineOut, EasingKind::SineInOut,
        EasingKind::ExpoIn, EasingKind::ExpoOut, EasingKind::ExpoInOut,
        EasingKind::CircIn, EasingKind::CircOut, EasingKind::CircInOut,
        EasingKind::BackIn, EasingKind::BackOut, EasingKind::BackInOut,
        EasingKind::ElasticIn, EasingKind::ElasticOut, EasingKind::ElasticInOut,
        EasingKind::BounceIn, EasingKind::BounceOut, EasingKind::BounceInOut,
    ];

    /// The kind's name as C# prints it (`kind.ToString()`), for example "QuadInOut".
    pub fn name(self) -> &'static str {
        match self {
            EasingKind::Linear => "Linear",
            EasingKind::SmoothStep => "SmoothStep",
            EasingKind::SmootherStep => "SmootherStep",
            EasingKind::QuadIn => "QuadIn",
            EasingKind::QuadOut => "QuadOut",
            EasingKind::QuadInOut => "QuadInOut",
            EasingKind::CubicIn => "CubicIn",
            EasingKind::CubicOut => "CubicOut",
            EasingKind::CubicInOut => "CubicInOut",
            EasingKind::QuartIn => "QuartIn",
            EasingKind::QuartOut => "QuartOut",
            EasingKind::QuartInOut => "QuartInOut",
            EasingKind::QuintIn => "QuintIn",
            EasingKind::QuintOut => "QuintOut",
            EasingKind::QuintInOut => "QuintInOut",
            EasingKind::SineIn => "SineIn",
            EasingKind::SineOut => "SineOut",
            EasingKind::SineInOut => "SineInOut",
            EasingKind::ExpoIn => "ExpoIn",
            EasingKind::ExpoOut => "ExpoOut",
            EasingKind::ExpoInOut => "ExpoInOut",
            EasingKind::CircIn => "CircIn",
            EasingKind::CircOut => "CircOut",
            EasingKind::CircInOut => "CircInOut",
            EasingKind::BackIn => "BackIn",
            EasingKind::BackOut => "BackOut",
            EasingKind::BackInOut => "BackInOut",
            EasingKind::ElasticIn => "ElasticIn",
            EasingKind::ElasticOut => "ElasticOut",
            EasingKind::ElasticInOut => "ElasticInOut",
            EasingKind::BounceIn => "BounceIn",
            EasingKind::BounceOut => "BounceOut",
            EasingKind::BounceInOut => "BounceInOut",
        }
    }
}
