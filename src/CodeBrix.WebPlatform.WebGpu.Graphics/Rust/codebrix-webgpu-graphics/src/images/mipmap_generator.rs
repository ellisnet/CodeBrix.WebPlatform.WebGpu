//! The GPU mipmap generator and [`GraphicsContext::generate_mipmaps`] (C#: the internal
//! `MipmapGenerator` and `GraphicsContext.GenerateMipmaps`).

use std::collections::HashMap;

use crate::common::Color4;
use crate::context::{GraphicsContext, GraphicsException};
use crate::frames::DrawOptions;
use crate::pipelines::{ColorTargetOptions, GpuRenderPipeline};
use crate::presentation::RenderTargetView;
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroupLayout, GpuSampler, GpuShader, GpuTexture, SamplerDescription, TextureFormats};
use crate::shaders::MipmapShader;

/// Generates mip levels on the GPU: each level is rendered from the one above with a full-screen
/// triangle and a linear clamp sampler (a 2 x 2 box filter for even sizes). sRGB textures filter
/// in linear light because sampling decodes and the attachment encodes. One pipeline per format,
/// made on first use; owned by the context.
pub(crate) struct MipmapGenerator {
    parts: Option<(GpuShader, GpuBindGroupLayout, GpuSampler)>,
    pipelines: HashMap<wgpu::TextureFormat, GpuRenderPipeline>,
}

impl MipmapGenerator {
    /// A generator (its GPU objects are made on first use).
    pub(crate) fn new() -> MipmapGenerator {
        MipmapGenerator { parts: None, pipelines: HashMap::new() }
    }

    /// Fills mip levels 1..n of every layer of `texture` from level 0, in one submitted frame.
    pub(crate) fn generate(&mut self, context: &GraphicsContext, texture: &GpuTexture) -> Result<(), GraphicsException> {
        let (shader, layout, sampler) = self.parts.get_or_insert_with(|| {
            let shader = context.create_shader(MipmapShader::SOURCE, Some("mipmap generator"));
            let layout = context.create_bind_group_layout(Some("mipmap source"), &[BindingLayoutEntry::texture(0, wgpu::ShaderStages::FRAGMENT), BindingLayoutEntry::sampler(1, wgpu::ShaderStages::FRAGMENT)]);
            let sampler = context.create_sampler(&SamplerDescription::linear_clamp());
            (shader, layout, sampler)
        });
        let format = texture.format();
        let pipeline = match self.pipelines.get(&format) {
            Some(pipeline) => pipeline.clone(),
            None => {
                let pipeline = context.create_render_pipeline(shader).with_label(&format!("mipmap {format:?}")).with_color_target(format, ColorTargetOptions::default()).with_bind_group_layouts(&[layout]).build()?;
                self.pipelines.insert(format, pipeline.clone());
                pipeline
            }
        };
        let mut frame = context.begin_frame();
        let level_view = |level: u32, layer: u32| {
            texture.handle().create_view(&wgpu::TextureViewDescriptor {
                label: Some("mip level"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_mip_level: level,
                mip_level_count: Some(1),
                base_array_layer: layer,
                array_layer_count: Some(1),
                ..wgpu::TextureViewDescriptor::default()
            })
        };
        for layer in 0..texture.array_layer_count() {
            let mut source = level_view(0, layer);
            for level in 1..texture.mip_level_count() {
                let target = level_view(level, layer);
                let group = context.create_bind_group(layout, Some("mipmap source"), &[BindingEntry::for_texture_view(0, &source), BindingEntry::for_sampler(1, sampler)]);
                let view = RenderTargetView::new(target.clone(), format, (texture.width() >> level).max(1), (texture.height() >> level).max(1), None);
                {
                    let mut pass = frame.begin_render_pass_to(&view, Some(Color4::TRANSPARENT), None)?;
                    pass.set_pipeline(&pipeline);
                    pass.set_bind_group(0, &group);
                    pass.draw(3, DrawOptions::default());
                }
                source = target;
            }
        }
        frame.submit()
    }
}

impl GraphicsContext {
    /// Fills mip levels 1 and up of every layer from level 0, on the GPU (C#: `GenerateMipmaps`):
    /// each level rendered from the previous one with linear filtering (sRGB formats filter in
    /// linear light). Records and submits a frame of its own. An error for a texture with one mip
    /// level, without TEXTURE_BINDING and RENDER_ATTACHMENT, or with a depth format.
    pub fn generate_mipmaps(&self, texture: &GpuTexture) -> Result<(), GraphicsException> {
        if texture.mip_level_count() < 2 {
            return Err(GraphicsException::new("The texture has only one mip level; make it with more (GpuTexture::full_mip_chain_length) to generate mipmaps."));
        }
        let needed = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;
        if !texture.usage().contains(needed) {
            return Err(GraphicsException::new("Generating mipmaps needs a texture made with TEXTURE_BINDING and RENDER_ATTACHMENT usage; add both when you create it."));
        }
        if TextureFormats::is_depth(texture.format()) {
            return Err(GraphicsException::new("Depth textures have no generated mipmaps; use a colour format."));
        }
        self.mipmap_generator().borrow_mut().generate(self, texture)
    }
}
