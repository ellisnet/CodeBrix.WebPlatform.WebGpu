//! The graphics application of the GraphicsDemo sample, served below /graphics. It asks the
//! browser for a WebGPU adapter and device (WebGPU only: no WebGL fallback) through the
//! codebrix-webgpu crate - with `timestamp-query` where the browser grants it, for the GPU timer
//! of the "More" scene - then shows one of ten SCENES, chosen with the buttons at the top:
//!
//! - "Viewer" (the first, shown when the page opens): the desktop model viewer's complete scene -
//!   its ground, static props and animated actors (the chest, the animals and the Avian, each
//!   instance played by the graphics crate's AnimationPlayer on the page's clock), glTF models the
//!   BACKEND converted into one model-data blob, fetched
//!   through the generated client's `_bytes` call and drawn by the ModelRenderer with physically
//!   based shading under the viewer's orbit camera, lit by image-based light from its procedural
//!   sky (or from an HDR panorama the backend serves), with the skybox, rendered through an HDR
//!   target and tone mapped; `?ldr` and `?no-skybox` are the desktop viewer's switches
//!   (viewer_scene.rs, viewer.rs).
//! - "Layers": one canvas with TWO LAYERS on ONE clock: a triangle (triangle.rs), and
//!   over it a translucent (or, with a button, solid) square (overlay.rs) that knows nothing of
//!   the triangle. The page puts the layers together, owns the clock (pause and resume, a button)
//!   and defines the clock's named point "title in", at which the triangle turns lighter and the
//!   square appears. It shows what wgpu says about the adapter, a frame counter and the clock.
//! - "Shaders": every built-in WGSL shader of codebrix-webgpu-graphics compiled once by this
//!   browser, with the compiler's messages (shader_scene.rs).
//! - "Textured quad": a textured, rotating quad drawn THROUGH codebrix-webgpu-graphics (its
//!   context, buffers, texture, sampler, bind groups and pipeline builder), as a layer of the
//!   canvas host on the host's clock (quad_scene.rs, quad.rs).
//! - "Sprites": the graphics crate's SpriteBatch - an atlas made in code, tinted, rotated,
//!   scaled and ordered sprites, the transform stack and the shapes - as a layer on the host's
//!   clock, after a check of the device's one shared error handler (sprite_scene.rs, sprites.rs).
//! - "Lit shapes": the built-in mesh primitives lit by the MeshRenderer on a ground plane, with
//!   depth, under a perspective or orthographic camera driven by the OrbitController (drag,
//!   wheel, one-finger orbit and pinch zoom, auto-orbit until the first drag), with a legend of
//!   the controls in HTML over the canvas (lit_scene.rs, lit_shapes.rs).
//! - "Skinned model" and "Morph targets": the two validation scenes of skinning and morph targets
//!   (none of the viewer's models has either) - the Kenney archer playing its clips, and a shape
//!   the backend builds in code with three morph targets and a weights clip (model_scene.rs,
//!   model_layer.rs).
//! - "More": the graphics crate's remaining features, each visible: an HDR panorama made in code
//!   as the background, a ring of cubes from a render bundle, particles moved by a compute pass
//!   and drawn by an indirect draw whose count the compute pass decides, two animated textures,
//!   and a line of occlusion and timestamp query results in HTML (more_scene.rs, more_layer.rs).
//! - "On the page" (also `?scene=on-the-page`): DRAWING ON THE PAGE - a frameless, transparent
//!   canvas over the page's own gradient and text, with a few of the viewer's models turning on
//!   it through the HDR path (tone mapped with `preserve_alpha`), no skybox (page_scene.rs).
//!
//! What a scene fetched from the backend is kept for the life of the page (page_cache.rs):
//! returning to a scene asks the backend for nothing.
//!
//! Without WebGPU it shows the crate's "WebGPU is not available" message instead.

mod generated;
mod lit_scene;
mod lit_shapes;
mod model_layer;
mod model_scene;
mod more_layer;
mod more_scene;
mod page_scene;
mod overlay;
mod page_cache;
mod quad;
mod quad_scene;
mod shader_scene;
mod sprite_scene;
mod sprites;
mod triangle;
mod viewer;
mod viewer_actors;
mod viewer_scene;

