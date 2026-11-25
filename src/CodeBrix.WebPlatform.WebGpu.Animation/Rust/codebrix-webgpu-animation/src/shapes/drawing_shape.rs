//! `DrawingShape`: any shape a layer holds, with the element properties every drawing element
//! has - identity, transform, opacity, time window, effects (C# `DrawingShape : DrawingElement`).

use kurbo::{Affine, Cap, Circle, Ellipse, Join, Line, Rect, RoundedRect, Stroke};
use peniko::{BlendMode, Brush, Compose, Fill};

use crate::color::Color;
use crate::effects::effect_sizes::EffectParams;
use crate::effects::{DrawingEffects, DrawingMask, DrawingMaskKind};
use crate::geometry::RectangleF;
use crate::lottie::DrawingLottieShape;
use crate::models::{DrawingLayer, DrawingTransform, ElementId};
use crate::paths::PathFillRule;
use crate::rendering::VectorScene;
use crate::shapes::animated_shape::AnimatedShape;
use crate::shapes::arrow_shape::ArrowShape;
use crate::shapes::basic_shapes::{CircleShape, EllipseShape, LineShape, PolylineShape, RectangleShape};
use crate::shapes::brush_stroke_shape::BrushStrokeShape;
use crate::shapes::brush_strokes_shape::BrushStrokesShape;
use crate::shapes::noise_shape::NoiseShape;
use crate::shapes::ownership_shape::OwnershipShape;
use crate::shapes::particles_shape::ParticlesShape;
use crate::shapes::drawing_group::DrawingGroup;
use crate::shapes::image_shape::ImageShape;
use crate::shapes::path_shape::PathShape;
use crate::shapes::sprite_shape::SpriteShape;
use crate::shapes::text_block_shape::TextBlockShape;
use crate::shapes::text_shape::TextShape;
use crate::svg::DrawingSvgShape;

/// The kind of a [`DrawingShape`] and its own data.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeKind {
    /// A rectangle.
    Rectangle(RectangleShape),
    /// A circle.
    Circle(CircleShape),
    /// An ellipse.
    Ellipse(EllipseShape),
    /// A line.
    Line(LineShape),
    /// An arrow.
    Arrow(ArrowShape),
    /// A polyline or polygon.
    Polyline(PolylineShape),
    /// A path.
    Path(PathShape),
    /// A single line of text.
    Text(TextShape),
    /// A group of elements.
    Group(DrawingGroup),
    /// An element rebuilt for each moment.
    Animated(AnimatedShape),
    /// A picture.
    Image(ImageShape),
    /// A sprite (a frame of a sprite sheet chosen by the time).
    Sprite(SpriteShape),
    /// A laid-out block of text.
    TextBlock(TextBlockShape),
    /// A painterly brushstroke.
    BrushStroke(BrushStrokeShape),
    /// A set of brushstrokes drawn as one instanced GPU draw (no C# equivalent).
    BrushStrokes(BrushStrokesShape),
    /// A rectangle of noise drawn on the GPU.
    Noise(NoiseShape),
    /// The particles of a particle system at one time.
    Particles(ParticlesShape),
    /// An ownership map coloured on the GPU.
    Ownership(OwnershipShape),
    /// An SVG document with its parts.
    Svg(DrawingSvgShape),
    /// A Lottie animation (its frame chosen by the time).
    Lottie(DrawingLottieShape),
}

/// What encoding a shape needs besides the scene: the session time (time windows, sprites), the
/// canvas region a faded group is composited over, the scene's turn (the camera's roll, radians:
/// shadows fixed in the scene follow it), and the session's layers (the mattes of layer masks).
pub(crate) struct EncodeContext<'a> {
    pub(crate) time: f64,
    pub(crate) area: Rect,
    pub(crate) scene_rotation: f64,
    pub(crate) layers: &'a [DrawingLayer],
    /// The transform from the drawing's own (calibrated) units to canvas pixels as this layer
    /// sees them - the calibration and the camera, without the layer's or any element's
    /// transform: the space a FIXED-width stroke ([`StrokeWidthMode::Fixed`](crate::paths::StrokeWidthMode))
    /// is expanded in.
    pub(crate) drawing: Affine,
    /// Where problems found while drawing are reported (once each).
    pub(crate) reports: &'a std::cell::RefCell<crate::drawing_session::SessionReports>,
}

