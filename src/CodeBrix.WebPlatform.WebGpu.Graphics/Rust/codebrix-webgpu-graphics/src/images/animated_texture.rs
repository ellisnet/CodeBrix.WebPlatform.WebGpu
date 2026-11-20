//! [`AnimatedTexture`]: an animated image on the GPU - a list of RGBA frames with per-frame
//! durations (C#: `AnimatedTexture`), its [`AnimatedTextureOptions`] and
//! [`AnimatedTextureStorage`] (C#: `AnimatedTextureOptions.cs`).

use std::cell::{Cell, RefCell};

use crate::animation::AnimationLoopMode;
use crate::context::{label_text, GraphicsContext, GraphicsException, Texture2DOptions};
use crate::resources::{GpuTexture, TextureRegion};

/// 100-nanosecond ticks per second (C#'s `TimeSpan` resolution, kept so the frame arithmetic is
/// C#'s).
const TICKS_PER_SECOND: f64 = 10_000_000.0;

/// Where the frames of an [`AnimatedTexture`] live on the GPU (C#: `AnimatedTextureStorage`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimatedTextureStorage {
    /// Every frame is uploaded once into its own layer of a texture array; showing a frame is free.
    TextureArray,
    /// The frames stay in memory and the current one is uploaded into a small ring of layers when
    /// it changes - for animations too large for one array. No mipmaps.
    Streaming,
}

/// The settings of [`GraphicsContext::create_animated_texture`] (C#: `AnimatedTextureOptions`,
/// with C#'s defaults). Times are SECONDS (C#: `TimeSpan`).
#[derive(Clone, Debug, PartialEq)]
pub struct AnimatedTextureOptions {
    /// How playback continues after the last frame (C#: `LoopMode`; default Loop).
    pub loop_mode: AnimationLoopMode,
    /// True for sRGB frames, false for linear data (C#: `ColorSpace`; default sRGB).
    pub srgb: bool,
    /// True for GPU-made mipmaps on every layer, texture-array storage only (C#: `Mipmaps`).
    pub mipmaps: bool,
    /// True to premultiply colour by alpha once, while loading (C#: `PremultiplyAlpha`).
    pub premultiply_alpha: bool,
    /// The duration of every frame when `frame_durations` is empty, in seconds (C#:
    /// `DefaultFrameDuration`; default 0.1).
    pub default_frame_duration: f64,
    /// Per-frame durations in seconds (C#: `FrameDurations`); a shorter list repeats its last value.
    pub frame_durations: Vec<f64>,
    /// The largest texture array, in bytes with mipmaps, before the frames stream instead (C#:
    /// `MaxArrayBytes`; default 256 MiB).
    pub max_array_bytes: u64,
    /// A storage to use instead of choosing by size (C#: `Storage`; None = choose).
    pub storage: Option<AnimatedTextureStorage>,
    /// The ring layers of the streaming storage, 2 to 16 (C#: `StreamingSlots`; default 3).
    pub streaming_slots: u32,
    /// A label (C#: `Label`).
    pub label: Option<String>,
}

impl AnimatedTextureOptions {
    /// C#'s `DefaultMaxArrayBytes`: 256 MiB.
    pub const DEFAULT_MAX_ARRAY_BYTES: u64 = 256 * 1024 * 1024;
}

impl Default for AnimatedTextureOptions {
    fn default() -> AnimatedTextureOptions {
        AnimatedTextureOptions { loop_mode: AnimationLoopMode::Loop, srgb: true, mipmaps: false, premultiply_alpha: false, default_frame_duration: 0.1, frame_durations: Vec::new(), max_array_bytes: AnimatedTextureOptions::DEFAULT_MAX_ARRAY_BYTES, storage: None, streaming_slots: 3, label: None }
    }
}

