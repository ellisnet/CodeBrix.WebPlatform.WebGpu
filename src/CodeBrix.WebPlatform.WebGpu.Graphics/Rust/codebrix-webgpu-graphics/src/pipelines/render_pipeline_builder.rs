//! [`RenderPipelineBuilder`]: a render pipeline put together from a shader, vertex layouts,
//! colour targets with blend presets, depth, culling and bind-group layouts; and
//! [`GpuRenderPipeline`], what it builds (C#: `RenderPipelineBuilder`, `GpuRenderPipeline`).

use crate::context::label_text;
use crate::context::{GraphicsContext, GraphicsException};
use crate::pipelines::{BlendPreset, CullPreset, DepthPreset, VertexLayout, VertexLayoutOptions, VertexType};
use crate::presentation::SAMPLE_COUNTS;
use crate::resources::{GpuBindGroupLayout, GpuShader};

/// The defaulted arguments of [`RenderPipelineBuilder::with_color_target`] (C#: `blend =
/// Opaque`, `writeMask = All`, and the overload that takes a `BlendState`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorTargetOptions {
    /// The blend preset.
    pub blend: BlendPreset,
    /// A blend state of the application's own, used instead of `blend`.
    pub custom_blend: Option<wgpu::BlendState>,
    /// Which channels are written.
    pub write_mask: wgpu::ColorWrites,
}

impl Default for ColorTargetOptions {
    fn default() -> ColorTargetOptions {
        ColorTargetOptions { blend: BlendPreset::Opaque, custom_blend: None, write_mask: wgpu::ColorWrites::ALL }
    }
}

impl ColorTargetOptions {
    /// The options with this blend preset and every channel written.
    pub fn blended(blend: BlendPreset) -> ColorTargetOptions {
        ColorTargetOptions { blend, ..ColorTargetOptions::default() }
    }
}

/// The defaulted arguments of [`RenderPipelineBuilder::with_depth`] (C#: `preset = ReadWrite`,
/// `bias = 0`, `slopeScale = 0`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DepthOptions {
    /// How depth is tested and written.
    pub preset: DepthPreset,
    /// The constant depth bias.
    pub bias: i32,
    /// The slope-scaled depth bias.
    pub slope_scale: f32,
}

/// The defaulted arguments of [`RenderPipelineBuilder::with_stencil`] (C#: `readMask` and
/// `writeMask`, both all ones).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StencilMasks {
    /// The read mask.
    pub read_mask: u32,
    /// The write mask.
    pub write_mask: u32,
}

impl Default for StencilMasks {
    fn default() -> StencilMasks {
        StencilMasks { read_mask: 0xFFFF_FFFF, write_mask: 0xFFFF_FFFF }
    }
}

/// Builds a render pipeline (C#: `RenderPipelineBuilder`, from
/// `GraphicsContext.CreateRenderPipeline(shader)`). The defaults are C#'s: entry points `vs_main`
/// and `fs_main` of the one shader, triangle lists, counter-clockwise front faces, no culling, no
/// depth, one sample. Every `with_*` method returns the builder; [`RenderPipelineBuilder::build`]
/// makes the pipeline.
///
/// Left out: `WithImmediates` / `WithImmediateSize` (a browser's WebGPU has no immediates).
pub struct RenderPipelineBuilder {
    context: GraphicsContext,
    vertex_shader: wgpu::ShaderModule,
    fragment_shader: wgpu::ShaderModule,
    vertex_entry: String,
    fragment_entry: String,
    has_fragment: bool,
    vertex_buffers: Vec<VertexLayout>,
    color_targets: Vec<(wgpu::TextureFormat, ColorTargetOptions)>,
    layouts: Vec<GpuBindGroupLayout>,
    explicit_layout: Option<wgpu::PipelineLayout>,
    topology: wgpu::PrimitiveTopology,
    front_face: wgpu::FrontFace,
    cull: CullPreset,
    depth: Option<(wgpu::TextureFormat, DepthOptions)>,
    stencil: Option<(wgpu::StencilFaceState, wgpu::StencilFaceState, StencilMasks)>,
    sample_count: u32,
    label: Option<String>,
}

impl GraphicsContext {
    /// Starts a render pipeline whose stages are `shader`'s `vs_main` and `fs_main` (C#:
    /// `CreateRenderPipeline(shader)`).
    pub fn create_render_pipeline(&self, shader: &GpuShader) -> RenderPipelineBuilder {
        RenderPipelineBuilder {
            context: self.clone(),
            vertex_shader: shader.handle().clone(),
            fragment_shader: shader.handle().clone(),
            vertex_entry: "vs_main".to_string(),
            fragment_entry: "fs_main".to_string(),
            has_fragment: true,
            vertex_buffers: Vec::new(),
            color_targets: Vec::new(),
            layouts: Vec::new(),
            explicit_layout: None,
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull: CullPreset::None,
            depth: None,
            stencil: None,
            sample_count: 1,
            label: None,
        }
    }
}

