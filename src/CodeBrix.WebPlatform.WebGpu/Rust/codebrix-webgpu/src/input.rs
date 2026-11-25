//! The input the canvas host hands to a renderer: pointer, wheel and key events from Dioxus's own
//! event handlers, in the canvas's CSS pixels (0,0 at its top-left corner). Every pointer event
//! carries the browser's pointer id, so that a renderer can follow several fingers at once (a
//! pinch), and says which buttons are down ([`PointerButton`], [`PointerButtons`]), what kind of
//! pointer it is ([`PointerKind`]) and which modifier keys are held ([`ModifierKeys`]), so that a
//! renderer can tell a right-button or a Shift drag from a plain one.

use dioxus::html::geometry::WheelDelta;
use dioxus::html::input_data::{MouseButton, MouseButtonSet};
use dioxus::html::Modifiers;

/// Pixels per line and per page when the browser reports the wheel in lines or pages.
const PIXELS_PER_LINE: f64 = 16.0;
const PIXELS_PER_PAGE: f64 = 400.0;

/// One input event on the canvas.
#[derive(Clone, Debug, PartialEq)]
pub enum CanvasInput {
    /// A pointer (mouse button, pen or finger) went down on the canvas. The canvas captures the
    /// pointer until it goes up, so a drag that leaves the canvas still arrives. A second mouse
    /// button pressed while one is already down reaches the canvas as a `PointerMove` whose
    /// `buttons` changed (the browser sends no second pointer-down for the same pointer).
    PointerDown {
        /// Position from the canvas's left edge.
        x: f64,
        /// Position from the canvas's top edge.
        y: f64,
        /// The browser's pointer id.
        pointer_id: i32,
        /// The button that went down (the primary one for a finger or a pen's tip). Not in C#.
        button: PointerButton,
        /// Every button down now, this one included. Not in C#.
        buttons: PointerButtons,
        /// Mouse, pen or touch. Not in C#.
        kind: PointerKind,
        /// The modifier keys held. Not in C#.
        modifiers: ModifierKeys,
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
        /// Whether a button (or the finger or pen) is down: true while dragging (the same as
        /// `!buttons.is_empty()`).
        pressed: bool,
        /// The browser's pointer id (the same as its `PointerDown`'s): two fingers have two ids.
        pointer_id: i32,
        /// Every button down now (which buttons drag). Not in C#.
        buttons: PointerButtons,
        /// Mouse, pen or touch. Not in C#.
        kind: PointerKind,
        /// The modifier keys held now. Not in C#.
        modifiers: ModifierKeys,
    },
    /// A pointer went up, or the browser cancelled it.
    PointerUp {
        /// Position from the canvas's left edge.
        x: f64,
        /// Position from the canvas's top edge.
        y: f64,
        /// The browser's pointer id.
        pointer_id: i32,
        /// The button that went up ([`PointerButton::Other`] when the browser cancelled the
        /// pointer). Not in C#.
        button: PointerButton,
        /// The buttons still down (empty when the last one went up). Not in C#.
        buttons: PointerButtons,
        /// Mouse, pen or touch. Not in C#.
        kind: PointerKind,
        /// The modifier keys held. Not in C#.
        modifiers: ModifierKeys,
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

/// One button of a pointer, as the browser numbers them (its `button` value). Not in C#.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerButton {
    /// The primary button: a mouse's left button (its right one when the visitor swapped them),
    /// a finger touching, a pen's tip.
    Primary,
    /// The secondary button: a mouse's right button, a pen's barrel button.
    Secondary,
    /// The middle button: a mouse's wheel pressed.
    Middle,
    /// The fourth button, usually "back".
    Back,
    /// The fifth button, usually "forward".
    Forward,
    /// No button, or one the browser numbers beyond the fifth (a cancelled pointer, a pen's
    /// eraser).
    Other,
}

/// The set of buttons down, with the browser's own bits (its `buttons` value): primary 1,
/// secondary 2, middle 4, back 8, forward 16. Not in C#.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PointerButtons(u8);

impl PointerButtons {
    /// No button down.
    pub const NONE: PointerButtons = PointerButtons(0);
    /// The primary button (left mouse button, finger, pen tip).
    pub const PRIMARY: PointerButtons = PointerButtons(1);
    /// The secondary button (right mouse button, pen barrel).
    pub const SECONDARY: PointerButtons = PointerButtons(2);
    /// The middle button (the wheel pressed).
    pub const MIDDLE: PointerButtons = PointerButtons(4);
    /// The fourth ("back") button.
    pub const BACK: PointerButtons = PointerButtons(8);
    /// The fifth ("forward") button.
    pub const FORWARD: PointerButtons = PointerButtons(16);

