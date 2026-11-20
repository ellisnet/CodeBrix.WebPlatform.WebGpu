//! The session model of CodeBrix.Imaging.Drawing's `Models` namespace: layers, element
//! identities and transforms, the camera, and the animations - bindings of tracks to elements,
//! layers and the camera, combined bindings, staggers, trains and travel along paths.

mod animated_property;
mod animation_binding;
pub(crate) mod binding_combine;
mod binding_mode;
mod binding_target;
pub(crate) mod bound_track;
pub(crate) mod drawing_animations;
mod drawing_animations_motion;
mod drawing_animations_mut;
mod drawing_animations_text;
mod drawing_camera;
mod drawing_layer;
mod drawing_transform;
mod element_axes;
mod element_id;
mod path_train;
mod path_train_car;
mod path_train_options;
mod path_travel;
mod path_travel_options;
mod stagger_options;
mod stagger_order;
mod stagger_tracks;

#[cfg(test)]
mod stagger_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_step12;

pub use animated_property::AnimatedProperty;
pub use animation_binding::AnimationBinding;
pub use binding_mode::BindingMode;
pub use binding_target::BindingTarget;
pub use bound_track::BoundTrack;
pub use drawing_animations::DrawingAnimations;
pub use drawing_animations_mut::DrawingAnimationsMut;
pub use drawing_camera::DrawingCamera;
pub use drawing_layer::DrawingLayer;
pub use drawing_transform::{DrawingTransform, DrawingTransformOptions};
pub use element_axes::ElementAxes;
pub use element_id::ElementId;
pub use path_train::PathTrain;
pub use path_train_car::PathTrainCar;
pub use path_train_options::PathTrainOptions;
pub use path_travel_options::PathTravelOptions;
pub use stagger_options::StaggerOptions;
pub use stagger_order::StaggerOrder;
pub use stagger_tracks::{StaggerState, StaggerTracks};

/// The default stroke thickness, calibrated drawing units (C# `Stroke.DefaultWidth`).
pub const DEFAULT_STROKE_WIDTH: f32 = 15.0;
