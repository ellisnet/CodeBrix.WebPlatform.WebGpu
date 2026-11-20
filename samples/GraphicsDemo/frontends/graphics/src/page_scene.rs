//! The "On the page" scene (also `?scene=on-the-page`): DRAWING ON THE PAGE with the graphics
//! crate. The page itself has a visible background (a gradient in the page's stylesheet) and
//! ordinary HTML text; a FRAMELESS, TRANSPARENT canvas (`transparent: true`, no border, ring or
//! rounded corners) lies over it, and a few of the viewer's models turn on it - with HDR ON (the
//! scene into an RGBA16Float HeadlessRenderTarget cleared to TRANSPARENT, then the ToneMapper with
//! `preserve_alpha` into the canvas frame) and no skybox, so the page shows behind and around
//! them. The models are lit by the context's default environment (the procedural sky); the sky
//! itself is not drawn.
//!
//! "Hide the canvas" makes the canvas invisible (a class, no inline style), so the page can be
//! seen - or read back by a test tool - without it, at the same place; "Change the page's
//! background" swaps the page's gradient (where the models are drawn nothing changes; around and
//! between them the new page shows).

use std::rc::Rc;

use codebrix_webgpu::glam::{Mat4, Quat, Vec3};
use codebrix_webgpu::{wgpu, CanvasClock, CanvasFrame, CanvasLayers, CanvasRenderer, CanvasTarget, GpuContext, RendererHandle, WebGpuCanvas};
use codebrix_webgpu_graphics::{Camera, Color4, GraphicsContext, HeadlessRenderTarget, HeadlessTargetOptions, ModelInstance, ModelLoadOptions, ModelRenderer, SceneLighting, ToneMapOperator, ToneMapper};
use dioxus::prelude::*;

use crate::generated::api::models;
use crate::page_cache;

/// How many of the viewer's models turn on the page.
const MODEL_COUNT: usize = 4;
/// The depth format of the HDR target.
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

/// What the layer draws with.
struct Gpu {
    renderer: ModelRenderer,
    tone_mapper: ToneMapper,
    /// Each instance with its size-normalising scale, its centre and its place in the row.
    instances: Vec<(ModelInstance, f32, Vec3, f32)>,
}

/// A few models turning in a row, HDR on, the scene's alpha kept for the page.
struct PageModelsLayer {
    graphics: GraphicsContext,
    blob: Rc<Vec<u8>>,
    gpu: Option<Gpu>,
    hdr: Option<HeadlessRenderTarget>,
    camera: Camera,
    errors: Signal<Vec<String>>,
    frames: Signal<u64>,
}

impl PageModelsLayer {
    fn report(&mut self, error: String) {
        let mut errors = self.errors.write();
        if errors.len() < 20 {
            errors.push(error);
        }
    }

    /// Loads the viewer's models and takes MODEL_COUNT of them that stand up (not flat ground
    /// tiles), each scaled to the same size.
    fn create_gpu(&mut self) -> Result<Gpu, String> {
        let models = self.graphics.load_models(&self.blob, &ModelLoadOptions::default()).map_err(|e| e.to_string())?;
        let mut chosen = Vec::new();
        for model in &models {
            let size = model.bounds_max() - model.bounds_min();
            let wide = size.x.max(size.z);
            if size.y > 0.4 * wide && wide > 0.0 && chosen.len() < MODEL_COUNT {
                chosen.push(model);
            }
        }
        let mut instances = Vec::new();
        for (i, model) in chosen.iter().enumerate() {
            let radius = ((model.bounds_max() - model.bounds_min()).length() * 0.5).max(0.001);
            let place = (i as f32 - (chosen.len() as f32 - 1.0) / 2.0) * 2.4;
            instances.push((model.create_instance(), 1.0 / radius, model.bounds_center(), place));
        }
        let renderer = self.graphics.create_model_renderer().map_err(|e| e.to_string())?;
        let mut tone_mapper = self.graphics.create_tone_mapper().map_err(|e| e.to_string())?;
        tone_mapper.operator = ToneMapOperator::KhronosPbrNeutral;
        // DRAWING ON THE PAGE WITH HDR: keep the scene's alpha through the tone map.
        tone_mapper.preserve_alpha = true;
        Ok(Gpu { renderer, tone_mapper, instances })
    }
}

