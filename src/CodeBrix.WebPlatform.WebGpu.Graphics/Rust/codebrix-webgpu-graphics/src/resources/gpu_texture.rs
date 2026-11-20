//! [`GpuTexture`]: a 2D WebGPU texture (or an array of 2D layers, such as the six faces of a cube
//! map) with its default view, written from pixels (C#: `GpuTexture`).

use crate::context::{GraphicsContext, GraphicsException};
use crate::presentation::RenderTargetView;
use crate::resources::TextureFormats;

/// A 2D WebGPU texture and its default view (C#: `GpuTexture`). Made by the context's
/// `create_texture*` factories; dropped to release it. A texture made by
/// [`GraphicsContext::create_texture_array`] has more than one array layer; its default view is a
/// 2D-array view, and [`GpuTexture::create_cube_view`] views six layers as a cube.
pub struct GpuTexture {
    context: GraphicsContext,
    handle: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    mip_level_count: u32,
    usage: wgpu::TextureUsages,
    array_layers: u32,
    view_layer: u32,
    wrapped: bool,
    label: Option<String>,
}

impl GpuTexture {
    pub(crate) fn new(context: GraphicsContext, handle: wgpu::Texture, view: wgpu::TextureView, width: u32, height: u32, format: wgpu::TextureFormat, mip_level_count: u32, usage: wgpu::TextureUsages, wrapped: bool, label: Option<&str>) -> GpuTexture {
        GpuTexture { context, handle, view, width, height, format, mip_level_count, usage, array_layers: 1, view_layer: 0, wrapped, label: label.map(str::to_string) }
    }

    /// The same texture, recorded as having `array_layers` layers (the array factory's).
    pub(crate) fn with_array_layers(mut self, array_layers: u32) -> GpuTexture {
        self.array_layers = array_layers;
        self
    }

    /// The `wgpu` texture (C#: `Handle`).
    pub fn handle(&self) -> &wgpu::Texture {
        &self.handle
    }

