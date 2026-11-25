//! The graphics application of the GraphicsDemo sample, served below /graphics. It asks the
//! browser for a WebGPU adapter and device (WebGPU only: no WebGL fallback) through the
//! codebrix-webgpu crate - with `timestamp-query` where the browser grants it, for the GPU timer
//! of the "More" scene - then shows one of seventeen SCENES, chosen with the buttons at the top:
//!
//! - "Viewer" (the first, shown when the page opens): the desktop model viewer's complete scene -
//!   its ground, static props and animated actors (the chest, the animals and the Avian, each
//!   instance played by the graphics crate's AnimationPlayer on the page's clock), the kits' .glb
//!   files the backend serves unchanged, fetched through the generated client's `_bytes` calls and
//!   read in the browser by the graphics crate's glTF loader (model_files.rs), the ground from the
//!   blob of a model the backend builds in code, drawn by the ModelRenderer with physically
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
//!   (none of the viewer's models has either) - the Kenney archer (its .glb file, through the glTF
//!   loader) playing its clips, and a shape the backend builds in code with three morph targets and
//!   a weights clip (its blob) (model_scene.rs, model_layer.rs).
//! - "More": the graphics crate's remaining features, each visible: an HDR panorama made in code
//!   as the background, a ring of cubes from a render bundle, particles moved by a compute pass
//!   and drawn by an indirect draw whose count the compute pass decides, two animated textures,
//!   and a line of occlusion and timestamp query results in HTML (more_scene.rs, more_layer.rs).
//! - "On the page" (also `?scene=on-the-page`): DRAWING ON THE PAGE - a frameless, transparent
//!   canvas over the page's own gradient and text, with a few of the viewer's models turning on
//!   it through the HDR path (tone mapped with `preserve_alpha`), no skybox (page_scene.rs).
//!
//! - The FEATURE DEMOS (demos/, also `?scene=<name>`): "Lights" (`lights`), "Shadows"
//!   (`shadows`), "Culling" (`culling`), "Translucency" (`translucency`), "Glow and bloom"
//!   (`glow`), "Particles" (`particles`), "Picking and labels" (`picking`), "Anti-aliasing"
//!   (`antialiasing`), "Level of detail" (`lod`), "Water pond" (`water-pond`) and "Water lake"
//!   (`water-lake`) - the graphics crate's local lights, shadows,
//!   frustum culling, two-pass translucency and per-instance look, glow layer, bloom, particle
//!   system, picking and world-to-canvas labels, multisampling (1 or 4 samples, switched while
//!   the page runs, beside a second canvas held at 1; the HDR path and the direct path over a
//!   layer below), level of detail with a fade band, and water (a simulated wave field over the
//!   owner's mosaic rosette pool floor, stirred by the pointer and rain; open-water waves on a lake
//!   with a floating barrel and a wake), each with its controls, read-outs and the frame timer's
//!   timings.
//!
//! The button of the scene on show is filled and marked `aria-current="true"` (one at a time).
//!
//! What a scene fetched from the backend is kept for the life of the page (page_cache.rs):
//! returning to a scene asks the backend for nothing.
//!
//! The scenes are inside a WebGpuBoundary: without WebGPU the page shows its OWN fallback panel
//! instead (what the scenes would show), not the crate's "WebGPU is not available" message.

mod demos;
mod generated;
mod lit_scene;
mod lit_shapes;
mod model_files;
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

use codebrix_webgpu::{set_page_gpu_options, use_gpu, wgpu, CanvasClock, CanvasLayers, GpuContext, PageGpuOptions, RendererHandle, WebGpuBoundary, WebGpuCanvas};
use dioxus::prelude::*;

use crate::demos::{DemoKind, DemoScene};
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
    /// One of the feature demos of the graphics crate's newer capabilities (demos/).
    Demo(DemoKind),
}

/// The clock's named point at which the overlay appears and the triangle turns lighter.
pub const TITLE_IN: &str = "title in";
/// Where on the clock's timeline "title in" is, in seconds.
const TITLE_IN_SECONDS: f64 = 2.0;

