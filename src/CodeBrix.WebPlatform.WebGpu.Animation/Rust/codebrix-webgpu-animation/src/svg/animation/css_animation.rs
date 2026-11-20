//! Crate-internal: the CSS animation SUBSET inside an SVG's `<style>` - `@keyframes` and the
//! `animation-*` properties on elements selected by type, id and class (descendant and child
//! combinators) - our own small reader (simplecss skips at-rules) and evaluator.

use std::collections::HashMap;
use std::sync::Arc;

use kurbo::{Affine, Rect};
use usvg::roxmltree;

use crate::animation::{CubicBezierEasing, Easing, StepPosition};
use crate::svg::animation::svg_animations::store;
use crate::svg::animation::svg_value::{self, AnimAttr, AnimValue};
use crate::svg::drawing_svg_warning::{DrawingSvgWarningKind, WarningList};
use crate::svg::svg_part_override::SvgAnimationFrame;
use crate::svg::svg_scene::SvgScene;

/// A CSS timing function.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TimingFunction {
    Linear,
    Cubic(CubicBezierEasing),
    Steps(i32, StepPosition),
}

impl TimingFunction {
    pub(crate) fn parse(t: &str) -> Option<TimingFunction> {
        let t = t.trim();
        Some(match t {
            "linear" => TimingFunction::Linear,
            "ease" => TimingFunction::Cubic(CubicBezierEasing::EASE),
            "ease-in" => TimingFunction::Cubic(CubicBezierEasing::EASE_IN),
            "ease-out" => TimingFunction::Cubic(CubicBezierEasing::EASE_OUT),
            "ease-in-out" => TimingFunction::Cubic(CubicBezierEasing::EASE_IN_OUT),
            "step-start" => TimingFunction::Steps(1, StepPosition::Start),
            "step-end" => TimingFunction::Steps(1, StepPosition::End),
            _ => {
                let (name, args) = function(t)?;
                match name {
                    "cubic-bezier" => {
                        let n: Vec<f64> = args.iter().filter_map(|a| a.parse::<f64>().ok()).collect();
                        if n.len() != 4 {
                            return None;
                        }
                        TimingFunction::Cubic(CubicBezierEasing::new(n[0], n[1], n[2], n[3])?)
                    }
                    "steps" => {
                        let count = args.first()?.parse::<i32>().ok()?;
                        let pos = match args.get(1).map(|s| s.as_str()) {
                            None | Some("end") | Some("jump-end") => StepPosition::End,
                            Some("start") | Some("jump-start") => StepPosition::Start,
                            Some("jump-none") => StepPosition::None,
                            Some("jump-both") => StepPosition::Both,
                            _ => return None,
                        };
                        Easing::steps(0.0, count, pos)?;
                        TimingFunction::Steps(count, pos)
                    }
                    _ => return None,
                }
            }
        })
    }

    pub(crate) fn evaluate(&self, t: f64) -> f64 {
        match self {
            TimingFunction::Linear => t,
            TimingFunction::Cubic(c) => c.evaluate(t),
            TimingFunction::Steps(n, p) => Easing::steps(t, *n, *p).unwrap_or(t),
        }
    }
}

/// `name(a, b, ...)` -> (name, args).
fn function(t: &str) -> Option<(&str, Vec<String>)> {
    let open = t.find('(')?;
    let inner = t[open + 1..].strip_suffix(')')?;
    Some((t[..open].trim(), inner.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()).map(|s| s.to_string()).collect()))
}

