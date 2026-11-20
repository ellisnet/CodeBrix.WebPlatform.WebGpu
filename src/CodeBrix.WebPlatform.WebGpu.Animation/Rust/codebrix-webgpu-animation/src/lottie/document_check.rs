//! The crate's pass over a document's JSON BEFORE velato reads it (crate-internal).
//!
//! It does three things, all on `serde_json::Value`:
//! 1. REPORTS what the document uses that velato does not draw or draws differently
//!    (every warning of [`DrawingLottieWarningKind`]).
//! 2. KEEPS velato FROM STOPPING on a document Lottie allows: velato's importer panics on a
//!    layer transform without a rotation, on 3D rotations, on a split position in a shape
//!    group's transform, on the "add" and "hard mix" blend modes and on per-dimension easing
//!    of a scalar, shape or gradient keyframe; its reader REFUSES a whole file for one text
//!    layer, one unknown layer type or one unknown shape item. Each is turned into what velato
//!    reads (or removed) and reported.
//! 3. FIXES what has a plain exact answer: hidden shape items (`hd`) removed (velato draws
//!    them), masks of mode "none" removed (velato clips by them), solid-colour layers turned
//!    into a rectangle (velato draws nothing for them), old-format keyframes without `s`
//!    completed from the previous keyframe's `e`, scalar keyframe values written as numbers
//!    wrapped in an array, repeaters limited.
//!
//! It also collects what velato does not keep: the markers, the image assets and the top-level
//! layers' blend modes (velato stores them and never applies them; the recording sink does).

use serde_json::{json, Map, Value};

use crate::lottie::drawing_lottie_options::DrawingLottieOptions;
use crate::lottie::drawing_lottie_warning::{DrawingLottieWarningKind as K, WarningList};
use crate::lottie::lottie_blend_mode::LottieBlendMode;
use crate::lottie::lottie_image_asset::LottieImageAsset;
use crate::lottie::lottie_marker::LottieMarker;

/// What the pass collected besides the warnings.
#[derive(Debug, Default)]
pub(crate) struct Checked {
    pub(crate) markers: Vec<LottieMarker>,
    pub(crate) images: Vec<LottieImageAsset>,
    /// One entry per top-level layer, in the order velato keeps them.
    pub(crate) layer_blends: Vec<Option<LottieBlendMode>>,
}

