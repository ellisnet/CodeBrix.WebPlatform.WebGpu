//! What a shape COVERS and the frame it is measured in (C# `DrawingShape.GetCoverageAreas` and
//! `DrawingShape.GetAxes` with every built-in shape's override), for the hard-edged mask and the
//! ownership map.

use crate::geometry::{PointF, RectangleF};
use crate::models::ElementAxes;
use crate::paths::{PathGeometry, PathGeometryBuilder, PathStrokeStyle, PathStrokeStyleOptions};
use crate::rendering::hard_edge_rasterizer::HardEdgeMatrix;
use crate::shapes::{DrawingShape, ShapeKind};

/// The band a stroke of this style covers (C#'s `ToStrokeOutline`): NoSkia's stroker's PIECES
/// without their union (path combining's stroker) - filled non-zero they cover the same area as
/// the union, which the hard-edged pixel rule does not need (proved bit for bit against NoSkia).
fn stroke_band(geometry: &PathGeometry, style: &PathStrokeStyle, tolerance: f32) -> Result<PathGeometry, String> {
    crate::paths::path_clipper::stroke_band_pieces(geometry, style, tolerance as f64)
}

/// The area of an outline drawn with the standard outline paint - round caps and joins at the
/// shape's thickness (C#'s `OutlineArea`).
fn outline_area(geometry: &PathGeometry, thickness: f32, tolerance: f32) -> Result<Vec<PathGeometry>, String> {
    let style = PathStrokeStyle::new(thickness, PathStrokeStyleOptions::default())?;
    Ok(vec![stroke_band(geometry, &style, tolerance)?])
}

fn filled_or_outlined(outline: PathGeometry, is_filled: bool, thickness: f32, tolerance: f32) -> Result<Vec<PathGeometry>, String> {
    if is_filled {
        Ok(vec![outline])
    } else {
        outline_area(&outline, thickness, tolerance)
    }
}

