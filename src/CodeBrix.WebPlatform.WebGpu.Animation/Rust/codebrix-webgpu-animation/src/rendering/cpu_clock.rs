//! The internal CPU clock of [`RenderStats`](crate::rendering::RenderStats)' timings (Not in C#):
//! milliseconds from a fixed start, for MEASURING how long the CPU spends on a frame. Nothing the
//! crate draws reads it (THE RULE OF THIS CRATE: time comes from the frame).
//!
//! In a browser (wasm32-unknown-unknown) it is the page's `performance.now()`, reached through
//! js-sys's reflection (js-sys comes with web-sys, which wgpu re-exports and codebrix-webgpu
//! re-exports wgpu: no crate of this crate's own, no web-sys feature, no JavaScript). The
//! browser coarsens it (to 0.1 ms or more unless the page is cross-origin isolated). Where
//! `performance` is missing, it reads 0, so the timings read 0. Everywhere else (the host's unit
//! tests) it is `std::time::Instant`, which wasm32-unknown-unknown does not have.

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

/// The milliseconds from `start` (a reading of [`now_ms`]) to now; never below 0.
pub(crate) fn since_ms(start: f64) -> f64 {
    (now_ms() - start).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cpu_clock_moves_forward_in_milliseconds() {
        //Arrange
        let start = now_ms();
        //Act
        std::thread::sleep(std::time::Duration::from_millis(5));
        let later = now_ms();
        //Assert
        assert!(later >= start + 4.0, "{start} -> {later}");
        assert!(since_ms(start) >= 4.0);
        assert!(since_ms(later + 1000.0) == 0.0, "never below 0");
    }

    #[test]
    fn a_frames_cpu_time_is_its_three_steps_and_starts_at_zero() {
        //Arrange
        let stats = crate::rendering::RenderStats { record_ms: 1.25, build_ms: 2.0, encode_ms: 0.5, ..Default::default() };
        //Act
        let total = stats.cpu_ms();
        //Assert
        assert_eq!(total, 3.75);
        assert_eq!(crate::rendering::RenderStats::default().cpu_ms(), 0.0);
    }
}
