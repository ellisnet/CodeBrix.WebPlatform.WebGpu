//! `DrawingSession::check_safe_area` (C# `CheckSafeArea`): which checked elements reach into a
//! guide's unsafe zones at the session's time.

use kurbo::Affine;

use crate::drawing_session::DrawingSession;
use crate::geometry::RectangleF;
use crate::rendering::CanvasCalibration;
use crate::safe_areas::{SafeAreaCheckOptions, SafeAreaFinding, SafeAreaFindingKind, SafeAreaGuide};
use crate::shapes::{DrawingShape, ShapeKind};

/// C#'s element type names, for findings.
fn kind_name(shape: &DrawingShape) -> &'static str {
    match shape.kind() {
        ShapeKind::Rectangle(_) => "RectangleShape",
        ShapeKind::Circle(_) => "CircleShape",
        ShapeKind::Ellipse(_) => "EllipseShape",
        ShapeKind::Line(_) => "LineShape",
        ShapeKind::Arrow(_) => "ArrowShape",
        ShapeKind::Polyline(_) => "PolylineShape",
        ShapeKind::Path(_) => "PathShape",
        ShapeKind::Text(_) => "TextShape",
        ShapeKind::Group(_) => "DrawingGroup",
        ShapeKind::Animated(_) => "AnimatedShape",
        ShapeKind::BrushStroke(_) => "BrushStrokeShape",
        ShapeKind::Noise(_) => "NoiseShape",
        ShapeKind::Particles(_) => "ParticlesShape",
        ShapeKind::Ownership(_) => "OwnershipShape",
        ShapeKind::Image(_) => "ImageShape",
        ShapeKind::Sprite(_) => "SpriteShape",
        ShapeKind::TextBlock(_) => "TextBlockShape",
        ShapeKind::Svg(_) => "DrawingSvgShape",
        ShapeKind::Lottie(_) => "DrawingLottieShape",
    }
}

/// The axis-aligned box of a rectangle's corners through a matrix (C#'s static
/// `DrawingTransform.TransformBounds(bounds, matrix)`).
fn transform_bounds(bounds: RectangleF, matrix: Affine) -> RectangleF {
    let corners = [(bounds.x, bounds.y), (bounds.right(), bounds.y), (bounds.x, bounds.bottom()), (bounds.right(), bounds.bottom())].map(|(x, y)| matrix * kurbo::Point::new(x as f64, y as f64));
    let left = corners.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let top = corners.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let right = corners.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let bottom = corners.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    RectangleF::new(left as f32, top as f32, (right - left) as f32, (bottom - top) as f32)
}

/// System.Drawing's `RectangleF.Intersect` (empty when they do not meet).
fn intersect(a: RectangleF, b: RectangleF) -> RectangleF {
    let x1 = a.x.max(b.x);
    let x2 = a.right().min(b.right());
    let y1 = a.y.max(b.y);
    let y2 = a.bottom().min(b.bottom());
    if x2 >= x1 && y2 >= y1 {
        RectangleF::new(x1, y1, x2 - x1, y2 - y1)
    } else {
        RectangleF::default()
    }
}

/// An element's bounds as the frame sees them (C#'s `GetElementBounds`): its own bounds (a
/// group's of the children that exist at the time), through its transform, then through its
/// layer's view (the camera as the layer feels it, then the layer's transform); None when they
/// are not known or not finite.
fn element_bounds(element: &DrawingShape, time: f64, view: Affine) -> Option<RectangleF> {
    let bounds = match element.kind() {
        ShapeKind::Group(_) => element.get_transformed_bounds_at(Some(time)),
        _ => element.get_transformed_bounds(),
    }?;
    let b = if view == Affine::IDENTITY { bounds } else { transform_bounds(bounds, view) };
    if !b.x.is_finite() || !b.y.is_finite() || !b.width.is_finite() || !b.height.is_finite() || b.width < 0.0 || b.height < 0.0 {
        return None;
    }
    Some(b)
}

impl DrawingSession {
    /// Which elements reach into a guide's UNSAFE ZONES in a frame of `frame_size` pixels (None:
    /// the calibration size, C#'s `DefaultExportSize` without a background image) at the session's
    /// time (C# `CheckSafeArea(guide, frameSize, options)`): one [`SafeAreaFinding`] per element
    /// and zone it overlaps, with the overlap in calibrated units and in frame pixels. Checked are
    /// the elements flagged `is_safe_area_checked` and every element of a layer so flagged (or all,
    /// with `check_all_elements`); hidden layers and elements outside their time window are
    /// skipped. A preview aid: drawing the guide's overlay is the renderer's.
    pub fn check_safe_area(&self, guide: &SafeAreaGuide, frame_size: Option<(u32, u32)>, options: SafeAreaCheckOptions) -> Vec<SafeAreaFinding> {
        let mut findings = Vec::new();
        let calibration = self.calibration_size();
        let (fw, fh) = frame_size.unwrap_or(calibration);
        if fw < 1 || fh < 1 {
            return findings;
        }
        let time = self.time();
        let zones = guide.get_unsafe_zones((fw as f32, fh as f32), Some(time), options.duration);
        let drawing_rect = CanvasCalibration::get_drawing_rect(fw as f32, fh as f32, calibration);
        let scale = drawing_rect.width / calibration.0 as f32;
        if !(scale > 0.0) {
            return findings;
        }
        for layer in self.layers() {
            if !layer.is_visible() {
                continue;
            }
            let camera = self.camera().map_or(Affine::IDENTITY, |c| c.get_matrix(calibration, layer.parallax()));
            let view = camera * layer.transform().to_matrix();
            for (i, element) in layer.get_elements().iter().enumerate() {
                if !options.check_all_elements && !layer.is_safe_area_checked() && !element.is_safe_area_checked() {
                    continue;
                }
                if !element.is_visible_at(time) {
                    continue;
                }
                let Some(b) = element_bounds(element, time, view) else {
                    if options.report_unchecked_elements {
                        findings.push(SafeAreaFinding { kind: SafeAreaFindingKind::CouldNotCheck, layer_name: layer.name().to_string(), element: element.id(), element_kind: kind_name(element), element_index: i, zone: None, overlap: None, frame_overlap: None });
                    }
                    continue;
                };
                let frame_bounds = RectangleF::new(drawing_rect.x + (b.x * scale), drawing_rect.y + (b.y * scale), b.width * scale, b.height * scale);
                for zone in &zones {
                    let overlap = intersect(frame_bounds, zone.bounds());
                    if overlap.width <= 0.0 || overlap.height <= 0.0 {
                        continue;
                    }
                    let calibrated = RectangleF::new((overlap.x - drawing_rect.x) / scale, (overlap.y - drawing_rect.y) / scale, overlap.width / scale, overlap.height / scale);
                    findings.push(SafeAreaFinding {
                        kind: SafeAreaFindingKind::Overlap,
                        layer_name: layer.name().to_string(),
                        element: element.id(),
                        element_kind: kind_name(element),
                        element_index: i,
                        zone: Some(zone.clone()),
                        overlap: Some(calibrated),
                        frame_overlap: Some(overlap),
                    });
                }
            }
        }
        findings
    }
}
