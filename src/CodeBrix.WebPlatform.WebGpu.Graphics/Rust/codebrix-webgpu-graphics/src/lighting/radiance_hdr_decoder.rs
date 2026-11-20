//! [`RadianceHdrDecoder`]: Radiance RGBE (`.hdr`) images to linear-light [`FloatImage`]s (C#:
//! `RadianceHdrDecoder`). The decoding logic - the header, flat and old-style run-length
//! scanlines, new-style (adaptive) run-length scanlines, and the shared-exponent RGBE to float
//! conversion - follows the public-domain stb_image.h HDR loader, by way of the C# library's port.

use crate::context::GraphicsException;
use crate::lighting::FloatImage;

/// Decodes Radiance RGBE images (`.hdr`, the format Poly Haven and most HDRI sites serve) into
/// linear-light [`FloatImage`]s (C#: `RadianceHdrDecoder`). Orientations `-Y h +X w` (the
/// standard) and `+Y h +X w` (bottom-up) are read; the EXPOSURE header is ignored (the values are
/// returned as stored). OpenEXR (`.exr`) is not supported: convert such files to `.hdr`.
///
/// It takes the file's BYTES and never fetches anything: a page gets them from its backend (the
/// platform's generated `_bytes` call) or packs them into the frontend (`include_bytes!`).
pub struct RadianceHdrDecoder;

impl RadianceHdrDecoder {
    /// True when the bytes start like a Radiance file (`#?RADIANCE` or `#?RGBE`) (C#:
    /// `IsRadianceHdr`).
    pub fn is_radiance_hdr(data: &[u8]) -> bool {
        data.starts_with(b"#?RADIANCE") || data.starts_with(b"#?RGBE")
    }

    /// Decodes a whole Radiance file (C#: `Decode(ReadOnlySpan<byte>)`). An error that says what
    /// is wrong when the data is not a valid Radiance RGBE image.
    pub fn decode(data: &[u8]) -> Result<FloatImage, GraphicsException> {
        let mut position = 0usize;
        let signature = read_line(data, &mut position)?;
        if signature.as_deref() != Some("#?RADIANCE") && signature.as_deref() != Some("#?RGBE") {
            return Err(invalid("Not a Radiance RGBE (.hdr) file: the #?RADIANCE signature is missing."));
        }
        let mut format_seen = false;
        loop {
            let Some(line) = read_line(data, &mut position)? else {
                return Err(invalid("The .hdr header ends unexpectedly."));
            };
            if line.is_empty() {
                break;
            }
            if line.starts_with("FORMAT=") {
                if line != "FORMAT=32-bit_rle_rgbe" {
                    return Err(invalid(&format!("Unsupported .hdr format '{line}': only 32-bit_rle_rgbe is supported.")));
                }
                format_seen = true;
            }
        }
        if !format_seen {
            return Err(invalid("Invalid .hdr header: there is no FORMAT line."));
        }
        let Some(resolution) = read_line(data, &mut position)? else {
            return Err(invalid("The .hdr file ends before its resolution line."));
        };
        let parts: Vec<&str> = resolution.split(' ').filter(|p| !p.is_empty()).collect();
        let size = if parts.len() == 4 && (parts[0] == "-Y" || parts[0] == "+Y") && parts[2] == "+X" { parts[1].parse::<i64>().ok().zip(parts[3].parse::<i64>().ok()) } else { None };
        let (height, width) = match size {
            Some((height, width)) if width >= 1 && height >= 1 && width <= i32::MAX as i64 && height <= i32::MAX as i64 => (height as usize, width as usize),
            _ => return Err(invalid(&format!("Unsupported .hdr resolution line '{resolution}': only '-Y h +X w' and '+Y h +X w' are supported."))),
        };
        if (width as u64) * (height as u64) > 1u64 << 28 {
            return Err(invalid(&format!("The .hdr image is too large ({width} x {height}).")));
        }
        let bottom_up = parts[0] == "+Y";
        let scale = exponent_scale();
        let mut pixels = vec![0f32; width * height * 3];
        let mut scanline = vec![0u8; width * 4];
        for row in 0..height {
            read_scanline(data, &mut position, &mut scanline, width)?;
            let y = if bottom_up { height - 1 - row } else { row };
            let mut at = y * width * 3;
            for quad in scanline.chunks_exact(4) {
                let factor = scale[quad[3] as usize];
                pixels[at] = quad[0] as f32 * factor;
                pixels[at + 1] = quad[1] as f32 * factor;
                pixels[at + 2] = quad[2] as f32 * factor;
                at += 3;
            }
        }
        FloatImage::new(width as u32, height as u32, pixels)
    }

