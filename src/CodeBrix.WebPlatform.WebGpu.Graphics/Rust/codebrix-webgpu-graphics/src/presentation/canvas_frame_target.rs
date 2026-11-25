//! [`CanvasFrameTarget`]: what a graphics layer keeps for its canvas so that a frame on the canvas
//! can draw with depth and, optionally, multisampling (MSAA) - the buffers follow the canvas's
//! size and format by themselves, and the sample count may change while the page runs (Not in
//! C#; C# multisamples by the application's own render textures).

use std::collections::HashMap;

use crate::context::{DepthTextureOptions, GraphicsContext, GraphicsException, Texture2DOptions};
use crate::pipelines::{BlendPreset, ColorTargetOptions, GpuRenderPipeline};
use crate::presentation::multisampling::{check_sample_count, create_multisampled_texture, plan_buffers, BufferSets};
use crate::presentation::RenderTargetView;
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuSampler, GpuShader, GpuTexture, SamplerDescription};
use crate::shaders::MipmapShader;

/// The settings of a [`CanvasFrameTarget`] (Not in C#).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasFrameTargetOptions {
    /// The samples per pixel: 1 (default, no multisampling) or 4 (4x MSAA). Changeable later with
    /// [`CanvasFrameTarget::set_sample_count`].
    pub sample_count: u32,
    /// The depth format of the depth buffer the target keeps at the canvas's size; None for a
    /// layer that draws without depth (2D). Default `Depth32Float`.
    pub depth_format: Option<wgpu::TextureFormat>,
}

impl Default for CanvasFrameTargetOptions {
    fn default() -> CanvasFrameTargetOptions {
        CanvasFrameTargetOptions { sample_count: 1, depth_format: Some(wgpu::TextureFormat::Depth32Float) }
    }
}

/// The 4-sample buffers of one canvas size and format.
struct CanvasMultisampled {
    color: GpuTexture,
    depth: Option<GpuTexture>,
    resolve: GpuTexture,
    group: GpuBindGroup,
}

/// The draw that lays a multisampled layer's resolved picture over the canvas, made by
/// [`GraphicsContext::begin_canvas_frame_on`] and recorded by the frame when it is dropped.
pub(crate) struct CanvasLayerResolve {
    /// The multisampled colour view: a pass on it is a pass of this layer.
    pub(crate) multisampled_view: wgpu::TextureView,
    /// The canvas frame's own view (what the layer's picture is laid over).
    pub(crate) canvas_view: wgpu::TextureView,
    /// The premultiplied-alpha blit pipeline for the canvas format.
    pub(crate) pipeline: wgpu::RenderPipeline,
    /// The bind group of the resolve texture.
    pub(crate) group: wgpu::BindGroup,
    /// True once the frame has begun a pass on the multisampled view (the first one clears it).
    pub(crate) started: bool,
}

/// What a graphics LAYER keeps for its canvas (Not in C#): the sample-count setting and the
/// buffers a frame on the canvas draws into, made at the canvas's size and draw format and made
/// again when the canvas is resized (no `resize` code needed). Hand it to
/// [`GraphicsContext::begin_canvas_frame_on`] every frame.
///
/// With 1 sample a frame draws straight into the canvas's picture, with the target's depth
/// buffer - what [`GraphicsContext::begin_canvas_frame`] with a depth texture of the layer's own
/// does. With 4 samples the frame's target ([`crate::GraphicsFrame::target`]) is a multisampled
/// colour and depth the target owns; every pass ends by resolving into an ordinary texture of the
/// canvas's size and format, and when the frame is dropped that resolved picture is laid over the
/// canvas with premultiplied-alpha blending, so the layer still draws OVER what the host and the
/// layers below drew, its edges smoothed. (WebGPU's resolve replaces its destination; laying the
/// resolved picture over the canvas is what keeps the layers below.) The first pass of each frame
/// on the multisampled colour CLEARS it - to the pass's clear colour, or to transparent when the
/// pass asks to load - because what lies below is not in the multisampled buffer.
///
/// The buffers of a count are made the first time a frame uses it and KEPT while the size and
/// format stay, so switching the setting back and forth makes nothing new;
/// [`CanvasFrameTarget::release_unused_buffers`] frees the other count's. Each canvas has its own
/// target, so two canvases on a page may differ. Memory with 4 samples: about four times the
/// canvas's colour and depth, plus one more colour texture.
pub struct CanvasFrameTarget {
    context: GraphicsContext,
    sample_count: u32,
    depth_format: Option<wgpu::TextureFormat>,
    made_for: Option<((u32, u32), wgpu::TextureFormat)>,
    depth: Option<GpuTexture>,
    multi: Option<CanvasMultisampled>,
    blit: Option<(GpuShader, GpuBindGroupLayout, GpuSampler)>,
    blit_pipelines: HashMap<wgpu::TextureFormat, GpuRenderPipeline>,
    buffer_sets_made: u64,
}

