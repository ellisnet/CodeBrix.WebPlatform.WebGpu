//! The tap count of a pointer press (the `tap_count` of
//! [`CanvasInput::PointerDown`](crate::CanvasInput::PointerDown)): presses that follow each other
//! quickly at nearly the same place form one series - 1 for a single press, 2 for the second press
//! of a double-click or a double tap, 3 for a triple. Not in C#.

use crate::input::{PointerButton, PointerKind};

/// When a press continues the previous press's series ([`WebGpuCanvas`](crate::WebGpuCanvas)'s
/// `tap_options`): within `interval_ms` of it, within a distance of it, with the same kind of
/// pointer and the same button, and with no other pointer down on the canvas (a second finger
/// of a pinch starts no double tap). Not in C#.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapOptions {
    /// The longest time from one press to the next of the same series, in milliseconds (default
    /// 500, the usual double-click time).
    pub interval_ms: f64,
    /// The farthest a mouse press may be from the previous one, in CSS pixels (default 5).
    pub mouse_distance: f64,
    /// The farthest a finger's or a pen's press may be from the previous one, in CSS pixels
    /// (default 24: a fingertip lands less precisely than a mouse pointer).
    pub touch_distance: f64,
}

impl Default for TapOptions {
    fn default() -> Self {
        TapOptions { interval_ms: 500.0, mouse_distance: 5.0, touch_distance: 24.0 }
    }
}

/// The last press and its count.
#[derive(Clone, Copy, Debug, PartialEq)]
struct LastPress {
    time_ms: f64,
    x: f64,
    y: f64,
    kind: PointerKind,
    button: PointerButton,
    count: u32,
}

/// Counts the presses of a series (one per canvas).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TapCounter {
    last: Option<LastPress>,
}

impl TapCounter {
    /// A press at `time_ms` (any millisecond clock that runs forwards) at (x, y): its tap count.
    /// `others_down` is true while another pointer is already down on the canvas.
    pub(crate) fn press(&mut self, options: &TapOptions, time_ms: f64, x: f64, y: f64, kind: PointerKind, button: PointerButton, others_down: bool) -> u32 {
        let distance = match kind {
            PointerKind::Mouse => options.mouse_distance,
            PointerKind::Pen | PointerKind::Touch => options.touch_distance,
        };
        let continues = !others_down
            && self.last.is_some_and(|last| {
                let since = time_ms - last.time_ms;
                last.kind == kind && last.button == button && since >= 0.0 && since <= options.interval_ms && (x - last.x).hypot(y - last.y) <= distance
            });
        let count = if continues { self.last.map_or(1, |last| last.count.saturating_add(1)) } else { 1 };
        self.last = Some(LastPress { time_ms, x, y, kind, button, count });
        count
    }

    /// The series is forgotten (the canvas stopped).
    pub(crate) fn clear(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{TapCounter, TapOptions};
    use crate::input::{PointerButton, PointerKind};

    fn mouse(counter: &mut TapCounter, time_ms: f64, x: f64, y: f64) -> u32 {
        counter.press(&TapOptions::default(), time_ms, x, y, PointerKind::Mouse, PointerButton::Primary, false)
    }

    #[test]
    fn the_defaults_are_a_usual_double_click() {
        assert_eq!(TapOptions::default(), TapOptions { interval_ms: 500.0, mouse_distance: 5.0, touch_distance: 24.0 });
    }

    #[test]
    fn quick_presses_at_one_place_count_up() {
        let mut counter = TapCounter::default();
        assert_eq!(mouse(&mut counter, 1000.0, 10.0, 10.0), 1);
        assert_eq!(mouse(&mut counter, 1200.0, 12.0, 13.0), 2);
        assert_eq!(mouse(&mut counter, 1650.0, 10.0, 10.0), 3, "each press is measured from the one before");
    }

    #[test]
    fn a_slow_press_starts_a_new_series() {
        let mut counter = TapCounter::default();
        mouse(&mut counter, 1000.0, 10.0, 10.0);
        assert_eq!(mouse(&mut counter, 1500.0, 10.0, 10.0), 2, "the interval itself still counts");
        assert_eq!(mouse(&mut counter, 2000.5, 10.0, 10.0), 1);
    }

    #[test]
    fn a_press_too_far_away_starts_a_new_series() {
        let mut counter = TapCounter::default();
        mouse(&mut counter, 0.0, 10.0, 10.0);
        assert_eq!(mouse(&mut counter, 100.0, 14.0, 13.0), 2, "5 pixels away");
        assert_eq!(mouse(&mut counter, 200.0, 20.0, 13.0), 1, "6 pixels away");
    }

    #[test]
    fn a_finger_may_land_farther_from_its_last_tap_than_a_mouse() {
        let mut counter = TapCounter::default();
        let options = TapOptions::default();
        counter.press(&options, 0.0, 100.0, 100.0, PointerKind::Touch, PointerButton::Primary, false);
        assert_eq!(counter.press(&options, 250.0, 120.0, 110.0, PointerKind::Touch, PointerButton::Primary, false), 2);
        counter.press(&options, 1000.0, 100.0, 100.0, PointerKind::Pen, PointerButton::Primary, false);
        assert_eq!(counter.press(&options, 1100.0, 110.0, 100.0, PointerKind::Pen, PointerButton::Primary, false), 2, "a pen counts as a finger");
    }

    #[test]
    fn another_button_or_kind_of_pointer_starts_a_new_series() {
        let mut counter = TapCounter::default();
        let options = TapOptions::default();
        mouse(&mut counter, 0.0, 10.0, 10.0);
        assert_eq!(counter.press(&options, 100.0, 10.0, 10.0, PointerKind::Mouse, PointerButton::Secondary, false), 1);
        assert_eq!(counter.press(&options, 200.0, 10.0, 10.0, PointerKind::Touch, PointerButton::Secondary, false), 1);
    }

    #[test]
    fn a_second_finger_of_a_pinch_is_no_double_tap() {
        let mut counter = TapCounter::default();
        let options = TapOptions::default();
        counter.press(&options, 0.0, 100.0, 100.0, PointerKind::Touch, PointerButton::Primary, false);
        assert_eq!(counter.press(&options, 40.0, 110.0, 100.0, PointerKind::Touch, PointerButton::Primary, true), 1);
    }

    #[test]
    fn a_clock_that_goes_back_and_clear_start_new_series() {
        let mut counter = TapCounter::default();
        mouse(&mut counter, 1000.0, 10.0, 10.0);
        assert_eq!(mouse(&mut counter, 900.0, 10.0, 10.0), 1);
        counter.clear();
        assert_eq!(mouse(&mut counter, 950.0, 10.0, 10.0), 1);
    }

    #[test]
    fn the_options_are_the_applications() {
        let mut counter = TapCounter::default();
        let options = TapOptions { interval_ms: 300.0, mouse_distance: 2.0, ..TapOptions::default() };
        counter.press(&options, 0.0, 0.0, 0.0, PointerKind::Mouse, PointerButton::Primary, false);
        assert_eq!(counter.press(&options, 301.0, 0.0, 0.0, PointerKind::Mouse, PointerButton::Primary, false), 1);
        assert_eq!(counter.press(&options, 400.0, 3.0, 0.0, PointerKind::Mouse, PointerButton::Primary, false), 1);
        assert_eq!(counter.press(&options, 500.0, 3.0, 2.0, PointerKind::Mouse, PointerButton::Primary, false), 2);
    }
}
