//! The resource factories of [`GraphicsContext`]: buffers, textures, samplers, shaders, bind-group
//! layouts and bind groups (C#: `GraphicsContext.Resources.cs`), with the options structs that
//! stand for the C# default arguments.

use wgpu::util::DeviceExt;

use crate::context::label_text;
use crate::context::{GraphicsContext, GraphicsException, ShaderCompilationException};
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuShader, GpuTexture, SamplerDescription, ShaderDiagnostics, TextureFormats};

/// The defaulted arguments of [`GraphicsContext::create_texture_2d`] (C#: `mipLevelCount = 1`,
/// `label = null`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texture2DOptions {
    /// The number of mip levels (1: none; see [`GpuTexture::full_mip_chain_length`]).
    pub mip_level_count: u32,
    /// The label.
    pub label: Option<String>,
}

impl Default for Texture2DOptions {
    fn default() -> Texture2DOptions {
        Texture2DOptions { mip_level_count: 1, label: None }
    }
}

/// The defaulted arguments of [`GraphicsContext::create_texture_from_rgba`] (C#: `mipmaps =
/// false`, `srgb = false`, `label = null`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RgbaTextureOptions {
    /// True for a full mip chain, made on the CPU with a 2 x 2 box filter.
    pub mipmaps: bool,
    /// True for `Rgba8UnormSrgb` (sampling decodes to linear; right for colour images); false for
    /// `Rgba8Unorm` (data such as normal maps).
    pub srgb: bool,
    /// The label.
    pub label: Option<String>,
}

/// The defaulted arguments of [`GraphicsContext::create_render_texture`] (C#: `format =
/// RGBA8Unorm`, `label = null`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderTextureOptions {
    /// The format.
    pub format: wgpu::TextureFormat,
    /// The label.
    pub label: Option<String>,
}

impl Default for RenderTextureOptions {
    fn default() -> RenderTextureOptions {
        RenderTextureOptions { format: wgpu::TextureFormat::Rgba8Unorm, label: None }
    }
}

/// The defaulted arguments of [`GraphicsContext::create_depth_texture`] (C#: `format =
/// Depth32Float`, `label = null`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepthTextureOptions {
    /// The depth format.
    pub format: wgpu::TextureFormat,
    /// The label.
    pub label: Option<String>,
}

impl Default for DepthTextureOptions {
    fn default() -> DepthTextureOptions {
        DepthTextureOptions { format: wgpu::TextureFormat::Depth32Float, label: None }
    }
}

impl GraphicsContext {
    /// Creates a buffer (C#: `CreateBuffer(size, usage, label)`). The size is rounded up to a
    /// multiple of 4; `COPY_SRC | COPY_DST` are added unless the usage maps the buffer.
    pub fn create_buffer(&self, size: u64, usage: wgpu::BufferUsages, label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        if size == 0 {
            return Err(GraphicsException::new(format!("The buffer{} needs at least one byte; ask for a size of 1 or more.", label_text(label))));
        }
        let size = (size + 3) & !3;
        let usage = buffer_usage(usage);
        let handle = self.device().create_buffer(&wgpu::BufferDescriptor { label, size, usage, mapped_at_creation: false });
        Ok(GpuBuffer::new(self.clone(), handle, size, usage, label))
    }

    /// Creates a buffer holding `data` (C#: `CreateBuffer<T>(data, usage, label)`; empty data
    /// makes a 4-byte buffer).
    pub fn create_buffer_from<T: bytemuck::Pod>(&self, data: &[T], usage: wgpu::BufferUsages, label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        let bytes: &[u8] = bytemuck::cast_slice(data);
        if bytes.is_empty() {
            return self.create_buffer(4, usage, label);
        }
        let mut contents = bytes.to_vec();
        contents.resize((contents.len() + 3) & !3, 0);
        let usage = buffer_usage(usage);
        let handle = self.device().create_buffer_init(&wgpu::util::BufferInitDescriptor { label, contents: &contents, usage });
        Ok(GpuBuffer::new(self.clone(), handle, contents.len() as u64, usage, label))
    }

    /// Creates a vertex buffer holding `vertices` (C#: `CreateVertexBuffer<T>`).
    pub fn create_vertex_buffer<T: bytemuck::Pod>(&self, vertices: &[T], label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        self.create_buffer_from(vertices, wgpu::BufferUsages::VERTEX, label)
    }

    /// Creates a 16-bit index buffer (C#: `CreateIndexBuffer(ReadOnlySpan<ushort>)`).
    pub fn create_index_buffer_u16(&self, indices: &[u16], label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        self.create_buffer_from(indices, wgpu::BufferUsages::INDEX, label)
    }