impl GraphicsContext {
    /// A canvas frame target (Not in C#). Err for a sample count other than 1 or 4, or a depth
    /// format that is not a depth format.
    pub fn create_canvas_frame_target(&self, options: CanvasFrameTargetOptions) -> Result<CanvasFrameTarget, GraphicsException> {
        check_sample_count(options.sample_count, "a canvas frame target")?;
        if let Some(format) = options.depth_format {
            if !crate::resources::TextureFormats::is_depth(format) {
                return Err(GraphicsException::new(format!("{format:?} is not a depth format; give the canvas frame target Depth32Float, Depth24Plus or another depth format, or None for no depth.")));
            }
        }
        Ok(CanvasFrameTarget { context: self.clone(), sample_count: options.sample_count, depth_format: options.depth_format, made_for: None, depth: None, multi: None, blit: None, blit_pipelines: HashMap::new(), buffer_sets_made: 0 })
    }
}

impl CanvasFrameTarget {
    /// The samples per pixel the next frame draws with: 1 or 4.
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// Sets the samples per pixel, 1 or 4, at any time; the next frame draws with it (its
    /// buffers are made then, the first time). Err for any other count; the setting is then
    /// unchanged.
    pub fn set_sample_count(&mut self, count: u32) -> Result<(), GraphicsException> {
        check_sample_count(count, "a canvas frame target")?;
        self.sample_count = count;
        Ok(())
    }

    /// The depth format, None for a target without depth.
    pub fn depth_format(&self) -> Option<wgpu::TextureFormat> {
        self.depth_format
    }

    /// The depth buffer the last frame drew with (multisampled with 4 samples), once a frame has
    /// begun; None without depth.
    pub fn depth(&self) -> Option<&GpuTexture> {
        if self.sample_count > 1 {
            self.multi.as_ref().and_then(|multi| multi.depth.as_ref())
        } else {
            self.depth.as_ref()
        }
    }

    /// How many buffer sets the target has made so far - one per (size, format, count) it drew
    /// with (for a page that shows that switching the setting back makes nothing new).
    pub fn buffer_sets_made(&self) -> u64 {
        self.buffer_sets_made
    }

    /// Frees the buffers of the sample count NOT in use; switching back makes them again.
    pub fn release_unused_buffers(&mut self) {
        if self.sample_count > 1 {
            self.depth = None;
        } else {
            self.multi = None;
        }
    }