impl RenderPipelineBuilder {
    /// The vertex entry point, and optionally another shader for it (C#: `WithVertexEntry`).
    pub fn with_vertex_entry(mut self, entry_point: &str, shader: Option<&GpuShader>) -> RenderPipelineBuilder {
        self.vertex_entry = entry_point.to_string();
        if let Some(shader) = shader {
            self.vertex_shader = shader.handle().clone();
        }
        self
    }

    /// The fragment entry point, and optionally another shader for it (C#: `WithFragmentEntry`).
    pub fn with_fragment_entry(mut self, entry_point: &str, shader: Option<&GpuShader>) -> RenderPipelineBuilder {
        self.fragment_entry = entry_point.to_string();
        if let Some(shader) = shader {
            self.fragment_shader = shader.handle().clone();
        }
        self.has_fragment = true;
        self
    }

    /// No fragment stage (depth-only drawing) (C#: `WithoutFragment`). A depth-only pipeline that
    /// needs a fragment shader (one that discards, for alpha-tested parts) keeps its fragment
    /// entry and has no colour target: `with_depth` and no `with_color_target` (Not in C#).
    pub fn without_fragment(mut self) -> RenderPipelineBuilder {
        self.has_fragment = false;
        self
    }

    /// Adds a vertex buffer slot with this layout (C#: `WithVertexBuffer(layout)`).
    pub fn with_vertex_buffer(mut self, layout: VertexLayout) -> RenderPipelineBuilder {
        self.vertex_buffers.push(layout);
        self
    }

    /// Adds a vertex buffer slot of a vertex type (C#: `WithVertexBuffer<T>(stepMode, firstLocation)`).
    pub fn with_vertex_buffer_of<T: VertexType>(self, options: VertexLayoutOptions) -> RenderPipelineBuilder {
        self.with_vertex_buffer(T::vertex_layout(options))
    }

    /// Adds a colour target (C#: `WithColorTarget`). On a canvas, use the frame's
    /// `target.format` and `BlendPreset::PremultipliedAlpha` (or `Opaque` for a layer that covers
    /// everything), as the canvas host's blending convention says.
    pub fn with_color_target(mut self, format: wgpu::TextureFormat, options: ColorTargetOptions) -> RenderPipelineBuilder {
        self.color_targets.push((format, options));
        self
    }

    /// Stencil testing (C#: `WithStencil`); it needs a depth format with stencil (`with_depth`).
    pub fn with_stencil(mut self, front: wgpu::StencilFaceState, back: wgpu::StencilFaceState, masks: StencilMasks) -> RenderPipelineBuilder {
        self.stencil = Some((front, back, masks));
        self
    }

    /// A pipeline layout of the application's own instead of one made from
    /// [`RenderPipelineBuilder::with_bind_group_layouts`] (C#: `WithPipelineLayout`).
    pub fn with_pipeline_layout(mut self, layout: &wgpu::PipelineLayout) -> RenderPipelineBuilder {
        self.explicit_layout = Some(layout.clone());
        self
    }

    /// Depth testing against a depth attachment of `format` (C#: `WithDepth`).
    pub fn with_depth(mut self, format: wgpu::TextureFormat, options: DepthOptions) -> RenderPipelineBuilder {
        self.depth = Some((format, options));
        self
    }

    /// Face culling and the winding of front faces (C#: `WithCulling(preset, front)`).
    pub fn with_culling(mut self, preset: CullPreset, front: wgpu::FrontFace) -> RenderPipelineBuilder {
        self.cull = preset;
        self.front_face = front;
        self
    }

    /// The primitive topology (C#: `WithTopology`).
    pub fn with_topology(mut self, value: wgpu::PrimitiveTopology) -> RenderPipelineBuilder {
        self.topology = value;
        self
    }

    /// The sample count, for a multisampled target (C#: `WithSampleCount`): 1 (the default) or 4
    /// - `pass.target().sample_count()` for the target it will draw into; `build` refuses others.
    pub fn with_sample_count(mut self, count: u32) -> RenderPipelineBuilder {
        self.sample_count = count;
        self
    }

    /// The bind-group layouts, group 0 first (C#: `WithBindGroupLayouts`). With none, WebGPU
    /// derives the layout from the shader ([`GpuRenderPipeline::get_bind_group_layout`]).
    pub fn with_bind_group_layouts(mut self, bind_group_layouts: &[&GpuBindGroupLayout]) -> RenderPipelineBuilder {
        self.layouts = bind_group_layouts.iter().map(|layout| (*layout).clone()).collect();
        self
    }

