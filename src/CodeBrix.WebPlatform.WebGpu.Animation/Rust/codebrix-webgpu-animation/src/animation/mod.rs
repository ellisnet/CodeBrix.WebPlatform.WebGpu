//! The animation vocabulary of CodeBrix.Imaging.Drawing's `Animation` namespace: easing curves,
//! colour ramps, keyframe tracks (float, point, colour, bool) with per-segment easing, hold keys
//! and loop modes, timelines with cues and the playback controller. All pure functions of the time
//! they are given - nothing here reads a clock.

mod animation_timeline;
mod color_ramp;
mod cubic_bezier_easing;
mod easing;
mod easing_kind;
mod keyframe;
mod keyframe_track;
mod playback_controller;
mod step_position;
mod timeline_cue;
mod track_loop_mode;
mod track_value;

#[cfg(test)]
mod tests;

pub use animation_timeline::AnimationTimeline;
pub use color_ramp::ColorRamp;
pub use cubic_bezier_easing::CubicBezierEasing;
pub use easing::Easing;
pub use easing_kind::EasingKind;
pub use keyframe::Keyframe;
pub use keyframe_track::{BoolTrack, ColorTrack, FloatTrack, KeyframeTrack, PointTrack};
pub use playback_controller::PlaybackController;
pub use step_position::StepPosition;
pub use timeline_cue::TimelineCue;
pub use track_loop_mode::TrackLoopMode;
pub use track_value::TrackValue;