    /// The set from the browser's bits (bits beyond the five buttons are dropped).
    pub const fn from_bits(bits: u8) -> PointerButtons {
        PointerButtons(bits & 0b1_1111)
    }

    /// The browser's bits of the set.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// True when no button is down.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// True when `button` is down ([`PointerButton::Other`] is never in the set).
    pub const fn contains(self, button: PointerButton) -> bool {
        let bit = match button {
            PointerButton::Primary => 1,
            PointerButton::Secondary => 2,
            PointerButton::Middle => 4,
            PointerButton::Back => 8,
            PointerButton::Forward => 16,
            PointerButton::Other => 0,
        };
        self.0 & bit != 0
    }

    /// True when the primary button is down.
    pub const fn primary(self) -> bool {
        self.0 & 1 != 0
    }

    /// True when the secondary button is down.
    pub const fn secondary(self) -> bool {
        self.0 & 2 != 0
    }

    /// True when the middle button is down.
    pub const fn middle(self) -> bool {
        self.0 & 4 != 0
    }

    /// The union of two sets.
    pub const fn with(self, other: PointerButtons) -> PointerButtons {
        PointerButtons(self.0 | other.0)
    }
}

/// What kind of pointer an event comes from (the browser's `pointerType`). Not in C#.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PointerKind {
    /// A mouse or a trackpad (also any pointer type the browser does not name).
    #[default]
    Mouse,
    /// A pen or stylus.
    Pen,
    /// A finger on a touch screen.
    Touch,
}

/// The modifier keys held during an event. Not in C#.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ModifierKeys {
    /// Shift.
    pub shift: bool,
    /// Control (Ctrl).
    pub ctrl: bool,
    /// Alt (Option on a Mac).
    pub alt: bool,
    /// Meta: the Windows key, or Command on a Mac.
    pub meta: bool,
}

impl ModifierKeys {
    /// No modifier key held.
    pub const NONE: ModifierKeys = ModifierKeys { shift: false, ctrl: false, alt: false, meta: false };

    /// True when none of the four keys is held.
    pub const fn is_empty(self) -> bool {
        !(self.shift || self.ctrl || self.alt || self.meta)
    }
}

/// The button of a pointer event, from Dioxus's.
pub(crate) fn pointer_button(button: Option<MouseButton>) -> PointerButton {
    match button {
        Some(MouseButton::Primary) => PointerButton::Primary,
        Some(MouseButton::Secondary) => PointerButton::Secondary,
        Some(MouseButton::Auxiliary) => PointerButton::Middle,
        Some(MouseButton::Fourth) => PointerButton::Back,
        Some(MouseButton::Fifth) => PointerButton::Forward,
        Some(MouseButton::Unknown) | None => PointerButton::Other,
    }
}

/// The buttons down, from Dioxus's set.
pub(crate) fn pointer_buttons(set: MouseButtonSet) -> PointerButtons {
    let mut buttons = PointerButtons::NONE;
    for (button, bits) in [
        (MouseButton::Primary, PointerButtons::PRIMARY),
        (MouseButton::Secondary, PointerButtons::SECONDARY),
        (MouseButton::Auxiliary, PointerButtons::MIDDLE),
        (MouseButton::Fourth, PointerButtons::BACK),
        (MouseButton::Fifth, PointerButtons::FORWARD),
    ] {
        if set.contains(button) {
            buttons = buttons.with(bits);
        }
    }
    buttons
}

