//! The textured quad layer: a square with a texture made in code, turning with the canvas
//! clock's time, drawn entirely THROUGH codebrix-webgpu-graphics - its context, a vertex and an
//! index buffer, a uniform buffer, a texture with mip levels, a sampler, two bind groups and the
//! render pipeline builder - with the library's own built-in sprite shader. It is a LAYER of the
//! canvas host: it draws into the frame it is handed, over what is there (it never clears), and
//! takes its time from the frame (it reads no clock), so it stands still while the clock is
//! paused. A drag turns it further: the content follows the pointer, as on the layering scene.

use codebrix_webgpu::{glam, wgpu, CanvasFrame, CanvasInput, CanvasRenderer, CanvasTarget, GpuContext};
use codebrix_webgpu_graphics::shaders::BuiltInShaders;
use codebrix_webgpu_graphics::{
    bytemuck, BindingEntry, BindingLayoutEntry, BlendPreset, BufferRange, ColorTargetOptions, DrawIndexedOptions, GpuBindGroup, GpuBuffer, GpuRenderPipeline, GraphicsContext, GraphicsException, RgbaTextureOptions, SamplerDescription, VertexLayout,
    VertexLayoutOptions, VertexType,
};
use dioxus::prelude::*;

/// Radians per clock second of the quad's turn.
const SPIN_SPEED: f64 = 0.6;
/// Radians per CSS pixel of a drag.
const DRAG_SPEED: f32 = 0.01;
/// The side of the generated texture, in pixels.
const TEXTURE_SIZE: u32 = 128;
/// Half the quad's side, in clip-space units of the canvas's shorter side.
const HALF_SIZE: f32 = 0.6;

/// One corner of the quad, as the built-in sprite shader reads it.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct QuadVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    textured: f32,
}

// SAFETY: nine f32 fields with #[repr(C)] and no padding: every bit pattern is a valid value.
unsafe impl bytemuck::Zeroable for QuadVertex {}
// SAFETY: as above; the type is Copy and 'static.
unsafe impl bytemuck::Pod for QuadVertex {}

impl VertexType for QuadVertex {
    fn vertex_layout(options: VertexLayoutOptions) -> VertexLayout {
        VertexLayout::describe(options)
            .add(wgpu::VertexFormat::Float32x2)
            .add(wgpu::VertexFormat::Float32x2)
            .add(wgpu::VertexFormat::Float32x4)
            .add(wgpu::VertexFormat::Float32)
            .build(std::mem::size_of::<QuadVertex>() as u64)
    }
}

/// What the page shows about the quad.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuadView {
    /// The frame number on the clock.
    pub frame: u64,
    /// The quad's angle, in degrees (0 to 360).
    pub degrees: f32,
}

/// Draws the quad.
pub struct QuadRenderer {
    graphics: GraphicsContext,
    gpu: Option<Gpu>,
    drag_angle: f32,
    aspect: f32,
    view: Signal<QuadView>,
    errors: Signal<Vec<String>>,
}

struct Gpu {
    pipeline: GpuRenderPipeline,
    vertices: GpuBuffer,
    indices: GpuBuffer,
    uniforms: GpuBuffer,
    globals: GpuBindGroup,
    material: GpuBindGroup,
}

impl QuadRenderer {
    /// A renderer drawing with `graphics`, publishing its view into `view` and every error (its
    /// own, and WebGPU's from `take_errors`) into `errors`.
    pub fn new(graphics: GraphicsContext, view: Signal<QuadView>, errors: Signal<Vec<String>>) -> QuadRenderer {
        QuadRenderer { graphics, gpu: None, drag_angle: 0.0, aspect: 1.0, view, errors }
    }

    fn report(&mut self, error: String) {
        let mut errors = self.errors.write();
        if errors.len() < 20 {
            errors.push(error);
        }
    }
}

/// The quad's texture: a checkerboard whose four quarters have four colours (red at the top
/// left, green at the top right, blue at the bottom left, yellow at the bottom right), with a
/// white bar along its top edge, so that its turning and its orientation are plain to see. RGBA,
/// red first, sRGB-encoded colours.
pub fn texture_pixels(size: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    let cell = (size / 8).max(1);
    for y in 0..size {
        for x in 0..size {
            let top = y < size / 2;
            let left = x < size / 2;
            let base: [u8; 3] = match (top, left) {
                (true, true) => [220, 60, 50],
                (true, false) => [60, 190, 90],
                (false, true) => [60, 110, 230],
                (false, false) => [235, 200, 60],
            };
            let dark = ((x / cell) + (y / cell)) % 2 == 1;
            let pixel = if y < size / 16 {
                [255, 255, 255]
            } else if dark {
                [base[0] / 2, base[1] / 2, base[2] / 2]
            } else {
                base
            };
            pixels.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
        }
    }
    pixels
}

/// The quad's corners (white tint, textured), counter-clockwise from the bottom left.
fn corners() -> [QuadVertex; 4] {
    let corner = |x: f32, y: f32, u: f32, v: f32| QuadVertex { position: [x * HALF_SIZE, y * HALF_SIZE], uv: [u, v], color: [1.0, 1.0, 1.0, 1.0], textured: 1.0 };
    [corner(-1.0, -1.0, 0.0, 1.0), corner(1.0, -1.0, 1.0, 1.0), corner(1.0, 1.0, 1.0, 0.0), corner(-1.0, 1.0, 0.0, 0.0)]
}

