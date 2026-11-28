//! GPU TIMING of the renderer's work (Not in C#): when it is switched on, two timestamps per
//! frame, at the beginning of the frame's FIRST render pass and at the end of its LAST, resolved,
//! copied into one of a few read-back buffers and mapped ASYNCHRONOUSLY, as the graphics crate's
//! `FrameTimer` reads its results: a frame's copy is mapped once that frame has been SUBMITTED
//! (for a layer at its next frame, the host submits every frame; for an offscreen renderer when
//! the application says so), read at a later frame, and its buffer is free again. Nothing ever
//! waits for the GPU. A buffer is copied into only when FREE and mapped only from COPIED, so a
//! buffer that is mapped or being mapped is never mapped again or used by a frame; a frame that
//! finds no buffer free is not timed. Results come a few frames late: the latest one wins, and
//! one from an older frame than the result already shown (or from before the timing was last
//! switched on) is dropped.
//!
//! OFF (the default) nothing of it exists on the GPU and every pass is recorded exactly as without
//! it. A device without `TIMESTAMP_QUERY` makes it a quiet no-op (no result, no error).
//!
//! The CPU halves (`TimerBook`: the read-back buffers' states and the results; `StampPlan`: which
//! pass writes which timestamp) are plain data, tested on the host; `GpuTiming` does what they say.

use std::cell::Cell;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use codebrix_webgpu::wgpu;

/// The read-back buffers the timing cycles through: how many frames' results may be on their way
/// back at once (the graphics crate's frame timer has as many).
pub(crate) const READBACK_BUFFERS: usize = 4;

/// The bytes of one frame's result: two 64-bit timestamps.
const RESULT_BYTES: u64 = 16;

/// Where one read-back buffer is (shared with its map callback, which may run on another thread
/// on a native target, hence the atomic).
pub(crate) mod state {
    /// Free to copy a frame's result into.
    pub(crate) const FREE: u8 = 0;
    /// A frame's result was copied into it; it is mapped once that frame was submitted.
    pub(crate) const COPIED: u8 = 1;
    /// Being mapped.
    pub(crate) const MAPPING: u8 = 2;
    /// Mapped: the result can be read (then it is unmapped and free again).
    pub(crate) const READY: u8 = 3;
    /// The map failed (the device may be lost); it is free again at the next collection.
    pub(crate) const FAILED: u8 = 4;
}

/// Milliseconds from a start and an end tick and the tick period in nanoseconds - the graphics
/// crate's arithmetic: 0 for an unwritten start or an end not after the start.
pub(crate) fn milliseconds_from_ticks(start: u64, end: u64, period_nanoseconds: f64) -> f64 {
    if start != 0 && end > start {
        (end - start) as f64 * period_nanoseconds / 1_000_000.0
    } else {
        0.0
    }
}

/// The CPU half: the read-back buffers' states, the frame each holds, and the latest result.
#[derive(Debug)]
pub(crate) struct TimerBook {
    enabled: bool,
    slots: Vec<Arc<AtomicU8>>,
    /// The frame each buffer's result is from.
    frames: Vec<u64>,
    /// Frames begun (timed or not).
    frame: u64,
    /// The first frame of the current switched-on period: results of earlier frames are dropped.
    enabled_from: u64,
    /// The latest result: (its frame, its milliseconds).
    latest: Option<(u64, f64)>,
}

impl TimerBook {
    /// A book of `buffers` read-back buffers, all free, switched off.
    pub(crate) fn new(buffers: usize) -> TimerBook {
        TimerBook { enabled: false, slots: (0..buffers).map(|_| Arc::new(AtomicU8::new(state::FREE))).collect(), frames: vec![0; buffers], frame: 0, enabled_from: 0, latest: None }
    }

    /// Switches the timing on or off. Off forgets the result shown; on again shows only results of
    /// frames timed from now. Buffers in flight finish their round either way.
    pub(crate) fn set_enabled(&mut self, on: bool) {
        if on && !self.enabled {
            self.enabled_from = self.frame + 1;
        }
        if !on {
            self.latest = None;
        }
        self.enabled = on;
    }

    /// The latest result in milliseconds (None while off, or before the first result arrives).
    pub(crate) fn latest_ms(&self) -> Option<f64> {
        if self.enabled { self.latest.map(|(_, ms)| ms) } else { None }
    }

