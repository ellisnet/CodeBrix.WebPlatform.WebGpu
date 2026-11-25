//! `SessionWipe`: a wipe with a soft edge between ANY two renders - two sessions, or one session
//! rendered twice with different render options (a look, a blend) - as a straight edge at any
//! angle or a circle growing from a point (an iris). Not in C#: the reporting application built
//! it by hand from an isolated group per render and a gradient that cuts one away; this is that
//! technique inside the crate. The layer transitions of a session stay as they are.

use kurbo::{Affine, BezPath, Circle, Point, Rect, Shape};
use peniko::{BlendMode, Brush, ColorStop, ColorStops, Compose, Fill, Gradient, InterpolationAlphaSpace};

use crate::color::Color;
use crate::drawing_session::{DrawingSession, SessionRenderOptions};
use crate::drawing_session_blend::SessionBlendMode;
use crate::geometry::{PointF, RectangleF};
use crate::rendering::VectorScene;

/// The stops of the soft edge's ramp (a smoothstep from fully shown to cut away).
const EDGE_STOPS: usize = 9;

/// The shape of a [`SessionWipe`]'s edge. Not in C#.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WipeEdge {
    /// A straight edge travelling across the area in the direction `angle_degrees` (0 = left to
    /// right, 90 = top to bottom - canvas y points down - 180 = right to left, 270 = bottom to
    /// top; any angle). The second render is shown where the edge has passed.
    Line {
        /// The direction the edge travels, degrees.
        angle_degrees: f32,
    },
    /// A circle growing from `center` (canvas pixels; it may lie outside the area) until it
    /// covers the whole area: the second render inside it (an iris opening). For an iris
    /// closing, swap the two renders and run the progress from 1 to 0.
    Circle {
        /// The circle's centre, canvas pixels.
        center: PointF,
    },
}

/// A wipe between two renders: at `progress` 0 only the first is drawn, at 1 only the second;
/// in between the edge has travelled that fraction of the way across the area, with a soft
/// band `softness` canvas pixels wide where the two blend. Everything is a function of what is
/// passed in: the application animates `progress` (from the frame's time - the crate reads no
/// clock).
///
/// HOW IT IS DRAWN: the first render under, clipped to where it still shows (the band
/// included); the second over it in one isolated group whose content is kept destination-in by
/// the edge's gradient (the band's ramp). For opaque content the band is an exact cross-fade.
/// With `softness` 0 the edge is hard and both are only clipped (no group at all).
///
/// LIMITS: while 0 < progress < 1 BOTH renders are drawn in full (the cost of both); a soft
/// edge uses ONE level of the renderer's nesting allowance for the second render (a scene
/// already at the limit draws its deepest groups as clips, counted in
/// [`RenderStats::unsupported`](crate::rendering::RenderStats::unsupported)); a mix-mode blend
/// INSIDE the second render ([`SessionRenderOptions::blend`]) would blend inside the group - use
/// [`render_sessions`](SessionWipe::render_sessions), which composites the second session's
/// group itself with that blend and opacity (two levels then: the wipe's and the blend's are one
/// group). Both renders are clipped to the area.
///
/// Not in C#.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SessionWipe {
    /// How far the wipe has gone, 0..1 (clamped; NaN counts as 0).
    pub progress: f32,
    /// The shape of the edge.
    pub edge: WipeEdge,
    /// The width of the soft band, canvas pixels (0 for a hard edge; negative counts as 0).
    pub softness: f32,
}

impl Default for SessionWipe {
    /// A left-to-right wipe at progress 0 with a 48-pixel soft band.
    fn default() -> SessionWipe {
        SessionWipe { progress: 0.0, edge: WipeEdge::Line { angle_degrees: 0.0 }, softness: 48.0 }
    }
}

