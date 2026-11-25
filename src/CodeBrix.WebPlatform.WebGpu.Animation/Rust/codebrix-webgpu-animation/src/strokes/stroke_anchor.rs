//! `StrokeAnchor`: what a stroke set's strokes move with - a fixed transform, or the current
//! transform of an element of the session - and the CPU side of resolving and blending anchors
//! (no C# equivalent).

use kurbo::Affine;

use crate::models::{DrawingLayer, DrawingTransform, ElementId};

/// An anchor of a [`BrushStrokesShape`](crate::shapes::BrushStrokesShape): a transform from the
/// set's own coordinates into the shape's, evaluated every frame at the session's time. A record
/// names up to two anchors (`anchors`) and blends between them (`anchor_blend`: the two
/// transforms mixed linearly, as skinning does). An anchor index the shape has no anchor for is
/// the identity; a shape with no anchors draws every stroke in the set's coordinates.
#[derive(Clone, Debug, PartialEq)]
pub enum StrokeAnchor {
    /// A fixed transform (the identity leaves the strokes where the set put them).
    Fixed(DrawingTransform),
    /// The CURRENT transform of an element of the session - its own
    /// [`DrawingTransform`] at the session's time, bindings applied: strokes placed in that
    /// element's own coordinates move as its children would. Put the stroke shape at the same
    /// level as the element (the same layer, or the same group) so that their parents agree. Only
    /// the element's own transform is followed, not its parents' nor changes of its geometry. An
    /// id the session does not have is reported once (`DrawingSession::take_reports`) and acts as
    /// the identity.
    FollowElement(ElementId),
}

/// The anchors' transforms for this frame: a `Fixed` anchor's matrix, a followed element's own
/// transform found in the session's layers (groups searched), the identity for an id the session
/// does not have (reported through `report`).
pub(crate) fn resolve_anchors(anchors: &[StrokeAnchor], layers: &[DrawingLayer], report: &mut dyn FnMut(String)) -> Vec<Affine> {
    anchors
        .iter()
        .map(|a| match a {
            StrokeAnchor::Fixed(t) => t.to_matrix(),
            StrokeAnchor::FollowElement(id) => match layers.iter().find_map(|l| l.get_elements().iter().find_map(|e| e.find(*id))) {
                Some(element) => element.transform().to_matrix(),
                None => {
                    report(format!("A stroke anchor follows the element {id:?}, which the session does not have: anchor the strokes to an element of the session (StrokeAnchor::FollowElement) or use StrokeAnchor::Fixed."));
                    Affine::IDENTITY
                }
            },
        })
        .collect()
}

/// The anchor of an index (the identity past the list).
pub(crate) fn anchor_at(anchors: &[Affine], index: u16) -> Affine {
    anchors.get(index as usize).copied().unwrap_or(Affine::IDENTITY)
}

/// Two anchors mixed linearly (each coefficient), `blend` 0 = the first only .. 1 = the second
/// (the stroke shader does the same).
pub(crate) fn blend(first: Affine, second: Affine, blend: f32) -> Affine {
    let (a, b) = (first.as_coeffs(), second.as_coeffs());
    let t = blend.clamp(0.0, 1.0) as f64;
    let mut c = [0.0; 6];
    for i in 0..6 {
        c[i] = a[i] + (b[i] - a[i]) * t;
    }
    Affine::new(c)
}

#[cfg(test)]
/// The CPU mirror of where the stroke shader puts a record: its centre and its direction
/// (degrees clockwise on screen from +x) after its two anchors, blended.
pub(crate) fn anchored(record: &crate::strokes::BrushStrokeRecord, anchors: &[Affine]) -> (crate::geometry::PointF, f32) {
    let m = blend(anchor_at(anchors, record.anchors[0]), anchor_at(anchors, record.anchors[1]), record.anchor_blend);
    let c = m * kurbo::Point::new(record.center.x as f64, record.center.y as f64);
    let r = (record.angle as f64).to_radians();
    let k = m.as_coeffs();
    let (dx, dy) = (k[0] * r.cos() + k[2] * r.sin(), k[1] * r.cos() + k[3] * r.sin());
    (crate::geometry::PointF::new(c.x as f32, c.y as f32), dy.atan2(dx).to_degrees() as f32)
}
