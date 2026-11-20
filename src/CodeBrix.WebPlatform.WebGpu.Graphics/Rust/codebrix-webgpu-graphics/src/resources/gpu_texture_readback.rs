//! Region writes, the ASYNC readback of textures, and the texture copy of [`GpuTexture`] (C#:
//! `GpuTexture.ReadPixels`, `ReadRgba`, `ReadRegion`, `WriteRegion`, `CopyTo` - the reads wait
//! for the GPU in C#; a browser answers with a promise, so here they are awaited).

use crate::context::{label_text, GraphicsException};
use crate::internal::oneshot;
use crate::resources::texture_region::align_row;
use crate::resources::{GpuTexture, TextureCopyLayout, TextureFormats, TextureRegion};

impl GpuTexture {
    /// Uploads tightly packed texels (the texture's byte order) into a region through the queue
    /// (C#: `WriteRegion`); one layer at a time. Needs `COPY_DST`.
    pub fn write_region(&self, region: &TextureRegion, pixels: &[u8]) -> Result<(), GraphicsException> {
        self.check_region(region)?;
        if region.layers != 1 {
            return Err(GraphicsException::new("Write one layer at a time: give the region layers = 1."));
        }
        if !self.usage().contains(wgpu::TextureUsages::COPY_DST) {
            return Err(GraphicsException::new(format!("The texture{} was not created with COPY_DST, so it cannot be written; create it with that usage.", label_text(self.label()))));
        }
        let row = TextureCopyLayout::tight_bytes_per_row(region.width, self.format())?;
        let expected = row as usize * region.height as usize;
        if pixels.len() != expected {
            return Err(GraphicsException::new(format!("The region {region} needs {expected} bytes; got {}.", pixels.len())));
        }
        self.context().queue().write_texture(region.to_copy_info(self.handle()), pixels, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(region.height) }, region.to_extent());
        Ok(())
    }

    /// Reads one mip level back as tightly packed rows in the texture's byte order, after the work
    /// submitted so far (C#: `ReadPixels(mipLevel)`; ASYNC here). Needs `COPY_SRC`; depth only as
    /// `Depth32Float` (4 bytes per texel, the depth as f32).
    pub async fn read_pixels(&self, mip_level: u32) -> Result<Vec<u8>, GraphicsException> {
        if mip_level >= self.mip_level_count() {
            return Err(GraphicsException::new(format!("The texture has {} mip level(s); level {mip_level} does not exist.", self.mip_level_count())));
        }
        self.read_region(&TextureRegion::whole(self, mip_level)).await
    }

    /// Reads mip level 0 of an RGBA8 or BGRA8 texture as RGBA pixels, red first (C#: `ReadRgba`;
    /// async here).
    pub async fn read_rgba(&self) -> Result<Vec<u8>, GraphicsException> {
        if !TextureFormats::is_rgba8_family(self.format()) {
            return Err(GraphicsException::new(format!("read_rgba needs an RGBA8 or BGRA8 texture; this one is {:?}. Use read_pixels for the texture's own byte order.", self.format())));
        }
        let mut pixels = self.read_pixels(0).await?;
        if TextureFormats::is_bgra8(self.format()) {
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
        Ok(pixels)
    }

    /// Reads a region of one layer back as tightly packed texels (C#: `ReadRegion`; async here).
    pub async fn read_region(&self, region: &TextureRegion) -> Result<Vec<u8>, GraphicsException> {
        self.check_region(region)?;
        if region.layers != 1 {
            return Err(GraphicsException::new("Read one layer at a time: give the region layers = 1."));
        }
        if !self.usage().contains(wgpu::TextureUsages::COPY_SRC) {
            return Err(GraphicsException::new(format!("The texture{} was not created with COPY_SRC, so it cannot be read back; create it with that usage (create_render_texture and create_texture_from_rgba add it).", label_text(self.label()))));
        }
        let depth = TextureFormats::is_depth(self.format());
        if depth && self.format() != wgpu::TextureFormat::Depth32Float {
            return Err(GraphicsException::new(format!("WebGPU cannot copy {:?} out of a texture; use Depth32Float for a readable depth buffer.", self.format())));
        }
        let tight = TextureCopyLayout::tight_bytes_per_row(region.width, self.format())?;
        let aligned = align_row(tight);
        let size = aligned as u64 * region.height as u64;
        let device = self.context().device();
        let staging = device.create_buffer(&wgpu::BufferDescriptor { label: Some("codebrix-webgpu-graphics texture readback"), size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("codebrix-webgpu-graphics texture readback") });
        let mut source = region.to_copy_info(self.handle());
        if depth {
            source.aspect = wgpu::TextureAspect::DepthOnly;
        }
        encoder.copy_texture_to_buffer(source, wgpu::TexelCopyBufferInfo { buffer: &staging, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(aligned), rows_per_image: Some(region.height) } }, region.to_extent());
        self.context().queue().submit([encoder.finish()]);
        let (sender, receiver) = oneshot::channel();
        staging.map_async(wgpu::MapMode::Read, .., move |result| sender.send(result));
        if receiver.await.is_err() {
            return Err(GraphicsException::new(format!("The read-back copy of the texture{} could not be mapped (the device may be lost); check GraphicsContext::lost_reason.", label_text(self.label()))));
        }
        let mut pixels = vec![0u8; tight as usize * region.height as usize];
        let unpacked = match staging.get_mapped_range(..) {
            Ok(view) => TextureCopyLayout::unpack_rows(&view, aligned as usize, region.height as usize, &mut pixels, tight as usize),
            Err(error) => Err(GraphicsException::new(format!("The read-back copy of the texture{} could not be read ({error}); read it again.", label_text(self.label())))),
        };
        staging.unmap();
        unpacked?;
        Ok(pixels)
    }

    /// Copies this texture's mip levels into another texture of the same format and size on the
    /// GPU, submitted at once (C#: `CopyTo`). Needs `COPY_SRC` here and `COPY_DST` there. Returns
    /// the queue's submission index.
    pub fn copy_to(&self, destination: &GpuTexture) -> Result<wgpu::SubmissionIndex, GraphicsException> {
        if self.width() != destination.width() || self.height() != destination.height() || self.format() != destination.format() {
            return Err(GraphicsException::new(format!("copy_to needs a texture of the same size and format: {} x {} {:?} into {} x {} {:?}.", self.width(), self.height(), self.format(), destination.width(), destination.height(), destination.format())));
        }
        let mut frame = self.context().begin_frame();
        for level in 0..self.mip_level_count().min(destination.mip_level_count()) {
            frame.copy_texture_to_texture(self, destination, level)?;
        }
        let commands = frame.finish()?;
        Ok(self.context().queue().submit([commands]))
    }

    /// Checks a region against the whole `wgpu` texture (every layer, not only a layer view's).
    pub(crate) fn check_region(&self, region: &TextureRegion) -> Result<(), GraphicsException> {
        region.check(self.width(), self.height(), self.mip_level_count(), self.handle().depth_or_array_layers())
    }
}
