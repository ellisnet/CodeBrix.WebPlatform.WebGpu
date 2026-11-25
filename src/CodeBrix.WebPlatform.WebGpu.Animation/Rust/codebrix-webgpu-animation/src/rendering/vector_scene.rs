//! `VectorScene`: what the renderer draws in one frame - fills, strokes, clips and groups with
//! peniko brushes and kurbo shapes. A [`DrawingSession`](crate::DrawingSession) records itself
//! into one ([`DrawingSession::render`](crate::DrawingSession::render)); it is also the
//! low-level door for content that has no session API yet (the Lottie step, for example, feeds
//! velato's output into it), the way low-level wgpu stays available beside the host.

use std::collections::HashMap;
use std::ops::Range;

use kurbo::{Affine, PathEl, Shape, Stroke};
use peniko::{BlendMode, Brush, Fill, ImageBrush, ImageData, ImageQuality, ImageSampler};

use crate::effects::effect_sizes::EffectParams;
use crate::paths::PathStrokeStyle;
use crate::rendering::scene_look::SceneLook;
use crate::shapes::noise_shape::NoisePaintParams;
use crate::shapes::ownership_shape::OwnersPaintParams;
use crate::shapes::particles_shape::DrawnParticle;
use crate::shapes::brush_strokes_shape::StrokeDraw;

/// The tolerance at which kurbo's shapes (circles, ellipses, rounded rectangles) become curves.
const SHAPE_TOLERANCE: f64 = 0.01;

/// One recorded item.
#[derive(Clone, Debug)]
pub(crate) enum Item {
    Fill { rule: Fill, transform: Affine, brush: u32, brush_transform: Option<Affine>, path: Range<u32>, key: u64, tint: Option<[f32; 4]> },
    Stroke { stroke: u32, transform: Affine, brush: u32, brush_transform: Option<Affine>, path: Range<u32>, key: u64 },
    PushLayer { blend: BlendMode, alpha: f32, transform: Affine, shape: Range<u32> },
    PushClip { rule: Fill, transform: Affine, shape: Range<u32> },
    /// A rectangle (or any path) filled with noise computed on the GPU; `brush` is the ramp.
    FillNoise { transform: Affine, noise: NoisePaintParams, brush: u32, path: Range<u32>, key: u64 },
    /// An ownership map's picture (owner numbers, the colours below them) coloured per pixel on
    /// the GPU; `brush` is the picture, `palette` (a brush index) the colour table drawn with
    /// it when it is not the picture's own rows.
    FillOwners { transform: Affine, owners: OwnersPaintParams, brush: u32, palette: Option<u32>, place: Affine, path: Range<u32>, key: u64 },
    /// Particles as soft discs in one instanced draw: `particles` indexes the scene's list.
    Particles { transform: Affine, particles: Range<u32>, softness: f32, alpha: f32 },
    /// A stroke set in one instanced draw: `draw` indexes the scene's stroke draws (Not in C#).
    Strokes { transform: Affine, draw: u32 },
    /// A group drawn offscreen and put down through its effects (blur, glow, shadow) at `alpha`.
    PushEffects { params: EffectParams, alpha: f32 },
    Pop,
}

/// A frame's drawing: an ordered list of fills, strokes, clip layers and blended groups. Every
/// `push_*` is closed by a [`pop_layer`](VectorScene::pop_layer). Colours are peniko's (sRGB,
/// straight alpha); the renderer draws them linear and premultiplied, as the canvas host
/// requires. Reuse one scene from frame to frame ([`reset`](VectorScene::reset)) so its storage
/// is kept.
#[derive(Clone, Debug, Default)]
pub struct VectorScene {
    pub(crate) items: Vec<Item>,
    pub(crate) elements: Vec<PathEl>,
    pub(crate) brushes: Vec<Brush>,
    pub(crate) strokes: Vec<Stroke>,
    pub(crate) particles: Vec<DrawnParticle>,
    /// The stroke sets' draws (sets, tips, anchors and the frame's numbers).
    pub(crate) stroke_draws: Vec<StrokeDraw>,
    /// The render-time look a session records under (crate-internal: its paint override and
    /// effects switch, set around a layer's content); None draws everything as given.
    pub(crate) look: Option<SceneLook>,
    /// One entry per open push: true when it is a COVERAGE group (destination-in / -out and the
    /// like - a mask), inside which the look never applies.
    pub(crate) open: Vec<bool>,
    /// How many coverage groups are open.
    pub(crate) coverage_depth: usize,
    /// The average colours of pictures (by their pixels' blob), kept from frame to frame.
    pub(crate) averages: HashMap<u64, [f32; 4]>,
    /// The water of the frame's effect groups (no C# equivalent): each refraction's and
    /// reflection's source as drawn this frame, named by the group's `EffectParams::water.draw`.
    pub(crate) water: Vec<WaterDraw>,
}

