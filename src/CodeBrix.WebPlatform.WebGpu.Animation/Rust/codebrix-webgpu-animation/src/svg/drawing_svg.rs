//! `DrawingSvg`: a loaded SVG document (C# `DrawingSvg`): its scene as data, its parts, its own
//! animations and the report of what is not drawn or played. It takes the document's TEXT (or
//! bytes) from anywhere - the backend, a packed asset, a string the program writes - and never
//! reads a file, fetches anything or reads a clock.

use std::collections::HashMap;
use std::sync::Arc;

use kurbo::Affine;
use usvg::roxmltree;

use crate::geometry::RectangleF;
use crate::svg::animation::css_animation::{self, StyleSheet};
use crate::svg::animation::smil_animation::{self, SMIL_ELEMENTS};
use crate::svg::animation::svg_value::AnimAttr;
use crate::svg::animation::svg_animations::SvgAnimations;
use crate::svg::drawing_svg_options::DrawingSvgOptions;
use crate::svg::drawing_svg_part::DrawingSvgPart;
use crate::svg::drawing_svg_warning::{DrawingSvgWarning, DrawingSvgWarningKind, WarningList};
use crate::svg::scene_builder::{SceneBuilder, HREF_MARKER};
use crate::svg::scene_walker::Walker;
use crate::svg::svg_draw_command::SvgDrawList;
use crate::svg::svg_part_override::SvgAnimationFrame;
use crate::svg::svg_scene::SvgScene;

/// The prefix of the ids the crate gives to animated elements that have none (so that their
/// overrides can find them). Never write ids with this prefix in a document.
pub const SYNTHETIC_ID_PREFIX: &str = "codebrix-svg-part-";

/// A loaded document (C# `DrawingSvg`). Immutable once loaded; share it with an `Arc` between
/// several [`DrawingSvgShape`](crate::svg::DrawingSvgShape)s.
#[derive(Clone, Debug)]
pub struct DrawingSvg {
    scene: SvgScene,
    animations: SvgAnimations,
    warnings: Vec<DrawingSvgWarning>,
}

fn qualified_name_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut i = start + 1;
    while i < bytes.len() && !(bytes[i].is_ascii_whitespace() || bytes[i] == b'/' || bytes[i] == b'>') {
        i += 1;
    }
    i
}

/// A PNG signature and IHDR chunk of `w` x `h` (enough for usvg's size probe), then the marker
/// and the href.
fn href_stand_in(href: &str, w: u32, h: u32) -> Vec<u8> {
    let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
    b.extend_from_slice(&w.to_be_bytes());
    b.extend_from_slice(&h.to_be_bytes());
    b.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    b.extend_from_slice(HREF_MARKER);
    b.extend_from_slice(href.as_bytes());
    b
}