impl CanvasRenderer for PageModelsLayer {
    fn prepare(&mut self, _context: &GpuContext, _target: &CanvasTarget) {
        match self.create_gpu() {
            Ok(gpu) => self.gpu = Some(gpu),
            Err(error) => self.report(error),
        }
        for error in self.graphics.take_errors() {
            self.report(error.to_string());
        }
    }

    fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
        let options = HeadlessTargetOptions { format: wgpu::TextureFormat::Rgba16Float, with_depth: true, depth_format: DEPTH_FORMAT };
        match self.graphics.create_headless_target(width, height, options) {
            Ok(hdr) => self.hdr = Some(hdr),
            Err(error) => self.report(error.to_string()),
        }
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let time = frame.info.time as f32;
        let number = frame.info.number;
        let graphics = self.graphics.clone();
        let mut failure = None;
        if let (Some(gpu), Some(hdr)) = (self.gpu.as_mut(), self.hdr.as_ref()) {
            // Each model turns about its own upright axis, at its place in the row (time from the
            // frame: pausing the clock stops them).
            for (k, (instance, scale, center, place)) in gpu.instances.iter_mut().enumerate() {
                let turn = Quat::from_rotation_y(time * 0.8 + k as f32 * 0.9);
                instance.world = Mat4::from_translation(Vec3::new(*place, 0.0, 0.0)) * Mat4::from_quat(turn) * Mat4::from_scale(Vec3::splat(*scale)) * Mat4::from_translation(-*center);
            }
            let lighting = SceneLighting::default();
            let camera = &self.camera;
            let result: Result<(), codebrix_webgpu_graphics::GraphicsException> = (|| {
                let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
                {
                    // The scene into the HDR target, cleared to TRANSPARENT: no sky, no backdrop.
                    let mut pass = graphics_frame.begin_render_pass_to(&hdr.view(), Some(Color4::TRANSPARENT), Some(1.0))?;
                    gpu.renderer.begin(&pass, camera, Some(&lighting))?;
                    for (instance, _, _, _) in &gpu.instances {
                        gpu.renderer.draw(instance)?;
                    }
                    gpu.renderer.end(&mut pass)?;
                }
                // The tone map over the canvas frame (LOAD), keeping the scene's alpha.
                let mut pass = graphics_frame.begin_render_pass(None, None)?;
                gpu.tone_mapper.apply_in_pass(&mut pass, hdr.color())?;
                Ok(())
            })();
            if let Err(error) = result {
                failure = Some(error.to_string());
            }
        }
        if let Some(error) = failure {
            self.report(error);
        }
        for error in self.graphics.take_errors() {
            self.report(error.to_string());
        }
        self.frames.set(number);
    }
}

/// Fetches the viewer's model data, or takes it from what the page kept.
async fn load() -> Result<Rc<Vec<u8>>, String> {
    page_cache::bytes("models:scene", async { models::scene_data_bytes().await.map_err(|e| format!("The scene's model data could not be read from the backend: {}", e.message())) }).await
}

/// The loading line, the failure message, or the page with its canvas.
#[component]
pub fn PageScene(context: GpuContext) -> Element {
    let loaded = use_resource(load);
    let state = loaded.read();
    match &*state {
        None => rsx! { p { id: "page-loading", role: "status", class: "rounded-lg bg-slate-900 p-4 text-sm text-slate-300 ring-1 ring-white/10", "Loading the viewer's models from the backend..." } },
        Some(Err(message)) => rsx! { p { id: "page-failed", role: "alert", class: "rounded-lg border border-red-400/40 bg-red-400/10 p-5 text-red-100", "{message}" } },
        Some(Ok(blob)) => rsx! { PageCanvas { context: context.clone(), blob: blob.clone() } },
    }
}

