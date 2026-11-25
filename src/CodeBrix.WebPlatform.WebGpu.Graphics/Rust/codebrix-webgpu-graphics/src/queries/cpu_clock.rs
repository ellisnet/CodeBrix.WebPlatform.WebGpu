//! The internal CPU clock of the [`FrameTimer`](crate::FrameTimer) (Not in C#): milliseconds from
//! a fixed start, for MEASURING how long the CPU spends on a frame. Nothing the crate draws reads
//! it (THE LAYERING RULE: drawing takes its time from the frame); the frame timer is the one
//! measurement-only reading, and only when an application creates one.
//!
//! In a browser (wasm32-unknown-unknown) it is the page's `performance.now()`, reached through
//! js-sys's reflection (js-sys comes with web-sys, which wgpu re-exports, and codebrix-webgpu
//! re-exports wgpu: no crate of this crate's own, no web-sys feature, no JavaScript) - the
//! Animation crate's clock, the same way. The browser coarsens it (to 0.1 ms or more unless the
//! page is cross-origin isolated). Where `performance` is missing it reads 0, so the CPU
//! timings read 0. Everywhere else (the host's unit tests) it is `std::time::Instant`, which
//! wasm32-unknown-unknown does not have.

/// Milliseconds since a fixed start (the page's time origin in a browser, the first call on the
/// host). Never goes back.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) fn now_ms() -> f64 {
    use codebrix_webgpu::wgpu::web_sys::js_sys::{global, Function, Reflect};
    use codebrix_webgpu::wgpu::web_sys::wasm_bindgen::{JsCast, JsValue};

    thread_local! {
        /// The page's `performance` object and its `now` function, looked up once.
        static NOW: Option<(JsValue, Function)> = {
            let performance = Reflect::get(&global(), &JsValue::from_str("performance")).ok().filter(|p| p.is_object());
            performance.and_then(|p| {
                let now = Reflect::get(&p, &JsValue::from_str("now")).ok()?.dyn_into::<Function>().ok()?;
                Some((p, now))
            })
        };
    }
    NOW.with(|now| match now {
        Some((performance, now)) => now.call0(performance).ok().and_then(|v| v.as_f64()).unwrap_or(0.0),
        None => 0.0,
    })
}

/// Milliseconds since a fixed start (the page's time origin in a browser, the first call on the
/// host). Never goes back.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub(crate) fn now_ms() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;

    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::now_ms;

    #[test]
    fn the_cpu_clock_moves_forward_in_milliseconds() {
        let start = now_ms();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let later = now_ms();
        assert!(later >= start + 4.0, "{start} -> {later}");
    }
}
