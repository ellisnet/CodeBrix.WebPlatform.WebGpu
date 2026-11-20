//! Crate-internal: the DRAWING HALF of SVG documents - a [`DrawingSvgShape`]'s draw list for the
//! session's time mapped, command by command, onto the renderer's [`VectorScene`] (groups with
//! opacity and blend modes, clips - several shapes as their UNION -, masks, fills and strokes
//! with solid and gradient paints, the one plain Gaussian blur, and images whose pixels the
//! application supplied).

use kurbo::{Affine, Rect};
use peniko::{BlendMode, Brush, ColorStops, Compose, Fill, Gradient, InterpolationAlphaSpace, Mix};

use crate::color::Color;
use crate::effects::effect_sizes::EffectParams;
use crate::effects::DrawingEffects;
use crate::rendering::VectorScene;
use crate::shapes::EncodeContext;
use crate::svg::drawing_svg_shape::DrawingSvgShape;
use crate::svg::svg_draw_command::{SvgClipShape, SvgDrawCommand};
use crate::svg::svg_group_node::SvgBlendMode;
use crate::svg::svg_paint::{SvgFillRule, SvgGradientStop, SvgPaint, SvgSpread};

/// What a push left open, closed by its pop.
enum Open {
    /// This many renderer layers (groups, clips) to pop.
    Layers(usize),
    /// A clip of several shapes: the content is in a group of its own; at the pop the shapes
    /// are drawn into a destination-in group (their union keeps the content), then both close.
    Union(Vec<SvgClipShape>),
}

fn rule(r: SvgFillRule) -> Fill {
    match r {
        SvgFillRule::NonZero => Fill::NonZero,
        SvgFillRule::EvenOdd => Fill::EvenOdd,
    }
}

fn extend(s: SvgSpread) -> peniko::Extend {
    match s {
        SvgSpread::Pad => peniko::Extend::Pad,
        SvgSpread::Reflect => peniko::Extend::Reflect,
        SvgSpread::Repeat => peniko::Extend::Repeat,
    }
}

pub(crate) fn blend_mode(b: SvgBlendMode) -> BlendMode {
    let mix = match b {
        SvgBlendMode::Normal => Mix::Normal,
        SvgBlendMode::Multiply => Mix::Multiply,
        SvgBlendMode::Screen => Mix::Screen,
        SvgBlendMode::Overlay => Mix::Overlay,
        SvgBlendMode::Darken => Mix::Darken,
        SvgBlendMode::Lighten => Mix::Lighten,
        SvgBlendMode::ColorDodge => Mix::ColorDodge,
        SvgBlendMode::ColorBurn => Mix::ColorBurn,
        SvgBlendMode::HardLight => Mix::HardLight,
        SvgBlendMode::SoftLight => Mix::SoftLight,
        SvgBlendMode::Difference => Mix::Difference,
        SvgBlendMode::Exclusion => Mix::Exclusion,
        SvgBlendMode::Hue => Mix::Hue,
        SvgBlendMode::Saturation => Mix::Saturation,
        SvgBlendMode::Color => Mix::Color,
        SvgBlendMode::Luminosity => Mix::Luminosity,
        // plus-lighter is a compose mode (sum), not a mix.
        SvgBlendMode::PlusLighter => return BlendMode::new(Mix::Normal, Compose::Plus),
    };
    BlendMode::new(mix, Compose::SrcOver)
}

fn stops(list: &[SvgGradientStop], opacity: f32) -> ColorStops {
    let mut out = ColorStops::new();
    for s in list {
        out.0.push(peniko::ColorStop::from((s.offset, s.color.to_peniko(opacity))));
    }
    out
}

/// A paint as a renderer brush and the brush's own transform (gradients: their
/// gradientTransform, in the path's coordinates). None for a paint that is not drawn.
pub(crate) fn brush(paint: &SvgPaint, opacity: f32) -> Option<(Brush, Option<Affine>)> {
    Some(match paint {
        SvgPaint::Solid(c) => (Brush::Solid(c.to_peniko(opacity)), None),
        SvgPaint::Linear(l) => {
            let g = Gradient::new_linear(l.start, l.end).with_extend(extend(l.spread));
            (Brush::Gradient(Gradient { stops: stops(&l.stops, opacity), interpolation_alpha_space: InterpolationAlphaSpace::Unpremultiplied, ..g }), Some(l.transform))
        }
        SvgPaint::Radial(r) => {
            let g = Gradient::new_two_point_radial(r.focal, r.focal_radius as f32, r.center, r.radius as f32).with_extend(extend(r.spread));
            (Brush::Gradient(Gradient { stops: stops(&r.stops, opacity), interpolation_alpha_space: InterpolationAlphaSpace::Unpremultiplied, ..g }), Some(r.transform))
        }
        SvgPaint::Pattern(_) => return None,
    })
}

