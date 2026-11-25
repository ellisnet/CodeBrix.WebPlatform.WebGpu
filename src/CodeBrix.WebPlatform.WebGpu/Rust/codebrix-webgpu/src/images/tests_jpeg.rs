//! Host tests of decode_jpeg. The JPEG files in fixtures/jpeg/ were made for these tests from
//! synthetic pixels (no outside picture): 16 x 8 pixels, the left half (200, 40, 40) and the right
//! half (40, 80, 220) - `cjpeg -quality 100 -sample 1x1` (libjpeg-turbo) for baseline.jpg, the
//! same with `-progressive` for progressive.jpg and with `-arithmetic` for arithmetic.jpg; the same
//! halves as grey 50 and 200, `cjpeg -quality 100 -grayscale`, for grey.jpg; 13 x 9 pixels of
//! (120, 160, 60) with cjpeg's default 4:2:0 subsampling, `-quality 95`, for odd-420.jpg; and the
//! 16 x 8 colour picture written by ImageMagick with `-colorspace CMYK` (an Adobe YCCK file) for
//! ycck.jpg. The 12-bit, lossless, hierarchical, CMYK and oversized cases are made from them by
//! changing header bytes.

use crate::images::jpeg_decode::{frame_header, FrameHeader};
use crate::images::{decode_jpeg, DecodedImage};

const BASELINE: &[u8] = include_bytes!("../../fixtures/jpeg/baseline.jpg");
const PROGRESSIVE: &[u8] = include_bytes!("../../fixtures/jpeg/progressive.jpg");
const ARITHMETIC: &[u8] = include_bytes!("../../fixtures/jpeg/arithmetic.jpg");
const GREY: &[u8] = include_bytes!("../../fixtures/jpeg/grey.jpg");
const ODD_420: &[u8] = include_bytes!("../../fixtures/jpeg/odd-420.jpg");
const YCCK: &[u8] = include_bytes!("../../fixtures/jpeg/ycck.jpg");

/// The pixel at (x, y).
fn pixel(image: &DecodedImage, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * image.width + x) * 4) as usize;
    [image.rgba[i], image.rgba[i + 1], image.rgba[i + 2], image.rgba[i + 3]]
}

/// True when every channel is within 4 of the expected value (JPEG is lossy).
fn near(actual: [u8; 4], expected: [u8; 4]) -> bool {
    actual.iter().zip(expected.iter()).all(|(a, e)| (i16::from(*a) - i16::from(*e)).abs() <= 4)
}

/// The offset of the first frame header's marker (FF Cn) in a fixture.
fn sof_offset(bytes: &[u8]) -> usize {
    (2..bytes.len() - 1).find(|&i| bytes[i] == 0xFF && matches!(bytes[i + 1], 0xC0 | 0xC2)).unwrap()
}

/// A copy of `bytes` with `change` applied.
fn changed(bytes: &[u8], change: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut copy = bytes.to_vec();
    change(&mut copy);
    copy
}

#[test]
fn a_baseline_jpeg_becomes_opaque_rgba_rows_top_to_bottom() {
    //Act
    let image = decode_jpeg(BASELINE).unwrap();
    //Assert
    assert_eq!(image.size(), (16, 8));
    assert_eq!(image.rgba.len(), 16 * 8 * 4);
    assert!(near(pixel(&image, 0, 0), [200, 40, 40, 255]), "{:?}", pixel(&image, 0, 0));
    assert!(near(pixel(&image, 7, 7), [200, 40, 40, 255]), "{:?}", pixel(&image, 7, 7));
    assert!(near(pixel(&image, 8, 0), [40, 80, 220, 255]), "{:?}", pixel(&image, 8, 0));
    assert!(near(pixel(&image, 15, 7), [40, 80, 220, 255]), "{:?}", pixel(&image, 15, 7));
    assert!(image.rgba.chunks_exact(4).all(|p| p[3] == 255));
}

#[test]
fn a_progressive_jpeg_reads_as_the_baseline_one() {
    //Act
    let progressive = decode_jpeg(PROGRESSIVE).unwrap();
    let baseline = decode_jpeg(BASELINE).unwrap();
    //Assert
    assert_eq!(progressive.size(), (16, 8));
    for (p, b) in progressive.rgba.chunks_exact(4).zip(baseline.rgba.chunks_exact(4)) {
        assert!(near([p[0], p[1], p[2], p[3]], [b[0], b[1], b[2], b[3]]), "{p:?} {b:?}");
    }
}

#[test]
fn a_greyscale_jpeg_becomes_equal_red_green_and_blue() {
    //Act
    let image = decode_jpeg(GREY).unwrap();
    //Assert
    assert_eq!(image.size(), (16, 8));
    assert!(image.rgba.chunks_exact(4).all(|p| p[0] == p[1] && p[1] == p[2] && p[3] == 255));
    assert!(near(pixel(&image, 0, 0), [50, 50, 50, 255]), "{:?}", pixel(&image, 0, 0));
    assert!(near(pixel(&image, 15, 7), [200, 200, 200, 255]), "{:?}", pixel(&image, 15, 7));
}