/// One effect group's water this frame (crate-internal; no C# equivalent): the refraction's
/// source and the reflection's OWN source (None when it has none or shares the refraction's),
/// ripple fields already packed at the frame's time.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct WaterDraw {
    pub(crate) refraction: Option<crate::water::water_source::WaterTap>,
    pub(crate) reflection: Option<crate::water::water_source::WaterTap>,
}

impl VectorScene {
    /// An empty scene.
    pub fn new() -> VectorScene {
        VectorScene::default()
    }

    /// Empties the scene, keeping its storage.
    pub fn reset(&mut self) {
        self.items.clear();
        self.elements.clear();
        self.brushes.clear();
        self.strokes.clear();
        self.particles.clear();
        self.stroke_draws.clear();
        self.water.clear();
        self.look = None;
        self.open.clear();
        self.coverage_depth = 0;
    }

    /// True when nothing is recorded.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Fills a shape with a fill rule. `brush_transform` places a gradient or image brush
    /// within the shape's space (None: the shape's own coordinates).
    pub fn fill(&mut self, style: Fill, transform: Affine, brush: &Brush, brush_transform: Option<Affine>, shape: &impl Shape) {
        if let Some(look) = self.take_paint_look() {
            self.fill_in_look(&look, style, transform, brush, brush_transform, shape);
            self.look = Some(look);
            return;
        }
        self.record_fill(style, transform, brush, brush_transform, shape);
    }

    /// Records a fill as given (no look).
    pub(crate) fn record_fill(&mut self, style: Fill, transform: Affine, brush: &Brush, brush_transform: Option<Affine>, shape: &impl Shape) {
        let (path, key) = self.push_shape(shape);
        let brush = self.push_brush(brush);
        self.items.push(Item::Fill { rule: style, transform, brush, brush_transform, path, key, tint: None });
    }