use codebrix_webgpu::{wgpu, CanvasClock, CanvasLayers, GpuContext, GpuUnavailable, RendererHandle, WebGpuCanvas, WebGpuUnavailable};
use codebrix_webgpu_graphics::{GraphicsContext, GraphicsContextOptions};
use dioxus::prelude::*;

use crate::lit_scene::LitScene;
use crate::model_scene::{ModelScene, ModelSceneKind};
use crate::more_scene::MoreScene;
use crate::page_scene::PageScene;
use crate::overlay::OverlayRenderer;
use crate::quad_scene::QuadScene;
use crate::shader_scene::ShaderScene;
use crate::sprite_scene::SpriteScene;
use crate::triangle::{TriangleRenderer, View};
use crate::viewer_scene::ViewerScene;

/// The scenes of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SceneKind {
    /// The model viewer's scene: models from the backend, physically based shading, image-based
    /// lighting, the skybox, HDR and tone mapping (the default).
    Viewer,
    /// Two layers on one clock (the layering proof).
    Layers,
    /// Every built-in shader compiled.
    Shaders,
    /// The textured, rotating quad through the graphics crate.
    Quad,
    /// The sprite batch.
    Sprites,
    /// The lit-mesh renderer under the orbit camera.
    LitShapes,
    /// The skinned Kenney archer (a validation scene).
    Skinned,
    /// The morph-target shape the backend builds (a validation scene).
    Morph,
    /// Animated textures, a panorama, a render bundle, compute, an indirect draw and queries.
    More,
    /// Models on a transparent canvas over the page (drawing "on the page").
    OnThePage,
}

/// The clock's named point at which the overlay appears and the triangle turns lighter.
pub const TITLE_IN: &str = "title in";
/// Where on the clock's timeline "title in" is, in seconds.
const TITLE_IN_SECONDS: f64 = 2.0;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
/// The site icon (the CodeBrix icon as an .ico file). Served without a hash suffix, at the fixed
/// path `index.html` also links, so the browser finds it before the application has started.
const FAVICON: Asset = asset!("/assets/favicon.ico", AssetOptions::builder().with_hash_suffix(false));

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // The platform's REST and SignalR clients, set up for this backend, on the first render.
    use_hook(generated::configure_platform);
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        document::Link { rel: "icon", r#type: "image/x-icon", href: FAVICON }
        main { class: "mx-auto flex min-h-dvh max-w-5xl flex-col gap-6 px-4 py-8 sm:px-6",
            header { class: "flex flex-wrap items-baseline justify-between gap-3",
                h1 { id: "page-title", class: "text-2xl font-semibold text-white", "Graphics" }
                a { href: "/", class: "text-sm text-sky-300 hover:text-sky-200", "All applications" }
            }
            GpuPage {}
        }
    }
}

/// Asks the browser for an adapter and a device as `GpuContext::request` does (the browser's
/// default adapter, WebGPU's default limits), adding the optional feature `timestamp-query`
/// WHERE THE ADAPTER HAS IT - the GPU timer of the "More" scene measures with it, and goes without
/// where it is missing.
async fn request_gpu() -> Result<GpuContext, GpuUnavailable> {
    let options = GraphicsContextOptions {
        power_preference: wgpu::PowerPreference::None,
        label: Some("codebrix-webgpu device".to_string()),
        optional_features: wgpu::Features::TIMESTAMP_QUERY,
        ..GraphicsContextOptions::default()
    };
    GraphicsContext::create(options).await.map(|graphics| graphics.gpu_context().clone())
}

/// Asks for the GPU once, then shows the scene or the "not available" message.
#[component]
fn GpuPage() -> Element {
    let gpu = use_resource(request_gpu);
    let state = gpu.read();
    match &*state {
        None => rsx! { p { id: "webgpu-pending", class: "text-sm text-slate-400", "Asking the browser for a WebGPU adapter..." } },
        Some(Err(reason)) => rsx! {
            WebGpuUnavailable {
                reason: reason.clone(),
                class: "rounded-lg border border-amber-400/40 bg-amber-400/10 p-5 text-amber-100",
                heading_class: "text-lg font-semibold text-amber-200",
                text_class: "mt-2 text-sm",
            }
        },
        Some(Ok(context)) => rsx! { Scenes { context: context.clone() } },
    }
}