impl DrawingSvgShape {
    /// Records the shape at the session's time: `transform` takes the shape's calibrated
    /// coordinates to canvas pixels; `opacity` is the element's (the whole drawing fades as one).
    pub(crate) fn encode(&self, scene: &mut VectorScene, transform: Affine, opacity: f32, ctx: &EncodeContext<'_>) {
        let list = self.draw_list_at(ctx.time);
        if list.commands.is_empty() {
            return;
        }
        let area = ctx.area;
        let fading = opacity < 1.0;
        if fading {
            scene.push_layer(BlendMode::default(), opacity, Affine::IDENTITY, &area);
        }
        let white = Brush::Solid(Color::WHITE.to_peniko(1.0));
        let mut open: Vec<Open> = Vec::new();
        for command in &list.commands {
            match command {
                SvgDrawCommand::PushGroup { alpha, blend, isolate: _, blur_std_deviation, transform: t } => {
                    // The renderer keeps a group offscreen whenever it must (another blend mode
                    // inside it, a mask composited into it); it folds the others.
                    let device = transform * *t;
                    let blur = blur_std_deviation.and_then(|s| DrawingEffects::blur(2.0 * s).ok()).and_then(|e| EffectParams::new(&e, device, ctx.scene_rotation));
                    match blur {
                        Some(params) => {
                            // SVG's stdDeviation is the Gaussian's sigma; the crate's blur radius
                            // is two sigma.
                            if *blend != SvgBlendMode::Normal {
                                scene.push_layer(blend_mode(*blend), *alpha, Affine::IDENTITY, &area);
                                scene.push_effects(params, 1.0);
                                open.push(Open::Layers(2));
                            } else {
                                scene.push_effects(params, *alpha);
                                open.push(Open::Layers(1));
                            }
                        }
                        None => {
                            scene.push_layer(blend_mode(*blend), *alpha, Affine::IDENTITY, &area);
                            open.push(Open::Layers(1));
                        }
                    }
                }
                SvgDrawCommand::PushClip { shapes } => match shapes.len() {
                    0 => {
                        // A clip path with nothing in it shows nothing.
                        scene.push_clip_layer(Affine::IDENTITY, &Rect::ZERO);
                        open.push(Open::Layers(1));
                    }
                    1 => {
                        let s = &shapes[0];
                        scene.push_clip_layer_with_rule(rule(s.rule), transform * s.transform, &*s.path);
                        open.push(Open::Layers(1));
                    }
                    _ => {
                        scene.push_layer(BlendMode::default(), 1.0, Affine::IDENTITY, &area);
                        open.push(Open::Union(shapes.clone()));
                    }
                },
                SvgDrawCommand::Fill { path, transform: t, rule: r, paint, opacity, anti_alias: _ } => {
                    if let Some((b, bt)) = brush(paint, *opacity) {
                        scene.fill(rule(*r), transform * *t, &b, bt, &**path);
                    }
                }
                SvgDrawCommand::Stroke { path, transform: t, stroke, anti_alias: _ } => {
                    if let Some((b, bt)) = brush(&stroke.paint, stroke.opacity) {
                        scene.stroke(&stroke.to_kurbo(), transform * *t, &b, bt, &**path);
                    }
                }
                SvgDrawCommand::Image { source, size, transform: t, smooth } => {
                    if let Some(pixels) = self.image(source) {
                        let (w, h) = (pixels.width().max(1) as f64, pixels.height().max(1) as f64);
                        let place = Affine::scale_non_uniform(size.0 / w, size.1 / h);
                        scene.fill_picture(transform * *t, pixels.image_data(), place, &Rect::new(0.0, 0.0, size.0, size.1), !*smooth, None, 1.0);
                    }
                }
                SvgDrawCommand::MaskBegin { kind: _, rect, transform: t } => {
                    // What follows is the mask's content, composited destination-in over what
                    // the group drew (its alpha keeps it; outside the mask's region nothing
                    // stays). Luminance masks are applied by their alpha (reported on load).
                    scene.push_layer(Compose::DestIn, 1.0, Affine::IDENTITY, &area);
                    scene.push_clip_layer(transform * *t, rect);
                    open.push(Open::Layers(2));
                }
                SvgDrawCommand::MaskEnd | SvgDrawCommand::Pop => match open.pop() {
                    Some(Open::Layers(n)) => {
                        for _ in 0..n {
                            scene.pop_layer();
                        }
                    }
                    Some(Open::Union(shapes)) => {
                        scene.push_layer(Compose::DestIn, 1.0, Affine::IDENTITY, &area);
                        for s in &shapes {
                            scene.fill(rule(s.rule), transform * s.transform, &white, None, &*s.path);
                        }
                        scene.pop_layer();
                        scene.pop_layer();
                    }
                    None => {}
                },
            }
        }
        // A list is always balanced; close anything left open all the same.
        while let Some(o) = open.pop() {
            let n = match o {
                Open::Layers(n) => n,
                Open::Union(_) => 1,
            };
            for _ in 0..n {
                scene.pop_layer();
            }
        }
        if fading {
            scene.pop_layer();
        }
    }
}
