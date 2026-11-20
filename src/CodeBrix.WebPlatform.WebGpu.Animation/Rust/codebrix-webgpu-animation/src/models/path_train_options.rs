//! `PathTrainOptions`: the settings of a train of elements along one path (C# `PathTrainOptions`).

use crate::animation::FloatTrack;
use crate::models::StaggerOrder;

/// The settings of a train (C# `PathTrainOptions`, the same defaults). THE RULE: car k (ranked
/// by `order`) is at distance `d_k = progress(t - rank x stagger) x L - rank x spacing x
/// spacing_scale(t)` along the path of length L (clamped to 0..L unless `extend_beyond_ends`); it
/// faces along the path with `orient_to_path`, rolls with a `roll_radius` (upright where it rests
/// at the spacing's last value), and is scaled and faded by `scale_along_path` /
/// `opacity_along_path` keyed by its OWN fraction of the path, d_k / L.
#[derive(Clone, Debug)]
pub struct PathTrainOptions {
    /// The gap between neighbouring cars, calibrated units, along the path (C# `Spacing`; 0).
    pub spacing: f32,
    /// A factor on the spacing over time - let it end at 0 to close the gaps at rest (C#
    /// `SpacingScale`; None = 1).
    pub spacing_scale: Option<FloatTrack>,
    /// Seconds each car follows the progress later than the one ahead (C# `Stagger`; 0).
    pub stagger: f64,
    /// How the cars are ranked (C# `Order`; Forward: the first element leads).
    pub order: StaggerOrder,
    /// True to turn each car to face along the path (C# `OrientToPath`).
    pub orient_to_path: bool,
    /// Degrees added to each car's rotation (C# `RotationOffset`).
    pub rotation_offset: f32,
    /// True to continue straight past the path's ends instead of clamping (C# `ExtendBeyondEnds`).
    pub extend_beyond_ends: bool,
    /// Each car's rolling radius, calibrated units; 0 rolls not at all (C# `RollRadius`).
    pub roll_radius: f32,
    /// For text trains: each piece rolls on its own baseline (radius = baseline - anchor, at
    /// least 1) (C# `RollOnBaseline`).
    pub roll_on_baseline: bool,
    /// The progress at which the lead car is upright when rolling (C# `UprightAtProgress`; 1).
    pub upright_at_progress: f32,
    /// Each car's uniform scale by its own fraction of the path (C# `ScaleAlongPath`; None).
    pub scale_along_path: Option<FloatTrack>,
    /// Each car's opacity by its own fraction of the path, clamped to 0..1 (C#
    /// `OpacityAlongPath`; None).
    pub opacity_along_path: Option<FloatTrack>,
    /// True to move each car's transform anchor to the centre of its bounds first (C#
    /// `AnchorAtCenter`; true).
    pub anchor_at_center: bool,
    /// True to send each car along a copy of the path moved so that its END lies on the car's own
    /// anchor - every car arrives exactly where it was drawn (C# `SettleInPlace`; false).
    pub settle_in_place: bool,
}

impl Default for PathTrainOptions {
    fn default() -> PathTrainOptions {
        PathTrainOptions {
            spacing: 0.0,
            spacing_scale: None,
            stagger: 0.0,
            order: StaggerOrder::Forward,
            orient_to_path: false,
            rotation_offset: 0.0,
            extend_beyond_ends: false,
            roll_radius: 0.0,
            roll_on_baseline: false,
            upright_at_progress: 1.0,
            scale_along_path: None,
            opacity_along_path: None,
            anchor_at_center: true,
            settle_in_place: false,
        }
    }
}

impl PathTrainOptions {
    /// C#'s `Validate`: the error says which setting is out of range.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !self.spacing.is_finite() || self.spacing < 0.0 {
            return Err("A train's spacing must be a finite distance, 0 or more.".to_string());
        }
        if !self.stagger.is_finite() || self.stagger < 0.0 {
            return Err("A train's stagger must be a finite number of seconds, 0 or more.".to_string());
        }
        if !self.rotation_offset.is_finite() || !self.roll_radius.is_finite() || !self.upright_at_progress.is_finite() {
            return Err("A train's rotation offset, roll radius and upright progress must be finite numbers.".to_string());
        }
        Ok(())
    }
}
