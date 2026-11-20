//! The "Sprites" scene: the sprites layer (sprites.rs) on a canvas of the codebrix-webgpu host,
//! with its own clock (pause and resume), a filtering switch, a plain-HTML legend of what the
//! canvas shows, and - when its button is pressed - a check of the device's ONE shared error
//! handler: two graphics contexts on the same device both receive the same deliberate WebGPU
//! error, and a context made afterwards receives none of it. The canvas's own context, made
//! before the error, receives it too: the layer counts it apart from its drawing errors.

use codebrix_webgpu::{wgpu, CanvasClock, CanvasLayers, GpuContext, RendererHandle, WebGpuCanvas};
use codebrix_webgpu_graphics::GraphicsContext;
use codebrix_webplatform::browser;
use dioxus::prelude::*;

use crate::sprites::{SpriteView, SpritesRenderer, CARD_NAMES, CELL, CELL_NAMES, REFUSED_LABEL};

/// The scene: the canvas at once; the shared error handler check when its button is pressed.
#[component]
pub fn SpriteScene(context: GpuContext) -> Element {
    rsx! { SpriteCanvas { context } }
}

/// Two contexts on one device, one deliberate error: both must see it, once each; a third
/// context made afterwards must not.
async fn check_shared_errors(context: &GpuContext) -> String {
    let first = GraphicsContext::wrap_gpu_context(context);
    let second = GraphicsContext::wrap_gpu_context(context);
    let refused = first.create_buffer(16, wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::UNIFORM, Some(REFUSED_LABEL));
    if refused.is_err() {
        return "FAILED: the library refused the buffer itself".to_string();
    }
    let mut first_errors = Vec::new();
    let mut second_errors = Vec::new();
    for _ in 0..40 {
        // The browser hands the error over when the device does work: an empty submission makes
        // it do some (a page that draws does that every frame anyway).
        first.queue().submit(std::iter::empty());
        browser::sleep_ms(50).await;
        first_errors.extend(first.take_errors());
        second_errors.extend(second.take_errors());
        if !first_errors.is_empty() && !second_errors.is_empty() {
            break;
        }
    }
    first.queue().submit(std::iter::empty());
    browser::sleep_ms(100).await;
    first_errors.extend(first.take_errors());
    second_errors.extend(second.take_errors());
    let later = GraphicsContext::wrap_gpu_context(context);
    let later_errors = later.take_errors().len();
    if first_errors.len() == 1 && second_errors.len() == 1 && first_errors == second_errors && later_errors == 0 {
        "ok (both contexts received the one error; a context made later received none)".to_string()
    } else {
        format!("FAILED: first {} error(s), second {} error(s), later {} error(s)", first_errors.len(), second_errors.len(), later_errors)
    }
}