    /// Takes the mapped results: for every READY buffer `read(buffer)` returns its two ticks (and
    /// unmaps it) or None; the buffer is free again. A FAILED buffer is free again too.
    pub(crate) fn collect(&mut self, period_nanoseconds: f64, mut read: impl FnMut(usize) -> Option<[u64; 2]>) {
        for i in 0..self.slots.len() {
            match self.slots[i].load(Ordering::Acquire) {
                state::READY => {
                    let ticks = read(i);
                    self.slots[i].store(state::FREE, Ordering::Release);
                    let frame = self.frames[i];
                    if let Some([start, end]) = ticks {
                        let newer = self.latest.is_none_or(|(shown, _)| frame > shown);
                        if self.enabled && frame >= self.enabled_from && newer {
                            self.latest = Some((frame, milliseconds_from_ticks(start, end, period_nanoseconds)));
                        }
                    }
                }
                state::FAILED => self.slots[i].store(state::FREE, Ordering::Release),
                _ => {}
            }
        }
    }

    /// The frames that copied into buffers have been submitted: the buffers to map now (they are
    /// MAPPING from here on, until their callbacks say READY or FAILED).
    pub(crate) fn submitted(&mut self) -> Vec<usize> {
        let mut map = Vec::new();
        for (i, slot) in self.slots.iter().enumerate() {
            if slot.load(Ordering::Acquire) == state::COPIED {
                slot.store(state::MAPPING, Ordering::Release);
                map.push(i);
            }
        }
        map
    }

    /// A new frame begins: the buffer its result goes to (None: switched off, or no buffer free -
    /// the frame is not timed).
    pub(crate) fn begin_frame(&mut self) -> Option<usize> {
        self.frame += 1;
        if !self.enabled {
            return None;
        }
        self.slots.iter().position(|s| s.load(Ordering::Acquire) == state::FREE)
    }

    /// The frame's result was copied into `buffer` (it is mapped once the frame was submitted).
    pub(crate) fn copied(&mut self, buffer: usize) {
        self.frames[buffer] = self.frame;
        self.slots[buffer].store(state::COPIED, Ordering::Release);
    }

    /// The state a buffer's map callback sets.
    pub(crate) fn shared_state(&self, buffer: usize) -> Arc<AtomicU8> {
        self.slots[buffer].clone()
    }

    /// A buffer's state (tests).
    #[cfg(test)]
    pub(crate) fn state(&self, buffer: usize) -> u8 {
        self.slots[buffer].load(Ordering::Acquire)
    }
}

/// Which timestamps a frame's passes write: the beginning of the first pass begun, the end of
/// the pass marked last (once). Inactive (an untimed frame) it writes none.
#[derive(Debug)]
pub(crate) struct StampPlan {
    active: bool,
    first: Cell<bool>,
    ended: Cell<bool>,
}

impl StampPlan {
    /// The plan of a timed (`active`) or untimed frame.
    pub(crate) fn new(active: bool) -> StampPlan {
        StampPlan { active, first: Cell::new(true), ended: Cell::new(false) }
    }

    /// The query indices the next pass writes (beginning, end), or None for neither.
    pub(crate) fn indices(&self, last: bool) -> Option<(Option<u32>, Option<u32>)> {
        if !self.active {
            return None;
        }
        let begin = self.first.replace(false);
        let end = last && !self.ended.get();
        if end {
            self.ended.set(true);
        }
        (begin || end).then_some((begin.then_some(0), end.then_some(1)))
    }

    /// True when the frame is timed and no pass has written the end yet.
    pub(crate) fn needs_end(&self) -> bool {
        self.active && !self.ended.get()
    }
}

/// The timestamps of one frame's passes: the plan and the query set it writes into.
pub(crate) struct PassStamps {
    query_set: Option<wgpu::QuerySet>,
    plan: StampPlan,
}

impl PassStamps {
    /// The stamps of a frame timed into `query_set` (None: an untimed frame - every pass gets
    /// `timestamp_writes: None`, as without the timing).
    pub(crate) fn new(query_set: Option<wgpu::QuerySet>) -> PassStamps {
        let plan = StampPlan::new(query_set.is_some());
        PassStamps { query_set, plan }
    }

