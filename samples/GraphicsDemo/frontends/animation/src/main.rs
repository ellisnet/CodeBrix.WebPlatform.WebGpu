//! The animation application of the GraphicsDemo sample, served below /animation: the scene
//! catalogue of the C# AnimationDemoApp and the sample's own SVG and Lottie scenes (demo/mod.rs
//! lists them), all drawn - text included - through codebrix-webgpu-animation as a layer of the
//! codebrix-webgpu canvas host on its clock; and, at ?scene=on-the-page, DRAWING ON THE PAGE (a
//! frameless, transparent canvas over the page's own background). WebGPU only: without it, the
//! demo shows the crate's "WebGPU is not available" message, and the page "on the page" shows its
//! own content without the animation (its WebGpuBoundary). EVERY RESOURCE COMES FROM THE BACKEND (this
//! sample's rule; the crate takes bytes from anywhere), fetched once with the generated `_bytes`
//! calls before the catalogue is made: the fonts (`font_file_bytes`, by file name: the backend
//! serves every font file it carries; this application asks for the two the demo draws with, in
//! FONT_FILES), registered with the crate's FontRegistry, and the variable Roboto (`roboto.ttf`)
//! registered at several weights (`register_font_with_variations`) for the "text" scene; the pictures - the Cube
//! Pets colour map decoded there and sent as pixels (`cube_pets_colormap_bytes`), the same picture
//! as its PNG file (`cube_pets_colormap_png_bytes`) and as a JPEG file made from it there
//! (`cube_pets_colormap_jpeg_bytes`, read with `ImageSource::from_jpeg`), and the Lottie image layer's badge made there
//! in code (`lottie_badge_bytes`); the caption file of the "captions" scene
//! (`caption_scene_srt_bytes`); the SVG documents of the SVG scenes (`..._svg_bytes`), their text
//! laid out in the same Roboto and Open Sans bytes; the Lottie files of the Lottie scenes
//! (`..._json_bytes`); and, for the "live-pickup" scene's 3D source, five model files of the kits
//! with the files they name (`model_file_bytes`, `model_folder_file_bytes`) and the ground's blob
//! (`model_data_bytes`), read by codebrix-webgpu-graphics' glTF loader - the one scene of this
//! application that draws through the graphics crate.

mod demo;
mod generated;

use codebrix_webgpu::{GpuContext, WebGpuUnavailable};
use codebrix_webgpu_animation::text::{FontRegistry, FontVariation};
use dioxus::prelude::*;

use crate::demo::on_page::OnThePage;
use crate::demo::page::DemoPage;
use crate::demo::captions::set_backend_captions;
use crate::demo::lottie_files::set_backend_lottie;
use crate::demo::lottie_images::set_backend_badge;
use crate::demo::pictures::{from_pixel_form, set_backend_jpeg_picture, set_backend_picture, set_backend_png_picture};
use crate::demo::svg_files::{fetch_svg_documents, set_svg_font};
use crate::generated::api::{captions, fonts, pictures};
use crate::generated::api::lottie;

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
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        document::Link { rel: "icon", r#type: "image/x-icon", href: FAVICON }
        main { class: "mx-auto flex min-h-dvh max-w-[1320px] flex-col gap-4 px-4 py-6",
            header { class: "flex flex-wrap items-baseline justify-between gap-3",
                h1 { id: "page-title", class: "text-2xl font-semibold text-white", "Animation" }
                nav { class: "flex gap-4 text-sm",
                    a { id: "nav-demo", href: "/animation/", class: "text-sky-300 hover:text-sky-200", "The demo scenes" }
                    a { id: "nav-on-the-page", href: "/animation/?scene=on-the-page", class: "text-sky-300 hover:text-sky-200", "On the page" }
                    a { href: "/", class: "text-sky-300 hover:text-sky-200", "All applications" }
                }
            }
            if on_the_page() {
                OnThePage {}
            } else {
                GpuPage {}
            }
        }
    }
}

/// Asks for the GPU once, then shows the page or the "not available" message.
#[component]
fn GpuPage() -> Element {
    let gpu = use_resource(GpuContext::request);
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
        Some(Ok(context)) => rsx! { FontsPage { context: context.clone() } },
    }
}

/// The fonts the demo draws with, by the backend's name for them (the font file's name in lower
/// case), with a name for messages: Roboto FIRST, so it is the registry's default (the C# demo's
/// font), then Open Sans (the missing-characters scene and the SVG text).
const FONT_FILES: [(&str, &str); 2] = [("roboto-regular.ttf", "Roboto"), ("opensans-regular.ttf", "Open Sans")];