/// Records content under a mask, in the masked content's coordinates (`transform`). `in_group`:
/// the content is already the only thing in an offscreen group of its own (an effects group, a
/// faded group, a session layer's group), which the mask may then compose into directly.
pub(crate) fn encode_masked(scene: &mut VectorScene, mask: Option<&DrawingMask>, transform: Affine, ctx: &EncodeContext<'_>, in_group: bool, content: &mut dyn FnMut(&mut VectorScene)) {
    let Some(mask) = mask else {
        content(scene);
        return;
    };
    let white = Brush::Solid(Color::WHITE.to_peniko(1.0));
    match (mask.kind(), mask.geometry()) {
        (DrawingMaskKind::Inside, Some(g)) => {
            // Inside a path: a stencil clip (anti-aliased by the multisampling).
            let rule = if g.fill_rule() == PathFillRule::EvenOdd { Fill::EvenOdd } else { Fill::NonZero };
            scene.push_clip_layer_with_rule(rule, transform, g.bez_path());
            content(scene);
            scene.pop_layer();
        }
        (DrawingMaskKind::Outside, Some(g)) => {
            // Outside a path: the content, then the path composited destination-out.
            if !in_group {
                scene.push_layer(BlendMode::default(), 1.0, Affine::IDENTITY, &ctx.area);
            }
            content(scene);
            let rule = if g.fill_rule() == PathFillRule::EvenOdd { Fill::EvenOdd } else { Fill::NonZero };
            scene.push_layer(Compose::DestOut, 1.0, Affine::IDENTITY, &ctx.area);
            scene.fill(rule, transform, &white, None, g.bez_path());
            scene.pop_layer();
            if !in_group {
                scene.pop_layer();
            }
        }
        (DrawingMaskKind::Layer, _) => {
            // A matte: the content, then the matte layer's elements composited destination-in
            // (destination-out when inverted) - their coverage, alpha included.
            if !in_group {
                scene.push_layer(BlendMode::default(), 1.0, Affine::IDENTITY, &ctx.area);
            }
            content(scene);
            let compose = if mask.is_inverted() { Compose::DestOut } else { Compose::DestIn };
            scene.push_layer(compose, 1.0, Affine::IDENTITY, &ctx.area);
            let name = mask.layer().unwrap_or("");
            match ctx.layers.iter().find(|l| l.name() == name) {
                Some(matte) => {
                    for element in matte.get_elements() {
                        if element.is_visible_at(ctx.time) {
                            // A matte's own masks are not followed (a matte of a matte could loop).
                            element.encode_with(scene, transform, matte.color(), ctx, false);
                        }
                    }
                }
                // An unknown name covers nothing - and is reported, once.
                None => ctx.reports.borrow_mut().report(format!("A layer matte names the layer \"{name}\", which the session does not have: add a layer of that name (it may be hidden) or correct the name in DrawingMask::from_layer.")),
            }
            scene.pop_layer();
            if !in_group {
                scene.pop_layer();
            }
        }
        (DrawingMaskKind::Plane, _) => {
            // A plane mask: the content, then the plane's picture (white, alpha = the plane's
            // values) stretched over its rectangle and composited destination-in (destination-out
            // when inverted): 0 outside the rectangle, 1 there when inverted.
            if !in_group {
                scene.push_layer(BlendMode::default(), 1.0, Affine::IDENTITY, &ctx.area);
            }
            content(scene);
            let compose = if mask.is_inverted() { Compose::DestOut } else { Compose::DestIn };
            scene.push_layer(compose, 1.0, Affine::IDENTITY, &ctx.area);
            if let Some((picture, (w, h))) = mask.plane() {
                let d = mask.destination();
                let rect = kurbo::Rect::new(d.x as f64, d.y as f64, (d.x + d.width) as f64, (d.y + d.height) as f64);
                let place = Affine::translate((d.x as f64, d.y as f64)) * Affine::scale_non_uniform(d.width as f64 / w as f64, d.height as f64 / h as f64);
                scene.fill_picture(transform, picture.image_data(), place, &rect, false, None, 1.0);
            }
            scene.pop_layer();
            if !in_group {
                scene.pop_layer();
            }
        }
        _ => content(scene),
    }
}