    /// The label (C#: `WithLabel`).
    pub fn with_label(mut self, value: &str) -> RenderPipelineBuilder {
        self.label = Some(value.to_string());
        self
    }

    /// Makes the pipeline (C#: `Build`). A builder the library can see is incomplete gives an
    /// error at once; WebGPU's own validation of the pipeline reports through
    /// `GraphicsContext::take_errors` - or await [`RenderPipelineBuilder::build_checked`].
    pub fn build(self) -> Result<GpuRenderPipeline, GraphicsException> {
        self.check()?;
        let device = self.context.device();
        let layout = match &self.explicit_layout {
            Some(layout) => Some(layout.clone()),
            None if self.layouts.is_empty() => None,
            None => {
                let handles: Vec<Option<&wgpu::BindGroupLayout>> = self.layouts.iter().map(|layout| Some(layout.handle())).collect();
                Some(device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: self.label.as_deref(), bind_group_layouts: &handles, immediate_size: 0 }))
            }
        };
        let buffers: Vec<Option<wgpu::VertexBufferLayout<'_>>> = self.vertex_buffers.iter().map(|layout| Some(layout.to_wgpu())).collect();
        let targets: Vec<Option<wgpu::ColorTargetState>> = self
            .color_targets
            .iter()
            .map(|(format, options)| Some(wgpu::ColorTargetState { format: *format, blend: options.custom_blend.or_else(|| options.blend.to_wgpu()), write_mask: options.write_mask }))
            .collect();
        let depth_stencil = self.depth.map(|(format, options)| {
            let (compare, write) = options.preset.to_wgpu();
            let (front, back, masks) = self.stencil.unwrap_or((wgpu::StencilFaceState::IGNORE, wgpu::StencilFaceState::IGNORE, StencilMasks::default()));
            wgpu::DepthStencilState {
                format,
                depth_write_enabled: Some(write),
                depth_compare: Some(compare),
                stencil: wgpu::StencilState { front, back, read_mask: masks.read_mask, write_mask: masks.write_mask },
                bias: wgpu::DepthBiasState { constant: options.bias, slope_scale: options.slope_scale, clamp: 0.0 },
            }
        });
        let fragment = if self.has_fragment {
            Some(wgpu::FragmentState { module: &self.fragment_shader, entry_point: Some(&self.fragment_entry), compilation_options: wgpu::PipelineCompilationOptions::default(), targets: &targets })
        } else {
            None
        };
        let handle = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: self.label.as_deref(),
            layout: layout.as_ref(),
            vertex: wgpu::VertexState { module: &self.vertex_shader, entry_point: Some(&self.vertex_entry), compilation_options: wgpu::PipelineCompilationOptions::default(), buffers: &buffers },
            primitive: wgpu::PrimitiveState { topology: self.topology, strip_index_format: None, front_face: self.front_face, cull_mode: self.cull.to_wgpu(), unclipped_depth: false, polygon_mode: wgpu::PolygonMode::Fill, conservative: false },
            depth_stencil,
            multisample: wgpu::MultisampleState { count: self.sample_count, mask: !0, alpha_to_coverage_enabled: false },
            fragment,
            multiview_mask: None,
            cache: None,
        });
        Ok(GpuRenderPipeline {
            handle,
            color_formats: self.color_targets.iter().map(|(format, _)| *format).collect(),
            depth_format: self.depth.map(|(format, _)| format),
            sample_count: self.sample_count,
            label: self.label.clone(),
        })
    }

    /// As [`RenderPipelineBuilder::build`], and waits for WebGPU's verdict (C#: what `Build` does
    /// before it returns or throws): Err carries WebGPU's validation message.
    pub async fn build_checked(self) -> Result<GpuRenderPipeline, GraphicsException> {
        let context = self.context.clone();
        let what = format!("The render pipeline{}", label_text(self.label.as_deref()));
        context.validate_named(&what, move |_| self.build()).await?
    }

    fn check(&self) -> Result<(), GraphicsException> {
        check_parts(self.has_fragment, self.color_targets.len(), self.depth.is_some(), self.stencil.is_some(), self.sample_count, self.label.as_deref())
    }
}

/// The checks [`RenderPipelineBuilder::build`] makes itself, on the builder's parts. A fragment
/// stage without a colour target is a DEPTH-ONLY pipeline whose fragment shader only discards
/// (an alpha-tested shadow caster) - valid WebGPU, so allowed when a depth format is set.
fn check_parts(has_fragment: bool, color_target_count: usize, has_depth: bool, has_stencil: bool, sample_count: u32, label: Option<&str>) -> Result<(), GraphicsException> {
    if has_fragment && color_target_count == 0 && !has_depth {
        return Err(GraphicsException::new(format!("The render pipeline{} has a fragment stage but neither a colour target nor depth; add one with with_color_target (on a canvas: the frame's target.format), add with_depth for a depth-only pipeline, or call without_fragment.", label_text(label))));
    }
    if has_stencil && !has_depth {
        return Err(GraphicsException::new(format!("The render pipeline{} tests the stencil but has no depth-stencil format; add with_depth with a format that has a stencil aspect.", label_text(label))));
    }
    if !SAMPLE_COUNTS.contains(&sample_count) {
        return Err(GraphicsException::new(format!("The render pipeline{} cannot draw with {sample_count} samples per pixel: the counts are 1 and 4 (4x MSAA), the two every WebGPU implementation supports. Use pass.target().sample_count().", label_text(label))));
    }
    Ok(())
}

