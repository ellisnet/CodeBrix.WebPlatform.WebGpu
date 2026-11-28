//! The GPU half of the renderer: pipelines, the offscreen targets, the buffers, the image
//! textures, and the encoding of one frame's operations into render passes of the frame's
//! shared encoder. It draws OVER what the layers below drew and never clears the host's view.
//! Groups with effects take extra passes into float textures (effects.wgsl): a resolve, box
//! filters for wide blurs, separable blur passes, then their composite into the target below.
//! Stroke sets (strokes.wgsl) are one instanced draw each: their records are a storage buffer
//! uploaded once per set, their tips a texture array uploaded once, and each draw's block is one
//! 256-byte-aligned piece of the frame's uniform buffer at a dynamic offset.
//! What is held is swept every frame by one policy (texture_ledger.rs `RESIDENCY`).
//! A stroke draw that PICKS its colours binds its source beside its tips (group 1: tips, their
//! sampler, the source, its sampler): a picture of the frame, a `PictureTarget`'s texture or the
//! application's view. A layer's SURFACE FINISH adds a HEIGHT PASS before its effect group's
//! composite (its stroke sets drawn again, single-sampled, additively into an R16Float height
//! record), which the composite reads. A `PictureTarget` layer's final pass writes its picture
//! (premultiplied sRGB-encoded bytes) instead of the canvas view.
//! STROKE CHECKPOINTS (strokes/stroke_checkpoints.rs): a stroke draw with a checkpoint plan leaves
//! the render pass - a held checkpoint (a copy of the level's multisampled colour target) is
//! copied back into the target, the strokes after it are drawn in a pass of their own, and a new
//! checkpoint is copied out of the target after the strokes up to it - then the level's pass goes
//! on. A TRANSITION's draw binds its two record buffers (group 2, bindings 0 and 1) in the order
//! the builder chose; a plain set binds its records at both.
//! WATER (Not in C#): an effect composite with a refraction or a reflection reads the frame's
//! `ripples` storage buffer (group 2 binding 2, one upload per frame) and up to two height pictures
//! (group 1 bindings 5 and 6: an image-cache texture, a picture target's texture, an application's
//! view, or a 1 x 1 blank); the composite module carries noise.wgsl for the caustic light.
//! GPU TIMING (Not in C#; gpu_timer.rs): switched on, the frame's first pass writes a timestamp at
//! its beginning and its last pass one at its end (a frame whose last pass would not be one of the
//! final passes gets an empty pass for it), resolved and copied after the last pass; switched off
//! (the default) every pass has `timestamp_writes: None` as before. A LIVE PICTURE
//! (images/live_picture.rs) is drawn from its own texture, its queued regions written first.
//! THE TARGET WINDOW (Not in C#; target_window.rs): the level targets, the backdrop copy, the
//! effect textures, the height record and the checkpoints are the size of the frame's window
//! (the builder's `window`), not the canvas's, made again when its size changes; the view
//! uniform carries the window's size and origin and the output's size, and the final passes
//! (`vs_final`) put the targets back at the window's origin on the canvas's view, the readback
//! copy or the picture target, which keep the output's size.

use std::collections::HashMap;
use std::ops::Range;

use codebrix_webgpu::{wgpu, CanvasTarget};
use peniko::{ImageData, ImageFormat};

use crate::images::LivePicture;
use crate::rendering::core::gpu_timer::{GpuTiming, PassStamps};
use crate::rendering::core::FrameOutput;
use crate::rendering::core::build::{Builder, CompositeMode, FxRole, FxStep, FxTexture, Op, ParticleInstance, StrokeOp, StrokeSource, WaterBinding};
use crate::strokes::stroke_checkpoints::{CheckpointCache, CheckpointStep};
use crate::strokes::StrokeCheckpoints;
use crate::rendering::picture_target::{PictureTarget, PICTURE_FORMAT};
use crate::rendering::core::paint::{PaintData, RAMP_SAMPLES, RAMP_SLOTS};
use crate::rendering::core::target_window::TargetWindow;
use crate::rendering::core::texture_ledger::{self, FrameImage, Residency, TextureLedger, Upload, CHECKPOINT_RESIDENCY, RESIDENCY};
use crate::strokes::stroke_gpu::{pack_set, DRAW_BINDING_BYTES, RECORD_BYTES};

const SAMPLES: u32 = 4;
const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const STENCIL: wgpu::TextureFormat = wgpu::TextureFormat::Stencil8;
/// The effect textures: linear premultiplied colour in half floats (filterable everywhere).
const FX_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const WIND: u32 = 0x1F;
const CLIP: u32 = 0xE0;
const SHIFT: u32 = 5;
/// The brush tips' texels: coverage and height.
const TIP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg8Unorm;
/// A finished layer's height record: paint thickness, summed (renderable, blendable and
/// filterable in core WebGPU).
const HEIGHT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