/// The canvas with the sprites layer, its clock, the switches and the legend.
#[component]
fn SpriteCanvas(context: GpuContext) -> Element {
    let graphics = use_hook(|| GraphicsContext::wrap_gpu_context(&context));
    let view = use_signal(SpriteView::default);
    let errors = use_signal(Vec::<String>::new);
    let deliberate = use_signal(|| 0usize);
    let mut check = use_signal(|| None::<String>);
    let mut checking = use_signal(|| false);
    let mut nearest = use_signal(|| false);
    let mut paused = use_signal(|| false);
    let clock = use_hook(CanvasClock::new);
    let layers = use_hook(|| CanvasLayers::single(RendererHandle::new(SpritesRenderer::new(graphics.clone(), view, errors, deliberate, nearest))));
    let toggle_clock = {
        let clock = clock.clone();
        move |_| paused.set(clock.toggle())
    };
    let run_check = {
        let context = context.clone();
        move |_| {
            if checking() {
                return;
            }
            checking.set(true);
            check.set(None);
            let context = context.clone();
            spawn(async move {
                check.set(Some(check_shared_errors(&context).await));
                checking.set(false);
            });
        }
    };
    let button = "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700";
    let sprites = view();
    let state = if paused() { "paused" } else { "running" };
    let top = CARD_NAMES[sprites.top_card];
    let filtering = if nearest() { "nearest" } else { "linear" };
    let check_text = match (check(), checking()) {
        (_, true) => "checking...".to_string(),
        (Some(text), false) => text,
        (None, false) => "not run yet (press \"Check the shared error handler\")".to_string(),
    };
    let atlas = match sprites.atlas {
        (0, _, _) => "not made yet".to_string(),
        (width, height, mips) => format!("{width} x {height} texels ({} cells of {CELL} x {CELL}), {mips} mip levels", width / CELL),
    };
    rsx! {
        WebGpuCanvas {
            context: context.clone(),
            layers,
            clock: clock.clone(),
            clear_color: wgpu::Color { r: 0.005, g: 0.006, b: 0.012, a: 1.0 },
            id: "sprites-canvas",
            label: "Sprites and shapes drawn with the graphics library's sprite batch, animated with the clock.",
            class: "aspect-video w-full touch-none select-none rounded-lg ring-1 ring-white/10 focus-visible:outline-2 focus-visible:outline-sky-400",
        }
        div { class: "flex flex-wrap gap-3",
            button { id: "sprites-clock-toggle", class: "{button}", onclick: toggle_clock, if paused() { "Resume the clock" } else { "Pause the clock" } }
            button { id: "sprites-filter-toggle", class: "{button}", onclick: move |_| nearest.set(!nearest()), if nearest() { "Use linear filtering" } else { "Use nearest filtering" } }
            button { id: "sprites-run-check", class: "{button} disabled:opacity-50", disabled: checking(), onclick: run_check, "Check the shared error handler" }
        }
        div { class: "grid gap-4 sm:grid-cols-2",
            div { class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
                p { class: "text-slate-300",
                    "Frame " span { id: "sprites-frame", class: "font-mono text-white", "{sprites.frame}" }
                    ", clock " span { id: "sprites-clock-state", "{state}" } " at "
                    span { id: "sprites-time", class: "font-mono text-white", "{sprites.time:.2}" } " s"
                }
                p { class: "text-slate-300",
                    "Last batch: " span { id: "sprites-draw-calls", class: "font-mono text-white", "{sprites.draw_calls}" } " draw call(s), "
                    span { id: "sprites-quads", class: "font-mono text-white", "{sprites.quads}" } " quads"
                }
                p { class: "text-slate-300",
                    "Arrow at " span { id: "sprites-arrow", class: "font-mono text-white", "{sprites.arrow_degrees:.1}" } " degrees; card on top: "
                    span { id: "sprites-top-card", class: "font-mono text-white", "{top}" } "; filtering "
                    span { id: "sprites-filtering", "{filtering}" }
                }
                p { class: "text-slate-300", "Atlas: " span { id: "sprites-atlas", class: "font-mono text-white", "{atlas}" } }
                p { class: "text-slate-300", "Shared error handler: " span { id: "sprites-error-check", class: "font-mono text-white", "{check_text}" } }
                p { class: "text-slate-300", "Errors while drawing: " span { id: "sprites-error-count", class: "font-mono text-white", "{errors.read().len()}" } }
                p { class: "text-slate-300",
                    "The check's deliberate error, as received by this canvas's own context: "
                    span { id: "sprites-deliberate-count", class: "font-mono text-white", "{deliberate()}" }
                    " (it reaches every context made before it; it is not a drawing error)"
                }
                for error in errors.read().iter() {
                    p { class: "font-mono text-red-300", "{error}" }
                }
            }
            div { id: "sprites-legend", class: "rounded-lg bg-slate-900 p-4 text-sm text-slate-300 ring-1 ring-white/10",
                p { class: "font-semibold text-slate-100", "What the canvas shows" }
                ul { class: "mt-2 list-disc space-y-1 pl-5",
                    li { "Top left: the four cells of one texture atlas ({CELL_NAMES[0]}, {CELL_NAMES[1]}, {CELL_NAMES[2]}, {CELL_NAMES[3]}), drawn by source rectangle, each tinted with a colour that cycles." }
                    li { "Centre: a yellow arrow turning clockwise about its centre and growing and shrinking; above it, a pale blue star turning the other way about its top-left corner; a pink dot circling." }
                    li { "Right: three checkered cards, red, green and blue; every two seconds a different one is drawn last and lies on top." }
                    li { "Bottom left: a ring of coloured squares turned as one group (the transform stack) inside a circle with two clock hands." }
                    li { "Top right: a triangle whose colour shifts. Everything stops when the clock is paused." }
                }
                p { class: "mt-2 text-slate-400", "All of it is one sprite batch: one draw call per run of one texture (shapes never break a run). Text on this page is HTML: the graphics library draws no text." }
            }
        }
    }
}
