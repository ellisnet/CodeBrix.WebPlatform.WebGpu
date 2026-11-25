//! Host tests of reading pictures from PNG files and from the picture-pixels form. The PNG rules
//! themselves are codebrix-webgpu's (decode_png) and are tested there; here, that from_png hands
//! over its pixels and its errors.

use crate::images::ImageSource;

/// A PNG file of 2 x 1 pixels, 8-bit RGBA: (10, 20, 30, 40) and (250, 240, 230, 0).
const TWO_PIXELS: [u8; 74] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0xF4, 0x22, 0x7F, 0x8A, 0x00, 0x00,
    0x00, 0x11, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0xE0, 0x12, 0x91, 0xD3, 0xF8, 0xF5, 0xE1, 0x19, 0x03, 0x00, 0x0A, 0xE5, 0x03, 0x35, 0x23, 0x2E, 0x7D, 0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44,
    0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn a_png_file_becomes_a_picture_of_straight_rgba() {
    //Act
    let picture = ImageSource::from_png(&TWO_PIXELS).unwrap();
    //Assert
    assert_eq!(picture.size(), (2, 1));
    assert_eq!(picture.to_rgba(), vec![10, 20, 30, 40, 250, 240, 230, 0]);
}

#[test]
fn damaged_and_foreign_bytes_are_refused_with_what_to_do() {
    //Arrange
    let cut = &TWO_PIXELS[..TWO_PIXELS.len() - 20];
    //Act
    let not_png = ImageSource::from_png(b"GIF89a....").unwrap_err();
    let damaged = ImageSource::from_png(cut).unwrap_err();
    //Assert
    assert!(not_png.contains("not a PNG file"), "{not_png}");
    assert!(damaged.contains("PNG"), "{damaged}");
}

/// The picture-pixels form as the .NET writer writes it.
fn picture_pixels(major: u16, minor: u16, width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = b"CBPX".to_vec();
    out.extend_from_slice(&major.to_le_bytes());
    out.extend_from_slice(&minor.to_le_bytes());
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(rgba);
    out
}

#[test]
fn the_picture_pixels_form_is_read_and_checked() {
    //Arrange
    let pixels = [1, 2, 3, 4, 5, 6, 7, 8];
    let current = picture_pixels(1, 0, 2, 1, &pixels);
    let mut later_minor = picture_pixels(1, 3, 2, 1, &pixels);
    later_minor.extend_from_slice(b"more");
    //Act
    let a = ImageSource::from_picture_pixels(&current).unwrap();
    let b = ImageSource::from_picture_pixels(&later_minor).unwrap();
    let other_major = ImageSource::from_picture_pixels(&picture_pixels(2, 0, 2, 1, &pixels)).unwrap_err();
    let short = ImageSource::from_picture_pixels(&picture_pixels(1, 0, 2, 2, &pixels)).unwrap_err();
    let foreign = ImageSource::from_picture_pixels(&[0; 20]).unwrap_err();
    //Assert
    assert_eq!(a.size(), (2, 1));
    assert_eq!(a.to_rgba(), pixels.to_vec());
    assert_eq!(b.to_rgba(), pixels.to_vec());
    assert!(other_major.contains("major version 2"), "{other_major}");
    assert!(short.contains("needs 16"), "{short}");
    assert!(foreign.contains("CBPX"), "{foreign}");
}