/// Splits on `sep` outside parentheses.
fn split_top(t: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for c in t.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if c == sep && depth == 0 {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

fn split_ws(t: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for c in t.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if c.is_whitespace() && depth == 0 {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn css_time(t: &str) -> Option<f64> {
    let t = t.trim();
    if let Some(v) = t.strip_suffix("ms") {
        v.parse::<f64>().ok().map(|v| v / 1000.0)
    } else {
        t.strip_suffix('s')?.parse::<f64>().ok()
    }
}

/// One keyframe: offset 0..1, declarations, its own timing function.
#[derive(Clone, Debug)]
pub(crate) struct Keyframe {
    pub(crate) offset: f64,
    pub(crate) values: HashMap<String, String>,
    pub(crate) timing: Option<TimingFunction>,
}

/// A simple selector part.
#[derive(Clone, Debug, PartialEq)]
struct Compound {
    tag: Option<String>,
    ids: Vec<String>,
    classes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct Selector {
    /// Right-most last; the bool says "child of the one before" (else descendant).
    parts: Vec<(Compound, bool)>,
    specificity: (u32, u32, u32),
}

fn parse_selector(s: &str) -> Option<Selector> {
    let mut parts = Vec::new();
    let mut child = false;
    for tok in s.replace('>', " > ").split_whitespace() {
        if tok == ">" {
            child = true;
            continue;
        }
        let mut c = Compound { tag: None, ids: Vec::new(), classes: Vec::new() };
        let mut rest = tok;
        let first_special = rest.find(['#', '.']).unwrap_or(rest.len());
        let tag = &rest[..first_special];
        if tag.chars().any(|ch| !(ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '*')) {
            return None;
        }
        if !tag.is_empty() && tag != "*" {
            c.tag = Some(tag.to_string());
        }
        rest = &rest[first_special..];
        while !rest.is_empty() {
            let kind = rest.chars().next()?;
            let body = &rest[1..];
            let end = body.find(['#', '.']).unwrap_or(body.len());
            let name = &body[..end];
            if name.is_empty() || name.chars().any(|ch| !(ch.is_alphanumeric() || ch == '-' || ch == '_')) {
                return None;
            }
            if kind == '#' {
                c.ids.push(name.to_string());
            } else {
                c.classes.push(name.to_string());
            }
            rest = &body[end..];
        }
        parts.push((c, child));
        child = false;
    }
    if parts.is_empty() {
        return None;
    }
    let specificity = parts.iter().fold((0, 0, 0), |a, (c, _)| (a.0 + c.ids.len() as u32, a.1 + c.classes.len() as u32, a.2 + c.tag.is_some() as u32));
    Some(Selector { parts, specificity })
}

fn compound_matches(c: &Compound, e: &roxmltree::Node) -> bool {
    if let Some(t) = &c.tag {
        if e.tag_name().name() != t {
            return false;
        }
    }
    if !c.ids.iter().all(|i| e.attribute("id") == Some(i.as_str())) {
        return false;
    }
    let classes: Vec<&str> = e.attribute("class").unwrap_or("").split_whitespace().collect();
    c.classes.iter().all(|k| classes.contains(&k.as_str()))
}

fn selector_matches(s: &Selector, e: roxmltree::Node) -> bool {
    fn at(parts: &[(Compound, bool)], e: roxmltree::Node) -> bool {
        let Some(((c, child), rest)) = parts.split_last() else { return true };
        if !compound_matches(c, &e) {
            return false;
        }
        if rest.is_empty() {
            return true;
        }
        let mut p = e.parent_element();
        while let Some(x) = p {
            if at(rest, x) {
                return true;
            }
            if *child {
                return false;
            }
            p = x.parent_element();
        }
        false
    }
    at(&s.parts, e)
}

fn declarations(block: &str) -> Vec<(String, String)> {
    split_top(block, ';')
        .into_iter()
        .filter_map(|d| {
            let (n, v) = d.split_once(':')?;
            Some((n.trim().to_ascii_lowercase(), v.trim().trim_end_matches("!important").trim().to_string()))
        })
        .collect()
}

/// A parsed `<style>`: rules and keyframes.
#[derive(Default)]
pub(crate) struct StyleSheet {
    rules: Vec<(Selector, Vec<(String, String)>, usize)>,
    /// Static rules (no animation) that set an inherited property the crate animates.
    style_rules: Vec<(Selector, Vec<(String, String)>, usize)>,
    keyframes: HashMap<String, Arc<Vec<Keyframe>>>,
}

fn strip_comments(t: &str) -> String {
    let mut out = String::new();
    let mut rest = t;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        match rest[i + 2..].find("*/") {
            Some(j) => rest = &rest[i + 2 + j + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// The blocks at the top level: (prelude, body).
fn blocks(t: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let b = t.as_bytes();
    let mut i = 0;
    let mut start = 0;
    while i < b.len() {
        if b[i] == b'{' {
            let mut depth = 1;
            let mut j = i + 1;
            while j < b.len() && depth > 0 {
                match b[j] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            out.push((t[start..i].trim().to_string(), t[i + 1..j.saturating_sub(1).max(i + 1)].to_string()));
            i = j;
            start = j;
        } else if b[i] == b';' && t[start..i].trim_start().starts_with('@') {
            out.push((t[start..i].trim().to_string(), String::new()));
            i += 1;
            start = i;
        } else {
            i += 1;
        }
    }
    out
}

impl StyleSheet {
    pub(crate) fn parse(text: &str, warnings: &mut WarningList) -> StyleSheet {
        let mut sheet = StyleSheet::default();
        let text = strip_comments(text);
        for (order, (prelude, body)) in blocks(&text).into_iter().enumerate() {
            if let Some(name) = prelude.strip_prefix("@keyframes").or_else(|| prelude.strip_prefix("@-webkit-keyframes")) {
                let mut frames = Vec::new();
                for (sel, decls) in blocks(&body) {
                    let decls = declarations(&decls);
                    let timing = decls.iter().find(|(n, _)| n == "animation-timing-function").and_then(|(_, v)| TimingFunction::parse(v));
                    let values: HashMap<String, String> = decls.into_iter().filter(|(n, _)| n != "animation-timing-function").collect();
                    for s in sel.split(',') {
                        let s = s.trim();
                        let offset = match s {
                            "from" => Some(0.0),
                            "to" => Some(1.0),
                            _ => s.strip_suffix('%').and_then(|p| p.trim().parse::<f64>().ok()).map(|p| p / 100.0),
                        };
                        match offset {
                            Some(o) if (0.0..=1.0).contains(&o) => frames.push(Keyframe { offset: o, values: values.clone(), timing }),
                            _ => warnings.add(DrawingSvgWarningKind::CssIgnored, format!("The keyframe selector '{s}' in @keyframes {} is not from, to or a percentage: ignored.", name.trim())),
                        }
                    }
                }
                frames.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap_or(std::cmp::Ordering::Equal));
                sheet.keyframes.insert(name.trim().to_string(), Arc::new(frames));
            } else if prelude.starts_with('@') {
                warnings.add(DrawingSvgWarningKind::CssIgnored, format!("The CSS at-rule '{}' is not used by the animation reader.", prelude.split_whitespace().next().unwrap_or("@")));
            } else {
                let decls = declarations(&body);
                if !decls.iter().any(|(n, _)| n.starts_with("animation") || n == "transform-origin" || n == "transform-box") {
                    // A static rule that sets an inherited property the crate animates: kept to
                    // know which elements set it themselves (see specified_inherited).
                    if decls.iter().any(|(n, _)| INHERITED_PROPERTIES.contains(&n.as_str())) {
                        for s in prelude.split(',') {
                            if let Some(sel) = parse_selector(s.trim()) {
                                sheet.style_rules.push((sel, decls.clone(), order));
                            }
                        }
                    }
                    continue;
                }
                for s in prelude.split(',') {
                    match parse_selector(s.trim()) {
                        Some(sel) => sheet.rules.push((sel, decls.clone(), order)),
                        None => warnings.add(DrawingSvgWarningKind::CssIgnored, format!("The selector '{}' is not matched (only type, #id and .class selectors with descendant and '>' combinators are): its animations are not played.", s.trim())),
                    }
                }
            }
        }
        sheet
    }

    /// The cascaded animation-related declarations of an element (rules by specificity and
    /// order, then its style attribute).
    pub(crate) fn computed(&self, e: roxmltree::Node) -> HashMap<String, String> {
        let mut matched: Vec<&(Selector, Vec<(String, String)>, usize)> = self.rules.iter().filter(|(s, _, _)| selector_matches(s, e)).collect();
        matched.sort_by_key(|(s, _, o)| (s.specificity, *o));
        let mut out = HashMap::new();
        for (_, decls, _) in matched {
            for (n, v) in decls {
                out.insert(n.clone(), v.clone());
            }
        }
        if let Some(style) = e.attribute("style") {
            for (n, v) in declarations(style) {
                if n.starts_with("animation") || n == "transform-origin" || n == "transform-box" {
                    out.insert(n, v);
                }
            }
        }
        out
    }

    /// The INHERITED properties the crate animates that an element sets ITSELF - by a
    /// presentation attribute, its style attribute or a matching rule of the style sheet (a
    /// value of `inherit` does not count). An animated value on an ancestor does not reach such
    /// an element's subtree for that property (what browsers do).
    pub(crate) fn specified_inherited(&self, e: roxmltree::Node) -> Vec<AnimAttr> {
        let mut out = Vec::new();
        let style = e.attribute("style").map(declarations).unwrap_or_default();
        let matched: Vec<&(Selector, Vec<(String, String)>, usize)> = self.rules.iter().chain(self.style_rules.iter()).filter(|(s, _, _)| selector_matches(s, e)).collect();
        for name in INHERITED_PROPERTIES {
            let set = |v: &str| v.trim() != "inherit";
            let by_attribute = e.attributes().any(|a| a.namespace().is_none() && a.name() == name && set(a.value()));
            let by_style = style.iter().any(|(n, v)| n == name && set(v));
            let by_rule = matched.iter().any(|(_, decls, _)| decls.iter().any(|(n, v)| n == name && set(v)));
            if by_attribute || by_style || by_rule {
                if let Some(a) = AnimAttr::from_name(name) {
                    out.push(a);
                }
            }
        }
        out
    }

    pub(crate) fn keyframes(&self, name: &str) -> Option<Arc<Vec<Keyframe>>> {
        self.keyframes.get(name).cloned()
    }

    pub(crate) fn has_animations(&self) -> bool {
        !self.rules.is_empty() || !self.keyframes.is_empty()
    }
}

/// `animation-direction`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

/// One CSS animation on one element.
#[derive(Clone, Debug)]
pub(crate) struct CssAnimation {
    pub(crate) target: String,
    pub(crate) name: String,
    pub(crate) keyframes: Arc<Vec<Keyframe>>,
    pub(crate) duration: f64,
    pub(crate) delay: f64,
    pub(crate) iterations: f64,
    pub(crate) direction: Direction,
    pub(crate) fill_forwards: bool,
    pub(crate) fill_backwards: bool,
    pub(crate) timing: TimingFunction,
    pub(crate) paused: bool,
    pub(crate) origin: (String, bool),
}

/// The animations of one element from its computed declarations.
pub(crate) fn element_animations(target: &str, decls: &HashMap<String, String>, sheet: &StyleSheet, warnings: &mut WarningList) -> Vec<CssAnimation> {
    let mut list: Vec<HashMap<&'static str, String>> = Vec::new();
    if let Some(short) = decls.get("animation") {
        for one in split_top(short, ',') {
            let mut m: HashMap<&'static str, String> = HashMap::new();
            let mut times = 0;
            for tok in split_ws(&one) {
                if css_time(&tok).is_some() {
                    m.insert(if times == 0 { "animation-duration" } else { "animation-delay" }, tok.clone());
                    times += 1;
                } else if TimingFunction::parse(&tok).is_some() {
                    m.insert("animation-timing-function", tok);
                } else if tok == "infinite" || tok.parse::<f64>().is_ok() {
                    m.insert("animation-iteration-count", tok);
                } else if ["normal", "reverse", "alternate", "alternate-reverse"].contains(&tok.as_str()) {
                    m.insert("animation-direction", tok);
                } else if ["none", "forwards", "backwards", "both"].contains(&tok.as_str()) && m.contains_key("animation-name") {
                    m.insert("animation-fill-mode", tok);
                } else if ["forwards", "backwards", "both"].contains(&tok.as_str()) {
                    m.insert("animation-fill-mode", tok);
                } else if ["running", "paused"].contains(&tok.as_str()) {
                    m.insert("animation-play-state", tok);
                } else {
                    m.insert("animation-name", tok);
                }
            }
            list.push(m);
        }
    }
    // Longhands (later in the cascade we cannot tell; they refine the shorthand's entries).
    if let Some(names) = decls.get("animation-name") {
        let names = split_top(names, ',');
        while list.len() < names.len() {
            list.push(HashMap::new());
        }
        for (i, n) in names.into_iter().enumerate() {
            list[i].insert("animation-name", n);
        }
    }
    for key in ["animation-duration", "animation-delay", "animation-iteration-count", "animation-direction", "animation-fill-mode", "animation-timing-function", "animation-play-state"] {
        if let Some(v) = decls.get(key) {
            let vals = split_top(v, ',');
            if vals.is_empty() {
                continue;
            }
            for (i, m) in list.iter_mut().enumerate() {
                m.insert(key, vals[i % vals.len()].clone());
            }
        }
    }
    let origin = (decls.get("transform-origin").cloned().unwrap_or_else(|| "0 0".to_string()), decls.get("transform-box").map(|b| b == "fill-box").unwrap_or(false));
    let mut out = Vec::new();
    for m in list {
        let Some(name) = m.get("animation-name").cloned() else { continue };
        if name == "none" {
            continue;
        }
        let Some(keyframes) = sheet.keyframes(&name) else {
            warnings.add(DrawingSvgWarningKind::AnimationIgnored, format!("The CSS animation '{name}' on '{target}' names no @keyframes in the document: not played."));
            continue;
        };
        let duration = m.get("animation-duration").and_then(|d| css_time(d)).unwrap_or(0.0);
        if duration <= 0.0 {
            warnings.add(DrawingSvgWarningKind::AnimationIgnored, format!("The CSS animation '{name}' on '{target}' has no positive duration: not played."));
            continue;
        }
        let iterations = match m.get("animation-iteration-count").map(|s| s.as_str()) {
            Some("infinite") => f64::INFINITY,
            Some(n) => n.parse::<f64>().unwrap_or(1.0).max(0.0),
            None => 1.0,
        };
        let fill = m.get("animation-fill-mode").map(|s| s.as_str()).unwrap_or("none");
        out.push(CssAnimation {
            target: target.to_string(),
            name,
            keyframes,
            duration,
            delay: m.get("animation-delay").and_then(|d| css_time(d)).unwrap_or(0.0),
            iterations,
            direction: match m.get("animation-direction").map(|s| s.as_str()) {
                Some("reverse") => Direction::Reverse,
                Some("alternate") => Direction::Alternate,
                Some("alternate-reverse") => Direction::AlternateReverse,
                _ => Direction::Normal,
            },
            fill_forwards: fill == "forwards" || fill == "both",
            fill_backwards: fill == "backwards" || fill == "both",
            timing: m.get("animation-timing-function").and_then(|t| TimingFunction::parse(t)).unwrap_or(TimingFunction::Cubic(CubicBezierEasing::EASE)),
            paused: m.get("animation-play-state").map(|s| s == "paused").unwrap_or(false),
            origin: origin.clone(),
        });
    }
    out
}

/// The CSS properties the subset animates.
/// The inherited properties among those the crate animates.
pub(crate) const INHERITED_PROPERTIES: [&str; 8] = ["fill", "fill-opacity", "stroke", "stroke-opacity", "stroke-width", "stroke-dashoffset", "stroke-dasharray", "visibility"];

pub(crate) const CSS_PROPERTIES: [&str; 10] = ["transform", "opacity", "fill", "stroke", "fill-opacity", "stroke-opacity", "stroke-width", "stroke-dashoffset", "stroke-dasharray", "visibility"];

fn css_angle(t: &str) -> Option<f64> {
    let t = t.trim();
    if let Some(v) = t.strip_suffix("deg") {
        v.parse::<f64>().ok()
    } else if let Some(v) = t.strip_suffix("grad") {
        v.parse::<f64>().ok().map(|g| g * 0.9)
    } else if let Some(v) = t.strip_suffix("rad") {
        v.parse::<f64>().ok().map(|r| r.to_degrees())
    } else if let Some(v) = t.strip_suffix("turn") {
        v.parse::<f64>().ok().map(|r| r * 360.0)
    } else {
        t.parse::<f64>().ok().filter(|v| *v == 0.0)
    }
}

/// A CSS length in px against a reference size for percentages.
fn css_length(t: &str, reference: f64) -> Option<f64> {
    let t = t.trim();
    if let Some(p) = t.strip_suffix('%') {
        return p.parse::<f64>().ok().map(|p| p / 100.0 * reference);
    }
    t.strip_suffix("px").unwrap_or(t).parse::<f64>().ok()
}

/// A CSS transform list as (function name, numbers in px / degrees / factors).
pub(crate) fn parse_transform(t: &str, box_size: (f64, f64)) -> Option<Vec<(String, Vec<f64>)>> {
    if t.trim() == "none" {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    let mut rest = t.trim();
    while !rest.is_empty() {
        let close = rest.find(')')?;
        let (name, args) = function(&rest[..=close])?;
        let a = |i: usize| args.get(i).map(|s| s.as_str());
        let v: Vec<f64> = match name {
            "translate" => vec![css_length(a(0)?, box_size.0)?, a(1).map(|y| css_length(y, box_size.1)).unwrap_or(Some(0.0))?],
            "translateX" => vec![css_length(a(0)?, box_size.0)?, 0.0],
            "translateY" => vec![0.0, css_length(a(0)?, box_size.1)?],
            "scale" => {
                let x = a(0)?.parse::<f64>().ok()?;
                vec![x, a(1).map(|y| y.parse::<f64>().ok()).unwrap_or(Some(x))?]
            }
            "scaleX" => vec![a(0)?.parse::<f64>().ok()?, 1.0],
            "scaleY" => vec![1.0, a(0)?.parse::<f64>().ok()?],
            "rotate" => vec![css_angle(a(0)?)?],
            "skewX" => vec![css_angle(a(0)?)?, 0.0],
            "skewY" => vec![0.0, css_angle(a(0)?)?],
            "skew" => vec![css_angle(a(0)?)?, a(1).map(css_angle).unwrap_or(Some(0.0))?],
            "matrix" => {
                let n: Option<Vec<f64>> = args.iter().map(|s| s.parse::<f64>().ok()).collect();
                let n = n?;
                if n.len() != 6 {
                    return None;
                }
                n
            }
            _ => return None,
        };
        let canonical = match name {
            "translateX" | "translateY" => "translate",
            "scaleX" | "scaleY" => "scale",
            "skewX" | "skewY" => "skew",
            n => n,
        };
        out.push((canonical.to_string(), v));
        rest = rest[close + 1..].trim_start_matches([',', ' ']).trim();
    }
    Some(out)
}

fn identity_of(name: &str) -> Vec<f64> {
    match name {
        "scale" => vec![1.0, 1.0],
        "rotate" => vec![0.0],
        "matrix" => vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        _ => vec![0.0, 0.0],
    }
}

fn transform_to_matrix(list: &[(String, Vec<f64>)]) -> Affine {
    let mut m = Affine::IDENTITY;
    for (n, v) in list {
        let step = match n.as_str() {
            "translate" => Affine::translate((v[0], v[1])),
            "scale" => Affine::scale_non_uniform(v[0], v[1]),
            "rotate" => Affine::rotate(v[0].to_radians()),
            "skew" => Affine::skew(v[0].to_radians().tan(), v[1].to_radians().tan()),
            "matrix" => Affine::new([v[0], v[1], v[2], v[3], v[4], v[5]]),
            _ => Affine::IDENTITY,
        };
        m *= step;
    }
    m
}

/// A 2D matrix decomposed as CSS Transforms Level 1 does it: translate, rotate (degrees),
/// a residual 2x2 (skew) and scale, with M = translate x rotate x residual x scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Decomposed {
    pub(crate) translate: (f64, f64),
    pub(crate) scale: (f64, f64),
    pub(crate) angle: f64,
    pub(crate) residual: [f64; 4],
}

pub(crate) fn decompose(m: Affine) -> Decomposed {
    let [a, b, c, d, e, f] = m.as_coeffs();
    let mut sx = (a * a + b * b).sqrt();
    let mut sy = (c * c + d * d).sqrt();
    if a * d - b * c < 0.0 {
        if a < d {
            sx = -sx;
        } else {
            sy = -sy;
        }
    }
    let (na, nb) = if sx != 0.0 { (a / sx, b / sx) } else { (a, b) };
    let (nc, nd) = if sy != 0.0 { (c / sy, d / sy) } else { (c, d) };
    let angle = nb.atan2(na);
    let (s, co) = angle.sin_cos();
    // residual = rotate(-angle) x normalised
    let residual = [co * na + s * nb, -s * na + co * nb, co * nc + s * nd, -s * nc + co * nd];
    Decomposed { translate: (e, f), scale: (sx, sy), angle: angle.to_degrees(), residual }
}

pub(crate) fn recompose(d: &Decomposed) -> Affine {
    let r = d.residual;
    Affine::translate(d.translate) * Affine::rotate(d.angle.to_radians()) * Affine::new([r[0], r[1], r[2], r[3], 0.0, 0.0]) * Affine::scale_non_uniform(d.scale.0, d.scale.1)
}

/// CSS's interpolation of two decomposed matrices (flipped axes unflipped, never the long way
/// round).
pub(crate) fn lerp_decomposed(a: Decomposed, b: Decomposed, t: f64) -> Affine {
    let mut a = a;
    let mut b = b;
    if (a.scale.0 < 0.0 && b.scale.1 < 0.0) || (a.scale.1 < 0.0 && b.scale.0 < 0.0) {
        a.scale = (-a.scale.0, -a.scale.1);
        a.angle += if a.angle < 0.0 { 180.0 } else { -180.0 };
    }
    if a.angle == 0.0 {
        a.angle = 360.0;
    }
    if b.angle == 0.0 {
        b.angle = 360.0;
    }
    if (a.angle - b.angle).abs() > 180.0 {
        if a.angle > b.angle {
            a.angle -= 360.0;
        } else {
            b.angle -= 360.0;
        }
    }
    let l = |x: f64, y: f64| x + (y - x) * t;
    let mixed = Decomposed {
        translate: (l(a.translate.0, b.translate.0), l(a.translate.1, b.translate.1)),
        scale: (l(a.scale.0, b.scale.0), l(a.scale.1, b.scale.1)),
        angle: l(a.angle, b.angle),
        residual: [l(a.residual[0], b.residual[0]), l(a.residual[1], b.residual[1]), l(a.residual[2], b.residual[2]), l(a.residual[3], b.residual[3])],
    };
    recompose(&mixed)
}

/// Interpolates two transform lists as CSS does: matching function lists per argument (a missing
/// side as identity functions); different lists through the decomposed matrices.
pub(crate) fn lerp_transform(a: &[(String, Vec<f64>)], b: &[(String, Vec<f64>)], t: f64) -> Affine {
    let a_full: Vec<(String, Vec<f64>)> = if a.is_empty() { b.iter().map(|(n, _)| (n.clone(), identity_of(n))).collect() } else { a.to_vec() };
    let b_full: Vec<(String, Vec<f64>)> = if b.is_empty() { a.iter().map(|(n, _)| (n.clone(), identity_of(n))).collect() } else { b.to_vec() };
    if a_full.len() == b_full.len() && a_full.iter().zip(&b_full).all(|(x, y)| x.0 == y.0 && x.1.len() == y.1.len()) {
        let mixed: Vec<(String, Vec<f64>)> = a_full.iter().zip(&b_full).map(|(x, y)| (x.0.clone(), x.1.iter().zip(&y.1).map(|(p, q)| p + (q - p) * t).collect())).collect();
        transform_to_matrix(&mixed)
    } else {
        lerp_decomposed(decompose(transform_to_matrix(&a_full)), decompose(transform_to_matrix(&b_full)), t)
    }
}

impl CssAnimation {
    /// The iteration progress (0..1, direction applied) at `time`, or None when not in effect.
    pub(crate) fn progress(&self, time: f64) -> Option<f64> {
        let local = if self.paused { 0.0 } else { time } - self.delay;
        let total = self.iterations * self.duration;
        let (iteration, p) = if local < 0.0 {
            if !self.fill_backwards {
                return None;
            }
            (0.0, 0.0)
        } else if local >= total {
            if !self.fill_forwards {
                return None;
            }
            if self.iterations.is_infinite() {
                return None;
            }
            let whole = self.iterations.floor();
            let frac = self.iterations - whole;
            if frac > 0.0 {
                (whole, frac)
            } else {
                ((whole - 1.0).max(0.0), 1.0)
            }
        } else {
            let i = (local / self.duration).floor();
            (i, local / self.duration - i)
        };
        let odd = (iteration as i64) % 2 == 1;
        Some(match self.direction {
            Direction::Normal => p,
            Direction::Reverse => 1.0 - p,
            Direction::Alternate => {
                if odd {
                    1.0 - p
                } else {
                    p
                }
            }
            Direction::AlternateReverse => {
                if odd {
                    p
                } else {
                    1.0 - p
                }
            }
        })
    }

    fn origin_matrix(&self, scene: &SvgScene) -> (Affine, (f64, f64)) {
        let info = scene.part(&self.target);
        let bx: Rect = if self.origin.1 {
            info.and_then(|p| {
                let owner = *p.owners.first()?;
                let abs = scene.absolute_transform(owner);
                let b = p.document_bounds?;
                if abs.determinant().abs() < 1e-12 {
                    return None;
                }
                Some((abs.inverse() * p.transform_attribute).transform_rect_bbox(b))
            })
            .unwrap_or(Rect::ZERO)
        } else {
            scene.view_box()
        };
        let words = split_ws(&self.origin.0);
        let coord = |w: &str, horizontal: bool| -> f64 {
            let (start, size) = if horizontal { (bx.x0, bx.width()) } else { (bx.y0, bx.height()) };
            match w {
                "left" | "top" => start,
                "center" => start + size / 2.0,
                "right" | "bottom" => start + size,
                _ => {
                    let v = css_length(w, size).unwrap_or(0.0);
                    if w.ends_with('%') {
                        start + v
                    } else {
                        v
                    }
                }
            }
        };
        let mut x_word = words.first().map(|s| s.as_str()).unwrap_or("0");
        let mut y_word = words.get(1).map(|s| s.as_str()).unwrap_or("center");
        if words.len() == 1 && (x_word == "top" || x_word == "bottom") {
            y_word = x_word;
            x_word = "center";
        }
        if x_word == "top" || x_word == "bottom" || y_word == "left" || y_word == "right" {
            std::mem::swap(&mut x_word, &mut y_word);
        }
        let ox = coord(x_word, true);
        let oy = if words.len() == 1 && x_word != "center" && !["left", "right"].contains(&x_word) { coord("center", false) } else { coord(y_word, false) };
        (Affine::translate((ox, oy)), (bx.width(), bx.height()))
    }

    /// Writes this animation's values at `time` into the frame.
    pub(crate) fn apply(&self, time: f64, scene: &SvgScene, frame: &mut SvgAnimationFrame) {
        let Some(p) = self.progress(time) else { return };
        let base = scene.part(&self.target).map(|i| i.base.clone()).unwrap_or_default();
        for prop in CSS_PROPERTIES {
            let frames: Vec<&Keyframe> = self.keyframes.iter().filter(|k| k.values.contains_key(prop)).collect();
            if frames.is_empty() {
                continue;
            }
            // Implicit 0% / 100% keyframes take the element's static value (None).
            let mut stops: Vec<(f64, Option<&str>, Option<TimingFunction>)> = Vec::new();
            if frames[0].offset > 0.0 {
                stops.push((0.0, None, None));
            }
            for k in &frames {
                stops.push((k.offset, k.values.get(prop).map(|s| s.as_str()), k.timing));
            }
            if frames[frames.len() - 1].offset < 1.0 {
                stops.push((1.0, None, None));
            }
            let i = (0..stops.len() - 1).find(|i| p <= stops[i + 1].0).unwrap_or(stops.len().saturating_sub(2));
            let (a, b) = (&stops[i], &stops[(i + 1).min(stops.len() - 1)]);
            let span = b.0 - a.0;
            let local = if span > 0.0 { ((p - a.0) / span).clamp(0.0, 1.0) } else { 1.0 };
            let eased = a.2.unwrap_or(self.timing).evaluate(local);
            let o = frame.parts.entry(self.target.clone()).or_default();
            if prop == "transform" {
                let (origin, box_size) = self.origin_matrix(scene);
                // A missing (or unparsable) keyframe takes the element's static transform attribute,
                // expressed so that the origin conjugation below gives it back exactly.
                let attr = scene.part(&self.target).map(|i| i.transform_attribute).unwrap_or(Affine::IDENTITY);
                let static_list = || vec![("matrix".to_string(), (origin.inverse() * attr * origin).as_coeffs().to_vec())];
                let pa = a.1.and_then(|v| parse_transform(v, box_size)).unwrap_or_else(static_list);
                let pb = b.1.and_then(|v| parse_transform(v, box_size)).unwrap_or_else(static_list);
                let m = lerp_transform(&pa, &pb, eased);
                o.transform = Some(origin * m * origin.inverse());
                continue;
            }
            let Some(attr) = AnimAttr::from_name(prop) else { continue };
            let base_v = match attr {
                AnimAttr::Opacity => AnimValue::Number(base.opacity as f64),
                AnimAttr::FillOpacity => AnimValue::Number(base.fill_opacity as f64),
                AnimAttr::StrokeOpacity => AnimValue::Number(base.stroke_opacity as f64),
                AnimAttr::Fill => svg_value::from_color(base.fill),
                AnimAttr::Stroke => svg_value::from_color(base.stroke),
                AnimAttr::StrokeWidth => AnimValue::Number(base.stroke_width as f64),
                AnimAttr::StrokeDashoffset => AnimValue::Number(base.stroke_dash_offset as f64),
                AnimAttr::StrokeDasharray => AnimValue::List(base.stroke_dash_array.clone().unwrap_or_default().iter().map(|v| *v as f64).collect()),
                _ => AnimValue::Shown(true),
            };
            let va = a.1.and_then(|v| svg_value::parse(attr, None, v)).unwrap_or_else(|| base_v.clone());
            let vb = b.1.and_then(|v| svg_value::parse(attr, None, v)).unwrap_or(base_v);
            store(o, attr, &svg_value::lerp(&va, &vb, eased));
        }
    }
}
