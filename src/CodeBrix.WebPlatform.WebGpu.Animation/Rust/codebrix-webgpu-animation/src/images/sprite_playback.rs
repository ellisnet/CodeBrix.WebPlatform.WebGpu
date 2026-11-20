//! `SpritePlayback` (C# `SpritePlayback`, the same members and numbers).

/// How a [`SpriteShape`](crate::shapes::SpriteShape) maps its running frame number onto the
/// frames of its sheet (C# `SpritePlayback`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpritePlayback {
    /// Repeats the frames in order forever (0, 1, 2, 0, 1, 2, ...), in both directions of time.
    #[default]
    Loop = 0,
    /// Plays the frames forward then back without repeating the ends (0, 1, 2, 1, 0, 1, ...).
    PingPong = 1,
    /// Plays the frames once and holds the last one (and the first one before the start).
    Hold = 2,
}

impl SpritePlayback {
    /// The frame index (0 to `count` - 1) of the running frame number `n` (C#'s
    /// `SpriteShape.MapFrame`).
    pub(crate) fn map_frame(self, n: i64, count: usize) -> usize {
        if count <= 1 {
            return 0;
        }
        let count = count as i64;
        match self {
            SpritePlayback::Hold => n.clamp(0, count - 1) as usize,
            SpritePlayback::PingPong => {
                let period = 2 * count - 2;
                let m = n.rem_euclid(period);
                (if m < count { m } else { period - m }) as usize
            }
            SpritePlayback::Loop => n.rem_euclid(count) as usize,
        }
    }
}
