//! Crate-internal: SMIL animation elements (`animate`, `set`, `animateTransform`,
//! `animateMotion`) read from the XML usvg drops them from, and their value at a phase.

use kurbo::{Affine, BezPath};
use usvg::roxmltree;

use crate::animation::CubicBezierEasing;
use crate::paths::{PathFillRule, PathGeometry};
use crate::svg::animation::smil_timing::{self, Restart, TimeSpec, Timing};
use crate::svg::animation::svg_value::{self, AnimAttr, AnimValue, TransformType};

/// `calcMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CalcMode {
    Discrete,
    Linear,
    Paced,
    Spline,
}

/// How the value list was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueMode {
    /// `values`, from-to (two values), or `set`'s `to`.
    Values,
    /// by-only and from-by: always additive (by-only from zero).
    By,
    /// to-only: from the underlying value to `to`.
    To,
}

/// `rotate` of animateMotion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MotionRotate {
    Angle(f64),
    Auto,
    AutoReverse,
}

/// animateMotion's path (the motion follows it; values are fractions of its length).
#[derive(Clone, Debug)]
pub(crate) struct MotionPath {
    pub(crate) geometry: PathGeometry,
}

/// One animation element.
#[derive(Clone, Debug)]
pub(crate) struct SmilAnimation {
    pub(crate) element: &'static str,
    /// The XML id of the animation element itself (for syncbases), "" for none.
    pub(crate) own_id: String,
    /// The target element (an id the crate gave it when it had none).
    pub(crate) target: roxmltree::NodeId,
    pub(crate) attr: AnimAttr,
    pub(crate) ty: Option<TransformType>,
    pub(crate) timing: Timing,
    pub(crate) values: Vec<AnimValue>,
    pub(crate) mode: ValueMode,
    pub(crate) calc: CalcMode,
    pub(crate) key_times: Option<Vec<f64>>,
    pub(crate) key_splines: Option<Vec<CubicBezierEasing>>,
    pub(crate) additive: bool,
    pub(crate) accumulate: bool,
    pub(crate) motion_path: Option<MotionPath>,
    pub(crate) rotate: MotionRotate,
    pub(crate) order: usize,
    /// Begin / end entries that are not time-based (events ...), played without (for the report).
    pub(crate) ignored_times: Vec<String>,
}

pub(crate) const SMIL_ELEMENTS: [&str; 4] = ["animate", "set", "animateTransform", "animateMotion"];

fn href<'a>(e: &roxmltree::Node<'a, 'a>) -> Option<&'a str> {
    e.attribute("href").or_else(|| e.attribute(("http://www.w3.org/1999/xlink", "href")))
}

