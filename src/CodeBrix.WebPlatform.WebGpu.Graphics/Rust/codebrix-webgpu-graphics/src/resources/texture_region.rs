//! [`TextureRegion`]: a rectangle of one mip level and of one or more array layers of a texture
//! (C#: `TextureRegion`), and [`TextureCopyLayout`]: the row rules of buffer-texture copies (C#:
//! `TextureCopyLayout`).

use std::fmt;

use crate::context::GraphicsException;
use crate::resources::{GpuTexture, TextureFormats};

/// A rectangle of one mip level and of `layers` array layers from `array_layer` (C#:
/// `TextureRegion`), for region writes, reads and copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureRegion {
    /// The left texel (C#: `X`).
    pub x: u32,
    /// The top texel (C#: `Y`).
    pub y: u32,
    /// The width in texels (C#: `Width`).
    pub width: u32,
    /// The height in texels (C#: `Height`).
    pub height: u32,
    /// The mip level (C#: `MipLevel`).
    pub mip_level: u32,
    /// The first array layer (C#: `ArrayLayer`).
    pub array_layer: u32,
    /// The number of layers (C#: `Layers`).
    pub layers: u32,
    /// The aspect (C#: `Aspect`; `All` for colour, `DepthOnly` is chosen for depth reads).
    pub aspect: wgpu::TextureAspect,
}

impl TextureRegion {
    /// A region of mip level 0, layer 0 (C#: `new TextureRegion(x, y, width, height)`; set the
    /// other fields for another level or layer).
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> TextureRegion {
        TextureRegion { x, y, width, height, mip_level: 0, array_layer: 0, layers: 1, aspect: wgpu::TextureAspect::All }
    }

    /// The whole of one mip level of `texture` (layer 0, or the layer a layer view shows) (C#:
    /// `Whole(texture, mipLevel)`).
    pub fn whole(texture: &GpuTexture, mip_level: u32) -> TextureRegion {
        let (width, height) = level_size(texture.width(), texture.height(), mip_level);
        TextureRegion { x: 0, y: 0, width, height, mip_level, array_layer: texture.view_layer(), layers: 1, aspect: wgpu::TextureAspect::All }
    }

    /// The same region on another layer (C#: the `arrayLayer` argument).
    pub fn on_layer(mut self, array_layer: u32) -> TextureRegion {
        self.array_layer = array_layer;
        self
    }

    /// The same region on another mip level (C#: the `mipLevel` argument).
    pub fn on_mip_level(mut self, mip_level: u32) -> TextureRegion {
        self.mip_level = mip_level;
        self
    }

    /// Checks that the region fits a texture of the given size, mip levels and layers (C#:
    /// `Validate`).
    pub(crate) fn check(&self, width: u32, height: u32, mip_levels: u32, layers: u32) -> Result<(), GraphicsException> {
        if self.mip_level >= mip_levels {
            return Err(GraphicsException::new(format!("The texture has {mip_levels} mip level(s); the region asks for level {}.", self.mip_level)));
        }
        let (level_width, level_height) = level_size(width, height, self.mip_level);
        if self.width == 0 || self.height == 0 || self.layers == 0 {
            return Err(GraphicsException::new(format!("The region {self} is empty; give it a width, a height and at least one layer.")));
        }
        if self.x + self.width > level_width || self.y + self.height > level_height {
            return Err(GraphicsException::new(format!("The region {self} does not fit mip level {} ({level_width} x {level_height}).", self.mip_level)));
        }
        if self.array_layer + self.layers > layers {
            return Err(GraphicsException::new(format!("The texture has {layers} layer(s); the region {self} asks for more.")));
        }
        Ok(())
    }

    /// The `wgpu` copy description of the region on `texture`.
    pub(crate) fn to_copy_info<'a>(&self, texture: &'a wgpu::Texture) -> wgpu::TexelCopyTextureInfo<'a> {
        wgpu::TexelCopyTextureInfo { texture, mip_level: self.mip_level, origin: wgpu::Origin3d { x: self.x, y: self.y, z: self.array_layer }, aspect: self.aspect }
    }

    /// The `wgpu` copy size of the region.
    pub(crate) fn to_extent(&self) -> wgpu::Extent3d {
        wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: self.layers }
    }
}

