//! The canvas clock: ONE clock per canvas host (or shared by several canvases) that turns the
//! browser's animation-frame timestamps into the frame time every layer draws with - elapsed
//! time, the time since the previous frame, and the frame number - and that pauses, resumes and
//! runs faster or slower for every layer at once.

use std::cell::RefCell;
use std::rc::Rc;

use crate::time_points::TimePoints;

/// The clock of a canvas. Every layer of the canvas gets its frame time from it
/// ([`FrameInfo`](crate::FrameInfo)), so pausing it or changing its speed keeps every layer in
/// step. Clone it to share it: the clones are the same clock.
///
/// Keep it in the page (`use_hook(CanvasClock::new)`) and hand it to
/// [`WebGpuCanvas`](crate::WebGpuCanvas) as `clock` to control it from the page (a pause key or
/// button); a canvas given none has a clock of its own. Several canvases may share one clock:
/// it advances once per animation frame, however many canvases draw in that frame.
#[derive(Clone, Default)]
pub struct CanvasClock {
    inner: Rc<RefCell<ClockState>>,
}

impl CanvasClock {
    /// A running clock at normal speed, at time 0.
    pub fn new() -> CanvasClock {
        CanvasClock::default()
    }

    /// Stops the clock: from the next frame on, the time stands still and the delta is 0 (the
    /// frames are still drawn and counted - unless the canvas idles while its clock is paused,
    /// `WebGpuCanvas`'s `idle_when_paused`).
    pub fn pause(&self) {
        self.inner.borrow_mut().pause();
    }

    /// Starts the clock again, from the time at which it was paused.
    pub fn resume(&self) {
        self.inner.borrow_mut().resume();
    }

    /// Pauses a running clock or resumes a paused one; true when it is now paused.
    pub fn toggle(&self) -> bool {
        let mut state = self.inner.borrow_mut();
        if state.paused {
            state.resume();
        } else {
            state.pause();
        }
        state.paused
    }

    /// True while the clock is paused.
    pub fn is_paused(&self) -> bool {
        self.inner.borrow().paused
    }

    /// Sets how fast the clock runs: 1 is real time, 0.5 half speed, 2 double. A negative or
    /// non-finite speed is taken as 0 (the time stands still, as when paused).
    pub fn set_speed(&self, speed: f64) {
        self.inner.borrow_mut().set_speed(speed);
    }

    /// How fast the clock runs (1 is real time).
    pub fn speed(&self) -> f64 {
        self.inner.borrow().speed
    }

    /// The clock's time, in seconds, at the last frame.
    pub fn elapsed(&self) -> f64 {
        self.inner.borrow().elapsed
    }

    /// The number of the last frame (0 before the first).
    pub fn frames(&self) -> u64 {
        self.inner.borrow().frames
    }

    /// Moves the clock to `seconds` on its timeline (for example 0 to start again); the next
    /// frame shows that time. Points between the old and the new time count as crossed when the
    /// clock moves forward, not when it moves back.
    pub fn seek(&self, seconds: f64) {
        self.inner.borrow_mut().seek(seconds);
    }

    /// Defines a named point in time on the clock's timeline, or moves an existing one: every
    /// layer then refers to it by name ([`CanvasFrame::points`](crate::CanvasFrame::points)).
    pub fn define_point(&self, name: &str, seconds: f64) {
        let mut state = self.inner.borrow_mut();
        state.points.define(name, seconds);
        state.changed();
    }

    /// Removes a named point; false when there was none of that name.
    pub fn remove_point(&self, name: &str) -> bool {
        let mut state = self.inner.borrow_mut();
        let removed = state.points.remove(name);
        if removed {
            state.changed();
        }
        removed
    }

    /// The named points as they are now (each frame carries the same, as they were when it began).
    pub fn points(&self) -> TimePoints {
        self.inner.borrow().points.clone()
    }

    /// Advances the clock to the browser's frame timestamp (milliseconds).
    pub(crate) fn tick(&self, timestamp_ms: f64) -> Tick {
        self.inner.borrow_mut().tick(timestamp_ms)
    }

    /// The count of the changes that alter what a paused clock's frame shows (a seek, a resume, a
    /// named point defined, moved or removed): a canvas that idles while its clock is paused draws
    /// again when it moves.
    pub(crate) fn changes(&self) -> u64 {
        self.inner.borrow().changes
    }

    /// Two handles are equal when they are the same clock.
    fn same(&self, other: &CanvasClock) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl PartialEq for CanvasClock {
    fn eq(&self, other: &Self) -> bool {
        self.same(other)
    }
}

/// One frame's time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Tick {
    /// The frame's number, from 1.
    pub(crate) number: u64,
    /// The clock's time in seconds.
    pub(crate) time: f64,
    /// Clock seconds since the previous frame.
    pub(crate) delta: f64,
    /// The clock's time at the previous frame (None for the first frame).
    pub(crate) previous: Option<f64>,
}