/// The Tailwind stylesheet the platform's frontend build generates from tailwind.css (build output,
/// at the platform's fixed path assets/generated/tailwind.css).
const TAILWIND_CSS: Asset = asset!("/assets/generated/tailwind.css");
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
    // How the page's one WebGPU request asks: as the default does, plus the optional feature
    // `timestamp-query` WHERE THE ADAPTER HAS IT - the GPU timer of the "More" scene measures with
    // it, and goes without where it is missing. Set before the boundary below first asks.
    use_hook(|| set_page_gpu_options(PageGpuOptions { optional_features: wgpu::Features::TIMESTAMP_QUERY, ..PageGpuOptions::default() }));
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        document::Link { rel: "icon", r#type: "image/x-icon", href: FAVICON }
        main { class: "mx-auto flex min-h-dvh max-w-5xl flex-col gap-6 px-4 py-8 sm:px-6",
            header { class: "flex flex-wrap items-baseline justify-between gap-3",
                h1 { id: "page-title", class: "text-2xl font-semibold text-white", "Graphics" }
                a { href: "/", class: "text-sm text-sky-300 hover:text-sky-200", "All applications" }
            }
            // The scenes where WebGPU works; this page's own panel where it does not.
            WebGpuBoundary {
                fallback: |_| rsx! { NoWebGpu {} },
                Scenes {}
            }
        }
    }
}

/// What the page shows where the browser has no WebGPU: its OWN panel (the boundary's fallback),
/// not the crate's "WebGPU is not available" message - what the scenes would show, and the way
/// back. (The animation application shows the crate's message instead, and its "on the page" page
/// shows nothing: the sample has all three.)
#[component]
fn NoWebGpu() -> Element {
    rsx! {
        section { id: "graphics-fallback", class: "rounded-lg bg-slate-900 p-6 ring-1 ring-white/10",
            h2 { class: "text-lg font-semibold text-white", "These 3D scenes are drawn with WebGPU" }
            p { class: "mt-2 text-sm text-slate-300", "This browser does not offer WebGPU, so the scenes are not shown here. In a browser that does, this page shows:" }
            ul { class: "mt-4 grid gap-x-6 gap-y-2 text-sm text-slate-300 sm:grid-cols-2",
                for (name, what) in SCENE_SUMMARIES {
                    li { key: "{name}",
                        span { class: "font-medium text-sky-200", "{name}" }
                        " - {what}"
                    }
                }
                for kind in DemoKind::ALL {
                    li { key: "{kind.slug()}",
                        span { class: "font-medium text-sky-200", "{kind.title()}" }
                        " - {kind.summary()}"
                    }
                }
            }
            p { class: "mt-5 text-sm",
                a { href: "/", class: "text-sky-300 hover:text-sky-200", "Back to all applications" }
            }
        }
    }
}

/// The scenes, in a sentence each, for the page without WebGPU.
const SCENE_SUMMARIES: [(&str, &str); 10] = [
    ("Viewer", "the model viewer: models with physically based shading, image-based light, a skybox, HDR and tone mapping"),
    ("Layers", "two layers on one clock: a triangle, and a translucent square over it"),
    ("Shaders", "every built-in shader of the graphics crate, compiled by the browser"),
    ("Textured quad", "a textured, rotating quad"),
    ("Sprites", "a sprite batch: tinted, rotated, scaled and ordered sprites"),
    ("Lit shapes", "lit shapes on a ground plane, under a camera you can drag around"),
    ("Skinned model", "an archer playing its animation clips"),
    ("Morph targets", "a shape blending between three morph targets"),
    ("More", "a panorama, a render bundle, particles moved by compute, animated textures and a GPU timer"),
    ("On the page", "models turning on a transparent canvas over the page's own background"),
];

