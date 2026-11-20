//! The "Textured quad" scene: the quad layer (quad.rs) on a canvas of the codebrix-webgpu host,
//! with its own clock (pause and resume), and three one-off checks of the graphics crate's
//! asynchronous side: a buffer read back from the GPU, a pipeline built with WebGPU's verdict
//! awaited (`build_checked`), and a deliberately invalid buffer whose validation error the
//! context's `validate` scope returns instead of losing it.

use codebrix_webgpu::{wgpu, CanvasClock, CanvasLayers, GpuContext, RendererHandle, WebGpuCanvas};
use codebrix_webgpu_graphics::shaders::BuiltInShaders;
use codebrix_webgpu_graphics::{BindingLayoutEntry, GraphicsContext};
use dioxus::prelude::*;

use crate::quad::{pipeline_builder, QuadRenderer, QuadView};

/// The values the readback check writes and expects back.
const READBACK: [u32; 5] = [1, 2, 3, 4, 0xDEAD_BEEF];

/// The canvas with the quad, its clock and the checks.
#[component]
pub fn QuadScene(context: GpuContext) -> Element {
    let graphics = use_hook(|| GraphicsContext::wrap_gpu_context(&context));
    let view = use_signal(QuadView::default);
    let errors = use_signal(Vec::<String>::new);
    let mut paused = use_signal(|| false);
    let mut readback = use_signal(|| "waiting".to_string());
    let mut pipeline_check = use_signal(|| "waiting".to_string());
    let mut validate_check = use_signal(|| "waiting".to_string());
    let clock = use_hook(CanvasClock::new);
    let layers = use_hook(|| CanvasLayers::single(RendererHandle::new(QuadRenderer::new(graphics.clone(), view, errors))));
    {
        let graphics = graphics.clone();
        use_hook(move || {
            spawn(async move {
                readback.set(read_back(&graphics).await);
                pipeline_check.set(check_pipeline(&graphics).await);
                validate_check.set(check_validation_error(&graphics).await);
            });
        });
    }
    let toggle_clock = {
        let clock = clock.clone();
        move |_| paused.set(clock.toggle())
    };
    let button = "rounded-md bg-slate-800 px-3 py-1.5 text-sm text-slate-100 ring-1 ring-white/10 hover:bg-slate-700";
    let quad = view();
    let state = if paused() { "paused" } else { "running" };
    rsx! {
        WebGpuCanvas {
            context: context.clone(),
            layers,
            clock: clock.clone(),
            clear_color: wgpu::Color { r: 0.01, g: 0.012, b: 0.02, a: 1.0 },
            id: "quad-canvas",
            label: "A textured square drawn with the graphics library, turning with the clock. Drag to turn it further.",
            class: "aspect-video w-full touch-none select-none rounded-lg ring-1 ring-white/10 focus-visible:outline-2 focus-visible:outline-sky-400",
        }
        div { class: "flex flex-wrap gap-3",
            button { id: "quad-clock-toggle", class: "{button}", onclick: toggle_clock, if paused() { "Resume the clock" } else { "Pause the clock" } }
        }
        div { class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
            p { class: "text-slate-300",
                "Frame " span { id: "quad-frame", class: "font-mono text-white", "{quad.frame}" }
                ", clock " span { id: "quad-clock-state", "{state}" }
                ", angle " span { id: "quad-angle", class: "font-mono text-white", "{quad.degrees:.1}" } " degrees"
            }
            p { class: "text-slate-300", "Buffer read back from the GPU: " span { id: "quad-readback", class: "font-mono text-white", "{readback}" } }
            p { class: "text-slate-300", "Pipeline built with WebGPU's verdict awaited: " span { id: "quad-pipeline-check", class: "font-mono text-white", "{pipeline_check}" } }
            p { class: "text-slate-300", "A deliberately invalid buffer inside validate(): " span { id: "quad-validate", class: "font-mono text-white", "{validate_check}" } }
            p { class: "text-slate-300", "Errors while drawing: " span { id: "quad-error-count", class: "font-mono text-white", "{errors.read().len()}" } }
            for error in errors.read().iter() {
                p { class: "font-mono text-red-300", "{error}" }
            }
            p { class: "mt-2 text-slate-400", "The square turns with the canvas clock; pausing the clock stops it, resuming goes on from the same time. Drag to turn it further (the square follows the pointer). Its texture is made in code: red, green, blue and yellow quarters with a white bar along the top edge." }
        }
    }
}

/// Writes a storage buffer, reads it back, and says whether the values came back unchanged.
async fn read_back(graphics: &GraphicsContext) -> String {
    let buffer = match graphics.create_storage_buffer_from(&READBACK, Some("readback check")) {
        Ok(buffer) => buffer,
        Err(error) => return format!("FAILED: {error}"),
    };
    match buffer.read::<u32>(None, 0).await {
        Ok(values) if values == READBACK => format!("ok ({})", values.iter().map(|v| format!("{v:#x}")).collect::<Vec<_>>().join(", ")),
        Ok(values) => format!("FAILED: read {values:?}"),
        Err(error) => format!("FAILED: {error}"),
    }
}

/// Builds the quad's pipeline once more, for an sRGB target, and awaits WebGPU's verdict.
async fn check_pipeline(graphics: &GraphicsContext) -> String {
    let shader = match graphics.try_create_shader(BuiltInShaders::SPRITE, Some("sprite (checked)")).await {
        Ok((shader, _)) => shader,
        Err(error) => return format!("FAILED: {error}"),
    };
    let globals = graphics.create_bind_group_layout(None, &[BindingLayoutEntry::uniform(0, wgpu::ShaderStages::VERTEX)]);
    let material = graphics.create_bind_group_layout(None, &[BindingLayoutEntry::texture(0, wgpu::ShaderStages::FRAGMENT), BindingLayoutEntry::sampler(1, wgpu::ShaderStages::FRAGMENT)]);
    match pipeline_builder(graphics, &shader, wgpu::TextureFormat::Rgba8UnormSrgb, &globals, &material).build_checked().await {
        Ok(_) => "ok".to_string(),
        Err(error) => format!("FAILED: {error}"),
    }
}

/// Asks for a buffer WebGPU refuses (map-read together with uniform) inside `validate`, and says
/// whether the refusal came back as an error (it must) rather than as an uncaptured error.
async fn check_validation_error(graphics: &GraphicsContext) -> String {
    let result = graphics.validate(|g| g.create_buffer(16, wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::UNIFORM, Some("refused on purpose"))).await;
    let leaked = graphics.take_errors().len();
    match result {
        Err(_) if leaked == 0 => "ok (the validation error was returned)".to_string(),
        Err(_) => format!("FAILED: returned, but {leaked} error(s) also reached take_errors"),
        Ok(_) => "FAILED: no error was returned".to_string(),
    }
}