/// Makes everything the quad draws with, for colour target `format`.
fn create_gpu(graphics: &GraphicsContext, format: wgpu::TextureFormat) -> Result<Gpu, GraphicsException> {
    let shader = graphics.create_shader(BuiltInShaders::SPRITE, Some("sprite (built-in)"));
    let globals_layout = graphics.create_bind_group_layout(Some("quad globals"), &[BindingLayoutEntry::uniform(0, wgpu::ShaderStages::VERTEX)]);
    let material_layout = graphics.create_bind_group_layout(Some("quad material"), &[BindingLayoutEntry::texture(0, wgpu::ShaderStages::FRAGMENT), BindingLayoutEntry::sampler(1, wgpu::ShaderStages::FRAGMENT)]);
    let pipeline = pipeline_builder(graphics, &shader, format, &globals_layout, &material_layout).build()?;
    let vertices = graphics.create_vertex_buffer(&corners(), Some("quad vertices"))?;
    let indices = graphics.create_index_buffer_u16(&[0, 1, 2, 0, 2, 3], Some("quad indices"))?;
    let uniforms = graphics.create_uniform_buffer_from(&glam::Mat4::IDENTITY, Some("quad globals"))?;
    let texture = graphics.create_texture_from_rgba(TEXTURE_SIZE, TEXTURE_SIZE, &texture_pixels(TEXTURE_SIZE), RgbaTextureOptions { mipmaps: true, srgb: true, label: Some("quad texture".to_string()) })?;
    let sampler = graphics.create_sampler(&SamplerDescription::linear_clamp());
    let globals = graphics.create_bind_group(&globals_layout, Some("quad globals"), &[BindingEntry::for_buffer(0, &uniforms)]);
    let material = graphics.create_bind_group(&material_layout, Some("quad material"), &[BindingEntry::for_texture(0, &texture), BindingEntry::for_sampler(1, &sampler)]);
    Ok(Gpu { pipeline, vertices, indices, uniforms, globals, material })
}

/// The quad's pipeline: the sprite shader, one vertex buffer of [`QuadVertex`], premultiplied
/// blending into `format` (the canvas host's convention), the two bind-group layouts.
pub fn pipeline_builder(graphics: &GraphicsContext, shader: &codebrix_webgpu_graphics::GpuShader, format: wgpu::TextureFormat, globals: &codebrix_webgpu_graphics::GpuBindGroupLayout, material: &codebrix_webgpu_graphics::GpuBindGroupLayout) -> codebrix_webgpu_graphics::RenderPipelineBuilder {
    graphics
        .create_render_pipeline(shader)
        .with_vertex_buffer_of::<QuadVertex>(VertexLayoutOptions::default())
        .with_color_target(format, ColorTargetOptions::blended(BlendPreset::PremultipliedAlpha))
        .with_bind_group_layouts(&[globals, material])
        .with_label("textured quad")
}

impl CanvasRenderer for QuadRenderer {
    fn prepare(&mut self, _context: &GpuContext, target: &CanvasTarget) {
        match create_gpu(&self.graphics, target.format) {
            Ok(gpu) => self.gpu = Some(gpu),
            Err(error) => self.report(error.to_string()),
        }
    }

    fn resize(&mut self, _context: &GpuContext, width: u32, height: u32) {
        self.aspect = width.max(1) as f32 / height.max(1) as f32;
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        let angle = (frame.info.time * SPIN_SPEED) as f32 + self.drag_angle;
        let number = frame.info.number;
        let fit = glam::Vec3::new((1.0 / self.aspect).min(1.0), self.aspect.min(1.0), 1.0);
        let transform = glam::Mat4::from_scale(fit) * glam::Mat4::from_rotation_z(angle);
        let graphics = self.graphics.clone();
        let mut failure = None;
        if let Some(gpu) = self.gpu.as_ref() {
            if let Err(error) = gpu.uniforms.write_value(&transform, 0) {
                failure = Some(error.to_string());
            }
            let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
            // None: LOAD what the host and the layers below drew; a layer never clears.
            match graphics_frame.begin_render_pass(None, None) {
                Ok(mut pass) => {
                    pass.set_pipeline(&gpu.pipeline);
                    pass.set_bind_group(0, &gpu.globals);
                    pass.set_bind_group(1, &gpu.material);
                    pass.set_vertex_buffer(0, &gpu.vertices, BufferRange::default());
                    pass.set_index_buffer(&gpu.indices, wgpu::IndexFormat::Uint16, BufferRange::default());
                    pass.draw_indexed(6, DrawIndexedOptions::default());
                }
                Err(error) => failure = Some(error.to_string()),
            };
        }
        if let Some(error) = failure {
            self.report(error);
        }
        for error in graphics.take_errors() {
            self.report(error.to_string());
        }
        let view = QuadView { frame: number, degrees: angle.to_degrees().rem_euclid(360.0) };
        if *self.view.peek() != view {
            self.view.set(view);
        }
    }

    /// Drag: the content follows the pointer - a drag to the right turns the quad
    /// counter-clockwise (the angle grows), as the triangle of the layering scene does.
    fn input(&mut self, input: &CanvasInput) {
        if let CanvasInput::PointerMove { dx, pressed: true, .. } = input {
            self.drag_angle += *dx as f32 * DRAG_SPEED;
        }
    }
}