    /// Fills elements whose geometry key the caller already knows (a glyph of a font face): the
    /// elements are copied, not hashed, and the renderer keeps their flattened geometry under
    /// that key from frame to frame. `element` takes the element (the text) to canvas pixels,
    /// `local` the elements (the glyph's font units) into the element's space.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fill_cached(&mut self, style: Fill, element: Affine, local: Affine, brush: &Brush, brush_transform: Option<Affine>, elements: &[PathEl], key: u64) {
        if let Some(look) = self.take_paint_look() {
            self.fill_cached_in_look(&look, style, element, local, brush, brush_transform, elements, key);
            self.look = Some(look);
            return;
        }
        self.record_fill_cached(style, element * local, brush, brush_transform, elements, key);
    }

    /// Records a cached fill as given (no look).
    pub(crate) fn record_fill_cached(&mut self, style: Fill, transform: Affine, brush: &Brush, brush_transform: Option<Affine>, elements: &[PathEl], key: u64) {
        let start = self.elements.len() as u32;
        self.elements.extend_from_slice(elements);
        let path = start..self.elements.len() as u32;
        let brush = self.push_brush(brush);
        self.items.push(Item::Fill { rule: style, transform, brush, brush_transform, path, key, tint: None });
    }

    /// Strokes a shape (kurbo's stroke: width, joins, caps, miter limit, dashes). The stroke is
    /// expanded in the shape's own space, so a transform scales it as it scales the shape.
    pub fn stroke(&mut self, style: &Stroke, transform: Affine, brush: &Brush, brush_transform: Option<Affine>, shape: &impl Shape) {
        if let Some(look) = self.take_paint_look() {
            self.stroke_in_look(&look, style, transform, None, brush, brush_transform, shape);
            self.look = Some(look);
            return;
        }
        self.record_stroke(style, transform, brush, brush_transform, shape);
    }

    /// Records a stroke as given (no look).
    pub(crate) fn record_stroke(&mut self, style: &Stroke, transform: Affine, brush: &Brush, brush_transform: Option<Affine>, shape: &impl Shape) {
        if !(style.width > 0.0) {
            return;
        }
        let (path, key) = self.push_shape(shape);
        let brush = self.push_brush(brush);
        self.strokes.push(style.clone());
        let stroke = (self.strokes.len() - 1) as u32;
        self.items.push(Item::Stroke { stroke, transform, brush, brush_transform, path, key });
    }

    /// Strokes a shape with a width FIXED in a reference space (Not in C#; SVG's
    /// `vector-effect="non-scaling-stroke"`): `transform` takes the shape to canvas pixels as in
    /// [`stroke`](VectorScene::stroke), `reference` takes the reference space (a session's
    /// drawing units: its calibration and camera) to canvas pixels. The shape is carried into the
    /// reference space and expanded there, so whatever scaling lies between the shape and the
    /// reference space - uneven scaling included - leaves the width (and the dashes) as given in
    /// reference units. A shape whose transform collapses it (a zero scale) draws nothing.
    #[allow(clippy::too_many_arguments)]
    pub fn stroke_fixed_width(&mut self, style: &Stroke, transform: Affine, reference: Affine, brush: &Brush, brush_transform: Option<Affine>, shape: &impl Shape) {
        if let Some(look) = self.take_paint_look() {
            self.stroke_in_look(&look, style, transform, Some(reference), brush, brush_transform, shape);
            self.look = Some(look);
            return;
        }
        self.record_stroke_fixed(style, transform, reference, brush, brush_transform, shape);
    }

    /// Records a fixed-width stroke as given (no look).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_stroke_fixed(&mut self, style: &Stroke, transform: Affine, reference: Affine, brush: &Brush, brush_transform: Option<Affine>, shape: &impl Shape) {
        if !(style.width > 0.0) {
            return;
        }
        let det = reference.determinant();
        if !(det.abs() > 1e-12) || !det.is_finite() {
            return;
        }
        let local = reference.inverse() * transform;
        let local_det = local.determinant();
        if !(local_det.abs() > 1e-12) || !local_det.is_finite() {
            return;
        }
        let start = self.elements.len() as u32;
        let mut hash = Hasher::new();
        for el in shape.path_elements(SHAPE_TOLERANCE) {
            let el = local * el;
            hash.element(&el);
            self.elements.push(el);
        }
        let path = start..self.elements.len() as u32;
        let key = hash.finish();
        // A gradient or image brush is placed in the shape's space: carried along with it.
        let brush_transform = match brush {
            Brush::Solid(_) => brush_transform,
            _ => Some(local * brush_transform.unwrap_or(Affine::IDENTITY)),
        };
        let brush = self.push_brush(brush);
        self.strokes.push(style.clone());
        let stroke = (self.strokes.len() - 1) as u32;
        self.items.push(Item::Stroke { stroke, transform: reference, brush, brush_transform, path, key });
    }

    /// Strokes a shape with a [`PathStrokeStyle`]: [`stroke`](VectorScene::stroke) when its width
    /// scales with the element, [`stroke_fixed_width`](VectorScene::stroke_fixed_width) against
    /// `reference` (the session's drawing units) when it is fixed.
    pub(crate) fn stroke_with_style(&mut self, style: &PathStrokeStyle, transform: Affine, reference: Affine, brush: &Brush, brush_transform: Option<Affine>, shape: &impl Shape) {
        if style.is_fixed_width() {
            self.stroke_fixed_width(&style.to_kurbo(), transform, reference, brush, brush_transform, shape);
        } else {
            self.stroke(&style.to_kurbo(), transform, brush, brush_transform, shape);
        }
    }

    /// Draws an image (its pixels at their own size, placed by `transform`).
    pub fn draw_image(&mut self, image: &ImageBrush, transform: Affine) {
        let rect = kurbo::Rect::new(0.0, 0.0, image.image.width as f64, image.image.height as f64);
        self.fill(Fill::NonZero, transform, &Brush::Image(image.clone()), None, &rect);
    }

    /// Fills a rectangle with a picture placed by `place` (picture pixels to the rectangle's
    /// space), sampled nearest or bilinear, its straight sRGB values multiplied by `tint`, at
    /// `alpha` (pictures and sprites).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fill_picture(&mut self, transform: Affine, image: &ImageData, place: Affine, rect: &kurbo::Rect, nearest: bool, tint: Option<[f32; 4]>, alpha: f32) {
        if let Some(look) = self.take_paint_look() {
            self.picture_in_look(&look, transform, image, rect, tint, alpha);
            self.look = Some(look);
            return;
        }
        let sampler = ImageSampler { quality: if nearest { ImageQuality::Low } else { ImageQuality::Medium }, alpha: alpha.clamp(0.0, 1.0), ..ImageSampler::default() };
        let brush = Brush::Image(ImageBrush { image: image.clone(), sampler });
        let (path, key) = self.push_shape(rect);
        let brush = self.push_brush(&brush);
        self.items.push(Item::Fill { rule: Fill::NonZero, transform, brush, brush_transform: Some(place), path, key, tint });
    }

    /// Fills a shape with noise computed per pixel on the GPU, its value mapped through the
    /// gradient `ramp` (crate-internal: [`NoiseShape`](crate::shapes::NoiseShape)).
    pub(crate) fn fill_noise(&mut self, transform: Affine, noise: NoisePaintParams, ramp: &peniko::Gradient, shape: &impl Shape) {
        if let Some(look) = self.take_paint_look() {
            // Line: left out; flat: the ramp's average colour.
            if matches!(look.paint, crate::drawing_session_paint::SessionPaint::Flat) {
                let brush = self.representative(&Brush::Gradient(ramp.clone()));
                self.record_fill(Fill::NonZero, transform, &brush, None, shape);
            }
            self.look = Some(look);
            return;
        }
        let (path, key) = self.push_shape(shape);
        let brush = self.push_brush(&Brush::Gradient(ramp.clone()));
        self.items.push(Item::FillNoise { transform, noise, brush, path, key });
    }

    /// Fills a rectangle with an ownership map's picture coloured on the GPU (crate-internal:
    /// [`OwnershipShape`](crate::shapes::OwnershipShape)): `palette` is the colour table (as
    /// rows of the picture's width) when it is not the picture's own; `representative` the
    /// colour a flat look paints it with.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fill_owners(&mut self, transform: Affine, image: &ImageData, palette: Option<&ImageData>, place: Affine, rect: &kurbo::Rect, owners: OwnersPaintParams, alpha: f32, representative: crate::color::Color) {
        if let Some(look) = self.take_paint_look() {
            // Line: left out; flat: the owners' colours averaged by the pixels they own.
            if matches!(look.paint, crate::drawing_session_paint::SessionPaint::Flat) {
                self.record_fill(Fill::NonZero, transform, &Brush::Solid(representative.to_peniko(alpha)), None, rect);
            }
            self.look = Some(look);
            return;
        }
        let sampler = ImageSampler { quality: ImageQuality::Low, alpha: alpha.clamp(0.0, 1.0), ..ImageSampler::default() };
        let brush = Brush::Image(ImageBrush { image: image.clone(), sampler });
        let (path, key) = self.push_shape(rect);
        let brush = self.push_brush(&brush);
        let palette = palette.map(|p| self.push_brush(&Brush::Image(ImageBrush { image: p.clone(), sampler: ImageSampler::default() })));
        self.items.push(Item::FillOwners { transform, owners, brush, palette, place, path, key });
    }

    /// Draws particles as soft discs in one instanced draw (crate-internal:
    /// [`ParticlesShape`](crate::shapes::ParticlesShape)).
    pub(crate) fn draw_particles(&mut self, transform: Affine, particles: &[DrawnParticle], softness: f32, alpha: f32) {
        if particles.is_empty() || alpha <= 0.0 {
            return;
        }
        // A line drawing leaves particles out (a flat look keeps their solid colours).
        if self.coverage_depth == 0 && matches!(self.look.as_ref().map(|l| &l.paint), Some(crate::drawing_session_paint::SessionPaint::Line(_))) {
            return;
        }
        let start = self.particles.len() as u32;
        self.particles.extend_from_slice(particles);
        self.items.push(Item::Particles { transform, particles: start..self.particles.len() as u32, softness, alpha: alpha.clamp(0.0, 1.0) });
    }

    /// Draws a stroke set in one instanced draw (crate-internal:
    /// [`BrushStrokesShape`](crate::shapes::BrushStrokesShape)). A line drawing leaves it out; a
    /// flat look draws each stroke in its own colour without its tip.
    pub(crate) fn draw_strokes(&mut self, transform: Affine, mut draw: StrokeDraw) {
        if draw.set.is_empty() || !(draw.opacity > 0.0) {
            return;
        }
        if self.coverage_depth == 0 {
            match self.look.as_ref().map(|l| &l.paint) {
                Some(crate::drawing_session_paint::SessionPaint::Line(_)) => return,
                Some(crate::drawing_session_paint::SessionPaint::Flat) => draw.tips = None,
                _ => {}
            }
        }
        draw.opacity = draw.opacity.clamp(0.0, 1.0);
        self.stroke_draws.push(draw);
        self.items.push(Item::Strokes { transform, draw: (self.stroke_draws.len() - 1) as u32 });
    }

    /// Starts a group drawn offscreen and put down through effects at `alpha` (crate-internal:
    /// the session's elements, groups and layers with [`DrawingEffects`](crate::effects::DrawingEffects)).
    pub(crate) fn push_effects(&mut self, params: EffectParams, alpha: f32) {
        let effects_off = self.coverage_depth == 0 && self.look.as_ref().map(|l| !l.effects).unwrap_or(false);
        if effects_off {
            // The effects switch off: the blur, glow and shadow go; a layer's frame operations,
            // transition and water (refraction, reflection), which ride on the same group, stay;
            // without them a plain group.
            if params.frame.is_some() || params.water.is_some() {
                self.open.push(false);
                self.items.push(Item::PushEffects { params: EffectParams { frame: params.frame, water: params.water, ..EffectParams::default() }, alpha: alpha.clamp(0.0, 1.0) });
            } else {
                self.push_layer(BlendMode::default(), alpha, Affine::IDENTITY, &kurbo::Rect::new(-1e9, -1e9, 1e9, 1e9));
            }
            return;
        }
        self.open.push(false);
        self.items.push(Item::PushEffects { params, alpha: alpha.clamp(0.0, 1.0) });
    }

    /// Starts a group: what is drawn until the matching pop is composited with `blend` (mix
    /// and compose modes) and `alpha`, inside `shape` (under `transform`, non-zero). A group
    /// is drawn offscreen only when it cannot be folded into direct drawing.
    pub fn push_layer(&mut self, blend: impl Into<BlendMode>, alpha: f32, transform: Affine, shape: &impl Shape) {
        let blend = blend.into();
        let coverage = crate::rendering::scene_look::is_coverage(blend.compose);
        self.open.push(coverage);
        if coverage {
            self.coverage_depth += 1;
        }
        let (shape, _) = self.push_shape(shape);
        self.items.push(Item::PushLayer { blend, alpha: alpha.clamp(0.0, 1.0), transform, shape });
    }

    /// Starts a clip: what is drawn until the matching pop shows only inside `shape` (non-zero).
    pub fn push_clip_layer(&mut self, transform: Affine, shape: &impl Shape) {
        self.push_clip_layer_with_rule(Fill::NonZero, transform, shape);
    }

    /// Starts a clip with a fill rule.
    pub fn push_clip_layer_with_rule(&mut self, rule: Fill, transform: Affine, shape: &impl Shape) {
        self.open.push(false);
        let (shape, _) = self.push_shape(shape);
        self.items.push(Item::PushClip { rule, transform, shape });
    }

    /// Ends the innermost group or clip.
    pub fn pop_layer(&mut self) {
        if self.open.pop() == Some(true) {
            self.coverage_depth -= 1;
        }
        self.items.push(Item::Pop);
    }

    /// The number of fills and strokes recorded.
    pub fn draw_count(&self) -> usize {
        self.items.iter().filter(|i| matches!(i, Item::Fill { .. } | Item::Stroke { .. } | Item::FillNoise { .. } | Item::Particles { .. } | Item::FillOwners { .. } | Item::Strokes { .. })).count()
    }

    /// The geometry keys of the recorded fills (tests).
    #[cfg(test)]
    pub(crate) fn fill_keys(&self) -> Vec<u64> {
        self.items.iter().filter_map(|i| if let Item::Fill { key, .. } = i { Some(*key) } else { None }).collect()
    }

    /// The kinds of the recorded items in order, one letter each: F fill, S stroke, L group,
    /// C clip, E effects, P pop, N noise, Q particles, O ownership map, B stroke set (tests).
    #[cfg(test)]
    pub(crate) fn item_kinds(&self) -> String {
        self.items.iter().map(|i| match i { Item::Fill { .. } => 'F', Item::Stroke { .. } => 'S', Item::PushLayer { .. } => 'L', Item::PushClip { .. } => 'C', Item::PushEffects { .. } => 'E', Item::Pop => 'P', Item::FillNoise { .. } => 'N', Item::Particles { .. } => 'Q', Item::FillOwners { .. } => 'O', Item::Strokes { .. } => 'B' }).collect()
    }

    pub(crate) fn push_brush(&mut self, brush: &Brush) -> u32 {
        self.brushes.push(brush.clone());
        (self.brushes.len() - 1) as u32
    }

    /// Appends the shape's elements; returns their range and a hash of them (the geometry
    /// cache's key: the same elements give the same outline, whatever the transform).
    pub(crate) fn push_shape(&mut self, shape: &impl Shape) -> (Range<u32>, u64) {
        let start = self.elements.len() as u32;
        let mut hash = Hasher::new();
        for el in shape.path_elements(SHAPE_TOLERANCE) {
            hash.element(&el);
            self.elements.push(el);
        }
        (start..self.elements.len() as u32, hash.finish())
    }
}

