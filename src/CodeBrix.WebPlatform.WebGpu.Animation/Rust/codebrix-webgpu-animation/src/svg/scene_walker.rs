//! Crate-internal: walks an [`SvgScene`] for one time with the document's own animation
//! overrides ([`SvgAnimationFrame`]) and the program-driven parts ([`DrawingSvgPart`]) into an
//! [`SvgDrawList`]. The scene is never changed.

use std::sync::Arc;

use kurbo::{Affine, BezPath, Rect, Shape};

use crate::color::Color;
use crate::geometry::PointF;
use crate::paths::{PathFillRule, PathGeometry};
use crate::svg::animation::svg_value::AnimAttr;
use crate::svg::drawing_svg_part::DrawingSvgPart;
use crate::svg::svg_draw_command::{SvgClipShape, SvgDrawCommand, SvgDrawList};
use crate::svg::svg_part_override::SvgAnimationFrame;
use crate::svg::svg_group_node::*;
use crate::svg::svg_leaf_nodes::*;
use crate::svg::svg_paint::*;
use crate::svg::svg_scene::*;
use crate::svg::svg_stroke::*;

/// Style overrides that flow down a subtree: an animated colour (or another inherited property)
/// on a group reaches the paths inside it that do not set that property themselves.
#[derive(Clone, Default)]
struct Inherited {
    fill: Option<Option<Color>>,
    fill_opacity: Option<f32>,
    stroke: Option<Option<Color>>,
    stroke_opacity: Option<f32>,
    stroke_width: Option<f32>,
    dash_offset: Option<f32>,
    dash_array: Option<Option<Vec<f32>>>,
    visibility: Option<bool>,
    trim: Option<(f32, f32)>,
}

impl Inherited {
    /// An element that sets `attr` itself: an animated value from an ancestor stops here.
    fn stop(&mut self, attr: AnimAttr) {
        match attr {
            AnimAttr::Fill => self.fill = None,
            AnimAttr::FillOpacity => self.fill_opacity = None,
            AnimAttr::Stroke => self.stroke = None,
            AnimAttr::StrokeOpacity => self.stroke_opacity = None,
            AnimAttr::StrokeWidth => self.stroke_width = None,
            AnimAttr::StrokeDashoffset => self.dash_offset = None,
            AnimAttr::StrokeDasharray => self.dash_array = None,
            AnimAttr::Visibility => self.visibility = None,
            _ => {}
        }
    }
}

pub(crate) struct Walker<'a> {
    scene: &'a SvgScene,
    frame: Option<&'a SvgAnimationFrame>,
    parts: &'a [DrawingSvgPart],
    out: SvgDrawList,
    active: Vec<usize>,
}

impl<'a> Walker<'a> {
    pub(crate) fn walk(scene: &'a SvgScene, frame: Option<&'a SvgAnimationFrame>, parts: &'a [DrawingSvgPart], root: Affine) -> SvgDrawList {
        let mut w = Walker { scene, frame, parts, out: SvgDrawList { commands: Vec::new(), part_bounds: vec![None; scene.parts.len()] }, active: Vec::new() };
        w.node(scene.root, Affine::IDENTITY, root, &Inherited::default(), false);
        w.out
    }

