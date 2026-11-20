//! [`GpuRenderBundle`]: a recorded list of draw commands a render pass replays (C#:
//! `GpuRenderBundle`).

/// A recorded list of draw commands that a render pass replays cheaply with
/// [`RenderPass::execute_bundle`](crate::RenderPass::execute_bundle) (C#: `GpuRenderBundle`), for
/// geometry drawn the same way every frame. Recorded by a
/// [`RenderBundleBuilder`](crate::RenderBundleBuilder). A cheap handle: clones are the same bundle.
#[derive(Clone, Debug)]
pub struct GpuRenderBundle {
    handle: wgpu::RenderBundle,
    color_formats: Vec<wgpu::TextureFormat>,
    depth_format: Option<wgpu::TextureFormat>,
    sample_count: u32,
    command_count: usize,
    label: Option<String>,
}

impl GpuRenderBundle {
    pub(crate) fn new(handle: wgpu::RenderBundle, color_formats: Vec<wgpu::TextureFormat>, depth_format: Option<wgpu::TextureFormat>, sample_count: u32, command_count: usize, label: Option<String>) -> GpuRenderBundle {
        GpuRenderBundle { handle, color_formats, depth_format, sample_count, command_count, label }
    }

    /// The `wgpu` bundle (C#: `Handle`).
    pub fn handle(&self) -> &wgpu::RenderBundle {
        &self.handle
    }

    /// The colour formats of the passes it can run in (C#: `ColorFormats`).
    pub fn color_formats(&self) -> &[wgpu::TextureFormat] {
        &self.color_formats
    }

    /// The depth format of the passes it can run in, or None (C#: `DepthFormat`).
    pub fn depth_format(&self) -> Option<wgpu::TextureFormat> {
        self.depth_format
    }

    /// The sample count of the passes it can run in (C#: `SampleCount`).
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// The number of commands it replays.
    pub fn command_count(&self) -> usize {
        self.command_count
    }

    /// The label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}