/// A small, fast, non-cryptographic hash (multiply-rotate) for geometry and style keys.
pub(crate) struct Hasher(u64);

impl Hasher {
    pub(crate) fn new() -> Hasher {
        Hasher(0xcbf2_9ce4_8422_2325)
    }

    pub(crate) fn word(&mut self, w: u64) {
        self.0 = (self.0.rotate_left(5) ^ w).wrapping_mul(0x517c_c1b7_2722_0a95);
    }

    pub(crate) fn f64(&mut self, v: f64) {
        self.word(v.to_bits());
    }

    pub(crate) fn element(&mut self, el: &PathEl) {
        match el {
            PathEl::MoveTo(p) => {
                self.word(1);
                self.f64(p.x);
                self.f64(p.y);
            }
            PathEl::LineTo(p) => {
                self.word(2);
                self.f64(p.x);
                self.f64(p.y);
            }
            PathEl::QuadTo(a, b) => {
                self.word(3);
                for p in [a, b] {
                    self.f64(p.x);
                    self.f64(p.y);
                }
            }
            PathEl::CurveTo(a, b, c) => {
                self.word(4);
                for p in [a, b, c] {
                    self.f64(p.x);
                    self.f64(p.y);
                }
            }
            PathEl::ClosePath => self.word(5),
        }
    }

    pub(crate) fn finish(&self) -> u64 {
        self.0 ^ (self.0 >> 29)
    }
}
