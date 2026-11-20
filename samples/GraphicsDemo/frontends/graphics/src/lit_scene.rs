//! The "Lit shapes" scene: the lit-shapes layer (lit_shapes.rs) on a canvas of the codebrix-webgpu
//! host, with its own clock, a projection switch, a button that starts the auto-orbit again, the
//! camera's values, and a plain-HTML legend of the controls.

use codebrix_webgpu::{wgpu, CanvasClock, CanvasLayers, GpuContext, RendererHandle, WebGpuCanvas};
use codebrix_webgpu_graphics::GraphicsContext;
use dioxus::prelude::*;

use crate::lit_shapes::{LitShapesRenderer, LitView, RING_CUBES};

/// The canvas with the lit shapes, its clock, the switches, the read-outs and the legend.
#[component]
pub fn LitScene(context: GpuContext) -> Element {
    let graphics = use_hook(|| GraphicsContext::wrap_gpu_context(&context));
    let view = use_signal(LitView::default);
    let errors = use_signal(Vec::<String>::new);
    let mut orthographic = use_signal(|| false);
    let mut resume_requests = use_signal(|| 0u64);
    let mut paused = use_signal(|| false);
    let clock = use_hook(CanvasClock::new);
    let layers = use_hook(|| CanvasLayers::single(RendererHandle::new(LitShapesRenderer::new(graphics.clone(), view, errors, orthographic, resume_requests))));
    let toggle_clock = {
        let clock = clock.clone();
        move |_| paused.set(clock.toggle())
    };
    let button = "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700";
    let lit = view();
    let state = if paused() { "paused" } else { "running" };
    let orbit = if lit.auto_orbit { "auto-orbiting" } else { "user" };
    let projection = if lit.orthographic { "orthographic" } else { "perspective" };
    rsx! {
        div { class: "relative",
            WebGpuCanvas {
                context: context.clone(),
                layers,
                clock: clock.clone(),
                clear_color: wgpu::Color { r: 0.05, g: 0.07, b: 0.12, a: 1.0 },
                id: "lit-canvas",
                label: "Lit shapes on a ground plane. Drag to orbit, use the wheel to zoom; on a touch screen, one finger orbits and two fingers pinch to zoom.",
                class: "aspect-video w-full touch-none select-none rounded-lg ring-1 ring-white/10 focus-visible:outline-2 focus-visible:outline-sky-400",
            }
            div { id: "lit-legend", class: "pointer-events-none absolute left-3 top-3 rounded-md bg-slate-950/70 px-3 py-2 text-xs text-slate-200 ring-1 ring-white/10",
                p { class: "font-semibold text-white", "Controls" }
                p { "Drag: orbit (the scene follows the pointer)" }
                p { "Wheel: zoom" }
                p { "Touch: one finger orbits, two fingers pinch to zoom" }
                p { "It turns by itself until you drag." }
            }
        }
        div { class: "flex flex-wrap gap-3",
            button { id: "lit-clock-toggle", class: "{button}", onclick: toggle_clock, if paused() { "Resume the clock" } else { "Pause the clock" } }
            button { id: "lit-projection-toggle", class: "{button}", onclick: move |_| orthographic.set(!orthographic()), if orthographic() { "Use the perspective camera" } else { "Use the orthographic camera" } }
            button { id: "lit-auto-orbit", class: "{button}", onclick: move |_| resume_requests.set(resume_requests() + 1), "Orbit by itself again" }
        }
        div { class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
            p { class: "text-slate-300",
                "Frame " span { id: "lit-frame", class: "font-mono text-white", "{lit.frame}" }
                ", clock " span { id: "lit-clock-state", "{state}" }
                ", camera " span { id: "lit-projection", "{projection}" } ", "
                span { id: "lit-orbit-mode", "{orbit}" }
            }
            p { class: "text-slate-300",
                "Yaw " span { id: "lit-yaw", class: "font-mono text-white", "{lit.yaw:.3}" }
                " rad, pitch " span { id: "lit-pitch", class: "font-mono text-white", "{lit.pitch:.3}" }
                " rad (0.05 to 1.35), distance " span { id: "lit-distance", class: "font-mono text-white", "{lit.distance:.3}" } " (3 to 30); pointers pressed "
                span { id: "lit-pointers", class: "font-mono text-white", "{lit.pointers}" }
            }
            p { class: "text-slate-300",
                "Last frame: " span { id: "lit-draw-calls", class: "font-mono text-white", "{lit.draw_calls}" } " draw call(s), "
                span { id: "lit-instances", class: "font-mono text-white", "{lit.instances}" } " instances (the ring of {RING_CUBES} small cubes is drawn instanced; its one mirrored cube splits it into three draws)"
            }
            p { class: "text-slate-300", "Errors while drawing: " span { id: "lit-error-count", class: "font-mono text-white", "{errors.read().len()}" } }
            for error in errors.read().iter() {
                p { class: "font-mono text-red-300", "{error}" }
            }
            p { class: "mt-2 text-slate-400", "A red cube turning, a yellow pillar, a shiny blue sphere bouncing, a pale translucent sphere you can see the ground through, and a ring of small white cubes going round, on a green checkered ground. The light comes from high front left; the ambient light is a cool grey. Pausing the clock stops everything, the auto-orbit included." }
        }
    }
}
