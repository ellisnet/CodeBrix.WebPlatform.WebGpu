//! [`FrameTimer`]: frame timings an application asks for - CPU time for the frame, GPU time per
//! marked pass and in total, smoothed values of each, and the renderers' counts beside them
//! ([`FrameTimings`], [`PassTiming`], [`FrameTimerOptions`]). Not in C# (the C# library has the
//! per-pass [`GpuTimer`](crate::GpuTimer) only).

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use crate::context::{GraphicsContext, GraphicsException};
use crate::frames::GraphicsFrame;
use crate::queries::cpu_clock::now_ms;
use crate::queries::gpu_timer::milliseconds_from_ticks;
use crate::queries::{GpuQuerySet, PassTimestamps};
use crate::resources::GpuBuffer;

/// The read-back buffers a frame timer cycles through: how many frames' GPU results may be on
/// their way back at once (a frame that finds none free is not measured on the GPU).
const READBACK_BUFFERS: usize = 4;

/// The settings of a [`FrameTimer`] (Not in C#).
#[derive(Clone, Debug, PartialEq)]
pub struct FrameTimerOptions {
    /// The most passes one frame can mark with [`FrameTimer::pass`] (1 to 2048); a further pass
    /// in the same frame is not timed. Default 16.
    pub max_passes: u32,
    /// How many recent frames the smoothed values average over (at least 1). Default 30.
    pub smoothing_frames: u32,
    /// The label of the timer's query set and buffers.
    pub label: Option<String>,
}

impl Default for FrameTimerOptions {
    fn default() -> FrameTimerOptions {
        FrameTimerOptions { max_passes: 16, smoothing_frames: 30, label: None }
    }
}

/// The GPU time of one marked pass (Not in C#). Passes marked with the same name in one frame
/// are added together.
#[derive(Clone, Debug, PartialEq)]
pub struct PassTiming {
    /// The name given to [`FrameTimer::pass`].
    pub name: String,
    /// The GPU milliseconds of the pass in the frame the GPU results come from.
    pub gpu_ms: f64,
    /// The mean of the pass's GPU milliseconds over the recent measured frames.
    pub gpu_ms_smoothed: f64,
}

/// What a [`FrameTimer`] measured (Not in C#), read with [`FrameTimer::timings`]. CPU values are
/// those of the last frame that ended; GPU values ARRIVE A FEW FRAMES LATE (the browser hands
/// timestamp results back asynchronously) - [`FrameTimings::gpu_frame`] says which frame they
/// are from.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameTimings {
    /// The frames the timer has ended (from 1; 0 before the first `end_frame`).
    pub frame: u64,
    /// CPU milliseconds from [`FrameTimer::begin_frame`] to [`FrameTimer::end_frame`] of the
    /// last frame.
    pub cpu_ms: f64,
    /// The mean CPU milliseconds over the recent frames.
    pub cpu_ms_smoothed: f64,
    /// True when the device has the `timestamp-query` feature, so GPU times can be measured.
    /// When false every GPU value is None - "not available", never an error.
    pub gpu_available: bool,
    /// The frame the GPU values come from (None until the first results arrive).
    pub gpu_frame: Option<u64>,
    /// The GPU milliseconds of all marked passes of that frame, added together (None: not
    /// available, or no results yet). Work outside the marked passes is not in it.
    pub gpu_total_ms: Option<f64>,
    /// The mean of [`FrameTimings::gpu_total_ms`] over the recent measured frames.
    pub gpu_total_ms_smoothed: Option<f64>,
    /// Each marked pass of that frame, in the order the passes were marked (empty when not
    /// available or before the first results).
    pub passes: Vec<PassTiming>,
    /// The counts the application put beside the timings with [`FrameTimer::set_count`] in the
    /// last frame (draw calls, instances, culled ...), in the order they were set.
    pub counts: Vec<(String, u64)>,
}

