//! The "More" scene: the layer of more_layer.rs (a panorama background, a ring of cubes from a
//! render bundle, particles moved by a compute pass and drawn indirectly, two animated textures)
//! on its own clock, with a line of QUERY RESULTS in plain HTML - the GPU timer's pass times and
//! the count the compute pass wrote, read from the GPU twice a second, asynchronously, as a
//! browser must; and what the crate says when it is asked for occlusion queries - and one-off checks of the crate's asynchronous texture and
//! buffer paths: a headless target read back (colour and depth), a texture region written, copied
//! and read back, a buffer mapped at creation, a compute pipeline and a render bundle with
//! WebGPU's verdict awaited.

use std::cell::RefCell;
use std::rc::Rc;

use codebrix_webgpu::{wgpu, CanvasClock, CanvasLayers, GpuContext, RendererHandle, WebGpuCanvas};
use codebrix_webgpu_graphics::{Color4, GraphicsContext, HeadlessTargetOptions, RenderBundleOptions, Texture2DOptions, TextureRegion};
use codebrix_webplatform::browser::sleep_ms;
use dioxus::prelude::*;

use crate::more_layer::{MoreLayer, MoreView, ReadbackSlot, PARTICLES};

/// The query results the page shows.
#[derive(Clone, Debug, Default, PartialEq)]
struct QueryView {
    /// How many reads have completed.
    reads: u64,
    /// The instance count the compute pass wrote into the indirect arguments.
    particles_drawn: Option<u32>,
    /// The GPU milliseconds of the render pass and the compute pass (None: no timestamps).
    gpu_ms: Option<[f64; 2]>,
    /// True when the device has `timestamp-query`.
    timestamps: bool,
    /// The last read's failure, if any.
    failure: Option<String>,
}