impl fmt::Display for TextureRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}) {} x {} mip {}", self.x, self.y, self.width, self.height, self.mip_level)?;
        if self.layers > 1 || self.array_layer > 0 {
            write!(f, " layers {}+{}", self.array_layer, self.layers)?;
        }
        Ok(())
    }
}

/// The size of mip level `mip_level` of a `width` x `height` texture.
pub(crate) fn level_size(width: u32, height: u32, mip_level: u32) -> (u32, u32) {
    ((width >> mip_level.min(31)).max(1), (height >> mip_level.min(31)).max(1))
}

/// The row rules of buffer-texture copies (C#: the static class `TextureCopyLayout`): a buffer row
/// of a copy starts at a multiple of [`TextureCopyLayout::ROW_ALIGNMENT`] bytes.
pub struct TextureCopyLayout;

impl TextureCopyLayout {
    /// The alignment of `bytes_per_row` in buffer copies: 256 (C#: `RowAlignment`).
    pub const ROW_ALIGNMENT: u32 = 256;

    /// The bytes of one tightly packed row (C#: `TightBytesPerRow`).
    pub fn tight_bytes_per_row(width: u32, format: wgpu::TextureFormat) -> Result<u32, GraphicsException> {
        let bytes_per_pixel = TextureFormats::bytes_per_pixel(format);
        if bytes_per_pixel == 0 {
            return Err(GraphicsException::new(format!("The library does not know the texel size of {format:?}; use an uncompressed colour format or Depth32Float.")));
        }
        Ok(width * bytes_per_pixel)
    }

    /// The bytes of one row in a copy buffer, rounded up to 256 (C#: `AlignedBytesPerRow`).
    pub fn aligned_bytes_per_row(width: u32, format: wgpu::TextureFormat) -> Result<u32, GraphicsException> {
        Ok(align_row(TextureCopyLayout::tight_bytes_per_row(width, format)?))
    }

    /// The buffer size a copy of `width` x `height` texels with aligned rows needs (C#:
    /// `BufferSize`).
    pub fn buffer_size(width: u32, height: u32, format: wgpu::TextureFormat) -> Result<u64, GraphicsException> {
        Ok(TextureCopyLayout::aligned_bytes_per_row(width, format)? as u64 * height as u64)
    }

    /// Copies tightly packed rows into a buffer image with aligned rows (C#: `PackRows`).
    pub fn pack_rows(tight: &[u8], tight_bytes_per_row: usize, rows: usize, aligned: &mut [u8], aligned_bytes_per_row: usize) -> Result<(), GraphicsException> {
        if rows == 0 || tight_bytes_per_row == 0 {
            return Ok(());
        }
        if tight.len() < tight_bytes_per_row * rows {
            return Err(GraphicsException::new(format!("The source holds fewer than {rows} rows of {tight_bytes_per_row} bytes.")));
        }
        if aligned_bytes_per_row < tight_bytes_per_row || aligned.len() < aligned_bytes_per_row * (rows - 1) + tight_bytes_per_row {
            return Err(GraphicsException::new(format!("The destination is too small for {rows} rows of {aligned_bytes_per_row} bytes.")));
        }
        for row in 0..rows {
            aligned[row * aligned_bytes_per_row..row * aligned_bytes_per_row + tight_bytes_per_row].copy_from_slice(&tight[row * tight_bytes_per_row..(row + 1) * tight_bytes_per_row]);
        }
        Ok(())
    }

    /// Copies a buffer image with aligned rows into tightly packed rows (C#: `UnpackRows`).
    pub fn unpack_rows(aligned: &[u8], aligned_bytes_per_row: usize, rows: usize, tight: &mut [u8], tight_bytes_per_row: usize) -> Result<(), GraphicsException> {
        if rows == 0 || tight_bytes_per_row == 0 {
            return Ok(());
        }
        if tight.len() < tight_bytes_per_row * rows {
            return Err(GraphicsException::new(format!("The destination holds fewer than {rows} rows of {tight_bytes_per_row} bytes.")));
        }
        if aligned_bytes_per_row < tight_bytes_per_row || aligned.len() < aligned_bytes_per_row * (rows - 1) + tight_bytes_per_row {
            return Err(GraphicsException::new(format!("The source holds fewer than {rows} rows of {aligned_bytes_per_row} bytes.")));
        }
        for row in 0..rows {
            tight[row * tight_bytes_per_row..(row + 1) * tight_bytes_per_row].copy_from_slice(&aligned[row * aligned_bytes_per_row..row * aligned_bytes_per_row + tight_bytes_per_row]);
        }
        Ok(())
    }
}