impl fmt::Display for FrameTimings {
    /// One line for an overlay: `cpu 1.20 ms (1.15) | gpu 2.40 ms (2.35): scene 2.00, bloom 0.40
    /// | draw calls 12`, or `gpu n/a` when not available.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cpu {:.2} ms ({:.2})", self.cpu_ms, self.cpu_ms_smoothed)?;
        match (self.gpu_available, self.gpu_total_ms, self.gpu_total_ms_smoothed) {
            (false, _, _) => write!(f, " | gpu n/a")?,
            (true, Some(total), Some(smoothed)) => {
                write!(f, " | gpu {total:.2} ms ({smoothed:.2})")?;
                for (index, pass) in self.passes.iter().enumerate() {
                    write!(f, "{}{} {:.2}", if index == 0 { ": " } else { ", " }, pass.name, pass.gpu_ms_smoothed)?;
                }
            }
            (true, _, _) => write!(f, " | gpu ...")?,
        }
        for (name, value) in &self.counts {
            write!(f, " | {name} {value}")?;
        }
        Ok(())
    }
}

/// The mean of the last `window` values (Not in C#).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RollingMean {
    window: usize,
    values: std::collections::VecDeque<f64>,
    sum: f64,
    pushes: u64,
}

impl RollingMean {
    /// An empty mean over `window` values (at least 1).
    pub(crate) fn new(window: u32) -> RollingMean {
        let window = window.max(1) as usize;
        RollingMean { window, values: std::collections::VecDeque::with_capacity(window), sum: 0.0, pushes: 0 }
    }

    /// Adds a value (dropping the oldest when the window is full) and returns the new mean.
    pub(crate) fn push(&mut self, value: f64) -> f64 {
        if self.values.len() == self.window {
            if let Some(old) = self.values.pop_front() {
                self.sum -= old;
            }
        }
        self.values.push_back(value);
        self.sum += value;
        // Summing again once per window keeps the running sum's rounding from drifting.
        self.pushes += 1;
        if self.pushes.is_multiple_of(self.window as u64) {
            self.sum = self.values.iter().sum();
        }
        self.mean()
    }

    /// The mean of the values in the window (0 when empty).
    pub(crate) fn mean(&self) -> f64 {
        if self.values.is_empty() {
            0.0
        } else {
            self.sum / self.values.len() as f64
        }
    }
}

/// Where one read-back buffer is (shared with the map callback, which may run on another thread
/// on a native target, hence the atomic).
mod state {
    /// Free to copy into.
    pub(super) const FREE: u8 = 0;
    /// A frame's results were copied into it; it is mapped at the next `begin_frame` (after the
    /// frame was submitted).
    pub(super) const COPIED: u8 = 1;
    /// Being mapped.
    pub(super) const MAPPING: u8 = 2;
    /// Mapped: the results can be read.
    pub(super) const READY: u8 = 3;
    /// The map failed (the device may be lost).
    pub(super) const FAILED: u8 = 4;
}

/// One read-back buffer and what it holds.
struct Readback {
    buffer: GpuBuffer,
    state: Arc<AtomicU8>,
    frame: u64,
    names: Vec<String>,
}

/// The GPU half of a frame timer, when the device has `timestamp-query`.
struct GpuHalf {
    query_set: GpuQuerySet,
    resolve_buffer: GpuBuffer,
    readbacks: Vec<Readback>,
    period_nanoseconds: f64,
}

/// FRAME TIMINGS the application asks for (Not in C#): made by
/// [`GraphicsContext::create_frame_timer`]; nothing is measured unless an application creates
/// one. Each frame:
///
/// 1. [`FrameTimer::begin_frame`] at the start of the frame's work (the top of the layer's
///    `render`) - it also collects GPU results that have arrived;
/// 2. [`FrameTimer::pass`] for each pass to time on the GPU: give what it returns to the pass as
///    its `timestamps` option ([`RenderPassOptions`](crate::RenderPassOptions) /
///    [`ComputePassOptions`](crate::ComputePassOptions)); it returns None when GPU timing is
///    not available or the frame's slots are used up, and the pass then simply goes untimed;
/// 3. optionally [`FrameTimer::set_count`] for the renderers' counts
///    (`ModelRenderer::last_draw_call_count` ...), shown beside the timings;
/// 4. [`FrameTimer::end_frame`] after the last timed pass, outside any pass, on the same frame.
///
/// Then [`FrameTimer::timings`] reads CPU milliseconds for the frame (begin to end, as measured
/// by the page's `performance.now()`), GPU milliseconds per marked pass and in total, and a mean
/// of each over [`FrameTimerOptions::smoothing_frames`] recent frames.
///
/// GPU TIME NEEDS THE BROWSER FEATURE `timestamp-query` (ask for it with
/// `GraphicsContextOptions::optional_features`; a canvas-host context has what its device was
/// created with). Where it is missing the GPU values are None ("not available"), the CPU values
/// still work, and nothing is an error. GPU TIMES ARRIVE A FEW FRAMES LATE: the results of a
/// frame are mapped after the frame was submitted and read at a later `begin_frame`. One
/// begin/end pair per frame; a frame whose read-back buffers are all still busy is not measured
/// on the GPU (its CPU time is). Browsers coarsen timestamps (to about 0.1 ms or more), so short
/// passes may read 0.
pub struct FrameTimer {
    context: GraphicsContext,
    max_passes: u32,
    smoothing_frames: u32,
    gpu: Option<GpuHalf>,
    frame_started: Option<f64>,
    frame_names: Vec<String>,
    frame_counts: Vec<(String, u64)>,
    cpu: RollingMean,
    gpu_total: RollingMean,
    gpu_passes: HashMap<String, RollingMean>,
    timings: FrameTimings,
}

