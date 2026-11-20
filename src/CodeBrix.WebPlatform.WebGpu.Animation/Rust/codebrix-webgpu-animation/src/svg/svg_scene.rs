//! `SvgScene`: a loaded SVG document as plain data - the renderer's drawing primitives (kurbo
//! paths, fills and strokes with their paints, groups with opacity / blend / clip / mask, images
//! as "an image is wanted here" records, transforms) in a flat arena of nodes, every element id
//! addressable as a PART.

use std::collections::HashMap;

use kurbo::{Affine, Rect};

use crate::color::Color;
use crate::svg::animation::svg_value::AnimAttr;
use crate::svg::svg_group_node::SvgGroupNode;
use crate::svg::svg_leaf_nodes::{SvgImageNode, SvgPathNode};

/// What a node is.
#[derive(Clone, Debug, PartialEq)]
pub enum SvgNodeKind {
    /// A group of children.
    Group(SvgGroupNode),
    /// A filled and/or stroked path.
    Path(SvgPathNode),
    /// An image.
    Image(SvgImageNode),
}

/// One node of the arena.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgSceneNode {
    /// The element id usvg kept on this node ("" for none). Ids the crate gave to animation
    /// targets that had none start with [`SYNTHETIC_ID_PREFIX`](crate::svg::SYNTHETIC_ID_PREFIX).
    pub id: String,
    /// The node's own transform (relative to its parent). Paths and images carry identity:
    /// usvg puts an element's transform on a group above it.
    pub transform: Affine,
    /// The parent's arena index (None for the root and for clip / mask content roots).
    pub parent: Option<usize>,
    /// The node's box in DOCUMENT coordinates as loaded (fill box, no stroke; None when empty).
    pub document_bounds: Option<Rect>,
    /// What the node is.
    pub kind: SvgNodeKind,
}

/// The element a part id names, as found in the arena.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgPartInfo {
    /// The id.
    pub id: String,
    /// The XML element name ("g", "rect", "path", ...).
    pub element_name: String,
    /// Every OWNER node of the id: the node usvg made for the element - or the group usvg wrapped
    /// it in for its transform / opacity / clip / mask / filter. An id on content a `<use>`
    /// instances has one owner per instance.
    pub owners: Vec<usize>,
    /// The element's own `transform` attribute as written (identity when none), used to replace
    /// it by an animated one.
    pub transform_attribute: Affine,
    /// The part's box in document coordinates as loaded (all owners).
    pub document_bounds: Option<Rect>,
    /// The base (static) values of the element's animatable properties.
    pub base: SvgPartBase,
}

/// The static values an animation starts from ("to" animations, additive sums).
#[derive(Clone, Debug, PartialEq)]
pub struct SvgPartBase {
    /// The element's own `opacity`.
    pub opacity: f32,
    /// The first path's solid fill colour, if any (alpha = 255).
    pub fill: Option<Color>,
    /// The first path's `fill-opacity`.
    pub fill_opacity: f32,
    /// The first path's solid stroke colour, if any.
    pub stroke: Option<Color>,
    /// The first path's `stroke-opacity`.
    pub stroke_opacity: f32,
    /// The first path's stroke width (1 with no stroke).
    pub stroke_width: f32,
    /// The first path's dash offset.
    pub stroke_dash_offset: f32,
    /// The first path's dashes.
    pub stroke_dash_array: Option<Vec<f32>>,
    /// The first path's geometry length (for `pathLength`-free dash animations), 0 for none.
    pub path_length: f32,
}

impl Default for SvgPartBase {
    fn default() -> SvgPartBase {
        SvgPartBase { opacity: 1.0, fill: None, fill_opacity: 1.0, stroke: None, stroke_opacity: 1.0, stroke_width: 1.0, stroke_dash_offset: 0.0, stroke_dash_array: None, path_length: 0.0 }
    }
}

/// The whole document as data: the node arena, the root, the parts and the document size.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgScene {
    pub(crate) nodes: Vec<SvgSceneNode>,
    pub(crate) root: usize,
    pub(crate) parts: Vec<SvgPartInfo>,
    pub(crate) part_index: HashMap<String, usize>,
    /// For each arena node, the parts it is an owner of.
    pub(crate) owner_of: Vec<Vec<u32>>,
    pub(crate) size: (f64, f64),
    pub(crate) view_box: Rect,
    /// The inherited properties each node's element sets itself (by node index; only nodes of
    /// elements inside an animated element are listed): an animated value from an ancestor
    /// stops there.
    pub(crate) own_style: HashMap<usize, Vec<AnimAttr>>,
}

impl SvgScene {
    /// The arena: every node, the main tree and the detached clip / mask contents.
    pub fn nodes(&self) -> &[SvgSceneNode] {
        &self.nodes
    }

    /// The arena index of the root group.
    pub fn root(&self) -> usize {
        self.root
    }

    /// One node.
    pub fn node(&self, index: usize) -> Option<&SvgSceneNode> {
        self.nodes.get(index)
    }

    /// The document's width and height in CSS pixels at 96 DPI (DOCUMENT coordinates; the
    /// viewBox mapping is the root's first group transform).
    pub fn size(&self) -> (f64, f64) {
        self.size
    }

    /// The document's viewBox (its width and height when it has none).
    pub fn view_box(&self) -> Rect {
        self.view_box
    }

    /// Every part (every element id that reached the scene), in document order.
    pub fn parts(&self) -> &[SvgPartInfo] {
        &self.parts
    }

    /// The part with this id.
    pub fn part(&self, id: &str) -> Option<&SvgPartInfo> {
        self.part_index.get(id).map(|i| &self.parts[*i])
    }

    /// The index of a part in [`parts`](SvgScene::parts).
    pub fn part_position(&self, id: &str) -> Option<usize> {
        self.part_index.get(id).copied()
    }

    /// The arena indices of the nodes with this id (an id inside `<use>`d content is on every
    /// instance).
    pub fn find_nodes(&self, id: &str) -> Vec<usize> {
        self.nodes.iter().enumerate().filter(|(_, n)| n.id == id).map(|(i, _)| i).collect()
    }

    /// The number of drawable leaves (paths and images) in the main tree.
    pub fn leaf_count(&self) -> usize {
        let mut count = 0;
        let mut stack = vec![self.root];
        while let Some(i) = stack.pop() {
            match &self.nodes[i].kind {
                SvgNodeKind::Group(g) => stack.extend(g.children.iter().copied()),
                _ => count += 1,
            }
        }
        count
    }

    /// The node's absolute transform as loaded (its own and every ancestor's).
    pub fn absolute_transform(&self, index: usize) -> Affine {
        let mut m = Affine::IDENTITY;
        let mut at = Some(index);
        while let Some(i) = at {
            m = self.nodes[i].transform * m;
            at = self.nodes[i].parent;
        }
        m
    }
}
