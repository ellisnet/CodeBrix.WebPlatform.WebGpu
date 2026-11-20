//! The "Shaders" scene: every built-in WGSL shader of codebrix-webgpu-graphics - the sprite,
//! lit-mesh, mipmap, environment, skybox and tone-map shaders, and EVERY feature combination of
//! the PBR model shader - compiled once by this browser's own WGSL compiler, each listed with
//! what the compiler said. Each browser compiles WGSL itself, so this is the first thing to look
//! at in a new browser or after a change to the shaders.

use codebrix_webgpu::GpuContext;
use codebrix_webgpu_graphics::shaders::BuiltInShaderSource;
use codebrix_webgpu_graphics::{GraphicsContext, PbrFeatures, ShaderMessageKind};
use dioxus::prelude::*;

/// How many results are collected before the list on the page is updated.
const BATCH: usize = 24;

/// One compiled shader, as the page lists it.
#[derive(Clone, Debug, PartialEq)]
struct Compiled {
    name: String,
    lines: usize,
    ok: bool,
    warnings: usize,
    messages: String,
}

/// The list of built-in shaders and their compile results.
#[component]
pub fn ShaderScene(context: GpuContext) -> Element {
    let mut results = use_signal(Vec::<Compiled>::new);
    let mut done = use_signal(|| false);
    let mut uncaptured = use_signal(Vec::<String>::new);
    let total = use_hook(|| BuiltInShaderSource::all(PbrFeatures::every_shader_variant()).len());
    use_hook(move || {
        let graphics = GraphicsContext::wrap_gpu_context(&context);
        spawn(async move {
            let mut batch = Vec::new();
            for source in BuiltInShaderSource::all(PbrFeatures::every_shader_variant()) {
                let lines = source.source.lines().count();
                let row = match graphics.try_create_shader(&source.source, Some(&source.name)).await {
                    Ok((_shader, diagnostics)) => Compiled {
                        name: source.name.clone(),
                        lines,
                        ok: true,
                        warnings: diagnostics.count(ShaderMessageKind::Warning),
                        messages: diagnostics.to_string(),
                    },
                    Err(error) => Compiled { name: source.name.clone(), lines, ok: false, warnings: 0, messages: error.diagnostics().to_string() },
                };
                batch.push(row);
                if batch.len() >= BATCH {
                    results.write().append(&mut batch);
                }
            }
            results.write().append(&mut batch);
            uncaptured.set(graphics.take_errors().iter().map(ToString::to_string).collect());
            done.set(true);
        });
    });

    let list = results.read();
    let compiled = list.len();
    let failed = list.iter().filter(|r| !r.ok).count();
    let warnings: usize = list.iter().map(|r| r.warnings).sum();
    let state = if done() { "done" } else { "compiling" };
    rsx! {
        section { class: "rounded-lg bg-slate-900 p-4 text-sm ring-1 ring-white/10",
            h2 { class: "text-lg font-semibold text-white", "Built-in shaders, compiled by this browser" }
            p { class: "mt-1 text-slate-400", "The ten shaders of the library's renderers and all 512 feature combinations of the PBR model shader, each compiled once on this adapter. A message from the compiler is shown under its shader." }
            p { class: "mt-3 text-slate-200",
                span { id: "shader-state", "{state}" } ": "
                span { id: "shader-compiled", class: "font-mono text-white", "{compiled}" } " of "
                span { id: "shader-total", class: "font-mono text-white", "{total}" } " compiled, "
                span { id: "shader-failed", class: "font-mono text-white", "{failed}" } " with errors, "
                span { id: "shader-warnings", class: "font-mono text-white", "{warnings}" } " warnings; uncaptured WebGPU errors: "
                span { id: "shader-uncaptured", class: "font-mono text-white", "{uncaptured.read().len()}" }
            }
            for error in uncaptured.read().iter() {
                p { class: "mt-1 font-mono text-red-300", "{error}" }
            }
        }
        div { class: "max-h-[32rem] overflow-y-auto rounded-lg ring-1 ring-white/10",
            table { id: "shader-list", class: "w-full text-left text-sm",
                thead { class: "sticky top-0 bg-slate-800 text-slate-300",
                    tr {
                        th { class: "px-3 py-2", "Shader" }
                        th { class: "px-3 py-2", "Lines" }
                        th { class: "px-3 py-2", "Result" }
                        th { class: "px-3 py-2", "Compiler messages" }
                    }
                }
                tbody {
                    for row in list.iter() {
                        tr { key: "{row.name}", class: "border-t border-white/5", "data-ok": if row.ok { "true" } else { "false" },
                            td { class: "px-3 py-1 font-mono text-slate-100", "{row.name}" }
                            td { class: "px-3 py-1 font-mono text-slate-400", "{row.lines}" }
                            td { class: if row.ok { "px-3 py-1 text-emerald-300" } else { "px-3 py-1 text-red-300" }, if row.ok { "compiled" } else { "ERROR" } }
                            td { class: "whitespace-pre-wrap px-3 py-1 font-mono text-xs text-amber-200", if row.messages.is_empty() { "(none)" } else { "{row.messages}" } }
                        }
                    }
                }
            }
        }
    }
}
