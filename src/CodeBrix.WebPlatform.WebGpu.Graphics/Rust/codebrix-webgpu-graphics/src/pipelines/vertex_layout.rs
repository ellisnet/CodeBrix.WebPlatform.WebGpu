//! [`VertexLayout`]: how a vertex buffer's bytes map to shader locations, described element by
//! element ([`VertexLayoutBuilder`]) or by a vertex type ([`VertexType`]) (C#: `VertexLayout`,
//! `VertexLayoutBuilder`, `VertexElement`, `VertexFormats`).

use std::fmt;

/// One attribute of a vertex (C#: `VertexElement`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VertexElement {
    /// The shader location (`@location(n)`).
    pub location: u32,
    /// The format.
    pub format: wgpu::VertexFormat,
    /// The byte offset in the vertex.
    pub offset: u64,
}

/// The layout of one vertex buffer (C#: `VertexLayout`).
#[derive(Clone, Debug, PartialEq)]
pub struct VertexLayout {
    array_stride: u64,
    step_mode: wgpu::VertexStepMode,
    elements: Vec<VertexElement>,
    attributes: Vec<wgpu::VertexAttribute>,
}

/// The defaulted arguments of [`VertexLayout::describe`] (C#: `stepMode = Vertex`,
/// `firstLocation = 0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VertexLayoutOptions {
    /// Per vertex or per instance.
    pub step_mode: wgpu::VertexStepMode,
    /// The location of the first element.
    pub first_location: u32,
}

impl Default for VertexLayoutOptions {
    fn default() -> VertexLayoutOptions {
        VertexLayoutOptions { step_mode: wgpu::VertexStepMode::Vertex, first_location: 0 }
    }
}

impl VertexLayout {
    /// A layout from its parts (C#: the constructor).
    pub fn new(array_stride: u64, step_mode: wgpu::VertexStepMode, elements: Vec<VertexElement>) -> VertexLayout {
        let attributes = elements.iter().map(|e| wgpu::VertexAttribute { format: e.format, offset: e.offset, shader_location: e.location }).collect();
        VertexLayout { array_stride, step_mode, elements, attributes }
    }

    /// Starts describing a layout element by element (C#: `VertexLayout.Describe(stepMode,
    /// firstLocation)`).
    pub fn describe(options: VertexLayoutOptions) -> VertexLayoutBuilder {
        VertexLayoutBuilder { step_mode: options.step_mode, location: options.first_location, offset: 0, elements: Vec::new() }
    }

    /// The layout of a vertex type (C#: `VertexLayout.FromStruct<T>`; Rust has no reflection, so
    /// the type says it through [`VertexType`]).
    pub fn from_type<T: VertexType>(options: VertexLayoutOptions) -> VertexLayout {
        T::vertex_layout(options)
    }

    /// The bytes from one vertex to the next (C#: `ArrayStride`).
    pub fn array_stride(&self) -> u64 {
        self.array_stride
    }

    /// Per vertex or per instance (C#: `StepMode`).
    pub fn step_mode(&self) -> wgpu::VertexStepMode {
        self.step_mode
    }

    /// The elements (C#: `Elements`).
    pub fn elements(&self) -> &[VertexElement] {
        &self.elements
    }

    /// The location after the highest one used, for a second buffer's first location (C#:
    /// `NextLocation`).
    pub fn next_location(&self) -> u32 {
        self.elements.iter().map(|e| e.location + 1).max().unwrap_or(0)
    }

    /// The `wgpu` buffer layout, borrowing this one.
    pub fn to_wgpu(&self) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout { array_stride: self.array_stride, step_mode: self.step_mode, attributes: &self.attributes }
    }
}

impl fmt::Display for VertexLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} bytes, {:?}:", self.array_stride, self.step_mode)?;
        for element in &self.elements {
            write!(f, " @{} {:?}+{}", element.location, element.format, element.offset)?;
        }
        Ok(())
    }
}

/// Builds a [`VertexLayout`] element by element, in memory order (C#: `VertexLayoutBuilder`).
#[derive(Clone, Debug)]
pub struct VertexLayoutBuilder {
    step_mode: wgpu::VertexStepMode,
    location: u32,
    offset: u64,
    elements: Vec<VertexElement>,
}

