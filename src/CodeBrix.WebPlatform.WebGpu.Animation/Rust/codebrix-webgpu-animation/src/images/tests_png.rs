//! Host tests of reading pictures from PNG files and from the picture-pixels form. The PNG files
//! are made by the tests themselves with the `png` crate's own encoder: every colour type at
//! the bit depths the format allows, a palette with transparency and a colour key.

use png::{BitDepth, ColorType, Encoder};

use crate::images::ImageSource;

/// A PNG of `width` x `height` with these raw sample rows (already packed for the depth).
fn png_file(width: u32, height: u32, color: ColorType, depth: BitDepth, data: &[u8], setup: impl FnOnce(&mut Encoder<'_, &mut Vec<u8>>)) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = Encoder::new(&mut out, width, height);
        encoder.set_color(color);
        encoder.set_depth(depth);
        setup(&mut encoder);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(data).unwrap();
        writer.finish().unwrap();
    }
    out
}

#[test]
fn eight_bit_rgba_rgb_grey_and_grey_alpha_become_straight_rgba() {
    //Arrange
    let rgba = png_file(2, 1, ColorType::Rgba, BitDepth::Eight, &[10, 20, 30, 40, 250, 240, 230, 0], |_| {});
    let rgb = png_file(2, 1, ColorType::Rgb, BitDepth::Eight, &[10, 20, 30, 1, 2, 3], |_| {});
    let grey = png_file(2, 1, ColorType::Grayscale, BitDepth::Eight, &[0, 200], |_| {});
    let grey_alpha = png_file(1, 2, ColorType::GrayscaleAlpha, BitDepth::Eight, &[90, 128, 7, 255], |_| {});
    //Act
    let pictures = [rgba, rgb, grey, grey_alpha].map(|f| ImageSource::from_png(&f).unwrap());
    //Assert
    assert_eq!(pictures[0].to_rgba(), vec![10, 20, 30, 40, 250, 240, 230, 0]);
    assert_eq!(pictures[1].to_rgba(), vec![10, 20, 30, 255, 1, 2, 3, 255]);
    assert_eq!(pictures[2].to_rgba(), vec![0, 0, 0, 255, 200, 200, 200, 255]);
    assert_eq!(pictures[3].size(), (1, 2));
    assert_eq!(pictures[3].to_rgba(), vec![90, 90, 90, 128, 7, 7, 7, 255]);
}

#[test]
fn sixteen_bit_samples_keep_their_high_byte() {
    //Arrange
    let rgba16 = png_file(1, 1, ColorType::Rgba, BitDepth::Sixteen, &[0x12, 0x34, 0xAB, 0xCD, 0xFF, 0x00, 0x80, 0x01], |_| {});
    let grey16 = png_file(1, 1, ColorType::Grayscale, BitDepth::Sixteen, &[0x7F, 0xFF], |_| {});
    //Act
    let a = ImageSource::from_png(&rgba16).unwrap();
    let b = ImageSource::from_png(&grey16).unwrap();
    //Assert
    assert_eq!(a.to_rgba(), vec![0x12, 0xAB, 0xFF, 0x80]);
    assert_eq!(b.to_rgba(), vec![0x7F, 0x7F, 0x7F, 255]);
}

#[test]
fn low_bit_depth_grey_is_scaled_to_the_full_range() {
    //Arrange: 8 pixels of 1 bit (10110001), 4 of 2 bits (00 01 10 11), 2 of 4 bits (0x0, 0xF)
    let one = png_file(8, 1, ColorType::Grayscale, BitDepth::One, &[0b1011_0001], |_| {});
    let two = png_file(4, 1, ColorType::Grayscale, BitDepth::Two, &[0b0001_1011], |_| {});
    let four = png_file(2, 1, ColorType::Grayscale, BitDepth::Four, &[0x0F], |_| {});
    //Act
    let grey = |f: &[u8]| ImageSource::from_png(f).unwrap().to_rgba().chunks_exact(4).map(|p| p[0]).collect::<Vec<u8>>();
    //Assert
    assert_eq!(grey(&one), vec![255, 0, 255, 255, 0, 0, 0, 255]);
    assert_eq!(grey(&two), vec![0, 85, 170, 255]);
    assert_eq!(grey(&four), vec![0, 255]);
}

#[test]
fn a_palette_with_transparency_and_a_colour_key_become_alpha() {
    //Arrange: a 2-bit palette of three colours, the second half transparent; an RGB colour key
    let indexed = png_file(3, 1, ColorType::Indexed, BitDepth::Two, &[0b0001_1000], |e| {
        e.set_palette(vec![255, 0, 0, 0, 255, 0, 0, 0, 255]);
        e.set_trns(vec![255, 128]);
    });
    let keyed = png_file(2, 1, ColorType::Rgb, BitDepth::Eight, &[1, 2, 3, 9, 9, 9], |e| e.set_trns(vec![0, 9, 0, 9, 0, 9]));
    //Act
    let a = ImageSource::from_png(&indexed).unwrap();
    let b = ImageSource::from_png(&keyed).unwrap();
    //Assert: indices 0, 1, 2 - opaque red, half green, opaque blue (no tRNS entry = opaque)
    assert_eq!(a.to_rgba(), vec![255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255]);
    assert_eq!(b.to_rgba(), vec![1, 2, 3, 255, 9, 9, 9, 0]);
}

#[test]
fn a_picture_of_several_rows_keeps_every_pixel_in_place() {
    //Arrange
    let mut data = Vec::new();
    for y in 0..9u8 {
        for x in 0..9u8 {
            data.extend_from_slice(&[x * 20, y * 20, x ^ y, 255]);
        }
    }
    let plain = png_file(9, 9, ColorType::Rgba, BitDepth::Eight, &data, |_| {});
    //Act
    let picture = ImageSource::from_png(&plain).unwrap();
    //Assert
    assert_eq!(picture.to_rgba(), data);
}

#[test]
fn damaged_and_foreign_bytes_are_refused_with_what_to_do() {
    //Arrange
    let good = png_file(1, 1, ColorType::Rgb, BitDepth::Eight, &[1, 2, 3], |_| {});
    let cut = &good[..good.len() - 20];
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
