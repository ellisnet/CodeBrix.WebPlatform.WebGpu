//! The frame loop: the browser's animation frames (`requestAnimationFrame`, through web-sys),
//! with a wasm-bindgen `Closure` that schedules itself again after every frame. The browser
//! calls it once per display refresh while the page is visible and pauses it while it is hidden.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// A running frame loop. It calls its function once per animation frame, with the browser's
/// frame time in milliseconds, until it is stopped or dropped.
pub struct FrameLoop {
    state: Rc<LoopState>,
}

struct LoopState {
    callback: RefCell<Option<Closure<dyn FnMut(f64)>>>,
    request_id: Cell<Option<i32>>,
    running: Cell<bool>,
}

impl LoopState {
    fn request_next(&self) {
        let Some(window) = web_sys::window() else {
            return;
        };
        if let Some(callback) = self.callback.borrow().as_ref() {
            self.request_id.set(window.request_animation_frame(callback.as_ref().unchecked_ref()).ok());
        }
    }
}

impl FrameLoop {
    /// Starts a loop that calls `on_frame` once per animation frame. Never drop the loop from
    /// inside `on_frame`; stop it there instead ([`FrameLoop::stop`] is safe at any time).
    pub fn start(mut on_frame: impl FnMut(f64) + 'static) -> FrameLoop {
        let state = Rc::new(LoopState {
            callback: RefCell::new(None),
            request_id: Cell::new(None),
            running: Cell::new(true),
        });
        // The closure holds the loop only weakly, so dropping the FrameLoop frees everything.
        let weak = Rc::downgrade(&state);
        let callback = Closure::<dyn FnMut(f64)>::new(move |time: f64| {
            let Some(state) = weak.upgrade() else {
                return;
            };
            state.request_id.set(None);
            if !state.running.get() {
                return;
            }
            on_frame(time);
            if state.running.get() {
                state.request_next();
            }
        });
        *state.callback.borrow_mut() = Some(callback);
        state.request_next();
        FrameLoop { state }
    }

    /// Stops the loop: no more frames are delivered.
    pub fn stop(&self) {
        self.state.running.set(false);
        if let (Some(id), Some(window)) = (self.state.request_id.take(), web_sys::window()) {
            let _ = window.cancel_animation_frame(id);
        }
    }
}

impl Drop for FrameLoop {
    fn drop(&mut self) {
        self.stop();
    }
}
