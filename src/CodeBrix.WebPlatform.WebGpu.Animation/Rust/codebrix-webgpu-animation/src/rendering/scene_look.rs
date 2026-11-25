//! Crate-internal: the render-time LOOK a session records under - its paint override (as
//! authored, line, flat) and its effects switch ([`SessionRenderOptions`](crate::SessionRenderOptions)).
//! The look is applied where the scene records each drawing (fills, glyphs, strokes, pictures,
//! noise, ownership maps, particles, effect groups), so every kind of content - shapes, text,
//! SVG documents, Lottie animations - gets it the same way, and the scene's content is never
//! changed. Inside a COVERAGE group (destination-in / -out and the like: masks, mattes, SVG
//! masks and several-shape clips, cut-outs) nothing is changed: those colours are coverage, not
//! paint. Not in C#.

use kurbo::{Affine, BezPath, PathEl, Shape, Stroke};
use peniko::color::Srgb;
use peniko::{Brush, Compose, Fill, ImageAlphaType, ImageData, ImageFormat};

use crate::color::Color;
use crate::drawing_session_paint::{LinePaint, SessionPaint};
use crate::rendering::VectorScene;

/// At most this many picture averages are kept (the cache is emptied when it would grow past).
const MAX_AVERAGES: usize = 512;

/// How a session's content is recorded: its paint override, whether effects are drawn, and the
/// drawing-units-to-canvas transform a fixed-width line style is expanded in.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SceneLook {
    pub(crate) paint: SessionPaint,
    pub(crate) effects: bool,
    pub(crate) drawing: Affine,
}

/// True for a compose that uses what it draws as coverage (a mask) rather than as paint.
pub(crate) fn is_coverage(compose: Compose) -> bool {
    matches!(compose, Compose::DestIn | Compose::DestOut | Compose::DestAtop | Compose::SrcIn | Compose::SrcOut | Compose::SrcAtop | Compose::Xor | Compose::Clear)
}

/// The alpha a brush carries as a whole (a solid colour's; a gradient's mean; a picture's
/// sampler alpha): a line or flat colour keeps it, so a faded shape stays faded.
fn brush_alpha(brush: &Brush) -> f32 {
    match brush {
        Brush::Solid(c) => c.components[3],
        Brush::Gradient(g) => gradient_average(g)[3],
        Brush::Image(i) => i.sampler.alpha,
    }
}

/// The average of a gradient over its length (0..1, the stops held at the ends; straight
/// interpolation between them): alpha-weighted colour, mean alpha. Sampled at 256 points.
pub(crate) fn gradient_average(g: &peniko::Gradient) -> [f32; 4] {
    let stops: Vec<(f32, [f32; 4])> = g.stops.iter().map(|s| (s.offset, s.color.to_alpha_color::<Srgb>().components)).collect();
    if stops.is_empty() {
        return [0.0; 4];
    }
    let at = |t: f32| -> [f32; 4] {
        if t <= stops[0].0 {
            return stops[0].1;
        }
        for w in stops.windows(2) {
            let ((t0, c0), (t1, c1)) = (w[0], w[1]);
            if t <= t1 {
                let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 1.0 };
                return [c0[0] + (c1[0] - c0[0]) * f, c0[1] + (c1[1] - c0[1]) * f, c0[2] + (c1[2] - c0[2]) * f, c0[3] + (c1[3] - c0[3]) * f];
            }
        }
        stops[stops.len() - 1].1
    };
    const N: usize = 256;
    let mut sum = [0.0f64; 4];
    for i in 0..N {
        let c = at((i as f32 + 0.5) / N as f32);
        let a = c[3] as f64;
        sum[0] += c[0] as f64 * a;
        sum[1] += c[1] as f64 * a;
        sum[2] += c[2] as f64 * a;
        sum[3] += a;
    }
    unpremultiply(sum, N)
}

/// Alpha-weighted sums of N samples -> straight colour and mean alpha.
fn unpremultiply(sum: [f64; 4], n: usize) -> [f32; 4] {
    if sum[3] <= 0.0 {
        return [0.0; 4];
    }
    [(sum[0] / sum[3]) as f32, (sum[1] / sum[3]) as f32, (sum[2] / sum[3]) as f32, (sum[3] / n as f64) as f32]
}

/// The average of a picture's pixels (alpha-weighted colour, mean alpha; straight or
/// premultiplied, RGBA or BGRA bytes), sRGB-encoded values 0..1.
pub(crate) fn image_average(image: &ImageData) -> [f32; 4] {
    let n = (image.width as usize) * (image.height as usize);
    let bytes = image.data.data();
    if n == 0 || bytes.len() < n * 4 {
        return [0.0; 4];
    }
    let (ri, bi) = if image.format == ImageFormat::Bgra8 { (2, 0) } else { (0, 2) };
    let premultiplied = image.alpha_type == ImageAlphaType::AlphaPremultiplied;
    let mut sum = [0u64; 4];
    for px in bytes[..n * 4].chunks_exact(4) {
        let a = px[3] as u64;
        let (r, g, b) = (px[ri] as u64, px[1] as u64, px[bi] as u64);
        if premultiplied {
            sum[0] += r * 255;
            sum[1] += g * 255;
            sum[2] += b * 255;
        } else {
            sum[0] += r * a;
            sum[1] += g * a;
            sum[2] += b * a;
        }
        sum[3] += a;
    }
    let scale = 255.0 * 255.0;
    unpremultiply([sum[0] as f64 / scale, sum[1] as f64 / scale, sum[2] as f64 / scale, sum[3] as f64 / 255.0], n)
}

fn solid(c: [f32; 4], alpha: f32) -> Brush {
    Brush::Solid(peniko::color::AlphaColor::new([c[0], c[1], c[2], (c[3] * alpha).clamp(0.0, 1.0)]))
}