/// Reads one animation element; Err = why it is ignored (for the report).
pub(crate) fn read(e: roxmltree::Node, doc: &roxmltree::Document, order: usize) -> Result<SmilAnimation, String> {
    let name = e.tag_name().name();
    let element: &'static str = SMIL_ELEMENTS.iter().find(|n| **n == name).copied().ok_or("not an animation element")?;
    let target = match href(&e) {
        Some(h) => {
            let id = h.trim().strip_prefix('#').ok_or_else(|| format!("its href '{h}' is not a '#id' reference"))?;
            doc.descendants().find(|n| n.is_element() && n.attribute("id") == Some(id)).ok_or_else(|| format!("its target '#{id}' does not exist"))?
        }
        None => e.parent_element().ok_or("it has no parent element")?,
    };
    let (attr, ty) = if element == "animateMotion" {
        (AnimAttr::Motion, None)
    } else {
        let a = e.attribute("attributeName").ok_or("it has no attributeName")?;
        if element == "animateTransform" {
            if a != "transform" && a != "gradientTransform" && a != "patternTransform" {
                return Err(format!("animateTransform of '{a}' is not supported (only 'transform')"));
            }
            if a != "transform" {
                return Err(format!("'{a}' (a paint server's transform) cannot be overridden"));
            }
            let ty = match e.attribute("type").unwrap_or("translate") {
                "translate" => TransformType::Translate,
                "scale" => TransformType::Scale,
                "rotate" => TransformType::Rotate,
                "skewX" => TransformType::SkewX,
                "skewY" => TransformType::SkewY,
                other => return Err(format!("the transform type '{other}' is not one of translate, scale, rotate, skewX, skewY")),
            };
            (AnimAttr::Transform, Some(ty))
        } else {
            let attr = AnimAttr::from_name(a).ok_or_else(|| format!("the attribute '{a}' cannot be overridden (supported: opacity, fill-opacity, stroke-opacity, fill, stroke, stroke-width, stroke-dashoffset, stroke-dasharray, visibility, display)"))?;
            if attr == AnimAttr::Transform {
                return Err("animate on 'transform' is not valid SMIL (use animateTransform)".to_string());
            }
            (attr, None)
        }
    };
    let timing = timing(&e, element)?;
    let parse = |t: &str| svg_value::parse(attr, ty, t).ok_or_else(|| format!("the value '{t}' does not parse for '{}'", e.attribute("attributeName").unwrap_or("motion")));
    let mut mode = ValueMode::Values;
    let mut values: Vec<AnimValue> = Vec::new();
    let mut motion_path = None;
    if element == "animateMotion" {
        let mpath = e.children().find(|c| c.is_element() && c.tag_name().name() == "mpath");
        let path_text = if let Some(m) = mpath {
            let id = href(&m).and_then(|h| h.strip_prefix('#')).ok_or("its mpath has no '#id' href")?;
            let p = doc.descendants().find(|n| n.is_element() && n.attribute("id") == Some(id)).ok_or_else(|| format!("its mpath target '#{id}' does not exist"))?;
            Some(p.attribute("d").ok_or("its mpath target is not a <path> with 'd'")?.to_string())
        } else {
            e.attribute("path").map(|s| s.to_string())
        };
        if let Some(d) = path_text {
            let bez = BezPath::from_svg(&d).map_err(|_| "its motion path does not parse".to_string())?;
            motion_path = Some(MotionPath { geometry: PathGeometry::from_bez_path(bez, PathFillRule::NonZero) });
        }
    }
    if motion_path.is_some() {
        if let Some(kp) = e.attribute("keyPoints") {
            values = kp.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| svg_value::number(s).map(|v| AnimValue::Number(v.clamp(0.0, 1.0))).ok_or_else(|| format!("keyPoints value '{s}' does not parse"))).collect::<Result<_, _>>()?;
        } else {
            values = vec![AnimValue::Number(0.0), AnimValue::Number(1.0)];
        }
    } else if element == "set" {
        values.push(parse(e.attribute("to").ok_or("set has no 'to'")?)?);
    } else if let Some(v) = e.attribute("values") {
        values = v.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).map(parse).collect::<Result<_, _>>()?;
    } else {
        let from = e.attribute("from").map(parse).transpose()?;
        let to = e.attribute("to").map(parse).transpose()?;
        let by = e.attribute("by").map(parse).transpose()?;
        match (from, to, by) {
            (Some(f), Some(t), _) => values = vec![f, t],
            (Some(f), None, Some(b)) => {
                values = vec![f.clone(), svg_value::add(&f, &b)];
            }
            (None, Some(t), _) => {
                mode = ValueMode::To;
                values = vec![t];
            }
            (None, None, Some(b)) => {
                mode = ValueMode::By;
                values = vec![svg_value::scale(&b, 0.0), b];
            }
            _ => return Err("it has none of values, from/to, from/by, to or by".to_string()),
        }
    }
    if values.is_empty() {
        return Err("its value list is empty".to_string());
    }
    let default_calc = if element == "animateMotion" { CalcMode::Paced } else { CalcMode::Linear };
    let mut calc = match e.attribute("calcMode") {
        None => default_calc,
        Some("discrete") => CalcMode::Discrete,
        Some("linear") => CalcMode::Linear,
        Some("paced") => CalcMode::Paced,
        Some("spline") => CalcMode::Spline,
        Some(o) => return Err(format!("calcMode '{o}' is not one of discrete, linear, paced, spline")),
    };
    if element == "set" || matches!(attr, AnimAttr::Visibility | AnimAttr::Display) {
        calc = CalcMode::Discrete;
    }
    let key_times = e.attribute("keyTimes").map(|k| k.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.parse::<f64>().map_err(|_| format!("keyTimes value '{s}' does not parse"))).collect::<Result<Vec<_>, _>>()).transpose()?;
    if let Some(k) = &key_times {
        let ok = k.len() == values.len() && k.first() == Some(&0.0) && k.windows(2).all(|w| w[0] <= w[1]) && (calc == CalcMode::Discrete || k.last() == Some(&1.0));
        if !ok && calc != CalcMode::Paced {
            return Err("its keyTimes do not match its values (same count, starting at 0, never decreasing, ending at 1 unless discrete)".to_string());
        }
    }
    let key_splines = if calc == CalcMode::Spline {
        let k = e.attribute("keySplines").ok_or("calcMode spline without keySplines")?;
        let groups: Vec<&str> = k.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let mut out = Vec::new();
        for g in groups {
            let n: Vec<f64> = g.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()).filter_map(|s| s.parse::<f64>().ok()).collect();
            if n.len() != 4 {
                return Err(format!("keySplines entry '{g}' is not four numbers"));
            }
            out.push(CubicBezierEasing::new(n[0], n[1], n[2], n[3]).ok_or_else(|| format!("keySplines entry '{g}' is outside 0..1"))?);
        }
        if out.len() + 1 != values.len() {
            return Err("keySplines must have one entry fewer than values".to_string());
        }
        Some(out)
    } else {
        None
    };
    let rotate = match e.attribute("rotate") {
        None => MotionRotate::Angle(0.0),
        Some("auto") => MotionRotate::Auto,
        Some("auto-reverse") => MotionRotate::AutoReverse,
        Some(v) => MotionRotate::Angle(v.trim().parse::<f64>().map_err(|_| format!("rotate '{v}' is not auto, auto-reverse or a number"))?),
    };
    let additive = match mode {
        ValueMode::By => true,
        ValueMode::To => false,
        ValueMode::Values => e.attribute("additive") == Some("sum") && element != "set",
    };
    let accumulate = e.attribute("accumulate") == Some("sum") && mode != ValueMode::To && element != "set";
    let ignored_times = timing.begin.iter().chain(timing.end.iter()).filter_map(|s| if let TimeSpec::Ignored(x) = s { Some(x.clone()) } else { None }).collect();
    Ok(SmilAnimation { element, own_id: e.attribute("id").unwrap_or("").to_string(), target: target.id(), attr, ty, timing, values, mode, calc, key_times, key_splines, additive, accumulate, motion_path, rotate, order, ignored_times })
}

