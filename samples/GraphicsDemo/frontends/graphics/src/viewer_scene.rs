//! The "Viewer" scene (the application's first scene): the desktop model viewer's complete scene
//! in the browser (viewer.rs, viewer_actors.rs). The page asks the backend for the scene's description (JSON) and its
//! model data (ONE binary blob, through the generated `_bytes` call of a [WireFile] action - the
//! platform's client does the fetching, the graphics crate takes the bytes), shows a loading line
//! while they arrive and a clear message when a request fails, then starts the canvas with the
//! viewer layer on its own clock.
//!
//! The desktop viewer's command-line switches are query parameters of the page: `?ldr` draws
//! straight into the canvas (no HDR target, no tone mapping) and `?no-skybox` clears to the flat
//! sky colour instead of drawing the environment; both together work too. Plain HTML controls
//! choose the tone mapper's operator and exposure while it runs, and a button switches the light
//! between the procedural sky and the Small Cathedral HDR panorama, whose .hdr bytes are fetched
//! from the backend the first time (once) through the generated `small_cathedral_bytes` call.
//! What was fetched is kept for the life of the page (page_cache.rs): coming back to the scene
//! asks the backend for nothing.

use std::rc::Rc;

use codebrix_webgpu::{wgpu, CanvasClock, CanvasLayers, GpuContext, RendererHandle, WebGpuCanvas};
use codebrix_webgpu_graphics::{GraphicsContext, ToneMapOperator};
use dioxus::prelude::*;

use crate::generated::api::{environments, models};
use crate::generated::models::ViewerScene as ViewerSceneData;
use crate::page_cache;
use crate::viewer::{srgb_to_linear, EnvironmentChoice, PanoramaBytes, ViewerControls, ViewerOptions, ViewerRenderer, ViewerView};

/// What the two requests brought.
#[derive(Clone)]
struct Loaded {
    scene: ViewerSceneData,
    blob: Rc<Vec<u8>>,
}

/// The page's key of the panorama's bytes.
const PANORAMA_KEY: &str = "environment:small-cathedral";

/// Fetches the description, then the blob - or takes them from what the page kept.
async fn load() -> Result<Loaded, String> {
    let scene = page_cache::viewer_scene(async { models::scene().await.map_err(|e| format!("The scene's description could not be read from the backend: {}", e.message())) }).await?;
    let blob = page_cache::bytes("models:scene", async { models::scene_data_bytes().await.map_err(|e| format!("The scene's model data could not be read from the backend: {}", e.message())) }).await?;
    Ok(Loaded { scene, blob })
}

/// The viewer's switches from the page's query string: `?ldr`, `?no-skybox` (names alone or with
/// a value, separated by `&`).
fn options_from_query() -> ViewerOptions {
    let search = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
    let names: Vec<String> = search.trim_start_matches('?').split('&').map(|pair| pair.split('=').next().unwrap_or("").to_ascii_lowercase()).collect();
    ViewerOptions { hdr: !names.iter().any(|n| n == "ldr"), skybox: !names.iter().any(|n| n == "no-skybox") }
}

/// The loading line, the failure message, or the canvas.
#[component]
pub fn ViewerScene(context: GpuContext) -> Element {
    let loaded = use_resource(load);
    let state = loaded.read();
    match &*state {
        None => rsx! { p { id: "viewer-loading", role: "status", class: "rounded-lg bg-slate-900 p-4 text-sm text-slate-300 ring-1 ring-white/10", "Loading the viewer's models from the backend..." } },
        Some(Err(message)) => rsx! {
            div { id: "viewer-failed", role: "alert", class: "rounded-lg border border-red-400/40 bg-red-400/10 p-5 text-red-100",
                p { class: "text-lg font-semibold text-red-200", "The viewer's models could not be loaded" }
                p { class: "mt-2 text-sm", "{message}" }
                p { class: "mt-2 text-sm", "Check that the backend is running, then switch to another scene and back to try again." }
            }
        },
        Some(Ok(loaded)) => rsx! { ViewerCanvas { context: context.clone(), scene: loaded.scene.clone(), blob: loaded.blob.clone() } },
    }
}