impl DrawingShape {
    /// The AREA the shape covers when drawn, as fillable geometry in calibrated units before its
    /// own transform (C# `GetCoverageAreas`): a point is covered when it lies inside ANY of the
    /// geometries under that geometry's own fill rule. A filled shape reports its outline; an
    /// outlined shape the area its outline's thickness covers (the stroke turned into polygons,
    /// with the caps and joins it is drawn with); text its glyph outlines; a group every child's
    /// area through the child's transform (time windows and opacities ignored - the mask and
    /// the map apply them child by child); an animated shape its current shape's. Paint plays no
    /// part. `tolerance` is how closely curves are followed when a thickness is turned into
    /// polygons, calibrated units (positive). The error says what to do. A stroke set (Not in C#)
    /// reports every stroke WHOLE here; masks and ownership maps take its strokes as painted at the
    /// session's time.
    ///
    /// An outline's thickness is NoSkia's stroker's band (see the path-booleans step's
    /// `to_stroke_outline`).
    pub fn get_coverage_areas(&self, tolerance: f32) -> Result<Vec<PathGeometry>, String> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("The coverage tolerance must be a positive, finite distance.".to_string());
        }
        match self.kind() {
            ShapeKind::Rectangle(r) => {
                let rectangle = RectangleF::new(r.x, r.y, r.width, r.height);
                let outline = if r.corner_radius > 0.0 { PathGeometry::rounded_rectangle(rectangle, r.corner_radius) } else { PathGeometry::rectangle(rectangle) };
                filled_or_outlined(outline, r.is_filled, r.stroke_thickness, tolerance)
            }
            ShapeKind::Circle(c) => filled_or_outlined(PathGeometry::circle(PointF::new(c.center_x, c.center_y), c.radius), c.is_filled, c.stroke_thickness, tolerance),
            ShapeKind::Ellipse(e) => filled_or_outlined(PathGeometry::ellipse(PointF::new(e.center_x, e.center_y), e.radius_x, e.radius_y), e.is_filled, e.stroke_thickness, tolerance),
            ShapeKind::Line(l) => {
                let line = PathGeometryBuilder::new().move_to(l.x1, l.y1).line_to(l.x2, l.y2).build();
                outline_area(&line, l.stroke_thickness, tolerance)
            }
            ShapeKind::Arrow(a) => {
                let (h1, h2) = a.head_points();
                let outline = PathGeometryBuilder::new().move_to(a.x1, a.y1).line_to(a.x2, a.y2).move_to(h1.x, h1.y).line_to(a.x2, a.y2).line_to(h2.x, h2.y).build();
                outline_area(&outline, a.stroke_thickness, tolerance)
            }
            ShapeKind::Polyline(p) => {
                let outline = PathGeometryBuilder::new().add_polygon(&p.points, p.is_closed).build();
                filled_or_outlined(outline, p.is_filled, p.stroke_thickness, tolerance)
            }
            ShapeKind::Path(p) => {
                let drawn = p.get_drawn_geometry();
                let mut areas = Vec::with_capacity(2);
                if drawn.is_empty() {
                    return Ok(areas);
                }
                if p.is_filled {
                    areas.push(drawn.clone());
                }
                if let Some(style) = &p.stroke_style {
                    areas.push(stroke_band(&drawn, style, tolerance)?);
                }
                Ok(areas)
            }
            ShapeKind::Text(t) => Ok(vec![t.get_outline()]),
            ShapeKind::BrushStroke(b) => Ok(b.get_coverage_areas(tolerance)),
            // Without a time: every stroke whole (see coverage_areas_at for the session's time).
            ShapeKind::BrushStrokes(b) => Ok(b.coverage_areas_at(f64::INFINITY)),
            ShapeKind::Image(i) => Ok(vec![PathGeometry::rectangle(i.destination())]),
            ShapeKind::Noise(n) => Ok(vec![PathGeometry::rectangle(n.area())]),
            ShapeKind::Ownership(m) => Ok(vec![PathGeometry::rectangle(m.destination())]),
            ShapeKind::Particles(p) => Ok(vec![p.get_outline()]),
            ShapeKind::Sprite(s) => Ok(vec![PathGeometry::rectangle(s.destination())]),
            // C#'s DrawingSvgShape covers its whole drawn box (the document's own transparency is
            // not looked at); a Lottie animation (no C# counterpart) is treated the same way.
            ShapeKind::Svg(s) => Ok(vec![PathGeometry::rectangle(s.get_bounds())]),
            ShapeKind::Lottie(l) => Ok(vec![PathGeometry::rectangle(l.get_bounds())]),
            ShapeKind::TextBlock(t) => {
                // C#'s GetCoverageAreas ignores the outline's placement (centre / outside), as here.
                let glyphs = t.get_outline();
                let mut areas = Vec::with_capacity(2);
                if glyphs.is_empty() {
                    return Ok(areas);
                }
                if t.is_filled() {
                    areas.push(glyphs.clone());
                }
                if let Some(style) = t.stroke_style() {
                    areas.push(stroke_band(&glyphs, style, tolerance)?);
                }
                Ok(areas)
            }
            ShapeKind::Group(g) => {
                let mut areas = Vec::new();
                for child in g.get_children() {
                    areas.extend(child.transformed_coverage_areas(tolerance)?);
                }
                Ok(areas)
            }
            ShapeKind::Animated(a) => match a.current() {
                Some(current) => current.transformed_coverage_areas(tolerance),
                None => Ok(Vec::new()),
            },
        }
    }

    /// The areas at a session time (what masks and ownership maps use): a stroke set's painted
    /// strokes at that time ([`BrushStrokesShape::coverage_areas_at`](crate::shapes::BrushStrokesShape::coverage_areas_at)),
    /// every other shape's [`get_coverage_areas`](DrawingShape::get_coverage_areas). Not in C#.
    pub(crate) fn coverage_areas_at(&self, tolerance: f32, seconds: f64) -> Result<Vec<PathGeometry>, String> {
        match self.kind() {
            ShapeKind::BrushStrokes(b) => {
                if !tolerance.is_finite() || tolerance <= 0.0 {
                    return Err("The coverage tolerance must be a positive, finite distance.".to_string());
                }
                Ok(b.coverage_areas_at(seconds))
            }
            _ => self.get_coverage_areas(tolerance),
        }
    }

    /// The element's covered areas after its own transform, the tolerance divided by the
    /// transform's largest stretch so that it holds after the transform too (C#'s
    /// `DrawingGroup.TransformedAreas`; nothing when the transform collapses everything).
    pub(crate) fn transformed_coverage_areas(&self, tolerance: f32) -> Result<Vec<PathGeometry>, String> {
        let transform = self.transform();
        let matrix = HardEdgeMatrix::from_affine(transform.to_matrix());
        let stretch = matrix.max_stretch();
        if !(stretch > 0.0) || !stretch.is_finite() {
            return Ok(Vec::new());
        }
        let local = (tolerance as f64 / stretch) as f32;
        if !(local > 0.0) || !local.is_finite() {
            return Ok(Vec::new());
        }
        let areas = self.get_coverage_areas(local)?;
        if transform.is_identity() {
            return Ok(areas);
        }
        let affine = transform.to_matrix();
        Ok(areas.iter().map(|a| a.transformed(affine)).collect())
    }

    /// The shape's own frame of reference - centre, direction, length and width - in its own
    /// calibrated coordinates, which an ownership map measures each owned pixel in (C#
    /// `GetAxes`). A brushstroke, a line, a rectangle, an ellipse and a circle report one; every
    /// other shape None (the map then uses [`ElementAxes::from_bounds`] of its bounds).
    pub fn get_axes(&self) -> Option<ElementAxes> {
        match self.kind() {
            ShapeKind::BrushStroke(b) => Some(b.get_axes()),
            ShapeKind::Rectangle(r) => ElementAxes::new(PointF::new(r.x + (r.width / 2.0), r.y + (r.height / 2.0)), 0.0, r.width, r.height).ok(),
            ShapeKind::Circle(c) => ElementAxes::new(PointF::new(c.center_x, c.center_y), 0.0, 2.0 * c.radius, 2.0 * c.radius).ok(),
            ShapeKind::Ellipse(e) => ElementAxes::new(PointF::new(e.center_x, e.center_y), 0.0, 2.0 * e.radius_x, 2.0 * e.radius_y).ok(),
            ShapeKind::Line(l) => {
                // C# subtracts in single precision, then measures in double.
                let (dx, dy) = ((l.x2 - l.x1) as f64, (l.y2 - l.y1) as f64);
                let angle = (dy.atan2(dx) * (180.0 / std::f64::consts::PI)) as f32;
                ElementAxes::new(PointF::new((l.x1 + l.x2) / 2.0, (l.y1 + l.y2) / 2.0), angle, ((dx * dx) + (dy * dy)).sqrt() as f32, l.stroke_thickness).ok()
            }
            _ => None,
        }
    }
}