    /// Creates a 32-bit index buffer (C#: `CreateIndexBuffer(ReadOnlySpan<uint>)`).
    pub fn create_index_buffer_u32(&self, indices: &[u32], label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        self.create_buffer_from(indices, wgpu::BufferUsages::INDEX, label)
    }

    /// Creates a uniform buffer holding one value, its size rounded up to 16 bytes (C#:
    /// `CreateUniformBufferFrom<T>`). Lay the value out as WGSL does (a `vec3` takes 16 bytes).
    pub fn create_uniform_buffer_from<T: bytemuck::Pod>(&self, value: &T, label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        let size = ((std::mem::size_of::<T>() as u64) + 15) & !15;
        let buffer = self.create_buffer(size.max(16), wgpu::BufferUsages::UNIFORM, label)?;
        buffer.write_value(value, 0)?;
        Ok(buffer)
    }

    /// Creates an empty uniform buffer, its size rounded up to 16 bytes (C#: `CreateUniformBuffer`).
    pub fn create_uniform_buffer(&self, size: u64, label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        self.create_buffer((size + 15) & !15, wgpu::BufferUsages::UNIFORM, label)
    }

    /// Creates a storage buffer holding `data` (C#: `CreateStorageBuffer<T>(data)`).
    pub fn create_storage_buffer_from<T: bytemuck::Pod>(&self, data: &[T], label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        self.create_buffer_from(data, wgpu::BufferUsages::STORAGE, label)
    }

    /// Creates an empty storage buffer (C#: `CreateStorageBuffer(size)`).
    pub fn create_storage_buffer(&self, size: u64, label: Option<&str>) -> Result<GpuBuffer, GraphicsException> {
        self.create_buffer(size, wgpu::BufferUsages::STORAGE, label)
    }

    /// Creates a 2D texture with a default view (C#: `CreateTexture2D`).
    pub fn create_texture_2d(&self, width: u32, height: u32, format: wgpu::TextureFormat, usage: wgpu::TextureUsages, options: Texture2DOptions) -> Result<GpuTexture, GraphicsException> {
        let label = options.label.as_deref();
        if width < 1 || height < 1 {
            return Err(GraphicsException::new(format!("The texture{} must be at least 1 x 1 pixels; it was asked for {width} x {height}.", label_text(label))));
        }
        let limit = self.limits().max_texture_dimension_2d;
        if width > limit || height > limit {
            return Err(GraphicsException::new(format!("The texture{} of {width} x {height} pixels is larger than the device allows ({limit} per side); make it smaller.", label_text(label))));
        }
        let full = GpuTexture::full_mip_chain_length(width, height);
        if options.mip_level_count < 1 || options.mip_level_count > full {
            return Err(GraphicsException::new(format!("A {width} x {height} texture has 1 to {full} mip levels; {} were asked for.", options.mip_level_count)));
        }
        let handle = self.device().create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: options.mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let view = handle.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(GpuTexture::new(self.clone(), handle, view, width, height, format, options.mip_level_count, usage, false, label))
    }

    /// Creates a texture of `layers` 2D layers (C#: `CreateTextureArray`; the six faces of a cube
    /// map are an array of six). Its default view is a 2D-array view (a 2D view for one layer);
    /// [`GpuTexture::create_cube_view`] gives a cube view of six layers.
    pub fn create_texture_array(&self, width: u32, height: u32, layers: u32, format: wgpu::TextureFormat, usage: wgpu::TextureUsages, options: Texture2DOptions) -> Result<GpuTexture, GraphicsException> {
        let label = options.label.as_deref();
        if width < 1 || height < 1 {
            return Err(GraphicsException::new(format!("The texture array{} must be at least 1 x 1 pixels; it was asked for {width} x {height}.", label_text(label))));
        }
        let limits = self.limits();
        if width > limits.max_texture_dimension_2d || height > limits.max_texture_dimension_2d {
            return Err(GraphicsException::new(format!("The texture array{} of {width} x {height} pixels is larger than the device allows ({} per side); make it smaller.", label_text(label), limits.max_texture_dimension_2d)));
        }
        if layers < 1 || layers > limits.max_texture_array_layers {
            return Err(GraphicsException::new(format!("The device allows 1 to {} array layers; the texture array{} was asked for {layers}.", limits.max_texture_array_layers, label_text(label))));
        }
        let full = GpuTexture::full_mip_chain_length(width, height);
        if options.mip_level_count < 1 || options.mip_level_count > full {
            return Err(GraphicsException::new(format!("A {width} x {height} texture has 1 to {full} mip levels; {} were asked for.", options.mip_level_count)));
        }
        let handle = self.device().create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d { width, height, depth_or_array_layers: layers },
            mip_level_count: options.mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let dimension = if layers > 1 { wgpu::TextureViewDimension::D2Array } else { wgpu::TextureViewDimension::D2 };
        let view = handle.create_view(&wgpu::TextureViewDescriptor { dimension: Some(dimension), ..wgpu::TextureViewDescriptor::default() });
        Ok(GpuTexture::new(self.clone(), handle, view, width, height, format, options.mip_level_count, usage, false, label).with_array_layers(layers))
    }

