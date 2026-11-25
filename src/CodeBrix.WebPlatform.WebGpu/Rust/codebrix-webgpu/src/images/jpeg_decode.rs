//! [`decode_jpeg`]: the pixels of a JPEG file, through the `zune-jpeg` crate (pure Rust; the
//! JPEG decoder the `image` crate uses). Not in C#.

use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

use crate::images::{check_size, DecodedImage};

/// Decodes the bytes of a JPEG FILE into a [`DecodedImage`] (the module's pixel contract: tightly
/// packed straight-alpha 8-bit RGBA, rows top to bottom). Baseline and progressive JPEG (Huffman
/// coded, 8-bit samples) are read, with any chroma subsampling; colour (YCbCr or RGB) becomes
/// RGBA, greyscale becomes equal red, green and blue, and alpha is 255 throughout (JPEG has
/// none). The values are returned as stored: an embedded ICC profile is NOT applied (the caller
/// says whether they are sRGB colour or linear data), and the EXIF orientation is NOT applied
/// (the pixels are as stored; a photograph straight from a phone may rely on that tag to stand
/// upright - glTF expects exactly the stored pixels).
///
/// REFUSED, each with a message that says so: CMYK and YCCK files (from print workflows; without
/// their colour profile a conversion to RGB is a guess - save them as RGB JPEG or PNG), 12-bit and
/// 16-bit samples, lossless JPEG, arithmetic coding, hierarchical (differential) JPEG, files with
/// other numbers of colour components, images wider or higher than
/// [`MAX_IMAGE_DIMENSION`](crate::MAX_IMAGE_DIMENSION), and damaged or cut-short files (the
/// decoder runs in its strict mode: it never fills a missing part with grey).
pub fn decode_jpeg(bytes: &[u8]) -> Result<DecodedImage, String> {
    if !bytes.starts_with(&[0xFF, 0xD8]) {
        return Err("The bytes are not a JPEG file (they do not start with the JPEG start-of-image marker FF D8): pass the whole file, as it was stored.".to_string());
    }
    if let Some(frame) = frame_header(bytes) {
        refuse_frame(frame)?;
    }
    // Strict: a damaged or cut-short file is an error, not a picture with a grey part. The size
    // limits are this function's own (checked below, with its message), so the decoder's are
    // set out of the way.
    let options = DecoderOptions::default().set_strict_mode(true).set_max_width(usize::from(u16::MAX)).set_max_height(usize::from(u16::MAX)).jpeg_set_out_colorspace(ColorSpace::RGBA);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    decoder.decode_headers().map_err(|e| format!("The JPEG's header could not be read ({e}): the file is damaged or cut short; pass the whole file, as it was stored."))?;
    let info = decoder.info().ok_or_else(|| "The JPEG's header could not be read: the file is damaged.".to_string())?;
    let (width, height) = (u32::from(info.width), u32::from(info.height));
    check_size("JPEG", width, height)?;
    match decoder.input_colorspace() {
        Some(ColorSpace::YCbCr | ColorSpace::RGB | ColorSpace::Luma) => {}
        Some(space @ (ColorSpace::CMYK | ColorSpace::YCCK)) => {
            let name = if space == ColorSpace::CMYK { "CMYK" } else { "YCCK (Adobe CMYK)" };
            return Err(format!("The JPEG is a {name} image, as print workflows make them; CMYK and YCCK JPEGs are not read, because without the file's colour profile their conversion to RGB is a guess: save it as an RGB JPEG or a PNG."));
        }
        _ => {
            return Err(format!("The JPEG has {} colour components, which is neither greyscale nor colour (1 or 3): save it as an RGB or greyscale JPEG, or a PNG.", info.components));
        }
    }
    let rgba = decoder.decode().map_err(|e| format!("The JPEG's image data could not be decoded ({e}): the file is damaged or cut short."))?;
    let expected = width as usize * height as usize * 4;
    if rgba.len() != expected {
        return Err(format!("The JPEG decoded to {} bytes where {expected} were expected: this is a fault of the decoder, not of the file.", rgba.len()));
    }
    Ok(DecodedImage { width, height, rgba })
}

/// What the frame header (the first SOFn segment) of a JPEG says: the SOF marker's number (0 to
/// 15) and the sample precision in bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrameHeader {
    pub(crate) sof: u8,
    pub(crate) precision: u8,
}

/// Finds the frame header by walking the segments from the start of the file. None when the
/// walk does not reach one (the decoder then judges the file).
pub(crate) fn frame_header(bytes: &[u8]) -> Option<FrameHeader> {
    let mut pos = 2;
    loop {
        // A marker: 0xFF, optional 0xFF fill bytes, the marker code.
        if *bytes.get(pos)? != 0xFF {
            return None;
        }
        while *bytes.get(pos)? == 0xFF {
            pos += 1;
        }
        let code = *bytes.get(pos)?;
        pos += 1;
        match code {
            // Markers without a length: TEM, RSTn, SOI.
            0x01 | 0xD0..=0xD8 => continue,
            // End of image or start of scan before any frame header.
            0xD9 | 0xDA => return None,
            _ => {}
        }
        let length = u16::from_be_bytes([*bytes.get(pos)?, *bytes.get(pos + 1)?]) as usize;
        if length < 2 {
            return None;
        }
        // SOF0 to SOF15, except DHT (C4), JPG (C8) and DAC (CC).
        if (0xC0..=0xCF).contains(&code) && !matches!(code, 0xC4 | 0xC8 | 0xCC) {
            return Some(FrameHeader { sof: code - 0xC0, precision: *bytes.get(pos + 2)? });
        }
        pos += length;
    }
}

/// The refusals the frame header decides: lossless, other than 8-bit samples, arithmetic
/// coding, hierarchical.
pub(crate) fn refuse_frame(frame: FrameHeader) -> Result<(), String> {
    let sof = frame.sof;
    if matches!(sof, 3 | 7 | 11 | 15) {
        return Err("The JPEG is a LOSSLESS JPEG; only baseline and progressive JPEG are read: save it as a baseline JPEG or a PNG.".to_string());
    }
    if frame.precision != 8 {
        return Err(format!("The JPEG has {}-bit samples; only 8-bit JPEG is read: save it as an 8-bit JPEG, or as a PNG.", frame.precision));
    }
    if matches!(sof, 9 | 10 | 13 | 14) {
        return Err("The JPEG is ARITHMETIC-coded; only Huffman-coded baseline and progressive JPEG are read: save it again as an ordinary (Huffman-coded) JPEG.".to_string());
    }
    if matches!(sof, 5 | 6) {
        return Err("The JPEG is a HIERARCHICAL (differential) JPEG; only baseline and progressive JPEG are read: save it as a baseline JPEG.".to_string());
    }
    Ok(())
}
