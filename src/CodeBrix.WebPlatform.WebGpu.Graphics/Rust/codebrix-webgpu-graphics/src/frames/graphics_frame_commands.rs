//! The copies, clears, query resolves and debug groups of [`GraphicsFrame`] (C#:
//! `GraphicsFrame.Commands.cs`). Left out, because the browser has no such commands:
//! `ClearTexture` (clear it with a render pass instead), `WriteTimestamp` between commands (use pass
//! timestamps), `TransitionTexture` and `WrapRenderPass` / `WrapComputePass` (`wgpu`'s own passes
//! are at hand through `encoder()`).

use crate::context::{label_text, GraphicsException};
use crate::frames::GraphicsFrame;
use crate::queries::GpuQuerySet;
use crate::resources::texture_region::align_row;
use crate::resources::{GpuBuffer, GpuTexture, TextureCopyLayout, TextureRegion};

impl<'a> GraphicsFrame<'a> {
    /// Records a copy from a buffer into a texture region (C#: `CopyBufferToTexture(source,
    /// sourceOffset, bytesPerRow, destination, region)`). The buffer holds the region's rows
    /// `bytes_per_row` apart - a multiple of 256 when the region has more than one row
    /// ([`TextureCopyLayout::aligned_bytes_per_row`]).
    pub fn copy_buffer_to_texture(&mut self, source: &GpuBuffer, source_offset: u64, bytes_per_row: u32, destination: &GpuTexture, region: &TextureRegion) -> Result<(), GraphicsException> {
        destination.check_region(region)?;
        check_row_pitch(bytes_per_row, region, destination.format())?;
        self.encoder().copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo { buffer: source.handle(), layout: wgpu::TexelCopyBufferLayout { offset: source_offset, bytes_per_row: Some(bytes_per_row), rows_per_image: Some(region.height) } },
            region.to_copy_info(destination.handle()),
            region.to_extent(),
        );
        Ok(())
    }

    /// Records a copy from a buffer into a whole mip level, rows 256-aligned (C#:
    /// `CopyBufferToTexture(source, destination, mipLevel, sourceOffset)`).
    pub fn copy_buffer_to_texture_level(&mut self, source: &GpuBuffer, destination: &GpuTexture, mip_level: u32, source_offset: u64) -> Result<(), GraphicsException> {
        let region = TextureRegion::whole(destination, mip_level);
        let row = TextureCopyLayout::aligned_bytes_per_row(region.width, destination.format())?;
        self.copy_buffer_to_texture(source, source_offset, row, destination, &region)
    }

    /// Records a copy from a texture region into a buffer, rows `bytes_per_row` apart (a multiple
    /// of 256 for more than one row) (C#: `CopyTextureToBuffer(source, region, destination,
    /// destinationOffset, bytesPerRow)`). Read the buffer afterwards with `read::<u8>(..).await`
    /// and [`TextureCopyLayout::unpack_rows`] - or use `GpuTexture::read_region(..).await`.
    pub fn copy_texture_to_buffer(&mut self, source: &GpuTexture, region: &TextureRegion, destination: &GpuBuffer, destination_offset: u64, bytes_per_row: u32) -> Result<(), GraphicsException> {
        source.check_region(region)?;
        check_row_pitch(bytes_per_row, region, source.format())?;
        self.encoder().copy_texture_to_buffer(
            region.to_copy_info(source.handle()),
            wgpu::TexelCopyBufferInfo { buffer: destination.handle(), layout: wgpu::TexelCopyBufferLayout { offset: destination_offset, bytes_per_row: Some(bytes_per_row), rows_per_image: Some(region.height) } },
            region.to_extent(),
        );
        Ok(())
    }

    /// Records a copy of a whole mip level into a buffer, rows 256-aligned (C#:
    /// `CopyTextureToBuffer(source, destination, mipLevel, destinationOffset)`; the buffer needs
    /// [`TextureCopyLayout::buffer_size`] bytes after the offset).
    pub fn copy_texture_to_buffer_level(&mut self, source: &GpuTexture, destination: &GpuBuffer, mip_level: u32, destination_offset: u64) -> Result<(), GraphicsException> {
        let region = TextureRegion::whole(source, mip_level);
        let row = TextureCopyLayout::aligned_bytes_per_row(region.width, source.format())?;
        self.copy_texture_to_buffer(source, &region, destination, destination_offset, row)
    }

    /// Records a copy of a texture region into another texture at (`destination_x`,
    /// `destination_y`) of a mip level and layer (C#: `CopyTextureToTexture(source, sourceRegion,
    /// destination, destinationMipLevel, destinationX, destinationY, destinationLayer)`).
    pub fn copy_texture_region_to_texture(&mut self, source: &GpuTexture, source_region: &TextureRegion, destination: &GpuTexture, destination_mip_level: u32, destination_x: u32, destination_y: u32, destination_layer: u32) -> Result<(), GraphicsException> {
        source.check_region(source_region)?;
        let target = TextureRegion { x: destination_x, y: destination_y, mip_level: destination_mip_level, array_layer: destination_layer, ..*source_region };
        destination.check_region(&target)?;
        self.encoder().copy_texture_to_texture(source_region.to_copy_info(source.handle()), target.to_copy_info(destination.handle()), source_region.to_extent());
        Ok(())
    }

    /// Records a copy of a whole mip level into the same level of another texture of the same
    /// size (C#: `CopyTextureToTexture(source, destination, mipLevel)`).
    pub fn copy_texture_to_texture(&mut self, source: &GpuTexture, destination: &GpuTexture, mip_level: u32) -> Result<(), GraphicsException> {
        let region = TextureRegion::whole(source, mip_level);
        self.copy_texture_region_to_texture(source, &region, destination, mip_level, 0, 0, destination.view_layer())
    }

    /// Uploads tightly packed texels into a texture region as part of this frame - ordered with
    /// the frame's other commands, unlike `GpuTexture::write_region`, which lands before the next
    /// submit (C#: `UploadTexture`). One layer at a time.
    pub fn upload_texture(&mut self, destination: &GpuTexture, region: &TextureRegion, pixels: &[u8]) -> Result<(), GraphicsException> {
        destination.check_region(region)?;
        if region.layers != 1 {
            return Err(GraphicsException::new("Upload one layer at a time: give the region layers = 1."));
        }
        let tight = TextureCopyLayout::tight_bytes_per_row(region.width, destination.format())?;
        let expected = tight as usize * region.height as usize;
        if pixels.len() != expected {
            return Err(GraphicsException::new(format!("The region {region} needs {expected} bytes; got {}.", pixels.len())));
        }
        let aligned = align_row(tight);
        let mut staging_bytes = vec![0u8; aligned as usize * region.height as usize];
        TextureCopyLayout::pack_rows(pixels, tight as usize, region.height as usize, &mut staging_bytes, aligned as usize)?;
        let staging = self.context().create_buffer_from(&staging_bytes, wgpu::BufferUsages::COPY_SRC, Some("upload staging"))?;
        self.copy_buffer_to_texture(&staging, 0, aligned, destination, region)
    }

    /// Records zero-filling a buffer range (C#: `ClearBuffer(buffer, offset, size)`; size None, or
    /// C#'s 0, for the rest of the buffer). Offset and size are multiples of 4.
    pub fn clear_buffer(&mut self, buffer: &GpuBuffer, offset: u64, size: Option<u64>) -> Result<(), GraphicsException> {
        let size = check_clear(buffer.size(), offset, size).map_err(|message| GraphicsException::new(format!("{message} (buffer{})", label_text(buffer.label()))))?;
        if size > 0 {
            self.encoder().clear_buffer(buffer.handle(), offset, Some(size));
        }
        Ok(())
    }

    /// Records the resolve of `query_count` queries from `first_query` into a buffer as 64-bit
    /// values (C#: `ResolveQuerySet`). Make the buffer with
    /// `GraphicsContext::create_query_resolve_buffer`; the offset is a multiple of 256.
    pub fn resolve_query_set(&mut self, query_set: &GpuQuerySet, first_query: u32, query_count: u32, destination: &GpuBuffer, destination_offset: u64) -> Result<(), GraphicsException> {
        if first_query as u64 + query_count as u64 > query_set.count() as u64 {
            return Err(GraphicsException::new(format!("The set has {} queries; {first_query} + {query_count} is past its end.", query_set.count())));
        }
        if !destination.usage().contains(wgpu::BufferUsages::QUERY_RESOLVE) {
            return Err(GraphicsException::new("The destination needs QUERY_RESOLVE; make it with GraphicsContext::create_query_resolve_buffer."));
        }
        if destination_offset % 256 != 0 {
            return Err(GraphicsException::new(format!("The resolve offset {destination_offset} must be a multiple of 256.")));
        }
        let bytes = query_count as u64 * 8;
        if destination_offset + bytes > destination.size() {
            return Err(GraphicsException::new(format!("{bytes} bytes at offset {destination_offset} overrun the {}-byte buffer.", destination.size())));
        }
        self.encoder().resolve_query_set(query_set.handle(), first_query..first_query + query_count, destination.handle(), destination_offset);
        Ok(())
    }

    /// Opens a named debug group around the following commands (C#: `PushDebugGroup`); the
    /// browser's developer tools show it in validation messages.
    pub fn push_debug_group(&mut self, label: &str) {
        self.encoder().push_debug_group(label);
    }

    /// Closes the innermost debug group (C#: `PopDebugGroup`).
    pub fn pop_debug_group(&mut self) {
        self.encoder().pop_debug_group();
    }

    /// Inserts a named marker between commands (C#: `InsertDebugMarker`).
    pub fn insert_debug_marker(&mut self, label: &str) {
        self.encoder().insert_debug_marker(label);
    }
}