    /// Creates a sampled RGBA8 texture from RGBA pixels (red first, tightly packed) (C#:
    /// `CreateTextureFromRgba`).
    pub fn create_texture_from_rgba(&self, width: u32, height: u32, rgba: &[u8], options: RgbaTextureOptions) -> Result<GpuTexture, GraphicsException> {
        let format = if options.srgb { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm };
        let mip_level_count = if options.mipmaps { GpuTexture::full_mip_chain_length(width, height) } else { 1 };
        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC;
        let texture = self.create_texture_2d(width, height, format, usage, Texture2DOptions { mip_level_count, label: options.label })?;
        texture.write_rgba(rgba)?;
        Ok(texture)
    }

    /// Creates a colour texture to render into (render attachment, sampled, copyable) (C#:
    /// `CreateRenderTexture`).
    pub fn create_render_texture(&self, width: u32, height: u32, options: RenderTextureOptions) -> Result<GpuTexture, GraphicsException> {
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST;
        self.create_texture_2d(width, height, options.format, usage, Texture2DOptions { mip_level_count: 1, label: options.label })
    }

    /// Creates a depth texture (render attachment; also copyable and sampled for `Depth32Float`)
    /// (C#: `CreateDepthTexture`).
    pub fn create_depth_texture(&self, width: u32, height: u32, options: DepthTextureOptions) -> Result<GpuTexture, GraphicsException> {
        if !TextureFormats::is_depth(options.format) {
            return Err(GraphicsException::new(format!("{:?} is not a depth format; use Depth32Float, Depth24Plus or another depth format.", options.format)));
        }
        let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        if options.format == wgpu::TextureFormat::Depth32Float {
            usage |= wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING;
        }
        self.create_texture_2d(width, height, options.format, usage, Texture2DOptions { mip_level_count: 1, label: options.label })
    }

    /// Creates a sampler (C#: `CreateSampler(description)`; pass
    /// `&SamplerDescription::default()` for C#'s null).
    pub fn create_sampler(&self, description: &SamplerDescription) -> GpuSampler {
        let handle = self.device().create_sampler(&description.to_wgpu());
        GpuSampler::new(handle, description.clone())
    }

    /// Creates a shader from WGSL (C#: `CreateShader`). The browser compiles it in the
    /// background: a shader that does not compile reports through
    /// [`GraphicsContext::take_errors`]; await [`GraphicsContext::try_create_shader`] to get the
    /// compiler's messages instead.
    pub fn create_shader(&self, wgsl: &str, label: Option<&str>) -> GpuShader {
        let handle = self.device().create_shader_module(wgpu::ShaderModuleDescriptor { label, source: wgpu::ShaderSource::Wgsl(wgsl.into()) });
        GpuShader::new(handle, wgsl, label)
    }

    /// Creates a shader and waits for the browser's compiler (C#: `TryCreateShader(wgsl, label,
    /// out shader, out diagnostics)`; async here). Ok: the shader and the compiler's warnings and
    /// notes (often none). Err: the compiler's errors, with lines and columns.
    pub async fn try_create_shader(&self, wgsl: &str, label: Option<&str>) -> Result<(GpuShader, ShaderDiagnostics), ShaderCompilationException> {
        let validation = self.device().push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = self.create_shader(wgsl, label);
        let scope = validation.pop();
        let diagnostics = shader.diagnostics().await;
        let refused = scope.await;
        if diagnostics.has_errors() {
            return Err(ShaderCompilationException::new(label, diagnostics.to_string()));
        }
        if let Some(error) = refused {
            return Err(ShaderCompilationException::new(label, error.to_string()));
        }
        Ok((shader, diagnostics))
    }

