//! The GPU half of the renderer: pipelines, the offscreen targets, the buffers, the image
//! textures, and the encoding of one frame's operations into render passes of the frame's
//! shared encoder. It draws OVER what the layers below drew and never clears the host's view.
//! Groups with effects take extra passes into float textures (effects.wgsl): a resolve, box
//! filters for wide blurs, separable blur passes, then their composite into the target below.

use std::collections::HashMap;
use std::ops::Range;

use codebrix_webgpu::{wgpu, CanvasFrame, CanvasTarget};
use peniko::{ImageBrush, ImageFormat};

use crate::rendering::core::build::{Builder, CompositeMode, FxRole, FxStep, FxTexture, Op, ParticleInstance};
use crate::rendering::core::paint::{PaintData, RAMP_SAMPLES, RAMP_SLOTS};

const SAMPLES: u32 = 4;
const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const STENCIL: wgpu::TextureFormat = wgpu::TextureFormat::Stencil8;
/// The effect textures: linear premultiplied colour in half floats (filterable everywhere).
const FX_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const WIND: u32 = 0x1F;
const CLIP: u32 = 0xE0;
const SHIFT: u32 = 5;
/// Frames an image texture may go unused before it is dropped.
const IMAGE_KEEP_FRAMES: u64 = 120;

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

/// The shared head of the effect shaders and one part (the text after "//@part <name>").
fn fx_source(part: &str) -> String {
    let all = include_str!("effects.wgsl");
    let mut pieces = all.split("//@part ");
    let head = pieces.next().unwrap_or_default();
    let body = pieces.find(|p| p.starts_with(part)).unwrap_or_default();
    let body = body.split_once('\n').map_or("", |(_, rest)| rest);
    format!("{}{}{}", include_str!("common.wgsl"), head, body)
}