/// An animated image on the GPU with per-frame durations (C#: `AnimatedTexture`).
/// [`AnimatedTexture::get_frame_texture`] gives the frame to show at a playback time as an
/// ordinary 2D [`GpuTexture`]: draw it with a [`SpriteBatch`](crate::SpriteBatch) (or
/// `SpriteBatch::draw_animated`), bind it, or use it as a material texture. Picking a frame
/// allocates nothing.
///
/// The frames come as decoded RGBA8 pixels, from anywhere (C#'s file, stream and Image overloads
/// decode with CodeBrix.Imaging; a browser does not decode): the backend decodes a GIF or an
/// animated image and sends the frames, or the application makes them. TIME COMES FROM THE FRAME:
/// pass the frame's time (or the time since the animation started), never a clock of its own.
pub struct AnimatedTexture {
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    storage: AnimatedTextureStorage,
    array_texture: GpuTexture,
    views: Vec<GpuTexture>,
    loop_mode: Cell<AnimationLoopMode>,
    timing: FrameTiming,
    retained: Vec<Vec<u8>>,
    slot_frames: RefCell<Vec<Option<usize>>>,
    next_slot: Cell<usize>,
    label: Option<String>,
}

impl GraphicsContext {
    /// An animated texture from equally sized RGBA8 frames (red first, tightly packed,
    /// `width` x `height` x 4 bytes each), shown for `options.frame_durations` (or
    /// `default_frame_duration`) (C#: `CreateAnimatedTexture(frames, options)`). The frames are
    /// copied. Bytes of several frames in one buffer split with `bytes.chunks_exact(w * h * 4)`.
    pub fn create_animated_texture(&self, width: u32, height: u32, frames: &[&[u8]], options: &AnimatedTextureOptions) -> Result<AnimatedTexture, GraphicsException> {
        AnimatedTexture::new(self, width, height, frames, options)
    }
}

impl AnimatedTexture {
    fn new(context: &GraphicsContext, width: u32, height: u32, frames: &[&[u8]], options: &AnimatedTextureOptions) -> Result<AnimatedTexture, GraphicsException> {
        let label = options.label.as_deref();
        if frames.is_empty() {
            return Err(GraphicsException::new(format!("The animated texture{} needs at least one frame.", label_text(label))));
        }
        let frame_bytes = width as usize * height as usize * 4;
        for (index, frame) in frames.iter().enumerate() {
            if frame.len() != frame_bytes {
                return Err(GraphicsException::new(format!("Frame {index} of the animated texture{} has {} bytes; a {width} x {height} RGBA8 frame has {frame_bytes}.", label_text(label), frame.len())));
            }
        }
        let format = if options.srgb { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm };
        let timing = FrameTiming::new(&durations(frames.len(), options));
        let storage = choose_storage(context.limits().max_texture_array_layers, width, height, frames.len(), options);
        let prepared: Vec<Vec<u8>> = frames
            .iter()
            .map(|frame| {
                let mut pixels = frame.to_vec();
                if options.premultiply_alpha {
                    premultiply(&mut pixels);
                }
                pixels
            })
            .collect();
        let (array_texture, views, retained, slots) = match storage {
            AnimatedTextureStorage::TextureArray => {
                let mips = if options.mipmaps { GpuTexture::full_mip_chain_length(width, height) } else { 1 };
                let mut usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC;
                if mips > 1 {
                    usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
                }
                let array_label = format!("{} frames", label.unwrap_or("animated"));
                let array = context.create_texture_array(width, height, frames.len() as u32, format, usage, Texture2DOptions { mip_level_count: mips, label: Some(array_label) })?;
                for (index, pixels) in prepared.iter().enumerate() {
                    array.write_region(&TextureRegion::new(0, 0, width, height).on_layer(index as u32), pixels)?;
                }
                if mips > 1 {
                    context.generate_mipmaps(&array)?;
                }
                let views = (0..frames.len() as u32).map(|layer| array.create_layer_view(layer, label)).collect::<Result<Vec<_>, _>>()?;
                (array, views, Vec::new(), Vec::new())
            }
            AnimatedTextureStorage::Streaming => {
                let slots = options.streaming_slots.clamp(2, 16);
                let ring_label = format!("{} ring", label.unwrap_or("animated"));
                let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC;
                let array = context.create_texture_array(width, height, slots, format, usage, Texture2DOptions { mip_level_count: 1, label: Some(ring_label) })?;
                let views = (0..slots).map(|layer| array.create_layer_view(layer, label)).collect::<Result<Vec<_>, _>>()?;
                (array, views, prepared, vec![None; slots as usize])
            }
        };
        Ok(AnimatedTexture { width, height, format, storage, array_texture, views, loop_mode: Cell::new(options.loop_mode), timing, retained, slot_frames: RefCell::new(slots), next_slot: Cell::new(0), label: options.label.clone() })
    }

    /// The number of frames (C#: `FrameCount`).
    pub fn frame_count(&self) -> usize {
        self.timing.frame_ends.len()
    }

