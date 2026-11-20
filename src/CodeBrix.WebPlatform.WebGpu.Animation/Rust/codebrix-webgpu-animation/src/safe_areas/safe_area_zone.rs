//! `SafeAreaZone`: one unsafe rectangle of a frame (C# `SafeAreaZone`).

use crate::geometry::RectangleF;

/// One unsafe rectangle of a frame, in frame pixels, with the guide and zone it belongs to (C#
/// `SafeAreaZone`).
#[derive(Clone, Debug, PartialEq)]
pub struct SafeAreaZone {
    pub(crate) guide_name: String,
    pub(crate) zone_name: String,
    pub(crate) bounds: RectangleF,
}

impl SafeAreaZone {
    /// The guide's name (C# `GuideName`).
    pub fn guide_name(&self) -> &str {
        &self.guide_name
    }

    /// The zone's name ("top margin", a crop's name, "whole frame") (C# `ZoneName`).
    pub fn zone_name(&self) -> &str {
        &self.zone_name
    }

    /// The rectangle, frame pixels (C# `Bounds`).
    pub fn bounds(&self) -> RectangleF {
        self.bounds
    }
}
