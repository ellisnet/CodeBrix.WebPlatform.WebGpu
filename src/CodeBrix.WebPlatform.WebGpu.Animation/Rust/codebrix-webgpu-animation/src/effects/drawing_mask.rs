//! `DrawingMask`: what part of a layer's or a group's content is shown (C# `DrawingMask`).

use std::rc::Rc;

use crate::effects::DrawingMaskKind;
use crate::geometry::RectangleF;
use crate::images::ImageSource;
use crate::paths::PathGeometry;

/// What part of a layer's or a group's content is shown: only inside (or only outside) a path,
/// or only where another layer's elements are (a matte) (C# `DrawingMask`). Set it on a layer
/// ([`DrawingLayer::set_mask`](crate::models::DrawingLayer::set_mask)) or a group
/// ([`DrawingGroup::set_mask`](crate::shapes::DrawingGroup::set_mask)); to mask one element, put
/// it in a group. A mask is a value: clone it to share it.
///
/// A mask is always in the SAME coordinates as the content it masks: a layer's mask in the
/// layer's element coordinates (it follows the layer's transform and the camera, as its elements
/// do), a group's mask in the group's coordinates (it moves, turns and scales with the group).
/// The content is drawn whole first and the mask then multiplies its coverage once, so
/// overlapping elements never show a seam along the mask's edge.
///
/// THE MATTE LAYER is named, not held: Rust has no shared object references, so
/// [`DrawingMask::from_layer`] names a layer of the SAME session (C# takes the layer object, which
/// may be outside the session). Hide that layer (`set_is_visible(false)`) to use it only as a
/// matte; its elements, bindings and time windows apply as usual. An unknown name covers nothing,
/// and the session REPORTS it once ([`DrawingSession::take_reports`](crate::DrawingSession::take_reports)).
///
/// A PLANE MASK ([`DrawingMask::from_plane`]) keeps the content in proportion to the values of a
/// [`DrawingPlane`](crate::planes::DrawingPlane) stretched over a rectangle (C# `FromPlane`): the
/// values are taken once as 8-bit alpha (C#'s `UnitToByte`), uploaded to the GPU as a picture
/// and sampled bilinear; outside the rectangle the mask is 0 (1 when inverted).
///
/// Edges are always anti-aliased (4x multisampling); C#'s `antialias: false` has no effect here.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingMask {
    kind: DrawingMaskKind,
    geometry: Option<PathGeometry>,
    layer: Option<String>,
    is_inverted: bool,
    is_antialiased: bool,
    plane: Option<Rc<ImageSource>>,
    destination: RectangleF,
}

impl DrawingMask {
    /// Keeps only the content inside a path (its fill rule decides what is inside; C#
    /// `Inside(geometry, antialias = true)`).
    pub fn inside(geometry: PathGeometry) -> DrawingMask {
        DrawingMask { kind: DrawingMaskKind::Inside, geometry: Some(geometry), layer: None, is_inverted: false, is_antialiased: true, plane: None, destination: RectangleF::default() }
    }

    /// Keeps only the content outside a path (C# `Outside(geometry, antialias = true)`).
    pub fn outside(geometry: PathGeometry) -> DrawingMask {
        DrawingMask { kind: DrawingMaskKind::Outside, geometry: Some(geometry), layer: None, is_inverted: true, is_antialiased: true, plane: None, destination: RectangleF::default() }
    }

    /// Keeps the content only where the elements of the session's layer `name` cover it - their
    /// coverage as they are drawn (anti-aliased edges and their colours' alpha count), in the
    /// masked content's coordinates, at the session's time; with `invert`, only where they do NOT
    /// cover it (C# `FromLayer(layer, invert = false)`). The matte layer's own opacity,
    /// transform, mask, effects and visibility are not used.
    pub fn from_layer(name: &str, invert: bool) -> DrawingMask {
        DrawingMask { kind: DrawingMaskKind::Layer, geometry: None, layer: Some(name.to_string()), is_inverted: invert, is_antialiased: true, plane: None, destination: RectangleF::default() }
    }

    /// Keeps the content in proportion to a plane's values stretched over `destination` (in the
    /// masked content's coordinates), 0 outside it; with `invert`, the opposite (C#
    /// `FromPlane(plane, destination, invert = false)`). The plane is copied now, as 8-bit values
    /// (C#'s `UnitToByte`). The error says what is wrong with the rectangle.
    pub fn from_plane(plane: &crate::planes::DrawingPlane, destination: RectangleF, invert: bool) -> Result<DrawingMask, String> {
        let (x, y, w, h) = (destination.x, destination.y, destination.width, destination.height);
        if !(w > 0.0) || !(h > 0.0) || !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() {
            return Err("The destination must be a finite rectangle with a positive width and height.".to_string());
        }
        let alpha = plane.to_bytes();
        let mut rgba = Vec::with_capacity(alpha.len() * 4);
        for a in alpha {
            rgba.extend_from_slice(&[255, 255, 255, a]);
        }
        let picture = ImageSource::from_rgba_vec(rgba, plane.width(), plane.height())?;
        Ok(DrawingMask { kind: DrawingMaskKind::Plane, geometry: None, layer: None, is_inverted: invert, is_antialiased: true, plane: Some(Rc::new(picture)), destination })
    }

    /// A plane mask over the plane's own size at the origin (C# `FromPlane(plane, invert =
    /// false)`: destination (0, 0, width, height)).
    pub fn from_plane_at_origin(plane: &crate::planes::DrawingPlane, invert: bool) -> DrawingMask {
        let destination = RectangleF::new(0.0, 0.0, plane.width() as f32, plane.height() as f32);
        // A plane is never empty, so its own rectangle is always valid.
        DrawingMask::from_plane(plane, destination, invert).unwrap_or_else(|_| DrawingMask::from_layer("", invert))
    }

    /// The picture of a plane mask (white, alpha = the plane's 8-bit values) and its size.
    pub fn plane(&self) -> Option<(&ImageSource, (u32, u32))> {
        self.plane.as_deref().map(|p| (p, (p.width(), p.height())))
    }

    /// The rectangle a plane mask is stretched over (empty for other kinds).
    pub fn destination(&self) -> RectangleF {
        self.destination
    }

    /// What the mask keeps.
    pub fn kind(&self) -> DrawingMaskKind {
        self.kind
    }

    /// The path of an Inside or Outside mask.
    pub fn geometry(&self) -> Option<&PathGeometry> {
        self.geometry.as_ref()
    }

    /// The name of the matte layer of a Layer mask.
    pub fn layer(&self) -> Option<&str> {
        self.layer.as_deref()
    }

    /// True when the mask keeps the opposite of what it describes: an Outside mask, and an
    /// inverted layer mask (C# `IsInverted`).
    pub fn is_inverted(&self) -> bool {
        self.is_inverted
    }

    /// True (C# `IsAntialiased`; edges here are always anti-aliased).
    pub fn is_antialiased(&self) -> bool {
        self.is_antialiased
    }
}