/// The scene switch and the scene it shows. Each scene owns its canvas and clock; a scene that is
/// switched away from stops drawing.
#[component]
fn Scenes(context: GpuContext) -> Element {
    // `?scene=on-the-page` opens the page scene directly.
    let mut scene = use_signal(|| {
        let search = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
        if search.trim_start_matches('?').split('&').any(|pair| pair == "scene=on-the-page") {
            SceneKind::OnThePage
        } else {
            SceneKind::Viewer
        }
    });
    let tab = |kind: SceneKind| {
        if scene() == kind {
            "rounded-md bg-sky-700 px-3 py-1.5 text-sm text-white ring-1 ring-sky-300/40"
        } else {
            "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700"
        }
    };
    rsx! {
        nav { class: "flex flex-wrap gap-2", "aria-label": "Scenes",
            button { id: "scene-viewer", class: tab(SceneKind::Viewer), onclick: move |_| scene.set(SceneKind::Viewer), "Viewer" }
            button { id: "scene-layers", class: tab(SceneKind::Layers), onclick: move |_| scene.set(SceneKind::Layers), "Layers" }
            button { id: "scene-shaders", class: tab(SceneKind::Shaders), onclick: move |_| scene.set(SceneKind::Shaders), "Shaders" }
            button { id: "scene-quad", class: tab(SceneKind::Quad), onclick: move |_| scene.set(SceneKind::Quad), "Textured quad" }
            button { id: "scene-sprites", class: tab(SceneKind::Sprites), onclick: move |_| scene.set(SceneKind::Sprites), "Sprites" }
            button { id: "scene-lit", class: tab(SceneKind::LitShapes), onclick: move |_| scene.set(SceneKind::LitShapes), "Lit shapes" }
            button { id: "scene-skinned", class: tab(SceneKind::Skinned), onclick: move |_| scene.set(SceneKind::Skinned), "Skinned model" }
            button { id: "scene-morph", class: tab(SceneKind::Morph), onclick: move |_| scene.set(SceneKind::Morph), "Morph targets" }
            button { id: "scene-more", class: tab(SceneKind::More), onclick: move |_| scene.set(SceneKind::More), "More" }
            button { id: "scene-on-the-page", class: tab(SceneKind::OnThePage), onclick: move |_| scene.set(SceneKind::OnThePage), "On the page" }
        }
        match scene() {
            SceneKind::Viewer => rsx! { ViewerScene { context: context.clone() } },
            SceneKind::Layers => rsx! { Scene { context: context.clone() } },
            SceneKind::Shaders => rsx! { ShaderScene { context: context.clone() } },
            SceneKind::Quad => rsx! { QuadScene { context: context.clone() } },
            SceneKind::Sprites => rsx! { SpriteScene { context: context.clone() } },
            SceneKind::LitShapes => rsx! { LitScene { context: context.clone() } },
            SceneKind::Skinned => rsx! { ModelScene { key: "{scene():?}", context: context.clone(), kind: ModelSceneKind::Skinned } },
            SceneKind::Morph => rsx! { ModelScene { key: "{scene():?}", context: context.clone(), kind: ModelSceneKind::Morph } },
            SceneKind::More => rsx! { MoreScene { context: context.clone() } },
            SceneKind::OnThePage => rsx! { PageScene { context: context.clone() } },
        }
    }
}

