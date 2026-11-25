//! `ImageSource::from_jpeg`: a picture read from the bytes of a JPEG file (through
//! codebrix-webgpu's `decode_jpeg`). Not in C# (C#'s `FromEncoded` decodes through its own image
//! stack).

use crate::images::ImageSource;

impl ImageSource {
    /// Reads a picture from the bytes of a JPEG FILE, through [`codebrix_webgpu::decode_jpeg`]
    /// and under its rules (Not in C#): baseline and progressive JPEG with 8-bit samples and any
    /// chroma subsampling; colour becomes straight-alpha 8-bit RGBA exactly as
    /// [`ImageSource::from_rgba`] takes it, greyscale becomes equal red, green and blue, and alpha
    /// is 255 throughout (JPEG has none). The values are taken as sRGB-encoded, as every picture
    /// here is: an embedded ICC profile is not applied. THE EXIF ORIENTATION IS NOT APPLIED: the
    /// pixels are as the file stores them, so a photograph straight from a phone camera that
    /// relies on that tag to stand upright is drawn on its side - rotate it before it is saved,
    /// or turn its [`ImageShape`](crate::shapes::ImageShape) yourself. CMYK and YCCK files,
    /// 12-bit, lossless, arithmetic-coded and hierarchical JPEG, and damaged or cut-short files
    /// are refused. A picture is at most 8192 pixels wide and high. The error says what is wrong
    /// with the file.
    pub fn from_jpeg(bytes: &[u8]) -> Result<ImageSource, String> {
        let image = codebrix_webgpu::decode_jpeg(bytes)?;
        ImageSource::from_rgba_vec(image.rgba, image.width, image.height)
    }
}