    /// The default view (all mip levels) (C#: `View`).
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// The width in pixels (C#: `Width`).
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels (C#: `Height`).
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The format (C#: `Format`).
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// The number of mip levels (C#: `MipLevelCount`).
    pub fn mip_level_count(&self) -> u32 {
        self.mip_level_count
    }

    /// The usages (C#: `Usage`).
    pub fn usage(&self) -> wgpu::TextureUsages {
        self.usage
    }

    /// The number of array layers: 1 for an ordinary 2D texture, 6 for a cube map's faces (C#:
    /// `DepthOrArrayLayers`).
    pub fn array_layer_count(&self) -> u32 {
        self.array_layers
    }

    /// A cube view of a six-layer texture, every mip level (C#: `CreateCubeView`, which returns a
    /// view object; here it is the `wgpu` view). Layers +X, -X, +Y, -Y, +Z, -Z.
    pub fn create_cube_view(&self, label: Option<&str>) -> Result<wgpu::TextureView, GraphicsException> {
        if self.array_layers != 6 {
            return Err(GraphicsException::new(format!("A cube view needs a texture of six array layers; this one has {}. Make it with create_texture_array(.., 6, ..).", self.array_layers)));
        }
        Ok(self.handle.create_view(&wgpu::TextureViewDescriptor { label, dimension: Some(wgpu::TextureViewDimension::Cube), array_layer_count: Some(6), ..wgpu::TextureViewDescriptor::default() }))
    }

    /// A 2D view of ONE array layer (C#: `CreateLayerView(layer, label)`): a texture that shares
    /// this one's memory, whose [`GpuTexture::view`] shows layer `layer` (every mip level) and
    /// whose writes and reads go to that layer. An animated texture's frames are such views.
    pub fn create_layer_view(&self, layer: u32, label: Option<&str>) -> Result<GpuTexture, GraphicsException> {
        if layer >= self.array_layers {
            return Err(GraphicsException::new(format!("The texture has {} array layer(s); layer {layer} does not exist.", self.array_layers)));
        }
        let view = self.handle.create_view(&wgpu::TextureViewDescriptor { label, dimension: Some(wgpu::TextureViewDimension::D2), base_array_layer: layer, array_layer_count: Some(1), ..wgpu::TextureViewDescriptor::default() });
        let label = label.map(str::to_string).or_else(|| self.label.clone());
        Ok(GpuTexture { context: self.context.clone(), handle: self.handle.clone(), view, width: self.width, height: self.height, format: self.format, mip_level_count: self.mip_level_count, usage: self.usage, array_layers: 1, view_layer: layer, wrapped: self.wrapped, label })
    }

    /// The array layer this texture's view shows: 0, or the layer of a
    /// [`GpuTexture::create_layer_view`] view (C#: `ViewLayer`).
    pub fn view_layer(&self) -> u32 {
        self.view_layer
    }

    /// The dimension of the `wgpu` texture (C#: `Dimension`; always D2 for the library's factories).
    pub fn dimension(&self) -> wgpu::TextureDimension {
        self.handle.dimension()
    }

    /// The sample count (C#: `SampleCount`).
    pub fn sample_count(&self) -> u32 {
        self.handle.sample_count()
    }

    /// The context the texture belongs to.
    pub fn context(&self) -> &GraphicsContext {
        &self.context
    }

    /// Frees the texture's GPU memory now, even while other handles still name it (C#: `Destroy`);
    /// dropping the last handle frees it too. Commands recorded with it afterwards fail validation.
    pub fn destroy(&self) {
        self.handle.destroy();
    }

    /// True when the texture was made outside the library and wrapped (C#: `IsWrapped`).
    pub fn is_wrapped(&self) -> bool {
        self.wrapped
    }

    /// The label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// A render target over mip level 0 of this colour texture (C#: `AsRenderTarget`).
    pub fn as_render_target(&self) -> RenderTargetView {
        RenderTargetView::new(self.view.clone(), self.format, self.width, self.height, None)
    }

    /// Uploads mip level 0 from tightly packed rows in the texture's own byte order (C#:
    /// `Write(pixels)`). For an RGBA8 or BGRA8 texture with more than one mip level, the other
    /// levels are made on the CPU with a 2 x 2 box filter and uploaded too.
    pub fn write(&self, pixels: &[u8]) -> Result<(), GraphicsException> {
        self.write_mip_level(0, pixels)?;
        if self.mip_level_count <= 1 || !TextureFormats::is_rgba8_family(self.format) {
            return Ok(());
        }
        let mut level = pixels.to_vec();
        let (mut width, mut height) = (self.width, self.height);
        for mip in 1..self.mip_level_count {
            let (next, next_width, next_height) = downsample(&level, width, height);
            self.write_mip_level(mip, &next)?;
            level = next;
            width = next_width;
            height = next_height;
        }
        Ok(())
    }

    /// Uploads RGBA pixels (red first, 4 bytes each, tightly packed) to an RGBA8 or BGRA8 texture,
    /// swapping red and blue for BGRA; mip levels as for [`GpuTexture::write`] (C#: `WriteRgba`).
    pub fn write_rgba(&self, rgba: &[u8]) -> Result<(), GraphicsException> {
        if !TextureFormats::is_rgba8_family(self.format) {
            return Err(GraphicsException::new(format!("write_rgba needs an RGBA8 or BGRA8 texture; this one is {:?}. Use write with pixels in the texture's own format.", self.format)));
        }
        if !TextureFormats::is_bgra8(self.format) {
            return self.write(rgba);
        }
        let mut swapped = rgba.to_vec();
        swap_red_blue(&mut swapped);
        self.write(&swapped)
    }

    /// Uploads one mip level from tightly packed rows in the texture's byte order, with no
    /// mipmap generation (C#: `WriteMipLevel`).
    pub fn write_mip_level(&self, mip_level: u32, pixels: &[u8]) -> Result<(), GraphicsException> {
        if self.array_layers != 1 {
            return Err(GraphicsException::new("write_mip_level writes a 2D texture; this one is an array of layers. Write one layer through a layer view (create_layer_view), or a region of a layer with write_region."));
        }
        if mip_level >= self.mip_level_count {
            return Err(GraphicsException::new(format!("The texture has {} mip level(s); level {mip_level} does not exist.", self.mip_level_count)));
        }
        if !self.usage.contains(wgpu::TextureUsages::COPY_DST) {
            return Err(GraphicsException::new("The texture was not created with COPY_DST, so it cannot be written; create it with that usage."));
        }
        let bytes_per_pixel = TextureFormats::bytes_per_pixel(self.format);
        if bytes_per_pixel == 0 || TextureFormats::is_depth(self.format) {
            return Err(GraphicsException::new(format!("The library cannot upload {:?} textures; use a colour format such as Rgba8Unorm.", self.format)));
        }
        let width = (self.width >> mip_level).max(1);
        let height = (self.height >> mip_level).max(1);
        let expected = (width * height * bytes_per_pixel) as usize;
        if pixels.len() != expected {
            return Err(GraphicsException::new(format!("Mip level {mip_level} needs {expected} bytes ({width} x {height} x {bytes_per_pixel}); got {}.", pixels.len())));
        }
        self.context.queue().write_texture(
            wgpu::TexelCopyTextureInfo { texture: &self.handle, mip_level, origin: wgpu::Origin3d { x: 0, y: 0, z: self.view_layer }, aspect: wgpu::TextureAspect::All },
            pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * bytes_per_pixel), rows_per_image: Some(height) },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        Ok(())
    }

    /// The number of mip levels a full chain for a size has: 1 + floor(log2(max(width, height)))
    /// (C#: `FullMipChainLength`).
    pub fn full_mip_chain_length(width: u32, height: u32) -> u32 {
        let mut size = width.max(height).max(1);
        let mut levels = 1;
        while size > 1 {
            size >>= 1;
            levels += 1;
        }
        levels
    }
}

/// The next mip level of RGBA8 pixels by a 2 x 2 box filter (edges clamp), with its size.
pub(crate) fn downsample(source: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let new_width = (width >> 1).max(1);
    let new_height = (height >> 1).max(1);
    let (w, h, nw) = (width as usize, height as usize, new_width as usize);
    let mut result = vec![0u8; nw * new_height as usize * 4];
    for y in 0..new_height as usize {
        let y0 = (y * 2).min(h - 1);
        let y1 = (y * 2 + 1).min(h - 1);
        for x in 0..nw {
            let x0 = (x * 2).min(w - 1);
            let x1 = (x * 2 + 1).min(w - 1);
            for channel in 0..4 {
                let sum = source[(y0 * w + x0) * 4 + channel] as u32 + source[(y0 * w + x1) * 4 + channel] as u32 + source[(y1 * w + x0) * 4 + channel] as u32 + source[(y1 * w + x1) * 4 + channel] as u32;
                result[(y * nw + x) * 4 + channel] = ((sum + 2) / 4) as u8;
            }
        }
    }
    (result, new_width, new_height)
}

/// Swaps bytes 0 and 2 of every 4-byte pixel in place.
fn swap_red_blue(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
}

#[cfg(test)]
mod tests {
    use super::{downsample, swap_red_blue, GpuTexture};