/// A shape on a layer: its kind, and the element properties - its [`ElementId`] (what bindings
/// name it by), its [`DrawingTransform`], its opacity, and the time window in which it is shown
/// (C# `StartTime` / `EndTime`, in session seconds).
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingShape {
    id: ElementId,
    kind: ShapeKind,
    transform: DrawingTransform,
    opacity: f32,
    start_time: Option<f64>,
    end_time: Option<f64>,
    effects: Option<DrawingEffects>,
    is_safe_area_checked: bool,
}

macro_rules! from_kind {
    ($t:ty, $v:ident) => {
        impl From<$t> for DrawingShape {
            fn from(shape: $t) -> DrawingShape {
                DrawingShape::new(ShapeKind::$v(shape))
            }
        }
    };
}

from_kind!(RectangleShape, Rectangle);
from_kind!(CircleShape, Circle);
from_kind!(EllipseShape, Ellipse);
from_kind!(LineShape, Line);
from_kind!(ArrowShape, Arrow);
from_kind!(PolylineShape, Polyline);
from_kind!(PathShape, Path);
from_kind!(TextShape, Text);
from_kind!(DrawingGroup, Group);
from_kind!(AnimatedShape, Animated);
from_kind!(ImageShape, Image);
from_kind!(SpriteShape, Sprite);
from_kind!(TextBlockShape, TextBlock);
from_kind!(BrushStrokeShape, BrushStroke);
from_kind!(BrushStrokesShape, BrushStrokes);
from_kind!(NoiseShape, Noise);
from_kind!(ParticlesShape, Particles);
from_kind!(OwnershipShape, Ownership);

fn union(a: RectangleF, b: RectangleF) -> RectangleF {
    let (l, t) = (a.x.min(b.x), a.y.min(b.y));
    let (r, bt) = (a.right().max(b.right()), a.bottom().max(b.bottom()));
    RectangleF::new(l, t, r - l, bt - t)
}

fn grow(r: RectangleF, half: f32) -> RectangleF {
    RectangleF::new(r.x - half, r.y - half, r.width + 2.0 * half, r.height + 2.0 * half)
}

impl DrawingShape {
    /// A shape of this kind with a new id: untransformed, fully opaque, always shown.
    pub fn new(kind: ShapeKind) -> DrawingShape {
        DrawingShape { id: ElementId::next(), kind, transform: DrawingTransform::IDENTITY, opacity: 1.0, start_time: None, end_time: None, effects: None, is_safe_area_checked: false }
    }

    /// The element's identity (what bindings name it by; kept by clones).
    pub fn id(&self) -> ElementId {
        self.id
    }

    /// The kind and its data.
    pub fn kind(&self) -> &ShapeKind {
        &self.kind
    }

    /// The kind and its data, to change them.
    pub fn kind_mut(&mut self) -> &mut ShapeKind {
        &mut self.kind
    }

    /// The group, when this element is one.
    pub fn as_group(&self) -> Option<&DrawingGroup> {
        match &self.kind {
            ShapeKind::Group(g) => Some(g),
            _ => None,
        }
    }

    /// The group, to change it, when this element is one.
    pub fn as_group_mut(&mut self) -> Option<&mut DrawingGroup> {
        match &mut self.kind {
            ShapeKind::Group(g) => Some(g),
            _ => None,
        }
    }

    /// The path shape, when this element is one.
    pub fn as_path(&self) -> Option<&PathShape> {
        match &self.kind {
            ShapeKind::Path(p) => Some(p),
            _ => None,
        }
    }

    /// The path shape, to change it, when this element is one.
    pub fn as_path_mut(&mut self) -> Option<&mut PathShape> {
        match &mut self.kind {
            ShapeKind::Path(p) => Some(p),
            _ => None,
        }
    }

    /// The text block, when this element is one.
    pub fn as_text_block(&self) -> Option<&TextBlockShape> {
        match &self.kind {
            ShapeKind::TextBlock(t) => Some(t),
            _ => None,
        }
    }

    /// The text block, to change it, when this element is one.
    pub fn as_text_block_mut(&mut self) -> Option<&mut TextBlockShape> {
        match &mut self.kind {
            ShapeKind::TextBlock(t) => Some(t),
            _ => None,
        }
    }

    /// The image shape, when this element is one.
    pub fn as_image_mut(&mut self) -> Option<&mut ImageShape> {
        match &mut self.kind {
            ShapeKind::Image(i) => Some(i),
            _ => None,
        }
    }