    /// The `timestamp_writes` of the next pass begun (`last`: the frame's last pass).
    pub(crate) fn writes(&self, last: bool) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let query_set = self.query_set.as_ref()?;
        let (beginning_of_pass_write_index, end_of_pass_write_index) = self.plan.indices(last)?;
        Some(wgpu::RenderPassTimestampWrites { query_set, beginning_of_pass_write_index, end_of_pass_write_index })
    }

    /// True when no pass of the timed frame has written the end yet.
    pub(crate) fn needs_end(&self) -> bool {
        self.plan.needs_end()
    }
}

/// The GPU objects of the timing, made at the first timed frame.
struct Parts {
    query_set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readbacks: Vec<wgpu::Buffer>,
    period_nanoseconds: f64,
}

/// The GPU half: the query set, the resolve buffer and the read-back buffers on one device.
pub(crate) struct GpuTiming {
    book: TimerBook,
    supported: bool,
    /// True for a layer (the host submits every frame, so a frame's copy is mapped at the next
    /// frame); false for an offscreen renderer (mapped when the application says it submitted).
    map_on_next_frame: bool,
    parts: Option<Parts>,
    /// The buffer of the frame being encoded.
    slot: Option<usize>,
}

impl GpuTiming {
    /// The timing on `device`, switched off (supported when the device has `TIMESTAMP_QUERY`).
    pub(crate) fn new(device: &wgpu::Device) -> GpuTiming {
        GpuTiming { book: TimerBook::new(READBACK_BUFFERS), supported: device.features().contains(wgpu::Features::TIMESTAMP_QUERY), map_on_next_frame: true, parts: None, slot: None }
    }

    /// Switches the timing on or off.
    pub(crate) fn set_enabled(&mut self, on: bool) {
        self.book.set_enabled(on);
    }

    /// True when the device has `TIMESTAMP_QUERY`.
    pub(crate) fn supported(&self) -> bool {
        self.supported
    }

    /// See the field (an offscreen renderer sets false).
    pub(crate) fn set_map_on_next_frame(&mut self, on: bool) {
        self.map_on_next_frame = on;
    }

    /// The latest result in milliseconds (None while off, unsupported or not yet known).
    pub(crate) fn latest_ms(&self) -> Option<f64> {
        if self.supported { self.book.latest_ms() } else { None }
    }

    /// Takes the results whose maps have finished.
    pub(crate) fn collect(&mut self) {
        let Some(parts) = self.parts.as_ref() else { return };
        self.book.collect(parts.period_nanoseconds, |i| {
            let buffer = &parts.readbacks[i];
            let ticks = match buffer.get_mapped_range(..) {
                Ok(view) => {
                    let word = |at: usize| view.get(at..at + 8).map(|b| u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]));
                    match (word(0), word(8)) {
                        (Some(start), Some(end)) => Some([start, end]),
                        _ => None,
                    }
                }
                Err(_) => None,
            };
            buffer.unmap();
            ticks
        });
    }

    /// The frames that copied a result have been submitted: their buffers are mapped now.
    pub(crate) fn submitted(&mut self) {
        let Some(parts) = self.parts.as_ref() else { return };
        for i in self.book.submitted() {
            let shared = self.book.shared_state(i);
            parts.readbacks[i].map_async(wgpu::MapMode::Read, .., move |result| {
                shared.store(if result.is_ok() { state::READY } else { state::FAILED }, Ordering::Release);
            });
        }
    }

    /// A frame begins: collects what has arrived (and, for a layer, maps the last frame's copy),
    /// and returns the query set to time this frame into (None: an untimed frame).
    pub(crate) fn begin_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Option<wgpu::QuerySet> {
        self.collect();
        if self.map_on_next_frame {
            self.submitted();
        }
        self.slot = None;
        if !self.supported {
            return None;
        }
        let slot = self.book.begin_frame()?;
        let parts = self.parts.get_or_insert_with(|| {
            let query_set = device.create_query_set(&wgpu::QuerySetDescriptor { label: Some("animation gpu timing"), ty: wgpu::QueryType::Timestamp, count: 2 });
            let resolve = device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation gpu timing resolve"), size: RESULT_BYTES, usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
            let readbacks = (0..READBACK_BUFFERS).map(|_| device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation gpu timing read-back"), size: RESULT_BYTES, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false })).collect();
            Parts { query_set, resolve, readbacks, period_nanoseconds: queue.get_timestamp_period() as f64 }
        });
        self.slot = Some(slot);
        Some(parts.query_set.clone())
    }

    /// The frame's last pass has been recorded: the resolve of its two timestamps and their copy
    /// into the frame's read-back buffer (outside any pass). Nothing for an untimed frame.
    pub(crate) fn end_frame(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let (Some(slot), Some(parts)) = (self.slot.take(), self.parts.as_ref()) else { return };
        encoder.resolve_query_set(&parts.query_set, 0..2, &parts.resolve, 0);
        encoder.copy_buffer_to_buffer(&parts.resolve, 0, &parts.readbacks[slot], 0, RESULT_BYTES);
        self.book.copied(slot);
    }
}