/// The canvas, its clock, the query line and the checks.
#[component]
pub fn MoreScene(context: GpuContext) -> Element {
    let graphics = use_hook(|| GraphicsContext::wrap_gpu_context(&context));
    let view = use_signal(MoreView::default);
    let errors = use_signal(Vec::<String>::new);
    let mut queries = use_signal(|| QueryView { timestamps: graphics.has_feature(wgpu::Features::TIMESTAMP_QUERY), ..QueryView::default() });
    let mut paused = use_signal(|| false);
    let mut texture_check = use_signal(|| "waiting".to_string());
    let mut region_check = use_signal(|| "waiting".to_string());
    let mut mapped_check = use_signal(|| "waiting".to_string());
    let mut checked_builds = use_signal(|| "waiting".to_string());
    let readback: ReadbackSlot = use_hook(|| Rc::new(RefCell::new(None)));
    let clock = use_hook(CanvasClock::new);
    let layers = use_hook(|| CanvasLayers::single(RendererHandle::new(MoreLayer::new(graphics.clone(), readback.clone(), view, errors))));
    {
        let graphics = graphics.clone();
        use_hook(move || {
            spawn(async move {
                texture_check.set(check_headless_readback(&graphics).await);
                region_check.set(check_region_round_trip(&graphics).await);
                mapped_check.set(check_mapped_buffer(&graphics).await);
                checked_builds.set(check_awaited_builds(&graphics).await);
            });
        });
    }
    {
        // Twice a second: the query results and the indirect arguments, read asynchronously. The
        // task ends with the scene.
        let readback = readback.clone();
        use_hook(move || {
            spawn(async move {
                loop {
                    sleep_ms(500).await;
                    let Some(handles) = readback.borrow().clone() else { continue };
                    let mut next = queries.peek().clone();
                    next.failure = None;
                    match handles.indirect.read::<u32>(Some(4), 0).await {
                        Ok(arguments) => next.particles_drawn = arguments.get(1).copied(),
                        Err(error) => next.failure = Some(error.to_string()),
                    }
                    if let Some(timer) = handles.timer.as_ref() {
                        match timer.read_milliseconds().await {
                            Ok(Some(ms)) if ms.len() >= 2 => next.gpu_ms = Some([ms[0], ms[1]]),
                            Ok(_) => {}
                            Err(error) => next.failure = Some(error.to_string()),
                        }
                    }
                    next.reads += 1;
                    queries.set(next);
                }
            });
        });
    }
    let toggle_clock = {
        let clock = clock.clone();
        move |_| paused.set(clock.toggle())
    };
    let button = "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700";
    let state = if paused() { "paused" } else { "running" };
    let shown = view();
    let q = queries();
    // What the crate answers when asked for occlusion queries (it refuses them in this version).
    let occlusion = match graphics.create_occlusion_query_set(1, None) {
        Ok(_) => "available".to_string(),
        Err(error) => error.to_string(),
    };
    let drawn = q.particles_drawn.map(|n| n.to_string()).unwrap_or_else(|| "waiting".to_string());
    let gpu_time = match (q.timestamps, q.gpu_ms) {
        (false, _) => "not measured: this browser did not grant timestamp-query".to_string(),
        (true, Some([render, compute])) => format!("render pass {render:.3} ms, compute pass {compute:.3} ms"),
        (true, None) => "waiting".to_string(),
    };
    rsx! {
        WebGpuCanvas {
            context: context.clone(),
            layers,
            clock: clock.clone(),
            clear_color: wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            id: "more-canvas",
            label: "A ring of cubes and a turning helix of particles under a dusk panorama, with two animated badges in the corner. Drag to orbit, drag with the right or middle button or with Shift to pan, use the wheel to zoom.",
            class: "aspect-video w-full touch-none select-none rounded-lg ring-1 ring-white/10 focus-visible:outline-2 focus-visible:outline-sky-400",
        }
        div { class: "flex flex-wrap gap-3",
            button { id: "more-clock-toggle", class: "{button}", onclick: toggle_clock, if paused() { "Resume the clock" } else { "Pause the clock" } }
        }
        div { class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
            p { class: "text-slate-300",
                "Frame " span { id: "more-frame", class: "font-mono text-white", "{shown.frame}" }
                ", clock " span { id: "more-clock-state", "{state}" }
                " at " span { id: "more-time", class: "font-mono text-white", "{shown.time:.3}" } " s"
                "; animated textures on frames " span { id: "more-array-frame", class: "font-mono text-white", "{shown.array_frame}" }
                " (texture array) and " span { id: "more-streaming-frame", class: "font-mono text-white", "{shown.streaming_frame}" } " (streaming, ping-pong)"
            }
            p { id: "more-queries", class: "text-slate-300",
                "Queries (read #" span { id: "more-query-reads", class: "font-mono", "{q.reads}" } "): GPU time "
                span { id: "more-gpu-time", class: "font-mono text-white", "{gpu_time}" }
                "; particles drawn by the indirect draw "
                span { id: "more-particles-drawn", class: "font-mono text-white", "{drawn}" } " of {PARTICLES}"
            }
            p { class: "text-slate-300", "Occlusion queries: " span { id: "more-occlusion", class: "text-amber-200", "{occlusion}" } }
            if let Some(failure) = q.failure.as_ref() {
                p { id: "more-query-failure", class: "font-mono text-red-300", "{failure}" }
            }
            p { class: "text-slate-300", "Headless target read back (colour and depth): " span { id: "more-texture-readback", class: "font-mono text-white", "{texture_check}" } }
            p { class: "text-slate-300", "Texture region written, copied and read back: " span { id: "more-region", class: "font-mono text-white", "{region_check}" } }
            p { class: "text-slate-300", "Buffer mapped at creation, then read back: " span { id: "more-mapped", class: "font-mono text-white", "{mapped_check}" } }
            p { class: "text-slate-300", "Compute pipeline and render bundle with WebGPU's verdict awaited: " span { id: "more-checked", class: "font-mono text-white", "{checked_builds}" } }
            p { class: "text-slate-300", "Errors while drawing: " span { id: "more-error-count", class: "font-mono text-white", "{errors.read().len()}" } }
            for error in errors.read().iter() {
                p { class: "font-mono text-red-300", "{error}" }
            }
            p { class: "mt-2 text-slate-400", "The background is an HDR panorama made in code, drawn by the skybox. The ring of twelve cubes is a render bundle recorded once. The helix of particles is moved every frame by a compute pass, which also decides how many are drawn: the indirect draw reads its count from the GPU. The two badges at the top right are animated textures with the same twelve frames, one from a texture array, one streaming through a ring of layers. Pausing the clock freezes everything; drag to orbit, the wheel zooms." }
        }
    }
}

