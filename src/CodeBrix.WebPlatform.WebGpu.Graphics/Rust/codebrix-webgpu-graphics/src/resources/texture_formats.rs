//! [`TextureFormats`]: facts about texture formats the library needs (C#: `TextureFormats`).

use wgpu::TextureFormat as F;

/// Facts about texture formats (C#: the static class `TextureFormats`).
pub struct TextureFormats;

impl TextureFormats {
    /// The bytes one texel takes in an uploadable format; 0 for a format the library cannot upload
    /// (compressed, or depth-stencil) (C#: `BytesPerPixel`).
    pub fn bytes_per_pixel(format: wgpu::TextureFormat) -> u32 {
        match format {
            F::R8Unorm | F::R8Snorm | F::R8Uint | F::R8Sint | F::Stencil8 => 1,
            F::R16Unorm | F::R16Snorm | F::R16Uint | F::R16Sint | F::R16Float | F::Rg8Unorm | F::Rg8Snorm | F::Rg8Uint | F::Rg8Sint | F::Depth16Unorm => 2,
            F::R32Float | F::R32Uint | F::R32Sint | F::Rg16Unorm | F::Rg16Snorm | F::Rg16Uint | F::Rg16Sint | F::Rg16Float | F::Rgba8Unorm | F::Rgba8UnormSrgb | F::Rgba8Snorm | F::Rgba8Uint | F::Rgba8Sint | F::Bgra8Unorm | F::Bgra8UnormSrgb | F::Rgb10a2Uint | F::Rgb10a2Unorm | F::Rg11b10Ufloat | F::Rgb9e5Ufloat | F::Depth32Float => 4,
            F::Rg32Float | F::Rg32Uint | F::Rg32Sint | F::Rgba16Unorm | F::Rgba16Snorm | F::Rgba16Uint | F::Rgba16Sint | F::Rgba16Float => 8,
            F::Rgba32Float | F::Rgba32Uint | F::Rgba32Sint => 16,
            _ => 0,
        }
    }

    /// True for a depth (or depth-stencil) format (C#: `IsDepth`).
    pub fn is_depth(format: wgpu::TextureFormat) -> bool {
        matches!(format, F::Depth16Unorm | F::Depth24Plus | F::Depth24PlusStencil8 | F::Depth32Float | F::Depth32FloatStencil8)
    }

    /// True for a format with a stencil aspect.
    pub fn has_stencil(format: wgpu::TextureFormat) -> bool {
        matches!(format, F::Depth24PlusStencil8 | F::Depth32FloatStencil8 | F::Stencil8)
    }

    /// True for BGRA8 (linear or sRGB) (C#: `IsBgra8`).
    pub fn is_bgra8(format: wgpu::TextureFormat) -> bool {
        matches!(format, F::Bgra8Unorm | F::Bgra8UnormSrgb)
    }

    /// True for RGBA8 or BGRA8, linear or sRGB (C#: `IsRgba8Family`).
    pub fn is_rgba8_family(format: wgpu::TextureFormat) -> bool {
        matches!(format, F::Rgba8Unorm | F::Rgba8UnormSrgb) || TextureFormats::is_bgra8(format)
    }
}

#[cfg(test)]
mod tests {
    use super::TextureFormats;

    #[test]
    fn bytes_per_pixel_of_common_formats() {
        assert_eq!(TextureFormats::bytes_per_pixel(wgpu::TextureFormat::Rgba8Unorm), 4);
        assert_eq!(TextureFormats::bytes_per_pixel(wgpu::TextureFormat::Rgba16Float), 8);
        assert_eq!(TextureFormats::bytes_per_pixel(wgpu::TextureFormat::Rg16Float), 4);
        assert_eq!(TextureFormats::bytes_per_pixel(wgpu::TextureFormat::Depth24Plus), 0);
    }

    #[test]
    fn format_families() {
        assert!(TextureFormats::is_depth(wgpu::TextureFormat::Depth32Float));
        assert!(!TextureFormats::is_depth(wgpu::TextureFormat::Rgba8Unorm));
        assert!(TextureFormats::is_rgba8_family(wgpu::TextureFormat::Bgra8UnormSrgb));
        assert!(!TextureFormats::is_bgra8(wgpu::TextureFormat::Rgba8UnormSrgb));
        assert!(TextureFormats::has_stencil(wgpu::TextureFormat::Depth24PlusStencil8));
    }
}
