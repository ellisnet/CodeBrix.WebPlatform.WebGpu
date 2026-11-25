//! Crate-internal: usvg's resolved tree -> [`SvgScene`] (plain data), with the report of what
//! the renderer cannot draw. Our own code on usvg's public tree API; nothing here comes from
//! another SVG renderer.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use kurbo::{Affine, BezPath, Rect};
use usvg::roxmltree;

use crate::color::Color;
use crate::paths::{PathFillRule, PathGeometry};
use crate::svg::drawing_svg_warning::{DrawingSvgWarningKind, WarningList};
use crate::svg::svg_group_node::*;
use crate::svg::svg_leaf_nodes::*;
use crate::svg::svg_paint::*;
use crate::svg::svg_scene::*;
use crate::svg::svg_stroke::*;

/// The prefix of the private marker a resolver puts in front of an external image href so that
/// usvg keeps the image node and the scene can turn it back into `SvgImageSource::Href`.
pub(crate) const HREF_MARKER: &[u8] = b"\0codebrix-svg-href\0";

pub(crate) fn affine(t: usvg::Transform) -> Affine {
    Affine::new([t.sx as f64, t.ky as f64, t.kx as f64, t.sy as f64, t.tx as f64, t.ty as f64])
}

fn rect(r: usvg::Rect) -> Rect {
    Rect::new(r.left() as f64, r.top() as f64, r.right() as f64, r.bottom() as f64)
}

pub(crate) fn bez(path: &usvg::tiny_skia_path::Path) -> BezPath {
    use usvg::tiny_skia_path::PathSegment;
    let mut b = BezPath::new();
    for seg in path.segments() {
        match seg {
            PathSegment::MoveTo(p) => b.move_to((p.x as f64, p.y as f64)),
            PathSegment::LineTo(p) => b.line_to((p.x as f64, p.y as f64)),
            PathSegment::QuadTo(a, p) => b.quad_to((a.x as f64, a.y as f64), (p.x as f64, p.y as f64)),
            PathSegment::CubicTo(a, c, p) => b.curve_to((a.x as f64, a.y as f64), (c.x as f64, c.y as f64), (p.x as f64, p.y as f64)),
            PathSegment::Close => b.close_path(),
        }
    }
    b
}