/// The clock's arithmetic (no browser needed).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ClockState {
    last_timestamp: Option<f64>,
    last_tick: Option<Tick>,
    elapsed: f64,
    frames: u64,
    paused: bool,
    /// The next frame after a resume adds no time (the pause is not counted).
    skip_next: bool,
    speed: f64,
    pub(crate) points: TimePoints,
    /// See [`CanvasClock::changes`].
    changes: u64,
}

impl Default for ClockState {
    fn default() -> Self {
        ClockState { last_timestamp: None, last_tick: None, elapsed: 0.0, frames: 0, paused: false, skip_next: false, speed: 1.0, points: TimePoints::default(), changes: 0 }
    }
}

impl ClockState {
    pub(crate) fn pause(&mut self) {
        self.paused = true;
    }

    pub(crate) fn resume(&mut self) {
        if self.paused {
            self.paused = false;
            self.skip_next = true;
            self.changed();
        }
    }

    pub(crate) fn seek(&mut self, seconds: f64) {
        self.elapsed = if seconds.is_finite() { seconds } else { 0.0 };
        self.skip_next = true;
        self.changed();
    }

    fn changed(&mut self) {
        self.changes = self.changes.wrapping_add(1);
    }

    pub(crate) fn set_speed(&mut self, speed: f64) {
        self.speed = if speed.is_finite() && speed > 0.0 { speed } else { 0.0 };
    }

    /// The frame time at the browser's timestamp. The same timestamp again (another canvas in
    /// the same animation frame) gives the same tick; a timestamp that goes back adds no time.
    pub(crate) fn tick(&mut self, timestamp_ms: f64) -> Tick {
        if let (Some(last), Some(tick)) = (self.last_timestamp, self.last_tick) {
            if last == timestamp_ms {
                return tick;
            }
        }
        let real = match self.last_timestamp {
            Some(last) => ((timestamp_ms - last) / 1000.0).max(0.0),
            None => 0.0,
        };
        self.last_timestamp = Some(timestamp_ms);
        let delta = if self.paused || self.skip_next { 0.0 } else { real * self.speed };
        if !self.paused {
            self.skip_next = false;
        }
        let previous = self.last_tick.map(|t| t.time);
        self.elapsed += delta;
        self.frames += 1;
        let tick = Tick { number: self.frames, time: self.elapsed, delta, previous };
        self.last_tick = Some(tick);
        tick
    }
}

#[cfg(test)]
mod tests {
    use super::{CanvasClock, ClockState, Tick};

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn the_first_frame_is_number_one_at_time_zero() {
        let mut clock = ClockState::default();
        assert_eq!(clock.tick(1234.5), Tick { number: 1, time: 0.0, delta: 0.0, previous: None });
    }

    #[test]
    fn later_frames_count_seconds_from_the_first() {
        let mut clock = ClockState::default();
        clock.tick(1000.0);
        let second = clock.tick(1016.0);
        assert_eq!(second.number, 2);
        assert!(close(second.time, 0.016) && close(second.delta, 0.016));
        let third = clock.tick(1500.0);
        assert!(close(third.time, 0.5) && close(third.delta, 0.484));
    }

    #[test]
    fn a_paused_clock_stands_still_and_still_counts_frames() {
        let mut clock = ClockState::default();
        clock.tick(0.0);
        clock.tick(1000.0);
        clock.pause();
        let paused = clock.tick(2000.0);
        let later = clock.tick(5000.0);
        assert!(close(paused.time, 1.0) && close(later.time, 1.0));
        assert_eq!(later.delta, 0.0);
        assert_eq!(later.number, 4);
    }

    #[test]
    fn a_resumed_clock_continues_from_the_paused_time() {
        let mut clock = ClockState::default();
        clock.tick(0.0);
        clock.tick(1000.0);
        clock.pause();
        clock.tick(2000.0);
        clock.resume();
        let first = clock.tick(60000.0);
        let next = clock.tick(60500.0);
        assert!(close(first.time, 1.0), "the pause is not counted");
        assert_eq!(first.delta, 0.0);
        assert!(close(next.time, 1.5) && close(next.delta, 0.5));
    }

    #[test]
    fn the_speed_scales_the_time() {
        let mut clock = ClockState::default();
        clock.tick(0.0);
        clock.set_speed(2.0);
        assert!(close(clock.tick(1000.0).time, 2.0));
        clock.set_speed(0.25);
        let tick = clock.tick(2000.0);
        assert!(close(tick.time, 2.25) && close(tick.delta, 0.25));
    }

