//! An SVG document as an element of a drawing session (C# `session.DrawSvg(svg, destination,
//! aspect = Meet)`, `DrawingSvgSessionExtensions`): the shape on the active layer, found again by
//! its [`ElementId`] to move, turn, fade or bind its parts.

use std::sync::Arc;

use crate::drawing_session::DrawingSession;
use crate::geometry::RectangleF;
use crate::models::ElementId;
use crate::shapes::{DrawingShape, ShapeKind};
use crate::svg::drawing_svg::DrawingSvg;
use crate::svg::drawing_svg_aspect::DrawingSvgAspect;
use crate::svg::drawing_svg_shape::DrawingSvgShape;

impl From<DrawingSvgShape> for DrawingShape {
    fn from(shape: DrawingSvgShape) -> DrawingShape {
        DrawingShape::new(ShapeKind::Svg(shape))
    }
}

impl DrawingShape {
    /// The SVG document element, if this element is one.
    pub fn as_svg(&self) -> Option<&DrawingSvgShape> {
        match self.kind() {
            ShapeKind::Svg(s) => Some(s),
            _ => None,
        }
    }

    /// The SVG document element, mutable (its parts, bindings and pictures), if this element is
    /// one.
    pub fn as_svg_mut(&mut self) -> Option<&mut DrawingSvgShape> {
        match self.kind_mut() {
            ShapeKind::Svg(s) => Some(s),
            _ => None,
        }
    }
}

impl DrawingSession {
    /// An SVG document on the active layer, fitted into `destination` (calibrated units) with
    /// `aspect` (C# `DrawSvg(svg, destination, aspect = Meet)`, which returns the shape; here
    /// the element's id - [`get_svg_mut`](DrawingSession::get_svg_mut) reaches the shape). It is
    /// drawn at the session's time: its part bindings evaluated, the document's own animations
    /// played. The document is shared, not copied.
    pub fn draw_svg(&mut self, svg: &Arc<DrawingSvg>, destination: RectangleF, aspect: DrawingSvgAspect) -> ElementId {
        self.draw_shape(DrawingSvgShape::new(svg.clone(), destination, aspect))
    }

    /// A prepared SVG shape (parts set, parts bound, pictures supplied) on the active layer;
    /// returns its id.
    pub fn draw_svg_shape(&mut self, shape: DrawingSvgShape) -> ElementId {
        self.draw_shape(shape)
    }

    /// The SVG element with this id, if there is one.
    pub fn get_svg(&self, id: ElementId) -> Option<&DrawingSvgShape> {
        self.find_element(id).and_then(|e| e.as_svg())
    }

    /// The SVG element with this id, mutable (C#'s shape object: its parts, bindings and
    /// pictures), if there is one.
    pub fn get_svg_mut(&mut self, id: ElementId) -> Option<&mut DrawingSvgShape> {
        self.find_element_mut(id).and_then(|e| e.as_svg_mut())
    }
}