#[cfg(target_arch = "wasm32")]
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn bytes<T>(v: &[T]) -> &[u8] {
    // SAFETY: only called with #[repr(C)] plain-old-data structs of f32/u32 fields (Vertex,
    // PaintData, [f32; 4]): no padding, no pointers.
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

struct Target {
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    stencil_view: wgpu::TextureView,
    bind: wgpu::BindGroup,
}

struct Backdrop {
    texture: wgpu::Texture,
    bind: wgpu::BindGroup,
}

struct FxEntry {
    view: wgpu::TextureView,
    bind: wgpu::BindGroup,
}

/// The shared head of the effect shaders and one part (the text after "//@part <name>"); the
/// composite part also carries the shared noise (noise.wgsl: the caustic light of a refraction).
pub(crate) fn fx_source(part: &str) -> String {
    let all = include_str!("effects.wgsl");
    let mut pieces = all.split("//@part ");
    let head = pieces.next().unwrap_or_default();
    let body = pieces.find(|p| p.starts_with(part)).unwrap_or_default();
    let body = body.split_once('\n').map_or("", |(_, rest)| rest);
    let noise = if part == "composite" { include_str!("noise.wgsl") } else { "" };
    format!("{}{}{}{}", include_str!("common.wgsl"), noise, head, body)
}

struct ImageEntry {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind: wgpu::BindGroup,
}

/// A stroke set's records on the GPU (the buffer stays alive through its bind group, which binds
/// it at both record bindings), or a transition's two record buffers in one of its orders.
struct StrokeSetEntry {
    bind: wgpu::BindGroup,
}

/// A set of brush tips on the GPU (the texture stays alive through its view and bind group;
/// the bind group carries the blank colour source).
struct TipsEntry {
    view: wgpu::TextureView,
    bind: wgpu::BindGroup,
}

/// A finished layer's height record (the target window's size).
struct HeightTarget {
    view: wgpu::TextureView,
}

/// Reads the 2D layer's own pixels back (testing only).
pub(crate) struct Readback {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_row: u32,
}

pub(crate) struct Gpu {
    stencil: wgpu::RenderPipeline,
    cover: [wgpu::RenderPipeline; 2],
    clip_push: [wgpu::RenderPipeline; 2],
    wind_reset: wgpu::RenderPipeline,
    clip_pop: wgpu::RenderPipeline,
    clear: wgpu::RenderPipeline,
    composite: Vec<wgpu::RenderPipeline>,
    blend: wgpu::RenderPipeline,
    final_view: wgpu::RenderPipeline,
    final_readback: wgpu::RenderPipeline,
    fx_resolve: wgpu::RenderPipeline,
    fx_down: wgpu::RenderPipeline,
    fx_blur: wgpu::RenderPipeline,
    fx_composite: wgpu::RenderPipeline,
    particles: wgpu::RenderPipeline,
    particle_buffer: Option<wgpu::Buffer>,
    strokes: wgpu::RenderPipeline,
    stroke_height: wgpu::RenderPipeline,
    picture_final: wgpu::RenderPipeline,
    layout_tips: wgpu::BindGroupLayout,
    layout_records: wgpu::BindGroupLayout,
    layout_draw: wgpu::BindGroupLayout,
    no_tips: wgpu::BindGroup,
    blank_tips_view: wgpu::TextureView,
    tip_sampler: wgpu::Sampler,
    blank_source_view: wgpu::TextureView,
    source_sampler: wgpu::Sampler,
    /// The draws' texture groups with a colour source, by (tips id or 0, source view), with the
    /// frame each was last used in.
    source_groups: HashMap<(u64, wgpu::TextureView), (wgpu::BindGroup, u64)>,
    height: Option<HeightTarget>,
    blank_height_view: wgpu::TextureView,
    stroke_sets: HashMap<u64, StrokeSetEntry>,
    set_residency: Residency<u64>,
    /// Transitions' record buffers, by (morph id, order).
    morph_sets: HashMap<(u64, u8), StrokeSetEntry>,
    morph_residency: Residency<(u64, u8)>,
    /// The checkpoints held (the CPU half) and their textures, by slot.
    checkpoints: CheckpointCache,
    checkpoint_textures: Vec<Option<wgpu::Texture>>,
    tips: HashMap<u64, TipsEntry>,
    tip_residency: Residency<u64>,
    draw_buffer: Option<wgpu::Buffer>,
    draw_bind: Option<wgpu::BindGroup>,
    layout_fx: wgpu::BindGroupLayout,
    layout_fx_tex: wgpu::BindGroupLayout,
    layout_fx_comp: wgpu::BindGroupLayout,
    fx_textures: HashMap<FxTexture, FxEntry>,
    fx_buffer: Option<wgpu::Buffer>,
    fx_kernel_buffer: Option<wgpu::Buffer>,
    /// The frame's ripple records (water; group 2 binding 2 of the effect passes).
    ripple_buffer: Option<wgpu::Buffer>,
    fx_bind: Option<wgpu::BindGroup>,
    layout0: wgpu::BindGroupLayout,
    layout_image: wgpu::BindGroupLayout,
    layout_ms: wgpu::BindGroupLayout,
    view_buffer: wgpu::Buffer,
    ramp_buffer: wgpu::Buffer,
    vertex_buffer: Option<wgpu::Buffer>,
    paint_buffer: Option<wgpu::Buffer>,
    bind0: Option<wgpu::BindGroup>,
    no_image: wgpu::BindGroup,
    linear: wgpu::Sampler,
    nearest: wgpu::Sampler,
    /// The picture textures, by picture and slot (see texture_ledger.rs).
    images: HashMap<((u64, u32), usize), ImageEntry>,
    /// Which textures are held and which colour table each holds.
    ledger: TextureLedger,
    targets: Vec<Target>,
    backdrop: Option<Backdrop>,
    /// The output's size (the canvas's, or a picture target's).
    pub(crate) size: (u32, u32),
    /// The internal targets' size (the frame's target window; (0, 0) until the first frame).
    target_size: (u32, u32),
    frame: u64,
    pub(crate) readback: Option<Readback>,
    /// The GPU timing of the frames (off by default).
    pub(crate) timing: GpuTiming,
}

fn face(compare: wgpu::CompareFunction, pass_op: wgpu::StencilOperation) -> wgpu::StencilFaceState {
    wgpu::StencilFaceState { compare, fail_op: wgpu::StencilOperation::Keep, depth_fail_op: wgpu::StencilOperation::Keep, pass_op }
}

fn stencil_state(front: wgpu::StencilFaceState, back: wgpu::StencilFaceState, read_mask: u32, write_mask: u32) -> Option<wgpu::DepthStencilState> {
    Some(wgpu::DepthStencilState { format: STENCIL, depth_write_enabled: None, depth_compare: None, stencil: wgpu::StencilState { front, back, read_mask, write_mask }, bias: wgpu::DepthBiasState::default() })
}

/// The fixed-function blend of a Porter-Duff compose on premultiplied colours (index = peniko's
/// `Compose` value).
fn compose_blend(compose: u8) -> wgpu::BlendState {
    use wgpu::BlendFactor as F;
    let (src, dst) = match compose {
        0 => (F::Zero, F::Zero),                          // Clear
        1 => (F::One, F::Zero),                           // Copy
        2 => (F::Zero, F::One),                           // Dest
        4 => (F::OneMinusDstAlpha, F::One),               // DestOver
        5 => (F::DstAlpha, F::Zero),                      // SrcIn
        6 => (F::Zero, F::SrcAlpha),                      // DestIn
        7 => (F::OneMinusDstAlpha, F::Zero),              // SrcOut
        8 => (F::Zero, F::OneMinusSrcAlpha),              // DestOut
        9 => (F::DstAlpha, F::OneMinusSrcAlpha),          // SrcAtop
        10 => (F::OneMinusDstAlpha, F::SrcAlpha),         // DestAtop
        11 => (F::OneMinusDstAlpha, F::OneMinusSrcAlpha), // Xor
        12 | 13 => (F::One, F::One),                      // Plus, PlusLighter (the target clamps)
        _ => (F::One, F::OneMinusSrcAlpha),               // SrcOver
    };
    let component = wgpu::BlendComponent { src_factor: src, dst_factor: dst, operation: wgpu::BlendOperation::Add };
    wgpu::BlendState { color: component, alpha: component }
}

impl Gpu {
    pub(crate) fn new(device: &wgpu::Device, target: &CanvasTarget) -> Gpu {
        let paint_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("animation paint"), source: wgpu::ShaderSource::Wgsl(concat!(include_str!("common.wgsl"), include_str!("noise.wgsl"), include_str!("paint.wgsl")).into()) });
        let strokes_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("animation strokes"), source: wgpu::ShaderSource::Wgsl(concat!(include_str!("common.wgsl"), include_str!("noise.wgsl"), include_str!("strokes.wgsl")).into()) });
        let composite_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("animation composite"), source: wgpu::ShaderSource::Wgsl(concat!(include_str!("common.wgsl"), include_str!("composite.wgsl")).into()) });
        let buffer_entry = |binding: u32, visibility: wgpu::ShaderStages, ty: wgpu::BufferBindingType| wgpu::BindGroupLayoutEntry { binding, visibility, ty: wgpu::BindingType::Buffer { ty, has_dynamic_offset: false, min_binding_size: None }, count: None };
        let layout0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("animation frame"),
            entries: &[
                buffer_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT, wgpu::BufferBindingType::Uniform),
                buffer_entry(1, wgpu::ShaderStages::FRAGMENT, wgpu::BufferBindingType::Storage { read_only: true }),
                buffer_entry(2, wgpu::ShaderStages::FRAGMENT, wgpu::BufferBindingType::Storage { read_only: true }),
            ],
        });
        let layout_image = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("animation image"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let layout_ms = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("animation layer texture"),
            entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: true }, count: None }],
        });
        let texture_entry = |binding: u32| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None };
        let layout_fx = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("animation effect passes"),
            // 0 the passes, 1 the blur kernels, 2 the frame's ripple records (the water composite).
            entries: &[buffer_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT, wgpu::BufferBindingType::Storage { read_only: true }), buffer_entry(1, wgpu::ShaderStages::FRAGMENT, wgpu::BufferBindingType::Storage { read_only: true }), buffer_entry(2, wgpu::ShaderStages::FRAGMENT, wgpu::BufferBindingType::Storage { read_only: true })],
        });
        let layout_fx_tex = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("animation effect source"), entries: &[texture_entry(0)] });
        let layout_fx_comp = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("animation effect composite"),
            // 4 the finish's height record; 5 and 6 the water's height pictures (refraction,
            // reflection).
            entries: &[texture_entry(0), texture_entry(1), texture_entry(2), wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None }, texture_entry(4), texture_entry(5), texture_entry(6)],
        });
        let pl_paint = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("animation paint"), bind_group_layouts: &[Some(&layout0), Some(&layout_image)], immediate_size: 0 });
        let pl_composite = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("animation composite"), bind_group_layouts: &[Some(&layout0), Some(&layout_ms)], immediate_size: 0 });
        let pl_blend = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("animation blend"), bind_group_layouts: &[Some(&layout0), Some(&layout_ms), Some(&layout_ms)], immediate_size: 0 });
        let pl_fx_resolve = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("animation effect resolve"), bind_group_layouts: &[Some(&layout0), Some(&layout_ms), Some(&layout_fx)], immediate_size: 0 });
        let pl_fx_filter = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("animation effect filter"), bind_group_layouts: &[Some(&layout0), Some(&layout_fx_tex), Some(&layout_fx)], immediate_size: 0 });
        let pl_fx_comp = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("animation effect composite"), bind_group_layouts: &[Some(&layout0), Some(&layout_fx_comp), Some(&layout_fx)], immediate_size: 0 });
        let fx_resolve_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("animation effect resolve"), source: wgpu::ShaderSource::Wgsl(fx_source("resolve").into()) });
        let fx_filter_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("animation effect filter"), source: wgpu::ShaderSource::Wgsl(fx_source("filter").into()) });
        let fx_comp_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("animation effect composite"), source: wgpu::ShaderSource::Wgsl(fx_source("composite").into()) });
        let picture_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("animation picture target"), source: wgpu::ShaderSource::Wgsl(fx_source("picture").into()) });
        let attributes = [wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 }, wgpu::VertexAttribute { format: wgpu::VertexFormat::Uint32, offset: 8, shader_location: 1 }];
        let buffers = [Some(wgpu::VertexBufferLayout { array_stride: 12, step_mode: wgpu::VertexStepMode::Vertex, attributes: &attributes })];
        #[allow(clippy::too_many_arguments)]
        let make_with = |vs: &str, label: &str, layout: &wgpu::PipelineLayout, module: &wgpu::ShaderModule, fs: &str, format: wgpu::TextureFormat, blend: Option<wgpu::BlendState>, writes: wgpu::ColorWrites, ds: Option<wgpu::DepthStencilState>, samples: u32| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState { module, entry_point: Some(vs), compilation_options: wgpu::PipelineCompilationOptions::default(), buffers: &buffers },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..wgpu::PrimitiveState::default() },
                depth_stencil: ds,
                multisample: wgpu::MultisampleState { count: samples, mask: !0, alpha_to_coverage_enabled: false },
                fragment: Some(wgpu::FragmentState { module, entry_point: Some(fs), compilation_options: wgpu::PipelineCompilationOptions::default(), targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: writes })] }),
                multiview_mask: None,
                cache: None,
            })
        };
        #[allow(clippy::too_many_arguments)]
        let make = |label: &str, layout: &wgpu::PipelineLayout, module: &wgpu::ShaderModule, fs: &str, format: wgpu::TextureFormat, blend: Option<wgpu::BlendState>, writes: wgpu::ColorWrites, ds: Option<wgpu::DepthStencilState>, samples: u32| make_with("vs_main", label, layout, module, fs, format, blend, writes, ds, samples);
        use wgpu::{CompareFunction as C, StencilOperation as S};
        let premul = Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
        let none = wgpu::ColorWrites::empty();
        let all = wgpu::ColorWrites::ALL;
        let (pm, cm) = (&paint_module, &composite_module);
        // The winding count, inside the current clip depth only.
        let stencil = make("animation stencil", &pl_paint, pm, "fs_stencil", COLOR, None, none, stencil_state(face(C::Equal, S::IncrementWrap), face(C::Equal, S::DecrementWrap), CLIP, WIND), SAMPLES);
        // Cover: paint where the winding is non-zero (or odd), and clear the winding. Even-odd
        // also clears the even, non-zero windings it does not paint (fail_op Zero; the write
        // mask keeps the clip depth).
        let cover_nonzero = make("animation cover non-zero", &pl_paint, pm, "fs_paint", COLOR, premul, all, stencil_state(face(C::NotEqual, S::Zero), face(C::NotEqual, S::Zero), WIND, WIND), SAMPLES);
        let even_odd_face = wgpu::StencilFaceState { compare: C::NotEqual, fail_op: S::Zero, depth_fail_op: S::Keep, pass_op: S::Zero };
        let cover_even_odd = make("animation cover even-odd", &pl_paint, pm, "fs_paint", COLOR, premul, all, stencil_state(even_odd_face, even_odd_face, 0x01, WIND), SAMPLES);
        let clip_push_nonzero = make("animation clip push non-zero", &pl_paint, pm, "fs_stencil", COLOR, None, none, stencil_state(face(C::NotEqual, S::Replace), face(C::NotEqual, S::Replace), WIND, 0xFF), SAMPLES);
        let clip_push_even_odd = make("animation clip push even-odd", &pl_paint, pm, "fs_stencil", COLOR, None, none, stencil_state(face(C::NotEqual, S::Replace), face(C::NotEqual, S::Replace), 0x01, 0xFF), SAMPLES);
        let wind_reset = make("animation winding reset", &pl_paint, pm, "fs_stencil", COLOR, None, none, stencil_state(face(C::Equal, S::Zero), face(C::Equal, S::Zero), CLIP, WIND), SAMPLES);
        let clip_pop = make("animation clip pop", &pl_paint, pm, "fs_stencil", COLOR, None, none, stencil_state(face(C::Equal, S::DecrementClamp), face(C::Equal, S::DecrementClamp), CLIP, CLIP), SAMPLES);
        let clear = make("animation clear", &pl_paint, pm, "fs_clear", COLOR, None, all, stencil_state(face(C::Always, S::Replace), face(C::Always, S::Replace), 0xFF, 0xFF), SAMPLES);
        let comp_state = || stencil_state(face(C::Equal, S::Keep), face(C::Equal, S::Keep), CLIP, 0);
        let composite = (0..14u8).map(|c| make("animation composite", &pl_composite, cm, "fs_composite", COLOR, Some(compose_blend(c)), all, comp_state(), SAMPLES)).collect();
        let blend = make("animation blend", &pl_blend, cm, "fs_blend", COLOR, None, all, comp_state(), SAMPLES);
        // The final passes draw onto the OUTPUT (the canvas's size): vs_final, not the targets'
        // mapping.
        let final_view = make_with("vs_final", "animation final", &pl_composite, cm, "fs_final", target.format, premul, all, None, 1);
        let final_readback = make_with("vs_final", "animation final (readback)", &pl_composite, cm, "fs_final", COLOR, premul, all, None, 1);
        // A picture target's final pass: the resolved layer as premultiplied sRGB-encoded bytes,
        // over the cleared picture.
        let picture_final = make_with("vs_final", "animation final (picture target)", &pl_composite, &picture_module, "fs_picture", PICTURE_FORMAT, None, all, None, 1);
        let fx_resolve = make_with("vs_fx", "animation effect resolve", &pl_fx_resolve, &fx_resolve_module, "fs_resolve", FX_FORMAT, None, all, None, 1);
        let fx_down = make_with("vs_fx", "animation effect down", &pl_fx_filter, &fx_filter_module, "fs_down", FX_FORMAT, None, all, None, 1);
        let fx_blur = make_with("vs_fx", "animation effect blur", &pl_fx_filter, &fx_filter_module, "fs_blur", FX_FORMAT, None, all, None, 1);
        let fx_composite = make("animation effect composite", &pl_fx_comp, &fx_comp_module, "fs_composite", COLOR, premul, all, comp_state(), SAMPLES);
        // Particles: soft discs, one instance each (a quad made from the vertex index), inside
        // the clip depth, blended premultiplied over the target.
        let particle_attributes = [
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 8, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 24, shader_location: 2 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 32, shader_location: 3 },
        ];
        let particle_buffers = [Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<ParticleInstance>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &particle_attributes })];
        let particles = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("animation particles"),
            layout: Some(&pl_paint),
            vertex: wgpu::VertexState { module: pm, entry_point: Some("vs_particle"), compilation_options: wgpu::PipelineCompilationOptions::default(), buffers: &particle_buffers },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..wgpu::PrimitiveState::default() },
            depth_stencil: comp_state(),
            multisample: wgpu::MultisampleState { count: SAMPLES, mask: !0, alpha_to_coverage_enabled: false },
            fragment: Some(wgpu::FragmentState { module: pm, entry_point: Some("fs_particle"), compilation_options: wgpu::PipelineCompilationOptions::default(), targets: &[Some(wgpu::ColorTargetState { format: COLOR, blend: premul, write_mask: all })] }),
            multiview_mask: None,
            cache: None,
        });
        // Stroke sets: no vertex buffer (the hull from the vertex index, the record from the
        // instance index), inside the clip depth, blended premultiplied over the target.
        // Group 1 of a stroke draw: its textures - the tips (and their trilinear sampler) and the
        // colour source the vertex shader picks from (and its bilinear, clamped sampler).
        let layout_tips = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("animation stroke textures"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2Array, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        // Group 2: the records (binding 0) and a transition's targets (binding 1; a plain set binds
        // its records there too).
        let layout_records = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("animation stroke records"), entries: &[buffer_entry(0, wgpu::ShaderStages::VERTEX, wgpu::BufferBindingType::Storage { read_only: true }), buffer_entry(1, wgpu::ShaderStages::VERTEX, wgpu::BufferBindingType::Storage { read_only: true })] });
        let layout_draw = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("animation stroke draw"),
            entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: wgpu::BufferSize::new(DRAW_BINDING_BYTES as u64) }, count: None }],
        });
        let pl_strokes = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("animation strokes"), bind_group_layouts: &[Some(&layout0), Some(&layout_tips), Some(&layout_records), Some(&layout_draw)], immediate_size: 0 });
        let strokes = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("animation strokes"),
            layout: Some(&pl_strokes),
            vertex: wgpu::VertexState { module: &strokes_module, entry_point: Some("vs_stroke"), compilation_options: wgpu::PipelineCompilationOptions::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, cull_mode: None, ..wgpu::PrimitiveState::default() },
            depth_stencil: comp_state(),
            multisample: wgpu::MultisampleState { count: SAMPLES, mask: !0, alpha_to_coverage_enabled: false },
            fragment: Some(wgpu::FragmentState { module: &strokes_module, entry_point: Some("fs_stroke"), compilation_options: wgpu::PipelineCompilationOptions::default(), targets: &[Some(wgpu::ColorTargetState { format: COLOR, blend: premul, write_mask: all })] }),
            multiview_mask: None,
            cache: None,
        });
        // The height pass of a finished layer: the same strokes, single-sampled, no stencil (it
        // ignores clips and masks), summed into the height record.
        let add = wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add };
        let stroke_height = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("animation stroke height"),
            layout: Some(&pl_strokes),
            vertex: wgpu::VertexState { module: &strokes_module, entry_point: Some("vs_stroke"), compilation_options: wgpu::PipelineCompilationOptions::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, cull_mode: None, ..wgpu::PrimitiveState::default() },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState { module: &strokes_module, entry_point: Some("fs_stroke_height"), compilation_options: wgpu::PipelineCompilationOptions::default(), targets: &[Some(wgpu::ColorTargetState { format: HEIGHT_FORMAT, blend: Some(wgpu::BlendState { color: add, alpha: add }), write_mask: all })] }),
            multiview_mask: None,
            cache: None,
        });
        let tip_sampler = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("animation brush tips"), mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::MipmapFilterMode::Linear, ..wgpu::SamplerDescriptor::default() });
        let source_sampler = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("animation stroke colour source"), mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..wgpu::SamplerDescriptor::default() });
        let blank_tips = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation no brush tips"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: TIP_FORMAT, usage: wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
        let blank_tips_view = blank_tips.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..wgpu::TextureViewDescriptor::default() });
        let blank_source = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation no colour source"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
        let blank_source_view = blank_source.create_view(&wgpu::TextureViewDescriptor::default());
        let blank_height = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation no height"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: HEIGHT_FORMAT, usage: wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
        let blank_height_view = blank_height.create_view(&wgpu::TextureViewDescriptor::default());
        let no_tips = Gpu::tips_bind(device, &layout_tips, &blank_tips_view, &tip_sampler, &blank_source_view, &source_sampler);
        // The view (common.wgsl View): the target window's size and origin, the output's size.
        let view_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation view"), size: 32, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let ramp_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation ramps"), size: (RAMP_SLOTS * RAMP_SAMPLES * 16) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let linear = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("animation image linear"), mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..wgpu::SamplerDescriptor::default() });
        let nearest = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("animation image nearest"), ..wgpu::SamplerDescriptor::default() });
        let blank = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation no image"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
        let blank_view = blank.create_view(&wgpu::TextureViewDescriptor::default());
        let no_image = Gpu::image_bind(device, &layout_image, &blank_view, &linear, &nearest);
        Gpu {
            stencil,
            cover: [cover_nonzero, cover_even_odd],
            clip_push: [clip_push_nonzero, clip_push_even_odd],
            wind_reset,
            clip_pop,
            clear,
            composite,
            blend,
            final_view,
            final_readback,
            fx_resolve,
            fx_down,
            fx_blur,
            fx_composite,
            particles,
            particle_buffer: None,
            strokes,
            stroke_height,
            picture_final,
            layout_tips,
            layout_records,
            layout_draw,
            no_tips,
            blank_tips_view,
            tip_sampler,
            blank_source_view,
            source_sampler,
            source_groups: HashMap::new(),
            height: None,
            blank_height_view,
            stroke_sets: HashMap::new(),
            set_residency: Residency::default(),
            morph_sets: HashMap::new(),
            morph_residency: Residency::default(),
            checkpoints: CheckpointCache::default(),
            checkpoint_textures: Vec::new(),
            tips: HashMap::new(),
            tip_residency: Residency::default(),
            draw_buffer: None,
            draw_bind: None,
            layout_fx,
            layout_fx_tex,
            layout_fx_comp,
            fx_textures: HashMap::new(),
            fx_buffer: None,
            fx_kernel_buffer: None,
            ripple_buffer: None,
            fx_bind: None,
            layout0,
            layout_image,
            layout_ms,
            view_buffer,
            ramp_buffer,
            vertex_buffer: None,
            paint_buffer: None,
            bind0: None,
            no_image,
            linear,
            nearest,
            images: HashMap::new(),
            ledger: TextureLedger::default(),
            targets: Vec::new(),
            backdrop: None,
            size: (0, 0),
            target_size: (0, 0),
            frame: 0,
            readback: None,
            timing: GpuTiming::new(device),
        }
    }

    fn image_bind(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, view: &wgpu::TextureView, linear: &wgpu::Sampler, nearest: &wgpu::Sampler) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("animation image"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(linear) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(nearest) },
            ],
        })
    }

    fn tips_bind(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, view: &wgpu::TextureView, sampler: &wgpu::Sampler, source: &wgpu::TextureView, source_sampler: &wgpu::Sampler) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("animation stroke textures"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(source) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(source_sampler) },
            ],
        })
    }

    /// The bind groups a stroke draw uses (its set's records; its textures: tips and colour
    /// source), the set and the tips each uploaded the first time they are drawn (counted into
    /// `stats`). `image_views` are the frame's pictures (index 0: none).
    fn stroke_groups(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, op: &StrokeOp, image_views: &[Option<wgpu::TextureView>], stats: &mut crate::rendering::RenderStats) -> (wgpu::BindGroup, wgpu::BindGroup) {
        let (sets, tips) = (&self.stroke_sets, &self.tips);
        let held = |is_tips: bool, id: u64| if is_tips { tips.contains_key(&id) } else { sets.contains_key(&id) };
        let (upload_set, upload_tips) = texture_ledger::stroke_uploads(&mut self.set_residency, &mut self.tip_residency, op, self.frame, &held, stats);
        let id = op.set.id();
        let records_buffer = |data: &[u8]| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation stroke records"), size: (data.len().max(64)) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            queue.write_buffer(&buffer, 0, data);
            buffer
        };
        let records = match &op.morph {
            // A transition: its two record buffers in the order the builder chose, uploaded once
            // per order while it stays held.
            Some((m, variant)) => {
                let key = (m.id, *variant);
                let bytes = 2 * m.morph.len() * RECORD_BYTES;
                if self.morph_residency.use_now(key, bytes, self.frame) || !self.morph_sets.contains_key(&key) {
                    stats.stroke_uploads += 1;
                    stats.stroke_upload_bytes += bytes;
                    let (first, second) = m.morph.pack_variant(&m.delays, *variant);
                    let (a, b) = (records_buffer(&first), records_buffer(&second));
                    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("animation stroke transition"), layout: &self.layout_records, entries: &[wgpu::BindGroupEntry { binding: 0, resource: a.as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: b.as_entire_binding() }] });
                    self.morph_sets.insert(key, StrokeSetEntry { bind });
                }
                self.morph_sets[&key].bind.clone()
            }
            None => {
                if upload_set {
                    let buffer = records_buffer(&pack_set(&op.set));
                    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("animation stroke records"), layout: &self.layout_records, entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: buffer.as_entire_binding() }] });
                    self.stroke_sets.insert(id, StrokeSetEntry { bind });
                }
                self.stroke_sets[&id].bind.clone()
            }
        };
        let (tips_id, tips_view, tips) = match &op.tips {
            None => (0, self.blank_tips_view.clone(), self.no_tips.clone()),
            Some(t) => {
                if upload_tips {
                    let (w, h) = t.size();
                    let layers = t.len() as u32;
                    let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation brush tips"), size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: layers }, mip_level_count: t.mip_levels(), sample_count: 1, dimension: wgpu::TextureDimension::D2, format: TIP_FORMAT, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
                    for (level, l) in t.levels().iter().enumerate() {
                        queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: level as u32, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, &l.bytes, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(l.width * 2), rows_per_image: Some(l.height) }, wgpu::Extent3d { width: l.width, height: l.height, depth_or_array_layers: layers });
                    }
                    let view = texture.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..wgpu::TextureViewDescriptor::default() });
                    let bind = Gpu::tips_bind(device, &self.layout_tips, &view, &self.tip_sampler, &self.blank_source_view, &self.source_sampler);
                    self.tips.insert(t.id(), TipsEntry { view, bind });
                }
                let entry = &self.tips[&t.id()];
                (t.id(), entry.view.clone(), entry.bind.clone())
            }
        };
        // The colour source (none: the tips' own group, with the blank source).
        let source = match &op.source {
            StrokeSource::None => None,
            StrokeSource::Image(i) => Some(image_views.get(*i as usize).cloned().flatten().unwrap_or_else(|| self.blank_source_view.clone())),
            StrokeSource::Target(t) => Some(t.texture(device).view),
            StrokeSource::Texture(v) => Some(v.clone()),
        };
        let Some(source) = source else {
            return (records, tips);
        };
        let frame = self.frame;
        let (layout, sampler, source_sampler) = (&self.layout_tips, &self.tip_sampler, &self.source_sampler);
        let entry = self.source_groups.entry((tips_id, source.clone())).or_insert_with(|| (Gpu::tips_bind(device, layout, &tips_view, sampler, &source, source_sampler), frame));
        entry.1 = frame;
        (records, entry.0.clone())
    }

    /// The size changed: the targets are made again on use.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.size = (width, height);
        self.target_size = (0, 0);
        self.drop_targets();
    }

    /// The internal targets go (a new output size or a new target window size): made again on
    /// use, the checkpoints forgotten (a copy fits only a target of its own size and place).
    fn drop_targets(&mut self) {
        self.targets.clear();
        self.backdrop = None;
        self.fx_textures.clear();
        self.height = None;
        for slot in self.checkpoints.clear() {
            if let Some(t) = self.checkpoint_textures.get_mut(slot) {
                *t = None;
            }
        }
        self.checkpoint_textures.clear();
    }

    /// The frame's checkpoint plan of every stroke draw that asks for one (the CPU half decides;
    /// a checkpoint made this frame gets its texture now), counted into `stats`.
    fn plan_checkpoints(&mut self, device: &wgpu::Device, b: &Builder, stats: &mut crate::rendering::RenderStats) -> Vec<Option<CheckpointStep>> {
        let (w, h) = self.target_size;
        let bytes = StrokeCheckpoints::bytes_at(w.max(1), h.max(1));
        let mut steps = Vec::with_capacity(b.stroke_ops.len());
        for op in &b.stroke_ops {
            let Some(ask) = op.checkpoint else {
                steps.push(None);
                continue;
            };
            let mut step = self.checkpoints.step(ask.key, ask.whole.min(op.instances), ask.options, self.frame, bytes);
            // A slot without its texture cannot be restored (defensive: every held slot has one).
            if let Some((slot, _)) = step.restore {
                if !matches!(self.checkpoint_textures.get(slot), Some(Some(_))) {
                    step.restore = None;
                }
            }
            if let Some((slot, _)) = step.bake {
                if self.checkpoint_textures.len() <= slot {
                    self.checkpoint_textures.resize(slot + 1, None);
                }
                if self.checkpoint_textures[slot].is_none() {
                    self.checkpoint_textures[slot] = Some(self.ms_texture(device, COLOR, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST));
                }
                stats.checkpoint_bakes += 1;
            }
            match step.restore {
                Some((_, n)) => {
                    stats.checkpoint_hits += 1;
                    stats.strokes_restored += n as usize;
                    stats.strokes = stats.strokes.saturating_sub(n as usize);
                }
                None => stats.checkpoint_misses += 1,
            }
            steps.push(Some(step));
        }
        for slot in self.checkpoints.sweep(self.frame, |entries, frame| texture_ledger::sweep_choice(entries, frame, CHECKPOINT_RESIDENCY)) {
            if let Some(t) = self.checkpoint_textures.get_mut(slot) {
                *t = None;
            }
        }
        stats.checkpoints = self.checkpoints.held();
        stats.checkpoint_bytes = self.checkpoints.held_bytes();
        steps
    }

    /// A stroke draw's instances `range` in a pass of their own on the level's target (colour
    /// and stencil loaded and stored), inside the clip `depth`.
    #[allow(clippy::too_many_arguments)]
    fn stroke_pass(&self, encoder: &mut wgpu::CommandEncoder, level: usize, depth: u32, bind0: &wgpu::BindGroup, groups: &(wgpu::BindGroup, wgpu::BindGroup), offset: u32, range: Range<u32>, stamps: &PassStamps) -> bool {
        let Some(db) = self.draw_bind.as_ref() else {
            return false;
        };
        if range.end <= range.start {
            return false;
        }
        let t = &self.targets[level];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("animation stroke pass (checkpoint)"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &t.color_view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &t.stencil_view, depth_ops: None, stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }) }),
            timestamp_writes: stamps.writes(false),
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.strokes);
        pass.set_bind_group(0, bind0, &[]);
        pass.set_bind_group(1, &groups.1, &[]);
        pass.set_bind_group(2, &groups.0, &[]);
        pass.set_bind_group(3, db, &[offset]);
        pass.set_stencil_reference(depth << SHIFT);
        pass.draw(0..8, range);
        true
    }

    /// The height record's view, made (the target window's size) on first use.
    fn height_view(&mut self, device: &wgpu::Device) -> wgpu::TextureView {
        let (w, h) = self.target_size;
        self.height
            .get_or_insert_with(|| {
                let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation height record"), size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: HEIGHT_FORMAT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
                HeightTarget { view: texture.create_view(&wgpu::TextureViewDescriptor::default()) }
            })
            .view
            .clone()
    }

    /// The effect texture of a role and level, made on first use (the level's size: the target
    /// window divided by the level, rounded up).
    fn fx_texture(&mut self, device: &wgpu::Device, key: FxTexture) -> &FxEntry {
        let (w, h) = self.target_size;
        let layout = &self.layout_fx_tex;
        self.fx_textures.entry(key).or_insert_with(|| {
            let level = key.1.max(1);
            let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation effect texture"), size: wgpu::Extent3d { width: w.max(1).div_ceil(level), height: h.max(1).div_ceil(level), depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: FX_FORMAT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("animation effect source"), layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }] });
            FxEntry { view, bind }
        })
    }

    /// Uploads the frame's effect pass data, kernels and ripple records, makes the textures the
    /// passes use, and returns the composite bind group of each effect (`image_views`: the frame's
    /// pictures, for a water slot's height picture).
    fn prepare_effects(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, b: &Builder, image_views: &[Option<wgpu::TextureView>]) -> Vec<wgpu::BindGroup> {
        if b.effects.is_empty() {
            return Vec::new();
        }
        let pass_bytes = (b.fx_passes.len().max(1) * std::mem::size_of::<crate::rendering::core::build::FxPassData>()) as u64;
        let kernel_bytes = (b.fx_kernels.len() * 4).max(16) as u64;
        if self.fx_buffer.as_ref().map_or(true, |buf| buf.size() < pass_bytes) {
            self.fx_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation effect passes"), size: pass_bytes.next_power_of_two(), usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
            self.fx_bind = None;
        }
        if self.fx_kernel_buffer.as_ref().map_or(true, |buf| buf.size() < kernel_bytes) {
            self.fx_kernel_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation effect kernels"), size: kernel_bytes.next_power_of_two(), usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
            self.fx_bind = None;
        }
        let ripple_bytes = (b.ripples.len() * 16).max(16) as u64;
        if self.ripple_buffer.as_ref().map_or(true, |buf| buf.size() < ripple_bytes) {
            self.ripple_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation ripples"), size: ripple_bytes.next_power_of_two(), usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
            self.fx_bind = None;
        }
        if self.fx_bind.is_none() {
            self.fx_bind = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("animation effect passes"),
                layout: &self.layout_fx,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: self.fx_buffer.as_ref().unwrap().as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: self.fx_kernel_buffer.as_ref().unwrap().as_entire_binding() }, wgpu::BindGroupEntry { binding: 2, resource: self.ripple_buffer.as_ref().unwrap().as_entire_binding() }],
            }));
        }
        queue.write_buffer(self.fx_buffer.as_ref().unwrap(), 0, bytes(&b.fx_passes));
        if !b.fx_kernels.is_empty() {
            queue.write_buffer(self.fx_kernel_buffer.as_ref().unwrap(), 0, bytes(&b.fx_kernels));
        }
        if !b.ripples.is_empty() {
            queue.write_buffer(self.ripple_buffer.as_ref().unwrap(), 0, bytes(&b.ripples));
        }
        let mut groups = Vec::with_capacity(b.effects.len());
        for e in &b.effects {
            for step in &e.steps {
                match step {
                    FxStep::Resolve { .. } => {
                        self.fx_texture(device, (FxRole::Resolved, 1));
                    }
                    FxStep::Down { src, dst, .. } | FxStep::Blur { src, dst, .. } => {
                        self.fx_texture(device, *src);
                        self.fx_texture(device, *dst);
                    }
                }
            }
            let views: Vec<wgpu::TextureView> = e.reads.iter().map(|r| self.fx_texture(device, *r).view.clone()).collect();
            // A finished layer's composite reads the height record (the others a blank one).
            let height = if e.params.finish.is_some() { self.height_view(device) } else { self.blank_height_view.clone() };
            // The water's height pictures (a blank where a slot has none).
            let water = |i: usize| -> wgpu::TextureView {
                match e.water.as_ref().and_then(|w| w.textures[i].as_ref()) {
                    None => self.blank_source_view.clone(),
                    Some(WaterBinding::Image(k)) => image_views.get(*k as usize).cloned().flatten().unwrap_or_else(|| self.blank_source_view.clone()),
                    Some(WaterBinding::Target(t)) => t.texture(device).view,
                    Some(WaterBinding::Texture(v)) => v.clone(),
                }
            };
            let (water_a, water_b) = (water(0), water(1));
            groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("animation effect composite"),
                layout: &self.layout_fx_comp,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&views[0]) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&views[1]) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&views[2]) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.linear) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&height) },
                    wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&water_a) },
                    wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&water_b) },
                ],
            }));
        }
        groups
    }

    fn ms_texture(&self, device: &wgpu::Device, format: wgpu::TextureFormat, usage: wgpu::TextureUsages) -> wgpu::Texture {
        let (w, h) = self.target_size;
        device.create_texture(&wgpu::TextureDescriptor { label: Some("animation target"), size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: SAMPLES, dimension: wgpu::TextureDimension::D2, format, usage, view_formats: &[] })
    }

    fn ms_bind(&self, device: &wgpu::Device, view: &wgpu::TextureView) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("animation layer texture"), layout: &self.layout_ms, entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) }] })
    }

    fn ensure_targets(&mut self, device: &wgpu::Device, levels: usize, backdrop: bool) {
        while self.targets.len() < levels {
            // COPY_DST: a stroke draw's checkpoint is copied back into the target.
            let color = self.ms_texture(device, COLOR, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST);
            let stencil = self.ms_texture(device, STENCIL, wgpu::TextureUsages::RENDER_ATTACHMENT);
            let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
            let stencil_view = stencil.create_view(&wgpu::TextureViewDescriptor::default());
            let bind = self.ms_bind(device, &color_view);
            self.targets.push(Target { color, color_view, stencil_view, bind });
        }
        if backdrop && self.backdrop.is_none() {
            // WebGPU requires RENDER_ATTACHMENT on every multisampled texture.
            let texture = self.ms_texture(device, COLOR, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind = self.ms_bind(device, &view);
            self.backdrop = Some(Backdrop { texture, bind });
        }
    }

    /// The bind group of an image, its texture made on first use (straight or premultiplied
    /// sRGB-encoded bytes, kept as they are; BGRA is swapped to RGBA once). An ownership map's
    /// texture keeps its owner numbers: a draw with another colour table writes only those rows
    /// (the ledger decides; the uploads are counted into `stats`).
    fn image_group(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, image: &FrameImage, stats: &mut crate::rendering::RenderStats) -> Option<(wgpu::BindGroup, wgpu::TextureView)> {
        let data = &image.brush.image;
        // A picture target's stand-in: its texture as it is (nothing uploaded).
        if let Some(target) = PictureTarget::find(data.data.id()) {
            let view = target.texture(device).view;
            return Some((Gpu::image_bind(device, &self.layout_image, &view, &self.linear, &self.nearest), view));
        }
        // A live picture's stand-in: its own texture on this device, the regions queued for it
        // written first (counted as live-picture writes; the ledger never holds it).
        if let Some(live) = LivePicture::find(data.data.id()) {
            let (view, written) = live.texture_view(device, queue);
            texture_ledger::count_live(stats, &written);
            return Some((Gpu::image_bind(device, &self.layout_image, &view, &self.linear, &self.nearest), view));
        }
        let (w, h) = (data.width, data.height);
        if w == 0 || h == 0 || data.data.data().len() < (w * h * 4) as usize {
            return None;
        }
        let key = image.key();
        let (slot, mut upload) = self.ledger.acquire_bytes(key, image.palette_id(), self.frame, image.whole_bytes());
        if upload != Upload::Whole && !self.images.contains_key(&(key, slot)) {
            upload = Upload::Whole;
        }
        texture_ledger::count(stats, image, upload);
        let row = image.palette_row.min(h);
        // The colour-table rows the draw wants, as RGBA bytes (its own table, or the picture's).
        let table = || -> Vec<u8> {
            match &image.palette {
                Some(p) if p.width == w && p.height == h - row && p.data.data().len() >= (w * (h - row) * 4) as usize => rgba(p, (w * (h - row) * 4) as usize, 0),
                _ => rgba(data, (w * (h - row) * 4) as usize, (w * row * 4) as usize),
            }
        };
        match upload {
            Upload::None => self.images.get(&(key, slot)).map(|e| (e.bind.clone(), e.view.clone())),
            Upload::ColorTable => {
                let entry = self.images.get(&(key, slot))?;
                if row < h {
                    let rows = table();
                    queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &entry.texture, mip_level: 0, origin: wgpu::Origin3d { x: 0, y: row, z: 0 }, aspect: wgpu::TextureAspect::All }, &rows, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h - row) }, wgpu::Extent3d { width: w, height: h - row, depth_or_array_layers: 1 });
                }
                Some((entry.bind.clone(), entry.view.clone()))
            }
            Upload::Whole => {
                let mut pixels = rgba(data, (w * h * 4) as usize, 0);
                if image.palette.is_some() && row < h {
                    let rows = table();
                    pixels[(w * row * 4) as usize..].copy_from_slice(&rows);
                }
                let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation image"), size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
                queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, &pixels, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) }, wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let bind = Gpu::image_bind(device, &self.layout_image, &view, &self.linear, &self.nearest);
                self.images.insert((key, slot), ImageEntry { texture, view: view.clone(), bind: bind.clone() });
                Some((bind, view))
            }
        }
    }

    /// Records the frame's operations into the frame's encoder - onto the frame's view, or into
    /// `picture` (a picture target's layer: the picture's size, cleared and drawn whole). Returns
    /// (draw calls, passes). `frame` is a canvas frame's output or an offscreen one's
    /// ([`FrameOutput`]).
    pub(crate) fn encode(&mut self, frame: &mut FrameOutput<'_>, b: &mut Builder, read: bool, picture: Option<&PictureTarget>) -> (usize, usize) {
        self.frame += 1;
        let device = frame.context.device();
        let queue = frame.context.queue();
        let (w, h) = picture.map_or((frame.width, frame.height), |p| p.size());
        if self.size != (w, h) {
            self.resize(w, h);
        }
        // The frame's target window: the internal targets are made again when its size changes.
        let window: TargetWindow = b.window;
        if self.target_size != window.size() {
            self.drop_targets();
            self.target_size = window.size();
        }
        let (tw, th) = self.target_size;
        // GPU timing: what has arrived, and the query set of this frame (None: untimed).
        let stamps = PassStamps::new(self.timing.begin_frame(device, queue));
        let needs_backdrop = b.ops.iter().any(|o| matches!(o, Op::LayerEnd { mode: CompositeMode::Mix, .. }));
        self.ensure_targets(device, b.levels_needed, needs_backdrop);
        // Images: one bind group per image used this frame (index 0 = none).
        let mut image_groups = vec![self.no_image.clone()];
        let mut image_views: Vec<Option<wgpu::TextureView>> = vec![None];
        let mut stats = b.stats;
        stats.gpu_ms = self.timing.latest_ms();
        for image in b.images.clone() {
            match self.image_group(device, queue, &image, &mut stats) {
                Some((group, view)) => {
                    image_groups.push(group);
                    image_views.push(Some(view));
                }
                None => {
                    image_groups.push(self.no_image.clone());
                    image_views.push(None);
                }
            }
        }
        // Stroke sets: the records and tips of each draw (uploaded the first time), and the
        // frame's draw blocks in one buffer read at dynamic offsets.
        let mut stroke_groups = Vec::with_capacity(b.stroke_ops.len());
        for op in b.stroke_ops.clone() {
            stroke_groups.push(self.stroke_groups(device, queue, &op, &image_views, &mut stats));
        }
        if !b.stroke_ops.is_empty() {
            let need = (b.stroke_blocks.len() * 4 + DRAW_BINDING_BYTES) as u64;
            if self.draw_buffer.as_ref().map_or(true, |buf| buf.size() < need) {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation stroke draws"), size: need.next_power_of_two(), usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
                self.draw_bind = Some(device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("animation stroke draws"), layout: &self.layout_draw, entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &buffer, offset: 0, size: wgpu::BufferSize::new(DRAW_BINDING_BYTES as u64) }) }] }));
                self.draw_buffer = Some(buffer);
            }
            queue.write_buffer(self.draw_buffer.as_ref().unwrap(), 0, bytes(&b.stroke_blocks));
        }
        // The checkpoint plans (restores, bakes) of the draws that ask for them.
        let steps = self.plan_checkpoints(device, b, &mut stats);
        b.stats = stats;
        // What is no longer drawn goes (one policy for pictures, stroke sets and tips).
        for gone in self.ledger.sweep(self.frame, RESIDENCY) {
            self.images.remove(&gone);
        }
        for gone in self.set_residency.sweep(self.frame, RESIDENCY) {
            self.stroke_sets.remove(&gone);
        }
        for gone in self.morph_residency.sweep(self.frame, RESIDENCY) {
            self.morph_sets.remove(&gone);
        }
        for gone in self.tip_residency.sweep(self.frame, RESIDENCY) {
            self.tips.remove(&gone);
        }
        // Texture groups of colour sources not used this frame or the last go (a view the
        // application drops is released then).
        let frame_now = self.frame;
        self.source_groups.retain(|_, (_, used)| frame_now - *used <= 1);
        // Uploads: the view, the vertices (stencil list, then the cover list), paints, new ramps.
        queue.write_buffer(&self.view_buffer, 0, bytes(&window.view_words((w, h))));
        for (slot, samples) in b.ramps.uploads.drain(..) {
            queue.write_buffer(&self.ramp_buffer, (slot as usize * RAMP_SAMPLES * 16) as u64, bytes(&samples));
        }
        let stencil_count = b.stencil.len() as u32;
        let vbytes = ((b.stencil.len() + b.cover.len()) * 12).max(12) as u64;
        if self.vertex_buffer.as_ref().map_or(true, |buf| buf.size() < vbytes) {
            self.vertex_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation vertices"), size: vbytes.next_power_of_two(), usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
        }
        let pbytes = (b.paints.len() * std::mem::size_of::<PaintData>()).max(80) as u64;
        if self.paint_buffer.as_ref().map_or(true, |buf| buf.size() < pbytes) {
            self.paint_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation paints"), size: pbytes.next_power_of_two(), usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
            self.bind0 = None;
        }
        if self.bind0.is_none() {
            self.bind0 = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("animation frame"),
                layout: &self.layout0,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.view_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: self.paint_buffer.as_ref().unwrap().as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: self.ramp_buffer.as_entire_binding() },
                ],
            }));
        }
        let vb = self.vertex_buffer.clone().unwrap();
        if !b.stencil.is_empty() {
            queue.write_buffer(&vb, 0, bytes(&b.stencil));
        }
        if !b.cover.is_empty() {
            queue.write_buffer(&vb, stencil_count as u64 * 12, bytes(&b.cover));
        }
        queue.write_buffer(self.paint_buffer.as_ref().unwrap(), 0, bytes(&b.paints));
        let bind0 = self.bind0.clone().unwrap();
        let cv = |r: &Range<u32>| r.start + stencil_count..r.end + stencil_count;
        let fx_groups = self.prepare_effects(device, queue, b, &image_views);
        // The particle instances of the frame.
        if !b.particles.is_empty() {
            let pbytes = (b.particles.len() * std::mem::size_of::<ParticleInstance>()) as u64;
            if self.particle_buffer.as_ref().map_or(true, |buf| buf.size() < pbytes) {
                self.particle_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation particles"), size: pbytes.next_power_of_two(), usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
            }
            queue.write_buffer(self.particle_buffer.as_ref().unwrap(), 0, bytes(&b.particles));
        }
        let particle_buffer = self.particle_buffer.clone();
        let mut particle_draws = 0usize;
        let mut stroke_draws = 0usize;

        let mut draw_calls = 0usize;
        let mut passes = 0usize;
        let mut i = 0usize;
        let mut saved: Vec<u32> = Vec::new();
        let mut level = 0usize;
        let mut depth = 0u32;
        let mut clear = true;
        let mut clear_quad = b.root_clear.clone();
        // True when the level's pass goes on after a stroke draw that left it (a checkpoint).
        let mut resume = false;
        loop {
            let resuming = std::mem::replace(&mut resume, false);
            let mut split: Option<u32> = None;
            // A group with a mix mode is composited next: copy the target below first.
            if !clear && !resuming {
                if let Some(Op::LayerEnd { mode: CompositeMode::Mix, .. }) = b.ops.get(i) {
                    if let Some(bd) = self.backdrop.as_ref() {
                        let size = wgpu::Extent3d { width: tw.max(1), height: th.max(1), depth_or_array_layers: 1 };
                        frame.encoder.copy_texture_to_texture(self.targets[level].color.as_image_copy(), bd.texture.as_image_copy(), size);
                    }
                }
                // A group with effects is composited next: its offscreen passes first.
                if let Some(Op::LayerEnd { mode: CompositeMode::Effect(k), .. }) = b.ops.get(i) {
                    // A finished layer: its stroke sets' height, drawn again into the cleared
                    // height record (single sample, no stencil), which the composite reads.
                    if b.effects[*k as usize].params.finish.is_some() {
                        let view = self.height_view(device);
                        let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("animation height pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
                            depth_stencil_attachment: None,
                            timestamp_writes: stamps.writes(false),
                            occlusion_query_set: None,
                            multiview_mask: None,
                        });
                        passes += 1;
                        pass.set_pipeline(&self.stroke_height);
                        pass.set_bind_group(0, &bind0, &[]);
                        if let Some(db) = self.draw_bind.as_ref() {
                            for d in &b.effects[*k as usize].height_draws {
                                let op = &b.stroke_ops[*d as usize];
                                let (records, textures) = &stroke_groups[*d as usize];
                                if op.instances > 0 {
                                    pass.set_bind_group(1, textures, &[]);
                                    pass.set_bind_group(2, records, &[]);
                                    pass.set_bind_group(3, db, &[op.offset]);
                                    pass.draw(0..8, 0..op.instances);
                                    draw_calls += 1;
                                }
                            }
                        }
                    }
                    let fx_bind = self.fx_bind.clone().unwrap();
                    for step in &b.effects[*k as usize].steps {
                        let (dst, pipeline, source, quad) = match step {
                            FxStep::Resolve { quad } => ((FxRole::Resolved, 1), &self.fx_resolve, self.targets[level + 1].bind.clone(), quad),
                            FxStep::Down { src, dst, quad } => (*dst, &self.fx_down, self.fx_textures[src].bind.clone(), quad),
                            FxStep::Blur { src, dst, quad } => (*dst, &self.fx_blur, self.fx_textures[src].bind.clone(), quad),
                        };
                        let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("animation effect pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &self.fx_textures[&dst].view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                            depth_stencil_attachment: None,
                            timestamp_writes: stamps.writes(false),
                            occlusion_query_set: None,
                            multiview_mask: None,
                        });
                        passes += 1;
                        pass.set_vertex_buffer(0, vb.slice(..));
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(0, &bind0, &[]);
                        pass.set_bind_group(1, &source, &[]);
                        pass.set_bind_group(2, &fx_bind, &[]);
                        let r = cv(quad);
                        if r.end > r.start {
                            pass.draw(r, 0..1);
                            draw_calls += 1;
                        }
                    }
                }
            }
            {
                let t = &self.targets[level];
                let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("animation pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &t.color_view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &t.stencil_view, depth_ops: None, stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }) }),
                    timestamp_writes: stamps.writes(false),
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                passes += 1;
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_bind_group(0, &bind0, &[]);
                pass.set_bind_group(1, &self.no_image, &[]);
                let mut draw = |pass: &mut wgpu::RenderPass<'_>, r: Range<u32>| {
                    if r.end > r.start {
                        pass.draw(r, 0..1);
                        draw_calls += 1;
                    }
                };
                if resuming {
                    // Back in the level's pass after a checkpointed stroke draw.
                } else if clear {
                    // A target used for the first time this frame (or again for a new group):
                    // cleared, colour and stencil, where it will be used.
                    pass.set_pipeline(&self.clear);
                    pass.set_stencil_reference(0);
                    draw(&mut pass, cv(&clear_quad));
                } else if let Some(Op::LayerEnd { mode, clip, quad }) = b.ops.get(i) {
                    // Back from a group: composite it, inside its shape.
                    let mut d = depth;
                    if let Some((stencil, cover)) = clip {
                        pass.set_pipeline(&self.stencil);
                        pass.set_stencil_reference(d << SHIFT);
                        draw(&mut pass, stencil.clone());
                        pass.set_pipeline(&self.clip_push[0]);
                        pass.set_stencil_reference((d + 1) << SHIFT);
                        draw(&mut pass, cv(cover));
                        d += 1;
                    }
                    match mode {
                        CompositeMode::Compose(c) => {
                            pass.set_pipeline(&self.composite[*c as usize]);
                            pass.set_bind_group(1, &self.targets[level + 1].bind, &[]);
                        }
                        CompositeMode::Mix => {
                            pass.set_pipeline(&self.blend);
                            pass.set_bind_group(1, &self.targets[level + 1].bind, &[]);
                            if let Some(bd) = self.backdrop.as_ref() {
                                pass.set_bind_group(2, &bd.bind, &[]);
                            }
                        }
                        CompositeMode::Effect(k) => {
                            pass.set_pipeline(&self.fx_composite);
                            pass.set_bind_group(1, &fx_groups[*k as usize], &[]);
                            if let Some(fx) = self.fx_bind.as_ref() {
                                pass.set_bind_group(2, fx, &[]);
                            }
                        }
                    }
                    pass.set_stencil_reference(d << SHIFT);
                    if !(matches!(mode, CompositeMode::Mix) && self.backdrop.is_none()) {
                        draw(&mut pass, cv(quad));
                    }
                    // Back to the paint pipelines' bind group before anything else draws.
                    pass.set_bind_group(1, &self.no_image, &[]);
                    if let Some((_, cover)) = clip {
                        pass.set_pipeline(&self.clip_pop);
                        pass.set_stencil_reference(d << SHIFT);
                        draw(&mut pass, cv(cover));
                    }
                    i += 1;
                }
                let mut bound_image = 0u32;
                while i < b.ops.len() {
                    match &b.ops[i] {
                        Op::Fill { stencil, cover, even_odd, image } => {
                            pass.set_pipeline(&self.stencil);
                            pass.set_stencil_reference(depth << SHIFT);
                            draw(&mut pass, stencil.clone());
                            pass.set_pipeline(&self.cover[*even_odd as usize]);
                            if *image != bound_image {
                                pass.set_bind_group(1, &image_groups[*image as usize], &[]);
                                bound_image = *image;
                            }
                            pass.set_stencil_reference(0);
                            draw(&mut pass, cv(cover));
                        }
                        Op::ClipPush { stencil, cover, even_odd } => {
                            pass.set_pipeline(&self.stencil);
                            pass.set_stencil_reference(depth << SHIFT);
                            draw(&mut pass, stencil.clone());
                            pass.set_pipeline(&self.clip_push[*even_odd as usize]);
                            pass.set_stencil_reference((depth + 1) << SHIFT);
                            draw(&mut pass, cv(cover));
                            if *even_odd {
                                // Even windings that did not raise the depth are cleared.
                                pass.set_pipeline(&self.wind_reset);
                                pass.set_stencil_reference(depth << SHIFT);
                                draw(&mut pass, cv(cover));
                            }
                            depth += 1;
                        }
                        Op::ClipPop { cover } => {
                            pass.set_pipeline(&self.clip_pop);
                            pass.set_stencil_reference(depth << SHIFT);
                            draw(&mut pass, cv(cover));
                            depth = depth.saturating_sub(1);
                        }
                        Op::Particles { instances } => {
                            if let Some(pb) = particle_buffer.as_ref() {
                                pass.set_pipeline(&self.particles);
                                pass.set_vertex_buffer(0, pb.slice(..));
                                pass.set_stencil_reference(depth << SHIFT);
                                if instances.end > instances.start {
                                    pass.draw(0..6, instances.clone());
                                    particle_draws += 1;
                                }
                                pass.set_vertex_buffer(0, vb.slice(..));
                            }
                        }
                        Op::Strokes { draw } => {
                            // A draw with a checkpoint plan is drawn outside this pass (below).
                            if steps[*draw as usize].is_some_and(|s| s.splits()) {
                                split = Some(*draw);
                                break;
                            }
                            let op = &b.stroke_ops[*draw as usize];
                            let (records, tips) = &stroke_groups[*draw as usize];
                            if let (Some(db), true) = (self.draw_bind.as_ref(), op.instances > 0) {
                                pass.set_pipeline(&self.strokes);
                                pass.set_bind_group(1, tips, &[]);
                                pass.set_bind_group(2, records, &[]);
                                pass.set_bind_group(3, db, &[op.offset]);
                                pass.set_stencil_reference(depth << SHIFT);
                                pass.draw(0..8, 0..op.instances);
                                stroke_draws += 1;
                                // Back to the paint pipelines' picture group.
                                pass.set_bind_group(1, &self.no_image, &[]);
                                bound_image = 0;
                            }
                        }
                        Op::LayerBegin { .. } | Op::LayerEnd { .. } => break,
                    }
                    i += 1;
                }
            }
            if let Some(d) = split {
                // The checkpointed draw: the held checkpoint copied into the level's target, the
                // strokes up to the new checkpoint, the target copied into it, the rest - each
                // stroke range in a pass of its own; then the level's pass goes on.
                let op = &b.stroke_ops[d as usize];
                let step = steps[d as usize].unwrap_or_default();
                let size = wgpu::Extent3d { width: tw.max(1), height: th.max(1), depth_or_array_layers: 1 };
                if let Some(cp) = step.restore.and_then(|(slot, _)| self.checkpoint_textures.get(slot).cloned().flatten()) {
                    frame.encoder.copy_texture_to_texture(cp.as_image_copy(), self.targets[level].color.as_image_copy(), size);
                }
                let groups = &stroke_groups[d as usize];
                let mut copied = false;
                for range in step.ranges(op.instances) {
                    if let Some((slot, n)) = step.bake {
                        if !copied && range.start >= n {
                            if let Some(cp) = self.checkpoint_textures.get(slot).cloned().flatten() {
                                frame.encoder.copy_texture_to_texture(self.targets[level].color.as_image_copy(), cp.as_image_copy(), size);
                            }
                            copied = true;
                        }
                    }
                    if self.stroke_pass(frame.encoder, level, depth, &bind0, groups, op.offset, range, &stamps) {
                        passes += 1;
                        draw_calls += 1;
                    }
                }
                if let (Some((slot, _)), false) = (step.bake, copied) {
                    if let Some(cp) = self.checkpoint_textures.get(slot).cloned().flatten() {
                        frame.encoder.copy_texture_to_texture(self.targets[level].color.as_image_copy(), cp.as_image_copy(), size);
                    }
                }
                i += 1;
                clear = false;
                resume = true;
                continue;
            }
            if i >= b.ops.len() {
                break;
            }
            match &b.ops[i] {
                Op::LayerBegin { clear: c } => {
                    clear_quad = c.clone();
                    saved.push(depth);
                    level += 1;
                    depth = 0;
                    clear = true;
                    i += 1;
                }
                _ => {
                    level = level.saturating_sub(1);
                    depth = saved.pop().unwrap_or(0);
                    clear = false;
                }
            }
        }
        // The main 2D target onto the host's view (and, when asked, into the readback texture) -
        // or into the picture target, cleared first (the picture is the layer's whole drawing).
        let final_quad = cv(&b.final_quad);
        if let Some(target) = picture {
            let view = target.texture(device).view;
            let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("animation picture target"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: stamps.writes(!read),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            passes += 1;
            if final_quad.end > final_quad.start {
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_bind_group(0, &bind0, &[]);
                pass.set_pipeline(&self.picture_final);
                pass.set_bind_group(1, &self.targets[0].bind, &[]);
                pass.draw(final_quad.clone(), 0..1);
                draw_calls += 1;
            }
        } else if final_quad.end > final_quad.start {
            let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("animation final"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: frame.view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: stamps.writes(!read),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            passes += 1;
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_bind_group(0, &bind0, &[]);
            pass.set_pipeline(&self.final_view);
            pass.set_bind_group(1, &self.targets[0].bind, &[]);
            pass.draw(final_quad.clone(), 0..1);
            draw_calls += 1;
        }
        if read {
            self.encode_readback(frame, &vb, &bind0, final_quad, &stamps);
        }
        // A timed frame whose last pass did not end the timing (nothing reached the view): an
        // empty pass on the main target writes the end.
        if stamps.needs_end() {
            let t = &self.targets[0];
            let _pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("animation gpu timing end"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &t.color_view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: stamps.writes(true),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            passes += 1;
        }
        self.timing.end_frame(frame.encoder);
        (draw_calls + particle_draws + stroke_draws, passes)
    }

    fn encode_readback(&mut self, frame: &mut FrameOutput<'_>, vb: &wgpu::Buffer, bind0: &wgpu::BindGroup, quad: Range<u32>, stamps: &PassStamps) {
        let device = frame.context.device();
        let (w, h) = self.size;
        if self.readback.as_ref().map_or(true, |r| r.width != w || r.height != h) {
            let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation readback"), size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: COLOR, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[] });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let padded_row = (w.max(1) * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation readback"), size: (padded_row * h.max(1)) as u64, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            self.readback = Some(Readback { texture, view, buffer, width: w, height: h, padded_row });
        }
        let r = self.readback.as_ref().unwrap();
        {
            let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("animation readback"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &r.view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: stamps.writes(true),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if quad.end > quad.start {
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_bind_group(0, bind0, &[]);
                pass.set_pipeline(&self.final_readback);
                pass.set_bind_group(1, &self.targets[0].bind, &[]);
                pass.draw(quad, 0..1);
            }
        }
        frame.encoder.copy_texture_to_buffer(
            r.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo { buffer: &r.buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(r.padded_row), rows_per_image: Some(r.height.max(1)) } },
            wgpu::Extent3d { width: r.width.max(1), height: r.height.max(1), depth_or_array_layers: 1 },
        );
    }

    /// Outside a browser build this testing readback hands `done` NO pixels (an empty list, 0 x
    /// 0): it serves only AnimationLayerControl::read_pixels_for_testing, whose layer draws on a
    /// browser canvas. A native program draws with OffscreenRenderer into a texture of its own
    /// and reads that texture itself (a copy into a buffer, then `device.poll`), never through
    /// this.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn map_readback(&self, done: Box<dyn FnOnce(Vec<u8>, u32, u32)>) {
        // Not mapped here: a native device would need polling, and nothing native asks for it.
        done(Vec::new(), 0, 0);
    }

    /// Maps the readback buffer (after the frame was submitted) and hands the 2D layer's own
    /// pixels, straight RGBA, to `done`.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn map_readback(&self, done: Box<dyn FnOnce(Vec<u8>, u32, u32)>) {
        let Some(r) = self.readback.as_ref() else {
            return;
        };
        let buffer = r.buffer.clone();
        let mapped = buffer.clone();
        let (width, height, padded) = (r.width, r.height, r.padded_row);
        buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            let mut pixels = Vec::new();
            if result.is_ok() {
                if let Ok(view) = mapped.get_mapped_range(..) {
                    pixels.reserve((width * height * 4) as usize);
                    for row in 0..height as usize {
                        let start = row * padded as usize;
                        for px in view[start..start + width as usize * 4].chunks_exact(4) {
                            // The target holds linear premultiplied colour, sRGB-encoded on
                            // write: decode, un-premultiply, encode again (straight sRGB).
                            let a = px[3] as f32 / 255.0;
                            let un = |c: u8| if a > 0.0 { (linear_to_srgb((crate::rendering::core::paint::srgb_to_linear(c as f32 / 255.0) / a).min(1.0)) * 255.0).round() as u8 } else { 0 };
                            pixels.extend_from_slice(&[un(px[0]), un(px[1]), un(px[2]), px[3]]);
                        }
                    }
                }
                mapped.unmap();
            }
            done(pixels, width, height);
        });
    }
}

/// `len` bytes of a picture from `offset`, as RGBA (BGRA swapped).
fn rgba(data: &ImageData, len: usize, offset: usize) -> Vec<u8> {
    let mut pixels = data.data.data()[offset..offset + len].to_vec();
    if data.format == ImageFormat::Bgra8 {
        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
    }
    pixels
}