/// The font families text in the tree names (not generic ones) that the supplied fonts lack.
fn find_missing_families(g: &usvg::Group, have: &[String], out: &mut Vec<String>) {
    for n in g.children() {
        match n {
            usvg::Node::Group(c) => find_missing_families(c, have, out),
            usvg::Node::Text(t) => {
                for chunk in t.chunks() {
                    for span in chunk.spans() {
                        let named: Vec<&str> = span.font().families().iter().filter_map(|f| if let usvg::FontFamily::Named(s) = f { Some(s.as_str()) } else { None }).collect();
                        if !named.is_empty() && !named.iter().any(|f| have.iter().any(|h| h.eq_ignore_ascii_case(f))) {
                            let name = named.join(", ");
                            if !out.contains(&name) {
                                out.push(name);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        n.subroots(|s| find_missing_families(s, have, out));
    }
}

fn parse_xml(text: &str) -> Result<roxmltree::Document<'_>, String> {
    roxmltree::Document::parse_with_options(text, roxmltree::ParsingOptions { allow_dtd: true, ..roxmltree::ParsingOptions::default() }).map_err(|e| format!("The SVG is not well-formed XML ({e}); fix the markup and load it again."))
}

impl DrawingSvg {
    /// Loads a document from its markup (C# `FromSvg`). The error says what is wrong with it.
    pub fn from_svg(svg_markup: &str, options: &DrawingSvgOptions) -> Result<DrawingSvg, String> {
        if svg_markup.trim().is_empty() {
            return Err("The SVG markup is empty; pass the document's text.".to_string());
        }
        let mut warnings = WarningList::default();
        let doc = parse_xml(svg_markup)?;
        if doc.root_element().tag_name().name() != "svg" {
            return Err("The document's root element is not <svg>; pass an SVG document.".to_string());
        }
        let mut text_seen = false;
        for e in doc.descendants().filter(|n| n.is_element()) {
            let name = e.tag_name().name();
            if name == "script" {
                warnings.add(DrawingSvgWarningKind::ScriptIgnored, "The document has a <script>: scripts never run (scripted animation is not played).");
            }
            if let Some(a) = e.attributes().find(|a| a.name().starts_with("on") && a.name().len() > 2) {
                warnings.add(DrawingSvgWarningKind::ScriptIgnored, format!("The event attribute '{}' is ignored: scripts never run.", a.name()));
            }
            if name == "text" && !text_seen {
                text_seen = true;
                if options.fonts.is_empty() {
                    warnings.add(DrawingSvgWarningKind::NoFontsRegistered, "The document has <text> but no font was supplied, so the text is not drawn. Pass the font's bytes in DrawingSvgOptions::fonts (the crate never uses a system font).");
                }
            }
            if name == "animateColor" || name == "discard" {
                warnings.add(DrawingSvgWarningKind::AnimationIgnored, format!("<{name}> is not played (not part of the supported SMIL set: animate, set, animateTransform, animateMotion)."));
            }
        }

        // SMIL animation elements, read from the XML (usvg drops them).
        let mut smil = Vec::new();
        let mut order = 0;
        for e in doc.descendants().filter(|n| n.is_element() && SMIL_ELEMENTS.contains(&n.tag_name().name())) {
            match smil_animation::read(e, &doc, order) {
                Ok(a) => smil.push(a),
                Err(why) => warnings.add(DrawingSvgWarningKind::AnimationIgnored, format!("The <{}>{} is ignored: {why}.", e.tag_name().name(), e.attribute("id").map(|i| format!(" '{i}'")).unwrap_or_default())),
            }
            order += 1;
        }
        // The CSS subset: <style> text, cascaded per element.
        let css_text: String = doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "style").map(|s| s.descendants().filter(|t| t.is_text()).filter_map(|t| t.text()).collect::<String>()).collect::<Vec<_>>().join("\n");
        let sheet = StyleSheet::parse(&css_text, &mut warnings);
        let mut css_targets = Vec::new();
        for e in doc.descendants().filter(|n| n.is_element()) {
            let decls = if sheet.has_animations() || e.attribute("style").map(|s| s.contains("animation")).unwrap_or(false) { sheet.computed(e) } else { HashMap::new() };
            if decls.contains_key("animation") || decls.contains_key("animation-name") {
                css_targets.push((e.id(), decls));
            }
        }

        // Every target needs an id: the crate gives one to those without.
        // Every target needs an id: the crate gives one to those without. A target that is a
        // shape or an image also gets isolation="isolate" (no visible change: nothing inside it
        // blends), which makes usvg give it a group of its OWN - where its transform and opacity
        // live, so that animations can replace them; the scene drops that isolation again.
        let existing: std::collections::HashSet<&str> = doc.descendants().filter_map(|n| n.attribute("id")).collect();
        let mut ids: HashMap<roxmltree::NodeId, String> = HashMap::new();
        let mut forced: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut inserts: Vec<(usize, String)> = Vec::new();
        let mut counter = 0;
        let targets: Vec<roxmltree::NodeId> = smil.iter().map(|a| a.target).chain(css_targets.iter().map(|(n, _)| *n)).collect();
        for t in targets {
            if ids.contains_key(&t) {
                continue;
            }
            let node = doc.get_node(t).ok_or("internal: a target vanished")?;
            let mut insert = String::new();
            let id = match node.attribute("id") {
                Some(id) => id.to_string(),
                None => {
                    let mut id = format!("{SYNTHETIC_ID_PREFIX}{counter}");
                    while existing.contains(id.as_str()) {
                        counter += 1;
                        id = format!("{SYNTHETIC_ID_PREFIX}{counter}");
                    }
                    counter += 1;
                    insert.push_str(&format!(" id=\"{id}\""));
                    id
                }
            };
            // usvg reads `isolation` only from a style attribute (or CSS).
            let shape = matches!(node.tag_name().name(), "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" | "path" | "image");
            let style = node.attributes().find(|a| a.name() == "style" && a.namespace().is_none());
            let declares_isolation = node.attribute("isolation").is_some() || style.as_ref().map(|s| s.value().contains("isolation")).unwrap_or(false);
            if shape && !declares_isolation {
                match &style {
                    Some(s) => inserts.push((s.range_value().start, "isolation:isolate;".to_string())),
                    None => insert.push_str(" style=\"isolation:isolate\""),
                }
                forced.insert(id.clone());
            }
            if !insert.is_empty() {
                inserts.push((qualified_name_end(svg_markup, node.range().start), insert));
            }
            ids.insert(t, id);
        }
        // Inherited properties done as browsers do them: an animated colour, opacity of fill or
        // stroke, stroke value or visibility on an element reaches the elements inside it that
        // do NOT set that property themselves. Every descendant of a target that sets one of
        // them gets an id (when it has none) so that the scene can stop the value there.
        let mut blockers: Vec<(String, Vec<AnimAttr>)> = Vec::new();
        let mut looked_at: std::collections::HashSet<roxmltree::NodeId> = std::collections::HashSet::new();
        let target_nodes: Vec<roxmltree::NodeId> = ids.keys().copied().collect();
        for t in target_nodes {
            let Some(node) = doc.get_node(t) else { continue };
            for d in node.descendants().skip(1).filter(|n| n.is_element()) {
                if !looked_at.insert(d.id()) {
                    continue;
                }
                let own = sheet.specified_inherited(d);
                if own.is_empty() {
                    continue;
                }
                let id = match (ids.get(&d.id()), d.attribute("id")) {
                    (Some(id), _) => id.clone(),
                    (None, Some(id)) => id.to_string(),
                    (None, None) => {
                        let mut id = format!("{SYNTHETIC_ID_PREFIX}{counter}");
                        while existing.contains(id.as_str()) {
                            counter += 1;
                            id = format!("{SYNTHETIC_ID_PREFIX}{counter}");
                        }
                        counter += 1;
                        inserts.push((qualified_name_end(svg_markup, d.range().start), format!(" id=\"{id}\"")));
                        id
                    }
                };
                blockers.push((id, own));
            }
        }
        // The ids usvg clears: every id inside content a <use> instances, the <use>d element's
        // own included. An animation that targets one cannot be played - say so by name.
        let used: std::collections::HashSet<&str> = doc
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "use")
            .filter_map(|u| u.attribute(("http://www.w3.org/1999/xlink", "href")).or_else(|| u.attribute("href")))
            .filter_map(|h| h.trim().strip_prefix('#'))
            .collect();
        let inside_use = |n: roxmltree::Node| n.ancestors().filter(|a| a.is_element()).any(|a| a.attribute("id").map(|i| used.contains(i)).unwrap_or(false));
        let mut prepared = svg_markup.to_string();
        inserts.sort_by(|a, b| b.0.cmp(&a.0));
        for (at, text) in &inserts {
            prepared.insert_str(*at, text);
        }

        let mut opt = usvg::Options { dpi: options.dpi, style_sheet: options.style_sheet.clone(), ..usvg::Options::default() };
        // Fonts: only those the application supplied, loaded from their bytes. Every generic
        // family and the default family are the application's family, so text that names a
        // family the supplied fonts lack is drawn in it (no system font is ever used).
        let mut font_families: Vec<String> = Vec::new();
        if !options.fonts.is_empty() {
            let mut db = usvg::fontdb::Database::new();
            for f in &options.fonts {
                db.load_font_data(f.as_ref().clone());
            }
            for face in db.faces() {
                for (name, _) in &face.families {
                    if !font_families.contains(name) {
                        font_families.push(name.clone());
                    }
                }
            }
            if db.is_empty() {
                warnings.add(DrawingSvgWarningKind::NoFontsRegistered, "None of the supplied fonts could be read (they are not TrueType / OpenType data), so text is not drawn. Pass the bytes of .ttf / .otf files.");
            }
            let family = options.font_family.clone().or_else(|| font_families.first().cloned()).unwrap_or_default();
            db.set_serif_family(family.clone());
            db.set_sans_serif_family(family.clone());
            db.set_monospace_family(family.clone());
            db.set_cursive_family(family.clone());
            db.set_fantasy_family(family.clone());
            opt.font_family = family;
            opt.fontdb = Arc::new(db);
        }
        if let Some(size) = usvg::Size::from_wh(options.default_width, options.default_height) {
            opt.default_size = size;
        }
        // External hrefs are never fetched: the resolver hands usvg a stand-in (a PNG header of
        // the element's own width and height, so usvg keeps the node and its aspect) carrying
        // the href, which the scene turns back into SvgImageSource::Href.
        let mut sizes: HashMap<String, (u32, u32)> = HashMap::new();
        for e in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "image") {
            if let Some(h) = e.attribute("href").or_else(|| e.attribute(("http://www.w3.org/1999/xlink", "href"))) {
                let dim = |a: &str| e.attribute(a).and_then(|v| v.trim().trim_end_matches("px").parse::<f64>().ok()).map(|v| v.round().max(1.0) as u32).unwrap_or(1);
                sizes.entry(h.to_string()).or_insert((dim("width"), dim("height")));
            }
        }
        opt.image_href_resolver.resolve_string = Box::new(move |href: &str, _: &usvg::Options| {
            let (w, h) = sizes.get(href).copied().unwrap_or((1, 1));
            Some(usvg::ImageKind::PNG(Arc::new(href_stand_in(href, w, h))))
        });
        let tree = usvg::Tree::from_str(&prepared, &opt).map_err(|e| format!("The SVG could not be loaded as a drawing ({e}); check that it is a valid SVG document with a size."))?;
        let prepared_doc = parse_xml(&prepared)?;
        let mut scene = SceneBuilder::new(&mut warnings).build(&tree, &prepared_doc, &forced);
        for (id, own) in blockers {
            if let Some(p) = scene.part_index.get(&id).copied() {
                for o in scene.parts[p].owners.clone() {
                    scene.own_style.entry(o).or_default().extend(own.iter().copied());
                }
            }
        }
        if !options.fonts.is_empty() {
            let mut missing: Vec<String> = Vec::new();
            find_missing_families(tree.root(), &font_families, &mut missing);
            for family in missing {
                warnings.add(DrawingSvgWarningKind::FontSubstituted, format!("The document's text names the font family '{family}', which the supplied fonts do not have: it is drawn in '{}'. Supply that font, or set DrawingSvgOptions::font_family.", opt.font_family));
            }
        }

        let mut animations = SvgAnimations::default();
        for a in smil {
            let id = ids[&a.target].clone();
            if scene.part(&id).is_none() {
                if doc.get_node(a.target).map(inside_use).unwrap_or(false) {
                    let name = if a.own_id.is_empty() { String::new() } else { format!(" '{}'", a.own_id) };
                    warnings.add(DrawingSvgWarningKind::AnimationTargetInsideUse, format!("The <{}>{name} animating {:?} of '{}' is not played: '{}' is inside content a <use> instances, and those ids are cleared when the document is loaded. Give the <use> element an id of its own and animate that.", a.element, a.attr, id, id));
                    continue;
                }
                warnings.add(DrawingSvgWarningKind::AnimationIgnored, format!("An <{}> targets '{}', which is not drawn (inside <defs>, display:none, or the root <svg>): ignored.", a.element, if id.starts_with(SYNTHETIC_ID_PREFIX) { "an element without an id" } else { id.as_str() }));
                continue;
            }
            if !a.ignored_times.is_empty() {
                warnings.add(DrawingSvgWarningKind::AnimationIgnored, format!("An <{}> on '{}' has begin / end entries that are not time-based ({}): they are ignored, the time-based ones are played.", a.element, id, a.ignored_times.join("; ")));
            }
            if a.calc == smil_animation::CalcMode::Linear && a.motion_path.is_some() && a.key_times.is_none() {
                warnings.add(DrawingSvgWarningKind::AnimationApproximated, "animateMotion along a path with calcMode=\"linear\" and no keyPoints is played paced (an even speed along the path).");
            }
            animations.smil.push((id, a));
        }
        for (node, decls) in css_targets {
            let id = ids[&node].clone();
            if scene.part(&id).is_none() {
                if doc.get_node(node).map(inside_use).unwrap_or(false) {
                    let names = decls.get("animation-name").or_else(|| decls.get("animation")).cloned().unwrap_or_default();
                    warnings.add(DrawingSvgWarningKind::AnimationTargetInsideUse, format!("The CSS animation '{}' on '{id}' is not played: '{id}' is inside content a <use> instances, and those ids are cleared when the document is loaded. Give the <use> element an id of its own and animate that.", names.trim()));
                    continue;
                }
                warnings.add(DrawingSvgWarningKind::AnimationIgnored, format!("A CSS animation targets '{id}', which is not drawn: ignored."));
                continue;
            }
            animations.css.extend(css_animation::element_animations(&id, &decls, &sheet, &mut warnings));
        }
        Ok(DrawingSvg { scene, animations, warnings: warnings.items })
    }

    /// Loads a document from UTF-8 bytes (C# `Load(Stream)`; compressed `.svgz` is not
    /// supported - decompress first).
    pub fn load(bytes: &[u8], options: &DrawingSvgOptions) -> Result<DrawingSvg, String> {
        let text = std::str::from_utf8(bytes).map_err(|_| "The SVG bytes are not UTF-8 text (a compressed .svgz must be decompressed first).".to_string())?;
        DrawingSvg::from_svg(text, options)
    }

    /// The scene as data.
    pub fn scene(&self) -> &SvgScene {
        &self.scene
    }

    /// The document's box in CSS pixels at 96 DPI (C# `Bounds`).
    pub fn bounds(&self) -> RectangleF {
        RectangleF::new(0.0, 0.0, self.scene.size.0 as f32, self.scene.size.1 as f32)
    }

    /// What is not drawn or not played as written, deduplicated (C# `Warnings`).
    pub fn warnings(&self) -> &[DrawingSvgWarning] {
        &self.warnings
    }

    /// The report of ignored and approximated features: the same list as
    /// [`warnings`](DrawingSvg::warnings), one line each, "Kind: message".
    pub fn report(&self) -> Vec<String> {
        self.warnings.iter().map(|w| format!("{:?}: {}", w.kind, w.message)).collect()
    }

    /// The document's own animations.
    pub fn animations(&self) -> &SvgAnimations {
        &self.animations
    }

    /// True when the document animates itself (SMIL or CSS the crate plays).
    pub fn has_animations(&self) -> bool {
        !self.animations.is_empty()
    }

    /// Every part's override from the document's own animations at `time` (seconds of document
    /// time; the caller passes it - from the frame, a timeline, a slider).
    pub fn evaluate_animations(&self, time: f64) -> SvgAnimationFrame {
        self.animations.evaluate(time, &self.scene)
    }

    /// Every part id (C# `GetPartIds`), in document order.
    pub fn get_part_ids(&self) -> Vec<String> {
        self.scene.parts.iter().map(|p| p.id.clone()).collect()
    }

    /// True when the id names a drawn part (C# `HasPart`).
    pub fn has_part(&self, id: &str) -> bool {
        self.scene.part(id).is_some()
    }

    /// A fresh override for every part, in [`SvgScene::parts`] order.
    pub fn new_parts(&self) -> Vec<DrawingSvgPart> {
        self.scene.parts.iter().map(|p| DrawingSvgPart::new(&p.id, p.document_bounds)).collect()
    }

    /// Walks the scene at document coordinates mapped by `root`, with the document's own
    /// animation `frame` (None: as loaded) and program-driven `parts` (from
    /// [`new_parts`](DrawingSvg::new_parts); empty: none) - the low-level door
    /// [`DrawingSvgShape`](crate::svg::DrawingSvgShape) uses.
    pub fn walk(&self, frame: Option<&SvgAnimationFrame>, parts: &[DrawingSvgPart], root: Affine) -> SvgDrawList {
        Walker::walk(&self.scene, frame, parts, root)
    }
}