    /// One RGBE pixel as linear floats: value = mantissa x 2^(exponent - 136); exponent 0 is black
    /// (C#: `RgbeToFloat`, internal there; the decoder itself uses a table of the same factors).
    #[cfg(test)]
    pub(crate) fn rgbe_to_float(r: u8, g: u8, b: u8, e: u8) -> (f32, f32, f32) {
        let factor = exponent_factor(e);
        (r as f32 * factor, g as f32 * factor, b as f32 * factor)
    }
}

/// 2^(e - 136), and 0 for e = 0.
fn exponent_factor(e: u8) -> f32 {
    if e == 0 {
        0.0
    } else {
        2f32.powi(e as i32 - 136)
    }
}

/// The 256 factors, once per decode.
fn exponent_scale() -> [f32; 256] {
    let mut table = [0f32; 256];
    for (e, value) in table.iter_mut().enumerate() {
        *value = exponent_factor(e as u8);
    }
    table
}

fn invalid(message: &str) -> GraphicsException {
    GraphicsException::new(format!("{message} Give the decoder the bytes of a Radiance .hdr file (convert other formats to .hdr first)."))
}

fn truncated() -> GraphicsException {
    invalid("The .hdr pixel data ends unexpectedly.")
}

/// One scanline into `scanline` (RGBE quads): new-style RLE, or flat / old-style RLE.
fn read_scanline(data: &[u8], position: &mut usize, scanline: &mut [u8], width: usize) -> Result<(), GraphicsException> {
    if *position + 4 > data.len() {
        return Err(truncated());
    }
    let (b0, b1, b2, b3) = (data[*position], data[*position + 1], data[*position + 2], data[*position + 3]);

    // New-style RLE: 2, 2, the 16-bit width, then the four components run-length encoded one
    // after another (all R bytes, all G, all B, all E). Only used for widths 8..32767, as the
    // reference encoder does.
    if b0 == 2 && b1 == 2 && (b2 & 0x80) == 0 && (((b2 as usize) << 8) | b3 as usize) == width && width >= 8 {
        *position += 4;
        for component in 0..4 {
            let mut x = 0usize;
            while x < width {
                let Some(&count) = data.get(*position) else {
                    return Err(truncated());
                };
                *position += 1;
                let mut count = count as usize;
                if count > 128 {
                    count -= 128;
                    if x + count > width {
                        return Err(invalid("Corrupt .hdr RLE scanline: a run overflows the row."));
                    }
                    let Some(&value) = data.get(*position) else {
                        return Err(truncated());
                    };
                    *position += 1;
                    for i in 0..count {
                        scanline[(x + i) * 4 + component] = value;
                    }
                } else {
                    if count == 0 || x + count > width {
                        return Err(invalid("Corrupt .hdr RLE scanline: a literal overflows the row."));
                    }
                    if *position + count > data.len() {
                        return Err(truncated());
                    }
                    for i in 0..count {
                        scanline[(x + i) * 4 + component] = data[*position + i];
                    }
                    *position += count;
                }
                x += count;
            }
        }
        return Ok(());
    }

    // Flat / old-style scanline: RGBE quads; a quad (1, 1, 1, n) repeats the previous pixel
    // n << (8 x the number of repeat quads directly before it) times.
    let mut shift = 0u32;
    let mut pixel = 0usize;
    while pixel < width {
        if *position + 4 > data.len() {
            return Err(truncated());
        }
        let quad = [data[*position], data[*position + 1], data[*position + 2], data[*position + 3]];
        *position += 4;
        if quad[0] == 1 && quad[1] == 1 && quad[2] == 1 {
            if pixel == 0 {
                return Err(invalid("Corrupt .hdr scanline: a repeat marker has no previous pixel."));
            }
            let repeat = if shift >= 8 { u64::MAX } else { (quad[3] as u64) << (8 * shift) };
            if pixel as u64 + repeat > width as u64 {
                return Err(invalid("Corrupt .hdr scanline: an old-style run overflows the row."));
            }
            let previous = [scanline[(pixel - 1) * 4], scanline[(pixel - 1) * 4 + 1], scanline[(pixel - 1) * 4 + 2], scanline[(pixel - 1) * 4 + 3]];
            for i in 0..repeat as usize {
                scanline[(pixel + i) * 4..(pixel + i) * 4 + 4].copy_from_slice(&previous);
            }
            pixel += repeat as usize;
            shift += 1;
        } else {
            scanline[pixel * 4..pixel * 4 + 4].copy_from_slice(&quad);
            pixel += 1;
            shift = 0;
        }
    }
    Ok(())
}