impl VectorScene {
    /// Takes the look out of the scene when it changes paint here (not inside a coverage group,
    /// not "as authored"); the caller puts it back.
    pub(crate) fn take_paint_look(&mut self) -> Option<SceneLook> {
        if self.coverage_depth > 0 {
            return None;
        }
        match &self.look {
            Some(look) if look.paint != SessionPaint::AsAuthored => self.look.take(),
            _ => None,
        }
    }

    /// A brush's flat stand-in: a solid colour stays; a gradient becomes its average; a picture
    /// its pixels' average times its sampler alpha.
    pub(crate) fn representative(&mut self, brush: &Brush) -> Brush {
        match brush {
            Brush::Solid(_) => brush.clone(),
            Brush::Gradient(g) => solid(gradient_average(g), 1.0),
            Brush::Image(i) => {
                let average = self.picture_average(&i.image);
                solid(average, i.sampler.alpha)
            }
        }
    }

    /// A picture's average colour, computed once per pixel blob and kept.
    fn picture_average(&mut self, image: &ImageData) -> [f32; 4] {
        let id = image.data.id();
        if let Some(a) = self.averages.get(&id) {
            return *a;
        }
        let a = image_average(image);
        if self.averages.len() >= MAX_AVERAGES {
            self.averages.clear();
        }
        self.averages.insert(id, a);
        a
    }

    /// The line drawing's stroke of a shape (in the element's space, or fixed in the drawing's).
    fn record_line(&mut self, line: &LinePaint, drawing: Affine, transform: Affine, alpha: f32, shape: &impl Shape) {
        let brush = Brush::Solid(line.color.to_peniko(alpha));
        let stroke = line.stroke.to_kurbo();
        if line.stroke.is_fixed_width() {
            self.record_stroke_fixed(&stroke, transform, drawing, &brush, None, shape);
        } else {
            self.record_stroke(&stroke, transform, &brush, None, shape);
        }
    }

    /// A fill under a look: line - the paper colour (when given) and the outline, pictures left
    /// out; flat - one solid colour.
    pub(crate) fn fill_in_look(&mut self, look: &SceneLook, rule: Fill, transform: Affine, brush: &Brush, brush_transform: Option<Affine>, shape: &impl Shape) {
        match &look.paint {
            SessionPaint::Line(line) => {
                if matches!(brush, Brush::Image(_)) {
                    return;
                }
                let alpha = brush_alpha(brush);
                if let Some(paper) = line.fill {
                    self.record_fill(rule, transform, &Brush::Solid(paper.to_peniko(alpha)), None, shape);
                }
                self.record_line(line, look.drawing, transform, alpha, shape);
            }
            SessionPaint::Flat => {
                let brush = self.representative(brush);
                self.record_fill(rule, transform, &brush, None, shape);
            }
            SessionPaint::AsAuthored => self.record_fill(rule, transform, brush, brush_transform, shape),
        }
    }

    /// A glyph under a look: line - the paper colour and the glyph's outline (stroked in the
    /// text's space); flat - its fill's colour.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fill_cached_in_look(&mut self, look: &SceneLook, rule: Fill, element: Affine, local: Affine, brush: &Brush, brush_transform: Option<Affine>, elements: &[PathEl], key: u64) {
        match &look.paint {
            SessionPaint::Line(line) => {
                let alpha = brush_alpha(brush);
                if let Some(paper) = line.fill {
                    self.record_fill_cached(rule, element * local, &Brush::Solid(paper.to_peniko(alpha)), None, elements, key);
                }
                let outline = BezPath::from_vec(elements.iter().map(|e| local * *e).collect());
                self.record_line(line, look.drawing, element, alpha, &outline);
            }
            SessionPaint::Flat => {
                let brush = self.representative(brush);
                self.record_fill_cached(rule, element * local, &brush, None, elements, key);
            }
            SessionPaint::AsAuthored => self.record_fill_cached(rule, element * local, brush, brush_transform, elements, key),
        }
    }

    /// A stroke under a look: line - the same path in the line style; flat - as authored.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn stroke_in_look(&mut self, look: &SceneLook, style: &Stroke, transform: Affine, reference: Option<Affine>, brush: &Brush, brush_transform: Option<Affine>, shape: &impl Shape) {
        match (&look.paint, reference) {
            (SessionPaint::Line(line), _) => {
                let alpha = brush_alpha(brush);
                self.record_line(line, look.drawing, transform, alpha, shape);
            }
            (_, Some(reference)) => self.record_stroke_fixed(style, transform, reference, brush, brush_transform, shape),
            (_, None) => self.record_stroke(style, transform, brush, brush_transform, shape),
        }
    }

    /// A picture under a look: line - left out; flat - its average colour over its rectangle
    /// (times its tint and alpha).
    pub(crate) fn picture_in_look(&mut self, look: &SceneLook, transform: Affine, image: &ImageData, rect: &kurbo::Rect, tint: Option<[f32; 4]>, alpha: f32) {
        if !matches!(look.paint, SessionPaint::Flat) {
            return;
        }
        let mut c = self.picture_average(image);
        if let Some(t) = tint {
            for (v, f) in c.iter_mut().zip(t) {
                *v = (*v * f).clamp(0.0, 1.0);
            }
        }
        self.record_fill(Fill::NonZero, transform, &solid(c, alpha.clamp(0.0, 1.0)), None, rect);
    }
}

/// A colour from 0..1 components (tests and the ownership map's representative colour).
pub(crate) fn color_of(c: [f32; 4]) -> Color {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::from_rgba(b(c[0]), b(c[1]), b(c[2]), b(c[3]))
}