/// `bytes` rounded up to the copy row alignment.
pub(crate) fn align_row(bytes: u32) -> u32 {
    (bytes + TextureCopyLayout::ROW_ALIGNMENT - 1) & !(TextureCopyLayout::ROW_ALIGNMENT - 1)
}

#[cfg(test)]
mod tests {
    use super::{level_size, TextureCopyLayout, TextureRegion};

    #[test]
    fn rows_align_to_256_bytes() {
        let f = wgpu::TextureFormat::Rgba8Unorm;
        assert_eq!(TextureCopyLayout::tight_bytes_per_row(3, f).unwrap(), 12);
        assert_eq!(TextureCopyLayout::aligned_bytes_per_row(3, f).unwrap(), 256);
        assert_eq!(TextureCopyLayout::aligned_bytes_per_row(64, f).unwrap(), 256);
        assert_eq!(TextureCopyLayout::aligned_bytes_per_row(65, f).unwrap(), 512);
        assert_eq!(TextureCopyLayout::buffer_size(65, 3, f).unwrap(), 1536);
        assert_eq!(TextureCopyLayout::aligned_bytes_per_row(10, wgpu::TextureFormat::Rgba16Float).unwrap(), 256);
        assert!(TextureCopyLayout::tight_bytes_per_row(4, wgpu::TextureFormat::Bc1RgbaUnorm).is_err());
    }

    #[test]
    fn pack_and_unpack_are_inverse() {
        let tight: Vec<u8> = (0..24).collect();
        let mut aligned = vec![0xEE; 256 * 2 + 12];
        TextureCopyLayout::pack_rows(&tight, 12, 2, &mut aligned, 256).unwrap();
        assert_eq!(&aligned[0..12], &tight[0..12]);
        assert_eq!(aligned[12], 0xEE);
        assert_eq!(&aligned[256..268], &tight[12..24]);
        let mut back = vec![0; 24];
        TextureCopyLayout::unpack_rows(&aligned, 256, 2, &mut back, 12).unwrap();
        assert_eq!(back, tight);
        assert!(TextureCopyLayout::unpack_rows(&aligned[..100], 256, 2, &mut back, 12).is_err());
        assert!(TextureCopyLayout::pack_rows(&tight[..10], 12, 2, &mut aligned, 256).is_err());
    }

    #[test]
    fn a_region_must_fit_its_level_and_layers() {
        assert!(TextureRegion::new(0, 0, 4, 4).check(4, 4, 1, 1).is_ok());
        assert!(TextureRegion::new(2, 2, 2, 2).check(8, 8, 2, 1).is_ok());
        assert!(TextureRegion::new(2, 2, 3, 2).check(4, 4, 1, 1).unwrap_err().message().contains("does not fit"));
        assert!(TextureRegion::new(0, 0, 2, 2).on_mip_level(1).check(4, 4, 2, 1).is_ok());
        assert!(TextureRegion::new(0, 0, 3, 2).on_mip_level(1).check(4, 4, 2, 1).is_err());
        assert!(TextureRegion::new(0, 0, 1, 1).on_mip_level(2).check(4, 4, 2, 1).unwrap_err().message().contains("mip level"));
        assert!(TextureRegion::new(0, 0, 1, 1).on_layer(3).check(4, 4, 1, 3).unwrap_err().message().contains("layer"));
        assert!(TextureRegion::new(0, 0, 0, 1).check(4, 4, 1, 1).unwrap_err().message().contains("empty"));
        assert_eq!(level_size(5, 3, 2), (1, 1));
        assert_eq!(TextureRegion::new(1, 2, 3, 4).on_layer(2).to_string(), "(1, 2) 3 x 4 mip 0 layers 2+1");
    }
}