/// The page's own background and text, with the frameless transparent canvas over them.
#[component]
fn PageCanvas(context: GpuContext, blob: Rc<Vec<u8>>) -> Element {
    let graphics = use_hook(|| GraphicsContext::wrap_gpu_context(&context));
    let errors = use_signal(Vec::<String>::new);
    let frames = use_signal(|| 0u64);
    let mut hidden = use_signal(|| false);
    let mut paused = use_signal(|| false);
    let mut dusk = use_signal(|| false);
    let clock = use_hook(CanvasClock::new);
    let layers = use_hook(move || {
        let mut camera = Camera::new();
        camera.position = Vec3::new(0.0, 1.6, 7.5);
        camera.target = Vec3::new(0.0, 0.0, 0.0);
        camera.field_of_view = 40f32.to_radians();
        camera.near_plane = 0.1;
        camera.far_plane = 100.0;
        CanvasLayers::single(RendererHandle::new(PageModelsLayer { graphics: graphics.clone(), blob, gpu: None, hdr: None, camera, errors, frames }))
    });
    let toggle_clock = {
        let clock = clock.clone();
        move |_| paused.set(clock.toggle())
    };
    let button = "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700";
    let canvas_class = if hidden() { "invisible absolute inset-0 block h-full w-full touch-none select-none" } else { "absolute inset-0 block h-full w-full touch-none select-none" };
    rsx! {
        div { id: "page-surface", class: if dusk() { "relative aspect-video w-full overflow-hidden bg-linear-to-br from-indigo-950 via-purple-800 to-teal-700" } else { "relative aspect-video w-full overflow-hidden bg-linear-to-br from-sky-300 via-rose-200 to-amber-200" },
            div { class: "absolute inset-0 grid grid-cols-6 grid-rows-4",
                for i in 0..24 {
                    div { key: "{i}", class: if (i / 6 + i) % 2 == 0 { "bg-white/25" } else { "" } }
                }
            }
            div { class: "absolute inset-x-10 top-8 text-slate-800",
                h2 { class: "text-3xl font-bold", "Models on the page" }
                p { class: "mt-2 max-w-xl text-base", "This background and this text are the page's own HTML and CSS. The canvas above them has no frame and a transparent background: the models are drawn with HDR on, and everywhere else the page shows through." }
            }
            p { class: "absolute bottom-6 right-10 text-sm text-slate-700", "Ordinary page text, under the canvas." }
            WebGpuCanvas {
                context: context.clone(),
                layers,
                clock: clock.clone(),
                transparent: true,
                id: "page-canvas",
                label: "A few of the viewer's models turning, drawn on a transparent canvas over the page.",
                class: canvas_class,
            }
        }
        div { class: "flex flex-wrap items-center gap-3",
            button { id: "page-clock-toggle", class: "{button}", onclick: toggle_clock, if paused() { "Resume the clock" } else { "Pause the clock" } }
            button { id: "page-canvas-toggle", class: "{button}", onclick: move |_| hidden.set(!hidden()), if hidden() { "Show the canvas" } else { "Hide the canvas" } }
            button { id: "page-background-toggle", class: "{button}", onclick: move |_| dusk.set(!dusk()), "Change the page's background" }
        }
        div { class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
            p { class: "text-slate-300", "Frame " span { id: "page-frame", class: "font-mono text-white", "{frames}" } }
            p { class: "text-slate-300", "Errors while drawing: " span { id: "page-error-count", class: "font-mono text-white", "{errors.read().len()}" } }
            for error in errors.read().iter() {
                p { class: "font-mono text-red-300", "{error}" }
            }
            p { class: "mt-2 text-slate-400", "DRAWING ON THE PAGE: WebGpuCanvas with transparent: true and no frame in the page's styling; the HDR target cleared to transparent; no skybox; ToneMapper::preserve_alpha on." }
        }
    }
}
