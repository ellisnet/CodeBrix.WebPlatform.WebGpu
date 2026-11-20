//! The triangle layer: one render pipeline, one uniform buffer, and three vertices drawn every
//! frame with the `wgpu` device directly, over what the host cleared. It spins with the canvas
//! clock (so it stands still while the clock is paused); a drag turns it, the wheel scales it,
//! and keys do both (see [`TriangleRenderer::input`]). From the clock's named point "title in" on
//! its colours turn lighter. It tells the page what it is doing through signals: the frame count,
//! the current view, and the frame in which it first saw "title in" reached.

use codebrix_webgpu::{wgpu, CanvasFrame, CanvasInput, CanvasRenderer, CanvasTarget, GpuContext};
use dioxus::prelude::*;

use crate::TITLE_IN;

/// What the page shows about the triangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct View {
    /// The rotation, in degrees (0 to 360).
    pub degrees: f32,
    /// The scale factor.
    pub scale: f32,
    /// Whether the triangle spins on its own.
    pub spinning: bool,
}

const MIN_SCALE: f32 = 0.2;
const MAX_SCALE: f32 = 3.0;
/// Radians per second of the triangle's own spin.
const SPIN_SPEED: f32 = 0.8;

/// Draws the triangle.
pub struct TriangleRenderer {
    gpu: Option<Gpu>,
    angle: f32,
    scale: f32,
    spinning: bool,
    aspect: f32,
    frames: Signal<u64>,
    view: Signal<View>,
    lit_at: Signal<Option<u64>>,
}

struct Gpu {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl TriangleRenderer {
    /// A renderer that writes the frame count into `frames`, its view into `view`, and the number
    /// of the frame in which it first saw the "title in" point reached into `lit_at`.
    pub fn new(frames: Signal<u64>, view: Signal<View>, lit_at: Signal<Option<u64>>) -> TriangleRenderer {
        TriangleRenderer { gpu: None, angle: 0.0, scale: 1.0, spinning: true, aspect: 1.0, frames, view, lit_at }
    }

    fn publish_view(&mut self) {
        let degrees = self.angle.to_degrees().rem_euclid(360.0);
        let view = View { degrees, scale: self.scale, spinning: self.spinning };
        if *self.view.peek() != view {
            self.view.set(view);
        }
    }

    fn rescale(&mut self, factor: f32) {
        self.scale = (self.scale * factor).clamp(MIN_SCALE, MAX_SCALE);
    }
}

impl CanvasRenderer for TriangleRenderer {
    fn prepare(&mut self, context: &GpuContext, target: &CanvasTarget) {
        let device = context.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("triangle shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("triangle.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("triangle pipeline"),
            // The layout comes from the shader (one uniform block in group 0).
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
                    // Opaque: it replaces what is under it (the host's clear).
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("triangle uniforms"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("triangle bind group"),
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
        if self.spinning {
            self.angle += SPIN_SPEED * frame.info.delta as f32;
        }
        let lit = frame.point_reached(TITLE_IN);
        if lit && self.lit_at.peek().is_none() {
            self.lit_at.set(Some(frame.info.number));
        }
        let values = [self.angle, self.scale, self.aspect, frame.info.time as f32, if lit { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0];
        let mut bytes = [0u8; 32];
        for (chunk, value) in bytes.chunks_exact_mut(4).zip(values) {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        frame.context.queue().write_buffer(&gpu.uniforms, 0, &bytes);

        {
            let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("triangle pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    // A layer loads the target: the host has cleared it.
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&gpu.pipeline);
            pass.set_bind_group(0, &gpu.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.frames.set(frame.info.number);
        self.publish_view();
    }

    /// Drag: turn (left and right). Wheel: scale. Keys: arrow left and right turn, arrow up and
    /// down or + and - scale, the space bar starts and stops the spin, r resets.
    fn input(&mut self, input: &CanvasInput) {
        match input {
            // The content follows the pointer: a drag to the right turns the triangle
            // counter-clockwise (the angle grows), so its wide lower part moves with the pointer.
            CanvasInput::PointerMove { dx, pressed: true, .. } => self.angle += *dx as f32 * 0.01,
            CanvasInput::Wheel { delta_y, .. } => self.rescale((-*delta_y as f32 * 0.0015).exp()),
            CanvasInput::KeyDown { key } => match key.as_str() {
                "ArrowLeft" => self.angle += 0.1,
                "ArrowRight" => self.angle -= 0.1,
                "ArrowUp" | "+" | "=" => self.rescale(1.1),
                "ArrowDown" | "-" => self.rescale(1.0 / 1.1),
                " " => self.spinning = !self.spinning,
                "r" | "R" => {
                    self.angle = 0.0;
                    self.scale = 1.0;
                    self.spinning = true;
                }
                _ => return,
            },
            _ => return,
        }
        self.publish_view();
    }
}
