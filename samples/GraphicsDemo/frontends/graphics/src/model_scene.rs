//! The two validation scenes of the graphics crate's skinning and morph targets, each on its own
//! button (none of the desktop viewer's models has a skin or a morph target):
//!
//! - "Skinned model": the Kenney archer (Mini Forest, CC0), a skinned .glb file the backend serves
//!   unchanged and the graphics crate's glTF loader reads in the browser (model_files.rs), playing
//!   its "walk" clip; a list chooses another of its clips (the player cross-fades to it). The
//!   ModelRenderer skins it on the GPU.
//! - "Morph targets": a sphere the BACKEND BUILDS IN CODE with three morph targets (stretch,
//!   twist, bumps) and a clip that moves their weights in turn. The ModelRenderer morphs it on the
//!   GPU.
//!
//! The model's bytes come from the backend through the generated client - the archer's file and
//! its colour map with `model_file_bytes` / `model_folder_file_bytes`, the morph shape's blob with
//! `model_data_bytes` - once for the life of the page (page_cache.rs); the
//! page shows a loading line, then the canvas with the layer (model_layer.rs) on its own clock,
//! the clock's controls (pause, a jump back or forward, half speed) and read-outs of the pose.

use codebrix_webgpu::{wgpu, CanvasClock, CanvasLayers, GpuContext, RendererHandle, WebGpuCanvas};
use codebrix_webgpu_graphics::GraphicsContext;
use dioxus::prelude::*;

use crate::generated::api::models;
use crate::model_files::{self, ModelFileRequest};
use crate::model_layer::{ModelLayer, ModelSource, ModelView, Probe};
use crate::page_cache;

/// Which validation scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelSceneKind {
    /// The skinned Kenney archer.
    Skinned,
    /// The morph-target shape built by the backend.
    Morph,
}

impl ModelSceneKind {
    /// The model's name on the backend.
    fn model(self) -> &'static str {
        match self {
            ModelSceneKind::Skinned => "character-archer",
            ModelSceneKind::Morph => "morph-shape",
        }
    }

    /// Fetches the model: the archer's .glb file (and the colour map it names) for the glTF
    /// loader, the morph shape's blob (it is built in code on the backend and has no file).
    async fn load(self) -> Result<ModelSource, String> {
        let name = self.model();
        match self {
            ModelSceneKind::Skinned => {
                let request = ModelFileRequest { name: name.to_string(), file: "kenney-mini-forest/character-archer.glb".to_string(), pack: "kenney-mini-forest".to_string() };
                Ok(ModelSource::Files(model_files::fetch(&[request]).await?))
            }
            ModelSceneKind::Morph => {
                let bytes = page_cache::bytes(&format!("model:{name}"), async { models::model_data_bytes(name).await.map_err(|e| format!("The model '{name}' could not be read from the backend: {}", e.message())) }).await?;
                Ok(ModelSource::Blob(bytes))
            }
        }
    }

    /// The clip played first.
    fn clip(self) -> &'static str {
        match self {
            ModelSceneKind::Skinned => "walk",
            ModelSceneKind::Morph => "morph",
        }
    }

    /// What the page reads out of the pose.
    fn probe(self) -> Probe {
        match self {
            ModelSceneKind::Skinned => Probe::Node("arm-left"),
            ModelSceneKind::Morph => Probe::MorphWeights,
        }
    }

    /// The prefix of the page's element ids.
    fn id(self) -> &'static str {
        match self {
            ModelSceneKind::Skinned => "skinned",
            ModelSceneKind::Morph => "morph",
        }
    }

    /// The canvas's accessible label.
    fn label(self) -> &'static str {
        match self {
            ModelSceneKind::Skinned => "A small archer character walking on the spot under a blue sky. Drag to orbit, drag with the right or middle button or with Shift to pan, use the wheel to zoom.",
            ModelSceneKind::Morph => "A coral-coloured ball that in turn stretches tall, twists, and grows bumps, under a blue sky. Drag to orbit, drag with the right or middle button or with Shift to pan, use the wheel to zoom.",
        }
    }

    /// The text under the read-outs.
    fn description(self) -> &'static str {
        match self {
            ModelSceneKind::Skinned => "The Kenney archer (Mini Forest, CC0): its .glb file from the backend, read in the browser by the graphics crate's glTF loader. A SKINNED model - its mesh follows a skeleton of joints, posed every frame by the animation player and skinned on the GPU from the instance's joint palette. Choose another of its clips in the list: the player cross-fades to it.",
            ModelSceneKind::Morph => "A sphere the backend builds in code (no asset file) with three MORPH TARGETS - stretch, twist and bumps - and a clip that raises each weight to 1 and back in turn; the GPU blends the targets' position and normal deltas by the weights.",
        }
    }
}

/// The loading line, the failure message, or the canvas.
#[component]
pub fn ModelScene(context: GpuContext, kind: ModelSceneKind) -> Element {
    // Fetched once for the life of the page (page_cache.rs): coming back asks for nothing.
    let loaded = use_resource(move || async move { kind.load().await });
    let state = loaded.read();
    let id = kind.id();
    match &*state {
        None => rsx! { p { id: "{id}-loading", role: "status", class: "rounded-lg bg-slate-900 p-4 text-sm text-slate-300 ring-1 ring-white/10", "Loading the model from the backend..." } },
        Some(Err(message)) => rsx! {
            div { id: "{id}-failed", role: "alert", class: "rounded-lg border border-red-400/40 bg-red-400/10 p-5 text-red-100",
                p { class: "text-lg font-semibold text-red-200", "The model could not be loaded" }
                p { class: "mt-2 text-sm", "{message}" }
                p { class: "mt-2 text-sm", "Check that the backend is running, then switch to another scene and back to try again." }
            }
        },
        Some(Ok(source)) => rsx! { ModelCanvas { key: "{id}", context: context.clone(), kind, source: source.clone() } },
    }
}