impl FrameTimer {
    /// The most passes a frame may mark (the query set limit, two queries per pass).
    pub const MAX_PASSES: u32 = GraphicsContext::MAX_QUERIES_PER_SET / 2;

    pub(crate) fn new(context: &GraphicsContext, options: &FrameTimerOptions) -> Result<FrameTimer, GraphicsException> {
        check_max_passes(options.max_passes)?;
        let label = options.label.as_deref();
        let gpu = if context.has_feature(wgpu::Features::TIMESTAMP_QUERY) {
            let query_set = context.create_timestamp_query_set(options.max_passes * 2, label)?;
            let size = options.max_passes as u64 * 16;
            let resolve_buffer = context.create_buffer(size, wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC, label)?;
            let mut readbacks = Vec::with_capacity(READBACK_BUFFERS);
            for _ in 0..READBACK_BUFFERS {
                let buffer = context.create_buffer(size, wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, label)?;
                readbacks.push(Readback { buffer, state: Arc::new(AtomicU8::new(state::FREE)), frame: 0, names: Vec::new() });
            }
            let period_nanoseconds = context.queue().get_timestamp_period() as f64;
            Some(GpuHalf { query_set, resolve_buffer, readbacks, period_nanoseconds })
        } else {
            None
        };
        let timings = FrameTimings { gpu_available: gpu.is_some(), ..FrameTimings::default() };
        Ok(FrameTimer {
            context: context.clone(),
            max_passes: options.max_passes,
            smoothing_frames: options.smoothing_frames.max(1),
            gpu,
            frame_started: None,
            frame_names: Vec::new(),
            frame_counts: Vec::new(),
            cpu: RollingMean::new(options.smoothing_frames),
            gpu_total: RollingMean::new(options.smoothing_frames),
            gpu_passes: HashMap::new(),
            timings,
        })
    }

    /// True when GPU time can be measured (the device has `timestamp-query`).
    pub fn gpu_available(&self) -> bool {
        self.gpu.is_some()
    }

    /// The most passes one frame can mark ([`FrameTimerOptions::max_passes`]).
    pub fn max_passes(&self) -> u32 {
        self.max_passes
    }

    /// Marks the start of a frame (the CPU clock starts) and collects the GPU results that have
    /// arrived since the last call. Call it once per frame, before the frame's work.
    pub fn begin_frame(&mut self) {
        self.collect();
        self.frame_names.clear();
        self.frame_counts.clear();
        self.frame_started = Some(now_ms());
    }

    /// The timestamps that time the next pass under `name` (give them to the pass's
    /// `timestamps` option), or None when GPU time is not available, the frame has used its
    /// [`FrameTimer::max_passes`], or `begin_frame` was not called - the pass then goes
    /// untimed.
    pub fn pass(&mut self, name: &str) -> Option<PassTimestamps> {
        let gpu = self.gpu.as_ref()?;
        self.frame_started?;
        let slot = self.frame_names.len() as u32;
        if slot >= self.max_passes {
            return None;
        }
        self.frame_names.push(name.to_string());
        Some(PassTimestamps::new(&gpu.query_set, Some(slot * 2), Some(slot * 2 + 1)))
    }

    /// Puts a count beside this frame's timings (draw calls, instances, culled ...); setting the
    /// same name again in the frame replaces its value.
    pub fn set_count(&mut self, name: &str, value: u64) {
        match self.frame_counts.iter_mut().find(|(kept, _)| kept == name) {
            Some((_, kept)) => *kept = value,
            None => self.frame_counts.push((name.to_string(), value)),
        }
    }