    #[test]
    fn a_negative_or_nonsense_speed_stands_still() {
        let mut clock = ClockState::default();
        clock.tick(0.0);
        clock.set_speed(-1.0);
        assert_eq!(clock.tick(1000.0).time, 0.0);
        clock.set_speed(f64::NAN);
        assert_eq!(clock.tick(2000.0).time, 0.0);
    }

    #[test]
    fn the_same_timestamp_gives_the_same_tick() {
        let mut clock = ClockState::default();
        clock.tick(0.0);
        let first = clock.tick(500.0);
        assert_eq!(clock.tick(500.0), first, "a second canvas in the same animation frame");
        assert_eq!(clock.tick(1000.0).number, 3);
    }

    #[test]
    fn a_timestamp_that_goes_back_adds_no_time() {
        let mut clock = ClockState::default();
        clock.tick(1000.0);
        assert_eq!(clock.tick(990.0).delta, 0.0);
    }

    #[test]
    fn a_seek_sets_the_time_and_the_next_frame_shows_it() {
        let mut clock = ClockState::default();
        clock.tick(0.0);
        clock.tick(3000.0);
        clock.seek(0.0);
        let after = clock.tick(3016.0);
        assert_eq!((after.time, after.delta, after.previous), (0.0, 0.0, Some(3.0)));
        assert!(close(clock.tick(3516.0).time, 0.5));
    }

    #[test]
    fn a_seek_before_the_first_frame_is_the_first_frames_time_while_running() {
        //Arrange: a page that seeks its running clock (a `?t=12` address) before the canvas draws
        let clock = CanvasClock::new();
        clock.seek(12.0);
        //Act
        let first = clock.tick(5000.0);
        let second = clock.tick(5016.0);
        let third = clock.tick(5516.0);
        //Assert: the first frame shows 12 s with delta 0, and the clock runs on from there
        assert_eq!(first, Tick { number: 1, time: 12.0, delta: 0.0, previous: None });
        assert!(close(second.time, 12.016) && close(second.delta, 0.016), "{second:?}");
        assert_eq!(second.previous, Some(12.0));
        assert!(close(third.time, 12.516), "{third:?}");
        assert!(!clock.is_paused());
    }

    #[test]
    fn a_seek_before_the_first_frame_of_a_paused_clock_holds_that_time() {
        //Arrange
        let clock = CanvasClock::new();
        clock.seek(12.0);
        clock.pause();
        //Act
        let first = clock.tick(5000.0);
        let later = clock.tick(6000.0);
        //Assert
        assert_eq!((first.time, later.time, later.delta), (12.0, 12.0, 0.0));
    }

    #[test]
    fn each_tick_knows_the_previous_frames_time() {
        let mut clock = ClockState::default();
        clock.tick(0.0);
        clock.tick(1000.0);
        clock.pause();
        let paused = clock.tick(2000.0);
        assert_eq!(paused.previous, Some(1.0));
        assert_eq!(paused.time, 1.0);
    }

    #[test]
    fn the_points_are_defined_on_the_clock() {
        let clock = CanvasClock::new();
        clock.define_point("title in", 4.0);
        let copy = clock.points();
        clock.define_point("title in", 5.0);
        assert_eq!(copy.time("title in"), Some(4.0));
        assert_eq!(clock.points().time("title in"), Some(5.0));
        assert!(clock.remove_point("title in"));
    }

    #[test]
    fn what_a_paused_frame_shows_is_counted_as_a_change() {
        //Arrange
        let clock = CanvasClock::new();
        let start = clock.changes();
        //Act + Assert: each of these changes what the next frame shows
        clock.seek(3.0);
        let after_seek = clock.changes();
        clock.define_point("title in", 4.0);
        let after_define = clock.changes();
        clock.pause();
        clock.set_speed(2.0);
        assert_eq!(clock.changes(), after_define, "a pause and a speed change show the same frame");
        clock.resume();
        let after_resume = clock.changes();
        assert!(clock.remove_point("title in"));
        let after_remove = clock.changes();
        assert!(!clock.remove_point("title in"));
        clock.tick(1000.0);
        assert!(start < after_seek && after_seek < after_define && after_define < after_resume && after_resume < after_remove);
        assert_eq!(clock.changes(), after_remove, "nothing was removed, and a frame is no change");
    }

    #[test]
    fn the_handle_pauses_and_resumes_for_all_its_clones() {
        let clock = CanvasClock::new();
        let clone = clock.clone();
        assert!(clock.toggle());
        assert!(clone.is_paused());
        assert!(!clone.toggle());
        assert!(!clock.is_paused());
        assert!(clock == clone && clock != CanvasClock::new());
        clock.set_speed(3.0);
        assert_eq!(clone.speed(), 3.0);
    }
}
