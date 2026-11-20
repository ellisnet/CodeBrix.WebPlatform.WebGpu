//! `ElementAxes`: an element's own frame of reference - centre, direction, length and width (C#
//! `ElementAxes`).

use crate::geometry::{PointF, RectangleF};

/// An element's own frame of reference (C# `ElementAxes`) - a centre, a direction, and the
/// element's extent along and across that direction - in the element's own calibrated
/// coordinates (before its own transform). A brushstroke reports its centre line; a rectangle its
/// centre with the x axis. An ownership map measures every pixel an element owns in these axes
/// ("how far along the stroke, how far across it"), which is what bristle ridges,
/// along-the-stroke ramps and other brush textures are computed from. The direction is in
/// DEGREES, clockwise on screen from +x (y points down) - the unit of
/// [`DrawingTransform`](crate::models::DrawingTransform)'s rotation and of a path point's angle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ElementAxes {
    center: PointF,
    angle: f32,
    length: f32,
    width: f32,
}

impl ElementAxes {
    /// A frame of reference (C#'s constructor): the centre in the element's own coordinates, the
    /// direction of the "along" axis in degrees, the extent along it (end to end; 0 when it has
    /// none) and across it (side to side). The error says what to do: every value finite, the
    /// length and the width not negative.
    pub fn new(center: PointF, angle: f32, length: f32, width: f32) -> Result<ElementAxes, String> {
        if !center.x.is_finite() || !center.y.is_finite() {
            return Err("The axes' centre must be a finite point.".to_string());
        }
        if !angle.is_finite() {
            return Err("The axes' angle must be a finite number of degrees.".to_string());
        }
        if !length.is_finite() || length < 0.0 {
            return Err("The axes' length must be finite and not negative (0 for none).".to_string());
        }
        if !width.is_finite() || width < 0.0 {
            return Err("The axes' width must be finite and not negative (0 for none).".to_string());
        }
        Ok(ElementAxes { center, angle, length, width })
    }

    /// The axes of an axis-aligned box (C# `FromBounds`): centred on it, along +x, its width as
    /// the length and its height as the width - what an ownership map uses for an element that
    /// reports no axes of its own. A box with a non-finite value gives all zeros.
    pub fn from_bounds(bounds: RectangleF) -> ElementAxes {
        ElementAxes::new(PointF::new(bounds.x + (bounds.width / 2.0), bounds.y + (bounds.height / 2.0)), 0.0, bounds.width.max(0.0), bounds.height.max(0.0)).unwrap_or_default()
    }

    /// The origin, in the element's own calibrated coordinates (C# `Center`).
    pub fn center(&self) -> PointF {
        self.center
    }

    /// The direction of the "along" axis, degrees clockwise on screen from +x (C# `Angle`).
    pub fn angle(&self) -> f32 {
        self.angle
    }

    /// The element's extent along the axis, end to end (C# `Length`).
    pub fn length(&self) -> f32 {
        self.length
    }

    /// The element's extent across the axis, side to side (C# `Width`).
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Builds axes the caller has already checked (crate-internal).
    pub(crate) fn unchecked(center: PointF, angle: f32, length: f32, width: f32) -> ElementAxes {
        ElementAxes { center, angle, length, width }
    }
}
