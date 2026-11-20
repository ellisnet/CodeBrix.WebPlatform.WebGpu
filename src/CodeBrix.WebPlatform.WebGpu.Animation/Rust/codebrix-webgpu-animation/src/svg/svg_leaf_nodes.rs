//! `SvgPathNode` and `SvgImageNode`: the drawable leaves of a scene.

use std::sync::Arc;

use kurbo::BezPath;

use crate::svg::svg_paint::SvgFill;
use crate::svg::svg_stroke::{SvgPaintOrder, SvgStroke};

/// A path: geometry in its own coordinates, fill, stroke, order and visibility.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgPathNode {
    /// The geometry (shapes - rect, circle, ellipse, line, polyline, polygon - arrive as paths).
    pub path: Arc<BezPath>,
    /// The fill, None for `fill="none"`.
    pub fill: Option<SvgFill>,
    /// The stroke, None for `stroke="none"`.
    pub stroke: Option<SvgStroke>,
    /// Which of the two goes first.
    pub paint_order: SvgPaintOrder,
    /// False for `visibility="hidden"` (or a path with neither fill nor stroke).
    pub visible: bool,
    /// False for `shape-rendering="crispEdges"` / `optimizeSpeed` (no anti-aliasing asked).
    pub anti_alias: bool,
}

/// Where the pixels of an image come from. The crate decodes NOTHING: an application that
/// wants the picture decodes the bytes (or fetches the href) and hands an
/// [`ImageSource`](crate::images::ImageSource) for it to the drawing half.
#[derive(Clone, Debug, PartialEq)]
pub enum SvgImageSource {
    /// Encoded bytes from a `data:` URL; `format` is "png", "jpeg", "gif" or "webp".
    Encoded {
        /// The format of the bytes.
        format: &'static str,
        /// The encoded bytes, exactly as the document carried them.
        bytes: Arc<Vec<u8>>,
    },
    /// An external reference (`href="pictures/a.png"`), never fetched by the crate.
    Href(String),
}

/// An image: "an image is wanted here" - its source, its size in its own coordinates and how it
/// is sampled.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgImageNode {
    /// Where the pixels come from.
    pub source: SvgImageSource,
    /// The picture's own width and height (its pixel size as the encoded header states it; for an
    /// external href, the element's width and height): the picture fills 0,0 .. width,height of
    /// the image node's coordinates, and the node's transform (usvg folds the element's x, y,
    /// width, height and `preserveAspectRatio` into it, with a clip group) places it.
    pub size: (f64, f64),
    /// False for `image-rendering: pixelated` / `optimizeSpeed` / `crisp-edges` (nearest).
    pub smooth: bool,
    /// False for `visibility="hidden"`.
    pub visible: bool,
}
