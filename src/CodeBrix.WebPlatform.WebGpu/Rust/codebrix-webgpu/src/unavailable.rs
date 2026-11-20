//! The "WebGPU is not available in this browser" message, for a page whose request for a
//! [`GpuContext`](crate::GpuContext) failed.

use dioxus::prelude::*;

use crate::context::GpuUnavailable;

/// Shows that WebGPU is not available ([`GpuUnavailable::TITLE`] as a heading) and one sentence
/// of what to do ([`GpuUnavailable::advice`]).
///
/// The component carries no styling of its own: give the box, the heading and the sentence the
/// application's classes with `class`, `heading_class` and `text_class`. The box has the id
/// `webgpu-unavailable` and the ARIA role `alert`.
#[component]
pub fn WebGpuUnavailable(
    /// Why WebGPU is not available (the error of `GpuContext::request`).
    reason: GpuUnavailable,
    /// The classes of the box.
    #[props(default)]
    class: String,
    /// The classes of the heading.
    #[props(default)]
    heading_class: String,
    /// The classes of the sentence.
    #[props(default)]
    text_class: String,
) -> Element {
    rsx! {
        div {
            id: "webgpu-unavailable",
            role: "alert",
            class: if class.is_empty() { None } else { Some(class) },
            h2 { class: if heading_class.is_empty() { None } else { Some(heading_class) }, "{GpuUnavailable::TITLE}" }
            p { class: if text_class.is_empty() { None } else { Some(text_class) }, "{reason.advice()}" }
        }
    }
}
