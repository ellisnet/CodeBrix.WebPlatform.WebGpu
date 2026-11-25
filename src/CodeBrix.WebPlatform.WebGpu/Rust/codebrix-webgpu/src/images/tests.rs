//! Host tests of decode_image: the first bytes decide, PNG and JPEG go to their decoders, other
//! formats are named in the refusal.

use png::{BitDepth, ColorType, Encoder};

use crate::images::{decode_image, decode_jpeg, decode_png, other_format};

const BASELINE: &[u8] = include_bytes!("../../fixtures/jpeg/baseline.jpg");

fn one_pixel_png() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = Encoder::new(&mut out, 1, 1);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[1, 2, 3, 4]).unwrap();
        writer.finish().unwrap();
    }
    out
}

#[test]
fn a_png_and_a_jpeg_go_to_their_own_decoders() {
    //Arrange
    let png = one_pixel_png();
    //Act
    let from_png = decode_image(&png).unwrap();
    let from_jpeg = decode_image(BASELINE).unwrap();
    //Assert
    assert_eq!(from_png, decode_png(&png).unwrap());
    assert_eq!(from_jpeg, decode_jpeg(BASELINE).unwrap());
}

#[test]
fn other_formats_are_named_in_the_refusal() {
    //Arrange
    let webp = b"RIFF\x10\x00\x00\x00WEBPVP8 ";
    let ktx2 = [0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, 0, 0, 0, 0];
    //Act
    let errors = [decode_image(b"GIF89a\x01\x00\x01\x00"), decode_image(webp), decode_image(&ktx2)].map(|r| r.unwrap_err());
    //Assert
    assert!(errors[0].contains("a GIF file") && errors[0].contains("only PNG and JPEG"), "{}", errors[0]);
    assert!(errors[1].contains("a WebP file"), "{}", errors[1]);
    assert!(errors[2].contains("a KTX2 file"), "{}", errors[2]);
}

#[test]
fn unknown_and_empty_bytes_are_refused_with_what_they_start_with() {
    //Act
    let unknown = decode_image(&[1, 2, 3, 0xAB]).unwrap_err();
    let empty = decode_image(&[]).unwrap_err();
    //Assert
    assert!(unknown.contains("start with 01 02 03 AB"), "{unknown}");
    assert!(empty.contains("no bytes"), "{empty}");
}

#[test]
fn the_formats_recognised_by_their_first_bytes() {
    assert_eq!(other_format(b"BM\0\0\0\0\0\0\0\0\0\0\0\0"), Some("a BMP"));
    assert_eq!(other_format(b"II*\0...."), Some("a TIFF"));
    assert_eq!(other_format(b"\0\0\0\x1CftypavifXXXX"), Some("an AVIF"));
    assert_eq!(other_format(b"\0\0\0\x18ftypheicXXXX"), Some("a HEIF"));
    assert_eq!(other_format(b"#?RADIANCE\n"), Some("a Radiance HDR"));
    assert_eq!(other_format(b"qoif...."), Some("a QOI"));
    assert_eq!(other_format(b"DDS |..."), Some("a DDS"));
    assert_eq!(other_format(&[0xFF, 0x0A, 0, 0]), Some("a JPEG XL"));
    assert_eq!(other_format(b"BM"), None, "too short to be a BMP");
    assert_eq!(other_format(b"<svg"), None);
}