/// The canvas with the viewer layer, its clock, the controls, the read-outs and the legend.
#[component]
fn ViewerCanvas(context: GpuContext, scene: ViewerSceneData, blob: Rc<Vec<u8>>) -> Element {
    let graphics = use_hook(|| GraphicsContext::wrap_gpu_context(&context));
    let options = use_hook(options_from_query);
    let view = use_signal(ViewerView::default);
    let errors = use_signal(Vec::<String>::new);
    let mut paused = use_signal(|| false);
    let controls = ViewerControls {
        environment: use_signal(EnvironmentChoice::default),
        // The bytes the page kept from an earlier visit, if any.
        panorama: use_signal(|| page_cache::kept_bytes(PANORAMA_KEY).map(PanoramaBytes::Loaded).unwrap_or_default()),
        // The desktop viewer's operator: Khronos PBR Neutral keeps the kits' flat colours true.
        operator: use_signal(|| ToneMapOperator::KhronosPbrNeutral),
        exposure: use_signal(|| 1.0f32),
        resume_requests: use_signal(|| 0u64),
    };
    let clock = use_hook(CanvasClock::new);
    let blob_bytes = blob.len();
    let layers = {
        let scene = scene.clone();
        use_hook(move || CanvasLayers::single(RendererHandle::new(ViewerRenderer::new(graphics.clone(), blob, scene, options, controls, view, errors))))
    };
    let toggle_clock = {
        let clock = clock.clone();
        move |_| paused.set(clock.toggle())
    };
    // The clock's own seek and speed: every layer (the actors, the auto-orbit) follows them.
    let seek_back = {
        let clock = clock.clone();
        move |_| clock.seek((clock.elapsed() - 5.0).max(0.0))
    };
    let seek_forward = {
        let clock = clock.clone();
        move |_| clock.seek(clock.elapsed() + 5.0)
    };
    let mut speed = use_signal(|| 1.0f64);
    let toggle_speed = {
        let clock = clock.clone();
        move |_| {
            let next = if speed() == 1.0 { 0.5 } else { 1.0 };
            clock.set_speed(next);
            speed.set(next);
        }
    };
    let mut environment = controls.environment;
    let mut panorama = controls.panorama;
    let switch_environment = move |_| {
        let next = if environment() == EnvironmentChoice::ProceduralSky { EnvironmentChoice::Cathedral } else { EnvironmentChoice::ProceduralSky };
        if next == EnvironmentChoice::Cathedral && *panorama.peek() == PanoramaBytes::NotRequested {
            // Fetched once, the first time; the layer builds the environment when the bytes are here.
            panorama.set(PanoramaBytes::Loading);
            spawn(async move {
                let result = page_cache::bytes(PANORAMA_KEY, async { environments::small_cathedral_bytes().await.map_err(|error| format!("The panorama could not be read from the backend: {}", error.message())) }).await;
                panorama.set(match result {
                    Ok(bytes) => PanoramaBytes::Loaded(bytes),
                    Err(message) => PanoramaBytes::Failed(message),
                });
            });
        }
        environment.set(next);
    };
    let mut operator = controls.operator;
    let mut exposure = controls.exposure;
    let mut resume_requests = controls.resume_requests;
    let light = &scene.lighting;
    let clear = wgpu::Color { r: srgb_to_linear(light.sky_red) as f64, g: srgb_to_linear(light.sky_green) as f64, b: srgb_to_linear(light.sky_blue) as f64, a: 1.0 };
    let button = "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700";
    let field = "rounded-md bg-slate-800 px-2 py-1 text-sm text-slate-100 ring-1 ring-white/10 disabled:opacity-50";
    let shown = view();
    let state = if paused() { "paused" } else { "running" };
    let orbit = if shown.auto_orbit { "auto-orbiting" } else { "user" };
    let path = if options.hdr { format!("HDR: an RGBA16Float target of {} x {}, tone mapped into the canvas", shown.hdr_size.0, shown.hdr_size.1) } else { "LDR (?ldr): straight into the canvas, no tone mapping".to_string() };
    let skybox = if options.skybox { format!("on ({} draws)", shown.skybox_draws) } else { "off (?no-skybox): the flat sky colour".to_string() };
    let drawn_operator = shown.operator.map_or("none (LDR)", ToneMapOperator::name);
    let panorama_state = match &*panorama.read() {
        PanoramaBytes::NotRequested => "not requested yet".to_string(),
        PanoramaBytes::Loading => "loading from the backend...".to_string(),
        PanoramaBytes::Loaded(bytes) => format!("{} bytes from the backend", bytes.len()),
        PanoramaBytes::Failed(message) => message.clone(),
    };
    rsx! {
        div { class: "relative",
            WebGpuCanvas {
                context: context.clone(),
                layers,
                clock: clock.clone(),
                clear_color: clear,
                id: "viewer-canvas",
                label: "The model viewer's scene: a chest opening and closing, animals walking and dancing around it, a bird circling overhead, and trees, rocks, barrels, a camp fire, grass and a fence on a green ground under a sky. Drag to orbit, use the wheel to zoom; on a touch screen, one finger orbits and two fingers pinch to zoom.",
                class: "aspect-video w-full touch-none select-none rounded-lg ring-1 ring-white/10 focus-visible:outline-2 focus-visible:outline-sky-400",
            }
            div { id: "viewer-legend", class: "pointer-events-none absolute left-3 top-3 rounded-md bg-slate-950/70 px-3 py-2 text-xs text-slate-200 ring-1 ring-white/10",
                p { class: "font-semibold text-white", "Controls" }
                p { "Drag: orbit (the scene follows the pointer)" }
                p { "Wheel: zoom" }
                p { "Touch: one finger orbits, two fingers pinch to zoom" }
                p { "It turns by itself until you drag." }
            }
        }
        div { class: "flex flex-wrap items-center gap-3",
            button { id: "viewer-clock-toggle", class: "{button}", onclick: toggle_clock, if paused() { "Resume the clock" } else { "Pause the clock" } }
            button { id: "viewer-auto-orbit", class: "{button}", onclick: move |_| resume_requests.set(resume_requests() + 1), "Orbit by itself again" }
            button { id: "viewer-seek-back", class: "{button}", onclick: seek_back, "Clock -5 s" }
            button { id: "viewer-seek-forward", class: "{button}", onclick: seek_forward, "Clock +5 s" }
            button { id: "viewer-speed", class: "{button}", onclick: toggle_speed, if speed() == 1.0 { "Half speed" } else { "Normal speed" } }
            button { id: "viewer-environment-toggle", class: "{button}", onclick: switch_environment,
                if environment() == EnvironmentChoice::ProceduralSky { "Light it with the HDR panorama" } else { "Light it with the procedural sky" }
            }
        }
        div { class: "flex flex-wrap items-center gap-3 rounded-lg bg-slate-900 p-3 text-sm text-slate-300 ring-1 ring-white/10",
            label { r#for: "viewer-tone-operator-select", "Tone mapping" }
            select {
                id: "viewer-tone-operator-select",
                class: "{field}",
                disabled: !options.hdr,
                onchange: move |event: Event<FormData>| {
                    if let Some(chosen) = ToneMapOperator::ALL.iter().find(|o| o.name() == event.value()) {
                        operator.set(*chosen);
                    }
                },
                for choice in ToneMapOperator::ALL {
                    option { value: choice.name(), selected: operator() == choice, "{choice.name()}" }
                }
            }
            label { r#for: "viewer-exposure-input", "Exposure" }
            input {
                id: "viewer-exposure-input",
                class: "w-40 accent-sky-400 disabled:opacity-50",
                r#type: "range",
                min: "0.25",
                max: "4",
                step: "0.05",
                disabled: !options.hdr,
                value: "{exposure()}",
                oninput: move |event: Event<FormData>| {
                    if let Ok(value) = event.value().parse::<f32>() {
                        exposure.set(value.clamp(0.25, 4.0));
                    }
                },
            }
            span { id: "viewer-exposure-value", class: "font-mono text-white", "{exposure():.2}" }
            if !options.hdr {
                span { class: "text-slate-400", "(the tone mapper is used only in the HDR path; this page was opened with ?ldr)" }
            }
        }
        div { class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
            p { class: "text-slate-300",
                "Frame " span { id: "viewer-frame", class: "font-mono text-white", "{shown.frame}" }
                ", clock " span { id: "viewer-clock-state", "{state}" } ", camera "
                span { id: "viewer-orbit-mode", "{orbit}" }
            }
            p { class: "text-slate-300",
                "Yaw " span { id: "viewer-yaw", class: "font-mono text-white", "{shown.yaw:.3}" }
                " rad, pitch " span { id: "viewer-pitch", class: "font-mono text-white", "{shown.pitch:.3}" }
                " rad, distance " span { id: "viewer-distance", class: "font-mono text-white", "{shown.distance:.3}" }
            }
            p { class: "text-slate-300",
                "Light: " span { id: "viewer-environment", class: "text-white", "{shown.environment}" }
                " (" span { id: "viewer-environment-size", class: "font-mono text-white", "{shown.environment_size}" } "-texel cube; its decoding and baking recorded in "
                span { id: "viewer-environment-build-ms", class: "font-mono text-white", "{shown.environment_build_ms}" } " ms of CPU time, the GPU bakes it after); panorama: "
                span { id: "viewer-panorama-state", class: "text-white", "{panorama_state}" }
            }
            p { class: "text-slate-300", "Rendering: " span { id: "viewer-render-path", class: "text-white", "{path}" } }
            p { class: "text-slate-300",
                "Tone map drawn: " span { id: "viewer-tone-operator", class: "font-mono text-white", "{drawn_operator}" }
                ", exposure " span { id: "viewer-tone-exposure", class: "font-mono text-white", "{shown.exposure:.2}" }
                "; skybox " span { id: "viewer-skybox", class: "text-white", "{skybox}" }
            }
            p { class: "text-slate-300",
                "Model data: " span { id: "viewer-blob-bytes", class: "font-mono text-white", "{blob_bytes}" } " bytes, "
                span { id: "viewer-models", class: "font-mono text-white", "{shown.models}" } " models, "
                span { id: "viewer-instances", class: "font-mono text-white", "{shown.instances}" } " placed, "
                span { id: "viewer-triangles", class: "font-mono text-white", "{shown.triangles}" } " triangles"
            }
            p { class: "text-slate-300",
                "Last frame: " span { id: "viewer-draw-calls", class: "font-mono text-white", "{shown.draw_calls}" } " draw call(s), "
                span { id: "viewer-drawn", class: "font-mono text-white", "{shown.drawn}" } " primitive instances, "
                span { id: "viewer-pipelines", class: "font-mono text-white", "{shown.pipelines}" } " pipeline(s)"
            }
            p { class: "text-slate-300",
                "Animation: scene time " span { id: "viewer-scene-time", class: "font-mono text-white", "{shown.scene_time:.3}" } " s; "
                span { id: "viewer-actors", class: "font-mono text-white", "{shown.actors}" } " animated actors playing "
                span { id: "viewer-clips-playing", class: "font-mono text-white", "{shown.clips_playing}" } " clip(s); pose "
                span { id: "viewer-pose-signature", class: "font-mono text-white", "{shown.pose_signature:.4}" } "; the Avian at ("
                span { id: "viewer-bird", class: "font-mono text-white", "{shown.bird.0:.3}, {shown.bird.1:.3}, {shown.bird.2:.3}" } ")"
            }
            p { class: "text-slate-300",
                "Routine of " span { id: "viewer-routine", class: "font-mono text-white", "{shown.routine}" }
                "; re-posed after a jump of the clock " span { id: "viewer-seeks", class: "font-mono text-white", "{shown.seeks}" } " time(s)"
            }
            p { class: "text-slate-300", "Errors while drawing: " span { id: "viewer-error-count", class: "font-mono text-white", "{errors.read().len()}" } }
            for error in errors.read().iter() {
                p { class: "font-mono text-red-300", "{error}" }
            }
            p { class: "mt-2 text-slate-400",
                "The desktop viewer's ground and static props with physically based shading, lit by image-based light from its procedural sky and its sun (or by the HDR panorama alone), with the sky drawn behind them and the high-dynamic-range picture tone mapped for the screen. Open the page with "
                a { class: "text-sky-300 hover:text-sky-200", href: "?ldr", "?ldr" } ", "
                a { class: "text-sky-300 hover:text-sky-200", href: "?no-skybox", "?no-skybox" } " or "
                a { class: "text-sky-300 hover:text-sky-200", href: "?ldr&no-skybox", "?ldr&no-skybox" } " for the desktop viewer's switches ("
                a { class: "text-sky-300 hover:text-sky-200", href: "?", "back to the defaults" } "). The chest opens and closes, four animals walk a ring, four more stand facing the chest and cross-fade through idling, dancing, eating and cheering, and the Avian circles overhead - all on the page's clock. Pausing the clock stops everything, the auto-orbit included; the clock buttons under the canvas jump it back or forward, or halve its speed."
            }
        }
    }
}