/// Fetches the demo's fonts from the backend once (the generated `font_file_bytes` call),
/// registers them in FONT_FILES' order - and its decoded picture, then shows the demo.
async fn load_fonts() -> Result<FontRegistry, String> {
    let registry = FontRegistry::new();
    for (file, family) in FONT_FILES {
        let bytes = fonts::font_file_bytes(file).await.map_err(|e| format!("The font {family} ({file}) could not be read from the backend: {}", e.message()))?;
        set_svg_font(bytes.clone());
        registry.register_font(bytes, None)?;
    }
    // The VARIABLE Roboto (axes wght, wdth), registered once per weight the "text" scene shows,
    // each instance under its own family name (one instance per registration).
    let variable = fonts::font_file_bytes("roboto.ttf").await.map_err(|e| format!("The variable font Roboto (roboto.ttf) could not be read from the backend: {}", e.message()))?;
    for (weight, family) in demo::text::VARIABLE_WEIGHTS {
        registry.register_font_with_variations(variable.clone(), Some(family), &[FontVariation::weight(weight)])?;
    }
    // The picture the backend decoded (pixels: width, height, RGBA) - for the "images" scene.
    let pixels = pictures::cube_pets_colormap_bytes().await.map_err(|e| format!("The picture could not be read from the backend: {}", e.message()))?;
    set_backend_picture(from_pixel_form(&pixels)?);
    // The same picture as the PNG FILE itself, decoded here by the crate (ImageSource::from_png).
    let png = pictures::cube_pets_colormap_png_bytes().await.map_err(|e| format!("The PNG file could not be read from the backend: {}", e.message()))?;
    set_backend_png_picture(codebrix_webgpu_animation::images::ImageSource::from_png(&png)?);
    // The same picture as a JPEG FILE the backend made from it, decoded here by the crate (ImageSource::from_jpeg).
    let jpeg = pictures::cube_pets_colormap_jpeg_bytes().await.map_err(|e| format!("The JPEG file could not be read from the backend: {}", e.message()))?;
    set_backend_jpeg_picture(codebrix_webgpu_animation::images::ImageSource::from_jpeg(&jpeg)?);
    // The captions scene's cues (SRT text) - a text resource of the backend, like the fonts.
    let srt = captions::caption_scene_srt_bytes().await.map_err(|e| format!("The caption file could not be read from the backend: {}", e.message()))?;
    set_backend_captions(String::from_utf8(srt).map_err(|_| "The caption file from the backend is not UTF-8 text.".to_string())?);
    // The SVG scenes' documents (SVG text) - resources of the backend too.
    fetch_svg_documents().await?;
    // The Lottie scenes' animations (Lottie JSON) - resources of the backend too, read once here.
    let read = |e: codebrix_webplatform::api::ApiError| format!("A Lottie file could not be read from the backend: {}", e.message());
    set_backend_lottie("A_many_shapes.json", &lottie::many_shapes_json_bytes().await.map_err(read)?)?;
    set_backend_lottie("B_painterly.json", &lottie::painterly_json_bytes().await.map_err(read)?)?;
    set_backend_lottie("C_busy_scene.json", &lottie::busy_scene_json_bytes().await.map_err(read)?)?;
    set_backend_lottie("D_features.json", &lottie::features_json_bytes().await.map_err(read)?)?;
    set_backend_lottie("E_image_layer.json", &lottie::image_layer_json_bytes().await.map_err(read)?)?;
    // The picture of that file's image layer: a badge the backend made in code (pixels).
    let badge = pictures::lottie_badge_bytes().await.map_err(|e| format!("The Lottie badge could not be read from the backend: {}", e.message()))?;
    set_backend_badge(from_pixel_form(&badge)?);
    // The "live-pickup" scene's 3D source: model files the graphics crate's glTF loader reads, and
    // the ground's blob (about 0.4 MB; each file once).
    demo::live_pickup::fetch_scene_models().await?;
    Ok(registry)
}

/// Shows the demo once its fonts are registered.
#[component]
fn FontsPage(context: GpuContext) -> Element {
    let loaded = use_resource(load_fonts);
    let state = loaded.read();
    match &*state {
        None => rsx! { p { id: "fonts-pending", class: "text-sm text-slate-400", "Fetching the fonts from the backend..." } },
        Some(Err(reason)) => rsx! { p { id: "fonts-failed", class: "rounded-lg border border-red-400/40 bg-red-400/10 p-5 text-red-100", "{reason}" } },
        Some(Ok(registry)) => rsx! { DemoPage { context: context.clone(), fonts: registry.clone() } },
    }
}

/// True for the address `?scene=on-the-page`: the page that shows drawing ON THE PAGE (a
/// frameless, transparent canvas over the page's own background) instead of the demo.
fn on_the_page() -> bool {
    let search = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
    search.trim_start_matches('?').split('&').any(|pair| pair == "scene=on-the-page")
}
