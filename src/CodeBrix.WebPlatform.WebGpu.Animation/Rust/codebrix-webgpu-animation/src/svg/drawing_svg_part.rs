//! `DrawingSvgPart`: the program-driven override of one part of a document (C#
//! `DrawingSvgPart`): translation, rotation, scale about a pivot, opacity, visibility and stroke
//! trims, all in DOCUMENT coordinates, applied as if the element were wrapped in an extra group.

use kurbo::{Affine, Rect};

use crate::geometry::{PointF, RectangleF};
use crate::models::{DrawingTransform, DrawingTransformOptions};

/// One part's override (C# `DrawingSvgPart`). Defaults change nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingSvgPart {
    id: String,
    document_bounds: Option<Rect>,
    /// The move, in document units.
    pub translation: PointF,
    /// The turn, degrees clockwise on screen, about the pivot.
    pub rotation: f32,
    /// The horizontal scale about the pivot.
    pub scale_x: f32,
    /// The vertical scale about the pivot.
    pub scale_y: f32,
    /// The point rotation and scale turn about, in document coordinates; None = the centre of
    /// each owner node's own box (so every `<use>` instance turns about its own centre).
    pub pivot: Option<PointF>,
    /// Multiplies the part's own opacity, 0..1.
    pub opacity: f32,
    /// False hides the part and everything in it.
    pub is_visible: bool,
    /// The fraction of each stroked path's length where its drawn part starts, 0..1.
    pub trim_start: f32,
    /// The fraction where it ends, 0..1.
    pub trim_end: f32,
}

impl DrawingSvgPart {
    pub(crate) fn new(id: &str, document_bounds: Option<Rect>) -> DrawingSvgPart {
        DrawingSvgPart { id: id.to_string(), document_bounds, translation: PointF::new(0.0, 0.0), rotation: 0.0, scale_x: 1.0, scale_y: 1.0, pivot: None, opacity: 1.0, is_visible: true, trim_start: 0.0, trim_end: 1.0 }
    }

    /// The element id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The part's box in document coordinates as loaded (C# `DocumentBounds`; None when the part
    /// draws nothing).
    pub fn document_bounds(&self) -> Option<RectangleF> {
        self.document_bounds.map(|r| RectangleF::new(r.x0 as f32, r.y0 as f32, r.width() as f32, r.height() as f32))
    }

    /// The centre of the part's box (C# `Center`).
    pub fn center(&self) -> PointF {
        self.document_bounds.map(|r| PointF::new(r.center().x as f32, r.center().y as f32)).unwrap_or(PointF::new(0.0, 0.0))
    }

    /// True when anything differs from the defaults (C# `HasOverrides`).
    pub fn has_overrides(&self) -> bool {
        *self != DrawingSvgPart::new(&self.id, self.document_bounds)
    }

    /// Back to the defaults (C# `Reset`).
    pub fn reset(&mut self) {
        *self = DrawingSvgPart::new(&self.id, self.document_bounds);
    }

    /// Translation, rotation, scale and pivot as one transform (C# `Transform` get); the pivot is
    /// the transform's anchor (the part's centre when no pivot is set).
    pub fn transform(&self) -> DrawingTransform {
        let anchor = self.pivot.unwrap_or(self.center());
        DrawingTransform::new(self.translation, DrawingTransformOptions { rotation: self.rotation, scale_x: self.scale_x, scale_y: self.scale_y, anchor }).unwrap_or_default()
    }

    /// Sets translation, rotation, scale and pivot at once (C# `Transform` set).
    pub fn set_transform(&mut self, transform: DrawingTransform) {
        self.translation = transform.translation();
        self.rotation = transform.rotation();
        self.scale_x = transform.scale_x();
        self.scale_y = transform.scale_y();
        self.pivot = Some(transform.anchor());
    }

    /// The document-space matrix for one owner node whose own box centre is `node_center`
    /// (used when no pivot is set): translate(-pivot), scale, rotate, translate(pivot +
    /// translation).
    pub(crate) fn matrix_for(&self, node_center: PointF) -> Affine {
        let anchor = self.pivot.unwrap_or(node_center);
        DrawingTransform::new(self.translation, DrawingTransformOptions { rotation: self.rotation, scale_x: self.scale_x, scale_y: self.scale_y, anchor }).map(|t| t.to_matrix()).unwrap_or(Affine::IDENTITY)
    }

    /// True when the trims cut anything.
    pub(crate) fn is_trimmed(&self) -> bool {
        self.trim_start > 0.0 || self.trim_end < 1.0
    }
}