#[test]
fn a_subsampled_jpeg_of_an_odd_size_is_tightly_packed() {
    //Act
    let image = decode_jpeg(ODD_420).unwrap();
    //Assert
    assert_eq!(image.size(), (13, 9));
    assert_eq!(image.rgba.len(), 13 * 9 * 4);
    for y in 0..9 {
        for x in 0..13 {
            assert!(near(pixel(&image, x, y), [120, 160, 60, 255]), "({x}, {y}) {:?}", pixel(&image, x, y));
        }
    }
}

#[test]
fn the_frame_header_is_found_by_walking_the_segments() {
    assert_eq!(frame_header(BASELINE), Some(FrameHeader { sof: 0, precision: 8 }));
    assert_eq!(frame_header(PROGRESSIVE), Some(FrameHeader { sof: 2, precision: 8 }));
    assert_eq!(frame_header(ARITHMETIC), Some(FrameHeader { sof: 9, precision: 8 }));
    assert_eq!(frame_header(&BASELINE[..20]), None, "cut short before the frame header");
}

#[test]
fn cmyk_and_ycck_jpegs_are_refused_with_what_to_do() {
    //Arrange: the Adobe segment's transform byte (its last) says 2 for YCCK; 0 makes it CMYK
    let adobe = YCCK.windows(5).position(|w| w == b"Adobe").unwrap();
    let cmyk = changed(YCCK, |b| b[adobe + 11] = 0);
    //Act
    let ycck = decode_jpeg(YCCK).unwrap_err();
    let cmyk = decode_jpeg(&cmyk).unwrap_err();
    //Assert
    assert!(ycck.contains("YCCK") && ycck.contains("RGB JPEG or a PNG"), "{ycck}");
    assert!(cmyk.contains("a CMYK image") && cmyk.contains("not read"), "{cmyk}");
}

#[test]
fn twelve_bit_lossless_arithmetic_and_hierarchical_jpegs_are_refused() {
    //Arrange: the frame header is FF Cn, its length (2 bytes), then the precision
    let sof = sof_offset(BASELINE);
    let twelve_bit = changed(BASELINE, |b| b[sof + 4] = 12);
    let lossless = changed(BASELINE, |b| b[sof + 1] = 0xC3);
    let hierarchical = changed(BASELINE, |b| b[sof + 1] = 0xC5);
    //Act
    let errors = [decode_jpeg(&twelve_bit), decode_jpeg(&lossless), decode_jpeg(ARITHMETIC), decode_jpeg(&hierarchical)].map(|r| r.unwrap_err());
    //Assert
    assert!(errors[0].contains("12-bit samples"), "{}", errors[0]);
    assert!(errors[1].contains("LOSSLESS"), "{}", errors[1]);
    assert!(errors[2].contains("ARITHMETIC"), "{}", errors[2]);
    assert!(errors[3].contains("HIERARCHICAL"), "{}", errors[3]);
}

#[test]
fn a_jpeg_larger_than_the_limit_is_refused_before_it_is_decoded() {
    //Arrange: the height and the width follow the precision, big-endian
    let sof = sof_offset(BASELINE);
    let wide = changed(BASELINE, |b| b[sof + 7..sof + 9].copy_from_slice(&9000u16.to_be_bytes()));
    //Act
    let error = decode_jpeg(&wide).unwrap_err();
    //Assert
    assert!(error.contains("9000 x 8") && error.contains("8192 x 8192"), "{error}");
}

#[test]
fn damaged_cut_short_and_foreign_bytes_are_refused() {
    //Arrange: one file cut inside its headers, one cut inside its image data (after the SOS marker)
    let sos = (2..BASELINE.len() - 1).find(|&i| BASELINE[i] == 0xFF && BASELINE[i + 1] == 0xDA).unwrap();
    let in_header = &BASELINE[..sos / 2];
    let in_data = &BASELINE[..sos + (BASELINE.len() - sos) / 2];
    //Act
    let header = decode_jpeg(in_header).unwrap_err();
    let data = decode_jpeg(in_data).unwrap_err();
    let foreign = decode_jpeg(b"GIF89a....").unwrap_err();
    let empty = decode_jpeg(&[]).unwrap_err();
    //Assert
    assert!(header.contains("header") && header.contains("damaged or cut short"), "{header}");
    assert!(data.contains("image data") && data.contains("damaged or cut short"), "{data}");
    assert!(foreign.contains("not a JPEG file"), "{foreign}");
    assert!(empty.contains("not a JPEG file"), "{empty}");
}