/// The canvas (the triangle and the overlay, on one clock) with the adapter information, the
/// frame counter, the clock and its controls, and the view.
#[component]
fn Scene(context: GpuContext) -> Element {
    let frames = use_signal(|| 0u64);
    let view = use_signal(View::default);
    let lit_at = use_signal(|| None::<u64>);
    let shown_at = use_signal(|| None::<u64>);
    let mut paused = use_signal(|| false);
    let mut overlay_on = use_signal(|| true);
    let mut solid = use_signal(|| false);
    let clock = use_hook(|| {
        let clock = CanvasClock::new();
        clock.define_point(TITLE_IN, TITLE_IN_SECONDS);
        clock
    });
    let overlay = use_hook(|| RendererHandle::new(OverlayRenderer::new(shown_at, solid)));
    let layers = use_hook(|| {
        // Bottom first: the triangle, then the overlay over it.
        let triangle = RendererHandle::new(TriangleRenderer::new(frames, view, lit_at));
        CanvasLayers::new(vec![triangle, overlay.clone()])
    });
    let toggle_clock = {
        let clock = clock.clone();
        move |_| paused.set(clock.toggle())
    };
    let toggle_overlay = {
        let layers = layers.clone();
        move |_| {
            if !layers.remove(&overlay) {
                layers.push(overlay.clone());
            }
            overlay_on.set(layers.contains(&overlay));
        }
    };
    let info = context.adapter_info();
    let name = if info.name.is_empty() { "(the browser gives no description)".to_string() } else { info.name.clone() };
    let button = "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700";
    rsx! {
        WebGpuCanvas {
            context: context.clone(),
            layers,
            clock: clock.clone(),
            clear_color: codebrix_webgpu::wgpu::Color { r: 0.003, g: 0.004, b: 0.01, a: 1.0 },
            id: "webgpu-canvas",
            label: "A triangle drawn with WebGPU, with a translucent or solid square over it. Drag to turn the triangle, use the wheel to scale it.",
            class: "aspect-video w-full touch-none select-none rounded-lg ring-1 ring-white/10 focus-visible:outline-2 focus-visible:outline-sky-400",
        }
        div { class: "flex flex-wrap gap-3",
            button { id: "clock-toggle", class: "{button}", onclick: toggle_clock, if paused() { "Resume the clock" } else { "Pause the clock" } }
            button { id: "overlay-toggle", class: "{button}", onclick: toggle_overlay, if overlay_on() { "Remove the overlay layer" } else { "Add the overlay layer" } }
            button { id: "overlay-solid", class: "{button}", onclick: move |_| solid.set(!solid()), if solid() { "Make the square translucent" } else { "Make the square solid" } }
        }
        div { class: "grid gap-4 sm:grid-cols-2",
            dl { id: "adapter-info", class: "grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
                dt { class: "text-slate-400", "Adapter" }
                dd { id: "adapter-name", class: "text-slate-100", "{name}" }
                dt { class: "text-slate-400", "Type" }
                dd { id: "adapter-type", class: "text-slate-100", "{info.device_type:?}" }
                dt { class: "text-slate-400", "Backend" }
                dd { id: "adapter-backend", class: "text-slate-100", "{info.backend:?}" }
            }
            div { class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
                FrameCounter { frames }
                ClockReadout { frames, clock: clock.clone(), lit_at, shown_at }
                ViewReadout { view }
                p { class: "mt-2 text-slate-400", "Drag to turn the triangle; use the wheel to scale it. With the canvas focused: arrow keys, + and -, the space bar to start or stop the spin, r to reset. Pausing the clock stops both layers; resuming goes on from the same time." }
            }
        }
    }
}

/// The frame counter (its own component, so that only it renders again every frame).
#[component]
fn FrameCounter(frames: Signal<u64>) -> Element {
    rsx! {
        p { class: "text-slate-300", "Frames drawn: " span { id: "frame-count", class: "font-mono text-white", "{frames}" } }
    }
}

/// The clock's time and state, and the frame in which each layer first reacted to "title in"
/// (renders again with the frame counter).
#[component]
fn ClockReadout(frames: Signal<u64>, clock: CanvasClock, lit_at: Signal<Option<u64>>, shown_at: Signal<Option<u64>>) -> Element {
    let _ = frames();
    let state = if clock.is_paused() { "paused" } else { "running" };
    let frame_of = |at: Option<u64>| at.map(|n| n.to_string()).unwrap_or_else(|| "not yet".to_string());
    rsx! {
        p { class: "text-slate-300",
            "Clock " span { id: "clock-time", class: "font-mono text-white", "{clock.elapsed():.3}" } " s, "
            span { id: "clock-state", "{state}" }
        }
        p { class: "text-slate-300",
            "\"{TITLE_IN}\" at {TITLE_IN_SECONDS} s: triangle lit in frame "
            span { id: "triangle-point-frame", class: "font-mono text-white", "{frame_of(lit_at())}" }
            ", overlay shown in frame "
            span { id: "overlay-point-frame", class: "font-mono text-white", "{frame_of(shown_at())}" }
        }
    }
}

/// The triangle's rotation, scale and spin.
#[component]
fn ViewReadout(view: Signal<View>) -> Element {
    let view = view();
    let spin = if view.spinning { "spinning" } else { "still" };
    rsx! {
        p { class: "text-slate-300",
            "Rotation " span { id: "view-rotation", class: "font-mono text-white", "{view.degrees:.0}" } " degrees, scale "
            span { id: "view-scale", class: "font-mono text-white", "{view.scale:.2}" } ", "
            span { id: "view-spin", "{spin}" }
        }
    }
}