    /// The sprite, when this element is one.
    pub fn as_sprite(&self) -> Option<&SpriteShape> {
        match &self.kind {
            ShapeKind::Sprite(s) => Some(s),
            _ => None,
        }
    }

    /// The sprite, to change it, when this element is one.
    pub fn as_sprite_mut(&mut self) -> Option<&mut SpriteShape> {
        match &mut self.kind {
            ShapeKind::Sprite(s) => Some(s),
            _ => None,
        }
    }

    /// The element's effects - blur, glow, drop shadow (C# `Effects`; None by default).
    pub fn effects(&self) -> Option<DrawingEffects> {
        self.effects
    }

    /// Sets the effects (None, or empty effects, for none). Bindable through
    /// `AnimatedProperty::BlurRadius`, `ShadowOffset`, `ShadowBlurRadius`, `ShadowColor`,
    /// `GlowRadius` and `GlowColor`.
    pub fn set_effects(&mut self, effects: Option<DrawingEffects>) {
        self.effects = effects;
    }

    /// The same shape with effects.
    pub fn with_effects(mut self, effects: DrawingEffects) -> DrawingShape {
        self.effects = Some(effects);
        self
    }

    /// The element's transform (C# `Transform`; identity by default).
    pub fn transform(&self) -> DrawingTransform {
        self.transform
    }

    /// Sets the transform.
    pub fn set_transform(&mut self, transform: DrawingTransform) {
        self.transform = transform;
    }

    /// The same shape with a transform.
    pub fn with_transform(mut self, transform: DrawingTransform) -> DrawingShape {
        self.transform = transform;
        self
    }