/// One header line (CR dropped), or None at the end of the data.
fn read_line(data: &[u8], position: &mut usize) -> Result<Option<String>, GraphicsException> {
    if *position >= data.len() {
        return Ok(None);
    }
    let mut line = String::with_capacity(64);
    while *position < data.len() {
        let value = data[*position];
        *position += 1;
        if value == b'\n' {
            return Ok(Some(line));
        }
        if value != b'\r' {
            line.push(value as char);
        }
        if line.len() > 4096 {
            return Err(invalid("The .hdr header has a line longer than 4096 characters."));
        }
    }
    Ok(Some(line))
}

#[cfg(test)]
mod tests {
    use super::RadianceHdrDecoder;

    fn bytes(header: &str, parts: &[&[u8]]) -> Vec<u8> {
        let mut data = header.as_bytes().to_vec();
        for part in parts {
            data.extend_from_slice(part);
        }
        data
    }

    #[test]
    fn rgbe_conversion_matches_the_shared_exponent_formula() {
        // Mantissa 128 with exponent 129 = 128 x 2^(129 - 136) = 1.
        assert_eq!(RadianceHdrDecoder::rgbe_to_float(128, 64, 0, 129), (1.0, 0.5, 0.0));
    }

    #[test]
    fn zero_exponent_decodes_to_black() {
        assert_eq!(RadianceHdrDecoder::rgbe_to_float(200, 200, 200, 0), (0.0, 0.0, 0.0));
    }

    #[test]
    fn flat_scanlines_decode_with_correct_dimensions_and_values() {
        // 2 x 2: one bright red pixel (2.0, beyond display range), three black.
        let hdr = bytes("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 2 +X 2\n", &[&[128, 0, 0, 130], &[0, 0, 0, 0], &[0, 0, 0, 0], &[0, 0, 0, 0]]);
        let image = RadianceHdrDecoder::decode(&hdr).unwrap();
        assert_eq!((image.width(), image.height()), (2, 2));
        assert_eq!(image.get_pixel(0, 0), (2.0, 0.0, 0.0));
    }

    #[test]
    fn old_style_repeat_runs_are_expanded() {
        // Width 4: one pixel, (1, 1, 1, 2) = repeat it twice, then one more.
        let hdr = bytes("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 1 +X 4\n", &[&[128, 0, 0, 129], &[1, 1, 1, 2], &[0, 128, 0, 129]]);
        let image = RadianceHdrDecoder::decode(&hdr).unwrap();
        for x in 0..3 {
            assert_eq!(image.get_pixel(x, 0).0, 1.0);
        }
        assert_eq!(image.get_pixel(3, 0).1, 1.0);
    }

    #[test]
    fn consecutive_old_style_repeat_markers_shift_the_count() {
        // Width 262: one pixel, then repeat 5 (5) and repeat 1 shifted by 8 (256) = 261 copies.
        let hdr = bytes("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 1 +X 262\n", &[&[64, 64, 64, 129], &[1, 1, 1, 5], &[1, 1, 1, 1]]);
        let image = RadianceHdrDecoder::decode(&hdr).unwrap();
        assert_eq!(image.get_pixel(261, 0).0, 0.5);
    }

