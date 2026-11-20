//! The overlay layer: a translucent orange square that glides left and right with the canvas
//! clock's time, blended over whatever the layers below it drew. It is INDEPENDENT of the
//! triangle: it knows nothing of it, only the frame the canvas host hands every layer (the
//! target, the size, the device and queue, the clock's time and its named points). It appears
//! at the clock's named point "title in", the same point at which the triangle turns lighter.
//! The page can make it SOLID (alpha 1) instead of translucent, so that the order of the layers
//! is plain to see.

use codebrix_webgpu::{wgpu, CanvasFrame, CanvasRenderer, CanvasTarget, GpuContext};
use dioxus::prelude::*;

use crate::TITLE_IN;

/// The square's colour (linear), its opacity when translucent, and how fast it glides.
const COLOR: [f32; 3] = [1.0, 0.55, 0.1];
const ALPHA: f32 = 0.5;
/// Its opacity when solid.
const SOLID_ALPHA: f32 = 1.0;
/// Radians of the glide per clock second, and how far it glides from the centre (clip space).
const GLIDE_SPEED: f64 = 0.9;
const GLIDE_REACH: f32 = 0.35;
/// Half the square's side, in clip-space units of the canvas's shorter side.
const HALF_SIZE: f32 = 0.3;

/// Draws the square.
pub struct OverlayRenderer {
    gpu: Option<Gpu>,
    aspect: f32,
    shown_at: Signal<Option<u64>>,
    solid: Signal<bool>,
}

struct Gpu {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl OverlayRenderer {
    /// A renderer that writes the number of the frame in which it first appeared into `shown_at`,
    /// and draws the square solid while `solid` is true (translucent otherwise).
    pub fn new(shown_at: Signal<Option<u64>>, solid: Signal<bool>) -> OverlayRenderer {
        OverlayRenderer { gpu: None, aspect: 1.0, shown_at, solid }
    }
}

impl CanvasRenderer for OverlayRenderer {
    fn prepare(&mut self, context: &GpuContext, target: &CanvasTarget) {
        let device = context.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("overlay shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("overlay.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("overlay pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target.format,
                    // The host's convention: premultiplied colour, premultiplied blending.
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("overlay uniforms"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("overlay bind group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() }],
        });
        self.gpu = Some(Gpu { pipeline, uniforms, bind_group });
    }

    fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
        self.aspect = width.max(1) as f32 / height.max(1) as f32;
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let Some(gpu) = self.gpu.as_ref() else {
            return;
        };
        if !frame.point_reached(TITLE_IN) {
            return;
        }
        if self.shown_at.peek().is_none() {
            self.shown_at.set(Some(frame.info.number));
        }
        let x = GLIDE_REACH * (frame.info.time * GLIDE_SPEED).sin() as f32;
        let half = [HALF_SIZE * (1.0 / self.aspect).min(1.0), HALF_SIZE * self.aspect.min(1.0)];
        let alpha = if *self.solid.peek() { SOLID_ALPHA } else { ALPHA };
        let values = [x, 0.0, half[0], half[1], COLOR[0] * alpha, COLOR[1] * alpha, COLOR[2] * alpha, alpha];
        let mut bytes = [0u8; 32];
        for (chunk, value) in bytes.chunks_exact_mut(4).zip(values) {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        frame.context.queue().write_buffer(&gpu.uniforms, 0, &bytes);

        let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("overlay pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: frame.view,
                depth_slice: None,
                resolve_target: None,
                // Over the layers below: load, never clear.
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&gpu.pipeline);
        pass.set_bind_group(0, &gpu.bind_group, &[]);
        pass.draw(0..6, 0..1);
    }
}