/// Clears a headless target (colour and Depth32Float) and reads both back.
async fn check_headless_readback(graphics: &GraphicsContext) -> String {
    let target = match graphics.create_headless_target(64, 48, HeadlessTargetOptions::default()) {
        Ok(target) => target,
        Err(error) => return format!("FAILED: {error}"),
    };
    let mut frame = graphics.begin_frame();
    if let Err(error) = frame.begin_render_pass_to(&target.view(), Some(Color4::new(0.2, 0.4, 0.6, 1.0)), Some(0.25)).map(drop) {
        return format!("FAILED: {error}");
    }
    if let Err(error) = frame.submit() {
        return format!("FAILED: {error}");
    }
    let rgba = match target.read_rgba().await {
        Ok(rgba) => rgba,
        Err(error) => return format!("FAILED: {error}"),
    };
    let depth = match target.read_depth().await {
        Ok(depth) => depth,
        Err(error) => return format!("FAILED: {error}"),
    };
    let expected = [51u8, 102, 153, 255];
    let colours_ok = rgba.len() == 64 * 48 * 4 && rgba.chunks_exact(4).all(|p| p == expected);
    let depth_ok = depth.len() == 64 * 48 && depth.iter().all(|d| *d == 0.25);
    if colours_ok && depth_ok {
        "ok (64 x 48: every pixel [51, 102, 153, 255], every depth 0.25)".to_string()
    } else {
        format!("FAILED: first pixel {:?}, first depth {:?}", &rgba.get(0..4), depth.first())
    }
}

/// Writes a region of a texture, copies the texture on the GPU, and reads the region back.
async fn check_region_round_trip(graphics: &GraphicsContext) -> String {
    let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC;
    let make = || graphics.create_texture_2d(8, 8, wgpu::TextureFormat::Rgba8Unorm, usage, Texture2DOptions { mip_level_count: 1, label: Some("region check".to_string()) });
    let (first, second) = match (make(), make()) {
        (Ok(first), Ok(second)) => (first, second),
        (Err(error), _) | (_, Err(error)) => return format!("FAILED: {error}"),
    };
    let region = TextureRegion::new(2, 3, 4, 2);
    let pixels: Vec<u8> = (0..4 * 2 * 4).map(|i| (i * 7 + 1) as u8).collect();
    if let Err(error) = first.write_region(&region, &pixels).and_then(|_| first.copy_to(&second).map(drop)) {
        return format!("FAILED: {error}");
    }
    match second.read_region(&region).await {
        Ok(read) if read == pixels => "ok (a 4 x 2 region at (2, 3) came back unchanged)".to_string(),
        Ok(read) => format!("FAILED: read {:?}", &read[..8.min(read.len())]),
        Err(error) => format!("FAILED: {error}"),
    }
}

/// Fills a buffer through its mapping at creation and reads it back.
async fn check_mapped_buffer(graphics: &GraphicsContext) -> String {
    let values = [5u32, 6, 7, 0xC0DE];
    let buffer = match graphics.create_mapped_buffer(16, wgpu::BufferUsages::STORAGE, Some("mapped check")) {
        Ok(buffer) => buffer,
        Err(error) => return format!("FAILED: {error}"),
    };
    if let Err(error) = buffer.write_mapped(&values, 0) {
        return format!("FAILED: {error}");
    }
    buffer.unmap();
    match buffer.read::<u32>(None, 0).await {
        Ok(read) if read == values => format!("ok ({})", read.iter().map(|v| format!("{v:#x}")).collect::<Vec<_>>().join(", ")),
        Ok(read) => format!("FAILED: read {read:?}"),
        Err(error) => format!("FAILED: {error}"),
    }
}

/// Builds the compute pipeline and an (empty) render bundle with WebGPU's verdict awaited.
async fn check_awaited_builds(graphics: &GraphicsContext) -> String {
    let shader = graphics.create_shader(include_str!("more_compute.wgsl"), Some("more compute (checked)"));
    if let Err(error) = graphics.create_compute_pipeline(&shader).build_checked().await {
        return format!("FAILED: {error}");
    }
    let recorder = graphics.create_render_bundle_with_formats(&[wgpu::TextureFormat::Rgba8Unorm], &RenderBundleOptions { label: Some("empty (checked)".to_string()), ..RenderBundleOptions::default() });
    match recorder.finish_checked(None).await {
        Ok(bundle) if bundle.command_count() == 0 => "ok".to_string(),
        Ok(bundle) => format!("FAILED: {} commands", bundle.command_count()),
        Err(error) => format!("FAILED: {error}"),
    }
}
