//! The test frontend of the real-materialize tests: it names the first-party crate
//! codebrix-webgpu and uses its public API (the context, the canvas host with two layers and one
//! clock with a named point in time, renderers that draw with wgpu directly, the input, the "not
//! available" message, the frame loop and the maths), so that
//! `cargo check --offline --target wasm32-unknown-unknown` proves a frontend can build with it
//! from the materialized vendor folder alone. It is compiled, never run.

use codebrix_webgpu::math::{Mat4, Quat, Vec3};
use codebrix_webgpu::{
    wgpu, CanvasClock, CanvasFrame, CanvasInput, CanvasLayers, CanvasRenderer, CanvasTarget, FrameLoop, GpuContext, GpuUnavailable, RendererHandle, SurfaceFailure,
    WebGpuCanvas, WebGpuUnavailable,
};
use dioxus::prelude::*;

/// A layer that keeps a model matrix turning with the clock and records a pass over the target.
struct Spinner {
    format: Option<wgpu::TextureFormat>,
    model: Mat4,
    shown: bool,
}

impl CanvasRenderer for Spinner {
    fn prepare(&mut self, _context: &GpuContext, target: &CanvasTarget) {
        self.format = Some(target.format);
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        self.model = Mat4::from_translation(Vec3::new(0.0, 0.0, -2.0)) * Mat4::from_quat(Quat::from_rotation_z(frame.info.time as f32));
        if frame.point_crossed("title in") {
            self.shown = true;
        }
        let red = if self.shown { self.model.to_cols_array()[0] as f64 } else { 0.0 };
        // Read what the frame says before a pass borrows its encoder.
        let point = (frame.since_point("title in"), frame.point_reached("title in"), frame.point_time("title in"));
        let _pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("test layer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: frame.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        let _ = (red, self.format, point);
    }

    fn input(&mut self, input: &CanvasInput) {
        if let CanvasInput::KeyDown { key } = input {
            if key == "r" {
                self.shown = false;
            }
        }
    }

    fn surface_failed(&mut self, failure: &SurfaceFailure) {
        let _ = failure.to_string();
    }
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_hook(run_and_stop_a_frame_loop);
    let gpu = use_resource(GpuContext::request);
    let state = gpu.read();
    match &*state {
        None => rsx! { p { "Asking the browser for a WebGPU adapter..." } },
        Some(Err(reason)) => rsx! { WebGpuUnavailable { reason: reason.clone(), class: "notice" } },
        Some(Ok(context)) => rsx! { Scene { context: context.clone() } },
    }
}

#[component]
fn Scene(context: GpuContext) -> Element {
    let clock = use_hook(|| {
        let clock = CanvasClock::new();
        clock.define_point("title in", 4.0);
        clock
    });
    let layers = use_hook(|| {
        let bottom = RendererHandle::new(Spinner { format: None, model: Mat4::IDENTITY, shown: false });
        let top = RendererHandle::new(Spinner { format: None, model: Mat4::IDENTITY, shown: false });
        CanvasLayers::new(vec![bottom, top])
    });
    let name = context.adapter_info().name;
    let pause = {
        let clock = clock.clone();
        move |_| {
            clock.toggle();
        }
    };
    rsx! {
        WebGpuCanvas { context, layers, clock: clock.clone(), class: "canvas", label: "A test canvas" }
        button { onclick: pause, "Pause" }
        p { "{name} - {GpuUnavailable::TITLE}" }
    }
}

/// The frame loop on its own (an application may run one without a canvas).
fn run_and_stop_a_frame_loop() {
    let frame_loop = FrameLoop::start(|_time| {});
    frame_loop.stop();
}
