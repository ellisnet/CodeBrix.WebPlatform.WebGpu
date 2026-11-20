//! Crate-internal: the animatable attributes, their values, parsing, interpolation, addition and
//! distance (for `calcMode="paced"`), by the SVG animation rules.

use kurbo::Affine;

use crate::color::Color;

/// The attributes (and CSS properties) the crate can override.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum AnimAttr {
    Opacity,
    FillOpacity,
    StrokeOpacity,
    Fill,
    Stroke,
    StrokeWidth,
    StrokeDashoffset,
    StrokeDasharray,
    Visibility,
    Display,
    Transform,
    Motion,
}

impl AnimAttr {
    pub(crate) fn from_name(name: &str) -> Option<AnimAttr> {
        Some(match name {
            "opacity" => AnimAttr::Opacity,
            "fill-opacity" => AnimAttr::FillOpacity,
            "stroke-opacity" => AnimAttr::StrokeOpacity,
            "fill" => AnimAttr::Fill,
            "stroke" => AnimAttr::Stroke,
            "stroke-width" => AnimAttr::StrokeWidth,
            "stroke-dashoffset" => AnimAttr::StrokeDashoffset,
            "stroke-dasharray" => AnimAttr::StrokeDasharray,
            "visibility" => AnimAttr::Visibility,
            "display" => AnimAttr::Display,
            "transform" => AnimAttr::Transform,
            _ => return None,
        })
    }
}

/// The kind of an `animateTransform` (or one CSS transform function).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransformType {
    Translate,
    Scale,
    Rotate,
    SkewX,
    SkewY,
}

/// One value.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AnimValue {
    Number(f64),
    /// A colour as r, g, b, a (0..255 each); None = `none`.
    Paint(Option<[f64; 4]>),
    List(Vec<f64>),
    /// An `animateTransform` value: its parameters (translate tx ty, scale sx sy, rotate a cx cy,
    /// skew a).
    Transform(TransformType, [f64; 3]),
    Point(f64, f64),
    /// visibility / display: true = shown.
    Shown(bool),
}

fn numbers(text: &str) -> Option<Vec<f64>> {
    let mut out = Vec::new();
    for part in text.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()) {
        out.push(number(part)?);
    }
    Some(out)
}

