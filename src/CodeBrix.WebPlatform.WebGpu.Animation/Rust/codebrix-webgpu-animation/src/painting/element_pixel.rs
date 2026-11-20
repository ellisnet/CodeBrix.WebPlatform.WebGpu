//! `ElementPixel`: one pixel of an ownership map, measured in its owner's own axes (C#
//! `ElementPixel`).

use crate::models::ElementAxes;

/// One pixel of an ownership map, measured in its owning element's own axes - what a brush
/// texture function receives (C# `ElementPixel`; see
/// [`DrawingOwnershipMap::fill_by_element`](crate::painting::DrawingOwnershipMap::fill_by_element)).
/// The pixel's CENTRE is taken back through the element's full transform (output fit, layer
/// view, groups, the element's own transform) into the element's own coordinates, then measured
/// from the axes' centre: `along` on the direction `(cos angle, sin angle)` and `across` on
/// `(-sin angle, cos angle)` (to the right of travel on screen, y down), both in the element's own
/// calibrated units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElementPixel {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) element_index: usize,
    pub(crate) along: f64,
    pub(crate) across: f64,
    pub(crate) axes: ElementAxes,
}

impl ElementPixel {
    /// The pixel's column in the map (C# `X`).
    pub fn x(&self) -> u32 {
        self.x
    }

    /// The pixel's row in the map (C# `Y`).
    pub fn y(&self) -> u32 {
        self.y
    }

    /// The owning element's index in the map (C# `ElementIndex`).
    pub fn element_index(&self) -> usize {
        self.element_index
    }

    /// How far along the element's direction the pixel centre lies from its axes' centre,
    /// element units (C# `Along`).
    pub fn along(&self) -> f64 {
        self.along
    }

    /// How far across the element's direction the pixel centre lies from its axes' centre,
    /// element units (C# `Across`).
    pub fn across(&self) -> f64 {
        self.across
    }

    /// The owning element's axes (C# `Axes`).
    pub fn axes(&self) -> ElementAxes {
        self.axes
    }

    /// `along` divided by half the axes' length: -1 at one end, 0 at the centre, +1 at the other
    /// end (0 when the length is 0) (C# `NormalizedAlong`).
    pub fn normalized_along(&self) -> f64 {
        if self.axes.length() > 0.0 {
            self.along / (self.axes.length() as f64 / 2.0)
        } else {
            0.0
        }
    }

    /// `across` divided by half the axes' width: -1 at one side, +1 at the other (0 when the width
    /// is 0) (C# `NormalizedAcross`).
    pub fn normalized_across(&self) -> f64 {
        if self.axes.width() > 0.0 {
            self.across / (self.axes.width() as f64 / 2.0)
        } else {
            0.0
        }
    }
}