#[cfg(test)]
mod tests {
    use super::{milliseconds_from_ticks, state, StampPlan, TimerBook};
    use std::sync::atomic::Ordering;

    /// What a map callback does.
    fn map_done(book: &TimerBook, buffer: usize, ok: bool) {
        book.shared_state(buffer).store(if ok { state::READY } else { state::FAILED }, Ordering::Release);
    }

    #[test]
    fn ticks_become_milliseconds_as_in_the_graphics_crate() {
        assert_eq!(milliseconds_from_ticks(1_000_000, 3_500_000, 1.0), 2.5);
        assert_eq!(milliseconds_from_ticks(0, 9, 1.0), 0.0);
        assert_eq!(milliseconds_from_ticks(50, 40, 1.0), 0.0);
        assert_eq!(milliseconds_from_ticks(10, 20, 1_000_000.0), 10.0);
    }

    #[test]
    fn off_times_no_frame_and_shows_nothing() {
        //Arrange
        let mut book = TimerBook::new(4);
        //Act
        let slot = book.begin_frame();
        //Assert
        assert_eq!(slot, None);
        assert_eq!(book.latest_ms(), None);
        assert!(book.submitted().is_empty());
    }

    #[test]
    fn a_frame_is_copied_mapped_after_its_submit_read_later_and_its_buffer_freed() {
        //Arrange
        let mut book = TimerBook::new(4);
        book.set_enabled(true);
        //Act: frame 1 is timed into buffer 0.
        let slot = book.begin_frame().unwrap();
        book.copied(slot);
        //Assert: not mapped before the frame is submitted.
        assert_eq!(book.state(0), state::COPIED);
        assert_eq!(book.latest_ms(), None);
        // Frame 2: frame 1 was submitted - buffer 0 is mapped, frame 2 takes buffer 1.
        assert_eq!(book.submitted(), vec![0]);
        assert_eq!(book.state(0), state::MAPPING);
        assert_eq!(book.begin_frame(), Some(1));
        // Mapping twice never happens: a MAPPING buffer is not mapped again.
        assert!(book.submitted().is_empty());
        // The map finishes; the next collection reads it and frees the buffer.
        map_done(&book, 0, true);
        let mut read = Vec::new();
        book.collect(1.0, |i| {
            read.push(i);
            Some([1_000_000, 3_000_000])
        });
        assert_eq!(read, vec![0]);
        assert_eq!(book.state(0), state::FREE);
        assert_eq!(book.latest_ms(), Some(2.0));
    }

    #[test]
    fn a_frame_with_every_buffer_busy_is_not_timed() {
        //Arrange
        let mut book = TimerBook::new(2);
        book.set_enabled(true);
        for _ in 0..2 {
            let slot = book.begin_frame().unwrap();
            book.copied(slot);
        }
        //Act
        let third = book.begin_frame();
        book.submitted();
        let fourth = book.begin_frame();
        //Assert: copied or mapping buffers are never handed out.
        assert_eq!(third, None);
        assert_eq!(fourth, None);
        assert_eq!((book.state(0), book.state(1)), (state::MAPPING, state::MAPPING));
    }