    /// Marks the end of the frame (the CPU time is taken) and records the resolve of the
    /// frame's GPU timestamps into `frame` - after the last timed pass, outside any pass, on the
    /// frame whose passes were timed. Without a `begin_frame` before it, it does nothing.
    pub fn end_frame(&mut self, frame: &mut GraphicsFrame<'_>) {
        let Some(started) = self.frame_started.take() else { return };
        let cpu_ms = (now_ms() - started).max(0.0);
        self.timings.frame += 1;
        self.timings.cpu_ms = cpu_ms;
        self.timings.cpu_ms_smoothed = self.cpu.push(cpu_ms);
        self.timings.counts = std::mem::take(&mut self.frame_counts);
        let used = self.frame_names.len() as u32;
        let frame_number = self.timings.frame;
        let Some(gpu) = self.gpu.as_mut() else { return };
        if used == 0 {
            return;
        }
        let Some(readback) = gpu.readbacks.iter_mut().find(|r| r.state.load(Ordering::Acquire) == state::FREE) else { return };
        frame.encoder().resolve_query_set(gpu.query_set.handle(), 0..used * 2, gpu.resolve_buffer.handle(), 0);
        frame.copy_buffer(&gpu.resolve_buffer, 0, &readback.buffer, 0, used as u64 * 16);
        readback.frame = frame_number;
        readback.names = std::mem::take(&mut self.frame_names);
        readback.state.store(state::COPIED, Ordering::Release);
    }

    /// What has been measured so far.
    pub fn timings(&self) -> &FrameTimings {
        &self.timings
    }

    /// The context the timer measures with.
    pub fn context(&self) -> &GraphicsContext {
        &self.context
    }

    /// Reads the results that are mapped, frees their buffers, and starts the maps of the
    /// buffers the previous frames copied into (those frames have been submitted by now).
    fn collect(&mut self) {
        let Some(gpu) = self.gpu.as_mut() else { return };
        let mut arrived: Vec<(u64, Vec<(String, f64)>)> = Vec::new();
        for readback in gpu.readbacks.iter_mut() {
            match readback.state.load(Ordering::Acquire) {
                state::READY => {
                    let handle = readback.buffer.handle();
                    let ticks: Option<Vec<u64>> = match handle.get_mapped_range(..) {
                        Ok(view) => {
                            let values = bytemuck::pod_collect_to_vec(&view[..]);
                            drop(view);
                            Some(values)
                        }
                        Err(_) => None,
                    };
                    handle.unmap();
                    if let Some(ticks) = ticks {
                        let count = readback.names.len();
                        let milliseconds = milliseconds_from_ticks(&ticks[..(count * 2).min(ticks.len())], gpu.period_nanoseconds);
                        arrived.push((readback.frame, readback.names.drain(..).zip(milliseconds).collect()));
                    }
                    readback.names.clear();
                    readback.state.store(state::FREE, Ordering::Release);
                }
                state::FAILED => {
                    readback.names.clear();
                    readback.state.store(state::FREE, Ordering::Release);
                }
                state::COPIED => {
                    readback.state.store(state::MAPPING, Ordering::Release);
                    let shared = readback.state.clone();
                    readback.buffer.handle().map_async(wgpu::MapMode::Read, .., move |result| {
                        shared.store(if result.is_ok() { state::READY } else { state::FAILED }, Ordering::Release);
                    });
                }
                _ => {}
            }
        }
        arrived.sort_by_key(|(frame, _)| *frame);
        for (frame, passes) in arrived {
            self.apply_gpu(frame, &passes);
        }
    }

    /// Takes one frame's GPU results into the timings and the smoothed values.
    fn apply_gpu(&mut self, frame: u64, passes: &[(String, f64)]) {
        let summed = sum_by_name(passes);
        let total: f64 = summed.iter().map(|(_, ms)| ms).sum();
        self.timings.gpu_frame = Some(frame);
        self.timings.gpu_total_ms = Some(total);
        self.timings.gpu_total_ms_smoothed = Some(self.gpu_total.push(total));
        let window = self.smoothing_frames;
        self.gpu_passes.retain(|name, _| summed.iter().any(|(kept, _)| kept == name));
        self.timings.passes = summed
            .into_iter()
            .map(|(name, gpu_ms)| {
                let gpu_ms_smoothed = self.gpu_passes.entry(name.clone()).or_insert_with(|| RollingMean::new(window)).push(gpu_ms);
                PassTiming { name, gpu_ms, gpu_ms_smoothed }
            })
            .collect();
    }
}

