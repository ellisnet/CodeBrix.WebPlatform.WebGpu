//! `PlaneKeyframePlayer`: keyframe playback in float images (C# `PlaneKeyframePlayer`, on ONE
//! thread).

use crate::painting::keyframe_cache::{keyframe_at, KeyframeCache};
use crate::painting::keyframe_player::{check_player, KeyframePlayerOptions};
use crate::planes::{DrawingPlaneImage, DrawingTransition};

/// Paints keyframe (number, time) into the image given - a fresh, transparent black image of
/// the player's size (C#'s `Action<int, double, DrawingPlaneImage>`). An error is passed on to
/// the call that needed the keyframe.
pub type PlaneKeyframePaint = Box<dyn FnMut(i32, f64, &mut DrawingPlaneImage) -> Result<(), String>>;

/// KEYFRAME PLAYBACK in float images (C# `PlaneKeyframePlayer`): exactly
/// [`KeyframePlayer`](crate::painting::KeyframePlayer), but each keyframe is painted into a
/// [`DrawingPlaneImage`] (straight alpha, open-ended values) and the frames between are made with
/// the float form of [`DrawingTransition`] - so the dissolve happens before any tone curve or
/// 8-bit conversion. The same one-thread rules: keyframes are painted when first needed, or one
/// per [`PlaneKeyframePlayer::prefetch`] call ahead of time; memory is `cache_capacity x width x
/// height x 16` bytes (four f32 planes per keyframe, plus one recycled image).
pub struct PlaneKeyframePlayer {
    width: u32,
    height: u32,
    interval: f64,
    transition: DrawingTransition,
    cache_capacity: usize,
    paint: PlaneKeyframePaint,
    cache: KeyframeCache<DrawingPlaneImage>,
}

impl PlaneKeyframePlayer {
    /// A player (C#'s constructor): the frame size, seconds between keyframes (positive), the
    /// paint function, and the options. The error says what to do.
    pub fn new(width: u32, height: u32, interval: f64, paint: impl FnMut(i32, f64, &mut DrawingPlaneImage) -> Result<(), String> + 'static, options: KeyframePlayerOptions) -> Result<PlaneKeyframePlayer, String> {
        check_player(width, height, interval, options.cache_capacity)?;
        Ok(PlaneKeyframePlayer { width, height, interval, transition: options.transition.unwrap_or_else(DrawingTransition::dissolve), cache_capacity: options.cache_capacity, paint: Box::new(paint), cache: KeyframeCache::new(options.cache_capacity) })
    }

    /// The frame width, pixels (C# `Width`).
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The frame height, pixels (C# `Height`).
    pub fn height(&self) -> u32 {
        self.height
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

    /// How many keyframes are cached now (C# `CachedKeyframeCount`).
    pub fn cached_keyframe_count(&self) -> usize {
        self.cache.cached_count()
    }

    /// The keyframe a time starts from: `floor(seconds / interval)` (C# `KeyframeAt`).
    pub fn keyframe_at(&self, seconds: f64) -> Result<i32, String> {
        Ok(keyframe_at(seconds, self.interval)?.0)
    }

    /// True when [`PlaneKeyframePlayer::render_frame`] at this time would paint nothing (no C#
    /// equivalent).
    pub fn is_ready(&self, seconds: f64) -> Result<bool, String> {
        let (k, fraction) = keyframe_at(seconds, self.interval)?;
        Ok(self.cache.contains(k) && (fraction <= 0.0 || self.cache.contains(k + 1)))
    }

    fn ensure(&mut self, key: i32, focus: i32) -> Result<(), String> {
        let (width, height, interval) = (self.width, self.height, self.interval);
        let paint = &mut self.paint;
        self.cache.ensure(
            key,
            focus,
            || DrawingPlaneImage::new(width, height),
            |image| {
                image.clear();
            },
            |k, image| paint(k, k as f64 * interval, image),
        )
    }

    /// Writes the frame at a time into an image of the player's size, every plane overwritten:
    /// the transition from its keyframe to the next at the time's fraction of the interval (C#
    /// `RenderFrame`). Paints the keyframes it needs that are not cached yet. The error says what
    /// to do or is the paint function's own.
    pub fn render_frame(&mut self, seconds: f64, destination: &mut DrawingPlaneImage) -> Result<(), String> {
        if destination.width() != self.width || destination.height() != self.height {
            return Err("The image must be the player's size.".to_string());
        }
        let (k, fraction) = keyframe_at(seconds, self.interval)?;
        self.ensure(k, k)?;
        if fraction > 0.0 {
            self.ensure(k + 1, k)?;
        }
        let first = self.cache.get(k).ok_or_else(|| "The keyframe was dropped from a cache too small for the frame: use a capacity of at least 3.".to_string())?;
        if fraction <= 0.0 {
            return first.copy_to(destination);
        }
        let second = self.cache.get(k + 1).ok_or_else(|| "The keyframe was dropped from a cache too small for the frame: use a capacity of at least 3.".to_string())?;
        self.transition.apply_into(first, second, destination, fraction)
    }

    /// Paints AHEAD at most ONE missing keyframe of k, k + 1, k + 2 for a time (C# `Prefetch`);
    /// true when it painted one.
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
