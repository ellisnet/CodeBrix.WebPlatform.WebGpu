//! The image side of the glTF builder: telling an image's format from its first bytes, and
//! reducing a decoded picture to the largest size the options allow (C#: the reference
//! converter's `TextureDecoder` and its glTF library's format tests). Decoding itself is
//! `codebrix_webgpu::decode_image`: PNG and JPEG, nothing else.

/// What an image's first bytes say it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImageFormat {
    /// PNG: read.
    Png,
    /// JPEG: read.
    Jpeg,
    /// KTX2 (Basis Universal, `KHR_texture_basisu`): refused by name.
    Ktx2,
    /// WebP (`EXT_texture_webp`): refused by name.
    WebP,
    /// DDS (`MSFT_texture_dds`): refused by name.
    Dds,
    /// AVIF (`EXT_texture_avif`): refused by name.
    Avif,
    /// No bytes at all.
    Empty,
    /// Anything else.
    Other,
}

impl ImageFormat {
    /// The format of these bytes (C#: `MemoryImage.IsPng`, `IsJpg`, `IsKtx2`, `IsDds`, `IsWebp`).
    pub(crate) fn of(bytes: &[u8]) -> ImageFormat {
        if bytes.is_empty() {
            ImageFormat::Empty
        } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
            ImageFormat::Png
        } else if bytes.starts_with(&[0xFF, 0xD8]) {
            ImageFormat::Jpeg
        } else if bytes.starts_with(&[0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, 0x0D, 0x0A, 0x1A, 0x0A]) {
            ImageFormat::Ktx2
        } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            ImageFormat::WebP
        } else if bytes.starts_with(b"DDS ") {
            ImageFormat::Dds
        } else if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && (&bytes[8..12] == b"avif" || &bytes[8..12] == b"avis") {
            ImageFormat::Avif
        } else {
            ImageFormat::Other
        }
    }

    /// True for the two formats the loader decodes.
    pub(crate) fn is_decodable(self) -> bool {
        matches!(self, ImageFormat::Png | ImageFormat::Jpeg)
    }

    /// The format as the warning names it.
    pub(crate) fn phrase(self) -> &'static str {
        match self {
            ImageFormat::Png => "a PNG image",
            ImageFormat::Jpeg => "a JPEG image",
            ImageFormat::Ktx2 => "a KTX2 (Basis Universal) image",
            ImageFormat::WebP => "a WebP image",
            ImageFormat::Dds => "a DDS image",
            ImageFormat::Avif => "an AVIF image",
            ImageFormat::Empty => "an image with no bytes",
            ImageFormat::Other => "an image that is neither PNG nor JPEG",
        }
    }
}

