//! The FRAME TIMINGS read-out every demo shows (the graphics crate's FrameTimer): the CPU time of
//! the frame, the GPU time of each marked pass ("scene", "tone map") or "not available" where the
//! browser has no `timestamp-query`, both also smoothed over the last 30 frames, and the model
//! renderer's counts of the frame. The layer refreshes it twice a second (every frame changes too
//! fast to read); the GPU numbers arrive a few frames late.

use codebrix_webgpu_graphics::FrameTimings;
use dioxus::prelude::*;

/// What the read-out shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimingsView {
    /// The timer's own one-line summary (`FrameTimings`' Display).
    pub line: String,
    /// The frame the numbers are from.
    pub frame: u64,
    /// CPU milliseconds of that frame, and smoothed.
    pub cpu: String,
    /// GPU milliseconds in total, and smoothed, or why there are none.
    pub gpu: String,
    /// Per marked pass: name, GPU milliseconds (smoothed).
    pub passes: Vec<(String, String)>,
    /// The counts the layer set.
    pub counts: Vec<(String, u64)>,
}

impl TimingsView {
    /// The read-out of `timings`.
    pub fn from_timings(timings: &FrameTimings) -> TimingsView {
        let gpu = if !timings.gpu_available {
            "not available (this browser offers no timestamp-query)".to_string()
        } else {
            match (timings.gpu_total_ms, timings.gpu_total_ms_smoothed) {
                (Some(total), Some(smoothed)) => format!("{total:.2} ms (smoothed {smoothed:.2} ms) from frame {}", timings.gpu_frame.unwrap_or(0)),
                _ => "waiting for the first results".to_string(),
            }
        };
        TimingsView {
            line: timings.to_string(),
            frame: timings.frame,
            cpu: format!("{:.2} ms (smoothed {:.2} ms)", timings.cpu_ms, timings.cpu_ms_smoothed),
            gpu,
            passes: timings.passes.iter().map(|p| (p.name.clone(), format!("{:.2} ms (smoothed {:.2} ms)", p.gpu_ms, p.gpu_ms_smoothed))).collect(),
            counts: timings.counts.clone(),
        }
    }
}

/// The timings panel of a demo (ids `<slug>-timing-...`).
#[component]
pub fn Timings(slug: &'static str, view: Signal<TimingsView>) -> Element {
    let shown = view();
    rsx! {
        div { id: "{slug}-timings", class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
            p { class: "font-semibold text-white", "Frame timings (the graphics crate's FrameTimer)" }
            p { class: "text-slate-300", "CPU: " span { id: "{slug}-timing-cpu", class: "font-mono text-white", "{shown.cpu}" } }
            p { class: "text-slate-300", "GPU, the marked passes together: " span { id: "{slug}-timing-gpu", class: "font-mono text-white", "{shown.gpu}" } }
            for (name, value) in shown.passes.iter() {
                p { key: "{name}", class: "text-slate-300", "GPU, pass \"{name}\": " span { class: "font-mono text-white", "{value}" } }
            }
            p { class: "text-slate-300",
                "Counts: "
                span { id: "{slug}-timing-counts", class: "font-mono text-white",
                    {shown.counts.iter().map(|(name, value)| format!("{name} {value}")).collect::<Vec<_>>().join(", ")}
                }
            }
            p { class: "text-slate-400", "The timer's line: " span { id: "{slug}-timing-line", class: "font-mono", "{shown.line}" } }
            p { class: "text-slate-400", "Refreshed twice a second. GPU times arrive a few frames late; the shadow passes and the particle steps are not inside the marked passes." }
        }
    }
}