/// The kind of pointer, from the browser's `pointerType` ("mouse", "pen", "touch").
pub(crate) fn pointer_kind(pointer_type: &str) -> PointerKind {
    match pointer_type {
        "pen" => PointerKind::Pen,
        "touch" => PointerKind::Touch,
        _ => PointerKind::Mouse,
    }
}

/// The four modifier keys, from Dioxus's.
pub(crate) fn modifier_keys(modifiers: Modifiers) -> ModifierKeys {
    ModifierKeys { shift: modifiers.contains(Modifiers::SHIFT), ctrl: modifiers.contains(Modifiers::CONTROL), alt: modifiers.contains(Modifiers::ALT), meta: modifiers.contains(Modifiers::META) }
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
    use super::{is_canvas_key, modifier_keys, pointer_button, pointer_buttons, pointer_kind, wheel_in_pixels, ModifierKeys, PointerButton, PointerButtons, PointerKind};
    use dioxus::html::geometry::WheelDelta;
    use dioxus::html::input_data::{decode_mouse_button_set, MouseButton};
    use dioxus::html::Modifiers;

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

    #[test]
    fn the_buttons_keep_the_browsers_bits() {
        //Arrange: the browser's `buttons` 1 + 2 + 4 + 16, as Dioxus decodes them
        let set = decode_mouse_button_set(0b1_0111);
        //Act
        let buttons = pointer_buttons(set);
        //Assert
        assert_eq!(buttons.bits(), 0b1_0111);
        assert!(buttons.primary() && buttons.secondary() && buttons.middle());
        assert!(buttons.contains(PointerButton::Forward) && !buttons.contains(PointerButton::Back));
        assert!(!buttons.contains(PointerButton::Other));
        assert!(pointer_buttons(decode_mouse_button_set(0)).is_empty());
        assert_eq!(PointerButtons::from_bits(0xFF).bits(), 0b1_1111, "bits beyond the five buttons are dropped");
    }

    #[test]
    fn the_button_that_changed_is_named_as_the_browser_numbers_it() {
        assert_eq!(pointer_button(Some(MouseButton::from_web_code(0))), PointerButton::Primary);
        assert_eq!(pointer_button(Some(MouseButton::from_web_code(1))), PointerButton::Middle);
        assert_eq!(pointer_button(Some(MouseButton::from_web_code(2))), PointerButton::Secondary);
        assert_eq!(pointer_button(Some(MouseButton::from_web_code(3))), PointerButton::Back);
        assert_eq!(pointer_button(Some(MouseButton::from_web_code(4))), PointerButton::Forward);
        assert_eq!(pointer_button(Some(MouseButton::from_web_code(-1))), PointerButton::Other, "a cancelled pointer");
        assert_eq!(pointer_button(None), PointerButton::Other);
    }

    #[test]
    fn the_pointer_kind_comes_from_the_pointer_type() {
        assert_eq!(pointer_kind("mouse"), PointerKind::Mouse);
        assert_eq!(pointer_kind("pen"), PointerKind::Pen);
        assert_eq!(pointer_kind("touch"), PointerKind::Touch);
        assert_eq!(pointer_kind(""), PointerKind::Mouse, "a type the browser does not name");
    }

    #[test]
    fn the_four_modifier_keys_are_read_and_the_others_ignored() {
        //Act
        let shift_meta = modifier_keys(Modifiers::SHIFT | Modifiers::META | Modifiers::CAPS_LOCK);
        let ctrl_alt = modifier_keys(Modifiers::CONTROL | Modifiers::ALT);
        //Assert
        assert_eq!(shift_meta, ModifierKeys { shift: true, ctrl: false, alt: false, meta: true });
        assert_eq!(ctrl_alt, ModifierKeys { shift: false, ctrl: true, alt: true, meta: false });
        assert!(modifier_keys(Modifiers::empty()).is_empty() && ModifierKeys::NONE.is_empty());
    }
}