    #[test]
    fn new_style_rle_scanlines_decode() {
        // Width 8 (the minimum for new RLE): 2, 2, width, then four planar components, each one run of 8.
        let hdr = bytes("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 1 +X 8\n", &[&[2, 2, 0, 8], &[136, 128], &[136, 0], &[136, 0], &[136, 129]]);
        let image = RadianceHdrDecoder::decode(&hdr).unwrap();
        assert_eq!(image.width(), 8);
        for x in 0..8 {
            assert_eq!(image.get_pixel(x, 0), (1.0, 0.0, 0.0));
        }
    }

    #[test]
    fn new_style_rle_literal_packets_decode() {
        // R plane = literal (10, 20, 30) then a run of 5 x 40; exponent 129 scales by 1/128.
        let hdr = bytes("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 1 +X 8\n", &[&[2, 2, 0, 8], &[3, 10, 20, 30, 133, 40], &[136, 0], &[136, 0], &[136, 129]]);
        let image = RadianceHdrDecoder::decode(&hdr).unwrap();
        assert!((image.get_pixel(0, 0).0 - 10.0 / 128.0).abs() < 1e-6);
        assert!((image.get_pixel(2, 0).0 - 30.0 / 128.0).abs() < 1e-6);
        assert!((image.get_pixel(7, 0).0 - 40.0 / 128.0).abs() < 1e-6);
    }

    #[test]
    fn bottom_up_files_are_flipped_to_top_down() {
        // "+Y 2 +X 1": the first stored row is the BOTTOM row.
        let hdr = bytes("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n+Y 2 +X 1\n", &[&[128, 0, 0, 129], &[0, 0, 128, 129]]);
        let image = RadianceHdrDecoder::decode(&hdr).unwrap();
        assert_eq!(image.get_pixel(0, 0).2, 1.0);
        assert_eq!(image.get_pixel(0, 1).0, 1.0);
    }

    #[test]
    fn rgbe_signature_crlf_lines_comments_and_other_header_keys_are_accepted() {
        let hdr = bytes("#?RGBE\r\n# made by a test\r\nEXPOSURE=2.0\r\nFORMAT=32-bit_rle_rgbe\r\n\r\n-Y 1 +X 1\r\n", &[&[128, 128, 128, 129]]);
        let image = RadianceHdrDecoder::decode(&hdr).unwrap();
        // EXPOSURE is not applied.
        assert_eq!(image.get_pixel(0, 0).0, 1.0);
    }

    #[test]
    fn malformed_headers_are_rejected() {
        for header in [
            "not an hdr file at all",
            "#?RADIANCE\nFORMAT=32-bit_rle_xyze\n\n-Y 1 +X 1\n",
            "#?RADIANCE\nGAMMA=1\n\n-Y 1 +X 1\n",
            "#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n+X 1 -Y 1\n",
            "#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 0 +X 1\n",
            "#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n",
        ] {
            let data = bytes(header, &[&[0x80, 0x80, 0x80, 0x81]]);
            let error = RadianceHdrDecoder::decode(&data).unwrap_err();
            assert!(error.to_string().contains(".hdr"), "{header}: {error}");
        }
    }

    #[test]
    fn truncated_pixel_data_is_rejected() {
        // The header promises 2 x 2 but only one pixel follows.
        let hdr = bytes("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 2 +X 2\n", &[&[128, 0, 0, 129]]);
        let error = RadianceHdrDecoder::decode(&hdr).unwrap_err();
        assert!(error.to_string().contains("ends unexpectedly"), "{error}");
    }

    #[test]
    fn corrupt_rle_runs_are_rejected() {
        // A run of 9 in a row of 8.
        let hdr = bytes("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 1 +X 8\n", &[&[2, 2, 0, 8], &[137, 1], &[136, 0], &[136, 0], &[136, 129]]);
        assert!(RadianceHdrDecoder::decode(&hdr).is_err());
    }

    #[test]
    fn the_signature_is_recognised() {
        assert!(RadianceHdrDecoder::is_radiance_hdr(b"#?RADIANCE\n"));
        assert!(RadianceHdrDecoder::is_radiance_hdr(b"#?RGBE\n"));
        assert!(!RadianceHdrDecoder::is_radiance_hdr(b"\x76\x2f\x31\x01"));
    }
}
