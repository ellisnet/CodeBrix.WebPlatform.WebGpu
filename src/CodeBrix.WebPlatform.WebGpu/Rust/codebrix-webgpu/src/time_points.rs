//! Named points in time on a canvas clock's timeline ("camera arrives" at 2.5 s, "title in" at
//! 4 s): defined once by the application on the clock, read by every layer from the frame, so
//! that no two layers repeat the same raw seconds.

use std::collections::BTreeMap;
use std::rc::Rc;

/// The named points in time of a clock, as a layer reads them from its frame
/// ([`CanvasFrame::points`](crate::CanvasFrame::points)). Define them on the clock
/// ([`CanvasClock::define_point`](crate::CanvasClock::define_point)).
///
/// A name that is not defined gives None from [`TimePoints::time`] and [`TimePoints::since`]
/// and false from [`TimePoints::reached`] and [`TimePoints::crossed`]; check
/// [`TimePoints::contains`] (or `time(..).is_some()`) where a misspelt name must not pass
/// silently.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimePoints {
    points: Rc<BTreeMap<String, f64>>,
}

impl TimePoints {
    /// Defines a point, or moves an existing one, at `seconds` on the clock's timeline.
    pub(crate) fn define(&mut self, name: &str, seconds: f64) {
        Rc::make_mut(&mut self.points).insert(name.to_string(), seconds);
    }

    /// Removes a point; false when there was none of that name.
    pub(crate) fn remove(&mut self, name: &str) -> bool {
        Rc::make_mut(&mut self.points).remove(name).is_some()
    }

    /// True when a point of that name is defined.
    pub fn contains(&self, name: &str) -> bool {
        self.points.contains_key(name)
    }

    /// The point's time on the clock's timeline, in seconds; None for a name that is not defined.
    pub fn time(&self, name: &str) -> Option<f64> {
        self.points.get(name).copied()
    }

    /// The names and times of every point, in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, f64)> {
        self.points.iter().map(|(name, time)| (name.as_str(), *time))
    }

    /// Seconds of clock time since the point at clock time `time` (negative before it); None for
    /// a name that is not defined.
    pub fn since(&self, name: &str, time: f64) -> Option<f64> {
        self.time(name).map(|point| time - point)
    }

    /// True when the clock time `time` is at or past the point.
    pub fn reached(&self, name: &str, time: f64) -> bool {
        self.time(name).is_some_and(|point| time >= point)
    }

    /// True when the point lies in the step from `previous` to `time` (previous < point <= time):
    /// the frame that reaches it, and only that frame. A paused clock takes no step, so nothing is
    /// crossed while it stands still; a seek forward over a point crosses it; a seek back crosses
    /// nothing. `previous` is None for the clock's first frame, which crosses every point at or
    /// before its time.
    pub fn crossed(&self, name: &str, previous: Option<f64>, time: f64) -> bool {
        self.time(name).is_some_and(|point| point <= time && previous.is_none_or(|previous| previous < point))
    }
}

#[cfg(test)]
mod tests {
    use super::TimePoints;

    fn points() -> TimePoints {
        let mut points = TimePoints::default();
        points.define("camera arrives", 2.5);
        points.define("title in", 4.0);
        points
    }

    #[test]
    fn a_defined_point_has_its_time_and_an_unknown_one_none() {
        let points = points();
        assert_eq!(points.time("title in"), Some(4.0));
        assert_eq!(points.time("title out"), None);
        assert!(points.contains("camera arrives") && !points.contains("Title in"));
    }

    #[test]
    fn defining_again_moves_the_point_and_remove_takes_it_out() {
        let mut points = points();
        points.define("title in", 5.0);
        assert_eq!(points.time("title in"), Some(5.0));
        assert!(points.remove("title in"));
        assert!(!points.remove("title in"));
        assert_eq!(points.iter().collect::<Vec<_>>(), vec![("camera arrives", 2.5)]);
    }

    #[test]
    fn a_copy_keeps_the_points_it_was_taken_with() {
        let mut points = points();
        let frame_copy = points.clone();
        points.define("title in", 9.0);
        assert_eq!(frame_copy.time("title in"), Some(4.0));
    }

    #[test]
    fn since_is_negative_before_the_point() {
        let points = points();
        assert_eq!(points.since("title in", 3.0), Some(-1.0));
        assert_eq!(points.since("title in", 4.5), Some(0.5));
        assert_eq!(points.since("nothing", 4.5), None);
    }

    #[test]
    fn reached_is_at_or_after_the_point() {
        let points = points();
        assert!(!points.reached("title in", 3.99));
        assert!(points.reached("title in", 4.0));
        assert!(!points.reached("nothing", 100.0));
    }

    #[test]
    fn crossed_is_true_in_the_one_step_that_reaches_the_point() {
        let points = points();
        assert!(!points.crossed("title in", Some(3.9), 3.99));
        assert!(points.crossed("title in", Some(3.99), 4.0));
        assert!(points.crossed("title in", Some(3.99), 4.02));
        assert!(!points.crossed("title in", Some(4.0), 4.02));
    }

    #[test]
    fn a_paused_step_crosses_nothing_and_a_seek_back_crosses_nothing() {
        let points = points();
        assert!(!points.crossed("title in", Some(4.0), 4.0));
        assert!(!points.crossed("title in", Some(4.5), 1.0));
    }

    #[test]
    fn a_seek_forward_crosses_every_point_it_jumps_over() {
        let points = points();
        assert!(points.crossed("camera arrives", Some(1.0), 10.0));
        assert!(points.crossed("title in", Some(1.0), 10.0));
    }

    #[test]
    fn the_first_frame_crosses_the_points_at_or_before_it() {
        let mut points = points();
        points.define("start", 0.0);
        assert!(points.crossed("start", None, 0.0));
        assert!(!points.crossed("title in", None, 0.0));
    }
}