    /// Creates a bind-group layout (C#: `CreateBindGroupLayout(label, entries)`).
    pub fn create_bind_group_layout(&self, label: Option<&str>, entries: &[BindingLayoutEntry]) -> GpuBindGroupLayout {
        let wgpu_entries: Vec<wgpu::BindGroupLayoutEntry> = entries.iter().map(BindingLayoutEntry::to_wgpu).collect();
        let handle = self.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label, entries: &wgpu_entries });
        GpuBindGroupLayout::new(handle, entries, label)
    }

    /// Creates a bind group that follows `layout` (C#: `CreateBindGroup(layout, label, entries)`).
    pub fn create_bind_group(&self, layout: &GpuBindGroupLayout, label: Option<&str>, entries: &[BindingEntry]) -> GpuBindGroup {
        let wgpu_entries: Vec<wgpu::BindGroupEntry<'_>> = entries.iter().map(BindingEntry::to_wgpu).collect();
        let handle = self.device().create_bind_group(&wgpu::BindGroupDescriptor { label, layout: layout.handle(), entries: &wgpu_entries });
        GpuBindGroup::new(handle, layout.clone(), label)
    }

    /// Wraps a `wgpu` buffer made elsewhere (C#: `WrapBuffer`).
    pub fn wrap_buffer(&self, buffer: wgpu::Buffer, label: Option<&str>) -> GpuBuffer {
        let (size, usage) = (buffer.size(), buffer.usage());
        GpuBuffer::new(self.clone(), buffer, size, usage, label)
    }

    /// Wraps a `wgpu` sampler made elsewhere (C#: `WrapSampler`); its description reads as the
    /// defaults, since a `wgpu` sampler does not say what it was made with.
    pub fn wrap_sampler(&self, sampler: wgpu::Sampler, label: Option<&str>) -> GpuSampler {
        GpuSampler::new(sampler, SamplerDescription { label: label.map(str::to_string), ..SamplerDescription::default() })
    }

    /// Wraps a `wgpu` shader module made elsewhere (C#: `WrapShader`); its source reads as empty.
    pub fn wrap_shader(&self, module: wgpu::ShaderModule, label: Option<&str>) -> GpuShader {
        GpuShader::new(module, "", label)
    }

    /// Wraps a `wgpu` bind-group layout made elsewhere (C#: `WrapBindGroupLayout`); its entries
    /// read as empty.
    pub fn wrap_bind_group_layout(&self, layout: wgpu::BindGroupLayout, label: Option<&str>) -> GpuBindGroupLayout {
        GpuBindGroupLayout::new(layout, &[], label)
    }

    /// Wraps a `wgpu` bind group made elsewhere, following `layout` (C#: `WrapBindGroup(group,
    /// layout, label)`; the layout is needed here, wrap it first with `wrap_bind_group_layout`).
    pub fn wrap_bind_group(&self, group: wgpu::BindGroup, layout: &GpuBindGroupLayout, label: Option<&str>) -> GpuBindGroup {
        GpuBindGroup::new(group, layout.clone(), label)
    }

    /// Wraps a `wgpu` 2D texture made elsewhere, with its default view (C#: `WrapTexture`).
    pub fn wrap_texture(&self, texture: wgpu::Texture, label: Option<&str>) -> GpuTexture {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (width, height, format, mips, usage) = (texture.width(), texture.height(), texture.format(), texture.mip_level_count(), texture.usage());
        let layers = texture.depth_or_array_layers();
        GpuTexture::new(self.clone(), texture, view, width, height, format, mips, usage, true, label).with_array_layers(layers)
    }
}

/// The usages a factory gives a buffer: the asked-for ones, plus both copy usages unless it maps.
pub(crate) fn buffer_usage(usage: wgpu::BufferUsages) -> wgpu::BufferUsages {
    if usage.intersects(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::MAP_WRITE) {
        usage
    } else {
        usage | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST
    }
}

#[cfg(test)]
mod tests {
    use super::{buffer_usage, DepthTextureOptions, RenderTextureOptions, RgbaTextureOptions, Texture2DOptions};

    #[test]
    fn factories_add_the_copy_usages_except_for_map_buffers() {
        assert_eq!(buffer_usage(wgpu::BufferUsages::VERTEX), wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST);
        assert_eq!(buffer_usage(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST), wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST);
    }

    #[test]
    fn the_options_carry_the_csharp_defaults() {
        assert_eq!(Texture2DOptions::default().mip_level_count, 1);
        assert_eq!(RgbaTextureOptions::default(), RgbaTextureOptions { mipmaps: false, srgb: false, label: None });
        assert_eq!(RenderTextureOptions::default().format, wgpu::TextureFormat::Rgba8Unorm);
        assert_eq!(DepthTextureOptions::default().format, wgpu::TextureFormat::Depth32Float);
    }
}
