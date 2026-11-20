//! Folding of one-draw groups (crate-internal), a data-level simplification of velato's output.
//!
//! velato wraps every faded repeater copy in a group of its own (and a matte, a top-level blend
//! mode). A NORMAL group holding exactly ONE draw composites the same as that draw with the
//! group's alpha multiplied into it - a single non-zero fill or stroke never covers a pixel twice -
//! so it is replaced by the draw, with the group's area kept as a clip only when the draw could
//! reach outside it. Empty groups and clips are removed (but not an empty matte group: SourceIn
//! and SourceOut clear what is below them); a normal group of alpha 0 holding one draw is
//! removed with it. Groups of any other blend mode are kept as they are.

use std::sync::Arc;

use kurbo::{Affine, BezPath, Rect, Shape};

use crate::lottie::lottie_blend_mode::LottieBlendMode;
use crate::lottie::lottie_draw_command::LottieDrawCommand;

/// The commands folded, and how many groups were folded.
pub(crate) fn fold(commands: Vec<LottieDrawCommand>) -> (Vec<LottieDrawCommand>, usize) {
    let mut out: Vec<LottieDrawCommand> = Vec::with_capacity(commands.len());
    let mut open: Vec<usize> = Vec::new();
    let mut folded = 0usize;
    for command in commands {
        match command {
            LottieDrawCommand::PushGroup { .. } | LottieDrawCommand::PushClip { .. } => {
                open.push(out.len());
                out.push(command);
            }
            LottieDrawCommand::Pop => {
                let Some(start) = open.pop() else {
                    out.push(LottieDrawCommand::Pop);
                    continue;
                };
                let content = out.len() - start - 1;
                // An empty group or clip draws nothing - except a matte's SourceIn / SourceOut
                // group, which clears what is below it inside its area.
                let clears = matches!(&out[start], LottieDrawCommand::PushGroup { blend: LottieBlendMode::SourceIn | LottieBlendMode::SourceOut, .. });
                if content == 0 && !clears {
                    out.truncate(start);
                    continue;
                }
                let foldable = matches!(&out[start], LottieDrawCommand::PushGroup { blend: LottieBlendMode::Normal, .. }) && content == 1 && out[start + 1].is_draw();
                if !foldable {
                    out.push(LottieDrawCommand::Pop);
                    continue;
                }
                let draw = out.pop().expect("one draw");
                let Some(LottieDrawCommand::PushGroup { alpha, area, area_rect, transform, .. }) = out.pop() else { unreachable!("checked above") };
                folded += 1;
                if alpha <= 0.0 {
                    continue;
                }
                let draw = with_alpha(draw, alpha);
                if inside(&draw, area_rect, transform) {
                    out.push(draw);
                } else {
                    out.push(LottieDrawCommand::PushClip { path: area, rect: area_rect, transform });
                    out.push(draw);
                    out.push(LottieDrawCommand::Pop);
                }
            }
            other => out.push(other),
        }
    }
    (out, folded)
}

fn with_alpha(draw: LottieDrawCommand, alpha: f32) -> LottieDrawCommand {
    match draw {
        LottieDrawCommand::Fill { path, transform, paint, opacity } => LottieDrawCommand::Fill { path, transform, paint, opacity: opacity * alpha },
        LottieDrawCommand::Stroke { path, transform, mut stroke } => {
            stroke.opacity *= alpha;
            LottieDrawCommand::Stroke { path, transform, stroke }
        }
        LottieDrawCommand::Image { asset_id, size, transform, opacity } => LottieDrawCommand::Image { asset_id, size, transform, opacity: opacity * alpha },
        other => other,
    }
}

fn device_bounds(path: &Arc<BezPath>, transform: Affine, reach: f64) -> Rect {
    transform.transform_rect_bbox(path.bounding_box().inflate(reach, reach))
}

/// True when the draw cannot reach outside the group's area (a rectangle under a transform
/// that keeps it axis-aligned).
fn inside(draw: &LottieDrawCommand, area_rect: Option<Rect>, area_transform: Affine) -> bool {
    let Some(rect) = area_rect else { return false };
    let [_, b, c, _, _, _] = area_transform.as_coeffs();
    if b != 0.0 || c != 0.0 {
        return false;
    }
    let area = area_transform.transform_rect_bbox(rect);
    let bounds = match draw {
        LottieDrawCommand::Fill { path, transform, .. } => device_bounds(path, *transform, 0.0),
        LottieDrawCommand::Stroke { path, transform, stroke } => device_bounds(path, *transform, stroke.reach()),
        LottieDrawCommand::Image { size: Some((w, h)), transform, .. } => transform.transform_rect_bbox(Rect::new(0.0, 0.0, *w, *h)),
        _ => return false,
    };
    bounds.x0 >= area.x0 && bounds.y0 >= area.y0 && bounds.x1 <= area.x1 && bounds.y1 <= area.y1
}
