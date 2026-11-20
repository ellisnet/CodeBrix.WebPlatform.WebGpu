//! `LottiePlayback`: what a playing Lottie does at the end of its range.

/// What playback does at the end of its range - the same three modes as the crate's
/// [`SpritePlayback`](crate::images::SpritePlayback) and the Graphics crate's animation loop
/// mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LottiePlayback {
    /// Starts the range again from its first frame, forever (the default).
    #[default]
    Loop,
    /// Plays the range forwards, then backwards, then forwards again.
    PingPong,
    /// Plays the range once and holds its last frame.
    Hold,
}