/// The passes of one frame with the same name added together, in the order of first marking.
pub(crate) fn sum_by_name(passes: &[(String, f64)]) -> Vec<(String, f64)> {
    let mut summed: Vec<(String, f64)> = Vec::new();
    for (name, ms) in passes {
        match summed.iter_mut().find(|(kept, _)| kept == name) {
            Some((_, kept)) => *kept += ms,
            None => summed.push((name.clone(), *ms)),
        }
    }
    summed
}

/// The check of [`FrameTimerOptions::max_passes`].
pub(crate) fn check_max_passes(max_passes: u32) -> Result<(), GraphicsException> {
    if !(1..=FrameTimer::MAX_PASSES).contains(&max_passes) {
        return Err(GraphicsException::new(format!("A frame timer marks 1 to {} passes per frame; FrameTimerOptions::max_passes is {max_passes}.", FrameTimer::MAX_PASSES)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{check_max_passes, sum_by_name, FrameTimerOptions, FrameTimings, PassTiming, RollingMean};

    #[test]
    fn the_rolling_mean_averages_the_last_values_of_its_window() {
        let mut mean = RollingMean::new(3);
        assert_eq!(mean.mean(), 0.0);
        assert_eq!(mean.push(3.0), 3.0);
        assert_eq!(mean.push(6.0), 4.5);
        assert_eq!(mean.push(9.0), 6.0);
        // The oldest (3) leaves the window.
        assert_eq!(mean.push(12.0), 9.0);
        assert_eq!(mean.push(0.0), 7.0);
        // A window of 0 is a window of 1: the mean is the last value.
        let mut last = RollingMean::new(0);
        last.push(5.0);
        assert_eq!(last.push(2.0), 2.0);
    }

    #[test]
    fn the_rolling_mean_does_not_drift_over_many_frames() {
        let mut mean = RollingMean::new(30);
        for index in 0..100_000 {
            mean.push(if index % 2 == 0 { 0.1 } else { 16.7 });
        }
        assert!((mean.mean() - 8.4).abs() < 1e-9, "{}", mean.mean());
    }

    #[test]
    fn passes_of_one_name_are_added_in_the_order_first_marked() {
        let passes = vec![("scene".to_string(), 2.0), ("bloom".to_string(), 0.5), ("scene".to_string(), 1.0)];
        assert_eq!(sum_by_name(&passes), vec![("scene".to_string(), 3.0), ("bloom".to_string(), 0.5)]);
    }

    #[test]
    fn the_options_have_defaults_and_max_passes_is_checked() {
        let options = FrameTimerOptions::default();
        assert_eq!((options.max_passes, options.smoothing_frames), (16, 30));
        assert!(check_max_passes(1).is_ok());
        assert!(check_max_passes(2048).is_ok());
        assert!(check_max_passes(0).unwrap_err().message().contains("1 to 2048"));
        assert!(check_max_passes(2049).is_err());
    }

    #[test]
    fn the_timings_read_as_one_line_with_gpu_not_available() {
        let unavailable = FrameTimings { frame: 3, cpu_ms: 1.234, cpu_ms_smoothed: 1.1, counts: vec![("draw calls".to_string(), 12)], ..FrameTimings::default() };
        assert_eq!(unavailable.to_string(), "cpu 1.23 ms (1.10) | gpu n/a | draw calls 12");
        let waiting = FrameTimings { gpu_available: true, ..FrameTimings::default() };
        assert_eq!(waiting.to_string(), "cpu 0.00 ms (0.00) | gpu ...");
        let measured = FrameTimings {
            gpu_available: true,
            gpu_frame: Some(1),
            gpu_total_ms: Some(2.5),
            gpu_total_ms_smoothed: Some(2.25),
            passes: vec![PassTiming { name: "scene".to_string(), gpu_ms: 2.0, gpu_ms_smoothed: 1.75 }, PassTiming { name: "bloom".to_string(), gpu_ms: 0.5, gpu_ms_smoothed: 0.5 }],
            ..FrameTimings::default()
        };
        assert_eq!(measured.to_string(), "cpu 0.00 ms (0.00) | gpu 2.50 ms (2.25): scene 1.75, bloom 0.50");
    }
}