/// Checks and adjusts the document in place.
pub(crate) fn check(root: &mut Value, options: &DrawingLottieOptions, warnings: &mut WarningList) -> Result<Checked, String> {
    whole_numbers(root);
    let Some(map) = root.as_object_mut() else {
        return Err("The Lottie document's JSON is not an object: pass the animation JSON itself (an object with \"v\", \"fr\", \"ip\", \"op\", \"w\", \"h\" and \"layers\").".to_string());
    };
    let number = |map: &Map<String, Value>, key: &str, what: &str| -> Result<f64, String> {
        map.get(key).and_then(Value::as_f64).ok_or_else(|| format!("The Lottie document has no number \"{key}\" ({what}): it is not a Lottie animation, or it is damaged."))
    };
    let fr = number(map, "fr", "the frame rate")?;
    let ip = number(map, "ip", "the in point")?;
    let op = number(map, "op", "the out point")?;
    let w = number(map, "w", "the width")?;
    let h = number(map, "h", "the height")?;
    if fr.is_nan() || fr <= 0.0 || !fr.is_finite() {
        return Err(format!("The Lottie document's frame rate \"fr\" is {fr}: it must be greater than 0."));
    }
    if op.is_nan() || ip.is_nan() || op <= ip {
        return Err(format!("The Lottie document's out point \"op\" ({op}) is not after its in point \"ip\" ({ip}): there is nothing to play."));
    }
    if !(w >= 0.0 && h >= 0.0) || w.fract() != 0.0 || h.fract() != 0.0 {
        return Err(format!("The Lottie document's size \"w\" x \"h\" ({w} x {h}) must be whole numbers of 0 or more."));
    }
    if !map.get("layers").is_some_and(Value::is_array) {
        return Err("The Lottie document has no \"layers\" array: it is not a Lottie animation, or it is damaged.".to_string());
    }

    let mut checked = Checked::default();
    if let Some(markers) = map.get("markers").and_then(Value::as_array) {
        for m in markers {
            let name = match m.get("cm") {
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => String::new(),
            };
            let frame = m.get("tm").and_then(Value::as_f64).unwrap_or(0.0);
            let duration = m.get("dr").and_then(Value::as_f64).unwrap_or(0.0).max(0.0);
            checked.markers.push(LottieMarker { name, frame, duration });
        }
    }

    if let Some(assets) = map.get_mut("assets").and_then(Value::as_array_mut) {
        for asset in assets.iter_mut() {
            let Some(a) = asset.as_object_mut() else { continue };
            let id = a.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
            if let Some(layers) = a.get_mut("layers").and_then(Value::as_array_mut) {
                check_layers(layers, Some(&id), options, warnings);
            } else if let Some(p) = a.get("p").and_then(Value::as_str) {
                let dir = a.get("u").and_then(Value::as_str).unwrap_or_default();
                let is_data_url = p.starts_with("data:");
                let location = if is_data_url { p.to_string() } else { format!("{dir}{p}") };
                let size = match (a.get("w").and_then(Value::as_f64), a.get("h").and_then(Value::as_f64)) {
                    (Some(w), Some(h)) => Some((w, h)),
                    _ => None,
                };
                checked.images.push(LottieImageAsset { id, size, location, is_data_url });
            }
        }
    }

    let layers = map.get_mut("layers").and_then(Value::as_array_mut).expect("checked above");
    checked.layer_blends = check_layers(layers, None, options, warnings);
    Ok(checked)
}

