//! `PathTravelOptions`: how an element travels along a path (C# `PathTravelOptions`).

/// How [`DrawingAnimationsMut::bind_along_path`](crate::models::DrawingAnimationsMut::bind_along_path)
/// moves an element (C# `PathTravelOptions`): facing along the path, ROLLING like a wheel of
/// `roll_radius` (upright at `upright_at_progress`; the roll ADDS to the facing rotation), and
/// continuing straight past the path's ends for progress outside 0..1 (`extend_beyond_ends`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathTravelOptions {
    /// True to turn the element to face along the path (its rotation the path's direction there).
    pub orient_to_path: bool,
    /// Degrees added to the facing rotation (for content drawn facing another way than right).
    pub rotation_offset: f32,
    /// True to continue straight along the end tangents for progress below 0 or above 1 (an
    /// overshoot-and-settle) instead of clamping (C# `ExtendBeyondEnds`).
    pub extend_beyond_ends: bool,
    /// The rolling radius, hub to ground, calibrated units: 0 rolls not at all; negative rolls the
    /// other way (C# `RollRadius`).
    pub roll_radius: f32,
    /// The progress at which a rolling element is upright (C# `UprightAtProgress`; 1, the path's
    /// end, by default).
    pub upright_at_progress: f32,
}

impl Default for PathTravelOptions {
    fn default() -> PathTravelOptions {
        PathTravelOptions { orient_to_path: false, rotation_offset: 0.0, extend_beyond_ends: false, roll_radius: 0.0, upright_at_progress: 1.0 }
    }
}