/// The scene switch and the scene it shows. Each scene owns its canvas and clock; a scene that is
/// switched away from stops drawing.
#[component]
fn Scenes() -> Element {
    // The boundary's context: the page's one device.
    let context = use_gpu();
    // `?scene=on-the-page` opens the page scene directly.
    let mut scene = use_signal(|| {
        let search = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
        if search.trim_start_matches('?').split('&').any(|pair| pair == "scene=on-the-page") {
            SceneKind::OnThePage
        } else if let Some(kind) = DemoKind::from_query() {
            SceneKind::Demo(kind)
        } else {
            SceneKind::Viewer
        }
    });
    // THE CURRENT SCENE'S BUTTON is filled (a light fill with dark text and a white ring), every
    // other button stays dark: driven by the same `scene` signal that chooses what is drawn, so the
    // two cannot disagree; it also carries aria-current="true".
    let tab = |kind: SceneKind| {
        if scene() == kind {
            "rounded-md bg-sky-300 px-3 py-1.5 text-sm font-semibold text-slate-950 ring-2 ring-white"
        } else {
            "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700"
        }
    };
    let current = |kind: SceneKind| if scene() == kind { "true" } else { "false" };
    // When the scene changes, the current button is scrolled into view inside the button rows
    // (never the page, never the keyboard focus) - for when the rows ever scroll.
    use_effect(move || {
        let id = scene_button_id(scene());
        keep_in_view("scene-buttons", &id);
    });
    let base_scenes = [
        (SceneKind::Viewer, "scene-viewer", "Viewer"),
        (SceneKind::Layers, "scene-layers", "Layers"),
        (SceneKind::Shaders, "scene-shaders", "Shaders"),
        (SceneKind::Quad, "scene-quad", "Textured quad"),
        (SceneKind::Sprites, "scene-sprites", "Sprites"),
        (SceneKind::LitShapes, "scene-lit", "Lit shapes"),
        (SceneKind::Skinned, "scene-skinned", "Skinned model"),
        (SceneKind::Morph, "scene-morph", "Morph targets"),
        (SceneKind::More, "scene-more", "More"),
        (SceneKind::OnThePage, "scene-on-the-page", "On the page"),
    ];
    rsx! {
        nav { id: "scene-buttons", class: "flex max-h-48 flex-col gap-2 overflow-y-auto", "aria-label": "Scenes",
            div { class: "flex flex-wrap gap-2",
                for (kind, id, text) in base_scenes {
                    button { key: "{id}", id: "{id}", class: tab(kind), "aria-current": current(kind), onclick: move |_| scene.set(kind), "{text}" }
                }
            }
            div { class: "flex flex-wrap items-center gap-2",
                span { class: "pr-1 text-xs font-semibold uppercase tracking-wide text-slate-400", "Feature demos" }
                for kind in DemoKind::ALL {
                    button { key: "{kind.slug()}", id: "scene-{kind.slug()}", class: tab(SceneKind::Demo(kind)), "aria-current": current(SceneKind::Demo(kind)), onclick: move |_| scene.set(SceneKind::Demo(kind)), "{kind.title()}" }
                }
            }
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
            SceneKind::Demo(kind) => rsx! { DemoScene { key: "{kind.slug()}", context: context.clone(), kind } },
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

/// The id of a scene's button.
fn scene_button_id(kind: SceneKind) -> String {
    match kind {
        SceneKind::Viewer => "scene-viewer".to_string(),
        SceneKind::Layers => "scene-layers".to_string(),
        SceneKind::Shaders => "scene-shaders".to_string(),
        SceneKind::Quad => "scene-quad".to_string(),
        SceneKind::Sprites => "scene-sprites".to_string(),
        SceneKind::LitShapes => "scene-lit".to_string(),
        SceneKind::Skinned => "scene-skinned".to_string(),
        SceneKind::Morph => "scene-morph".to_string(),
        SceneKind::More => "scene-more".to_string(),
        SceneKind::OnThePage => "scene-on-the-page".to_string(),
        SceneKind::Demo(kind) => format!("scene-{}", kind.slug()),
    }
}

/// Scrolls the element `container_id` (only it: never the page, and the keyboard focus stays
/// where it is) so that its child `child_id` is wholly in view; nothing when it already is.
fn keep_in_view(container_id: &str, child_id: &str) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else { return };
    let (Some(container), Some(child)) = (document.get_element_by_id(container_id), document.get_element_by_id(child_id)) else { return };
    let (outer, inner) = (container.get_bounding_client_rect(), child.get_bounding_client_rect());
    let top = inner.top() - outer.top() + f64::from(container.scroll_top());
    let bottom = top + inner.height();
    let height = f64::from(container.client_height());
    let scroll = f64::from(container.scroll_top());
    if top < scroll {
        container.set_scroll_top(top.floor() as i32);
    } else if bottom > scroll + height {
        container.set_scroll_top((bottom - height).ceil() as i32);
    }
}