impl GraphicsContext {
    /// Wraps a `wgpu` render pipeline made elsewhere, with the formats it draws into (C#:
    /// `WrapRenderPipeline(pipeline, colorFormats, depthFormat, sampleCount, label)`), so that the
    /// library's passes and renderers can use it.
    pub fn wrap_render_pipeline(&self, pipeline: wgpu::RenderPipeline, color_formats: &[wgpu::TextureFormat], depth_format: Option<wgpu::TextureFormat>, sample_count: u32, label: Option<&str>) -> GpuRenderPipeline {
        GpuRenderPipeline { handle: pipeline, color_formats: color_formats.to_vec(), depth_format, sample_count: sample_count.max(1), label: label.map(str::to_string) }
    }
}

/// A render pipeline and the formats it draws into (C#: `GpuRenderPipeline`).
#[derive(Clone, Debug)]
pub struct GpuRenderPipeline {
    handle: wgpu::RenderPipeline,
    color_formats: Vec<wgpu::TextureFormat>,
    depth_format: Option<wgpu::TextureFormat>,
    sample_count: u32,
    label: Option<String>,
}

impl GpuRenderPipeline {
    /// The `wgpu` pipeline (C#: `Handle`).
    pub fn handle(&self) -> &wgpu::RenderPipeline {
        &self.handle
    }

    /// The colour target formats (C#: `ColorFormats`).
    pub fn color_formats(&self) -> &[wgpu::TextureFormat] {
        &self.color_formats
    }

    /// The depth format, if the pipeline tests depth (C#: `DepthFormat`).
    pub fn depth_format(&self) -> Option<wgpu::TextureFormat> {
        self.depth_format
    }

    /// The sample count (C#: `SampleCount`).
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// The label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The bind-group layout of group `index` as the pipeline has it (C#: `GetBindGroupLayout`)
    /// - for a pipeline built without bind-group layouts, the one WebGPU derived from the shader.
    pub fn get_bind_group_layout(&self, index: u32) -> GpuBindGroupLayout {
        GpuBindGroupLayout::new(self.handle.get_bind_group_layout(index), &[], None)
    }
}

#[cfg(test)]
mod tests {
    use super::{check_parts, ColorTargetOptions, DepthOptions, StencilMasks};
    use crate::pipelines::{BlendPreset, DepthPreset};

    #[test]
    fn the_options_carry_the_csharp_defaults() {
        let color = ColorTargetOptions::default();
        assert_eq!(color.blend, BlendPreset::Opaque);
        assert_eq!(color.write_mask, wgpu::ColorWrites::ALL);
        assert_eq!(ColorTargetOptions::blended(BlendPreset::Additive).blend, BlendPreset::Additive);
        assert_eq!(DepthOptions::default().preset, DepthPreset::ReadWrite);
        assert_eq!(StencilMasks::default().read_mask, u32::MAX);
    }

    #[test]
    fn a_fragment_stage_without_a_colour_target_is_a_depth_only_pipeline_when_depth_is_set() {
        assert!(check_parts(true, 0, true, false, 1, Some("shadow")).is_ok(), "depth-only with a discarding fragment stage");
        assert!(check_parts(false, 0, true, false, 1, None).is_ok(), "depth-only without a fragment stage");
        let message = check_parts(true, 0, false, false, 1, Some("quad")).unwrap_err().to_string();
        assert!(message.contains("'quad' has a fragment stage but neither a colour target nor depth"), "{message}");
        assert!(check_parts(true, 1, false, false, 1, None).is_ok());
        assert!(check_parts(true, 1, false, true, 1, None).unwrap_err().to_string().contains("stencil"));
    }

    #[test]
    fn the_builder_accepts_one_and_four_samples_only() {
        assert!(check_parts(true, 1, true, false, 1, None).is_ok());
        assert!(check_parts(true, 1, true, false, 4, None).is_ok());
        for refused in [0, 2, 8] {
            let message = check_parts(true, 1, true, false, refused, Some("p")).unwrap_err().to_string();
            assert!(message.contains(&format!("cannot draw with {refused} samples per pixel")), "{message}");
        }
    }
}
