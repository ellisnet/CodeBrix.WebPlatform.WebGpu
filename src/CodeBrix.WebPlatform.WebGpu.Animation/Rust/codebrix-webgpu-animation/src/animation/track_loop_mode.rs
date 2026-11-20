//! `TrackLoopMode`: what a keyframe track does outside its keys (C# `TrackLoopMode`).

/// What a [`KeyframeTrack`](crate::animation::KeyframeTrack) does before its first key and after
/// its last (C# `TrackLoopMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TrackLoopMode {
    /// The first key's value before it, the last key's after it (the default).
    #[default]
    Hold = 0,
    /// The keys repeat from the first after the last.
    Loop = 1,
    /// The keys play forwards, then backwards, and so on.
    PingPong = 2,
}