impl VertexLayoutBuilder {
    /// Adds an element at the next location and offset (C#: `Add(format)`).
    pub fn add(self, format: wgpu::VertexFormat) -> VertexLayoutBuilder {
        let location = self.location;
        self.add_at(format, location)
    }

    /// Adds an element at an explicit location; later elements continue after it (C#:
    /// `Add(format, shaderLocation)`).
    pub fn add_at(mut self, format: wgpu::VertexFormat, shader_location: u32) -> VertexLayoutBuilder {
        self.elements.push(VertexElement { location: shader_location, format, offset: self.offset });
        self.offset += VertexFormats::size_of(format);
        self.location = shader_location + 1;
        self
    }

    /// Skips bytes the shader does not read (C#: `Skip`).
    pub fn skip(mut self, bytes: u64) -> VertexLayoutBuilder {
        self.offset += bytes;
        self
    }

    /// The layout; `array_stride` 0 means the size of the elements described (C#: `Build`).
    pub fn build(self, array_stride: u64) -> VertexLayout {
        VertexLayout::new(if array_stride == 0 { self.offset } else { array_stride }, self.step_mode, self.elements)
    }
}

/// A vertex type that knows its layout (C#: a struct with `[VertexElement]` attributes read by
/// `VertexLayout.FromStruct<T>`). Implement it by hand with [`VertexLayout::describe`]:
///
/// ```
/// use codebrix_webgpu_graphics::{VertexLayout, VertexLayoutOptions, VertexType};
///
/// #[repr(C)]
/// #[derive(Clone, Copy)]
/// struct QuadVertex { position: [f32; 2], uv: [f32; 2] }
///
/// impl VertexType for QuadVertex {
///     fn vertex_layout(options: VertexLayoutOptions) -> VertexLayout {
///         VertexLayout::describe(options).add(wgpu::VertexFormat::Float32x2).add(wgpu::VertexFormat::Float32x2).build(std::mem::size_of::<QuadVertex>() as u64)
///     }
/// }
/// # use codebrix_webgpu_graphics::wgpu;
/// assert_eq!(QuadVertex::vertex_layout(VertexLayoutOptions::default()).array_stride(), 16);
/// ```
pub trait VertexType {
    /// The layout of a buffer of this type.
    fn vertex_layout(options: VertexLayoutOptions) -> VertexLayout;
}

/// Facts about vertex formats (C#: `VertexFormats`).
pub struct VertexFormats;

impl VertexFormats {
    /// The bytes one element of the format takes (C#: `SizeOf`).
    pub fn size_of(format: wgpu::VertexFormat) -> u64 {
        format.size()
    }
}

#[cfg(test)]
mod tests {
    use super::{VertexLayout, VertexLayoutOptions};

    #[test]
    fn describe_places_elements_in_order() {
        let layout = VertexLayout::describe(VertexLayoutOptions::default()).add(wgpu::VertexFormat::Float32x3).skip(4).add(wgpu::VertexFormat::Float32x2).build(0);
        assert_eq!(layout.array_stride(), 24);
        assert_eq!(layout.elements()[1].offset, 16);
        assert_eq!(layout.elements()[1].location, 1);
        assert_eq!(layout.next_location(), 2);
    }

    #[test]
    fn an_explicit_location_and_stride_and_instance_steps() {
        let options = VertexLayoutOptions { step_mode: wgpu::VertexStepMode::Instance, first_location: 4 };
        let layout = VertexLayout::describe(options).add(wgpu::VertexFormat::Float32x4).add_at(wgpu::VertexFormat::Unorm8x4, 9).add(wgpu::VertexFormat::Float32).build(64);
        assert_eq!(layout.array_stride(), 64);
        assert_eq!(layout.elements().iter().map(|e| e.location).collect::<Vec<_>>(), vec![4, 9, 10]);
        assert_eq!(layout.to_wgpu().attributes[2].offset, 20);
        assert_eq!(layout.step_mode(), wgpu::VertexStepMode::Instance);
        assert!(layout.to_string().starts_with("64 bytes, Instance:"));
    }
}
