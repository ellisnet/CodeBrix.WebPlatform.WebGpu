//! The input the canvas host hands to a renderer: pointer, wheel and key events from Dioxus's own
//! event handlers, in the canvas's CSS pixels (0,0 at its top-left corner). Every pointer event
//! carries the browser's pointer id, so that a renderer can follow several fingers at once (a
//! pinch).

use dioxus::html::geometry::WheelDelta;

/// Pixels per line and per page when the browser reports the wheel in lines or pages.
const PIXELS_PER_LINE: f64 = 16.0;
const PIXELS_PER_PAGE: f64 = 400.0;

/// One input event on the canvas.
#[derive(Clone, Debug, PartialEq)]
pub enum CanvasInput {
    /// A pointer (mouse button, pen or finger) went down on the canvas. The canvas captures the
    /// pointer until it goes up, so a drag that leaves the canvas still arrives.
    PointerDown {
        /// Position from the canvas's left edge.
        x: f64,
        /// Position from the canvas's top edge.
        y: f64,
        /// The browser's pointer id.
        pointer_id: i32,
    },
    /// A pointer moved over the canvas (or anywhere, while captured).
    PointerMove {
        /// Position from the canvas's left edge.
        x: f64,
        /// Position from the canvas's top edge.
        y: f64,
        /// How far THIS pointer moved to the right since its last move of the same press (0 when
        /// no button is down). Each pointer is tracked on its own, so two fingers on a touch
        /// screen each report their own distances.
        dx: f64,
        /// How far this pointer moved down since its last move of the same press (0 when no
        /// button is down).
        dy: f64,
        /// Whether a button (or the finger or pen) is down: true while dragging.
        pressed: bool,
        /// The browser's pointer id (the same as its `PointerDown`'s): two fingers have two ids.
        pointer_id: i32,
    },
    /// A pointer went up, or the browser cancelled it.
    PointerUp {
        /// Position from the canvas's left edge.
        x: f64,
        /// Position from the canvas's top edge.
        y: f64,
        /// The browser's pointer id.
        pointer_id: i32,
    },
    /// The wheel turned over the canvas (the page does not scroll): positive `delta_y` is
    /// towards the visitor ("scroll down"), in CSS pixels.
    Wheel {
        /// Horizontal distance.
        delta_x: f64,
        /// Vertical distance.
        delta_y: f64,
    },
    /// A key went down while the canvas has the focus (it takes the focus when clicked).
    KeyDown {
        /// The key's value, as the browser names it ("ArrowLeft", "a", "+", " ").
        key: String,
    },
    /// A key went up while the canvas has the focus.
    KeyUp {
        /// The key's value, as the browser names it.
        key: String,
    },
}

/// A wheel movement in CSS pixels, whatever unit the browser reported it in.
pub(crate) fn wheel_in_pixels(delta: WheelDelta) -> (f64, f64) {
    match delta {
        WheelDelta::Pixels(v) => (v.x, v.y),
        WheelDelta::Lines(v) => (v.x * PIXELS_PER_LINE, v.y * PIXELS_PER_LINE),
        WheelDelta::Pages(v) => (v.x * PIXELS_PER_PAGE, v.y * PIXELS_PER_PAGE),
    }
}

/// True for the keys whose default action (scrolling the page) the canvas suppresses while it
/// has the focus: the arrow keys and the space bar belong to the renderer there.
pub(crate) fn is_canvas_key(key: &str) -> bool {
    matches!(key, "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | " ")
}

#[cfg(test)]
mod tests {
    use super::{is_canvas_key, wheel_in_pixels};
    use dioxus::html::geometry::WheelDelta;

    #[test]
    fn wheel_pixels_are_kept() {
        assert_eq!(wheel_in_pixels(WheelDelta::pixels(3.0, -120.0, 0.0)), (3.0, -120.0));
    }

    #[test]
    fn wheel_lines_and_pages_become_pixels() {
        assert_eq!(wheel_in_pixels(WheelDelta::lines(1.0, -3.0, 0.0)), (16.0, -48.0));
        assert_eq!(wheel_in_pixels(WheelDelta::pages(0.0, 1.0, 0.0)), (0.0, 400.0));
    }

    #[test]
    fn arrow_keys_and_space_belong_to_the_canvas() {
        for key in ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", " "] {
            assert!(is_canvas_key(key), "{key}");
        }
        for key in ["a", "Tab", "Enter", "+", "Escape"] {
            assert!(!is_canvas_key(key), "{key}");
        }
    }
}
