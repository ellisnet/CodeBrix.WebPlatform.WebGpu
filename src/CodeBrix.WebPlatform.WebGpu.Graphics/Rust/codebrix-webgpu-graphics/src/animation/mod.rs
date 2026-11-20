//! Animation playback on model instances (C#: `Animation/` - `AnimationPlayer`, `AnimationState`
//! and the internal channel sampling; `AnimationLoopMode`). Pure CPU work: a player writes the
//! instance's pose (local transforms and morph weights), and the model renderer draws it. Time
//! always comes from the frame; nothing here reads a clock.

mod animation_loop_mode;
mod animation_player;
pub(crate) mod animation_sampling;
mod animation_state;

pub use animation_loop_mode::AnimationLoopMode;
pub use animation_player::{AnimationPlayer, PlayOptions};
pub(crate) use animation_player::{PlayerData, PoseSource, PoseTarget};
pub use animation_state::AnimationState;