    fn node(&mut self, index: usize, parent_abs: Affine, outer: Affine, inherited: &Inherited, show: bool) {
        let scene = self.scene;
        let node = &scene.nodes[index];
        let mut local = node.transform;
        let mut outer = outer;
        let mut style = inherited.clone();
        if let Some(own) = scene.own_style.get(&index) {
            for a in own {
                style.stop(*a);
            }
        }
        let mut alpha_replace: Option<f32> = None;
        let mut alpha_multiply = 1.0f32;
        let base_active = self.active.len();
        for &pi in &scene.owner_of[index] {
            let pi = pi as usize;
            let info = &scene.parts[pi];
            if let Some(f) = self.frame.and_then(|f| f.get(&info.id)) {
                if f.visible == Some(false) {
                    self.active.truncate(base_active);
                    return;
                }
                if f.visibility.is_some() {
                    style.visibility = f.visibility;
                }
                if f.transform.is_some() || f.motion.is_some() {
                    // Replace the element's own transform attribute, keep what usvg added beside it
                    // (a <use>'s x / y), and put the motion outside: motion x transform x extra.
                    let extra = if info.transform_attribute.determinant().abs() > 1e-12 { info.transform_attribute.inverse() * local } else { local };
                    let own = f.transform.unwrap_or(info.transform_attribute);
                    local = f.motion.unwrap_or(Affine::IDENTITY) * own * extra;
                }
                if let Some(o) = f.opacity {
                    alpha_replace = Some(o.clamp(0.0, 1.0));
                }
                if f.fill.is_some() {
                    style.fill = f.fill;
                }
                if f.fill_opacity.is_some() {
                    style.fill_opacity = f.fill_opacity;
                }
                if f.stroke.is_some() {
                    style.stroke = f.stroke;
                }
                if f.stroke_opacity.is_some() {
                    style.stroke_opacity = f.stroke_opacity;
                }
                if f.stroke_width.is_some() {
                    style.stroke_width = f.stroke_width;
                }
                if f.stroke_dash_offset.is_some() {
                    style.dash_offset = f.stroke_dash_offset;
                }
                if f.stroke_dash_array.is_some() {
                    style.dash_array = f.stroke_dash_array.clone();
                }
            }
            if let Some(p) = self.parts.get(pi) {
                if !p.is_visible {
                    self.active.truncate(base_active);
                    return;
                }
                alpha_multiply *= p.opacity.clamp(0.0, 1.0);
                if p.translation != PointF::new(0.0, 0.0) || p.rotation != 0.0 || p.scale_x != 1.0 || p.scale_y != 1.0 {
                    let c = node.document_bounds.map(|r| PointF::new(r.center().x as f32, r.center().y as f32)).unwrap_or(p.center());
                    outer = outer * p.matrix_for(c);
                }
                if p.is_trimmed() {
                    style.trim = Some((p.trim_start, p.trim_end));
                }
            }
            self.active.push(pi);
        }
        let abs = parent_abs * local;
        let draw = outer * abs;
        match &node.kind {
            SvgNodeKind::Group(g) => {
                let alpha = alpha_replace.unwrap_or(g.opacity) * alpha_multiply;
                if alpha > 0.0 {
                    self.group(g, abs, outer, draw, alpha, &style);
                }
            }
            SvgNodeKind::Path(p) => {
                // A path that owns its part carries the element's opacity itself (no group).
                let alpha = alpha_replace.unwrap_or(1.0) * alpha_multiply;
                if alpha > 0.0 && style.visibility.unwrap_or(p.visible || show) {
                    if alpha < 1.0 {
                        self.out.commands.push(SvgDrawCommand::PushGroup { alpha, blend: SvgBlendMode::Normal, isolate: false, blur_std_deviation: None, transform: draw });
                    }
                    self.path(p, draw, &style);
                    if alpha < 1.0 {
                        self.out.commands.push(SvgDrawCommand::Pop);
                    }
                }
            }
            SvgNodeKind::Image(img) => {
                let alpha = alpha_replace.unwrap_or(1.0) * alpha_multiply;
                if alpha > 0.0 && style.visibility.unwrap_or(img.visible || show) {
                    if alpha < 1.0 {
                        self.out.commands.push(SvgDrawCommand::PushGroup { alpha, blend: SvgBlendMode::Normal, isolate: false, blur_std_deviation: None, transform: draw });
                    }
                    self.out.commands.push(SvgDrawCommand::Image { source: img.source.clone(), size: img.size, transform: draw, smooth: img.smooth });
                    self.bound(draw.transform_rect_bbox(Rect::new(0.0, 0.0, img.size.0, img.size.1)));
                    if alpha < 1.0 {
                        self.out.commands.push(SvgDrawCommand::Pop);
                    }
                }
            }
        }
        self.active.truncate(base_active);
    }

    fn group(&mut self, g: &SvgGroupNode, abs: Affine, outer: Affine, draw: Affine, alpha: f32, style: &Inherited) {
        let mut clips = 0;
        let mut clip = g.clip.as_ref();
        while let Some(c) = clip {
            let mut shapes = Vec::new();
            self.clip_shapes(c.content, draw * c.transform, &mut shapes);
            self.out.commands.push(SvgDrawCommand::PushClip { shapes });
            clips += 1;
            clip = c.clip.as_deref();
        }
        let grouped = alpha < 1.0 || g.blend != SvgBlendMode::Normal || g.isolate || g.mask.is_some() || g.blur_std_deviation.is_some();
        if grouped {
            self.out.commands.push(SvgDrawCommand::PushGroup { alpha, blend: g.blend, isolate: g.isolate || g.mask.is_some() || g.blur_std_deviation.is_some(), blur_std_deviation: g.blur_std_deviation, transform: draw });
        }
        for &c in &g.children {
            self.node(c, abs, outer, style, false);
        }
        let mut mask = g.mask.as_ref();
        let mut masks = 0;
        while let Some(m) = mask {
            // A mask on a mask: each level masks what is drawn before it in the same group.
            if masks > 0 {
                self.out.commands.push(SvgDrawCommand::PushGroup { alpha: 1.0, blend: SvgBlendMode::Normal, isolate: true, blur_std_deviation: None, transform: draw });
            }
            self.out.commands.push(SvgDrawCommand::MaskBegin { kind: m.kind, rect: m.rect, transform: draw });
            let saved = std::mem::take(&mut self.out.part_bounds);
            self.out.part_bounds = vec![None; saved.len()];
            let active = std::mem::take(&mut self.active);
            self.node(m.content, abs, outer, &Inherited::default(), false);
            self.active = active;
            self.out.part_bounds = saved;
            self.out.commands.push(SvgDrawCommand::MaskEnd);
            masks += 1;
            mask = m.mask.as_deref();
        }
        for _ in 1..masks {
            self.out.commands.push(SvgDrawCommand::Pop);
        }
        if grouped {
            self.out.commands.push(SvgDrawCommand::Pop);
        }
        for _ in 0..clips {
            self.out.commands.push(SvgDrawCommand::Pop);
        }
    }

