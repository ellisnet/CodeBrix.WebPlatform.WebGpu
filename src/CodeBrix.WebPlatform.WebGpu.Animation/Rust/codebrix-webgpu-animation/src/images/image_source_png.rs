//! `ImageSource::from_png`: a picture read from the bytes of a PNG file (through codebrix-webgpu's
//! `decode_png`), and `ImageSource::from_picture_pixels`: a picture read from the Animation
//! package's picture-pixels form (what the .NET `PicturePixelsWriter` writes on a backend).

use crate::images::ImageSource;

/// The four bytes the picture-pixels form starts with: "CBPX" in ASCII.
pub(crate) const PICTURE_PIXELS_MAGIC: [u8; 4] = *b"CBPX";
/// The major version of the picture-pixels form this crate reads; another major version is refused.
pub(crate) const PICTURE_PIXELS_MAJOR_VERSION: u16 = 1;
/// The length of the picture-pixels header: magic, major and minor version, width, height.
pub(crate) const PICTURE_PIXELS_HEADER_LENGTH: usize = 16;

impl ImageSource {
    /// Reads a picture from the bytes of a PNG FILE, through
    /// [`codebrix_webgpu::decode_png`] and under its rules: every colour type and bit depth the
    /// PNG format has is brought to straight-alpha 8-bit RGBA, exactly as
    /// [`ImageSource::from_rgba`] takes it: grey becomes equal red, green and blue; a palette is
    /// looked up (its transparency chunk gives alpha); a `tRNS` colour key becomes alpha 0; 16-bit
    /// samples keep their high byte; 1, 2 and 4-bit samples are scaled to the full range; Adam7
    /// interlacing is undone. The values are taken as sRGB-encoded, as every picture here is: the
    /// file's gamma, chromaticity and ICC chunks are not applied. Of an animated PNG only the
    /// default image is read. A picture is at most 8192 pixels wide and high. The error says what
    /// is wrong with the file.
    pub fn from_png(bytes: &[u8]) -> Result<ImageSource, String> {
        let image = codebrix_webgpu::decode_png(bytes)?;
        ImageSource::from_rgba_vec(image.rgba, image.width, image.height)
    }

    /// Reads a picture from the picture-pixels form: DECODED pixels as the .NET
    /// `PicturePixelsWriter` of the CodeBrix.WebPlatform.WebGpu.Animation package writes them on a
    /// backend - the magic "CBPX", the major and the minor version (two unsigned 16-bit
    /// little-endian integers), the width and the height (two unsigned 32-bit little-endian
    /// integers), then width x height x 4 bytes of straight-alpha RGBA (sRGB-encoded, rows top to
    /// bottom, no padding). A higher MINOR version is read (what follows the pixels is ignored);
    /// another MAJOR version is refused. The error says what is wrong.
    pub fn from_picture_pixels(bytes: &[u8]) -> Result<ImageSource, String> {
        if bytes.len() < PICTURE_PIXELS_HEADER_LENGTH || bytes[0..4] != PICTURE_PIXELS_MAGIC {
            return Err("The bytes are not in the picture-pixels form (they do not start with \"CBPX\"): write them with PicturePixelsWriter on the backend, or read a PNG file with ImageSource::from_png.".to_string());
        }
        let major = u16::from_le_bytes([bytes[4], bytes[5]]);
        if major != PICTURE_PIXELS_MAJOR_VERSION {
            return Err(format!("The picture-pixels form is of major version {major}; this crate reads version {PICTURE_PIXELS_MAJOR_VERSION}: use packages of the same release on the backend and in the frontend."));
        }
        let width = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let height = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        ImageSource::from_rgba(&bytes[PICTURE_PIXELS_HEADER_LENGTH..], width, height)
    }
}
