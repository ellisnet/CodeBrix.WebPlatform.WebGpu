//! The last pass of a TRANSPARENT canvas: the page composites a canvas texture as colour that is
//! premultiplied in the canvas's OWN encoding (sRGB values times alpha), but the layers draw
//! LINEAR premultiplied colour through an sRGB view - which stores sRGB(colour x alpha), a
//! different (brighter) value wherever alpha is between 0 and 1. Written straight to the
//! surface, a half-transparent white would reach the page as about 74 % white instead of 50 %,
//! and anti-aliased edges would show a light fringe over a dark page.
//!
//! So on a transparent canvas the layers draw into a texture of the host's (the same format and
//! size as the surface's sRGB view), and this pass writes it onto the surface: each pixel's
//! linear colour is un-premultiplied, encoded to sRGB, and premultiplied again by its alpha in
//! that encoding - exactly what the browser expects. An opaque canvas does not need it and does
//! not have it.

/// The pass's shader: a full-screen triangle; each pixel read once (no filtering), converted.
const COMPOSE_WGSL: &str = r#"
@group(0) @binding(0) var drawn: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

fn encode(c: vec3<f32>) -> vec3<f32> {
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, c <= vec3<f32>(0.0031308));
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    // The view decodes: this is the layers' linear, premultiplied colour.
    let value = textureLoad(drawn, vec2<i32>(position.xy), 0);
    if (value.a <= 0.0) {
        return vec4<f32>(0.0);
    }
    let straight = clamp(value.rgb / value.a, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(encode(straight) * value.a, value.a);
}
"#;

/// The value the pass writes for one pixel (the shader's arithmetic, for the host's tests):
/// linear premultiplied colour and alpha in, sRGB-encoded premultiplied colour and alpha out.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn compose_pixel(linear_premultiplied: [f32; 3], alpha: f32) -> [f32; 4] {
    if alpha <= 0.0 {
        return [0.0; 4];
    }
    let encode = |c: f32| if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
    let channel = |c: f32| encode((c / alpha).clamp(0.0, 1.0)) * alpha;
    [channel(linear_premultiplied[0]), channel(linear_premultiplied[1]), channel(linear_premultiplied[2]), alpha]
}

/// The host's texture the layers of a transparent canvas draw into, and the pass that writes it
/// onto the surface.
pub(crate) struct PageCompose {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    texture: Option<(wgpu::Texture, wgpu::TextureView, wgpu::BindGroup, u32, u32)>,
}

impl PageCompose {
    /// The pass for a surface of `surface_format` whose layers draw in `format` (its sRGB form).
    pub(crate) fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> PageCompose {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("codebrix-webgpu page compose"), source: wgpu::ShaderSource::Wgsl(COMPOSE_WGSL.into()) });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("codebrix-webgpu page compose"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("codebrix-webgpu page compose"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("codebrix-webgpu page compose"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_main"), compilation_options: wgpu::PipelineCompilationOptions::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState { format: surface_format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        PageCompose { pipeline, layout, texture: None }
    }

    /// The texture the layers draw into this frame (made, or made again at a new size).
    pub(crate) fn target(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat, width: u32, height: u32) -> (&wgpu::Texture, &wgpu::TextureView) {
        let fits = self.texture.as_ref().is_some_and(|(_, _, _, w, h)| *w == width && *h == height);
        if !fits {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("codebrix-webgpu transparent canvas"),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("codebrix-webgpu page compose"),
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }],
            });
            self.texture = Some((texture, view, group, width, height));
        }
        let (texture, view, _, _, _) = self.texture.as_ref().unwrap();
        (texture, view)
    }

    /// Writes what the layers drew onto the surface's view (in the surface's own format).
    pub(crate) fn write(&self, encoder: &mut wgpu::CommandEncoder, surface_view: &wgpu::TextureView) {
        let Some((_, _, group, _, _)) = self.texture.as_ref() else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("codebrix-webgpu page compose"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::compose_pixel;

    #[test]
    fn compose_pixel_premultiplies_in_the_srgb_encoding_the_page_expects() {
        // Half-transparent white: the layers hold linear (0.5, 0.5, 0.5, 0.5); the page must get
        // 50 % white (0.5), not sRGB(0.5) = 0.735.
        let white = compose_pixel([0.5, 0.5, 0.5], 0.5);
        assert!((white[0] - 0.5).abs() < 1e-6 && (white[3] - 0.5).abs() < 1e-6);
        // Opaque colours are only encoded.
        let grey = compose_pixel([0.2140411, 0.2140411, 0.2140411], 1.0);
        assert!((grey[0] - 0.5).abs() < 1e-4, "linear 0.214 is sRGB 0.5, got {}", grey[0]);
        // A quarter-covered edge pixel of linear 0.214 grey: sRGB 0.5 x 0.25.
        let edge = compose_pixel([0.2140411 * 0.25, 0.0, 0.0], 0.25);
        assert!((edge[0] - 0.125).abs() < 1e-4);
        // Nothing drawn stays nothing; colour above alpha (not premultiplied) is clamped.
        assert_eq!(compose_pixel([0.3, 0.3, 0.3], 0.0), [0.0; 4]);
        assert!(compose_pixel([0.9, 0.0, 0.0], 0.5)[0] <= 0.5 + 1e-6);
    }
}
