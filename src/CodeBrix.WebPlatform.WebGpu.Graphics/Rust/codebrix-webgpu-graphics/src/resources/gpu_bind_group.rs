//! [`GpuBindGroupLayout`] and [`GpuBindGroup`]: the layout of a set of bindings and a set that
//! follows it (C#: `GpuBindGroupLayout`, `GpuBindGroup`).

use std::rc::Rc;

use crate::resources::BindingLayoutEntry;

/// A bind-group layout and the entries it was made from (C#: `GpuBindGroupLayout`). Clone it to
/// share it (the clones are the same layout).
#[derive(Clone, Debug)]
pub struct GpuBindGroupLayout {
    handle: wgpu::BindGroupLayout,
    entries: Rc<Vec<BindingLayoutEntry>>,
    label: Option<String>,
}

impl GpuBindGroupLayout {
    pub(crate) fn new(handle: wgpu::BindGroupLayout, entries: &[BindingLayoutEntry], label: Option<&str>) -> GpuBindGroupLayout {
        GpuBindGroupLayout { handle, entries: Rc::new(entries.to_vec()), label: label.map(str::to_string) }
    }

    /// The `wgpu` layout (C#: `Handle`).
    pub fn handle(&self) -> &wgpu::BindGroupLayout {
        &self.handle
    }

    /// The entries (C#: `Entries`; empty for a layout taken from a pipeline, whose entries the
    /// shader decides).
    pub fn entries(&self) -> &[BindingLayoutEntry] {
        &self.entries
    }

    /// The label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

/// A bind group: the resources one group of a shader's bindings uses (C#: `GpuBindGroup`).
#[derive(Clone, Debug)]
pub struct GpuBindGroup {
    handle: wgpu::BindGroup,
    layout: GpuBindGroupLayout,
    label: Option<String>,
}

impl GpuBindGroup {
    pub(crate) fn new(handle: wgpu::BindGroup, layout: GpuBindGroupLayout, label: Option<&str>) -> GpuBindGroup {
        GpuBindGroup { handle, layout, label: label.map(str::to_string) }
    }

    /// The `wgpu` bind group (C#: `Handle`).
    pub fn handle(&self) -> &wgpu::BindGroup {
        &self.handle
    }

    /// The layout it follows (C#: `Layout`).
    pub fn layout(&self) -> &GpuBindGroupLayout {
        &self.layout
    }

    /// The label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}
