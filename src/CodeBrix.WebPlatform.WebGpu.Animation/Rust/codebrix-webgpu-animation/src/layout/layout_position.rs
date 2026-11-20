//! `LayoutPosition`: an anchor of the frame or safe area, moved by units and by fractions (C#
//! `LayoutPosition`).

use crate::geometry::PointF;
use crate::layout::{LayoutAnchor, LayoutReference};

/// The optional arguments of C#'s `LayoutPosition` constructor: what it is measured against and
/// how far it is moved, in calibrated units and in fractions of the reference's size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutPositionOptions {
    /// The frame (default) or the safe area.
    pub reference: LayoutReference,
    /// Units right.
    pub offset_x: f32,
    /// Units down.
    pub offset_y: f32,
    /// Fractions of the reference's width, right.
    pub fraction_x: f32,
    /// Fractions of the reference's height, down.
    pub fraction_y: f32,
}

/// A place in a layout (C# `LayoutPosition`): an anchor of the frame or the safe area, moved by an
/// offset in units and an offset in fractions of that rectangle's size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutPosition {
    anchor: LayoutAnchor,
    reference: LayoutReference,
    offset: PointF,
    fraction_offset: PointF,
}

impl LayoutPosition {
    /// A position (C# `LayoutPosition(anchor, reference = Frame, offsetX = 0, offsetY = 0,
    /// fractionX = 0, fractionY = 0)`). The error says which value is not finite.
    pub fn new(anchor: LayoutAnchor, options: LayoutPositionOptions) -> Result<LayoutPosition, String> {
        if !options.offset_x.is_finite() || !options.offset_y.is_finite() {
            return Err("A layout position's offsets must be finite numbers.".to_string());
        }
        if !options.fraction_x.is_finite() || !options.fraction_y.is_finite() {
            return Err("A layout position's fractions must be finite numbers.".to_string());
        }
        Ok(LayoutPosition { anchor, reference: options.reference, offset: PointF::new(options.offset_x, options.offset_y), fraction_offset: PointF::new(options.fraction_x, options.fraction_y) })
    }

    /// The anchor.
    pub fn anchor(&self) -> LayoutAnchor {
        self.anchor
    }

    /// What it is measured against.
    pub fn reference(&self) -> LayoutReference {
        self.reference
    }

    /// The offset in units.
    pub fn offset(&self) -> PointF {
        self.offset
    }

    /// The offset in fractions of the reference's size.
    pub fn fraction_offset(&self) -> PointF {
        self.fraction_offset
    }
}
