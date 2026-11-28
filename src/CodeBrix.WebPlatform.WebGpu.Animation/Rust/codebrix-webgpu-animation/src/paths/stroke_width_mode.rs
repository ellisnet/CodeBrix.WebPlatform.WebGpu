//! `StrokeWidthMode`: whether a stroke's width scales with its element, stays fixed in the
//! drawing's own units, or is given in CSS pixels (Not in C#: NoSkia's strokes always scale with
//! their element).

/// How a stroke's width (and its dashes) responds to the scaling of its element.
///
/// - [`ScalesWithElement`](StrokeWidthMode::ScalesWithElement) (THE DEFAULT): the stroke is
///   expanded in the element's own space, so every transform between the element and the canvas
///   scales it - an element scaled up 3 times draws a line 3 times as wide, and an uneven scale
///   gives an uneven line.
/// - [`Fixed`](StrokeWidthMode::Fixed): the width is in the DRAWING'S OWN UNITS (the session's
///   calibrated units), whatever the element's transform: the scaling applied to the element
///   inside the drawing - its own transform (animated scale included), the transforms of the
///   groups it is in and its layer's transform - is ignored, and an uneven scale still gives an
///   even line. The view of the drawing still applies: the calibration (the canvas size) and the
///   session's camera scale it as they scale everything. The dash pattern and dash offset
///   follow the width (drawing units), not the element. SVG's
///   `vector-effect="non-scaling-stroke"` is read as this mode.
/// - [`CssPixels`](StrokeWidthMode::CssPixels): the width is in CSS PIXELS of the page - a
///   hairline of 1 is one CSS pixel on any screen: `width x pixel_ratio` device pixels, the ratio
///   of [`SessionRenderOptions::pixel_ratio`](crate::SessionRenderOptions::pixel_ratio) (pass
///   the frame's `frame.info.pixel_ratio`; 1 by default), whatever the calibration, the camera
///   and every transform between the element and the canvas. Recorded as a fixed-width stroke
///   whose reference space is the CSS pixel; the dash pattern and dash offset are CSS pixels
///   too. Honoured by path shapes, text blocks' outlines and the line look's stroke
///   ([`SessionPaint::Line`](crate::SessionPaint::Line): one CSS-pixel line weight over the
///   whole drawing, whatever its zoom).
///
/// Not in C#.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StrokeWidthMode {
    /// The width scales with the element (the default; what NoSkia does).
    #[default]
    ScalesWithElement,
    /// The width stays fixed in the drawing's own (calibrated) units.
    Fixed,
    /// The width is in CSS pixels: `width x pixel_ratio` device pixels, whatever scales the
    /// drawing or the element.
    CssPixels,
}
