//! `VectorScene`: what the renderer draws in one frame - fills, strokes, clips and groups with
//! peniko brushes and kurbo shapes. A [`DrawingSession`](crate::DrawingSession) records itself
//! into one ([`DrawingSession::render`](crate::DrawingSession::render)); it is also the
//! low-level door for content that has no session API yet (the Lottie step, for example, feeds
//! velato's output into it), the way low-level wgpu stays available beside the host.

use std::ops::Range;

use kurbo::{Affine, PathEl, Shape, Stroke};
use peniko::{BlendMode, Brush, Fill, ImageBrush, ImageData, ImageQuality, ImageSampler};

use crate::effects::effect_sizes::EffectParams;
use crate::shapes::noise_shape::NoisePaintParams;
use crate::shapes::ownership_shape::OwnersPaintParams;
use crate::shapes::particles_shape::DrawnParticle;

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
    /// the GPU; `brush` is the picture.
    FillOwners { transform: Affine, owners: OwnersPaintParams, brush: u32, place: Affine, path: Range<u32>, key: u64 },
    /// Particles as soft discs in one instanced draw: `particles` indexes the scene's list.
    Particles { transform: Affine, particles: Range<u32>, softness: f32, alpha: f32 },
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
    }

    /// True when nothing is recorded.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Fills a shape with a fill rule. `brush_transform` places a gradient or image brush
    /// within the shape's space (None: the shape's own coordinates).
    pub fn fill(&mut self, style: Fill, transform: Affine, brush: &Brush, brush_transform: Option<Affine>, shape: &impl Shape) {
        let (path, key) = self.push_shape(shape);
        let brush = self.push_brush(brush);
        self.items.push(Item::Fill { rule: style, transform, brush, brush_transform, path, key, tint: None });
    }

    /// Fills elements whose geometry key the caller already knows (a glyph of a font face): the
    /// elements are copied, not hashed, and the renderer keeps their flattened geometry under
    /// that key from frame to frame.
    pub(crate) fn fill_cached(&mut self, style: Fill, transform: Affine, brush: &Brush, brush_transform: Option<Affine>, elements: &[PathEl], key: u64) {
        let start = self.elements.len() as u32;
        self.elements.extend_from_slice(elements);
        let path = start..self.elements.len() as u32;
        let brush = self.push_brush(brush);
        self.items.push(Item::Fill { rule: style, transform, brush, brush_transform, path, key, tint: None });
    }

    /// Strokes a shape (kurbo's stroke: width, joins, caps, miter limit, dashes). The stroke is
    /// expanded in the shape's own space, so a transform scales it as it scales the shape.
    pub fn stroke(&mut self, style: &Stroke, transform: Affine, brush: &Brush, brush_transform: Option<Affine>, shape: &impl Shape) {
        if !(style.width > 0.0) {
            return;
        }
        let (path, key) = self.push_shape(shape);
        let brush = self.push_brush(brush);
        self.strokes.push(style.clone());
        let stroke = (self.strokes.len() - 1) as u32;
        self.items.push(Item::Stroke { stroke, transform, brush, brush_transform, path, key });
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
        let sampler = ImageSampler { quality: if nearest { ImageQuality::Low } else { ImageQuality::Medium }, alpha: alpha.clamp(0.0, 1.0), ..ImageSampler::default() };
        let brush = Brush::Image(ImageBrush { image: image.clone(), sampler });
        let (path, key) = self.push_shape(rect);
        let brush = self.push_brush(&brush);
        self.items.push(Item::Fill { rule: Fill::NonZero, transform, brush, brush_transform: Some(place), path, key, tint });
    }

    /// Fills a shape with noise computed per pixel on the GPU, its value mapped through the
    /// gradient `ramp` (crate-internal: [`NoiseShape`](crate::shapes::NoiseShape)).
    pub(crate) fn fill_noise(&mut self, transform: Affine, noise: NoisePaintParams, ramp: &peniko::Gradient, shape: &impl Shape) {
        let (path, key) = self.push_shape(shape);
        let brush = self.push_brush(&Brush::Gradient(ramp.clone()));
        self.items.push(Item::FillNoise { transform, noise, brush, path, key });
    }

    /// Fills a rectangle with an ownership map's picture coloured on the GPU (crate-internal:
    /// [`OwnershipShape`](crate::shapes::OwnershipShape)).
    pub(crate) fn fill_owners(&mut self, transform: Affine, image: &ImageData, place: Affine, rect: &kurbo::Rect, owners: OwnersPaintParams, alpha: f32) {
        let sampler = ImageSampler { quality: ImageQuality::Low, alpha: alpha.clamp(0.0, 1.0), ..ImageSampler::default() };
        let brush = Brush::Image(ImageBrush { image: image.clone(), sampler });
        let (path, key) = self.push_shape(rect);
        let brush = self.push_brush(&brush);
        self.items.push(Item::FillOwners { transform, owners, brush, place, path, key });
    }

    /// Draws particles as soft discs in one instanced draw (crate-internal:
    /// [`ParticlesShape`](crate::shapes::ParticlesShape)).
    pub(crate) fn draw_particles(&mut self, transform: Affine, particles: &[DrawnParticle], softness: f32, alpha: f32) {
        if particles.is_empty() || alpha <= 0.0 {
            return;
        }
        let start = self.particles.len() as u32;
        self.particles.extend_from_slice(particles);
        self.items.push(Item::Particles { transform, particles: start..self.particles.len() as u32, softness, alpha: alpha.clamp(0.0, 1.0) });
    }

    /// Starts a group drawn offscreen and put down through effects at `alpha` (crate-internal:
    /// the session's elements, groups and layers with [`DrawingEffects`](crate::effects::DrawingEffects)).
    pub(crate) fn push_effects(&mut self, params: EffectParams, alpha: f32) {
        self.items.push(Item::PushEffects { params, alpha: alpha.clamp(0.0, 1.0) });
    }

    /// Starts a group: what is drawn until the matching pop is composited with `blend` (mix
    /// and compose modes) and `alpha`, inside `shape` (under `transform`, non-zero). A group
    /// is drawn offscreen only when it cannot be folded into direct drawing.
    pub fn push_layer(&mut self, blend: impl Into<BlendMode>, alpha: f32, transform: Affine, shape: &impl Shape) {
        let (shape, _) = self.push_shape(shape);
        self.items.push(Item::PushLayer { blend: blend.into(), alpha: alpha.clamp(0.0, 1.0), transform, shape });
    }

    /// Starts a clip: what is drawn until the matching pop shows only inside `shape` (non-zero).
    pub fn push_clip_layer(&mut self, transform: Affine, shape: &impl Shape) {
        self.push_clip_layer_with_rule(Fill::NonZero, transform, shape);
    }

    /// Starts a clip with a fill rule.
    pub fn push_clip_layer_with_rule(&mut self, rule: Fill, transform: Affine, shape: &impl Shape) {
        let (shape, _) = self.push_shape(shape);
        self.items.push(Item::PushClip { rule, transform, shape });
    }

    /// Ends the innermost group or clip.
    pub fn pop_layer(&mut self) {
        self.items.push(Item::Pop);
    }

    /// The number of fills and strokes recorded.
    pub fn draw_count(&self) -> usize {
        self.items.iter().filter(|i| matches!(i, Item::Fill { .. } | Item::Stroke { .. } | Item::FillNoise { .. } | Item::Particles { .. } | Item::FillOwners { .. })).count()
    }

    /// The geometry keys of the recorded fills (tests).
    #[cfg(test)]
    pub(crate) fn fill_keys(&self) -> Vec<u64> {
        self.items.iter().filter_map(|i| if let Item::Fill { key, .. } = i { Some(*key) } else { None }).collect()
    }

    /// The kinds of the recorded items in order, one letter each: F fill, S stroke, L group,
    /// C clip, E effects, P pop (tests).
    #[cfg(test)]
    pub(crate) fn item_kinds(&self) -> String {
        self.items.iter().map(|i| match i { Item::Fill { .. } => 'F', Item::Stroke { .. } => 'S', Item::PushLayer { .. } => 'L', Item::PushClip { .. } => 'C', Item::PushEffects { .. } => 'E', Item::Pop => 'P', Item::FillNoise { .. } => 'N', Item::Particles { .. } => 'Q', Item::FillOwners { .. } => 'O' }).collect()
    }

    fn push_brush(&mut self, brush: &Brush) -> u32 {
        self.brushes.push(brush.clone());
        (self.brushes.len() - 1) as u32
    }

    /// Appends the shape's elements; returns their range and a hash of them (the geometry
    /// cache's key: the same elements give the same outline, whatever the transform).
    fn push_shape(&mut self, shape: &impl Shape) -> (Range<u32>, u64) {
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