    #[test]
    fn a_result_arriving_after_a_newer_one_is_dropped() {
        //Arrange: frames 1 and 2 in buffers 0 and 1.
        let mut book = TimerBook::new(4);
        book.set_enabled(true);
        for _ in 0..2 {
            let slot = book.begin_frame().unwrap();
            book.copied(slot);
        }
        book.submitted();
        //Act: frame 2's map finishes first.
        map_done(&book, 1, true);
        book.collect(1.0, |_| Some([1_000_000, 5_000_000]));
        let newer = book.latest_ms();
        map_done(&book, 0, true);
        book.collect(1.0, |_| Some([1_000_000, 2_000_000]));
        //Assert
        assert_eq!(newer, Some(4.0));
        assert_eq!(book.latest_ms(), Some(4.0), "frame 1's late result does not replace frame 2's");
        assert_eq!((book.state(0), book.state(1)), (state::FREE, state::FREE));
    }

    #[test]
    fn a_failed_map_frees_its_buffer_without_a_result() {
        //Arrange
        let mut book = TimerBook::new(1);
        book.set_enabled(true);
        let slot = book.begin_frame().unwrap();
        book.copied(slot);
        book.submitted();
        //Act
        map_done(&book, 0, false);
        let mut reads = 0;
        book.collect(1.0, |_| {
            reads += 1;
            None
        });
        //Assert
        assert_eq!(reads, 0, "a failed map is not read");
        assert_eq!(book.state(0), state::FREE);
        assert_eq!(book.latest_ms(), None);
        assert_eq!(book.begin_frame(), Some(0));
    }

    #[test]
    fn switching_off_mid_flight_finishes_the_round_and_drops_its_results() {
        //Arrange: frame 1 timed and copied.
        let mut book = TimerBook::new(4);
        book.set_enabled(true);
        let slot = book.begin_frame().unwrap();
        book.copied(slot);
        //Act: switched off; the buffer still goes its way (mapped, read, freed).
        book.set_enabled(false);
        let untimed = book.begin_frame();
        let mapped = book.submitted();
        map_done(&book, 0, true);
        book.collect(1.0, |_| Some([1, 1_000_001]));
        //Assert
        assert_eq!(untimed, None);
        assert_eq!(mapped, vec![0]);
        assert_eq!(book.state(0), state::FREE);
        assert_eq!(book.latest_ms(), None, "None while off");
    }

    #[test]
    fn switched_on_again_it_shows_only_frames_timed_since() {
        //Arrange: frame 1 timed, then off and on again before its result arrives.
        let mut book = TimerBook::new(4);
        book.set_enabled(true);
        let first = book.begin_frame().unwrap();
        book.copied(first);
        book.set_enabled(false);
        book.set_enabled(true);
        //Act: frame 2 is timed too; both results arrive.
        assert_eq!(book.submitted(), vec![first]);
        let second = book.begin_frame().unwrap();
        book.copied(second);
        map_done(&book, first, true);
        book.collect(1.0, |_| Some([1, 7_000_001]));
        let after_old = book.latest_ms();
        book.submitted();
        map_done(&book, second, true);
        book.collect(1.0, |_| Some([1, 3_000_001]));
        //Assert
        assert_eq!(after_old, None, "frame 1 was timed before the timing was switched on again");
        assert_eq!(book.latest_ms(), Some(3.0));
    }

    #[test]
    fn the_first_pass_writes_the_beginning_and_the_last_the_end_once() {
        //Arrange
        let plan = StampPlan::new(true);
        //Act
        let first = plan.indices(false);
        let middle = plan.indices(false);
        let before_end = plan.needs_end();
        let last = plan.indices(true);
        let again = plan.indices(true);
        //Assert
        assert_eq!(first, Some((Some(0), None)));
        assert_eq!(middle, None, "a pass in between is recorded as without timing");
        assert!(before_end);
        assert_eq!(last, Some((None, Some(1))));
        assert_eq!(again, None, "the end is written once");
        assert!(!plan.needs_end());
    }

    #[test]
    fn a_single_pass_writes_both_and_an_untimed_frame_writes_none() {
        //Arrange
        let single = StampPlan::new(true);
        let untimed = StampPlan::new(false);
        //Act
        let both = single.indices(true);
        //Assert
        assert_eq!(both, Some((Some(0), Some(1))));
        assert_eq!(untimed.indices(false), None);
        assert_eq!(untimed.indices(true), None);
        assert!(!untimed.needs_end());
    }
}
