//! `KeyframePlayer`: keyframe playback in 8-bit frames - a painting made every few seconds, the
//! frames between made by a transition (C# `KeyframePlayer`, on ONE thread).

use crate::painting::keyframe_cache::{keyframe_at, KeyframeCache};
use crate::planes::DrawingTransition;

/// The optional parts of a keyframe player (C#'s default arguments: a dissolve, a cache of
/// [`KeyframePlayer::DEFAULT_CACHE_CAPACITY`] keyframes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyframePlayerOptions {
    /// How frames between keyframes are made; None for a dissolve.
    pub transition: Option<DrawingTransition>,
    /// How many painted keyframes are kept; at least 3.
    pub cache_capacity: usize,
}

impl Default for KeyframePlayerOptions {
    fn default() -> KeyframePlayerOptions {
        KeyframePlayerOptions { transition: None, cache_capacity: KeyframePlayer::DEFAULT_CACHE_CAPACITY }
    }
}

/// Paints keyframe (number, time) into the frame given: straight-alpha RGBA, rows top-down,
/// stride width x 4, to be filled completely (C# `KeyframePaintCallback`). An error is passed on
/// to the call that needed the keyframe.
pub type KeyframePaint = Box<dyn FnMut(i32, f64, &mut [u8]) -> Result<(), String>>;

/// KEYFRAME PLAYBACK for animations too costly to paint every frame (C# `KeyframePlayer`) - a
/// painterly scene painted every few seconds, the frames between dissolving from one painting to
/// the next. Keyframe k is painted at time `k x interval` by the caller's function; the frame at
/// time t is the transition (a dissolve by default) from keyframe `k = floor(t / interval)` to
/// keyframe k + 1 at fraction `u = t / interval - k` (exactly keyframe k when u is 0) - the same
/// bytes as C#.
///
/// ONE THREAD, NO RENDER-AHEAD WORKER (a browser gives a frontend one thread; C# paints on a
/// background worker): a keyframe is painted when it is first needed - by
/// [`KeyframePlayer::render_frame`] (k, and k + 1 while u > 0), or EARLIER by
/// [`KeyframePlayer::prefetch`], which paints at most ONE missing keyframe of k, k + 1, k + 2 per
/// call. Call `prefetch` once per frame after drawing it and steady playback never paints more
/// than one keyframe in a frame, and none in the frame that crosses into the next interval; after
/// a seek the first frame paints its two keyframes. At most `cache_capacity` painted keyframes are
/// kept (C#'s eviction: the farthest from the shown keyframe first, never the three it needs);
/// memory is `cache_capacity x width x height x 4` bytes (plus one recycled frame). The paint
/// function is never entered twice at once and never from inside itself.
pub struct KeyframePlayer {
    width: u32,
    height: u32,
    frame_bytes: usize,
    interval: f64,
    transition: DrawingTransition,
    cache_capacity: usize,
    paint: KeyframePaint,
    cache: KeyframeCache<Vec<u8>>,
}

pub(crate) fn check_player(width: u32, height: u32, interval: f64, cache_capacity: usize) -> Result<(), String> {
    if width < 1 || height < 1 {
        return Err("A keyframe player's width and height must be at least 1 pixel.".to_string());
    }
    if !interval.is_finite() || interval <= 0.0 {
        return Err("The interval between keyframes must be a positive, finite number of seconds.".to_string());
    }
    if cache_capacity < 3 {
        return Err("The keyframe cache must hold at least 3 keyframes (the two shown and the one painted ahead).".to_string());
    }
    Ok(())
}

impl KeyframePlayer {
    /// The default cache capacity: the two keyframes on screen, the one painted ahead, and one
    /// spare (C# `DefaultCacheCapacity`).
    pub const DEFAULT_CACHE_CAPACITY: usize = 4;

    /// A player (C#'s constructor): the frame size in pixels, seconds between keyframes
    /// (positive), the paint function, and the options. The error says what to do.
    pub fn new(width: u32, height: u32, interval: f64, paint: impl FnMut(i32, f64, &mut [u8]) -> Result<(), String> + 'static, options: KeyframePlayerOptions) -> Result<KeyframePlayer, String> {
        check_player(width, height, interval, options.cache_capacity)?;
        let frame_bytes = (width as usize).checked_mul(height as usize).and_then(|n| n.checked_mul(4)).ok_or_else(|| "The frame is too large to hold in memory.".to_string())?;
        Ok(KeyframePlayer { width, height, frame_bytes, interval, transition: options.transition.unwrap_or_else(DrawingTransition::dissolve), cache_capacity: options.cache_capacity, paint: Box::new(paint), cache: KeyframeCache::new(options.cache_capacity) })
    }

