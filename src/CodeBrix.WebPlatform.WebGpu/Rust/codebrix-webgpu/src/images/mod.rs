//! Image files to pixels (not in C#: the desktop library decodes through its own image stack):
//! [`decode_png`], [`decode_jpeg`] and [`decode_image`], which looks at the first bytes and calls
//! one of the other two, all returning one [`DecodedImage`].
//!
//! THE PIXEL CONTRACT, the one form every CodeBrix.WebPlatform.WebGpu crate takes decoded pixels
//! in: tightly packed 8-bit RGBA (four bytes per pixel, red first, no padding between rows),
//! STRAIGHT alpha (not premultiplied), rows from the TOP of the picture to the bottom. The
//! functions here are NEUTRAL about colour space: they return the stored values as they are
//! (no gamma, chromaticity or ICC profile is applied), and the CALLER says whether they are
//! sRGB-encoded colour (a picture, a base-colour or emissive map) or linear data (a normal,
//! occlusion, roughness or metallic map).
//!
//! The three functions are separate on purpose: unused code is dropped when an application is
//! built, so an application that only calls [`decode_png`] does not carry the JPEG decoder, and
//! one that only calls [`decode_jpeg`] does not carry the PNG decoder. [`decode_image`] reaches
//! both.

mod jpeg_decode;
mod png_decode;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_jpeg;
#[cfg(test)]
mod tests_png;

pub use jpeg_decode::decode_jpeg;
pub use png_decode::decode_png;

/// The largest width or height, in pixels, that [`decode_png`], [`decode_jpeg`] and
/// [`decode_image`] accept (a GPU texture's limit in every browser that has WebGPU).
pub const MAX_IMAGE_DIMENSION: u32 = 8192;

/// The first eight bytes of every PNG file.
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// A decoded image: its size and its pixels in the pixel contract of this module - tightly
/// packed 8-bit RGBA, red first, straight alpha, rows top to bottom - as the file stored them
/// (the caller says whether they are sRGB-encoded colour or linear data).
#[derive(Clone, PartialEq, Eq)]
pub struct DecodedImage {
    /// The width in pixels (1 to [`MAX_IMAGE_DIMENSION`]).
    pub width: u32,
    /// The height in pixels (1 to [`MAX_IMAGE_DIMENSION`]).
    pub height: u32,
    /// `width * height * 4` bytes: R, G, B, A of each pixel, row by row from the top.
    pub rgba: Vec<u8>,
}

impl DecodedImage {
    /// The size in pixels (width, height).
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl std::fmt::Debug for DecodedImage {
    /// The size and the length of the pixels (not the pixels themselves).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodedImage").field("width", &self.width).field("height", &self.height).field("rgba_len", &self.rgba.len()).finish()
    }
}

/// Decodes the bytes of a PNG or a JPEG FILE, whichever they are (decided by the file's first
/// bytes, never by a name or a content type): a PNG goes to [`decode_png`], a JPEG to
/// [`decode_jpeg`], under their rules. Any other format is refused with a message that names it
/// where it can be recognised (GIF, WebP, KTX2, BMP, TIFF, AVIF, HEIF, JPEG XL, DDS, QOI, Radiance
/// HDR): only PNG and JPEG files are read, and an application that brings its own decoder for
/// another format hands over its pixels in the pixel contract instead.
///
/// Calling this function carries both decoders into the application; call [`decode_png`] or
/// [`decode_jpeg`] directly when only one format can occur.
pub fn decode_image(bytes: &[u8]) -> Result<DecodedImage, String> {
    if bytes.starts_with(&PNG_SIGNATURE) {
        return decode_png(bytes);
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return decode_jpeg(bytes);
    }
    Err(match other_format(bytes) {
        Some(name) => format!("The bytes are {name} file; only PNG and JPEG files are read here: convert it to PNG or JPEG, or decode it with your own decoder and hand over the pixels (tightly packed straight-alpha RGBA, rows top to bottom)."),
        None if bytes.is_empty() => "There are no bytes to decode: pass the whole PNG or JPEG file, as it was stored.".to_string(),
        None => {
            let start: Vec<String> = bytes.iter().take(8).map(|b| format!("{b:02X}")).collect();
            format!("The bytes are neither a PNG nor a JPEG file (they start with {}): only PNG and JPEG files are read here; pass the whole file, as it was stored.", start.join(" "))
        }
    })
}

/// The name (with its article) of a recognisable image format that is neither PNG nor JPEG.
pub(crate) fn other_format(bytes: &[u8]) -> Option<&'static str> {
    let at = |offset: usize, magic: &[u8]| bytes.len() >= offset + magic.len() && &bytes[offset..offset + magic.len()] == magic;
    if at(0, b"GIF87a") || at(0, b"GIF89a") {
        Some("a GIF")
    } else if at(0, b"RIFF") && at(8, b"WEBP") {
        Some("a WebP")
    } else if at(0, &[0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB]) {
        Some("a KTX2")
    } else if at(0, &[0xAB, b'K', b'T', b'X', b' ', b'1', b'1', 0xBB]) {
        Some("a KTX")
    } else if at(0, b"BM") && bytes.len() >= 14 {
        Some("a BMP")
    } else if at(0, b"II*\0") || at(0, b"MM\0*") {
        Some("a TIFF")
    } else if at(4, b"ftypavif") || at(4, b"ftypavis") {
        Some("an AVIF")
    } else if at(4, b"ftypheic") || at(4, b"ftypheix") || at(4, b"ftypmif1") || at(4, b"ftyphevc") {
        Some("a HEIF")
    } else if at(0, &[0xFF, 0x0A]) || at(0, &[0, 0, 0, 0x0C, b'J', b'X', b'L', b' ']) {
        Some("a JPEG XL")
    } else if at(0, b"DDS ") {
        Some("a DDS")
    } else if at(0, b"qoif") {
        Some("a QOI")
    } else if at(0, b"#?RADIANCE") || at(0, b"#?RGBE") {
        Some("a Radiance HDR")
    } else {
        None
    }
}

/// Checks a decoded size against the limits; the message names the format.
pub(crate) fn check_size(format: &str, width: u32, height: u32) -> Result<(), String> {
    if width < 1 || height < 1 || width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err(format!("The {format} is {width} x {height} pixels; an image must be between 1 x 1 and {MAX_IMAGE_DIMENSION} x {MAX_IMAGE_DIMENSION}: scale it down before handing it over."));
    }
    Ok(())
}
