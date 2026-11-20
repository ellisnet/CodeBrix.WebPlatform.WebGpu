//! [`ComputePipelineBuilder`] and [`GpuComputePipeline`]: compute pipelines (C#:
//! `ComputePipelineBuilder`, `GpuComputePipeline`), and [`GpuPipelineLayout`], a shareable
//! pipeline layout (C#: `GpuPipelineLayout`).

use crate::context::{label_text, GraphicsContext, GraphicsException};
use crate::resources::{GpuBindGroupLayout, GpuShader};

/// A pipeline layout that several pipelines share (C#: `GpuPipelineLayout`), made by
/// [`GraphicsContext::create_pipeline_layout`]. Give it to
/// [`ComputePipelineBuilder::with_pipeline_layout`] or, by its handle, to
/// `RenderPipelineBuilder::with_pipeline_layout`.
#[derive(Clone, Debug)]
pub struct GpuPipelineLayout {
    handle: wgpu::PipelineLayout,
    bind_group_layouts: Vec<GpuBindGroupLayout>,
    label: Option<String>,
}

impl GpuPipelineLayout {
    /// The `wgpu` layout (C#: `Handle`).
    pub fn handle(&self) -> &wgpu::PipelineLayout {
        &self.handle
    }

    /// The bind-group layouts of groups 0, 1, ... (C#: `BindGroupLayouts`).
    pub fn bind_group_layouts(&self) -> &[GpuBindGroupLayout] {
        &self.bind_group_layouts
    }

    /// The label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

/// Builds a compute pipeline from a shader (C#: `ComputePipelineBuilder`), started by
/// [`GraphicsContext::create_compute_pipeline`]. The entry point defaults to `main`, as in C#.
/// C#'s `WithImmediates` / `WithImmediateSize` have no counterpart: a browser has no immediates.
pub struct ComputePipelineBuilder {
    context: GraphicsContext,
    shader: wgpu::ShaderModule,
    entry: String,
    layouts: Vec<GpuBindGroupLayout>,
    explicit_layout: Option<GpuPipelineLayout>,
    label: Option<String>,
}

impl GraphicsContext {
    /// Starts a compute pipeline from `shader` (C#: `CreateComputePipeline(shader)`).
    pub fn create_compute_pipeline(&self, shader: &GpuShader) -> ComputePipelineBuilder {
        ComputePipelineBuilder { context: self.clone(), shader: shader.handle().clone(), entry: "main".to_string(), layouts: Vec::new(), explicit_layout: None, label: shader.label().map(str::to_string) }
    }

    /// A pipeline layout of `bind_group_layouts` for groups 0, 1, ... (C#:
    /// `CreatePipelineLayout(immediateSize, label, layouts)`, without the immediate size a
    /// browser does not have).
    pub fn create_pipeline_layout(&self, label: Option<&str>, bind_group_layouts: &[&GpuBindGroupLayout]) -> GpuPipelineLayout {
        let handles: Vec<Option<&wgpu::BindGroupLayout>> = bind_group_layouts.iter().map(|layout| Some(layout.handle())).collect();
        let handle = self.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label, bind_group_layouts: &handles, immediate_size: 0 });
        GpuPipelineLayout { handle, bind_group_layouts: bind_group_layouts.iter().map(|layout| (*layout).clone()).collect(), label: label.map(str::to_string) }
    }
}

impl GraphicsContext {
    /// Wraps a `wgpu` compute pipeline made elsewhere (C#: `WrapComputePipeline`).
    pub fn wrap_compute_pipeline(&self, pipeline: wgpu::ComputePipeline, label: Option<&str>) -> GpuComputePipeline {
        GpuComputePipeline { handle: pipeline, label: label.map(str::to_string) }
    }

    /// Wraps a `wgpu` pipeline layout made elsewhere (C#: `WrapPipelineLayout`); its bind-group
    /// layouts read as empty.
    pub fn wrap_pipeline_layout(&self, layout: wgpu::PipelineLayout, label: Option<&str>) -> GpuPipelineLayout {
        GpuPipelineLayout { handle: layout, bind_group_layouts: Vec::new(), label: label.map(str::to_string) }
    }
}

impl ComputePipelineBuilder {
    /// The entry point (C#: `WithEntry`; default `main`).
    pub fn with_entry(mut self, entry_point: &str) -> ComputePipelineBuilder {
        self.entry = entry_point.to_string();
        self
    }

    /// Explicit bind-group layouts for groups 0, 1, ... instead of the layout WebGPU derives from
    /// the shader (C#: `WithBindGroupLayouts`).
    pub fn with_bind_group_layouts(mut self, bind_group_layouts: &[&GpuBindGroupLayout]) -> ComputePipelineBuilder {
        self.layouts = bind_group_layouts.iter().map(|layout| (*layout).clone()).collect();
        self
    }

    /// A shareable pipeline layout (C#: `WithPipelineLayout`).
    pub fn with_pipeline_layout(mut self, layout: &GpuPipelineLayout) -> ComputePipelineBuilder {
        self.explicit_layout = Some(layout.clone());
        self
    }

    /// The pipeline's label (C#: `WithLabel`).
    pub fn with_label(mut self, value: &str) -> ComputePipelineBuilder {
        self.label = Some(value.to_string());
        self
    }

    /// Makes the pipeline (C#: `Build`). WebGPU's own validation reports through
    /// `GraphicsContext::take_errors` - or await [`ComputePipelineBuilder::build_checked`].
    pub fn build(self) -> Result<GpuComputePipeline, GraphicsException> {
        if self.entry.is_empty() {
            return Err(GraphicsException::new(format!("The compute pipeline{} needs an entry point; name the shader's @compute function with with_entry.", label_text(self.label.as_deref()))));
        }
        let device = self.context.device();
        let layout = match &self.explicit_layout {
            Some(layout) => Some(layout.handle().clone()),
            None if self.layouts.is_empty() => None,
            None => {
                let handles: Vec<Option<&wgpu::BindGroupLayout>> = self.layouts.iter().map(|layout| Some(layout.handle())).collect();
                Some(device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: self.label.as_deref(), bind_group_layouts: &handles, immediate_size: 0 }))
            }
        };
        let handle = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: self.label.as_deref(),
            layout: layout.as_ref(),
            module: &self.shader,
            entry_point: Some(&self.entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Ok(GpuComputePipeline { handle, label: self.label })
    }

    /// As [`ComputePipelineBuilder::build`], and waits for WebGPU's verdict: Err carries WebGPU's
    /// validation message (C#: what `Build` does before it returns or throws).
    pub async fn build_checked(self) -> Result<GpuComputePipeline, GraphicsException> {
        let context = self.context.clone();
        let what = format!("The compute pipeline{}", label_text(self.label.as_deref()));
        context.validate_named(&what, move |_| self.build()).await?
    }
}

/// A compute pipeline (C#: `GpuComputePipeline`).
#[derive(Clone, Debug)]
pub struct GpuComputePipeline {
    handle: wgpu::ComputePipeline,
    label: Option<String>,
}

impl GpuComputePipeline {
    /// The `wgpu` pipeline (C#: `Handle`).
    pub fn handle(&self) -> &wgpu::ComputePipeline {
        &self.handle
    }

    /// The label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The bind-group layout of group `index` as the pipeline has it (C#: `GetBindGroupLayout`) -
    /// for a pipeline built without layouts, the one WebGPU derived from the shader.
    pub fn get_bind_group_layout(&self, index: u32) -> GpuBindGroupLayout {
        GpuBindGroupLayout::new(self.handle.get_bind_group_layout(index), &[], None)
    }
}