    /// The frame width, pixels (C# `Width`).
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The frame height, pixels (C# `Height`).
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The size of one frame, bytes - width x height x 4 (C# `FrameBytes`).
    pub fn frame_bytes(&self) -> usize {
        self.frame_bytes
    }

    /// Seconds between keyframes (C# `Interval`).
    pub fn interval(&self) -> f64 {
        self.interval
    }

    /// How the frames between two keyframes are made (C# `Transition`).
    pub fn transition(&self) -> DrawingTransition {
        self.transition
    }

    /// How many painted keyframes are kept at most (C# `CacheCapacity`).
    pub fn cache_capacity(&self) -> usize {
        self.cache_capacity
    }

    /// How many times the paint function has been called so far (C# `PaintCount`).
    pub fn paint_count(&self) -> u64 {
        self.cache.paint_count()
    }

    /// How many keyframes are cached now (C# `CachedKeyframeCount`, which also counts queued
    /// ones; nothing is queued here).
    pub fn cached_keyframe_count(&self) -> usize {
        self.cache.cached_count()
    }

    /// The keyframe a time starts from: `floor(seconds / interval)` (C# `KeyframeAt`). The error
    /// says what to do with a time that is not finite or too far from 0.
    pub fn keyframe_at(&self, seconds: f64) -> Result<i32, String> {
        Ok(keyframe_at(seconds, self.interval)?.0)
    }

    /// True when [`KeyframePlayer::render_frame`] at this time would paint nothing (no C#
    /// equivalent: C# blocks instead).
    pub fn is_ready(&self, seconds: f64) -> Result<bool, String> {
        let (k, fraction) = keyframe_at(seconds, self.interval)?;
        Ok(self.cache.contains(k) && (fraction <= 0.0 || self.cache.contains(k + 1)))
    }

    fn ensure(&mut self, key: i32, focus: i32) -> Result<(), String> {
        let (frame_bytes, interval) = (self.frame_bytes, self.interval);
        let paint = &mut self.paint;
        self.cache.ensure(key, focus, || Ok(vec![0u8; frame_bytes]), |frame| frame.fill(0), |k, frame| paint(k, k as f64 * interval, frame))
    }

    /// Writes the frame at a time into the first [`KeyframePlayer::frame_bytes`] bytes of
    /// `destination` (extra bytes untouched): the transition from its keyframe to the next at the
    /// time's fraction of the interval (C# `RenderFrame`). Paints the keyframes it needs that are
    /// not cached yet. The error says what to do (a short destination, a time that is not finite
    /// or too far from 0) or is the paint function's own.
    pub fn render_frame(&mut self, seconds: f64, destination: &mut [u8]) -> Result<(), String> {
        if destination.len() < self.frame_bytes {
            return Err("The destination is shorter than one frame (width x height x 4 bytes).".to_string());
        }
        let (k, fraction) = keyframe_at(seconds, self.interval)?;
        self.ensure(k, k)?;
        if fraction > 0.0 {
            self.ensure(k + 1, k)?;
        }
        let destination = &mut destination[..self.frame_bytes];
        let first = self.cache.get(k).ok_or_else(|| "The keyframe was dropped from a cache too small for the frame: use a capacity of at least 3.".to_string())?;
        if fraction <= 0.0 {
            destination.copy_from_slice(first);
            return Ok(());
        }
        let second = self.cache.get(k + 1).ok_or_else(|| "The keyframe was dropped from a cache too small for the frame: use a capacity of at least 3.".to_string())?;
        self.transition.apply_bytes(first, second, destination, self.width, self.height, fraction)
    }

    /// Paints AHEAD (C# `Prefetch`, which queues the work for its worker): paints the first of
    /// the keyframes a time needs - k, k + 1, k + 2 - that is not cached yet, at most ONE per call.
    /// True when it painted one. Call it once per frame, after drawing, to spread the painting
    /// over frames.
    pub fn prefetch(&mut self, seconds: f64) -> Result<bool, String> {
        let (k, _) = keyframe_at(seconds, self.interval)?;
        for key in [k, k + 1, k + 2] {
            if !self.cache.contains(key) {
                self.ensure(key, k)?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}
