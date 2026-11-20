//! Drawn elements as the areas they cover in OUTPUT pixels, for the hard-edged mask and the
//! ownership map (internal; C#'s `CoverageCollector`): an element counts when it would draw at
//! the time (inside its time window, opacity above 0 - the renderer's skip rules); its geometry
//! is mapped by its own transform, its groups' transforms, the layer view (camera with parallax,
//! then the layer transform) and the calibrated-to-output fit, composed in double. Paint is
//! ignored: colours and their alpha, fills, effects and masks do not change what an element
//! covers.

use kurbo::Affine;

use crate::drawing_session::DrawingSession;
use crate::models::DrawingLayer;
use crate::paths::PathGeometry;
use crate::rendering::hard_edge_rasterizer::{self, HardEdgeMatrix};
use crate::rendering::CanvasCalibration;
use crate::shapes::{DrawingShape, ShapeKind};

/// How closely curves are followed, in OUTPUT pixels (C# `FrameTolerance`).
pub(crate) const FRAME_TOLERANCE: f64 = 0.1;

/// One covered area: a geometry, the map from its coordinates to the output, and the
/// flattening tolerance in its own units.
pub(crate) struct Area {
    pub(crate) geometry: PathGeometry,
    pub(crate) matrix: HardEdgeMatrix,
    pub(crate) tolerance: f32,
}

/// The map from a layer's element coordinates to an output of this size (C#'s
/// `GetCoverageLayerMatrix`): the layer's view (its transform, then the camera as the layer
/// feels it), then the fit the drawing occupies (scale, then the drawing rectangle's offset).
pub(crate) fn layer_matrix(session: &DrawingSession, layer: &DrawingLayer, width: u32, height: u32) -> HardEdgeMatrix {
    let calibration = session.calibration_size();
    let rect = CanvasCalibration::get_drawing_rect(width as f32, height as f32, calibration);
    // C#'s SKRect keeps right = left + width, and its Width is right - left.
    let scale = ((rect.x + rect.width) - rect.x) / calibration.0 as f32;
    let camera = session.camera().map_or(Affine::IDENTITY, |c| c.get_matrix(calibration, layer.parallax()));
    let view = camera * layer.transform().to_matrix();
    HardEdgeMatrix::from_affine(view).then(&HardEdgeMatrix::new(scale as f64, 0.0, 0.0, scale as f64, rect.x as f64, rect.y as f64))
}

/// The element's own coordinates -> output: its transform, then its parent's map (C#
/// `OwnMatrix`).
pub(crate) fn own_matrix(element: &DrawingShape, parent: &HardEdgeMatrix) -> HardEdgeMatrix {
    let transform = element.transform();
    if transform.is_identity() {
        *parent
    } else {
        HardEdgeMatrix::from_affine(transform.to_matrix()).then(parent)
    }
}

/// A curve tolerance in the element's own units that keeps the flattening within
/// [`FRAME_TOLERANCE`] output pixels (0 when the map collapses everything) (C# `ToleranceFor`).
pub(crate) fn tolerance_for(matrix: &HardEdgeMatrix) -> f32 {
    let stretch = matrix.max_stretch();
    if !(stretch > 0.0) || !stretch.is_finite() {
        return 0.0;
    }
    let tolerance = FRAME_TOLERANCE / stretch;
    tolerance.clamp(1e-6, 1e6) as f32
}

/// True when the renderer skips an element of this opacity: its alpha 255 scaled by the
/// opacity rounds to 0 (C#'s `ScaleAlpha(255, opacity) == 0`).
pub(crate) fn is_transparent(opacity: f32) -> bool {
    opacity < 1.0 && ((255.0f32 * opacity) as f64).round() == 0.0
}

/// The areas one element covers at a time, given the map from its PARENT's coordinates (C#
/// `Collect`). A group's or an animated shape's children are collected one by one (their own
/// time windows and opacities count).
pub(crate) fn collect(element: &DrawingShape, parent: &HardEdgeMatrix, time: f64, areas: &mut Vec<Area>) -> Result<(), String> {
    if !element.is_visible_at(time) || is_transparent(element.opacity()) {
        return Ok(());
    }
    let matrix = own_matrix(element, parent);
    match element.kind() {
        ShapeKind::Animated(animated) => {
            if let Some(current) = animated.current() {
                collect(current, &matrix, time, areas)?;
            }
        }
        ShapeKind::Group(group) => {
            for child in group.get_children() {
                collect(child, &matrix, time, areas)?;
            }
        }
        _ => {
            let tolerance = tolerance_for(&matrix);
            if !(tolerance > 0.0) {
                return Ok(());
            }
            for geometry in element.get_coverage_areas(tolerance)? {
                if !geometry.is_empty() {
                    areas.push(Area { geometry, matrix, tolerance });
                }
            }
        }
    }
    Ok(())
}

/// Fills the covered pixels of the areas through the writer (C# `Rasterize`).
pub(crate) fn rasterize(areas: &[Area], width: u32, height: u32, writer: &mut dyn FnMut(i32, i32, i32)) -> Result<(), String> {
    for area in areas {
        if area.geometry.is_empty() {
            continue;
        }
        let polylines = area.geometry.flatten(Some(area.tolerance))?;
        hard_edge_rasterizer::fill(&polylines, &area.matrix, area.geometry.fill_rule(), width as i32, height as i32, writer);
    }
    Ok(())
}