fn color(c: usvg::Color, opacity: f32) -> Color {
    Color::from_rgba(c.red, c.green, c.blue, (opacity.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn spread(s: usvg::SpreadMethod) -> SvgSpread {
    match s {
        usvg::SpreadMethod::Pad => SvgSpread::Pad,
        usvg::SpreadMethod::Reflect => SvgSpread::Reflect,
        usvg::SpreadMethod::Repeat => SvgSpread::Repeat,
    }
}

fn stops(s: &[usvg::Stop]) -> Vec<SvgGradientStop> {
    s.iter().map(|s| SvgGradientStop { offset: s.offset().get(), color: color(s.color(), s.opacity().get()) }).collect()
}

fn blend(b: usvg::BlendMode) -> SvgBlendMode {
    use usvg::BlendMode as B;
    match b {
        B::Normal => SvgBlendMode::Normal,
        B::Multiply => SvgBlendMode::Multiply,
        B::Screen => SvgBlendMode::Screen,
        B::Overlay => SvgBlendMode::Overlay,
        B::Darken => SvgBlendMode::Darken,
        B::Lighten => SvgBlendMode::Lighten,
        B::ColorDodge => SvgBlendMode::ColorDodge,
        B::ColorBurn => SvgBlendMode::ColorBurn,
        B::HardLight => SvgBlendMode::HardLight,
        B::SoftLight => SvgBlendMode::SoftLight,
        B::Difference => SvgBlendMode::Difference,
        B::Exclusion => SvgBlendMode::Exclusion,
        B::Hue => SvgBlendMode::Hue,
        B::Saturation => SvgBlendMode::Saturation,
        B::Color => SvgBlendMode::Color,
        B::Luminosity => SvgBlendMode::Luminosity,
        B::PlusLighter => SvgBlendMode::PlusLighter,
    }
}

/// Builds the arena.
pub(crate) struct SceneBuilder<'w> {
    nodes: Vec<SvgSceneNode>,
    warnings: &'w mut WarningList,
    /// Paint servers converted once, shared by identity.
    linear: HashMap<*const usvg::LinearGradient, Arc<SvgLinearGradient>>,
    radial: HashMap<*const usvg::RadialGradient, Arc<SvgRadialGradient>>,
    /// The ids of the elements with `vector-effect="non-scaling-stroke"` (usvg drops the
    /// property, so the crate reads it from the document and finds the paths by id).
    non_scaling: HashSet<String>,
}

impl<'w> SceneBuilder<'w> {
    pub(crate) fn new(warnings: &'w mut WarningList) -> SceneBuilder<'w> {
        SceneBuilder { nodes: Vec::new(), warnings, linear: HashMap::new(), radial: HashMap::new(), non_scaling: HashSet::new() }
    }

    /// The ids of the elements whose strokes do not scale (`vector-effect="non-scaling-stroke"`).
    pub(crate) fn with_non_scaling_strokes(mut self, ids: HashSet<String>) -> SceneBuilder<'w> {
        self.non_scaling = ids;
        self
    }

    /// Converts the tree; `xml` is the (prepared) document the tree was parsed from.
    /// `forced`: the ids of shape / image targets that were given isolation="isolate" so that
    /// usvg made a group of their own for them.
    pub(crate) fn build(mut self, tree: &usvg::Tree, xml: &roxmltree::Document, forced: &HashSet<String>) -> SvgScene {
        let root = self.group(tree.root(), None, true);
        let size = (tree.size().width() as f64, tree.size().height() as f64);
        let view_box = xml
            .root_element()
            .attribute("viewBox")
            .and_then(|v| v.parse::<svgtypes::ViewBox>().ok())
            .map(|v| Rect::new(v.x, v.y, v.x + v.w, v.y + v.h))
            .unwrap_or(Rect::new(0.0, 0.0, size.0, size.1));
        let nodes = self.nodes;
        let mut scene = SvgScene { owner_of: vec![Vec::new(); nodes.len()], nodes, root, parts: Vec::new(), part_index: HashMap::new(), size, view_box, own_style: HashMap::new() };
        collect_parts(&mut scene, xml, forced);
        scene
    }

    fn push(&mut self, node: SvgSceneNode) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn group(&mut self, g: &usvg::Group, parent: Option<usize>, main_tree: bool) -> usize {
        let index = self.push(SvgSceneNode {
            id: g.id().to_string(),
            transform: affine(g.transform()),
            parent,
            document_bounds: if main_tree { g.abs_bounding_box().map(rect) } else { None },
            kind: SvgNodeKind::Group(SvgGroupNode { opacity: g.opacity().get(), blend: blend(g.blend_mode()), isolate: g.isolate(), clip: None, mask: None, blur_std_deviation: None, has_undrawn_filter: false, children: Vec::new() }),
        });
        let clip = g.clip_path().map(|c| self.clip(c));
        let mask = g.mask().map(|m| self.mask(m));
        let (blur, undrawn) = self.filters(g);
        let mut children = Vec::new();
        for child in g.children() {
            if let Some(c) = self.node(child, index, main_tree) {
                children.push(c);
            }
        }
        if let SvgNodeKind::Group(n) = &mut self.nodes[index].kind {
            n.clip = clip;
            n.mask = mask;
            n.blur_std_deviation = blur;
            n.has_undrawn_filter = undrawn;
            n.children = children;
        }
        index
    }

    fn filters(&mut self, g: &usvg::Group) -> (Option<f32>, bool) {
        let filters = g.filters();
        if filters.is_empty() {
            return (None, false);
        }
        if filters.len() == 1 && filters[0].primitives().len() == 1 {
            if let usvg::filter::Kind::GaussianBlur(b) = filters[0].primitives()[0].kind() {
                let (x, y) = (b.std_dev_x().get(), b.std_dev_y().get());
                if matches!(b.input(), usvg::filter::Input::SourceGraphic) && (x - y).abs() < 1e-6 {
                    return (Some(x), false);
                }
            }
        }
        for f in filters {
            let names: Vec<&str> = f.primitives().iter().map(|p| primitive_name(p.kind())).collect();
            self.warnings.add(DrawingSvgWarningKind::UnsupportedFilterPrimitive, format!("The filter '{}' ({}) is not drawn: only a single feGaussianBlur of the source graphic with one standard deviation is. The element is drawn without it.", f.id(), names.join(", ")));
        }
        (None, true)
    }

    fn clip(&mut self, c: &usvg::ClipPath) -> SvgClip {
        let content = self.group(c.root(), None, false);
        SvgClip { id: c.id().to_string(), transform: affine(c.transform()), content, clip: c.clip_path().map(|n| Box::new(self.clip(n))) }
    }

    fn mask(&mut self, m: &usvg::Mask) -> SvgMask {
        let content = self.group(m.root(), None, false);
        let kind = match m.kind() {
            usvg::MaskType::Luminance => SvgMaskKind::Luminance,
            usvg::MaskType::Alpha => SvgMaskKind::Alpha,
        };
        if kind == SvgMaskKind::Luminance {
            self.warnings.add(DrawingSvgWarningKind::LuminanceMaskApproximated, format!("The mask '{}' is a luminance mask: it is applied by its content's alpha (the renderer has no luminance pass yet); give it mask-type=\"alpha\" to get exactly what is drawn.", m.id()));
        }
        let r = m.rect();
        SvgMask { id: m.id().to_string(), rect: Rect::new(r.left() as f64, r.top() as f64, r.right() as f64, r.bottom() as f64), kind, content, mask: m.mask().map(|n| Box::new(self.mask(n))) }
    }

    fn paint(&mut self, p: &usvg::Paint) -> SvgPaint {
        match p {
            usvg::Paint::Color(c) => SvgPaint::Solid(color(*c, 1.0)),
            usvg::Paint::LinearGradient(lg) => {
                let key = Arc::as_ptr(lg);
                if let Some(v) = self.linear.get(&key) {
                    return SvgPaint::Linear(v.clone());
                }
                let v = Arc::new(SvgLinearGradient { id: lg.id().to_string(), start: (lg.x1() as f64, lg.y1() as f64), end: (lg.x2() as f64, lg.y2() as f64), transform: affine(lg.transform()), spread: spread(lg.spread_method()), stops: stops(lg.stops()) });
                self.linear.insert(key, v.clone());
                SvgPaint::Linear(v)
            }
            usvg::Paint::RadialGradient(rg) => {
                let key = Arc::as_ptr(rg);
                if let Some(v) = self.radial.get(&key) {
                    return SvgPaint::Radial(v.clone());
                }
                let v = Arc::new(SvgRadialGradient { id: rg.id().to_string(), center: (rg.cx() as f64, rg.cy() as f64), radius: rg.r().get() as f64, focal: (rg.fx() as f64, rg.fy() as f64), focal_radius: rg.fr().get() as f64, transform: affine(rg.transform()), spread: spread(rg.spread_method()), stops: stops(rg.stops()) });
                self.radial.insert(key, v.clone());
                SvgPaint::Radial(v)
            }
            usvg::Paint::Pattern(pt) => {
                self.warnings.add(DrawingSvgWarningKind::PatternNotDrawn, format!("The pattern '{}' is not drawn (the renderer has no pattern paint yet): what it paints stays empty. Use a solid colour or a gradient to have it drawn.", pt.id()));
                SvgPaint::Pattern(pt.id().to_string())
            }
        }
    }

    fn node(&mut self, n: &usvg::Node, parent: usize, main_tree: bool) -> Option<usize> {
        match n {
            usvg::Node::Group(g) => Some(self.group(g, Some(parent), main_tree)),
            usvg::Node::Path(p) => {
                let fill = p.fill().map(|f| SvgFill {
                    paint: self.paint(f.paint()),
                    opacity: f.opacity().get(),
                    rule: match f.rule() {
                        usvg::FillRule::NonZero => SvgFillRule::NonZero,
                        usvg::FillRule::EvenOdd => SvgFillRule::EvenOdd,
                    },
                });
                let non_scaling_stroke = !p.id().is_empty() && self.non_scaling.contains(p.id());
                let stroke = p.stroke().map(|s| SvgStroke {
                    paint: self.paint(s.paint()),
                    opacity: s.opacity().get(),
                    width: s.width().get(),
                    cap: match s.linecap() {
                        usvg::LineCap::Butt => SvgLineCap::Butt,
                        usvg::LineCap::Round => SvgLineCap::Round,
                        usvg::LineCap::Square => SvgLineCap::Square,
                    },
                    join: match s.linejoin() {
                        usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => SvgLineJoin::Miter,
                        usvg::LineJoin::Round => SvgLineJoin::Round,
                        usvg::LineJoin::Bevel => SvgLineJoin::Bevel,
                    },
                    miter_limit: s.miterlimit().get(),
                    dash_array: s.dasharray().map(|d| d.to_vec()),
                    dash_offset: s.dashoffset(),
                    non_scaling: non_scaling_stroke,
                });
                Some(self.push(SvgSceneNode {
                    id: p.id().to_string(),
                    transform: Affine::IDENTITY,
                    parent: Some(parent),
                    document_bounds: if main_tree { p.abs_bounding_box().map(rect) } else { None },
                    kind: SvgNodeKind::Path(SvgPathNode {
                        path: Arc::new(bez(p.data())),
                        fill,
                        stroke,
                        paint_order: match p.paint_order() {
                            usvg::PaintOrder::FillAndStroke => SvgPaintOrder::FillAndStroke,
                            usvg::PaintOrder::StrokeAndFill => SvgPaintOrder::StrokeAndFill,
                        },
                        visible: p.is_visible(),
                        anti_alias: p.rendering_mode().use_shape_antialiasing(),
                        non_scaling_stroke,
                    }),
                }))
            }
            usvg::Node::Image(img) => {
                let source = match img.kind() {
                    usvg::ImageKind::PNG(b) if b.len() >= 33 && b[33..].starts_with(HREF_MARKER) => SvgImageSource::Href(String::from_utf8_lossy(&b[33 + HREF_MARKER.len()..]).into_owned()),
                    usvg::ImageKind::PNG(b) => SvgImageSource::Encoded { format: "png", bytes: b.clone() },
                    usvg::ImageKind::JPEG(b) => SvgImageSource::Encoded { format: "jpeg", bytes: b.clone() },
                    usvg::ImageKind::GIF(b) => SvgImageSource::Encoded { format: "gif", bytes: b.clone() },
                    usvg::ImageKind::WEBP(b) => SvgImageSource::Encoded { format: "webp", bytes: b.clone() },
                    usvg::ImageKind::SVG(_) => {
                        self.warnings.add(DrawingSvgWarningKind::NestedSvgImageNotDrawn, format!("The image '{}' is an SVG document: nested SVG images are not drawn in this version.", img.id()));
                        return None;
                    }
                };
                let what = match &source {
                    SvgImageSource::Href(h) => format!("the external picture '{h}' (never fetched by the crate)"),
                    SvgImageSource::Encoded { format, .. } => format!("an embedded {format} picture (never decoded by the crate)"),
                };
                self.warnings.add(DrawingSvgWarningKind::ImageNeedsPixels, format!("The document draws {what}: supply its decoded pixels for the image record to be drawn."));
                let size = img.size();
                Some(self.push(SvgSceneNode {
                    id: img.id().to_string(),
                    transform: Affine::IDENTITY,
                    parent: Some(parent),
                    document_bounds: if main_tree { Some(rect(img.abs_bounding_box())) } else { None },
                    kind: SvgNodeKind::Image(SvgImageNode {
                        source,
                        size: (size.width() as f64, size.height() as f64),
                        smooth: matches!(img.rendering_mode(), usvg::ImageRendering::OptimizeQuality | usvg::ImageRendering::Smooth | usvg::ImageRendering::HighQuality),
                        visible: img.is_visible(),
                    }),
                }))
            }
            // SVG text, laid out by usvg (its `text` feature) with the fonts the application
            // supplied and turned into paths: its flattened group carries the text element's id.
            usvg::Node::Text(t) => Some(self.group(t.flattened(), Some(parent), main_tree)),
        }
    }
}

fn primitive_name(k: &usvg::filter::Kind) -> &'static str {
    use usvg::filter::Kind as K;
    match k {
        K::Blend(_) => "feBlend",
        K::ColorMatrix(_) => "feColorMatrix",
        K::ComponentTransfer(_) => "feComponentTransfer",
        K::Composite(_) => "feComposite",
        K::ConvolveMatrix(_) => "feConvolveMatrix",
        K::DiffuseLighting(_) => "feDiffuseLighting",
        K::DisplacementMap(_) => "feDisplacementMap",
        K::DropShadow(_) => "feDropShadow",
        K::Flood(_) => "feFlood",
        K::GaussianBlur(_) => "feGaussianBlur",
        K::Image(_) => "feImage",
        K::Merge(_) => "feMerge",
        K::Morphology(_) => "feMorphology",
        K::Offset(_) => "feOffset",
        K::SpecularLighting(_) => "feSpecularLighting",
        K::Tile(_) => "feTile",
        K::Turbulence(_) => "feTurbulence",
    }
}

/// Finds the main-tree nodes of every id, their owners, and the base values.
fn collect_parts(scene: &mut SvgScene, xml: &roxmltree::Document, forced: &HashSet<String>) {
    let mut elements: HashMap<&str, roxmltree::Node> = HashMap::new();
    for e in xml.descendants().filter(|n| n.is_element()) {
        if let Some(id) = e.attribute("id") {
            elements.entry(id).or_insert(e);
        }
    }
    // Main tree only: walk from the root.
    let mut order = Vec::new();
    let mut stack = vec![scene.root];
    while let Some(i) = stack.pop() {
        order.push(i);
        let collect_mask = |n: &SvgSceneNode, out: &mut Vec<usize>| {
            if let SvgNodeKind::Group(g) = &n.kind {
                let mut m = g.mask.as_ref();
                while let Some(mask) = m {
                    out.push(mask.content);
                    m = mask.mask.as_deref();
                }
            }
        };
        let mut extra = Vec::new();
        collect_mask(&scene.nodes[i], &mut extra);
        if let SvgNodeKind::Group(g) = &scene.nodes[i].kind {
            for c in g.children.iter().rev() {
                stack.push(*c);
            }
        }
        stack.extend(extra);
    }
    for i in order {
        let id = scene.nodes[i].id.clone();
        if id.is_empty() {
            continue;
        }
        let element = elements.get(id.as_str());
        // A shape or image that is an animation target was given a group of its own (see
        // DrawingSvg::from_svg): that group is its owner, and its forced isolation is dropped.
        let mut owner = i;
        if !matches!(scene.nodes[i].kind, SvgNodeKind::Group(_)) && forced.contains(&id) {
            if let Some(p) = scene.nodes[i].parent {
                let own = scene.nodes[p].id.is_empty() && matches!(&scene.nodes[p].kind, SvgNodeKind::Group(g) if g.children.len() == 1 && g.isolate);
                if own {
                    owner = p;
                    if let SvgNodeKind::Group(g) = &mut scene.nodes[p].kind {
                        g.isolate = false;
                    }
                }
            }
        }
        let position = match scene.part_index.get(&id) {
            Some(p) => *p,
            None => {
                let transform_attribute = element.and_then(|e| e.attribute("transform")).and_then(|t| t.parse::<svgtypes::Transform>().ok()).map(|t| Affine::new([t.a, t.b, t.c, t.d, t.e, t.f])).unwrap_or(Affine::IDENTITY);
                let base = base_of(scene, owner);
                scene.parts.push(SvgPartInfo { id: id.clone(), element_name: element.map(|e| e.tag_name().name().to_string()).unwrap_or_default(), owners: Vec::new(), transform_attribute, document_bounds: None, base });
                scene.part_index.insert(id.clone(), scene.parts.len() - 1);
                scene.parts.len() - 1
            }
        };
        let bounds = scene.nodes[owner].document_bounds.or(scene.nodes[i].document_bounds);
        let part = &mut scene.parts[position];
        if !part.owners.contains(&owner) {
            part.owners.push(owner);
            part.document_bounds = match (part.document_bounds, bounds) {
                (Some(a), Some(b)) => Some(a.union(b)),
                (a, b) => a.or(b),
            };
            scene.owner_of[owner].push(position as u32);
        }
    }
}

fn base_of(scene: &SvgScene, owner: usize) -> SvgPartBase {
    let mut base = SvgPartBase::default();
    if let SvgNodeKind::Group(g) = &scene.nodes[owner].kind {
        base.opacity = g.opacity;
    }
    let mut stack = vec![owner];
    while let Some(i) = stack.pop() {
        match &scene.nodes[i].kind {
            SvgNodeKind::Group(g) => stack.extend(g.children.iter().rev().copied()),
            SvgNodeKind::Path(p) => {
                if let Some(f) = &p.fill {
                    if let SvgPaint::Solid(c) = f.paint {
                        base.fill = Some(c);
                    }
                    base.fill_opacity = f.opacity;
                }
                if let Some(s) = &p.stroke {
                    if let SvgPaint::Solid(c) = s.paint {
                        base.stroke = Some(c);
                    }
                    base.stroke_opacity = s.opacity;
                    base.stroke_width = s.width;
                    base.stroke_dash_offset = s.dash_offset;
                    base.stroke_dash_array = s.dash_array.clone();
                }
                base.path_length = PathGeometry::from_bez_path((*p.path).clone(), PathFillRule::NonZero).get_length();
                break;
            }
            SvgNodeKind::Image(_) => {}
        }
    }
    base
}
