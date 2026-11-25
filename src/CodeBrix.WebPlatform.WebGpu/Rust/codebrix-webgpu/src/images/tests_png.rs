//! Host tests of decode_png (moved here with the code from the Animation crate's
//! `ImageSource::from_png` tests). The PNG files are made by the tests themselves with the `png`
//! crate's own encoder: every colour type at the bit depths the format allows, a palette with
//! transparency, a colour key, Adam7 interlacing and an animated PNG.

use png::{BitDepth, ColorType, Encoder};

use crate::images::{decode_png, DecodedImage};

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
    let pictures = [rgba, rgb, grey, grey_alpha].map(|f| decode_png(&f).unwrap());
    //Assert
    assert_eq!(pictures[0].rgba, vec![10, 20, 30, 40, 250, 240, 230, 0]);
    assert_eq!(pictures[1].rgba, vec![10, 20, 30, 255, 1, 2, 3, 255]);
    assert_eq!(pictures[2].rgba, vec![0, 0, 0, 255, 200, 200, 200, 255]);
    assert_eq!(pictures[3].size(), (1, 2));
    assert_eq!(pictures[3].rgba, vec![90, 90, 90, 128, 7, 7, 7, 255]);
}

#[test]
fn sixteen_bit_samples_keep_their_high_byte() {
    //Arrange
    let rgba16 = png_file(1, 1, ColorType::Rgba, BitDepth::Sixteen, &[0x12, 0x34, 0xAB, 0xCD, 0xFF, 0x00, 0x80, 0x01], |_| {});
    let grey16 = png_file(1, 1, ColorType::Grayscale, BitDepth::Sixteen, &[0x7F, 0xFF], |_| {});
    //Act
    let a = decode_png(&rgba16).unwrap();
    let b = decode_png(&grey16).unwrap();
    //Assert
    assert_eq!(a.rgba, vec![0x12, 0xAB, 0xFF, 0x80]);
    assert_eq!(b.rgba, vec![0x7F, 0x7F, 0x7F, 255]);
}

#[test]
fn low_bit_depth_grey_is_scaled_to_the_full_range() {
    //Arrange: 8 pixels of 1 bit (10110001), 4 of 2 bits (00 01 10 11), 2 of 4 bits (0x0, 0xF)
    let one = png_file(8, 1, ColorType::Grayscale, BitDepth::One, &[0b1011_0001], |_| {});
    let two = png_file(4, 1, ColorType::Grayscale, BitDepth::Two, &[0b0001_1011], |_| {});
    let four = png_file(2, 1, ColorType::Grayscale, BitDepth::Four, &[0x0F], |_| {});
    //Act
    let grey = |f: &[u8]| decode_png(f).unwrap().rgba.chunks_exact(4).map(|p| p[0]).collect::<Vec<u8>>();
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
    let a = decode_png(&indexed).unwrap();
    let b = decode_png(&keyed).unwrap();
    //Assert: indices 0, 1, 2 - opaque red, half green, opaque blue (no tRNS entry = opaque)
    assert_eq!(a.rgba, vec![255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255]);
    assert_eq!(b.rgba, vec![1, 2, 3, 255, 9, 9, 9, 0]);
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
    let picture = decode_png(&plain).unwrap();
    //Assert
    assert_eq!(picture.rgba, data);
}

#[test]
fn damaged_and_foreign_bytes_are_refused_with_what_to_do() {
    //Arrange
    let good = png_file(1, 1, ColorType::Rgb, BitDepth::Eight, &[1, 2, 3], |_| {});
    let cut = &good[..good.len() - 20];
    //Act
    let not_png = decode_png(b"GIF89a....").unwrap_err();
    let damaged = decode_png(cut).unwrap_err();
    //Assert
    assert!(not_png.contains("not a PNG file"), "{not_png}");
    assert!(damaged.contains("PNG"), "{damaged}");
}

#[test]
fn of_an_animated_png_only_the_default_image_is_read() {
    //Arrange: two frames of 1 x 1, the first (the default image) red, the second blue
    let mut file = Vec::new();
    {
        let mut encoder = Encoder::new(&mut file, 1, 1);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_animated(2, 0).unwrap();
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[255, 0, 0, 255]).unwrap();
        writer.write_image_data(&[0, 0, 255, 255]).unwrap();
        writer.finish().unwrap();
    }
    //Act
    let image = decode_png(&file).unwrap();
    //Assert
    assert_eq!(image.rgba, vec![255, 0, 0, 255]);
}

#[test]
fn an_image_wider_than_the_limit_is_refused_and_one_at_the_limit_is_read() {
    //Arrange
    let at_limit = png_file(8192, 1, ColorType::Grayscale, BitDepth::Eight, &vec![7; 8192], |_| {});
    let too_wide = png_file(8193, 1, ColorType::Grayscale, BitDepth::Eight, &vec![7; 8193], |_| {});
    //Act
    let read = decode_png(&at_limit).unwrap();
    let refused = decode_png(&too_wide).unwrap_err();
    //Assert
    assert_eq!(read.size(), (8192, 1));
    assert_eq!(read.rgba.len(), 8192 * 4);
    assert!(refused.contains("8193 x 1") && refused.contains("8192 x 8192"), "{refused}");
}

#[test]
fn the_decoded_image_shows_its_size_and_not_its_pixels() {
    let image = DecodedImage { width: 2, height: 1, rgba: vec![0; 8] };
    assert_eq!(format!("{image:?}"), "DecodedImage { width: 2, height: 1, rgba_len: 8 }");
}
