//! `SvgAnimations`: a document's own animations (SMIL, then the CSS subset), evaluated for a
//! time passed in into an [`SvgAnimationFrame`] of per-part overrides. No clock is read.

use std::collections::HashMap;

use kurbo::Affine;

use crate::svg::animation::css_animation::CssAnimation;
use crate::svg::animation::smil_animation::{SmilAnimation, ValueMode};
use crate::svg::animation::smil_timing::{Interval, TimeSpec};
use crate::svg::animation::svg_value::{self, AnimAttr, AnimValue, TransformType};
use crate::svg::svg_part_override::{SvgAnimationFrame, SvgPartOverride};
use crate::svg::svg_scene::{SvgPartBase, SvgScene};

/// The document's own animations, ready to be evaluated at any time.
#[derive(Clone, Debug, Default)]
pub struct SvgAnimations {
    pub(crate) smil: Vec<(String, SmilAnimation)>,
    pub(crate) css: Vec<CssAnimation>,
}

fn base_value(attr: AnimAttr, base: &SvgPartBase) -> AnimValue {
    match attr {
        AnimAttr::Opacity => AnimValue::Number(base.opacity as f64),
        AnimAttr::FillOpacity => AnimValue::Number(base.fill_opacity as f64),
        AnimAttr::StrokeOpacity => AnimValue::Number(base.stroke_opacity as f64),
        AnimAttr::Fill => svg_value::from_color(base.fill),
        AnimAttr::Stroke => svg_value::from_color(base.stroke),
        AnimAttr::StrokeWidth => AnimValue::Number(base.stroke_width as f64),
        AnimAttr::StrokeDashoffset => AnimValue::Number(base.stroke_dash_offset as f64),
        AnimAttr::StrokeDasharray => AnimValue::List(base.stroke_dash_array.clone().unwrap_or_default().iter().map(|v| *v as f64).collect()),
        AnimAttr::Visibility | AnimAttr::Display => AnimValue::Shown(true),
        AnimAttr::Transform | AnimAttr::Motion => AnimValue::Number(0.0),
    }
}

/// Writes one attribute's final value into an override.
pub(crate) fn store(o: &mut SvgPartOverride, attr: AnimAttr, v: &AnimValue) {
    let num = |v: &AnimValue| if let AnimValue::Number(n) = v { Some(*n as f32) } else { None };
    match attr {
        AnimAttr::Opacity => o.opacity = num(v).map(|n| n.clamp(0.0, 1.0)),
        AnimAttr::FillOpacity => o.fill_opacity = num(v).map(|n| n.clamp(0.0, 1.0)),
        AnimAttr::StrokeOpacity => o.stroke_opacity = num(v).map(|n| n.clamp(0.0, 1.0)),
        AnimAttr::StrokeWidth => o.stroke_width = num(v).map(|n| n.max(0.0)),
        AnimAttr::StrokeDashoffset => o.stroke_dash_offset = num(v),
        AnimAttr::Fill | AnimAttr::Stroke => {
            if let AnimValue::Paint(p) = v {
                let c = Some(p.as_ref().map(svg_value::to_color));
                if attr == AnimAttr::Fill {
                    o.fill = c;
                } else {
                    o.stroke = c;
                }
            }
        }
        AnimAttr::StrokeDasharray => {
            if let AnimValue::List(l) = v {
                o.stroke_dash_array = Some(if l.is_empty() { None } else { Some(l.iter().map(|x| *x as f32).collect()) });
            }
        }
        AnimAttr::Display => {
            if let AnimValue::Shown(s) = v {
                o.visible = Some(match o.visible {
                    Some(false) => false,
                    _ => *s,
                });
            }
        }
        AnimAttr::Visibility => {
            if let AnimValue::Shown(s) = v {
                o.visibility = Some(*s);
            }
        }
        AnimAttr::Transform | AnimAttr::Motion => {}
    }
}

impl SvgAnimations {
    /// True when the document has no animation the crate plays.
    pub fn is_empty(&self) -> bool {
        self.smil.is_empty() && self.css.is_empty()
    }

    /// The number of SMIL animation elements played.
    pub fn smil_count(&self) -> usize {
        self.smil.len()
    }

    /// The names of the CSS animations played, with the part each one drives.
    pub fn css_animations(&self) -> Vec<(String, String)> {
        self.css.iter().map(|c| (c.target.clone(), c.name.clone())).collect()
    }

    /// The number of CSS animations played (one per element and animation name).
    pub fn css_count(&self) -> usize {
        self.css.len()
    }

