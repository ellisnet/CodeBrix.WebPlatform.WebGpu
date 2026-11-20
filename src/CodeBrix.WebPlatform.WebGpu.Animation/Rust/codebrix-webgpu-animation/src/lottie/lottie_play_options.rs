//! `LottiePlayOptions`: how a document plays as a function of the session time.

use crate::lottie::lottie_playback::LottiePlayback;
use crate::lottie::lottie_range::LottieRange;

/// How far inside the range's end a held or ping-ponged last frame is shown (Lottie's out point
/// is exclusive: a layer whose out point is the range's end is gone AT that frame).
pub(crate) const LAST_FRAME_INSET: f64 = 1e-6;

/// How a document plays: which frames, how they repeat, how fast, and from when. The playing
/// frame is a FUNCTION OF THE SESSION TIME passed in - nothing reads a clock - so pausing,
/// speeding up and seeking the whole page happen at the host's clock, and seeking ONE animation
/// is a change of `begin_time` or `start_offset`.
///
/// The playhead at session time `t` is `(t - begin_time) * speed + start_offset` seconds into
/// the range (before `begin_time`: `start_offset`), mapped by `playback` and multiplied by the
/// document's frame rate.
#[derive(Clone, Debug, PartialEq)]
pub struct LottiePlayOptions {
    /// Loop (default), ping-pong or hold the last frame.
    pub playback: LottiePlayback,
    /// The playback rate; default 1. Negative plays backwards; 0 holds `start_offset`.
    pub speed: f64,
    /// The frames that play; default the whole document.
    pub range: LottieRange,
    /// The session time, in seconds, at which playback begins; default 0. Before it the frame at
    /// `start_offset` is shown.
    pub begin_time: f64,
    /// Where in the range playback begins, in seconds of the document's time; default 0.
    pub start_offset: f64,
    /// Shows whole frames only (the frame number rounded down), as a player without sub-frame
    /// interpolation does; default false (every session time has its own in-between frame).
    pub whole_frames: bool,
}

impl Default for LottiePlayOptions {
    fn default() -> LottiePlayOptions {
        LottiePlayOptions { playback: LottiePlayback::Loop, speed: 1.0, range: LottieRange::Whole, begin_time: 0.0, start_offset: 0.0, whole_frames: false }
    }
}

impl LottiePlayOptions {
    /// The defaults with another playback mode (`LottiePlayOptions::playing(LottiePlayback::Hold)`).
    pub fn playing(playback: LottiePlayback) -> LottiePlayOptions {
        LottiePlayOptions { playback, ..LottiePlayOptions::default() }
    }

    /// The playhead in seconds into the range, before the playback mode is applied.
    fn playhead(&self, time: f64) -> f64 {
        if time < self.begin_time {
            self.start_offset
        } else {
            (time - self.begin_time) * self.speed + self.start_offset
        }
    }

    /// The Lottie frame shown at session `time` for a range of frames `start..end` at
    /// `frame_rate` frames per second.
    pub(crate) fn frame_in(&self, start: f64, end: f64, frame_rate: f64, time: f64) -> f64 {
        let frames = end - start;
        if frames.is_nan() || frames <= 0.0 || frame_rate.is_nan() || frame_rate <= 0.0 || !time.is_finite() {
            return start;
        }
        let length = frames / frame_rate;
        let local = self.playhead(time);
        let seconds = match self.playback {
            LottiePlayback::Loop => local.rem_euclid(length),
            LottiePlayback::PingPong => {
                let period = 2.0 * length;
                let m = local.rem_euclid(period);
                if m <= length {
                    m
                } else {
                    period - m
                }
            }
            LottiePlayback::Hold => local.clamp(0.0, length),
        };
        let mut offset = seconds * frame_rate;
        if self.whole_frames {
            offset = offset.floor();
        }
        (start + offset).min(end - LAST_FRAME_INSET).max(start)
    }

    /// True when a `Hold` playback has reached the end of its range (the start, playing
    /// backwards) at session `time`; always false for `Loop` and `PingPong`.
    pub(crate) fn finished_in(&self, start: f64, end: f64, frame_rate: f64, time: f64) -> bool {
        if self.playback != LottiePlayback::Hold || time < self.begin_time || frame_rate.is_nan() || frame_rate <= 0.0 {
            return false;
        }
        let length = (end - start).max(0.0) / frame_rate;
        let local = self.playhead(time);
        if self.speed > 0.0 {
            local >= length
        } else if self.speed < 0.0 {
            local <= 0.0
        } else {
            false
        }
    }

    /// The session time at which the playhead first reaches `frame` (playing forwards from
    /// `begin_time`); None when it never does in the first pass (speed 0 or less, the frame
    /// outside the range or before `start_offset`).
    pub(crate) fn time_of_frame(&self, start: f64, end: f64, frame_rate: f64, frame: f64) -> Option<f64> {
        if self.speed.is_nan() || self.speed <= 0.0 || frame_rate.is_nan() || frame_rate <= 0.0 || frame < start || frame >= end {
            return None;
        }
        let seconds = (frame - start) / frame_rate - self.start_offset;
        if seconds < 0.0 {
            return None;
        }
        Some(self.begin_time + seconds / self.speed)
    }
}
