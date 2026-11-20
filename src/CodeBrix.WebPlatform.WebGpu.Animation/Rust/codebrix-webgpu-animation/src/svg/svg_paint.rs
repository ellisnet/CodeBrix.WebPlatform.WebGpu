//! `SvgPaint` and the paint servers of a scene: solid colours, linear and radial gradients, fills.

use std::sync::Arc;

use kurbo::Affine;

use crate::color::Color;

/// The fill rule of a fill or a clip shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SvgFillRule {
    /// `nonzero`.
    #[default]
    NonZero,
    /// `evenodd`.
    EvenOdd,
}

/// How a gradient continues beyond its ends (SVG `spreadMethod`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SvgSpread {
    /// `pad`.
    #[default]
    Pad,
    /// `reflect`.
    Reflect,
    /// `repeat`.
    Repeat,
}

/// One gradient stop: offset 0..1 and a straight-alpha sRGB colour (the stop's `stop-opacity`
/// is in its alpha).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SvgGradientStop {
    /// The offset, 0..1, never decreasing along the list.
    pub offset: f32,
    /// The colour with the stop's opacity as alpha.
    pub color: Color,
}

/// A linear gradient in the painted path's own coordinates, placed by `transform`.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgLinearGradient {
    /// The gradient's id in the document (may be empty).
    pub id: String,
    /// The start point.
    pub start: (f64, f64),
    /// The end point.
    pub end: (f64, f64),
    /// `gradientTransform` (with objectBoundingBox units already turned into user space).
    pub transform: Affine,
    /// What happens beyond the ends.
    pub spread: SvgSpread,
    /// The stops.
    pub stops: Vec<SvgGradientStop>,
}

/// A radial (two-point conical) gradient in the painted path's own coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgRadialGradient {
    /// The gradient's id in the document (may be empty).
    pub id: String,
    /// The end circle's centre (`cx`, `cy`).
    pub center: (f64, f64),
    /// The end circle's radius (`r`).
    pub radius: f64,
    /// The focal point (`fx`, `fy`).
    pub focal: (f64, f64),
    /// The focal circle's radius (`fr`).
    pub focal_radius: f64,
    /// `gradientTransform` (objectBoundingBox units already turned into user space).
    pub transform: Affine,
    /// What happens beyond the ends.
    pub spread: SvgSpread,
    /// The stops.
    pub stops: Vec<SvgGradientStop>,
}

/// What a fill or a stroke paints with.
#[derive(Clone, Debug, PartialEq)]
pub enum SvgPaint {
    /// A solid colour (straight alpha; alpha 255 - the fill's or stroke's opacity is separate).
    Solid(Color),
    /// A linear gradient.
    Linear(Arc<SvgLinearGradient>),
    /// A radial gradient.
    Radial(Arc<SvgRadialGradient>),
    /// A `<pattern>` paint server, which the renderer cannot paint yet: drawn as NOTHING and
    /// reported (`DrawingSvgWarningKind::PatternNotDrawn`); the id is kept for the report.
    Pattern(String),
}

/// A fill: paint, opacity (`fill-opacity`, 0..1) and rule.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgFill {
    /// The paint.
    pub paint: SvgPaint,
    /// `fill-opacity`, 0..1.
    pub opacity: f32,
    /// `fill-rule`.
    pub rule: SvgFillRule,
}
