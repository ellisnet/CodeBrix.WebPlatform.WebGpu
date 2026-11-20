//! SAFE AREAS - a PREVIEW AID for animations that will be rendered to video for social networks
//! or broadcast (C# namespace `CodeBrix.Imaging.Drawing.SafeAreas`): [`SafeAreaGuide`]s of the
//! margins a platform's interface covers (with NoSkia's presets in [`SafeAreaGuides`] and their
//! sources), crops ([`SafeAreaCrop`]) and time rules ([`SafeAreaTimeRule`]), the unsafe
//! [`SafeAreaZone`]s and the safe rectangle of a frame, the preview [`SafeAreaOverlay`]'s
//! settings (its drawing is the renderer's), and
//! [`DrawingSession::check_safe_area`](crate::DrawingSession::check_safe_area), which reports
//! every checked element that reaches into an unsafe zone ([`SafeAreaFinding`]). Elements are
//! checked when they (or their layer) are flagged `is_safe_area_checked`. Captions are placed
//! against the FRAME by default, not inside a guide (a deliberate difference from NoSkia).

mod safe_area_check;
mod safe_area_check_options;
mod safe_area_crop;
mod safe_area_finding;
mod safe_area_finding_kind;
mod safe_area_guide;
mod safe_area_guides;
mod safe_area_margins;
mod safe_area_overlay;
mod safe_area_time_rule;
mod safe_area_zone;

#[cfg(test)]
mod tests;

pub use safe_area_check_options::SafeAreaCheckOptions;
pub use safe_area_crop::SafeAreaCrop;
pub use safe_area_finding::SafeAreaFinding;
pub use safe_area_finding_kind::SafeAreaFindingKind;
pub use safe_area_guide::{SafeAreaGuide, SafeAreaGuideOptions};
pub use safe_area_guides::SafeAreaGuides;
pub use safe_area_margins::SafeAreaMargins;
pub use safe_area_overlay::{SafeAreaOverlay, SafeAreaOverlayPlan};
pub use safe_area_time_rule::SafeAreaTimeRule;
pub use safe_area_zone::SafeAreaZone;
