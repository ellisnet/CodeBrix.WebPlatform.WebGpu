//! `SafeAreaFinding`: one element reaching into an unsafe zone, or one that could not be checked
//! (C# `SafeAreaFinding`).

use std::fmt;

use crate::geometry::RectangleF;
use crate::models::ElementId;
use crate::safe_areas::{SafeAreaFindingKind, SafeAreaZone};

/// One finding of [`DrawingSession::check_safe_area`](crate::DrawingSession::check_safe_area)
/// (C# `SafeAreaFinding`): the layer (by name) and element (by id and index on the layer), the
/// zone it reaches into, and the overlap in calibrated units and in frame pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct SafeAreaFinding {
    pub(crate) kind: SafeAreaFindingKind,
    pub(crate) layer_name: String,
    pub(crate) element: ElementId,
    pub(crate) element_kind: &'static str,
    pub(crate) element_index: usize,
    pub(crate) zone: Option<SafeAreaZone>,
    pub(crate) overlap: Option<RectangleF>,
    pub(crate) frame_overlap: Option<RectangleF>,
}

impl SafeAreaFinding {
    /// What it says (C# `Kind`).
    pub fn kind(&self) -> SafeAreaFindingKind {
        self.kind
    }

    /// The layer's name (C# `Layer`).
    pub fn layer_name(&self) -> &str {
        &self.layer_name
    }

    /// The element (C# `Element`).
    pub fn element(&self) -> ElementId {
        self.element
    }

    /// The element's index on its layer (C# `ElementIndex`).
    pub fn element_index(&self) -> usize {
        self.element_index
    }

    /// The zone (None when it could not be checked) (C# `Zone`).
    pub fn zone(&self) -> Option<&SafeAreaZone> {
        self.zone.as_ref()
    }

    /// The guide's name (C# `GuideName`).
    pub fn guide_name(&self) -> Option<&str> {
        self.zone.as_ref().map(|z| z.guide_name())
    }

    /// The zone's name (C# `ZoneName`).
    pub fn zone_name(&self) -> Option<&str> {
        self.zone.as_ref().map(|z| z.zone_name())
    }

    /// The overlap, calibrated units (C# `Overlap`).
    pub fn overlap(&self) -> Option<RectangleF> {
        self.overlap
    }

    /// The overlap, frame pixels (C# `FrameOverlap`).
    pub fn frame_overlap(&self) -> Option<RectangleF> {
        self.frame_overlap
    }
}

impl fmt::Display for SafeAreaFinding {
    /// C#'s `ToString`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.kind, &self.zone, self.overlap) {
            (SafeAreaFindingKind::Overlap, Some(z), Some(o)) => write!(f, "{} #{} ({}): in {} {}, overlap {{X={},Y={},Width={},Height={}}}", self.layer_name, self.element_index, self.element_kind, z.guide_name(), z.zone_name(), o.x, o.y, o.width, o.height),
            _ => write!(f, "{} #{} ({}): could not check (bounds unknown)", self.layer_name, self.element_index, self.element_kind),
        }
    }
}