/// The rule for `bytes_per_row` of a buffer-texture copy (C#: `CheckRowPitch`).
fn check_row_pitch(bytes_per_row: u32, region: &TextureRegion, format: wgpu::TextureFormat) -> Result<(), GraphicsException> {
    let tight = TextureCopyLayout::tight_bytes_per_row(region.width, format)?;
    if bytes_per_row < tight {
        return Err(GraphicsException::new(format!("{bytes_per_row} bytes per row is less than one row of the region ({tight}).")));
    }
    if (region.height > 1 || region.layers > 1) && bytes_per_row % TextureCopyLayout::ROW_ALIGNMENT != 0 {
        return Err(GraphicsException::new(format!("{bytes_per_row} bytes per row must be a multiple of 256 for a copy of more than one row; use TextureCopyLayout::aligned_bytes_per_row.")));
    }
    Ok(())
}

/// The size a clear of `size` bytes (None: the rest) at `offset` covers, or what is wrong.
pub(crate) fn check_clear(buffer_size: u64, offset: u64, size: Option<u64>) -> Result<u64, String> {
    if offset % 4 != 0 {
        return Err(format!("The clear offset {offset} must be a multiple of 4"));
    }
    if offset > buffer_size {
        return Err(format!("The clear offset {offset} is past the end of the {buffer_size}-byte buffer"));
    }
    let size = match size {
        Some(0) | None => buffer_size - offset,
        Some(size) => size,
    };
    if size % 4 != 0 {
        return Err(format!("The clear size {size} must be a multiple of 4"));
    }
    if offset + size > buffer_size {
        return Err(format!("Clearing {size} bytes at offset {offset} overruns the {buffer_size}-byte buffer"));
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::{check_clear, check_row_pitch};
    use crate::resources::TextureRegion;

    #[test]
    fn clears_cover_the_rest_or_a_multiple_of_4() {
        assert_eq!(check_clear(64, 0, None), Ok(64));
        assert_eq!(check_clear(64, 16, Some(0)), Ok(48));
        assert_eq!(check_clear(64, 16, Some(8)), Ok(8));
        assert!(check_clear(64, 2, None).unwrap_err().contains("multiple of 4"));
        assert!(check_clear(64, 16, Some(6)).unwrap_err().contains("multiple of 4"));
        assert!(check_clear(64, 60, Some(8)).unwrap_err().contains("overruns"));
        assert!(check_clear(64, 68, None).unwrap_err().contains("past the end"));
    }

    #[test]
    fn rows_of_a_multi_row_copy_are_256_aligned() {
        let f = wgpu::TextureFormat::Rgba8Unorm;
        assert!(check_row_pitch(256, &TextureRegion::new(0, 0, 10, 4), f).is_ok());
        assert!(check_row_pitch(40, &TextureRegion::new(0, 0, 10, 1), f).is_ok());
        assert!(check_row_pitch(40, &TextureRegion::new(0, 0, 10, 2), f).unwrap_err().message().contains("multiple of 256"));
        assert!(check_row_pitch(36, &TextureRegion::new(0, 0, 10, 1), f).unwrap_err().message().contains("less than one row"));
    }
}
