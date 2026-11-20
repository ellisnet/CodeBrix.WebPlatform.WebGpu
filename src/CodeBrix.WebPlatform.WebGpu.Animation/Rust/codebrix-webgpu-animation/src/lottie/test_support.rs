//! Helpers shared by the Lottie host tests.

use kurbo::{Affine, Point, Rect, Shape};

use crate::lottie::*;

/// Reads one of the test files.
pub(crate) fn load(json: &str) -> DrawingLottie {
    DrawingLottie::from_json(json, &DrawingLottieOptions::default()).expect("the test file loads")
}

/// One of the three large invented trial files (A_many_shapes.json, B_painterly.json,
/// C_busy_scene.json). They are NOT in the crate (it travels inside the package): they are the
/// GraphicsDemo sample's assets, and the environment variable CODEBRIX_ANIMATION_TEST_LOTTIE names
/// the folder that holds them (the .NET test that runs these tests sets it to a copy beside its
/// assembly). Without the variable the test that wants them prints a SKIPPED line and passes.
pub(crate) fn trial_file(name: &str) -> Option<String> {
    let root = match std::env::var("CODEBRIX_ANIMATION_TEST_LOTTIE") {
        Ok(root) => root,
        Err(_) => {
            eprintln!("SKIPPED: no Lottie trial file {name} (set CODEBRIX_ANIMATION_TEST_LOTTIE)");
            return None;
        }
    };
    let path = std::path::Path::new(&root).join(name);
    match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(e) => panic!("CODEBRIX_ANIMATION_TEST_LOTTIE is set, but {} cannot be read: {e}", path.display()),
    }
}

/// True when two numbers agree to `eps`.
pub(crate) fn near(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

/// True when a point is where it should be (to 1e-6).
pub(crate) fn at(p: Point, x: f64, y: f64) -> bool {
    near(p.x, x, 1e-6) && near(p.y, y, 1e-6)
}

/// True when the document reported a warning of that kind.
pub(crate) fn has(lottie: &DrawingLottie, kind: DrawingLottieWarningKind) -> bool {
    lottie.warnings().iter().any(|w| w.kind == kind)
}

/// The draws of a list (fills, strokes, images) in drawing order.
pub(crate) fn draws(list: &LottieDrawList) -> Vec<&LottieDrawCommand> {
    list.commands.iter().filter(|c| c.is_draw()).collect()
}

/// A draw's transform.
pub(crate) fn transform_of(c: &LottieDrawCommand) -> Affine {
    match c {
        LottieDrawCommand::Fill { transform, .. } | LottieDrawCommand::Stroke { transform, .. } | LottieDrawCommand::Image { transform, .. } => *transform,
        _ => Affine::IDENTITY,
    }
}

/// A fill's or stroke's path bounds in output coordinates.
pub(crate) fn device_bounds(c: &LottieDrawCommand) -> Rect {
    match c {
        LottieDrawCommand::Fill { path, transform, .. } | LottieDrawCommand::Stroke { path, transform, .. } => transform.transform_rect_bbox(path.bounding_box()),
        _ => Rect::ZERO,
    }
}

/// A draw's own opacity (the paint's alpha factor).
pub(crate) fn opacity_of(c: &LottieDrawCommand) -> f32 {
    match c {
        LottieDrawCommand::Fill { opacity, .. } | LottieDrawCommand::Image { opacity, .. } => *opacity,
        LottieDrawCommand::Stroke { stroke, .. } => stroke.opacity,
        _ => 1.0,
    }
}

/// For each draw, its opacity times the alphas of the NORMAL groups around it, with its
/// transform - what reaches the screen when nothing overlaps inside a group.
pub(crate) fn effective_draws(list: &LottieDrawList) -> Vec<(f32, Affine)> {
    let mut alphas: Vec<f32> = Vec::new();
    let mut out = Vec::new();
    for c in &list.commands {
        match c {
            LottieDrawCommand::PushGroup { alpha, .. } => alphas.push(*alpha),
            LottieDrawCommand::PushClip { .. } => alphas.push(1.0),
            LottieDrawCommand::Pop => {
                alphas.pop();
            }
            _ => out.push((alphas.iter().product::<f32>() * opacity_of(c), transform_of(c))),
        }
    }
    out
}
