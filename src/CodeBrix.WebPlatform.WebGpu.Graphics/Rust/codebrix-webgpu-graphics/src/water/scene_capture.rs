//! The water's per-frame SCENE CAPTURE (Not in C#): what a [`crate::HeadlessRenderTarget`] holds
//! after the pass with the opaque parts and the sky - its colour as resolved (a texture copy) and
//! its depth (one full-screen pass writing the raw depth into an `R32Float` texture, from a
//! 1-sample `texture_depth_2d` or sample 0 of a 4-sample `texture_depth_multisampled_2d`). The
//! water pass then reads only these copies, so no texture is bound and attached at once. The
//! copies are made per target size and format and kept.

use std::collections::HashMap;

use crate::common::Color4;
use crate::context::{GraphicsContext, GraphicsException, Texture2DOptions};
use crate::frames::{DrawOptions, GraphicsFrame};
use crate::pipelines::{ColorTargetOptions, GpuRenderPipeline};
use crate::presentation::{HeadlessRenderTarget, RenderTargetView};
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuShader, GpuTexture};
use crate::shaders::WaterShaders;
use crate::water::water_math::{capture_plan, DepthRead};

/// The depth copy's format: one f32 per pixel, renderable, read with `textureLoad`.
pub(crate) const DEPTH_COPY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;
/// The target depth views whose bind groups the capture keeps.
const KEPT_GROUPS: usize = 4;

/// The copies of one target size and format.
pub(crate) struct CaptureCopies {
    pub(crate) color: GpuTexture,
    pub(crate) depth: GpuTexture,
    /// Changes whenever the copies are made again (the water's bind group follows it).
    pub(crate) generation: u64,
}

/// The capture's GPU state.
pub(crate) struct SceneCapture {
    context: GraphicsContext,
    shader: GpuShader,
    single_layout: GpuBindGroupLayout,
    multi_layout: GpuBindGroupLayout,
    pipelines: HashMap<DepthRead, GpuRenderPipeline>,
    copies: Option<CaptureCopies>,
    generation: u64,
    depth_groups: Vec<(wgpu::TextureView, GpuBindGroup)>,
    /// The frame and the colour view of the last captured target.
    captured: Option<(u64, wgpu::TextureView)>,
}

impl SceneCapture {
    /// The capture, with its shader and layouts (no copies until the first capture).
    pub(crate) fn new(context: &GraphicsContext) -> SceneCapture {
        let shader = context.create_shader(WaterShaders::CAPTURE, Some("water capture"));
        let fragment = wgpu::ShaderStages::FRAGMENT;
        let single_layout = context.create_bind_group_layout(Some("water capture 1 sample"), &[BindingLayoutEntry::texture(0, fragment).with_sample_type(wgpu::TextureSampleType::Depth)]);
        let multi_layout = context.create_bind_group_layout(Some("water capture 4 samples"), &[BindingLayoutEntry::texture(1, fragment).with_sample_type(wgpu::TextureSampleType::Depth).with_multisampled(true)]);
        SceneCapture { context: context.clone(), shader, single_layout, multi_layout, pipelines: HashMap::new(), copies: None, generation: 0, depth_groups: Vec::new(), captured: None }
    }

    /// The copies, once a capture has made them.
    pub(crate) fn copies(&self) -> Option<&CaptureCopies> {
        self.copies.as_ref()
    }

    /// The frame of the last capture.
    pub(crate) fn captured_frame(&self) -> Option<u64> {
        self.captured.as_ref().map(|(frame, _)| *frame)
    }

    /// True when `target` is the target last captured (its colour view is the same view).
    pub(crate) fn is_target(&self, target: &RenderTargetView) -> bool {
        self.captured.as_ref().is_some_and(|(_, view)| view == target.color_view())
    }

    /// The pipelines made so far (at most two: 1 and 4 samples).
    pub(crate) fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }

    /// Copies what `target` holds so far: the resolved colour and the depth (see the module).
    pub(crate) fn capture(&mut self, frame: &mut GraphicsFrame<'_>, target: &HeadlessRenderTarget) -> Result<(), GraphicsException> {
        let depth = target.depth();
        let read = capture_plan(target.sample_count(), depth.map(GpuTexture::format))?;
        let depth = depth.expect("the plan checked the depth");
        let (width, height, format) = (target.width(), target.height(), target.format());
        let stale = self.copies.as_ref().is_none_or(|c| c.color.width() != width || c.color.height() != height || c.color.format() != format);
        if stale {
            let color = self.context.create_texture_2d(width, height, format, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, Texture2DOptions { mip_level_count: 1, label: Some("water scene colour".to_string()) })?;
            let depth_copy = self.context.create_texture_2d(width, height, DEPTH_COPY_FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, Texture2DOptions { mip_level_count: 1, label: Some("water scene depth".to_string()) })?;
            self.generation += 1;
            self.copies = Some(CaptureCopies { color, depth: depth_copy, generation: self.generation });
        }
        let pipeline = self.pipeline(read)?;
        let group = self.depth_group(read, depth.view());
        let copies = self.copies.as_ref().expect("made above");
        frame.copy_texture_to_texture(target.color(), &copies.color, 0)?;
        {
            let mut pass = frame.begin_render_pass_to(&copies.depth.as_render_target(), Some(Color4::new(1.0, 0.0, 0.0, 1.0)), None)?;
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group);
            pass.draw(3, DrawOptions::default());
        }
        self.captured = Some((frame.id(), target.view().color_view().clone()));
        Ok(())
    }

    fn pipeline(&mut self, read: DepthRead) -> Result<GpuRenderPipeline, GraphicsException> {
        if let Some(pipeline) = self.pipelines.get(&read) {
            return Ok(pipeline.clone());
        }
        let (entry, layout) = match read {
            DepthRead::Single => ("fs_single", &self.single_layout),
            DepthRead::Multisampled => ("fs_multi", &self.multi_layout),
        };
        let pipeline = self
            .context
            .create_render_pipeline(&self.shader)
            .with_label(&format!("water capture {entry}"))
            .with_fragment_entry(entry, None)
            .with_color_target(DEPTH_COPY_FORMAT, ColorTargetOptions::default())
            .with_bind_group_layouts(&[layout])
            .build()?;
        self.pipelines.insert(read, pipeline.clone());
        Ok(pipeline)
    }

    fn depth_group(&mut self, read: DepthRead, view: &wgpu::TextureView) -> GpuBindGroup {
        if let Some((_, group)) = self.depth_groups.iter().find(|(kept, _)| kept == view) {
            return group.clone();
        }
        let group = match read {
            DepthRead::Single => self.context.create_bind_group(&self.single_layout, Some("water capture"), &[BindingEntry::for_texture_view(0, view)]),
            DepthRead::Multisampled => self.context.create_bind_group(&self.multi_layout, Some("water capture"), &[BindingEntry::for_texture_view(1, view)]),
        };
        if self.depth_groups.len() >= KEPT_GROUPS {
            self.depth_groups.remove(0);
        }
        self.depth_groups.push((view.clone(), group.clone()));
        group
    }
}