    fn clip_shapes(&self, index: usize, transform: Affine, out: &mut Vec<SvgClipShape>) {
        let node = &self.scene.nodes[index];
        let t = transform * node.transform;
        match &node.kind {
            SvgNodeKind::Group(g) => {
                for &c in &g.children {
                    self.clip_shapes(c, t, out);
                }
            }
            SvgNodeKind::Path(p) => {
                if p.visible {
                    out.push(SvgClipShape { path: p.path.clone(), transform: t, rule: p.fill.as_ref().map(|f| f.rule).unwrap_or_default() });
                }
            }
            SvgNodeKind::Image(_) => {}
        }
    }

    fn bound(&mut self, r: Rect) {
        for &pi in &self.active {
            let b = &mut self.out.part_bounds[pi];
            *b = Some(match b {
                Some(old) => old.union(r),
                None => r,
            });
        }
    }

    fn path(&mut self, p: &SvgPathNode, draw: Affine, style: &Inherited) {
        let fill = match style.fill {
            Some(None) => None,
            Some(Some(c)) => Some(SvgFill { paint: SvgPaint::Solid(c), opacity: style.fill_opacity.or(p.fill.as_ref().map(|f| f.opacity)).unwrap_or(1.0), rule: p.fill.as_ref().map(|f| f.rule).unwrap_or_default() }),
            None => p.fill.clone().map(|mut f| {
                if let Some(o) = style.fill_opacity {
                    f.opacity = o;
                }
                f
            }),
        };
        let base_stroke = || SvgStroke { paint: SvgPaint::Solid(Color::BLACK), opacity: 1.0, width: 1.0, cap: SvgLineCap::Butt, join: SvgLineJoin::Miter, miter_limit: 4.0, dash_array: None, dash_offset: 0.0 };
        let stroke = match style.stroke {
            Some(None) => None,
            Some(Some(c)) => Some(SvgStroke { paint: SvgPaint::Solid(c), ..p.stroke.clone().unwrap_or_else(base_stroke) }),
            None => p.stroke.clone(),
        }
        .map(|mut s| {
            if let Some(o) = style.stroke_opacity {
                s.opacity = o;
            }
            if let Some(w) = style.stroke_width {
                s.width = w.max(0.0);
            }
            if let Some(o) = style.dash_offset {
                s.dash_offset = o;
            }
            if let Some(d) = &style.dash_array {
                s.dash_array = d.clone().filter(|d| !d.is_empty() && d.iter().all(|v| *v >= 0.0) && d.iter().any(|v| *v > 0.0)).map(|d| if d.len() % 2 == 1 { [d.clone(), d].concat() } else { d });
            }
            s
        });
        let mut drew: Option<Rect> = None;
        let mut grow = |r: Rect| drew = Some(drew.map(|d| d.union(r)).unwrap_or(r));
        let order: [bool; 2] = if p.paint_order == SvgPaintOrder::FillAndStroke { [true, false] } else { [false, true] };
        for is_fill in order {
            if is_fill {
                if let Some(f) = &fill {
                    if !matches!(f.paint, SvgPaint::Pattern(_)) {
                        self.out.commands.push(SvgDrawCommand::Fill { path: p.path.clone(), transform: draw, rule: f.rule, paint: f.paint.clone(), opacity: f.opacity, anti_alias: p.anti_alias });
                        let r = p.path.bounding_box();
                        if r.width() > 0.0 && r.height() > 0.0 {
                            grow(draw.transform_rect_bbox(r));
                        }
                    }
                }
            } else if let Some(s) = &stroke {
                if !matches!(s.paint, SvgPaint::Pattern(_)) && s.width > 0.0 {
                    let path = match style.trim {
                        Some((a, b)) => match PathGeometry::from_bez_path((*p.path).clone(), PathFillRule::NonZero).trim(a, b, 0.0) {
                            Ok(t) => Arc::new(t.bez_path().clone()),
                            Err(_) => p.path.clone(),
                        },
                        None => p.path.clone(),
                    };
                    if !path_is_empty(&path) {
                        grow(draw.transform_rect_bbox(path.bounding_box()));
                        self.out.commands.push(SvgDrawCommand::Stroke { path, transform: draw, stroke: s.clone(), anti_alias: p.anti_alias });
                    }
                }
            }
        }
        if let Some(r) = drew {
            self.bound(r);
        }
    }
}

fn path_is_empty(p: &BezPath) -> bool {
    p.elements().len() < 2
}