/// The size a `width` x `height` picture is reduced to for a largest side of `max` (0: any):
/// both sides are halved until both fit, or until one of them is 1 (C#: the loop of
/// `TextureDecoder.Decode`, the same sizes for the same arguments).
pub(crate) fn fitted_size(width: u32, height: u32, max: u32) -> (u32, u32) {
    let (mut w, mut h) = (width, height);
    if max > 0 {
        while (w > max || h > max) && w > 1 && h > 1 {
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
    }
    (w, h)
}

/// A picture reduced to `target` pixels: every new pixel is the average of the pixels it covers
/// (straight alpha; the colour of a pixel counts by its alpha, so that a transparent pixel's
/// colour does not bleed into its neighbours). `rgba` is `width * height * 4` bytes, rows top to
/// bottom. Not in C#: the reference converter resamples with its imaging library's default
/// filter; the sizes are the same, the pixels alike but not equal.
pub(crate) fn reduce(rgba: &[u8], width: u32, height: u32, target: (u32, u32)) -> Vec<u8> {
    let (target_width, target_height) = (target.0.max(1) as usize, target.1.max(1) as usize);
    let (width, height) = (width as usize, height as usize);
    let mut out = Vec::with_capacity(target_width * target_height * 4);
    for y in 0..target_height {
        let y0 = y * height / target_height;
        let y1 = ((y + 1) * height / target_height).max(y0 + 1).min(height);
        for x in 0..target_width {
            let x0 = x * width / target_width;
            let x1 = ((x + 1) * width / target_width).max(x0 + 1).min(width);
            let (mut red, mut green, mut blue, mut alpha, mut plain) = (0u64, 0u64, 0u64, 0u64, [0u64; 3]);
            for row in y0..y1 {
                for column in x0..x1 {
                    let at = (row * width + column) * 4;
                    let a = rgba[at + 3] as u64;
                    red += rgba[at] as u64 * a;
                    green += rgba[at + 1] as u64 * a;
                    blue += rgba[at + 2] as u64 * a;
                    alpha += a;
                    plain[0] += rgba[at] as u64;
                    plain[1] += rgba[at + 1] as u64;
                    plain[2] += rgba[at + 2] as u64;
                }
            }
            let count = ((y1 - y0) * (x1 - x0)) as u64;
            if alpha == 0 {
                // Fully transparent: keep the plain average colour (it may matter to a mask).
                out.extend_from_slice(&[((plain[0] + count / 2) / count) as u8, ((plain[1] + count / 2) / count) as u8, ((plain[2] + count / 2) / count) as u8, 0]);
            } else {
                out.extend_from_slice(&[((red + alpha / 2) / alpha) as u8, ((green + alpha / 2) / alpha) as u8, ((blue + alpha / 2) / alpha) as u8, ((alpha + count / 2) / count) as u8]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{fitted_size, reduce, ImageFormat};

    #[test]
    fn a_format_is_told_from_the_first_bytes() {
        assert_eq!(ImageFormat::of(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]), ImageFormat::Png);
        assert_eq!(ImageFormat::of(&[0xFF, 0xD8, 0xFF, 0xE0]), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::of(&[0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0]), ImageFormat::Ktx2);
        assert_eq!(ImageFormat::of(b"RIFF\x10\x00\x00\x00WEBPVP8 "), ImageFormat::WebP);
        assert_eq!(ImageFormat::of(b"DDS \x7c\x00\x00\x00"), ImageFormat::Dds);
        assert_eq!(ImageFormat::of(b"\x00\x00\x00\x1cftypavif\x00\x00"), ImageFormat::Avif);
        assert_eq!(ImageFormat::of(b"GIF89a"), ImageFormat::Other);
        assert_eq!(ImageFormat::of(&[]), ImageFormat::Empty);
        assert!(ImageFormat::Png.is_decodable() && ImageFormat::Jpeg.is_decodable() && !ImageFormat::WebP.is_decodable());
        assert!(ImageFormat::Ktx2.phrase().contains("KTX2") && ImageFormat::WebP.phrase().contains("WebP"));
    }

    #[test]
    fn a_size_is_halved_until_both_sides_fit() {
        assert_eq!(fitted_size(2048, 2048, 0), (2048, 2048));
        assert_eq!(fitted_size(2048, 1024, 1024), (1024, 512));
        assert_eq!(fitted_size(2048, 2048, 100), (64, 64));
        assert_eq!(fitted_size(100, 60, 64), (50, 30));
        assert_eq!(fitted_size(101, 61, 64), (50, 30));
        assert_eq!(fitted_size(64, 64, 64), (64, 64));
        // The reference's loop stops when a side reaches 1, so a thin picture stays too long.
        assert_eq!(fitted_size(4096, 2, 64), (2048, 1));
    }

    #[test]
    fn a_reduced_picture_averages_what_each_pixel_covers() {
        // 4 x 2 opaque: left half red 100 and 200, right half blue 50.
        let mut rgba = Vec::new();
        for _row in 0..2 {
            rgba.extend_from_slice(&[100, 0, 0, 255, 200, 0, 0, 255, 0, 0, 50, 255, 0, 0, 50, 255]);
        }
        assert_eq!(reduce(&rgba, 4, 2, (2, 1)), vec![150, 0, 0, 255, 0, 0, 50, 255]);
        assert_eq!(reduce(&rgba, 4, 2, (4, 2)), rgba);
        // A transparent pixel's colour does not bleed: red opaque beside green transparent.
        let mixed = [255, 0, 0, 255, 0, 255, 0, 0];
        assert_eq!(reduce(&mixed, 2, 1, (1, 1)), vec![255, 0, 0, 128]);
        // Fully transparent pixels keep their plain average colour.
        assert_eq!(reduce(&[10, 20, 30, 0, 30, 40, 50, 0], 2, 1, (1, 1)), vec![20, 30, 40, 0]);
    }
}