fn timing(e: &roxmltree::Node, element: &str) -> Result<Timing, String> {
    let begin = smil_timing::time_list(e.attribute("begin").unwrap_or("0s"));
    let end = e.attribute("end").map(smil_timing::time_list).unwrap_or_default();
    let dur = match e.attribute("dur") {
        None | Some("indefinite") | Some("media") => None,
        Some(d) => Some(smil_timing::clock(d).filter(|v| *v > 0.0).ok_or_else(|| format!("dur '{d}' is not a positive clock value"))?),
    };
    let _ = element;
    let repeat_count = match e.attribute("repeatCount") {
        None => None,
        Some("indefinite") => Some(f64::INFINITY),
        Some(v) => Some(v.trim().parse::<f64>().ok().filter(|n| *n > 0.0).ok_or_else(|| format!("repeatCount '{v}' is not a positive number"))?),
    };
    let repeat_dur = match e.attribute("repeatDur") {
        None => None,
        Some("indefinite") => Some(f64::INFINITY),
        Some(v) => Some(smil_timing::clock(v).ok_or_else(|| format!("repeatDur '{v}' is not a clock value"))?),
    };
    let min = e.attribute("min").and_then(smil_timing::clock).unwrap_or(0.0);
    let max = match e.attribute("max") {
        None | Some("indefinite") => f64::INFINITY,
        Some(v) => smil_timing::clock(v).unwrap_or(f64::INFINITY),
    };
    let restart = match e.attribute("restart") {
        Some("never") => Restart::Never,
        Some("whenNotActive") => Restart::WhenNotActive,
        _ => Restart::Always,
    };
    if !begin.iter().any(|b| matches!(b, TimeSpec::Offset(_) | TimeSpec::Sync { .. })) {
        let ignored: Vec<String> = begin.iter().filter_map(|b| if let TimeSpec::Ignored(s) = b { Some(s.clone()) } else { None }).collect();
        return Err(format!("it has no time-based begin (begin=\"{}\": events and other non-time begins are not played)", if ignored.is_empty() { "indefinite".to_string() } else { ignored.join(";") }));
    }
    Ok(Timing { begin, end, dur, repeat_count, repeat_dur, min, max: max.max(min), restart, freeze: e.attribute("fill") == Some("freeze") })
}

