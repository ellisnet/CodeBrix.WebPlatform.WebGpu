//! `ImageSource::from_png`: a picture read from the bytes of a PNG file (the `png` crate), and
//! `ImageSource::from_picture_pixels`: a picture read from the Animation package's picture-pixels
//! form (what the .NET `PicturePixelsWriter` writes on a backend).

use std::io::Cursor;

use png::{BitDepth, ColorType, Transformations};

use crate::images::ImageSource;

/// The four bytes the picture-pixels form starts with: "CBPX" in ASCII.
pub(crate) const PICTURE_PIXELS_MAGIC: [u8; 4] = *b"CBPX";
/// The major version of the picture-pixels form this crate reads; another major version is refused.
pub(crate) const PICTURE_PIXELS_MAJOR_VERSION: u16 = 1;
/// The length of the picture-pixels header: magic, major and minor version, width, height.
pub(crate) const PICTURE_PIXELS_HEADER_LENGTH: usize = 16;

/// The largest width or height a PNG may have here, in pixels (a GPU texture's limit in every
/// browser that has WebGPU).
const MAX_DIMENSION: u32 = 8192;

impl ImageSource {
    /// Reads a picture from the bytes of a PNG FILE (PNG is the only file format this crate
    /// reads). Every colour type and bit depth the PNG format has is brought to straight-alpha
    /// 8-bit RGBA, exactly as [`ImageSource::from_rgba`] takes it: grey becomes equal red, green
    /// and blue; a palette is looked up (its transparency chunk gives alpha); a `tRNS` colour key
    /// becomes alpha 0; 16-bit samples keep their high byte; 1, 2 and 4-bit samples are scaled to
    /// the full range; Adam7 interlacing is undone. The values are taken as sRGB-encoded, as
    /// every picture here is: the file's gamma, chromaticity and ICC chunks are not applied. Of an
    /// animated PNG only the default image is read. A picture is at most 8192 pixels wide and
    /// high. The error says what is wrong with the file.
    pub fn from_png(bytes: &[u8]) -> Result<ImageSource, String> {
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        // EXPAND: palettes to RGB, low bit depths to 8 bits, tRNS to an alpha channel; STRIP_16:
        // 16-bit samples to their high byte.
        decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|e| format!("The bytes are not a PNG file this crate can read ({e}): pass the whole file, as it was stored."))?;
        let (width, height) = {
            let info = reader.info();
            (info.width, info.height)
        };
        if width < 1 || height < 1 || width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(format!("The PNG is {width} x {height} pixels; a picture must be between 1 x 1 and {MAX_DIMENSION} x {MAX_DIMENSION}: scale it down before handing it over."));
        }
        let size = reader.output_buffer_size().ok_or_else(|| "The PNG is too large to decode here: scale it down before handing it over.".to_string())?;
        let mut buffer = vec![0u8; size];
        let frame = reader.next_frame(&mut buffer).map_err(|e| format!("The PNG's image data could not be decoded ({e}): the file is damaged or cut short."))?;
        if frame.bit_depth != BitDepth::Eight {
            return Err(format!("The PNG decoded to {:?}-bit samples where 8 were asked for: this is a fault of the decoder, not of the file.", frame.bit_depth));
        }
        let pixels = width as usize * height as usize;
        let line = frame.line_size;
        let mut rgba = Vec::with_capacity(pixels * 4);
        for y in 0..height as usize {
            let row = &buffer[y * line..(y + 1) * line];
            match frame.color_type {
                ColorType::Rgba => rgba.extend_from_slice(&row[..width as usize * 4]),
                ColorType::Rgb => {
                    for px in row[..width as usize * 3].chunks_exact(3) {
                        rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
                    }
                }
                ColorType::GrayscaleAlpha => {
                    for px in row[..width as usize * 2].chunks_exact(2) {
                        rgba.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
                    }
                }
                ColorType::Grayscale => {
                    for v in &row[..width as usize] {
                        rgba.extend_from_slice(&[*v, *v, *v, 255]);
                    }
                }
                ColorType::Indexed => return Err("The PNG's palette was not expanded: this is a fault of the decoder, not of the file.".to_string()),
            }
        }
        ImageSource::from_rgba_vec(rgba, width, height)
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