    /// The intervals of every SMIL animation up to `horizon`, syncbases resolved by repeated
    /// passes (a chain that loops - a.begin = b.end, b.begin = a.end - grows one interval a
    /// pass; capped).
    pub(crate) fn intervals(&self, horizon: f64) -> Vec<Vec<Interval>> {
        let mut by_id: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, (_, a)) in self.smil.iter().enumerate() {
            if !a.own_id.is_empty() {
                by_id.entry(a.own_id.as_str()).or_default().push(i);
            }
        }
        let mut intervals: Vec<Vec<Interval>> = vec![Vec::new(); self.smil.len()];
        for _pass in 0..2000 {
            let mut changed = false;
            for i in 0..self.smil.len() {
                let a = &self.smil[i].1;
                let times = |specs: &[TimeSpec]| {
                    let mut out = Vec::new();
                    for s in specs {
                        match s {
                            TimeSpec::Offset(o) => out.push(*o),
                            TimeSpec::Sync { id, end, offset } => {
                                for j in by_id.get(id.as_str()).into_iter().flatten() {
                                    for iv in &intervals[*j] {
                                        let t = if *end { iv.end } else { iv.begin };
                                        if t.is_finite() {
                                            out.push(t + offset);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    out.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
                    out.dedup();
                    out
                };
                let begins = times(&a.timing.begin);
                let ends = times(&a.timing.end);
                let next = a.timing.intervals(&begins, &ends, horizon);
                if next != intervals[i] {
                    intervals[i] = next;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        intervals
    }

    /// Every part's override at `time` (seconds of document time). SMIL first (the sandwich:
    /// lower begin time first, then document order; replace / sum; to-animations from the value
    /// below), then CSS animations, which win over SMIL as the CSS cascade has them.
    pub fn evaluate(&self, time: f64, scene: &SvgScene) -> SvgAnimationFrame {
        let mut frame = SvgAnimationFrame { time, parts: HashMap::new() };
        if !self.smil.is_empty() {
            let intervals = self.intervals(time);
            let mut groups: HashMap<(String, AnimAttr), Vec<(f64, usize, f64, f64)>> = HashMap::new();
            for (i, (target, a)) in self.smil.iter().enumerate() {
                if let Some(ph) = a.timing.phase(&intervals[i], time) {
                    groups.entry((target.clone(), a.attr)).or_default().push((ph.begin, i, ph.simple, ph.iteration));
                }
            }
            for ((target, attr), mut list) in groups {
                list.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal).then(self.smil[x.1].1.order.cmp(&self.smil[y.1].1.order)));
                let info = scene.part(&target);
                let o = frame.parts.entry(target.clone()).or_default();
                match attr {
                    AnimAttr::Transform | AnimAttr::Motion => {
                        let mut m = if attr == AnimAttr::Transform { info.map(|p| p.transform_attribute).unwrap_or(Affine::IDENTITY) } else { Affine::IDENTITY };
                        for (_, i, simple, iteration) in list {
                            let a = &self.smil[i].1;
                            let step = if attr == AnimAttr::Motion {
                                a.motion_matrix(simple, iteration)
                            } else {
                                // A to-animation starts from the identity of its type (scale 1, else 0).
                                let ty = a.ty.unwrap_or(TransformType::Translate);
                                let identity = if ty == TransformType::Scale { [1.0, 1.0, 0.0] } else { [0.0, 0.0, 0.0] };
                                match a.value(simple, iteration, &AnimValue::Transform(ty, identity)) {
                                    AnimValue::Transform(ty, p) => svg_value::transform_matrix(ty, p),
                                    _ => Affine::IDENTITY,
                                }
                            };
                            m = if a.additive && a.mode != ValueMode::To { m * step } else { step };
                        }
                        if attr == AnimAttr::Transform {
                            o.transform = Some(m);
                        } else {
                            o.motion = Some(m);
                        }
                    }
                    _ => {
                        let base = info.map(|p| p.base.clone()).unwrap_or_default();
                        let mut v = base_value(attr, &base);
                        for (_, i, simple, iteration) in list {
                            let a = &self.smil[i].1;
                            let x = a.value(simple, iteration, &v);
                            v = if a.additive { svg_value::add(&v, &x) } else { x };
                        }
                        store(o, attr, &v);
                    }
                }
            }
        }
        for c in &self.css {
            c.apply(time, scene, &mut frame);
        }
        frame.parts.retain(|_, o| !o.is_empty());
        frame
    }
}