/// The value list's position at progress `p` (0..1): the value, and the segment's direction for
/// motion (`rotate="auto"`).
pub(crate) fn interpolate(values: &[AnimValue], calc: CalcMode, key_times: Option<&[f64]>, key_splines: Option<&[CubicBezierEasing]>, p: f64) -> AnimValue {
    let n = values.len();
    if n == 1 {
        return values[0].clone();
    }
    let p = p.clamp(0.0, 1.0);
    match calc {
        CalcMode::Discrete => {
            let i = match key_times {
                Some(k) => k.iter().rposition(|t| *t <= p).unwrap_or(0),
                None => ((p * n as f64).floor() as usize).min(n - 1),
            };
            values[i].clone()
        }
        CalcMode::Paced => {
            let d: Vec<f64> = values.windows(2).map(|w| svg_value::distance(&w[0], &w[1]).unwrap_or(0.0)).collect();
            let total: f64 = d.iter().sum();
            if total <= 0.0 {
                return values[0].clone();
            }
            let target = p * total;
            let mut at = 0.0;
            for (i, seg) in d.iter().enumerate() {
                if target <= at + seg || i == d.len() - 1 {
                    let local = if *seg > 0.0 { (target - at) / seg } else { 0.0 };
                    return svg_value::lerp(&values[i], &values[i + 1], local.clamp(0.0, 1.0));
                }
                at += seg;
            }
            values[n - 1].clone()
        }
        CalcMode::Linear | CalcMode::Spline => {
            let uniform: Vec<f64>;
            let k = match key_times {
                Some(k) => k,
                None => {
                    uniform = (0..n).map(|i| i as f64 / (n - 1) as f64).collect();
                    &uniform
                }
            };
            let i = (0..n - 1).find(|i| p <= k[i + 1]).unwrap_or(n - 2);
            let span = k[i + 1] - k[i];
            let mut local = if span > 0.0 { (p - k[i]) / span } else { 1.0 };
            if let Some(s) = key_splines {
                local = s[i].evaluate(local);
            }
            svg_value::lerp(&values[i], &values[i + 1], local)
        }
    }
}

impl SmilAnimation {
    /// The value at a phase, before additive composition (`underlying` = the value below it in
    /// the sandwich, for to-animations).
    pub(crate) fn value(&self, simple: f64, iteration: f64, underlying: &AnimValue) -> AnimValue {
        let p = match self.timing.dur {
            Some(d) if d > 0.0 => simple / d,
            _ => 0.0,
        };
        if self.mode == ValueMode::To {
            let to = &self.values[0];
            return if self.calc == CalcMode::Discrete { if p < 0.5 { underlying.clone() } else { to.clone() } } else { svg_value::lerp(underlying, to, p) };
        }
        let mut v = interpolate(&self.values, self.calc, self.key_times.as_deref(), self.key_splines.as_deref(), p);
        if self.accumulate && iteration > 0.0 {
            v = svg_value::add(&v, &svg_value::scale(&self.values[self.values.len() - 1], iteration));
        }
        v
    }

    /// animateMotion's matrix at a phase.
    pub(crate) fn motion_matrix(&self, simple: f64, iteration: f64) -> Affine {
        let p = match self.timing.dur {
            Some(d) if d > 0.0 => simple / d,
            _ => 0.0,
        };
        let (x, y, dir) = if let Some(path) = &self.motion_path {
            let f = match interpolate(&self.values, if self.calc == CalcMode::Paced && self.values.len() == 2 { CalcMode::Linear } else { self.calc }, self.key_times.as_deref(), self.key_splines.as_deref(), p) {
                AnimValue::Number(f) => f,
                _ => 0.0,
            };
            let pt = path.geometry.get_point_at(f as f32);
            let mut x = pt.position.x as f64;
            let mut y = pt.position.y as f64;
            if self.accumulate && iteration > 0.0 {
                let end = path.geometry.get_point_at(1.0);
                x += end.position.x as f64 * iteration;
                y += end.position.y as f64 * iteration;
            }
            (x, y, (pt.tangent.y as f64).atan2(pt.tangent.x as f64))
        } else {
            let v = self.value(simple, iteration, &AnimValue::Point(0.0, 0.0));
            let (x, y) = if let AnimValue::Point(x, y) = v { (x, y) } else { (0.0, 0.0) };
            // Direction: of the segment being travelled (a small step ahead in progress).
            let ahead = self.value((simple + 1e-4 * self.timing.dur.unwrap_or(1.0)).min(self.timing.dur.unwrap_or(1.0)), iteration, &AnimValue::Point(0.0, 0.0));
            let behind = self.value((simple - 1e-4 * self.timing.dur.unwrap_or(1.0)).max(0.0), iteration, &AnimValue::Point(0.0, 0.0));
            let dir = match (ahead, behind) {
                (AnimValue::Point(ax, ay), AnimValue::Point(bx, by)) if (ax - bx).abs() + (ay - by).abs() > 1e-12 => (ay - by).atan2(ax - bx),
                _ => 0.0,
            };
            (x, y, dir)
        };
        let angle = match self.rotate {
            MotionRotate::Angle(a) => a.to_radians(),
            MotionRotate::Auto => dir,
            MotionRotate::AutoReverse => dir + std::f64::consts::PI,
        };
        Affine::translate((x, y)) * Affine::rotate(angle)
    }
}
