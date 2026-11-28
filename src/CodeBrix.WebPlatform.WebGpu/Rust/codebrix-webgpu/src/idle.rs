//! Whether the canvas host draws an animation frame: every frame (the default), or - on a canvas
//! with `idle_when_paused` - while its clock is paused only when drawing again would show
//! something new (WebGpuCanvas's documentation lists what). The browser keeps showing the last
//! picture presented, so a frame that is not drawn costs no GPU work at all. Not in C#.

/// A loop held back this long or longer (a hidden page, a background tab) draws once when it
/// comes back, in case the browser let the last picture go.
pub(crate) const HELD_BACK_MS: f64 = 500.0;

/// What a drawn frame showed: the surface's size and pixel ratio, and the change counts of the
/// layers ([`CanvasLayers`](crate::CanvasLayers)) and of the clock
/// ([`CanvasClock`](crate::CanvasClock)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Shown {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pixel_ratio: f64,
    pub(crate) layers: u64,
    pub(crate) clock: u64,
}

/// One canvas's record of its last drawn frame and of what asks for another.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct IdleWatch {
    shown: Option<Shown>,
    pending: bool,
    last_frame_ms: Option<f64>,
}

impl IdleWatch {
    /// Whether the animation frame at `time_ms` (the browser's timestamp) is drawn, `now` being
    /// what it would show. True on every frame of a canvas that does not idle, and of a running
    /// clock; on an idling canvas with a paused clock, only for the first frame, a new size or
    /// pixel ratio, a change of the layers or a redraw request, a seek, resume or named point of
    /// the clock, an input event ([`IdleWatch::request`]), a frame that could not be drawn, and
    /// after the loop was held back.
    pub(crate) fn decide(&mut self, idle_when_paused: bool, paused: bool, now: Shown, time_ms: f64) -> bool {
        if self.last_frame_ms.is_some_and(|last| time_ms - last >= HELD_BACK_MS) {
            self.pending = true;
        }
        self.last_frame_ms = Some(time_ms);
        !idle_when_paused || !paused || self.pending || self.shown != Some(now)
    }

    /// The frame was drawn and presented, showing `now`.
    pub(crate) fn drawn(&mut self, now: Shown) {
        self.shown = Some(now);
        self.pending = false;
    }

    /// Asks for the next frame (an input event reached the layers, or the frame could not be
    /// drawn).
    pub(crate) fn request(&mut self) {
        self.pending = true;
    }
}

#[cfg(test)]
mod tests {
    use super::{IdleWatch, Shown};

    fn shown() -> Shown {
        Shown { width: 800, height: 600, pixel_ratio: 1.0, layers: 0, clock: 0 }
    }

    /// The watch after one drawn frame of `shown()` at time 0.
    fn drawn_once() -> IdleWatch {
        let mut watch = IdleWatch::default();
        assert!(watch.decide(true, true, shown(), 0.0));
        watch.drawn(shown());
        watch
    }

    #[test]
    fn a_canvas_that_does_not_idle_draws_every_frame() {
        let mut watch = IdleWatch::default();
        for frame in 0..5 {
            assert!(watch.decide(false, true, shown(), frame as f64 * 16.0));
            watch.drawn(shown());
        }
    }

    #[test]
    fn a_running_clock_draws_every_frame() {
        let mut watch = drawn_once();
        assert!(watch.decide(true, false, shown(), 16.0));
        watch.drawn(shown());
        assert!(watch.decide(true, false, shown(), 32.0));
    }

    #[test]
    fn a_paused_clock_draws_its_first_frame_and_then_idles() {
        let mut watch = drawn_once();
        assert!(!watch.decide(true, true, shown(), 16.0));
        assert!(!watch.decide(true, true, shown(), 32.0));
    }

    #[test]
    fn a_resize_or_a_new_pixel_ratio_draws_again() {
        let mut watch = drawn_once();
        assert!(watch.decide(true, true, Shown { width: 400, ..shown() }, 16.0));
        watch.drawn(Shown { width: 400, ..shown() });
        assert!(!watch.decide(true, true, Shown { width: 400, ..shown() }, 32.0));
        assert!(watch.decide(true, true, Shown { width: 400, pixel_ratio: 2.0, ..shown() }, 48.0));
    }

    #[test]
    fn a_change_of_the_layers_or_of_the_clock_draws_again() {
        let mut watch = drawn_once();
        assert!(watch.decide(true, true, Shown { layers: 1, ..shown() }, 16.0), "a layer added, or a redraw request");
        watch.drawn(Shown { layers: 1, ..shown() });
        assert!(watch.decide(true, true, Shown { layers: 1, clock: 1, ..shown() }, 32.0), "a seek of the paused clock");
    }

    #[test]
    fn a_request_draws_once() {
        let mut watch = drawn_once();
        watch.request();
        assert!(watch.decide(true, true, shown(), 16.0));
        watch.drawn(shown());
        assert!(!watch.decide(true, true, shown(), 32.0));
    }

    #[test]
    fn a_frame_that_was_not_drawn_is_tried_again() {
        //Arrange: the next frame is wanted but the surface gives no texture
        let mut watch = drawn_once();
        watch.request();
        assert!(watch.decide(true, true, shown(), 16.0));
        //Act: not drawn (no `drawn` call)
        let next = watch.decide(true, true, shown(), 32.0);
        //Assert
        assert!(next);
    }

    #[test]
    fn a_loop_held_back_draws_once_when_it_comes_back() {
        let mut watch = drawn_once();
        assert!(!watch.decide(true, true, shown(), 400.0));
        assert!(watch.decide(true, true, shown(), 900.0), "500 ms without an animation frame: the page was hidden");
        watch.drawn(shown());
        assert!(!watch.decide(true, true, shown(), 916.0));
    }
}