    #[test]
    fn full_mip_chain_length_counts_down_to_one_pixel() {
        assert_eq!(GpuTexture::full_mip_chain_length(1, 1), 1);
        assert_eq!(GpuTexture::full_mip_chain_length(256, 256), 9);
        assert_eq!(GpuTexture::full_mip_chain_length(300, 20), 9);
        assert_eq!(GpuTexture::full_mip_chain_length(0, 0), 1);
    }

    #[test]
    fn downsample_averages_two_by_two_with_rounding() {
        let source = [0, 0, 0, 0, 255, 255, 255, 255, 1, 1, 1, 1, 2, 2, 2, 2];
        let (result, width, height) = downsample(&source, 2, 2);
        assert_eq!((width, height), (1, 1));
        assert_eq!(result, vec![65, 65, 65, 65]);
    }

    #[test]
    fn downsample_clamps_an_odd_edge() {
        let source = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let (result, width, height) = downsample(&source, 3, 1);
        assert_eq!((width, height), (1, 1));
        assert_eq!(result, vec![30, 40, 50, 60]);
    }

    #[test]
    fn swap_red_blue_swaps_every_pixel() {
        let mut pixels = [1, 2, 3, 4, 5, 6, 7, 8];
        swap_red_blue(&mut pixels);
        assert_eq!(pixels, [3, 2, 1, 4, 7, 6, 5, 8]);
    }
}