struct ImageEntry {
    bind: wgpu::BindGroup,
    last_used: u64,
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
    layout_fx: wgpu::BindGroupLayout,
    layout_fx_tex: wgpu::BindGroupLayout,
    layout_fx_comp: wgpu::BindGroupLayout,
    fx_textures: HashMap<FxTexture, FxEntry>,
    fx_buffer: Option<wgpu::Buffer>,
    fx_kernel_buffer: Option<wgpu::Buffer>,
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
    images: HashMap<(u64, u32), ImageEntry>,
    targets: Vec<Target>,
    backdrop: Option<Backdrop>,
    pub(crate) size: (u32, u32),
    frame: u64,
    pub(crate) readback: Option<Readback>,
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
        let paint_module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("animation paint"), source: wgpu::ShaderSource::Wgsl(concat!(include_str!("common.wgsl"), include_str!("paint.wgsl")).into()) });
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
            entries: &[buffer_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT, wgpu::BufferBindingType::Storage { read_only: true }), buffer_entry(1, wgpu::ShaderStages::FRAGMENT, wgpu::BufferBindingType::Storage { read_only: true })],
        });
        let layout_fx_tex = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("animation effect source"), entries: &[texture_entry(0)] });
        let layout_fx_comp = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("animation effect composite"),
            entries: &[texture_entry(0), texture_entry(1), texture_entry(2), wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None }],
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
        let final_view = make("animation final", &pl_composite, cm, "fs_final", target.format, premul, all, None, 1);
        let final_readback = make("animation final (readback)", &pl_composite, cm, "fs_final", COLOR, premul, all, None, 1);
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
        let view_buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation view"), size: 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
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
            layout_fx,
            layout_fx_tex,
            layout_fx_comp,
            fx_textures: HashMap::new(),
            fx_buffer: None,
            fx_kernel_buffer: None,
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
            targets: Vec::new(),
            backdrop: None,
            size: (0, 0),
            frame: 0,
            readback: None,
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

    /// The size changed: the targets are made again on use.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.size = (width, height);
        self.targets.clear();
        self.backdrop = None;
        self.fx_textures.clear();
    }

    /// The effect texture of a role and level, made on first use (the level's size: the canvas
    /// divided by the level, rounded up).
    fn fx_texture(&mut self, device: &wgpu::Device, key: FxTexture) -> &FxEntry {
        let (w, h) = self.size;
        let layout = &self.layout_fx_tex;
        self.fx_textures.entry(key).or_insert_with(|| {
            let level = key.1.max(1);
            let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation effect texture"), size: wgpu::Extent3d { width: w.max(1).div_ceil(level), height: h.max(1).div_ceil(level), depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: FX_FORMAT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("animation effect source"), layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }] });
            FxEntry { view, bind }
        })
    }

    /// Uploads the frame's effect pass data and kernels, makes the textures the passes use, and
    /// returns the composite bind group of each effect.
    fn prepare_effects(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, b: &Builder) -> Vec<wgpu::BindGroup> {
        if b.effects.is_empty() {
            return Vec::new();
        }
        let pass_bytes = (b.fx_passes.len() * std::mem::size_of::<crate::rendering::core::build::FxPassData>()).max(192) as u64;
        let kernel_bytes = (b.fx_kernels.len() * 4).max(16) as u64;
        if self.fx_buffer.as_ref().map_or(true, |buf| buf.size() < pass_bytes) {
            self.fx_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation effect passes"), size: pass_bytes.next_power_of_two(), usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
            self.fx_bind = None;
        }
        if self.fx_kernel_buffer.as_ref().map_or(true, |buf| buf.size() < kernel_bytes) {
            self.fx_kernel_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor { label: Some("animation effect kernels"), size: kernel_bytes.next_power_of_two(), usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
            self.fx_bind = None;
        }
        if self.fx_bind.is_none() {
            self.fx_bind = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("animation effect passes"),
                layout: &self.layout_fx,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: self.fx_buffer.as_ref().unwrap().as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: self.fx_kernel_buffer.as_ref().unwrap().as_entire_binding() }],
            }));
        }
        queue.write_buffer(self.fx_buffer.as_ref().unwrap(), 0, bytes(&b.fx_passes));
        if !b.fx_kernels.is_empty() {
            queue.write_buffer(self.fx_kernel_buffer.as_ref().unwrap(), 0, bytes(&b.fx_kernels));
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
            groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("animation effect composite"),
                layout: &self.layout_fx_comp,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&views[0]) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&views[1]) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&views[2]) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.linear) },
                ],
            }));
        }
        groups
    }

    fn ms_texture(&self, device: &wgpu::Device, format: wgpu::TextureFormat, usage: wgpu::TextureUsages) -> wgpu::Texture {
        let (w, h) = self.size;
        device.create_texture(&wgpu::TextureDescriptor { label: Some("animation target"), size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: SAMPLES, dimension: wgpu::TextureDimension::D2, format, usage, view_formats: &[] })
    }

    fn ms_bind(&self, device: &wgpu::Device, view: &wgpu::TextureView) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("animation layer texture"), layout: &self.layout_ms, entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) }] })
    }

    fn ensure_targets(&mut self, device: &wgpu::Device, levels: usize, backdrop: bool) {
        while self.targets.len() < levels {
            let color = self.ms_texture(device, COLOR, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC);
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
    /// sRGB-encoded bytes, kept as they are; BGRA is swapped to RGBA once).
    fn image_group(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, image: &ImageBrush) -> Option<wgpu::BindGroup> {
        let data = &image.image;
        let key = (data.data.id(), data.width);
        let frame = self.frame;
        if let Some(entry) = self.images.get_mut(&key) {
            entry.last_used = frame;
            return Some(entry.bind.clone());
        }
        let (w, h) = (data.width, data.height);
        if w == 0 || h == 0 || data.data.data().len() < (w * h * 4) as usize {
            return None;
        }
        let mut pixels = data.data.data()[..(w * h * 4) as usize].to_vec();
        if data.format == ImageFormat::Bgra8 {
            for px in pixels.chunks_exact_mut(4) {
                px.swap(0, 2);
            }
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("animation image"), size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, &pixels, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) }, wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind = Gpu::image_bind(device, &self.layout_image, &view, &self.linear, &self.nearest);
        self.images.insert(key, ImageEntry { bind: bind.clone(), last_used: frame });
        Some(bind)
    }

    /// Records the frame's operations into the frame's encoder. Returns (draw calls, passes).
    pub(crate) fn encode(&mut self, frame: &mut CanvasFrame<'_>, b: &mut Builder, read: bool) -> (usize, usize) {
        self.frame += 1;
        let device = frame.context.device();
        let queue = frame.context.queue();
        let (w, h) = (frame.info.width, frame.info.height);
        if self.size != (w, h) {
            self.resize(w, h);
        }
        let needs_backdrop = b.ops.iter().any(|o| matches!(o, Op::LayerEnd { mode: CompositeMode::Mix, .. }));
        self.ensure_targets(device, b.levels_needed, needs_backdrop);
        // Images: one bind group per image used this frame (index 0 = none).
        let mut image_groups = vec![self.no_image.clone()];
        for image in b.images.clone() {
            let group = self.image_group(device, queue, &image).unwrap_or_else(|| self.no_image.clone());
            image_groups.push(group);
        }
        if self.frame % 60 == 0 {
            let f = self.frame;
            self.images.retain(|_, e| f - e.last_used <= IMAGE_KEEP_FRAMES);
        }
        // Uploads: the view, the vertices (stencil list, then the cover list), paints, new ramps.
        queue.write_buffer(&self.view_buffer, 0, bytes(&[w as f32, h as f32, 0.0, 0.0]));
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
        let fx_groups = self.prepare_effects(device, queue, b);
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

        let mut draw_calls = 0usize;
        let mut passes = 0usize;
        let mut i = 0usize;
        let mut saved: Vec<u32> = Vec::new();
        let mut level = 0usize;
        let mut depth = 0u32;
        let mut clear = true;
        let mut clear_quad = b.root_clear.clone();
        loop {
            // A group with a mix mode is composited next: copy the target below first.
            if !clear {
                if let Some(Op::LayerEnd { mode: CompositeMode::Mix, .. }) = b.ops.get(i) {
                    if let Some(bd) = self.backdrop.as_ref() {
                        let size = wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 };
                        frame.encoder.copy_texture_to_texture(self.targets[level].color.as_image_copy(), bd.texture.as_image_copy(), size);
                    }
                }
                // A group with effects is composited next: its offscreen passes first.
                if let Some(Op::LayerEnd { mode: CompositeMode::Effect(k), .. }) = b.ops.get(i) {
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
                            timestamp_writes: None,
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
                    timestamp_writes: None,
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
                if clear {
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
                        Op::LayerBegin { .. } | Op::LayerEnd { .. } => break,
                    }
                    i += 1;
                }
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
        // The main 2D target onto the host's view (and, when asked, into the readback texture).
        let final_quad = cv(&b.final_quad);
        if final_quad.end > final_quad.start {
            let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("animation final"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: frame.view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
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
            self.encode_readback(frame, &vb, &bind0, final_quad);
        }
        (draw_calls + particle_draws, passes)
    }

    fn encode_readback(&mut self, frame: &mut CanvasFrame<'_>, vb: &wgpu::Buffer, bind0: &wgpu::BindGroup, quad: Range<u32>) {
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
                timestamp_writes: None,
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

    /// Without a browser there is no readback: `done` gets no pixels.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn map_readback(&self, done: Box<dyn FnOnce(Vec<u8>, u32, u32)>) {
        // Only a browser has this renderer's GPU device; elsewhere there is nothing to read.
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