/// A number with an optional `px` (or `%` = / 100 for opacities).
pub(crate) fn number(text: &str) -> Option<f64> {
    let t = text.trim();
    if let Some(p) = t.strip_suffix('%') {
        return p.trim().parse::<f64>().ok().map(|v| v / 100.0).filter(|v| v.is_finite());
    }
    let t = t.strip_suffix("px").unwrap_or(t);
    t.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

pub(crate) fn color(text: &str) -> Option<AnimValue> {
    let t = text.trim();
    if t == "none" || t == "transparent" {
        return Some(AnimValue::Paint(if t == "none" { None } else { Some([0.0, 0.0, 0.0, 0.0]) }));
    }
    let c = t.parse::<svgtypes::Color>().ok()?;
    Some(AnimValue::Paint(Some([c.red as f64, c.green as f64, c.blue as f64, c.alpha as f64])))
}

/// Parses one value of `attr`; `ty` is the animateTransform type.
pub(crate) fn parse(attr: AnimAttr, ty: Option<TransformType>, text: &str) -> Option<AnimValue> {
    let t = text.trim();
    match attr {
        AnimAttr::Opacity | AnimAttr::FillOpacity | AnimAttr::StrokeOpacity => number(t).map(|v| AnimValue::Number(v.clamp(0.0, 1.0))),
        AnimAttr::StrokeWidth | AnimAttr::StrokeDashoffset => {
            if t.ends_with('%') {
                return None;
            }
            number(t).map(AnimValue::Number)
        }
        AnimAttr::Fill | AnimAttr::Stroke => color(t),
        AnimAttr::StrokeDasharray => {
            if t == "none" {
                Some(AnimValue::List(Vec::new()))
            } else {
                numbers(t).map(AnimValue::List)
            }
        }
        AnimAttr::Visibility => match t {
            "visible" => Some(AnimValue::Shown(true)),
            "hidden" | "collapse" => Some(AnimValue::Shown(false)),
            _ => None,
        },
        AnimAttr::Display => Some(AnimValue::Shown(t != "none")),
        AnimAttr::Transform => {
            let n = numbers(t)?;
            let ty = ty?;
            let p = match (ty, n.len()) {
                (TransformType::Translate, 1) => [n[0], 0.0, 0.0],
                (TransformType::Translate, 2) => [n[0], n[1], 0.0],
                (TransformType::Scale, 1) => [n[0], n[0], 0.0],
                (TransformType::Scale, 2) => [n[0], n[1], 0.0],
                (TransformType::Rotate, 1) => [n[0], 0.0, 0.0],
                (TransformType::Rotate, 3) => [n[0], n[1], n[2]],
                (TransformType::SkewX | TransformType::SkewY, 1) => [n[0], 0.0, 0.0],
                _ => return None,
            };
            Some(AnimValue::Transform(ty, p))
        }
        AnimAttr::Motion => {
            let n = numbers(t)?;
            if n.len() == 2 {
                Some(AnimValue::Point(n[0], n[1]))
            } else {
                None
            }
        }
    }
}

/// a + (b - a) t; values of different shapes (or `none`) switch at the half.
pub(crate) fn lerp(a: &AnimValue, b: &AnimValue, t: f64) -> AnimValue {
    use AnimValue::*;
    let l = |x: f64, y: f64| x + (y - x) * t;
    match (a, b) {
        (Number(x), Number(y)) => Number(l(*x, *y)),
        (Paint(Some(x)), Paint(Some(y))) => Paint(Some([l(x[0], y[0]), l(x[1], y[1]), l(x[2], y[2]), l(x[3], y[3])])),
        (List(x), List(y)) if x.len() == y.len() && !x.is_empty() => List(x.iter().zip(y).map(|(p, q)| l(*p, *q)).collect()),
        (Transform(ta, x), Transform(tb, y)) if ta == tb => Transform(*ta, [l(x[0], y[0]), l(x[1], y[1]), l(x[2], y[2])]),
        (Point(x0, y0), Point(x1, y1)) => Point(l(*x0, *x1), l(*y0, *y1)),
        _ => {
            if t < 0.5 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

/// a + b (additive and accumulate); `none` and booleans do not add (b wins).
pub(crate) fn add(a: &AnimValue, b: &AnimValue) -> AnimValue {
    use AnimValue::*;
    match (a, b) {
        (Number(x), Number(y)) => Number(x + y),
        (Paint(Some(x)), Paint(Some(y))) => Paint(Some([x[0] + y[0], x[1] + y[1], x[2] + y[2], x[3].max(y[3])])),
        (List(x), List(y)) if x.len() == y.len() => List(x.iter().zip(y).map(|(p, q)| p + q).collect()),
        (Transform(ta, x), Transform(tb, y)) if ta == tb => Transform(*ta, [x[0] + y[0], x[1] + y[1], x[2] + y[2]]),
        (Point(x0, y0), Point(x1, y1)) => Point(x0 + x1, y0 + y1),
        _ => b.clone(),
    }
}

/// value x n (accumulate = sum: the last value times the repeat iteration).
pub(crate) fn scale(a: &AnimValue, n: f64) -> AnimValue {
    use AnimValue::*;
    match a {
        Number(x) => Number(x * n),
        Paint(Some(x)) => Paint(Some([x[0] * n, x[1] * n, x[2] * n, x[3]])),
        List(x) => List(x.iter().map(|v| v * n).collect()),
        Transform(t, x) => Transform(*t, [x[0] * n, x[1] * n, x[2] * n]),
        Point(x, y) => Point(x * n, y * n),
        other => other.clone(),
    }
}

/// The distance used by `calcMode="paced"`.
pub(crate) fn distance(a: &AnimValue, b: &AnimValue) -> Option<f64> {
    use AnimValue::*;
    Some(match (a, b) {
        (Number(x), Number(y)) => (x - y).abs(),
        (Paint(Some(x)), Paint(Some(y))) => ((x[0] - y[0]).powi(2) + (x[1] - y[1]).powi(2) + (x[2] - y[2]).powi(2)).sqrt(),
        (List(x), List(y)) if x.len() == y.len() => x.iter().zip(y).map(|(p, q)| (p - q).powi(2)).sum::<f64>().sqrt(),
        (Transform(TransformType::Rotate | TransformType::SkewX | TransformType::SkewY, x), Transform(_, y)) => (x[0] - y[0]).abs(),
        (Transform(_, x), Transform(_, y)) => ((x[0] - y[0]).powi(2) + (x[1] - y[1]).powi(2)).sqrt(),
        (Point(x0, y0), Point(x1, y1)) => ((x0 - x1).powi(2) + (y0 - y1).powi(2)).sqrt(),
        _ => return None,
    })
}

/// The matrix of an animateTransform value.
pub(crate) fn transform_matrix(ty: TransformType, p: [f64; 3]) -> Affine {
    match ty {
        TransformType::Translate => Affine::translate((p[0], p[1])),
        TransformType::Scale => Affine::scale_non_uniform(p[0], p[1]),
        TransformType::Rotate => Affine::translate((p[1], p[2])) * Affine::rotate(p[0].to_radians()) * Affine::translate((-p[1], -p[2])),
        TransformType::SkewX => Affine::skew(p[0].to_radians().tan(), 0.0),
        TransformType::SkewY => Affine::skew(0.0, p[0].to_radians().tan()),
    }
}

/// A paint value as a crate colour (clamped, rounded).
pub(crate) fn to_color(v: &[f64; 4]) -> Color {
    let c = |x: f64| x.round().clamp(0.0, 255.0) as u8;
    Color::from_rgba(c(v[0]), c(v[1]), c(v[2]), c(v[3]))
}

/// A crate colour as a paint value.
pub(crate) fn from_color(c: Option<Color>) -> AnimValue {
    AnimValue::Paint(c.map(|c| [c.r as f64, c.g as f64, c.b as f64, c.a as f64]))
}