/// Where the edge is, in the terms the drawing needs.
enum Edge {
    /// Along the unit direction `d`: second where t <= start, first where t >= start + width.
    Line { angle: f64, start: f64, width: f64, t_max: f64, t_min: f64, u_min: f64, u_max: f64 },
    /// Around `center`: second where r <= inner, first where r >= inner + width.
    Circle { center: Point, inner: f64, width: f64 },
}

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl SessionWipe {
    /// A wipe at `progress` with this edge and softness.
    pub fn new(progress: f32, edge: WipeEdge, softness: f32) -> SessionWipe {
        SessionWipe { progress, edge, softness }
    }

    /// The progress clamped to 0..1 (NaN as 0).
    fn fraction(&self) -> f64 {
        if self.progress.is_nan() {
            0.0
        } else {
            self.progress.clamp(0.0, 1.0) as f64
        }
    }

    fn edge_at(&self, area: Rect) -> Edge {
        let p = self.fraction();
        let w = if self.softness.is_finite() { self.softness.max(0.0) as f64 } else { 0.0 };
        let corners = [Point::new(area.x0, area.y0), Point::new(area.x1, area.y0), Point::new(area.x1, area.y1), Point::new(area.x0, area.y1)];
        match self.edge {
            WipeEdge::Line { angle_degrees } => {
                let angle = if angle_degrees.is_finite() { (angle_degrees as f64).to_radians() } else { 0.0 };
                let (sin, cos) = angle.sin_cos();
                let t: Vec<f64> = corners.iter().map(|c| c.x * cos + c.y * sin).collect();
                let u: Vec<f64> = corners.iter().map(|c| -c.x * sin + c.y * cos).collect();
                let min = |v: &[f64]| v.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let (t_min, t_max) = (min(&t), max(&t));
                // From "the band wholly before the area" to "the band wholly past it".
                let start = t_min - w + p * (t_max - t_min + w);
                Edge::Line { angle, start, width: w, t_max, t_min, u_min: min(&u), u_max: max(&u) }
            }
            WipeEdge::Circle { center } => {
                let c = Point::new(center.x as f64, center.y as f64);
                let far = corners.iter().map(|k| (*k - c).hypot()).fold(0.0, f64::max);
                Edge::Circle { center: c, inner: -w + p * (far + w), width: w }
            }
        }
    }

    /// Draws the wipe between two renders into `area` (canvas pixels): `first` and `second`
    /// record their content into the scene (any drawing at all: a session's `render_with`, two
    /// sessions, one session twice with different looks).
    pub fn render(&self, scene: &mut VectorScene, area: RectangleF, first: impl FnOnce(&mut VectorScene), second: impl FnOnce(&mut VectorScene)) {
        self.render_with_blend(scene, area, SessionBlendMode::Normal, 1.0, first, second);
    }

    /// Draws the wipe between two sessions over the whole canvas, each with its own render
    /// options. The second session's blend mode and opacity are applied to the wipe's group of
    /// the second render (so a multiplied session blends with what is below the wipe, as it does
    /// when drawn alone).
    pub fn render_sessions(&self, scene: &mut VectorScene, width: u32, height: u32, first: (&DrawingSession, &SessionRenderOptions), second: (&DrawingSession, &SessionRenderOptions)) {
        let area = RectangleF::new(0.0, 0.0, width as f32, height as f32);
        let (b, b_options) = second;
        let soft = self.fraction() > 0.0 && self.fraction() < 1.0 && self.softness > 0.0;
        // Through the wipe's group: the blend and opacity move onto it.
        let (blend, opacity, inner) = if soft {
            (b_options.blend, b_options.opacity, SessionRenderOptions { blend: SessionBlendMode::Normal, opacity: 1.0, ..b_options.clone() })
        } else {
            (SessionBlendMode::Normal, 1.0, b_options.clone())
        };
        self.render_with_blend(scene, area, blend, opacity, |scene| first.0.render_with(scene, width, height, first.1), |scene| b.render_with(scene, width, height, &inner));
    }

    fn render_with_blend(&self, scene: &mut VectorScene, area: RectangleF, blend: SessionBlendMode, opacity: f32, first: impl FnOnce(&mut VectorScene), second: impl FnOnce(&mut VectorScene)) {
        let area = Rect::new(area.x as f64, area.y as f64, area.right() as f64, area.bottom() as f64);
        let p = self.fraction();
        scene.push_clip_layer(Affine::IDENTITY, &area);
        if p <= 0.0 {
            first(scene);
            scene.pop_layer();
            return;
        }
        if p >= 1.0 {
            second(scene);
            scene.pop_layer();
            return;
        }
        let edge = self.edge_at(area);
        // The first render where it still shows: past the inner side of the band.
        match &edge {
            Edge::Line { angle, start, t_max, u_min, u_max, .. } => {
                scene.push_clip_layer(Affine::rotate(*angle), &Rect::new(*start, u_min - 1.0, t_max + 1.0, u_max + 1.0));
            }
            Edge::Circle { center, inner, .. } if *inner > 0.0 => {
                let mut hole = BezPath::new();
                hole.extend(area.path_elements(0.1));
                hole.extend(Circle::new(*center, *inner).path_elements(0.1));
                scene.push_clip_layer_with_rule(Fill::EvenOdd, Affine::IDENTITY, &hole);
            }
            Edge::Circle { .. } => scene.push_clip_layer(Affine::IDENTITY, &area),
        }
        first(scene);
        scene.pop_layer();
        let width = match &edge {
            Edge::Line { width, .. } | Edge::Circle { width, .. } => *width,
        };
        if width <= 0.0 {
            // A hard edge: the second render clipped to where the edge has passed.
            match &edge {
                Edge::Line { angle, start, t_min, u_min, u_max, .. } => scene.push_clip_layer(Affine::rotate(*angle), &Rect::new(t_min - 1.0, u_min - 1.0, *start, u_max + 1.0)),
                Edge::Circle { center, inner, .. } => scene.push_clip_layer(Affine::IDENTITY, &Circle::new(*center, inner.max(0.0))),
            }
            second(scene);
            scene.pop_layer();
            scene.pop_layer();
            return;
        }
        // The second render in one group, kept destination-in by the band's ramp.
        let alpha = if opacity.is_nan() { 0.0 } else { opacity.clamp(0.0, 1.0) };
        scene.push_layer(blend.to_peniko(), alpha, Affine::IDENTITY, &area);
        second(scene);
        scene.push_layer(BlendMode::new(peniko::Mix::Normal, Compose::DestIn), 1.0, Affine::IDENTITY, &area);
        scene.fill(Fill::NonZero, Affine::IDENTITY, &Brush::Gradient(edge_gradient(&edge)), None, &area);
        scene.pop_layer();
        scene.pop_layer();
        scene.pop_layer();
    }
}

