//! # codebrix-webgpu
//!
//! What every CodeBrix.WebPlatform frontend that draws with WebGPU needs, and nothing particular
//! to one application: the WebGPU context, ONE canvas host component with its frame loop and
//! input, the "WebGPU is not available" result and message, and the maths types.
//!
//! - [`GpuContext`] owns (or shares) the `wgpu` instance, adapter, device and queue.
//!   [`GpuContext::request`] asks the browser for them once; its error, [`GpuUnavailable`], is
//!   the "WebGPU is not available in this browser" result, which [`WebGpuUnavailable`] shows.
//!   It installs the device's ONE uncaptured-error handler (once per device, however many
//!   contexts wrap it) and hands the errors to any number of readers
//!   ([`GpuContext::subscribe_errors`], [`GpuErrorSubscription`]: every reader sees every error).
//! - [`WebGpuCanvas`] is the canvas host component: it makes its canvas a `wgpu` surface, keeps
//!   the surface at the canvas's size in device pixels, and once per animation frame
//!   ([`FrameLoop`]) clears it and lets its LAYERS draw over each other ([`CanvasLayers`], an
//!   ordered list of [`CanvasRenderer`]s the application puts together - for example a 3D scene
//!   and a 2D animation over it, from crates that know nothing of each other), with ONE command
//!   encoder and the time of ONE clock ([`CanvasClock`]: pause, resume, speed, seek, and named
//!   points in time, [`TimePoints`]). It hands pointer, wheel and key input to the layers
//!   ([`CanvasInput`]). The surface's format, alpha mode and colour encoding are decided by the
//!   host alone ([`CanvasTarget`], which also states the blending convention).
//! - [`math`] is the `glam` crate under its own names (`Vec3`, `Mat4`, `Quat`, ...).
//!
//! LOW-LEVEL `wgpu` STAYS AVAILABLE: the context hands out the device and queue, every frame
//! hands the renderer the surface and its texture, and the crate re-exports [`wgpu`] and
//! [`glam`] themselves, so a frontend uses exactly the versions this crate was built with.
//!
//! WebGPU only: there is no WebGL fallback. There is no JavaScript in this crate: the canvas
//! element comes from Dioxus's `onmounted`, the animation frames from `requestAnimationFrame`
//! through web-sys with a wasm-bindgen `Closure`, and the input from Dioxus's own event
//! handlers. The crate never talks to the application's backend.
//!
//! ```ignore
//! use codebrix_webgpu::{wgpu, CanvasClock, CanvasFrame, CanvasLayers, CanvasRenderer, GpuContext, RendererHandle, WebGpuCanvas, WebGpuUnavailable};
//! use dioxus::prelude::*;
//!
//! struct Shape;
//!
//! impl CanvasRenderer for Shape {
//!     fn render(&mut self, frame: &mut CanvasFrame<'_>) {
//!         let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//!             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//!                 view: frame.view,
//!                 depth_slice: None,
//!                 resolve_target: None,
//!                 // Layers load: the host has cleared the target.
//!                 ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
//!             })],
//!             ..Default::default()
//!         });
//!         // ... set the pipeline (made in `prepare` for frame.target.format), draw ...
//!     }
//! }
//!
//! #[component]
//! fn Page() -> Element {
//!     let gpu = use_resource(GpuContext::request);
//!     let layers = use_hook(|| CanvasLayers::single(RendererHandle::new(Shape)));
//!     let clock = use_hook(CanvasClock::new);
//!     let state = gpu.read();
//!     match &*state {
//!         None => rsx! { p { "Asking the browser for a WebGPU adapter..." } },
//!         Some(Err(reason)) => rsx! { WebGpuUnavailable { reason: reason.clone() } },
//!         Some(Ok(context)) => rsx! { WebGpuCanvas { context: context.clone(), layers, clock, class: "w-full aspect-video" } },
//!     }
//! }
//! ```

mod canvas;
mod clock;
mod context;
mod drag;
mod errors;
mod frame_loop;
mod input;
mod layers;
pub mod math;
mod page_compose;
mod renderer;
mod sizing;
mod time_points;
mod unavailable;

pub use canvas::WebGpuCanvas;
pub use clock::CanvasClock;
pub use context::{GpuContext, GpuUnavailable};
pub use errors::{GpuError, GpuErrorKind, GpuErrorSubscription};
pub use frame_loop::FrameLoop;
pub use input::CanvasInput;
pub use layers::CanvasLayers;
pub use renderer::{CanvasFrame, CanvasRenderer, CanvasTarget, FrameInfo, RendererHandle, SurfaceFailure};
pub use time_points::TimePoints;
pub use unavailable::WebGpuUnavailable;

/// The `glam` crate this crate was built with (vectors, matrices, quaternions); the same types
/// are in [`math`].
pub use glam;
/// The `wgpu` crate this crate was built with: the whole low-level WebGPU API.
pub use wgpu;
