//! `/animation/?scene=on-the-page`: DRAWING ON THE PAGE with the animation crate. The page has its
//! own visible background (a gradient and a checker pattern, in the page's stylesheet) and
//! ordinary HTML text; a FRAMELESS, TRANSPARENT canvas (`transparent: true`; no border, ring or
//! rounded corners) lies over it. A session drawn as a layer leaves its background out by default,
//! so its shapes and text float on the page: discs and a card move over it, translucent parts
//! blend with it, and a half-transparent square and an opaque grey disc stay still (fixed places
//! for a test tool to read). "Hide the canvas" makes the canvas invisible (a class, no inline style), so
//! the page can be seen - or read back - without it, at the same place. The animation is OPTIONAL
//! (`WebGpuBoundary` without a fallback): where WebGPU is not available the page shows its own
//! background and text alone, with no message.

use std::cell::RefCell;
use std::rc::Rc;

use codebrix_webgpu::{use_gpu, CanvasClock, CanvasFrame, CanvasLayers, CanvasRenderer, CanvasTarget, GpuContext, GpuErrorSubscription, RendererHandle, WebGpuBoundary, WebGpuCanvas};
use codebrix_webgpu_animation::rendering::AnimationLayer;
use codebrix_webgpu_animation::shapes::{CircleShape, RectangleShape, RectangleShapeOptions, ShapeOptions, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions};
use dioxus::prelude::*;

/// The drawing's calibrated size (the canvas is 1280 x 720 CSS pixels).
const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 720.0;
/// The still, half-transparent square (calibrated units = CSS pixels here): x, y, size.
pub const STILL_SQUARE: (f32, f32, f32) = (1040.0, 470.0, 160.0);
/// Its colour: blue at alpha 128 of 255.
pub const STILL_COLOR: Color = Color::from_rgba(30, 60, 200, 128);
/// A still, opaque mid-grey disc (centre x, y, radius): its anti-aliased edge over the bright page
/// is where a premultiplication error would show as a light fringe.
pub const STILL_DISC: (f32, f32, f32) = (1215.0, 120.0, 45.0);

/// The page's layer: the session rebuilt for the frame's time, drawn WITHOUT its background (the
/// layer default), and the device's error count.
struct PageLayer {
    inner: AnimationLayer,
    errors: Option<GpuErrorSubscription>,
    error_count: Signal<usize>,
    frames: Signal<u64>,
}

impl CanvasRenderer for PageLayer {
    fn prepare(&mut self, context: &GpuContext, target: &CanvasTarget) {
        if self.errors.is_none() {
            self.errors = Some(context.subscribe_errors());
        }
        self.inner.prepare(context, target);
    }

    fn resize(&mut self, context: &GpuContext, width: u32, height: u32) {
        self.inner.resize(context, width, height);
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        self.inner.render(frame);
        let new = self.errors.as_ref().map(|e| e.take().len()).unwrap_or(0);
        if new > 0 {
            let count = *self.error_count.peek();
            self.error_count.set(count + new);
        }
        self.frames.set(frame.info.number);
    }
}

/// What the session shows at a time (seconds): it never reads a clock.
fn build(session: &mut DrawingSession, seconds: f64) {
    let fonts = session.fonts().clone();
    let t = seconds as f32;
    let layer = session.add_layer("Floating", Color::WHITE);
    layer.clear();
    // Three translucent discs drifting across the page.
    for (k, color) in [Color::from_rgba(255, 90, 120, 150), Color::from_rgba(90, 200, 255, 150), Color::from_rgba(255, 200, 60, 150)].into_iter().enumerate() {
        let phase = t * 0.6 + k as f32 * 2.1;
        let x = 640.0 + 420.0 * phase.sin();
        let y = 330.0 + 140.0 * (phase * 1.3).cos();
        layer.add_shape(CircleShape::new(x, y, 90.0, ShapeOptions { color: Some(color), is_filled: true, ..ShapeOptions::default() }).unwrap());
    }
    // A card with an opaque outline and a translucent body, gliding left and right, with text.
    let card_x = 120.0 + 260.0 * (1.0 + (t * 0.5).sin());
    layer.add_shape(RectangleShape::new(card_x, 520.0, 380.0, 120.0, RectangleShapeOptions { color: Some(Color::from_rgba(20, 24, 40, 110)), is_filled: true, corner_radius: 18.0, ..RectangleShapeOptions::default() }).unwrap());
    layer.add_shape(RectangleShape::new(card_x, 520.0, 380.0, 120.0, RectangleShapeOptions { stroke_thickness: 3.0, color: Some(Color::from_rgb(255, 255, 255)), corner_radius: 18.0, ..RectangleShapeOptions::default() }).unwrap());
    layer.add_shape(TextShape::new("Floating on the page", card_x + 190.0, 592.0, 34.0, &fonts, TextShapeOptions { color: Some(Color::WHITE), align: TextAlign::Center, ..TextShapeOptions::default() }).unwrap());
    // The still square, half transparent.
    let (x, y, size) = STILL_SQUARE;
    layer.add_shape(RectangleShape::new(x, y, size, size, RectangleShapeOptions { color: Some(STILL_COLOR), is_filled: true, ..RectangleShapeOptions::default() }).unwrap());
    let (cx, cy, r) = STILL_DISC;
    layer.add_shape(CircleShape::new(cx, cy, r, ShapeOptions { color: Some(Color::from_rgb(128, 128, 128)), is_filled: true, ..ShapeOptions::default() }).unwrap());
    session.set_time(seconds);
}

