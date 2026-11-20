//! [`AnimationLoopMode`]: how a clip continues at its end (C#: `AnimationLoopMode`).

/// How playback continues at a clip's end (C#: `AnimationLoopMode`, which the C# library shares
/// with animated textures).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnimationLoopMode {
    /// Start again from the beginning, forever (the default).
    #[default]
    Loop,
    /// Play forwards, then backwards, then forwards again.
    PingPong,
    /// Play once and hold the last frame.
    Once,
}