    /// The element's opacity, 0..1 (C# `Opacity`). Below 1 a group (or an animated shape) is
    /// drawn whole and then faded; a single shape's colour is faded.
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Sets the opacity (clamped to 0..1; NaN counts as 0).
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = if opacity.is_nan() { 0.0 } else { opacity.clamp(0.0, 1.0) };
    }

    /// The same shape with an opacity.
    pub fn with_opacity(mut self, opacity: f32) -> DrawingShape {
        self.set_opacity(opacity);
        self
    }

    /// The session time from which it is shown (None: from the start).
    pub fn start_time(&self) -> Option<f64> {
        self.start_time
    }

    /// The session time from which it is no longer shown (None: never hidden).
    pub fn end_time(&self) -> Option<f64> {
        self.end_time
    }

    /// Sets the time window (C# `StartTime` / `EndTime`).
    pub fn set_time_window(&mut self, start_time: Option<f64>, end_time: Option<f64>) {
        self.start_time = start_time;
        self.end_time = end_time;
    }

    /// The same shape with a time window.
    pub fn with_time_window(mut self, start_time: Option<f64>, end_time: Option<f64>) -> DrawingShape {
        self.set_time_window(start_time, end_time);
        self
    }

    /// True when [`DrawingSession::check_safe_area`](crate::DrawingSession::check_safe_area) checks
    /// this element (C# `IsSafeAreaChecked`; false by default - or set its layer's flag).
    pub fn is_safe_area_checked(&self) -> bool {
        self.is_safe_area_checked
    }

    /// Sets whether the element is checked against safe areas.
    pub fn set_is_safe_area_checked(&mut self, value: bool) {
        self.is_safe_area_checked = value;
    }

    /// The same shape, checked (or not) against safe areas.
    pub fn with_is_safe_area_checked(mut self, value: bool) -> DrawingShape {
        self.is_safe_area_checked = value;
        self
    }

    /// True when the shape is shown at a session time: at or after its start, before its end (C#
    /// `IsVisibleAt`).
    pub fn is_visible_at(&self, seconds: f64) -> bool {
        self.start_time.map_or(true, |s| seconds >= s) && self.end_time.map_or(true, |e| seconds < e)
    }

    /// The element's bounds in its own coordinates, before its transform (C# `GetBounds`): the
    /// outline grown by half the stroke where it is stroked. None where they are not known.
    pub fn get_bounds(&self) -> Option<RectangleF> {
        self.local_bounds(None)
    }

    /// The bounds after the element's transform (C# `GetTransformedBounds`).
    pub fn get_transformed_bounds(&self) -> Option<RectangleF> {
        self.get_transformed_bounds_at(None)
    }

    pub(crate) fn get_transformed_bounds_at(&self, seconds: Option<f64>) -> Option<RectangleF> {
        let local = self.local_bounds(seconds)?;
        Some(if self.transform.is_identity() { local } else { self.transform.transform_bounds(local) })
    }

    fn local_bounds(&self, seconds: Option<f64>) -> Option<RectangleF> {
        Some(match &self.kind {
            ShapeKind::Rectangle(r) => r.get_bounds(),
            ShapeKind::Circle(c) => {
                let reach = if c.is_filled { c.radius } else { c.radius + c.stroke_thickness / 2.0 };
                RectangleF::new(c.center_x - reach, c.center_y - reach, 2.0 * reach, 2.0 * reach)
            }
            ShapeKind::Ellipse(e) => {
                let g = if e.is_filled { 0.0 } else { e.stroke_thickness / 2.0 };
                RectangleF::new(e.center_x - e.radius_x - g, e.center_y - e.radius_y - g, 2.0 * e.radius_x + 2.0 * g, 2.0 * e.radius_y + 2.0 * g)
            }
            ShapeKind::Line(l) => grow(union(RectangleF::new(l.x1, l.y1, 0.0, 0.0), RectangleF::new(l.x2, l.y2, 0.0, 0.0)), l.stroke_thickness / 2.0),
            ShapeKind::Arrow(a) => a.get_bounds(),
            ShapeKind::Polyline(p) => {
                let first = p.points.first()?;
                let b = p.points.iter().fold(RectangleF::new(first.x, first.y, 0.0, 0.0), |acc, q| union(acc, RectangleF::new(q.x, q.y, 0.0, 0.0)));
                if p.is_filled {
                    b
                } else {
                    grow(b, p.stroke_thickness / 2.0)
                }
            }
            ShapeKind::Path(p) => p.get_bounds(),
            ShapeKind::Text(t) => t.get_bounds(),
            ShapeKind::Group(g) => match seconds {
                Some(s) => g.get_bounds_at(s)?,
                None => g.get_bounds()?,
            },
            ShapeKind::Animated(a) => a.get_bounds()?,
            ShapeKind::Image(i) => i.get_bounds(),
            ShapeKind::Sprite(s) => s.get_bounds(),
            ShapeKind::TextBlock(t) => t.get_bounds(),
            ShapeKind::BrushStroke(b) => b.get_bounds(),
            ShapeKind::BrushStrokes(b) => b.get_bounds(),
            ShapeKind::Noise(n) => n.get_bounds(),
            ShapeKind::Particles(p) => p.get_bounds(),
            ShapeKind::Ownership(m) => m.get_bounds(),
            ShapeKind::Svg(s) => s.get_bounds(),
            ShapeKind::Lottie(l) => l.get_bounds(),
        })
    }

    /// Calls `visit` for this element and every element inside it (groups, depth first).
    pub(crate) fn visit_mut(&mut self, visit: &mut dyn FnMut(&mut DrawingShape)) {
        visit(self);
        if let ShapeKind::Group(g) = &mut self.kind {
            for child in &mut g.children {
                child.visit_mut(visit);
            }
        }
    }

    /// This element or the one inside it (groups) with the id.
    pub(crate) fn find_mut(&mut self, id: ElementId) -> Option<&mut DrawingShape> {
        if self.id == id {
            return Some(self);
        }
        if let ShapeKind::Group(g) = &mut self.kind {
            for child in &mut g.children {
                if let Some(found) = child.find_mut(id) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// This element or the one inside it (groups) with the id.
    pub(crate) fn find(&self, id: ElementId) -> Option<&DrawingShape> {
        if self.id == id {
            return Some(self);
        }
        if let ShapeKind::Group(g) = &self.kind {
            return g.children.iter().find_map(|c| c.find(id));
        }
        None
    }

    /// Records the shape into a scene: `transform` takes the element's parent coordinates to
    /// canvas pixels (the element's own transform is applied here), `inherited` is the colour of
    /// shapes without their own; the context gives the time, the region a faded group is
    /// composited over, the scene's turn and the layers (mattes).
    pub(crate) fn encode(&self, scene: &mut VectorScene, transform: Affine, inherited: Color, ctx: &EncodeContext<'_>) {
        self.encode_with(scene, transform, inherited, ctx, true);
    }

    /// `encode`, with or without the masks of groups (a matte's elements are drawn without).
    pub(crate) fn encode_with(&self, scene: &mut VectorScene, transform: Affine, inherited: Color, ctx: &EncodeContext<'_>, masks: bool) {
        let transform = if self.transform.is_identity() { transform } else { transform * self.transform.to_matrix() };
        if self.opacity <= 0.0 {
            return;
        }
        // Effects: the element drawn whole (at full opacity) into a group that goes through the
        // effects and is put down at the element's opacity (C#'s DrawThrough).
        if let Some(params) = self.effects.as_ref().and_then(|e| EffectParams::new(e, transform, ctx.scene_rotation)) {
            scene.push_effects(params, self.opacity);
            self.encode_body(scene, transform, inherited, ctx, 1.0, masks, true);
            scene.pop_layer();
            return;
        }
        self.encode_body(scene, transform, inherited, ctx, self.opacity, masks, false);
    }

    #[allow(clippy::too_many_arguments)]
    fn encode_body(&self, scene: &mut VectorScene, transform: Affine, inherited: Color, ctx: &EncodeContext<'_>, o: f32, masks: bool, in_group: bool) {
        let (time, area) = (ctx.time, &ctx.area);
        if o <= 0.0 {
            return;
        }
        let solid = |color: Option<Color>| Brush::Solid(color.unwrap_or(inherited).to_peniko(o));
        let outline = |thickness: f32| Stroke::new(thickness as f64).with_caps(Cap::Round).with_join(Join::Round);
        match &self.kind {
            ShapeKind::Rectangle(r) => {
                let rect = Rect::new(r.x as f64, r.y as f64, (r.x + r.width) as f64, (r.y + r.height) as f64);
                let brush = match (&r.fill, r.is_filled) {
                    (Some(fill), _) => fill.to_brush(o),
                    _ => solid(r.color),
                };
                if r.corner_radius > 0.0 {
                    let shape = RoundedRect::from_rect(rect, r.corner_radius as f64);
                    if r.is_filled {
                        scene.fill(Fill::NonZero, transform, &brush, None, &shape);
                    } else {
                        scene.stroke(&outline(r.stroke_thickness), transform, &brush, None, &shape);
                    }
                } else if r.is_filled {
                    scene.fill(Fill::NonZero, transform, &brush, None, &rect);
                } else {
                    scene.stroke(&outline(r.stroke_thickness), transform, &brush, None, &rect);
                }
            }
            ShapeKind::Circle(c) => {
                let shape = Circle::new((c.center_x as f64, c.center_y as f64), c.radius as f64);
                if c.is_filled {
                    let brush = c.fill.as_ref().map_or_else(|| solid(c.color), |f| f.to_brush(o));
                    scene.fill(Fill::NonZero, transform, &brush, None, &shape);
                } else {
                    scene.stroke(&outline(c.stroke_thickness), transform, &solid(c.color), None, &shape);
                }
            }
            ShapeKind::Ellipse(e) => {
                let shape = Ellipse::new((e.center_x as f64, e.center_y as f64), (e.radius_x as f64, e.radius_y as f64), 0.0);
                if e.is_filled {
                    let brush = e.fill.as_ref().map_or_else(|| solid(e.color), |f| f.to_brush(o));
                    scene.fill(Fill::NonZero, transform, &brush, None, &shape);
                } else {
                    scene.stroke(&outline(e.stroke_thickness), transform, &solid(e.color), None, &shape);
                }
            }
            ShapeKind::Line(l) => {
                let shape = Line::new((l.x1 as f64, l.y1 as f64), (l.x2 as f64, l.y2 as f64));
                scene.stroke(&outline(l.stroke_thickness), transform, &solid(l.color), None, &shape);
            }
            ShapeKind::Arrow(a) => {
                let (h1, h2) = a.head_points();
                let mut path = kurbo::BezPath::new();
                path.move_to((a.x1 as f64, a.y1 as f64));
                path.line_to((a.x2 as f64, a.y2 as f64));
                path.move_to(h1.to_kurbo());
                path.line_to((a.x2 as f64, a.y2 as f64));
                path.line_to(h2.to_kurbo());
                // C# strokes the shaft and the two head strokes as separate round-capped lines.
                scene.stroke(&outline(a.stroke_thickness), transform, &solid(a.color), None, &path);
            }
            ShapeKind::Polyline(p) => {
                let mut path = kurbo::BezPath::new();
                for (i, point) in p.points.iter().enumerate() {
                    if i == 0 {
                        path.move_to(point.to_kurbo());
                    } else {
                        path.line_to(point.to_kurbo());
                    }
                }
                if p.is_closed {
                    path.close_path();
                }
                if p.is_filled {
                    let brush = p.fill.as_ref().map_or_else(|| solid(p.color), |f| f.to_brush(o));
                    scene.fill(Fill::NonZero, transform, &brush, None, &path);
                } else {
                    scene.stroke(&outline(p.stroke_thickness), transform, &solid(p.color), None, &path);
                }
            }
            ShapeKind::Path(p) => p.encode(scene, transform, ctx.drawing, inherited, o),
            ShapeKind::BrushStroke(b) => {
                // C# draws a brushstroke as an anti-aliased filled polygon in its colour or fill;
                // below a progress of 1 only the painted part.
                let brush = b.fill.as_ref().map_or_else(|| solid(b.color), |f| f.to_brush(o));
                scene.fill(Fill::NonZero, transform, &brush, None, &b.drawn_path());
            }
            ShapeKind::BrushStrokes(b) => {
                // Strokes overlap: below full opacity the set is drawn whole into a layer that is
                // then faded (as a group), not each stroke faded.
                if o < 1.0 {
                    scene.push_layer(BlendMode::default(), o, Affine::IDENTITY, area);
                    b.encode(scene, transform, 1.0, ctx);
                    scene.pop_layer();
                } else {
                    b.encode(scene, transform, 1.0, ctx);
                }
            }
            ShapeKind::Noise(n) => {
                let a = n.area;
                scene.fill_noise(transform, n.gpu_params(), &n.ramp(o), &Rect::new(a.x as f64, a.y as f64, (a.x + a.width) as f64, (a.y + a.height) as f64));
            }
            ShapeKind::Particles(p) => scene.draw_particles(transform, &p.particles, p.softness, o),
            ShapeKind::Ownership(m) => {
                let d = m.destination;
                let (w, h) = m.map_size;
                let place = Affine::translate((d.x as f64, d.y as f64)) * Affine::scale_non_uniform(d.width as f64 / w as f64, d.height as f64 / h as f64);
                scene.fill_owners(transform, m.picture.image_data(), m.palette.as_ref().map(|p| p.image_data()), place, &Rect::new(d.x as f64, d.y as f64, (d.x + d.width) as f64, (d.y + d.height) as f64), m.gpu_params(), o, m.representative);
            }
            ShapeKind::Image(i) => i.encode(scene, transform, o),
            ShapeKind::Sprite(sp) => sp.encode(scene, transform, time, o),
            ShapeKind::Text(t) => t.encode(scene, transform, inherited, o),
            ShapeKind::TextBlock(t) => t.encode(scene, transform, inherited, o, ctx),
            ShapeKind::Svg(s) => s.encode(scene, transform, o, ctx),
            ShapeKind::Lottie(l) => l.encode(scene, transform, time, o),
            ShapeKind::Group(g) => {
                // Below full opacity the group is drawn whole into a layer that is then faded;
                // its mask (in the group's coordinates) applies to the whole content first.
                let fading = o < 1.0;
                if fading {
                    scene.push_layer(BlendMode::default(), o, Affine::IDENTITY, area);
                }
                let color = g.content_color.or(g.color).unwrap_or(inherited);
                let mask = if masks { g.mask.as_ref() } else { None };
                encode_masked(scene, mask, transform, ctx, fading || in_group, &mut |scene| {
                    for child in &g.children {
                        if child.is_visible_at(time) {
                            child.encode_with(scene, transform, color, ctx, masks);
                        }
                    }
                });
                if fading {
                    scene.pop_layer();
                }
            }
            ShapeKind::Animated(a) => {
                if let Some(current) = a.current() {
                    let fading = o < 1.0;
                    if fading {
                        scene.push_layer(BlendMode::default(), o, Affine::IDENTITY, area);
                    }
                    if current.is_visible_at(time) {
                        current.encode_with(scene, transform, inherited, ctx, masks);
                    }
                    if fading {
                        scene.pop_layer();
                    }
                }
            }
        }
    }
}