/// The page: its own background and text, the frameless transparent canvas over them, and a
/// button that hides the canvas.
///
/// THE ANIMATION IS OPTIONAL: the canvas, and the controls and counters that belong to it, are in
/// two `WebGpuBoundary`s without a fallback. Where WebGPU is not available the page is its own
/// background and text alone - no animation, no controls, no message - and the fonts are never
/// fetched. The two boundaries share the page's one request to the browser.
#[component]
pub fn OnThePage() -> Element {
    let clock = use_hook(CanvasClock::new);
    let error_count = use_signal(|| 0usize);
    let frames = use_signal(|| 0u64);
    let mut hidden = use_signal(|| false);
    let mut paused = use_signal(|| false);
    let toggle_clock = {
        let clock = clock.clone();
        move |_| paused.set(clock.toggle())
    };
    let button = "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700";
    rsx! {
        div { id: "page-surface", class: "relative h-[720px] w-[1280px] overflow-hidden bg-linear-to-br from-emerald-200 via-sky-200 to-violet-300",
            div { class: "absolute inset-0 grid grid-cols-8 grid-rows-4",
                for i in 0..32 {
                    div { key: "{i}", class: if (i / 8 + i) % 2 == 0 { "bg-white/30" } else { "" } }
                }
            }
            div { class: "absolute inset-x-12 top-10 text-slate-800",
                h2 { class: "text-4xl font-bold", "An animation on the page" }
                p { class: "mt-3 max-w-2xl text-lg", "This gradient, the checker pattern and this text belong to the page (HTML and CSS). The canvas over them has no frame and a transparent background; the drawing session leaves its background out, so only its shapes and text appear. The animation is optional: a browser without WebGPU leaves it out and shows this page alone, with no message." }
            }
            p { class: "absolute bottom-6 left-12 text-base text-slate-700", "Ordinary page text, under the canvas." }
            // The animation: only where WebGPU works. Elsewhere the page is simply the page.
            WebGpuBoundary {
                PageAnimation { clock: clock.clone(), hidden, error_count, frames }
            }
        }
        // The animation's controls and counters: a second boundary, sharing the first one's request.
        WebGpuBoundary {
            div { class: "flex flex-wrap items-center gap-3",
                button { id: "page-clock-toggle", class: "{button}", onclick: toggle_clock, if paused() { "Resume the clock" } else { "Pause the clock" } }
                button { id: "page-canvas-toggle", class: "{button}", onclick: move |_| hidden.set(!hidden()), if hidden() { "Show the canvas" } else { "Hide the canvas" } }
            }
            div { class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
                p { class: "text-slate-300", "Frame " span { id: "page-frame", class: "font-mono text-white", "{frames}" } }
                p { class: "text-slate-300", "Errors while drawing: " span { id: "page-error-count", class: "font-mono text-white", "{error_count}" } }
                p { class: "mt-2 text-slate-400", "DRAWING ON THE PAGE: WebGpuCanvas with transparent: true and no frame in the page's styling; the session drawn as a layer without its background (the default). The canvas and these controls are in WebGpuBoundary: without WebGPU the page shows neither." }
            }
        }
    }
}

/// Inside the boundary: fetches the fonts from the backend (only where WebGPU works), then shows
/// the canvas. Nothing shows while they load.
#[component]
fn PageAnimation(clock: CanvasClock, hidden: Signal<bool>, error_count: Signal<usize>, frames: Signal<u64>) -> Element {
    let loaded = use_resource(crate::load_fonts);
    let state = loaded.read();
    match &*state {
        None => rsx! {},
        Some(Err(reason)) => rsx! { p { id: "fonts-failed", class: "absolute bottom-16 left-12 rounded-lg border border-red-400/40 bg-red-400/10 p-5 text-red-900", "{reason}" } },
        Some(Ok(fonts)) => rsx! { PageCanvas { clock: clock.clone(), hidden, error_count, frames, fonts: fonts.clone() } },
    }
}

/// The frameless transparent canvas, on the boundary's context (`use_gpu`).
#[component]
fn PageCanvas(clock: CanvasClock, hidden: Signal<bool>, error_count: Signal<usize>, frames: Signal<u64>, fonts: FontRegistry) -> Element {
    let context = use_gpu();
    let layers = use_hook(|| {
        let session = Rc::new(RefCell::new(DrawingSession::new(DrawingSessionOptions { calibration_size: (WIDTH as u32, HEIGHT as u32), layer_opacity: 255, background_fill_color: Color::from_rgb(18, 20, 30), fonts: Some(fonts.clone()), ..DrawingSessionOptions::default() })));
        // The session HAS a background colour - and, drawn as a layer, leaves it out by default.
        let inner = AnimationLayer::new(move |frame, scene| {
            let mut s = session.borrow_mut();
            build(&mut s, frame.info.time);
            s.render(scene, frame.info.width, frame.info.height);
        });
        CanvasLayers::single(RendererHandle::new(PageLayer { inner, errors: None, error_count, frames }))
    });
    let canvas_class = if hidden() { "invisible absolute inset-0 block h-[720px] w-[1280px] touch-none select-none" } else { "absolute inset-0 block h-[720px] w-[1280px] touch-none select-none" };
    rsx! {
        WebGpuCanvas {
            context,
            layers,
            clock,
            transparent: true,
            id: "page-canvas",
            label: "Shapes and text drawn by the animation crate, floating over the page.",
            class: canvas_class,
        }
    }
}
