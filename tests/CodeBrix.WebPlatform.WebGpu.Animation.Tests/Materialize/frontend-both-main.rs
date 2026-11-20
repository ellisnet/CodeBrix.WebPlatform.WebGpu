//! The second test frontend of the Animation package's real-materialize tests: it names BOTH
//! codebrix-webgpu-graphics and codebrix-webgpu-animation, materialized with both packages
//! registered, so that `cargo check --offline --target wasm32-unknown-unknown` proves an
//! application can pull in either package or both. Only the APPLICATION puts the two renderers
//! together: a Graphics layer (lit shapes under a camera) under an Animation layer (a session of
//! shapes), on one canvas with one clock. It is compiled, never run.

use std::cell::RefCell;
use std::rc::Rc;

use codebrix_webgpu::{CanvasFrame, CanvasLayers, CanvasRenderer, CanvasTarget, GpuContext, RendererHandle, WebGpuCanvas, WebGpuUnavailable};
use codebrix_webgpu_animation::rendering::AnimationLayer;
use codebrix_webgpu_animation::shapes::{CircleShape, ShapeOptions};
use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions};
use codebrix_webgpu_graphics::{Color4, GraphicsContext};
use dioxus::prelude::*;

/// The 3D side: a layer of the Graphics crate that clears nothing and draws through the frame
/// it is handed.
struct Scene3d {
    graphics: GraphicsContext,
}

impl CanvasRenderer for Scene3d {
    fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
        let _ = Color4::CORNFLOWER_BLUE;
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let graphics = self.graphics.clone();
        let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
        let _ = graphics_frame.begin_render_pass(None, None);
    }
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let gpu = use_resource(GpuContext::request);
    let state = gpu.read();
    match &*state {
        None => rsx! { p { "Asking the browser for a WebGPU adapter..." } },
        Some(Err(reason)) => rsx! { WebGpuUnavailable { reason: reason.clone(), class: "notice" } },
        Some(Ok(context)) => rsx! { Both { context: context.clone() } },
    }
}

#[component]
fn Both(context: GpuContext) -> Element {
    let graphics = use_hook(|| GraphicsContext::wrap_gpu_context(&context));
    let layers = use_hook(|| {
        let mut session = DrawingSession::new(DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, ..Default::default() });
        if let Ok(dot) = CircleShape::new(640.0, 360.0, 40.0, ShapeOptions { is_filled: true, ..Default::default() }) {
            session.add_layer("Dots", Color::from_rgb(255, 176, 72)).add_shape(dot);
        }
        let session = Rc::new(RefCell::new(session));
        let overlay = AnimationLayer::new(move |frame, scene| {
            let mut s = session.borrow_mut();
            s.set_time(frame.info.time);
            s.render(scene, frame.info.width, frame.info.height);
        });
        CanvasLayers::new(vec![RendererHandle::new(Scene3d { graphics: graphics.clone() }), RendererHandle::new(overlay)])
    });
    rsx! {
        WebGpuCanvas { context: context.clone(), layers, class: "canvas" }
    }
}