    /// The width in pixels (C#: `Width`).
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels (C#: `Height`).
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The texture format: RGBA8, sRGB unless the options chose linear (C#: `Format`).
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Where the frames live (C#: `Storage`).
    pub fn storage(&self) -> AnimatedTextureStorage {
        self.storage
    }

    /// The texture array: one layer per frame, or the streaming ring (C#: `ArrayTexture`); index it
    /// from your own shaders with [`AnimatedTexture::get_layer`].
    pub fn array_texture(&self) -> &GpuTexture {
        &self.array_texture
    }

    /// How playback continues after the last frame (C#: `LoopMode`).
    pub fn loop_mode(&self) -> AnimationLoopMode {
        self.loop_mode.get()
    }

    /// Changes how playback continues after the last frame, at any time (C#: the `LoopMode` setter).
    pub fn set_loop_mode(&self, loop_mode: AnimationLoopMode) {
        self.loop_mode.set(loop_mode);
    }

    /// The duration of one forward pass through every frame, in seconds (C#: `TotalDuration`).
    pub fn total_duration(&self) -> f64 {
        self.timing.total() as f64 / TICKS_PER_SECOND
    }

    /// The duration of one frame in seconds (C#: `GetFrameDuration`).
    pub fn get_frame_duration(&self, frame: usize) -> Result<f64, GraphicsException> {
        self.check_frame(frame)?;
        Ok(self.timing.durations[frame] as f64 / TICKS_PER_SECOND)
    }

    /// The frame shown after `elapsed` seconds of playback, looping, ping-pong or once by the loop
    /// mode (C#: `GetFrameIndex`; a negative time counts as zero).
    pub fn get_frame_index(&self, elapsed: f64) -> usize {
        self.timing.frame_index(seconds_to_ticks(elapsed), self.loop_mode.get())
    }

    /// The 2D texture of the frame shown after `elapsed` seconds (C#: `GetFrameTexture`). With
    /// streaming storage the frame is uploaded into the next ring layer when it is not resident.
    pub fn get_frame_texture(&self, elapsed: f64) -> &GpuTexture {
        let frame = self.get_frame_index(elapsed);
        &self.views[self.resident(frame)]
    }

    /// The 2D texture of one frame (C#: `GetFrameTextureAt`).
    pub fn get_frame_texture_at(&self, frame: usize) -> Result<&GpuTexture, GraphicsException> {
        self.check_frame(frame)?;
        Ok(&self.views[self.resident(frame)])
    }

    /// The layer of [`AnimatedTexture::array_texture`] that holds the frame shown after `elapsed`
    /// seconds, uploading it first when streaming (C#: `GetLayer`).
    pub fn get_layer(&self, elapsed: f64) -> u32 {
        self.resident(self.get_frame_index(elapsed)) as u32
    }

    /// The label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    fn check_frame(&self, frame: usize) -> Result<(), GraphicsException> {
        if frame >= self.frame_count() {
            return Err(GraphicsException::new(format!("The animated texture{} has frames 0 to {}; frame {frame} does not exist.", label_text(self.label.as_deref()), self.frame_count() - 1)));
        }
        Ok(())
    }

    /// The layer holding `frame`, uploading it into the ring first when streaming.
    fn resident(&self, frame: usize) -> usize {
        if self.storage == AnimatedTextureStorage::TextureArray {
            return frame;
        }
        let mut slots = self.slot_frames.borrow_mut();
        if let Some(slot) = slots.iter().position(|resident| *resident == Some(frame)) {
            return slot;
        }
        let target = self.next_slot.get();
        self.next_slot.set((target + 1) % slots.len());
        // A size checked at creation: the write cannot fail for these arguments.
        let _ = self.array_texture.write_region(&TextureRegion::new(0, 0, self.width, self.height).on_layer(target as u32), &self.retained[frame]);
        slots[target] = Some(frame);
        target
    }
}

/// The frame timing of an animation: durations and running end times in 100 ns ticks (C#'s).
pub(crate) struct FrameTiming {
    durations: Vec<i64>,
    frame_ends: Vec<i64>,
}

impl FrameTiming {
    pub(crate) fn new(durations_in_seconds: &[f64]) -> FrameTiming {
        let durations: Vec<i64> = durations_in_seconds.iter().map(|seconds| seconds_to_ticks(*seconds).max(1)).collect();
        let mut total = 0;
        let frame_ends = durations
            .iter()
            .map(|duration| {
                total += duration;
                total
            })
            .collect();
        FrameTiming { durations, frame_ends }
    }

    fn total(&self) -> i64 {
        *self.frame_ends.last().unwrap_or(&0)
    }

    /// C#'s `GetFrameIndex` on ticks.
    pub(crate) fn frame_index(&self, elapsed: i64, loop_mode: AnimationLoopMode) -> usize {
        let total = self.total();
        let last = self.frame_ends.len().saturating_sub(1);
        if last == 0 || total <= 0 {
            return 0;
        }
        let time = elapsed.max(0);
        match loop_mode {
            AnimationLoopMode::Once => {
                if time >= total {
                    last
                } else {
                    self.find(time)
                }
            }
            AnimationLoopMode::PingPong => {
                let back = total - self.durations[0] - self.durations[last];
                if back <= 0 {
                    return self.find(time % total);
                }
                let position = time % (total + back);
                if position < total {
                    self.find(position)
                } else {
                    self.find(self.frame_ends[last - 1] - 1 - (position - total))
                }
            }
            AnimationLoopMode::Loop => self.find(time % total),
        }
    }

    /// The first frame whose end is after `time` (C#'s binary search).
    fn find(&self, time: i64) -> usize {
        let (mut low, mut high) = (0usize, self.frame_ends.len() - 1);
        while low < high {
            let middle = (low + high) >> 1;
            if self.frame_ends[middle] > time {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        low
    }
}

/// Seconds as 100 ns ticks, rounded.
fn seconds_to_ticks(seconds: f64) -> i64 {
    if seconds.is_finite() { (seconds * TICKS_PER_SECOND).round() as i64 } else { 0 }
}

/// The per-frame durations in seconds (C#'s `ReadDurations` for a source without timing):
/// `frame_durations` (its last value repeated), else the default; a duration that is not positive
/// becomes the default (or 0.1 s when the default is not positive either).
pub(crate) fn durations(count: usize, options: &AnimatedTextureOptions) -> Vec<f64> {
    let fallback = if options.default_frame_duration > 0.0 { options.default_frame_duration } else { 0.1 };
    (0..count)
        .map(|index| {
            let duration = if options.frame_durations.is_empty() { fallback } else { options.frame_durations[index.min(options.frame_durations.len() - 1)] };
            if duration > 0.0 { duration } else { fallback }
        })
        .collect()
}

/// C#'s `ChooseStorage`: the forced storage, else streaming above the device's layer limit or
/// the byte budget, else a texture array.
pub(crate) fn choose_storage(max_layers: u32, width: u32, height: u32, count: usize, options: &AnimatedTextureOptions) -> AnimatedTextureStorage {
    if let Some(storage) = options.storage {
        return storage;
    }
    if max_layers != 0 && count as u64 > max_layers as u64 {
        return AnimatedTextureStorage::Streaming;
    }
    let bytes = width as f64 * height as f64 * 4.0 * count as f64 * if options.mipmaps { 4.0 / 3.0 } else { 1.0 };
    if bytes > options.max_array_bytes as f64 { AnimatedTextureStorage::Streaming } else { AnimatedTextureStorage::TextureArray }
}

/// C#'s `ImageUpload.Premultiply`: colour times alpha, rounded, for every pixel that is not opaque.
pub(crate) fn premultiply(rgba: &mut [u8]) {
    for pixel in rgba.chunks_exact_mut(4) {
        let alpha = pixel[3] as u32;
        if alpha == 255 {
            continue;
        }
        for channel in &mut pixel[0..3] {
            *channel = ((*channel as u32 * alpha + 127) / 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{choose_storage, durations, premultiply, AnimatedTextureOptions, AnimatedTextureStorage, FrameTiming};
    use crate::animation::AnimationLoopMode;

    const MS: i64 = 10_000;

    fn timing(milliseconds: &[f64]) -> FrameTiming {
        FrameTiming::new(&milliseconds.iter().map(|m| m / 1000.0).collect::<Vec<_>>())
    }

    #[test]
    fn loop_wraps_at_the_total_duration() {
        let t = timing(&[100.0, 100.0, 100.0]);
        assert_eq!(t.frame_index(0, AnimationLoopMode::Loop), 0);
        assert_eq!(t.frame_index(99 * MS, AnimationLoopMode::Loop), 0);
        assert_eq!(t.frame_index(100 * MS, AnimationLoopMode::Loop), 1);
        assert_eq!(t.frame_index(299 * MS, AnimationLoopMode::Loop), 2);
        assert_eq!(t.frame_index(300 * MS, AnimationLoopMode::Loop), 0);
        assert_eq!(t.frame_index(-5 * MS, AnimationLoopMode::Loop), 0);
    }

    #[test]
    fn once_holds_the_last_frame() {
        let t = timing(&[100.0, 100.0, 100.0]);
        assert_eq!(t.frame_index(250 * MS, AnimationLoopMode::Once), 2);
        assert_eq!(t.frame_index(10_000 * MS, AnimationLoopMode::Once), 2);
    }

    #[test]
    fn ping_pong_does_not_repeat_the_end_frames() {
        let t = timing(&[100.0, 100.0, 100.0, 100.0]);
        let frames: Vec<usize> = (0..12).map(|step| t.frame_index(step * 100 * MS + MS, AnimationLoopMode::PingPong)).collect();
        assert_eq!(frames, vec![0, 1, 2, 3, 2, 1, 0, 1, 2, 3, 2, 1]);
        let two = timing(&[100.0, 100.0]);
        assert_eq!(two.frame_index(150 * MS, AnimationLoopMode::PingPong), 1);
        assert_eq!(two.frame_index(250 * MS, AnimationLoopMode::PingPong), 0);
    }

    #[test]
    fn uneven_durations_are_honoured() {
        let t = timing(&[50.0, 200.0, 10.0]);
        assert_eq!(t.frame_index(49 * MS, AnimationLoopMode::Loop), 0);
        assert_eq!(t.frame_index(50 * MS, AnimationLoopMode::Loop), 1);
        assert_eq!(t.frame_index(255 * MS, AnimationLoopMode::Loop), 2);
        assert_eq!(timing(&[100.0]).frame_index(12345 * MS, AnimationLoopMode::PingPong), 0);
    }

    #[test]
    fn durations_repeat_the_last_value_and_replace_bad_ones() {
        let mut options = AnimatedTextureOptions::default();
        assert_eq!(durations(2, &options), vec![0.1, 0.1]);
        options.frame_durations = vec![0.05, 0.2];
        assert_eq!(durations(4, &options), vec![0.05, 0.2, 0.2, 0.2]);
        options.frame_durations = vec![0.0, -1.0];
        options.default_frame_duration = 0.25;
        assert_eq!(durations(2, &options), vec![0.25, 0.25]);
        options.default_frame_duration = 0.0;
        assert_eq!(durations(1, &options), vec![0.1]);
    }

    #[test]
    fn storage_streams_above_the_budget_or_the_layer_limit() {
        let mut options = AnimatedTextureOptions::default();
        assert_eq!(choose_storage(256, 64, 64, 8, &options), AnimatedTextureStorage::TextureArray);
        assert_eq!(choose_storage(4, 64, 64, 8, &options), AnimatedTextureStorage::Streaming);
        options.max_array_bytes = 64 * 64 * 4 * 8 - 1;
        assert_eq!(choose_storage(256, 64, 64, 8, &options), AnimatedTextureStorage::Streaming);
        options.storage = Some(AnimatedTextureStorage::TextureArray);
        assert_eq!(choose_storage(4, 64, 64, 8, &options), AnimatedTextureStorage::TextureArray);
    }

    #[test]
    fn premultiply_rounds_as_csharp() {
        let mut pixels = [255, 128, 0, 128, 10, 20, 30, 255, 200, 200, 200, 0];
        premultiply(&mut pixels);
        assert_eq!(pixels, [128, 64, 0, 128, 10, 20, 30, 255, 0, 0, 0, 0]);
    }

    #[test]
    fn the_defaults_are_the_csharp_defaults() {
        let options = AnimatedTextureOptions::default();
        assert_eq!(options.loop_mode, AnimationLoopMode::Loop);
        assert!(options.srgb && !options.mipmaps && !options.premultiply_alpha);
        assert_eq!(options.default_frame_duration, 0.1);
        assert_eq!(options.max_array_bytes, 256 * 1024 * 1024);
        assert_eq!(options.streaming_slots, 3);
        assert!(options.storage.is_none());
    }
}