/// The canvas with the model layer, its clock, the controls and the read-outs.
#[component]
fn ModelCanvas(context: GpuContext, kind: ModelSceneKind, source: ModelSource) -> Element {
    let graphics = use_hook(|| GraphicsContext::wrap_gpu_context(&context));
    let view = use_signal(ModelView::default);
    let errors = use_signal(Vec::<String>::new);
    let mut clip = use_signal(|| kind.clip().to_string());
    let mut paused = use_signal(|| false);
    let mut speed = use_signal(|| 1.0f64);
    let clock = use_hook(CanvasClock::new);
    let byte_count = source.download_bytes();
    let layers = use_hook(move || CanvasLayers::single(RendererHandle::new(ModelLayer::new(graphics.clone(), source, clip, kind.probe(), view, errors))));
    let toggle_clock = {
        let clock = clock.clone();
        move |_| paused.set(clock.toggle())
    };
    let seek_back = {
        let clock = clock.clone();
        move |_| clock.seek((clock.elapsed() - 1.0).max(0.0))
    };
    let seek_forward = {
        let clock = clock.clone();
        move |_| clock.seek(clock.elapsed() + 1.0)
    };
    let toggle_speed = {
        let clock = clock.clone();
        move |_| {
            let next = if speed() == 1.0 { 0.5 } else { 1.0 };
            clock.set_speed(next);
            speed.set(next);
        }
    };
    let id = kind.id();
    let button = "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700";
    let field = "rounded-md bg-slate-800 px-2 py-1 text-sm text-slate-100 ring-1 ring-white/10";
    let shown = view();
    let state = if paused() { "paused" } else { "running" };
    rsx! {
        WebGpuCanvas {
            context: context.clone(),
            layers,
            clock: clock.clone(),
            clear_color: wgpu::Color { r: 0.05, g: 0.07, b: 0.1, a: 1.0 },
            id: "{id}-canvas",
            label: kind.label(),
            class: "aspect-video w-full touch-none select-none rounded-lg ring-1 ring-white/10 focus-visible:outline-2 focus-visible:outline-sky-400",
        }
        div { class: "flex flex-wrap items-center gap-3",
            button { id: "{id}-clock-toggle", class: "{button}", onclick: toggle_clock, if paused() { "Resume the clock" } else { "Pause the clock" } }
            button { id: "{id}-seek-back", class: "{button}", onclick: seek_back, "Clock -1 s" }
            button { id: "{id}-seek-forward", class: "{button}", onclick: seek_forward, "Clock +1 s" }
            button { id: "{id}-speed", class: "{button}", onclick: toggle_speed, if speed() == 1.0 { "Half speed" } else { "Normal speed" } }
            if shown.clips.len() > 1 {
                label { r#for: "{id}-clip-select", class: "text-sm text-slate-300", "Clip" }
                select {
                    id: "{id}-clip-select",
                    class: "{field}",
                    onchange: move |event: Event<FormData>| clip.set(event.value()),
                    for name in shown.clips.iter() {
                        option { value: "{name}", selected: *name == clip(), "{name}" }
                    }
                }
            }
        }
        div { class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
            p { class: "text-slate-300",
                "Frame " span { id: "{id}-frame", class: "font-mono text-white", "{shown.frame}" }
                ", clock " span { id: "{id}-clock-time", class: "font-mono text-white", "{shown.time:.3}" } " s, "
                span { id: "{id}-clock-state", "{state}" }
            }
            p { class: "text-slate-300",
                "Clip " span { id: "{id}-clip", class: "font-mono text-white", "{shown.clip}" }
                " at " span { id: "{id}-clip-time", class: "font-mono text-white", "{shown.clip_time:.3}" } " s ("
                span { id: "{id}-states", class: "font-mono text-white", "{shown.states}" } " playing); "
                span { id: "{id}-probe", class: "font-mono text-white", "{shown.probe}" }
            }
            p { class: "text-slate-300",
                "Downloaded: " span { id: "{id}-bytes", class: "font-mono text-white", "{byte_count}" } " bytes; "
                span { id: "{id}-skins", class: "font-mono text-white", "{shown.skins}" } " skin(s) with "
                span { id: "{id}-joints", class: "font-mono text-white", "{shown.joints}" } " joints, "
                span { id: "{id}-morph-targets", class: "font-mono text-white", "{shown.morph_targets}" } " morph target(s), "
                span { id: "{id}-triangles", class: "font-mono text-white", "{shown.triangles}" } " triangles; last frame "
                span { id: "{id}-draw-calls", class: "font-mono text-white", "{shown.draw_calls}" } " draw call(s), "
                span { id: "{id}-pipelines", class: "font-mono text-white", "{shown.pipelines}" } " pipeline(s)"
            }
            p { class: "text-slate-300", "Errors while drawing: " span { id: "{id}-error-count", class: "font-mono text-white", "{errors.read().len()}" } }
            for error in errors.read().iter() {
                p { class: "font-mono text-red-300", "{error}" }
            }
            p { class: "mt-2 text-slate-400", "{kind.description()} Drag to orbit, right or middle drag (or Shift + drag) to pan, wheel to zoom; it turns by itself until you drag. Pausing the clock holds the pose; a jump of the clock moves the clip to the matching place." }
        }
    }
}