    /// Makes what a frame of `width` x `height` in `format` with the current count needs, and
    /// hands out the frame's target and, with 4 samples, the draw that lays the resolved picture
    /// over `canvas_view`.
    pub(crate) fn prepare(&mut self, canvas_view: &wgpu::TextureView, width: u32, height: u32, format: wgpu::TextureFormat) -> Result<(RenderTargetView, Option<CanvasLayerResolve>), GraphicsException> {
        let wanted = ((width, height), format);
        let plan = plan_buffers(BufferSets { single: self.depth.is_some(), multi: self.multi.is_some() }, self.made_for, wanted, self.sample_count, self.depth_format.is_some());
        if plan.drop_all || self.made_for != Some(wanted) {
            self.depth = None;
            self.multi = None;
            self.made_for = Some(wanted);
        }
        if plan.create_single {
            let depth_format = self.depth_format.expect("planned only with depth");
            self.depth = Some(self.context.create_depth_texture(width, height, DepthTextureOptions { format: depth_format, label: Some("canvas depth".to_string()) })?);
            self.buffer_sets_made += 1;
        }
        if plan.create_multi {
            self.multi = Some(self.create_multi(width, height, format)?);
            self.buffer_sets_made += 1;
        }
        if self.sample_count == 1 {
            let target = RenderTargetView::new(canvas_view.clone(), format, width, height, None);
            let target = match &self.depth {
                Some(depth) => target.with_depth(depth),
                None => target,
            };
            return Ok((target, None));
        }
        let pipeline = self.blit_pipeline(format)?;
        let multi = self.multi.as_ref().expect("made above");
        let target = RenderTargetView::new(multi.color.view().clone(), format, width, height, multi.depth.as_ref().map(|depth| (depth.view().clone(), depth.format()))).with_resolve(self.sample_count, multi.resolve.view().clone())?;
        let resolve = CanvasLayerResolve { multisampled_view: multi.color.view().clone(), canvas_view: canvas_view.clone(), pipeline: pipeline.handle().clone(), group: multi.group.handle().clone(), started: false };
        Ok((target, Some(resolve)))
    }

    fn create_multi(&mut self, width: u32, height: u32, format: wgpu::TextureFormat) -> Result<CanvasMultisampled, GraphicsException> {
        let count = self.sample_count;
        let color = create_multisampled_texture(&self.context, width, height, format, count, "canvas multisampled colour")?;
        let depth = match self.depth_format {
            Some(depth_format) => Some(create_multisampled_texture(&self.context, width, height, depth_format, count, "canvas multisampled depth")?),
            None => None,
        };
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let resolve = self.context.create_texture_2d(width, height, format, usage, Texture2DOptions { mip_level_count: 1, label: Some("canvas resolved layer".to_string()) })?;
        let context = self.context.clone();
        let (_, layout, sampler) = self.blit_parts();
        let group = context.create_bind_group(layout, Some("canvas resolved layer"), &[BindingEntry::for_texture(0, &resolve), BindingEntry::for_sampler(1, sampler)]);
        Ok(CanvasMultisampled { color, depth, resolve, group })
    }

    /// The blit's shader (the mipmap generator's full-screen copy), layout and sampler.
    fn blit_parts(&mut self) -> &(GpuShader, GpuBindGroupLayout, GpuSampler) {
        let context = self.context.clone();
        self.blit.get_or_insert_with(|| {
            let shader = context.create_shader(MipmapShader::SOURCE, Some("canvas resolved layer"));
            let fragment = wgpu::ShaderStages::FRAGMENT;
            let layout = context.create_bind_group_layout(Some("canvas resolved layer"), &[BindingLayoutEntry::texture(0, fragment), BindingLayoutEntry::sampler(1, fragment)]);
            let sampler = context.create_sampler(&SamplerDescription::nearest_clamp());
            (shader, layout, sampler)
        })
    }

    /// The pipeline that lays the resolved picture over a canvas of `format` (premultiplied alpha).
    fn blit_pipeline(&mut self, format: wgpu::TextureFormat) -> Result<GpuRenderPipeline, GraphicsException> {
        if let Some(pipeline) = self.blit_pipelines.get(&format) {
            return Ok(pipeline.clone());
        }
        let context = self.context.clone();
        let (shader, layout, _) = self.blit_parts();
        let pipeline = context
            .create_render_pipeline(shader)
            .with_label(&format!("canvas resolved layer {format:?}"))
            .with_color_target(format, ColorTargetOptions::blended(BlendPreset::PremultipliedAlpha))
            .with_bind_group_layouts(&[layout])
            .build()?;
        self.blit_pipelines.insert(format, pipeline.clone());
        Ok(pipeline)
    }
}

#[cfg(test)]
mod tests {
    use super::CanvasFrameTargetOptions;

    #[test]
    fn the_default_is_one_sample_with_depth() {
        let options = CanvasFrameTargetOptions::default();
        assert_eq!(options.sample_count, 1);
        assert_eq!(options.depth_format, Some(wgpu::TextureFormat::Depth32Float));
    }
}