/// The band's ramp: white, fully opaque where the second render shows, smoothly to transparent
/// where it is cut away (held at both ends).
fn edge_gradient(edge: &Edge) -> Gradient {
    let white = |a: f64| Color::WHITE.to_peniko(a as f32);
    let mut stops = ColorStops::new();
    let gradient = match edge {
        Edge::Line { angle, start, width, .. } => {
            let (sin, cos) = angle.sin_cos();
            for k in 0..EDGE_STOPS {
                let f = k as f64 / (EDGE_STOPS - 1) as f64;
                stops.0.push(ColorStop::from((f as f32, white(1.0 - smoothstep(f)))));
            }
            Gradient::new_linear((start * cos, start * sin), ((start + width) * cos, (start + width) * sin))
        }
        Edge::Circle { center, inner, width } => {
            let outer = (inner + width).max(1e-6);
            let first = inner.max(0.0);
            for k in 0..EDGE_STOPS {
                let r = first + (outer - first) * k as f64 / (EDGE_STOPS - 1) as f64;
                let s = ((inner + width - r) / width).clamp(0.0, 1.0);
                stops.0.push(ColorStop::from(((r / outer) as f32, white(smoothstep(s)))));
            }
            Gradient::new_radial(*center, outer as f32)
        }
    };
    Gradient { stops, interpolation_alpha_space: InterpolationAlphaSpace::Unpremultiplied, ..gradient }
}
