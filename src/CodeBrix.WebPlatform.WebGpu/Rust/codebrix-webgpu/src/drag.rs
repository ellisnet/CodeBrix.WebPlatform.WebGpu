//! How far each pressed pointer moved between two of its move events (the `dx` and `dy` of
//! [`CanvasInput::PointerMove`](crate::CanvasInput::PointerMove)). Every pointer id is tracked on
//! its own, so that two fingers on a touch screen do not disturb each other's distances.

/// The last position of every pressed pointer.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DragTracker {
    pressed: Vec<(i32, f64, f64)>,
}

impl DragTracker {
    /// Pointer `id` went down at (x, y).
    pub(crate) fn press(&mut self, id: i32, x: f64, y: f64) {
        self.pressed.retain(|(known, _, _)| *known != id);
        self.pressed.push((id, x, y));
    }

    /// Pointer `id` moved to (x, y): the distance since its last position while pressed, (0, 0)
    /// when it is not pressed or this is the first move of a press.
    pub(crate) fn moved(&mut self, id: i32, x: f64, y: f64, pressed: bool) -> (f64, f64) {
        let index = self.pressed.iter().position(|(known, _, _)| *known == id);
        let delta = match (pressed, index) {
            (true, Some(index)) => {
                let (_, last_x, last_y) = self.pressed[index];
                (x - last_x, y - last_y)
            }
            _ => (0.0, 0.0),
        };
        match (pressed, index) {
            (true, Some(index)) => self.pressed[index] = (id, x, y),
            (true, None) => self.pressed.push((id, x, y)),
            (false, Some(index)) => {
                self.pressed.remove(index);
            }
            (false, None) => {}
        }
        delta
    }

    /// Pointer `id` went up or was cancelled.
    pub(crate) fn release(&mut self, id: i32) {
        self.pressed.retain(|(known, _, _)| *known != id);
    }

    /// Every pointer is forgotten (the canvas stopped).
    pub(crate) fn clear(&mut self) {
        self.pressed.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::DragTracker;

    #[test]
    fn a_drag_reports_the_distance_between_moves() {
        let mut drag = DragTracker::default();
        drag.press(1, 10.0, 20.0);
        assert_eq!(drag.moved(1, 15.0, 18.0, true), (5.0, -2.0));
        assert_eq!(drag.moved(1, 25.0, 18.0, true), (10.0, 0.0));
    }

    #[test]
    fn a_move_without_a_press_reports_nothing() {
        let mut drag = DragTracker::default();
        assert_eq!(drag.moved(1, 15.0, 18.0, false), (0.0, 0.0));
        assert_eq!(drag.moved(1, 30.0, 40.0, true), (0.0, 0.0));
        assert_eq!(drag.moved(1, 31.0, 42.0, true), (1.0, 2.0));
    }

    #[test]
    fn release_ends_the_drag() {
        let mut drag = DragTracker::default();
        drag.press(1, 0.0, 0.0);
        drag.release(1);
        assert_eq!(drag.moved(1, 5.0, 5.0, true), (0.0, 0.0));
    }

    #[test]
    fn two_pointers_are_tracked_apart() {
        let mut drag = DragTracker::default();
        drag.press(1, 100.0, 100.0);
        drag.press(2, 200.0, 100.0);
        assert_eq!(drag.moved(1, 90.0, 100.0, true), (-10.0, 0.0));
        assert_eq!(drag.moved(2, 215.0, 100.0, true), (15.0, 0.0));
        drag.release(1);
        assert_eq!(drag.moved(2, 220.0, 101.0, true), (5.0, 1.0));
        drag.clear();
        assert_eq!(drag.moved(2, 230.0, 101.0, true), (0.0, 0.0));
    }
}
