//! [`decode_png`]: the pixels of a PNG file, through the `png` crate. Moved here unchanged from
//! the Animation crate's `ImageSource::from_png`, which is now a call to it.

use std::io::Cursor;

use png::{BitDepth, ColorType, Transformations};

use crate::images::{check_size, DecodedImage};

/// Decodes the bytes of a PNG FILE into a [`DecodedImage`] (the module's pixel contract: tightly
/// packed straight-alpha 8-bit RGBA, rows top to bottom). Every colour type and bit depth the
/// PNG format has is brought to straight-alpha 8-bit RGBA: grey becomes equal red, green and
/// blue; a palette is looked up (its transparency chunk gives alpha); a `tRNS` colour key becomes
/// alpha 0; 16-bit samples keep their HIGH byte (fine for pictures; a 16-bit normal map loses
/// its low byte); 1, 2 and 4-bit samples are scaled to the full range; Adam7 interlacing is
/// undone. The values are returned as stored: the file's gamma, chromaticity and ICC chunks are
/// not applied (the caller says whether they are sRGB colour or linear data). Of an animated PNG
/// only the default image is read. An image is at most
/// [`MAX_IMAGE_DIMENSION`](crate::MAX_IMAGE_DIMENSION) pixels wide and high. The error says what
/// is wrong with the file.
pub fn decode_png(bytes: &[u8]) -> Result<DecodedImage, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    // EXPAND: palettes to RGB, low bit depths to 8 bits, tRNS to an alpha channel; STRIP_16:
    // 16-bit samples to their high byte.
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| format!("The bytes are not a PNG file this crate can read ({e}): pass the whole file, as it was stored."))?;
    let (width, height) = {
        let info = reader.info();
        (info.width, info.height)
    };
    check_size("PNG", width, height)?;
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
    Ok(DecodedImage { width, height, rgba })
}