/// Writes every whole number stored as a float (`4.0`) as an integer (`4`): velato reads layer
/// types, indices, sizes and flags as integers and refuses a float there; every other number
/// reads the same either way.
fn whole_numbers(v: &mut Value) {
    match v {
        Value::Number(n) => {
            if n.is_f64() {
                let f = n.as_f64().unwrap_or(0.0);
                if f.fract() == 0.0 && f.abs() < 9.0e15 {
                    *v = if f >= 0.0 { json!(f as u64) } else { json!(f as i64) };
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(whole_numbers),
        Value::Object(map) => map.values_mut().for_each(whole_numbers),
        _ => {}
    }
}

fn layer_label(layer: &Map<String, Value>, precomp: Option<&str>) -> String {
    let name = layer.get("nm").and_then(Value::as_str).unwrap_or("(unnamed)");
    match precomp {
        Some(id) => format!("layer '{name}' of precomposition '{id}'"),
        None => format!("layer '{name}'"),
    }
}

fn layer_type_name(ty: u64) -> &'static str {
    match ty {
        6 => "audio",
        7 => "video placeholder",
        8 => "image sequence",
        9 => "video",
        10 => "image placeholder",
        11 => "guide",
        12 => "adjustment",
        13 => "camera",
        14 => "light",
        15 => "data",
        _ => "unknown",
    }
}

/// Checks a list of layers; returns each kept layer's blend mode (used for top-level layers).
fn check_layers(layers: &mut Vec<Value>, precomp: Option<&str>, options: &DrawingLottieOptions, warnings: &mut WarningList) -> Vec<Option<LottieBlendMode>> {
    let mut kept = Vec::with_capacity(layers.len());
    let mut blends = Vec::with_capacity(layers.len());
    for mut layer in layers.drain(..) {
        let Some(obj) = layer.as_object_mut() else {
            kept.push(layer);
            blends.push(None);
            continue;
        };
        let label = layer_label(obj, precomp);
        match obj.get("ty").and_then(Value::as_u64) {
            Some(5) => {
                warnings.add(K::TextLayerNotDrawn, format!("{label} is a text layer: Lottie text is not drawn (it is kept as a null layer so that its children still move)."));
                to_null(obj);
            }
            Some(1) => {
                if options.draw_solid_layers {
                    if !solid_to_shape(obj) {
                        warnings.add(K::LayerTypeNotDrawn, format!("{label} is a solid-colour layer whose size or colour (\"sw\", \"sh\", \"sc\") cannot be read: it is not drawn."));
                    }
                } else {
                    warnings.add(K::LayerTypeNotDrawn, format!("{label} is a solid-colour layer: not drawn (the load option draw_solid_layers is off)."));
                }
            }
            Some(2) => {
                let id = obj.get("refId").and_then(Value::as_str).unwrap_or_default();
                warnings.add(K::ImageNeedsPixels, format!("{label} draws the image asset '{id}': the application supplies its decoded pixels, or it is not drawn."));
            }
            Some(0) | Some(3) | Some(4) => {}
            Some(ty) => {
                let what = layer_type_name(ty);
                if obj.contains_key("ks") {
                    warnings.add(K::LayerTypeNotDrawn, format!("{label} is a layer of type {ty} ({what}): not drawn (kept as a null layer so that its children still move)."));
                    to_null(obj);
                } else {
                    warnings.add(K::LayerTypeNotDrawn, format!("{label} is a layer of type {ty} ({what}) without a transform: removed."));
                    continue;
                }
            }
            None => {}
        }
        let ty = obj.get("ty").and_then(Value::as_u64);
        if ty == Some(0) {
            if obj.get("tm").is_some_and(|v| !v.is_null()) {
                warnings.add(K::TimeRemapIgnored, format!("{label} remaps its precomposition's time (\"tm\"): the precomposition plays at its own time."));
            }
            if !obj.get("st").is_some_and(Value::is_number) {
                obj.insert("st".to_string(), json!(0));
            }
        }
        check_layer_transform(obj, &label, warnings);
        let blend = check_layer_blend(obj, &label, precomp.is_some(), warnings);
        if matches!(obj.get("tt").and_then(Value::as_u64), Some(3) | Some(4)) {
            warnings.add(K::LumaMatteApproximated, format!("{label} uses a luma track matte: applied by the matte's alpha."));
        }
        check_masks(obj, &label, warnings);
        if obj.get("ef").and_then(Value::as_array).is_some_and(|e| !e.is_empty()) {
            let names: Vec<String> = obj["ef"].as_array().unwrap().iter().map(|e| e.get("nm").and_then(Value::as_str).unwrap_or("(unnamed)").to_string()).collect();
            warnings.add(K::EffectIgnored, format!("{label} has effects ({}): not applied.", names.join(", ")));
        }
        if obj.get("sy").and_then(Value::as_array).is_some_and(|e| !e.is_empty()) {
            warnings.add(K::LayerStyleIgnored, format!("{label} has layer styles (\"sy\"): not applied."));
        }
        if obj.get("mb").and_then(Value::as_bool) == Some(true) {
            warnings.add(K::MotionBlurIgnored, format!("{label} asks for motion blur: not applied."));
        }
        if obj.get("ao").and_then(Value::as_u64) == Some(1) {
            warnings.add(K::AutoOrientIgnored, format!("{label} auto-orients along its motion path: not applied."));
        }
        if ty == Some(4) {
            if let Some(shapes) = obj.get_mut("shapes").and_then(Value::as_array_mut) {
                check_shapes(shapes, &label, options, warnings);
            }
        }
        fix_properties(&mut layer, &label, warnings);
        kept.push(layer);
        blends.push(blend);
    }
    *layers = kept;
    blends
}

/// Keeps what a null layer needs (its transform, place in the hierarchy and time span).
fn to_null(obj: &mut Map<String, Value>) {
    const KEEP: [&str; 11] = ["ks", "ind", "parent", "ip", "op", "st", "sr", "nm", "hd", "td", "ddd"];
    obj.retain(|k, _| KEEP.contains(&k.as_str()));
    obj.insert("ty".to_string(), json!(3));
}

/// Turns a solid-colour layer into a shape layer holding a rectangle of its size and colour.
fn solid_to_shape(obj: &mut Map<String, Value>) -> bool {
    let (Some(w), Some(h), Some(c)) = (obj.get("sw").and_then(Value::as_f64), obj.get("sh").and_then(Value::as_f64), obj.get("sc").and_then(Value::as_str)) else {
        return false;
    };
    let hex = c.trim_start_matches('#');
    let channel = |i: usize| -> Option<f64> {
        if hex.len() == 6 {
            u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok().map(|v| v as f64 / 255.0)
        } else if hex.len() == 3 {
            u8::from_str_radix(hex.get(i..i + 1)?, 16).ok().map(|v| (v * 17) as f64 / 255.0)
        } else {
            None
        }
    };
    let (Some(r), Some(g), Some(b)) = (channel(0), channel(1), channel(2)) else {
        return false;
    };
    obj.remove("sw");
    obj.remove("sh");
    obj.remove("sc");
    obj.insert("ty".to_string(), json!(4));
    obj.insert(
        "shapes".to_string(),
        json!([
            { "ty": "rc", "nm": "solid", "p": { "a": 0, "k": [w / 2.0, h / 2.0] }, "s": { "a": 0, "k": [w, h] }, "r": { "a": 0, "k": 0 } },
            { "ty": "fl", "nm": "solid colour", "c": { "a": 0, "k": [r, g, b, 1] }, "o": { "a": 0, "k": 100 }, "r": 1 }
        ]),
    );
    true
}

/// A layer transform as velato reads it: a position, and a rotation (3D rotation: Z only).
fn check_layer_transform(obj: &mut Map<String, Value>, label: &str, warnings: &mut WarningList) {
    if obj.get("ddd").and_then(Value::as_u64) == Some(1) {
        warnings.add(K::ThreeDimensionalIgnored, format!("{label} is a 3D layer: drawn flat."));
    }
    let ks = obj.entry("ks").or_insert_with(|| json!({}));
    let Some(ks) = ks.as_object_mut() else { return };
    if !ks.contains_key("p") {
        ks.insert("p".to_string(), json!({ "a": 0, "k": [0, 0] }));
    }
    let three_d = ks.contains_key("rx") || ks.contains_key("ry") || ks.contains_key("or");
    if three_d {
        warnings.add(K::ThreeDimensionalIgnored, format!("{label} rotates in 3D: only its Z rotation is applied."));
    }
    let rotation_ok = ks.get("r").is_some_and(|r| r.get("k").is_some());
    if !rotation_ok {
        let z = ks.get("rz").cloned().or_else(|| ks.get("r").and_then(|r| r.get("z")).cloned());
        ks.insert("r".to_string(), z.unwrap_or_else(|| json!({ "a": 0, "k": 0 })));
    }
}

/// Reads a layer's blend mode and sets it to normal for velato (whose importer panics on
/// "add" and "hard mix" and whose renderer never applies the others).
fn check_layer_blend(obj: &mut Map<String, Value>, label: &str, nested: bool, warnings: &mut WarningList) -> Option<LottieBlendMode> {
    let bm = obj.get("bm").and_then(Value::as_u64).unwrap_or(0);
    if obj.contains_key("bm") {
        obj.insert("bm".to_string(), json!(0));
    }
    if bm == 0 {
        return None;
    }
    match LottieBlendMode::from_lottie(bm) {
        Some(mode) if !nested => Some(mode),
        Some(mode) => {
            warnings.add(K::BlendModeIgnored, format!("{label} uses the blend mode {mode:?}: blend modes are applied to top-level layers only; drawn as normal."));
            None
        }
        None => {
            warnings.add(K::BlendModeIgnored, format!("{label} uses the blend mode {bm} (hard mix or unknown): drawn as normal."));
            None
        }
    }
}

fn is_static(v: Option<&Value>, value: f64) -> bool {
    let Some(v) = v else { return true };
    match v.get("k") {
        Some(Value::Number(n)) => n.as_f64() == Some(value),
        Some(Value::Array(a)) if a.len() == 1 => a[0].as_f64() == Some(value),
        _ => false,
    }
}

/// Masks: "none" removed (exact); everything velato does not apply as written reported.
fn check_masks(obj: &mut Map<String, Value>, label: &str, warnings: &mut WarningList) {
    let Some(masks) = obj.get_mut("masksProperties").and_then(Value::as_array_mut) else { return };
    masks.retain(|m| m.get("mode").and_then(Value::as_str) != Some("n"));
    let mut reasons: Vec<String> = Vec::new();
    for (i, m) in masks.iter().enumerate() {
        let mode = m.get("mode").and_then(Value::as_str).unwrap_or("a");
        let exact_mode = if i == 0 { mode == "a" || mode == "i" } else { mode == "i" };
        if !exact_mode {
            let what = match mode {
                "a" => "add (a union with the masks before it)",
                "s" => "subtract",
                "i" => "intersect",
                "l" => "lighten",
                "d" => "darken",
                "f" => "difference",
                _ => "unknown",
            };
            reasons.push(format!("mode {what}"));
        }
        if m.get("inv").and_then(Value::as_bool) == Some(true) {
            reasons.push("inverted".to_string());
        }
        if !is_static(m.get("o"), 100.0) {
            reasons.push("opacity".to_string());
        }
        if !is_static(m.get("x"), 0.0) {
            reasons.push("expansion".to_string());
        }
    }
    if !reasons.is_empty() {
        reasons.dedup();
        warnings.add(K::MaskApproximated, format!("{label} has masks drawn as plain clips to the inside of their paths (their intersection); not applied: {}.", reasons.join(", ")));
    }
}

/// Shape items: unknown and unapplied items removed, hidden items removed, the rest checked.
fn check_shapes(shapes: &mut Vec<Value>, label: &str, options: &DrawingLottieOptions, warnings: &mut WarningList) {
    shapes.retain_mut(|item| {
        let Some(obj) = item.as_object_mut() else { return true };
        if obj.get("hd").and_then(Value::as_bool) == Some(true) {
            return false;
        }
        let ty = obj.get("ty").and_then(Value::as_str).unwrap_or_default().to_string();
        match ty.as_str() {
            "gr" => {
                if let Some(it) = obj.get_mut("it").and_then(Value::as_array_mut) {
                    check_shapes(it, label, options, warnings);
                }
                true
            }
            "rc" | "el" | "sr" | "sh" | "fl" => {
                if ty == "fl" && obj.get("r").and_then(Value::as_u64) == Some(2) {
                    warnings.add(K::EvenOddFillApproximated, format!("{label} fills with the even-odd rule: filled non-zero."));
                }
                true
            }
            "st" | "gs" | "gf" => {
                if ty != "gf" && obj.get("d").and_then(Value::as_array).is_some_and(|d| !d.is_empty()) {
                    warnings.add(K::StrokeDashesIgnored, format!("{label} has a dashed stroke: drawn solid."));
                }
                if ty == "gf" && obj.get("r").and_then(Value::as_u64) == Some(2) {
                    warnings.add(K::EvenOddFillApproximated, format!("{label} fills with the even-odd rule: filled non-zero."));
                }
                if ty != "st" {
                    if !is_static(obj.get("o"), 100.0) {
                        warnings.add(K::GradientOpacityIgnored, format!("{label} has a gradient with its own opacity: drawn at full opacity."));
                    }
                    if obj.get("t").and_then(Value::as_u64) == Some(2) && !is_static(obj.get("h"), 0.0) {
                        warnings.add(K::GradientHighlightIgnored, format!("{label} has a radial gradient with a highlight: drawn centred."));
                    }
                }
                true
            }
            "tm" => {
                if obj.get("m").and_then(Value::as_u64) == Some(2) {
                    warnings.add(K::TrimIndividuallyApproximated, format!("{label} trims its paths individually: each path is trimmed on its own."));
                }
                true
            }
            "tr" => {
                fix_shape_transform(obj, label, warnings);
                true
            }
            "rp" => {
                limit_repeater(obj, label, options.max_repeater_copies, warnings);
                true
            }
            "pb" | "mm" | "op" | "tw" | "rd" | "zz" => {
                let what = match ty.as_str() {
                    "pb" => "pucker/bloat",
                    "mm" => "merge paths",
                    "op" => "offset path",
                    "tw" => "twist",
                    "rd" => "rounded corners",
                    _ => "zig zag",
                };
                warnings.add(K::ShapeModifierIgnored, format!("{label} uses the shape modifier {what}: not applied; the shapes are drawn unmodified."));
                false
            }
            "no" => false,
            other => {
                warnings.add(K::UnknownShapeRemoved, format!("{label} has a shape item of the unknown type '{other}': removed."));
                false
            }
        }
    });
}

/// A shape group's transform with a split position or a split rotation: velato's importer
/// panics on both.
fn fix_shape_transform(obj: &mut Map<String, Value>, label: &str, warnings: &mut WarningList) {
    if obj.get("r").is_some_and(|r| r.get("k").is_none()) {
        let z = obj.get("r").and_then(|r| r.get("z")).cloned();
        obj.insert("r".to_string(), z.unwrap_or_else(|| json!({ "a": 0, "k": 0 })));
    }
    let Some(p) = obj.get("p") else {
        obj.insert("p".to_string(), json!({ "a": 0, "k": [0, 0] }));
        return;
    };
    let split = p.get("s").and_then(Value::as_bool) == Some(true) || (p.get("k").is_none() && p.get("x").is_some());
    if !split {
        return;
    }
    let component = |v: Option<&Value>| -> (f64, bool) {
        match v.and_then(|c| c.get("k")) {
            Some(Value::Number(n)) => (n.as_f64().unwrap_or(0.0), false),
            Some(Value::Array(a)) => match a.first() {
                Some(Value::Number(n)) => (n.as_f64().unwrap_or(0.0), false),
                Some(Value::Object(kf)) => (kf.get("s").and_then(|s| s.get(0).or(Some(s))).and_then(Value::as_f64).unwrap_or(0.0), true),
                _ => (0.0, false),
            },
            _ => (0.0, false),
        }
    };
    let (x, xa) = component(p.get("x"));
    let (y, ya) = component(p.get("y"));
    if xa || ya {
        warnings.add(K::SplitPositionApproximated, format!("{label} animates a shape group's position by separate X and Y: the first keyframe's position is used."));
    }
    obj.insert("p".to_string(), json!({ "a": 0, "k": [x, y] }));
}

fn limit_repeater(obj: &mut Map<String, Value>, label: &str, max: u32, warnings: &mut WarningList) {
    let max = max as f64;
    let mut limited = false;
    if let Some(k) = obj.get_mut("c").and_then(|c| c.get_mut("k")) {
        match k {
            Value::Number(n) if n.as_f64().unwrap_or(0.0) > max => {
                *k = json!(max);
                limited = true;
            }
            Value::Array(items) => {
                for item in items.iter_mut() {
                    match item {
                        Value::Number(n) if n.as_f64().unwrap_or(0.0) > max => {
                            *item = json!(max);
                            limited = true;
                        }
                        Value::Object(kf) => {
                            for key in ["s", "e"] {
                                if let Some(Value::Array(s)) = kf.get_mut(key) {
                                    if s.first().and_then(Value::as_f64).unwrap_or(0.0) > max {
                                        s[0] = json!(max);
                                        limited = true;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    if limited {
        warnings.add(K::RepeaterCopiesLimited, format!("{label} has a repeater with more than {max} copies: limited to {max}."));
    }
}

/// Walks every animated property: expressions reported, keyframes made readable for velato
/// (per-dimension easing handles reduced to the first one for EVERY property: velato's importer
/// panics on them for scalars, shapes and gradient colours, and turns them into duplicate
/// keyframes elsewhere).
fn fix_properties(v: &mut Value, label: &str, warnings: &mut WarningList) {
    match v {
        Value::Object(map) => {
            if map.get("x").is_some_and(Value::is_string) && map.contains_key("k") {
                warnings.add(K::ExpressionIgnored, format!("{label} has expressions: their keyframes or values are used instead."));
            }
            let keyframes = matches!(map.get("k"), Some(Value::Array(a)) if a.first().is_some_and(|f| f.get("t").is_some()));
            for (key, child) in map.iter_mut() {
                if keyframes && key == "k" {
                    if let Value::Array(frames) = child {
                        fix_keyframes(frames, label, warnings);
                    }
                    continue;
                }
                fix_properties(child, label, warnings);
            }
        }
        Value::Array(items) => {
            for item in items {
                fix_properties(item, label, warnings);
            }
        }
        _ => {}
    }
}

fn fix_keyframes(frames: &mut Vec<Value>, label: &str, warnings: &mut WarningList) {
    let mut previous: Option<Value> = None;
    for frame in frames.iter_mut() {
        let Some(kf) = frame.as_object_mut() else { continue };
        if let Some(Value::Number(n)) = kf.get("s") {
            let n = n.clone();
            kf.insert("s".to_string(), Value::Array(vec![Value::Number(n)]));
        }
        if let Some(Value::Number(n)) = kf.get("e") {
            let n = n.clone();
            kf.insert("e".to_string(), Value::Array(vec![Value::Number(n)]));
        }
        if kf.get("s").is_none_or(|s| s.as_array().is_some_and(Vec::is_empty)) {
            match previous.as_ref().and_then(|p| [p.get("e"), p.get("s")].into_iter().flatten().find(|v| v.as_array().is_some_and(|a| !a.is_empty()))) {
                Some(prev) => {
                    kf.insert("s".to_string(), prev.clone());
                }
                // A first keyframe without a value: velato's importer would index it and panic.
                None if kf.contains_key("s") => {
                    *frame = Value::Null;
                    continue;
                }
                None => {}
            }
        }
        for handle in ["i", "o"] {
            let Some(h) = kf.get_mut(handle).and_then(Value::as_object_mut) else { continue };
            for axis in ["x", "y"] {
                if let Some(Value::Array(a)) = h.get_mut(axis) {
                    if a.len() > 1 {
                        if a.iter().any(|c| c != &a[0]) {
                            warnings.add(K::EasingApproximated, format!("{label} eases the dimensions of a property with different curves: the first curve is used for all."));
                        }
                        a.truncate(1);
                    } else if a.is_empty() {
                        a.push(json!(if handle == "o" { 0.0 } else { 1.0 }));
                    }
                }
            }
        }
        let tangent = |key: &str| kf.get(key).and_then(Value::as_array).is_some_and(|t| t.iter().any(|c| c.as_f64().is_some_and(|c| c != 0.0)));
        if tangent("ti") || tangent("to") {
            warnings.add(K::MotionPathApproximated, format!("{label} moves along a curved motion path: it moves in straight lines between its keyframes."));
        }
        previous = Some(json!({ "e": kf.get("e").cloned().unwrap_or(Value::Null), "s": kf.get("s").cloned().unwrap_or(Value::Null) }));
    }
    frames.retain(|f| !f.is_null());
}
