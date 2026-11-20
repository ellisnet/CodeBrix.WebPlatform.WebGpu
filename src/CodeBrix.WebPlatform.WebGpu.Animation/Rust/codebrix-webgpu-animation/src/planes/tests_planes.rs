//! Host tests of the planes, filters, transitions, noise, particles and motion blur, ported from
//! NoSkia's Planes tests with the values they state (the bit-for-bit runs against NoSkia itself
//! are in tests_reference.rs).

use crate::planes::plane_kernels::gaussian_kernel;
use crate::planes::{
    DrawingFrameAccumulator, DrawingMotionBlur, DrawingNoise, DrawingNoiseFillOptions, DrawingNoiseFractalOptions, DrawingNoiseKind, DrawingParticleEmitter, DrawingParticleSystem, DrawingPlane, DrawingPlaneBloomOptions, DrawingPlaneImage,
    DrawingPlaneResampling, DrawingTransition, DrawingTransitionDirection, DrawingTransitionKind,
};
use crate::{Color, Rectangle};

fn row(values: &[f32]) -> DrawingPlane {
    DrawingPlane::from_data(values.len() as u32, 1, values.to_vec()).unwrap()
}

fn solid(count: usize, r: u8, g: u8, b: u8, a: u8) -> Vec<u8> {
    (0..count).flat_map(|_| [r, g, b, a]).collect()
}

fn ramp(count: usize, step: u8) -> Vec<u8> {
    (0..count).flat_map(|i| [i as u8 * step, 0, 0, 255]).collect()
}

fn reds(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks(4).map(|p| p[0]).collect()
}

fn apply(t: DrawingTransition, from: &[u8], to: &[u8], width: u32, height: u32, fraction: f32) -> Vec<u8> {
    let mut destination = vec![0u8; from.len()];
    t.apply_bytes(from, to, &mut destination, width, height, fraction).unwrap();
    destination
}

// ---- DrawingPlane ----------------------------------------------------------------------------

#[test]
fn a_new_plane_is_zeroed_row_major_and_refuses_empty_sizes() {
    let mut plane = DrawingPlane::new(3, 2).unwrap();
    assert_eq!((plane.width(), plane.height(), plane.length(), plane.size()), (3, 2, 6, (3, 2)));
    assert!(plane.data().iter().all(|v| *v == 0.0));
    plane.set(2, 1, 5.0).unwrap();
    assert_eq!(plane.data()[3 + 2], 5.0);
    assert_eq!(plane.get_row(1).unwrap()[2], 5.0);
    assert!(plane.get(2, 2).is_err());
    assert!(DrawingPlane::new(0, 1).is_err());
    assert!(DrawingPlane::new(1, 0).is_err());
    assert!(DrawingPlane::from_data(2, 2, vec![0.0; 3]).is_err());
}

#[test]
fn fill_area_is_clipped_and_fill_with_receives_absolute_positions() {
    let mut plane = DrawingPlane::new(4, 3).unwrap();
    plane.fill_area(Rectangle::new(2, 1, 10, 10), 1.0);
    assert_eq!(plane.data(), &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0]);
    let mut outside = DrawingPlane::new(4, 3).unwrap();
    outside.fill_area(Rectangle::new(-5, -5, 3, 3), 1.0);
    assert!(outside.data().iter().all(|v| *v == 0.0));
    let mut plane = DrawingPlane::new(3, 3).unwrap();
    plane.fill_area_with(Rectangle::new(1, 1, 2, 2), |x, y| (x * 10 + y) as f32);
    assert_eq!(plane.data(), &[0.0, 0.0, 0.0, 0.0, 11.0, 21.0, 0.0, 12.0, 22.0]);
}

#[test]
fn arithmetic_follows_the_formulas_and_does_not_clamp() {
    let mut plane = row(&[1.0, 2.0, 3.0]);
    plane.add_plane(&row(&[0.5, 1.0, -2.0]), Some(2.0)).unwrap().subtract(&row(&[1.0, 1.0, 1.0])).unwrap().add(0.25);
    assert_eq!(plane.data(), &[1.25, 3.25, -1.75]);
    let mut over = DrawingPlane::with_value(1, 1, 0.75).unwrap();
    over.add_plane(&DrawingPlane::with_value(1, 1, 0.75).unwrap(), None).unwrap();
    assert_eq!(over.data(), &[1.5]);
    let mut plane = row(&[1.0, 2.0, 3.0]);
    plane.multiply(2.0).multiply_plane(&row(&[0.5, 0.0, -1.0])).unwrap();
    assert_eq!(plane.data(), &[1.0, 0.0, -6.0]);
    assert!(DrawingPlane::new(2, 2).unwrap().add_plane(&DrawingPlane::new(2, 3).unwrap(), None).is_err());
}

#[test]
fn lerp_uses_one_minus_t_once_and_the_mask_per_pixel() {
    let (t, s) = (0.2f32, 1.0f32 - 0.2);
    let mut plane = row(&[0.0, 1.0, 0.3]);
    plane.lerp(&row(&[1.0, 0.0, 0.9]), t).unwrap();
    assert_eq!(plane.data(), &[(0.0 * s) + (1.0 * t), (1.0 * s) + (0.0 * t), (0.3 * s) + (0.9 * t)]);
    let mut plane = row(&[0.0, 0.0, 0.0]);
    plane.lerp_mask(&row(&[1.0, 1.0, 1.0]), &row(&[0.0, 0.5, 1.0])).unwrap();
    assert_eq!(plane.data(), &[0.0, 0.5, 1.0]);
}

#[test]
fn min_max_and_clamp_limit_values_and_clamp_sends_nan_to_min() {
    let mut a = row(&[-1.0, 0.5, 2.0]);
    a.min(1.0);
    assert_eq!(a.data(), &[-1.0, 0.5, 1.0]);
    let mut b = row(&[-1.0, 0.5, 2.0]);
    b.max(0.0);
    assert_eq!(b.data(), &[0.0, 0.5, 2.0]);
    let mut c = row(&[-1.0, 0.5, 2.0, f32::NAN]);
    c.clamp(0.0, 1.0).unwrap();
    assert_eq!(c.data(), &[0.0, 0.5, 1.0, 0.0]);
    let mut d = row(&[1.0, 5.0]);
    d.min_plane(&row(&[2.0, 3.0])).unwrap();
    assert_eq!(d.data(), &[1.0, 3.0]);
    assert!(row(&[1.0]).clamp(1.0, 0.0).is_err());
}

#[test]
fn tone_curve_gamma_and_apply_follow_the_formulas() {
    let mut plane = row(&[0.0, 0.5, 1.0, 4.0]);
    plane.apply_tone_curve(0.15, 1.08);
    assert_eq!(plane.data(), &[0.0, 0.5 / (1.0 + (0.15 * 0.5)) * 1.08, 1.0 / (1.0 + (0.15 * 1.0)) * 1.08, 4.0 / (1.0 + (0.15 * 4.0)) * 1.08]);
    let mut plane = row(&[0.25, 0.0, -0.5, 4.0]);
    plane.apply_gamma(0.5).unwrap();
    assert_eq!(plane.data(), &[0.25f32.powf(0.5), 0.0, -0.5, 4.0f32.powf(0.5)]);
    assert!(row(&[1.0]).apply_gamma(0.0).is_err());
    let mut plane = row(&[1.0, 2.0]);
    plane.apply(|v| (v * v) + 1.0);
    assert_eq!(plane.data(), &[2.0, 5.0]);
}

#[test]
fn create_vignette_follows_the_documented_formula_and_apply_vignette_equals_it() {
    let (cx, cy, rx, ry, strength, minimum) = (4.0f64, 2.0, 3.0, 2.0, 0.38f32 as f64, 0.4f32 as f64);
    let vignette = DrawingPlane::create_vignette(9, 5, cx, cy, rx, ry, strength, Some(minimum)).unwrap();
    for y in 0..5u32 {
        for x in 0..9u32 {
            let (dx, dy) = ((x as f64 - cx) / rx, (y as f64 - cy) / ry);
            let expected = (1.0 - (strength * ((dx * dx) + (dy * dy)))).clamp(minimum, 1.0);
            assert_eq!(vignette.get(x, y).unwrap(), expected as f32);
        }
    }
    assert_eq!(vignette.get(4, 2).unwrap(), 1.0);
    assert_eq!(vignette.get(0, 0).unwrap(), 0.4);
    let mut plane = DrawingPlane::new(140, 130).unwrap();
    plane.fill_with(|x, _| 0.3 + (x as f32 * 0.01));
    let mut expected = plane.clone();
    expected.multiply_plane(&DrawingPlane::create_vignette(140, 130, 70.0, 60.0, 90.0, 80.0, 0.5, Some(0.2)).unwrap()).unwrap();
    plane.apply_vignette(70.0, 60.0, 90.0, 80.0, 0.5, Some(0.2)).unwrap();
    assert_eq!(plane, expected);
}

#[test]
fn the_gaussian_kernel_radius_is_four_sigma_rounded_and_weights_are_normalised_in_double() {
    for (sigma, radius) in [(1.0f32, 4), (3.0, 12), (0.1, 0), (0.625, 3)] {
        let (kernel, r) = gaussian_kernel(sigma);
        assert_eq!((r, kernel.len()), (radius, (2 * radius + 1) as usize));
    }
    let (kernel, radius) = gaussian_kernel(0.35);
    assert_eq!(radius, 1);
    assert_eq!(kernel, vec![0.016328630968928337f32, 0.967342734336853, 0.016328630968928337]);
    assert_ne!(kernel[0], 0.016328632831573486f32);
    let sigma = 1.5f64;
    let sum: f64 = (-6..=6).map(|i| (-0.5 * i as f64 * i as f64 / (sigma * sigma)).exp()).sum();
    let (kernel, radius) = gaussian_kernel(1.5);
    assert_eq!(radius, 6);
    for i in -6i32..=6 {
        assert_eq!(kernel[(i + 6) as usize], ((-0.5 * i as f64 * i as f64 / (sigma * sigma)).exp() / sum) as f32);
    }
}

#[test]
fn a_blurred_impulse_is_the_kernel_outer_product_and_edges_mirror() {
    let mut plane = DrawingPlane::new(21, 21).unwrap();
    plane.set(10, 10, 1.0).unwrap();
    let (kernel, radius) = gaussian_kernel(1.2);
    plane.gaussian_blur(1.2).unwrap();
    for y in 0..21i32 {
        for x in 0..21i32 {
            let (kx, ky) = (x - 10 + radius, y - 10 + radius);
            let inside = kx >= 0 && (kx as usize) < kernel.len() && ky >= 0 && (ky as usize) < kernel.len();
            let expected = if inside { kernel[ky as usize] * kernel[kx as usize] } else { 0.0 };
            assert_eq!(plane.get(x as u32, y as u32).unwrap(), expected);
        }
    }
    let mut edge = row(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    let (kernel, radius) = gaussian_kernel(1.0);
    let vertical = |value: f32| kernel.iter().fold(0.0f32, |total, w| total + w * value);
    let r = radius as usize;
    edge.gaussian_blur(1.0).unwrap();
    assert_eq!(edge.get(0, 0).unwrap(), vertical((kernel[r - 1] * 1.0) + (kernel[r] * 1.0)));
    assert_eq!(edge.get(2, 0).unwrap(), vertical((kernel[r - 3] * 1.0) + (kernel[r - 2] * 1.0)));
    assert_eq!(edge.get(4, 0).unwrap(), vertical(kernel[0] * 1.0));
    assert_eq!(edge.get(5, 0).unwrap(), 0.0);
}

#[test]
fn blur_keeps_a_flat_plane_flat_ignores_sigma_zero_and_refuses_nan() {
    let mut plane = DrawingPlane::with_value(30, 20, 0.5).unwrap();
    plane.gaussian_blur(4.0).unwrap();
    assert!(plane.data().iter().all(|v| (v - 0.5).abs() < 1e-6));
    let mut plane = row(&[1.0, 2.0, 3.0]);
    plane.gaussian_blur(0.0).unwrap();
    assert_eq!(plane.data(), &[1.0, 2.0, 3.0]);
    let mut two = row(&[1.0, 0.0]);
    two.gaussian_blur(5.0).unwrap();
    assert!((two.data()[0] + two.data()[1] - 1.0).abs() < 1e-5 && two.data()[0] > two.data()[1]);
    assert!(row(&[1.0]).gaussian_blur(f32::NAN).is_err());
}

#[test]
fn resizing_uses_the_documented_grids() {
    let plane = row(&[10.0, 20.0, 30.0, 40.0]);
    assert_eq!(plane.resized(2, 1, Some(DrawingPlaneResampling::Nearest)).unwrap().data(), &[20.0, 40.0]);
    assert_eq!(row(&[1.0, 2.0]).resized(4, 1, Some(DrawingPlaneResampling::Nearest)).unwrap().data(), &[1.0, 1.0, 2.0, 2.0]);
    assert_eq!(row(&[1.0, 2.0, 3.0]).resized(2, 1, Some(DrawingPlaneResampling::Nearest)).unwrap().data(), &[1.0, 3.0]);
    let two = row(&[0.0, 1.0]);
    assert_eq!(two.resized(4, 1, None).unwrap().data(), &[0.0, 0.25, 0.75, 1.0]);
    assert_eq!(DrawingPlaneResampling::default(), DrawingPlaneResampling::Bilinear);
    assert_eq!(two.resized(4, 1, Some(DrawingPlaneResampling::BilinearAlignedCorners)).unwrap().data(), &[0.0, (1.0f64 / 3.0) as f32, (2.0f64 / 3.0) as f32, 1.0]);
    let square = DrawingPlane::from_data(2, 2, vec![0.0, 1.0, 2.0, 3.0]).unwrap();
    assert_eq!(square.resized(3, 3, Some(DrawingPlaneResampling::BilinearAlignedCorners)).unwrap().data(), &[0.0, 0.5, 1.0, 1.0, 1.5, 2.0, 2.0, 2.5, 3.0]);
    assert!(DrawingPlane::with_value(1, 1, 0.7).unwrap().resized(3, 2, None).unwrap().data().iter().all(|v| *v == 0.7));
}

#[test]
fn downsampling_averages_blocks_including_partial_edge_blocks() {
    let plane = DrawingPlane::from_data(3, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]).unwrap();
    let small = plane.downsampled(2).unwrap();
    assert_eq!(small.size(), (2, 2));
    assert_eq!(small.data(), &[(1.0 + 2.0 + 4.0 + 5.0) / 4.0, (3.0 + 6.0) / 2.0, (7.0 + 8.0) / 2.0, 9.0]);
    assert!(row(&[1.0]).downsampled(0).is_err());
}

#[test]
fn bloom_is_downsample_weighted_blurs_gain_then_resize() {
    let mut light = DrawingPlane::new(64, 48).unwrap();
    light.fill_with(|x, y| if x == 20 && y == 30 { 8.0 } else { 0.0 });
    let small = light.downsampled(4).unwrap();
    let mut tight = small.clone();
    tight.gaussian_blur(1.5).unwrap();
    let mut wide = small.clone();
    wide.gaussian_blur(4.0).unwrap();
    let mut expected = DrawingPlane::new(small.width(), small.height()).unwrap();
    expected.add_plane(&tight, Some(0.8)).unwrap().add_plane(&wide, Some(0.9)).unwrap().multiply(0.6);
    let expected = expected.resized(64, 48, Some(DrawingPlaneResampling::BilinearAlignedCorners)).unwrap();
    let bloom = light.create_bloom(4, &[(1.5, 0.8), (4.0, 0.9)], DrawingPlaneBloomOptions { gain: 0.6, output_size: None, upsampling: DrawingPlaneResampling::BilinearAlignedCorners }).unwrap();
    assert_eq!(bloom, expected);
    assert_eq!(light.get(20, 30).unwrap(), 8.0);
    let other = DrawingPlane::with_value(32, 16, 1.0).unwrap().create_bloom(4, &[(2.0, 1.0)], DrawingPlaneBloomOptions { output_size: Some((64, 32)), ..DrawingPlaneBloomOptions::default() }).unwrap();
    assert_eq!(other.size(), (64, 32));
    assert!(row(&[1.0]).create_bloom(1, &[], DrawingPlaneBloomOptions::default()).is_err());
}

#[test]
fn bytes_round_half_up_clamp_and_round_trip_every_value() {
    let plane = row(&[-0.1, 0.0, 0.5, 0.498, 1.0, 1.5, f32::NAN, 1.0 / 255.0]);
    assert_eq!(plane.to_bytes(), vec![0, 0, 128, 127, 255, 255, 0, 1]);
    let bytes: Vec<u8> = (0..=255).collect();
    assert_eq!(DrawingPlane::from_bytes(&bytes, 16, 16).unwrap().to_bytes(), bytes);
    let mut plane = DrawingPlane::with_value(16, 16, 0.5).unwrap();
    let reversed: Vec<u8> = (0..=255u8).rev().collect();
    plane.copy_from_bytes(&reversed).unwrap();
    assert_eq!(plane.to_bytes(), reversed);
    assert!(plane.copy_from_bytes(&[0u8; 255]).is_err());
}

// ---- DrawingPlaneImage -----------------------------------------------------------------------

#[test]
fn an_image_fills_with_byte_over_255_and_its_colour_arithmetic_leaves_alpha_alone() {
    let mut image = DrawingPlaneImage::new(4, 3).unwrap();
    assert!(image.a().data().iter().all(|v| *v == 0.0));
    image.fill_color(Color::from_rgba(255, 128, 0, 64));
    assert_eq!(image.get_pixel(1, 1).unwrap(), [1.0, 128.0 / 255.0, 0.0, 64.0 / 255.0]);
    let mut image = DrawingPlaneImage::new(1, 1).unwrap();
    image.fill(0.5, 0.25, 1.0, Some(0.3));
    let mut other = DrawingPlaneImage::new(1, 1).unwrap();
    other.fill(0.5, 0.5, 0.5, Some(0.9));
    image.add_image(&other, None).unwrap().multiply(2.0).multiply_rgb(1.0, 0.5, 0.25).add(0.1, 0.0, 0.0);
    assert_eq!(image.get_pixel(0, 0).unwrap(), [(((0.5 + 0.5) * 2.0) * 1.0) + 0.1, ((0.25 + 0.5) * 2.0) * 0.5, ((1.0 + 0.5) * 2.0) * 0.25, 0.3]);
    assert!(DrawingPlaneImage::from_planes(DrawingPlane::new(2, 2).unwrap(), DrawingPlane::new(2, 2).unwrap(), DrawingPlane::new(2, 3).unwrap(), DrawingPlane::new(2, 2).unwrap()).is_err());
}

#[test]
fn lerp_takes_no_colour_from_a_transparent_pixel_and_copies_exactly_at_the_ends() {
    let mut a = DrawingPlaneImage::new(1, 1).unwrap();
    a.fill(0.2, 0.4, 0.6, Some(0.0));
    let mut b = DrawingPlaneImage::new(1, 1).unwrap();
    b.fill(1.0, 0.0, 0.0, Some(1.0));
    a.lerp(&b, 0.5).unwrap();
    assert_eq!(a.get_pixel(0, 0).unwrap(), [1.0, 0.0, 0.0, 0.5]);
    let mut c = DrawingPlaneImage::new(1, 1).unwrap();
    c.fill(0.1, 0.2, 0.3, Some(0.4));
    let mut d = DrawingPlaneImage::new(1, 1).unwrap();
    d.fill(0.9, 0.8, 0.7, Some(0.0));
    let mut c0 = c.clone();
    c0.lerp(&d, 0.0).unwrap();
    assert_eq!(c0.get_pixel(0, 0).unwrap(), [0.1, 0.2, 0.3, 0.4]);
    c.lerp(&d, 1.0).unwrap();
    assert_eq!(c.get_pixel(0, 0).unwrap(), [0.9, 0.8, 0.7, 0.0]);
}

#[test]
fn composite_over_follows_porter_duff() {
    let mut destination = DrawingPlaneImage::new(1, 1).unwrap();
    destination.fill(0.2, 0.4, 0.6, Some(1.0));
    let mut source = DrawingPlaneImage::new(1, 1).unwrap();
    source.fill(1.0, 0.5, 0.0, Some(0.25));
    destination.composite_over(&source, None).unwrap();
    assert_eq!(destination.get_pixel(0, 0).unwrap(), [(1.0 * 0.25) + (0.2 * 0.75), (0.5 * 0.25) + (0.4 * 0.75), (0.0 * 0.25) + (0.6 * 0.75), 1.0]);
    let mut empty = DrawingPlaneImage::new(1, 1).unwrap();
    let mut s = DrawingPlaneImage::new(1, 1).unwrap();
    s.fill(0.3, 0.6, 0.9, Some(0.35));
    empty.composite_over(&s, None).unwrap();
    assert_eq!(empty.get_pixel(0, 0).unwrap(), [0.3, 0.6, 0.9, 0.35]);
    let mut d = DrawingPlaneImage::new(1, 1).unwrap();
    d.fill(0.0, 0.0, 1.0, Some(0.5));
    let mut s = DrawingPlaneImage::new(1, 1).unwrap();
    s.fill(1.0, 0.0, 0.0, Some(0.5));
    d.composite_over(&s, None).unwrap();
    let (rest, ao) = (0.5f32 * (1.0 - 0.5), 0.75f32);
    assert_eq!(d.get_pixel(0, 0).unwrap(), [(1.0 * 0.5) / ao, 0.0, (1.0 * rest) / ao, ao]);
}

#[test]
fn composite_add_and_blend_color_follow_their_formulas() {
    let mut d = DrawingPlaneImage::new(1, 1).unwrap();
    d.fill(0.5, 0.5, 0.5, Some(1.0));
    let mut s = DrawingPlaneImage::new(1, 1).unwrap();
    s.fill(1.0, 0.5, 0.0, Some(0.5));
    d.composite_add(&s, None).unwrap();
    assert_eq!(d.get_pixel(0, 0).unwrap(), [0.5 + 0.5, 0.5 + 0.25, 0.5, 1.0]);
    let mut image = DrawingPlaneImage::new(2, 1).unwrap();
    image.blend_color(Color::from_rgb(255, 0, 0), &row(&[0.25, 1.0])).unwrap();
    assert_eq!(image.get_pixel(0, 0).unwrap(), [1.0, 0.0, 0.0, 0.25]);
    assert_eq!(image.get_pixel(1, 0).unwrap(), [1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn blur_over_transparency_never_darkens_and_rgba_round_trips_exactly() {
    let mut image = DrawingPlaneImage::new(16, 16).unwrap();
    image.fill_area(Rectangle::new(0, 0, 8, 16), Color::from_rgb(255, 0, 0));
    image.gaussian_blur(1.5).unwrap();
    for i in 0..256 {
        if image.a().data()[i] > 0.0 {
            assert!((image.r().data()[i] - 1.0).abs() < 1e-5);
            assert_eq!(image.g().data()[i], 0.0);
        } else {
            assert_eq!(image.r().data()[i], 0.0);
        }
    }
    let mut pixels = DrawingPlaneImage::new(2, 1).unwrap();
    pixels.set_pixel(0, 0, [1.0, 0.5, 0.0, 0.498]).unwrap();
    pixels.set_pixel(1, 0, [0.2, 0.4, 0.6, 0.0]).unwrap();
    assert_eq!(pixels.to_rgba(), vec![255, 128, 0, 127, 51, 102, 153, 0]);
    let rgba: Vec<u8> = (0..=255u8).collect();
    assert_eq!(DrawingPlaneImage::from_rgba(&rgba, 8, 8).unwrap().to_rgba(), rgba);
    assert!(DrawingPlaneImage::from_rgba(&[0u8; 15], 2, 2).is_err());
}

// ---- DrawingTransition -------------------------------------------------------------------------

#[test]
fn every_transition_gives_the_exact_frames_at_zero_and_one() {
    let (a, b) = (ramp(12, 20), solid(12, 5, 6, 7, 128));
    let kinds = [DrawingTransition::dissolve(), DrawingTransition::wipe(DrawingTransitionDirection::Up, Some(3.0)).unwrap(), DrawingTransition::slide(DrawingTransitionDirection::Left), DrawingTransition::push(DrawingTransitionDirection::Down), DrawingTransition::fade_through_color(Color::WHITE)];
    for t in kinds {
        assert_eq!(apply(t, &a, &b, 4, 3, 0.0), a);
        assert_eq!(apply(t, &a, &b, 4, 3, 1.0), b);
        assert_eq!(apply(t, &a, &b, 4, 3, -2.0), a);
        assert_eq!(apply(t, &a, &b, 4, 3, 7.0), b);
    }
}

#[test]
fn dissolve_mixes_in_premultiplied_space() {
    let (a, b) = (ramp(4, 60), solid(4, 0, 0, 0, 255));
    let (u, v) = (0.3f32, 1.0f32 - 0.3);
    let result = apply(DrawingTransition::dissolve(), &a, &b, 4, 1, u);
    for i in 0..16 {
        assert_eq!(result[i], ((a[i] as f32 * v) + (b[i] as f32 * u) + 0.5) as u8);
    }
    assert_eq!(apply(DrawingTransition::dissolve(), &[0u8; 4], &solid(1, 255, 0, 0, 255), 1, 1, 0.5), vec![255, 0, 0, 128]);
    let (t, s) = (0.25f32, 0.75f32);
    let (wa, wb) = (64.0 * s, 192.0 * t);
    let alpha = wa + wb;
    let result = apply(DrawingTransition::dissolve(), &solid(1, 200, 100, 0, 64), &solid(1, 0, 50, 250, 192), 1, 1, t);
    assert_eq!(result, vec![((((200.0 * wa) + (0.0 * wb)) / alpha) + 0.5) as u8, ((((100.0 * wa) + (50.0 * wb)) / alpha) + 0.5) as u8, ((((0.0 * wa) + (250.0 * wb)) / alpha) + 0.5) as u8, (alpha + 0.5) as u8]);
    let mut onto = ramp(4, 60);
    let expected = apply(DrawingTransition::dissolve(), &onto, &b, 4, 1, 0.5);
    DrawingTransition::dissolve().apply_bytes_onto(&mut onto, &b, 4, 1, 0.5).unwrap();
    assert_eq!(onto, expected);
}

#[test]
fn wipes_reveal_from_the_start_side_with_the_documented_soft_ramp() {
    let (a, b) = (solid(4, 0, 0, 0, 255), solid(4, 1, 0, 0, 255));
    assert_eq!(reds(&apply(DrawingTransition::wipe(DrawingTransitionDirection::Right, None).unwrap(), &a, &b, 4, 1, 0.5)), vec![1, 1, 0, 0]);
    assert_eq!(reds(&apply(DrawingTransition::wipe(DrawingTransitionDirection::Left, None).unwrap(), &a, &b, 4, 1, 0.5)), vec![0, 0, 1, 1]);
    assert_eq!(reds(&apply(DrawingTransition::wipe(DrawingTransitionDirection::Down, None).unwrap(), &a, &b, 1, 4, 0.25)), vec![1, 0, 0, 0]);
    assert_eq!(reds(&apply(DrawingTransition::wipe(DrawingTransitionDirection::Up, None).unwrap(), &a, &b, 1, 4, 0.25)), vec![0, 0, 0, 1]);
    let (a, b) = (solid(10, 0, 0, 0, 255), solid(10, 200, 0, 0, 255));
    let result = reds(&apply(DrawingTransition::wipe(DrawingTransitionDirection::Right, Some(4.0)).unwrap(), &a, &b, 10, 1, 0.5));
    let expected: Vec<u8> = (0..10)
        .map(|x| {
            let w = ((7.0f32 - (x as f32 + 0.5)) / 4.0).clamp(0.0, 1.0);
            ((0.0 * (1.0 - w)) + (200.0 * w) + 0.5) as u8
        })
        .collect();
    assert_eq!(result, expected);
    assert_eq!((expected[0], expected[7]), (200, 0));
    assert!(DrawingTransition::wipe(DrawingTransitionDirection::Up, Some(-1.0)).is_err());
}

#[test]
fn slides_and_pushes_move_whole_pixels() {
    let a = ramp(4, 10);
    let mut b = ramp(4, 1);
    (0..4).for_each(|i| b[i * 4] += 100);
    assert_eq!(reds(&apply(DrawingTransition::slide(DrawingTransitionDirection::Right), &a, &b, 4, 1, 0.5)), vec![102, 103, 20, 30]);
    assert_eq!(reds(&apply(DrawingTransition::push(DrawingTransitionDirection::Right), &a, &b, 4, 1, 0.5)), vec![102, 103, 0, 10]);
    assert_eq!(reds(&apply(DrawingTransition::slide(DrawingTransitionDirection::Left), &a, &b, 4, 1, 0.25)), vec![0, 10, 20, 100]);
    assert_eq!(reds(&apply(DrawingTransition::push(DrawingTransitionDirection::Left), &a, &b, 4, 1, 0.25)), vec![10, 20, 30, 100]);
    assert_eq!(reds(&apply(DrawingTransition::push(DrawingTransitionDirection::Down), &a, &b, 1, 4, 0.25)), vec![103, 0, 10, 20]);
    assert_eq!(reds(&apply(DrawingTransition::slide(DrawingTransitionDirection::Up), &a, &b, 1, 4, 0.5)), vec![0, 10, 100, 101]);
    let result = apply(DrawingTransition::slide(DrawingTransitionDirection::Right), &solid(4, 10, 20, 30, 0), &solid(4, 0, 0, 0, 0), 4, 1, 0.5);
    assert_eq!(result, vec![0, 0, 0, 0, 0, 0, 0, 0, 10, 20, 30, 0, 10, 20, 30, 0]);
    let mut onto = a.clone();
    assert!(DrawingTransition::slide(DrawingTransitionDirection::Left).apply_bytes_onto(&mut onto, &b, 4, 1, 0.5).is_err());
}

#[test]
fn fade_through_color_is_the_colour_at_the_midpoint() {
    let (a, b) = (solid(2, 200, 200, 200, 255), solid(2, 0, 100, 0, 255));
    let fade = DrawingTransition::fade_through_color(Color::from_rgb(40, 0, 0));
    assert_eq!(apply(fade, &a, &b, 2, 1, 0.5), vec![40, 0, 0, 255, 40, 0, 0, 255]);
    assert_eq!(apply(fade, &a, &b, 2, 1, 0.25)[0], ((200.0f32 * 0.5) + (40.0 * 0.5) + 0.5) as u8);
    assert_eq!(apply(fade, &a, &b, 2, 1, 0.75)[1], ((0.0f32 * 0.5) + (100.0 * 0.5) + 0.5) as u8);
    let (red, blue) = (solid(1, 255, 0, 0, 255), solid(1, 0, 0, 255, 255));
    let clear = DrawingTransition::fade_through_color(Color::TRANSPARENT);
    assert_eq!(apply(clear, &red, &blue, 1, 1, 0.5), vec![0, 0, 0, 0]);
    assert_eq!(apply(clear, &red, &blue, 1, 1, 0.25), vec![255, 0, 0, 128]);
    let wipe = DrawingTransition::wipe(DrawingTransitionDirection::Up, Some(12.0)).unwrap();
    assert_eq!((wipe.kind(), wipe.direction(), wipe.softness()), (DrawingTransitionKind::Wipe, DrawingTransitionDirection::Up, 12.0));
    assert_eq!(DrawingTransition::fade_through_color(Color::WHITE).color(), Color::WHITE);
}

#[test]
fn float_transitions_write_onto_an_input_for_per_pixel_kinds_and_refuse_it_for_moves() {
    let mut a = DrawingPlaneImage::new(3, 2).unwrap();
    a.fill_with(|x, y| [x as f32 * 0.3, y as f32 * 0.5, 0.2, 0.25 + x as f32 * 0.25]);
    let mut b = DrawingPlaneImage::new(3, 2).unwrap();
    b.fill(1.0, 0.0, 0.5, Some(1.0));
    for t in [DrawingTransition::dissolve(), DrawingTransition::wipe(DrawingTransitionDirection::Left, Some(1.5)).unwrap(), DrawingTransition::fade_through_color(Color::from_rgba(9, 99, 199, 77))] {
        for f in [0.3f32, 0.7] {
            let expected = t.apply(&a, &b, f).unwrap();
            let mut onto = a.clone();
            t.apply_onto(&mut onto, &b, f).unwrap();
            assert_eq!(onto, expected);
        }
    }
    let mut onto = a.clone();
    assert!(DrawingTransition::push(DrawingTransitionDirection::Up).apply_onto(&mut onto, &b, 0.5).is_err());
    assert!(DrawingTransition::dissolve().apply(&a, &b, f32::NAN).is_err());
}

// ---- Accumulator and motion blur -----------------------------------------------------------------

#[test]
fn byte_frames_average_in_exact_integers_and_premultiplied_space() {
    let mut acc = DrawingFrameAccumulator::new(1, 1).unwrap();
    for frame in [[10u8, 0, 255, 255], [11, 1, 0, 255], [12, 1, 0, 255]] {
        acc.add_rgba(&frame).unwrap();
    }
    assert_eq!((acc.get_average().unwrap(), acc.count()), (vec![11, 1, 85, 255], 3));
    let mut acc = DrawingFrameAccumulator::new(1, 1).unwrap();
    acc.add_rgba(&[0, 0, 0, 255]).unwrap();
    acc.add_rgba(&[1, 3, 0, 255]).unwrap();
    assert_eq!(acc.get_average().unwrap(), vec![1, 2, 0, 255]);
    let mut acc = DrawingFrameAccumulator::new(1, 1).unwrap();
    acc.add_rgba(&[200, 100, 0, 255]).unwrap();
    acc.add_rgba(&[0, 0, 0, 0]).unwrap();
    assert_eq!(acc.get_average().unwrap(), vec![200, 100, 0, 128]);
    let mut acc = DrawingFrameAccumulator::new(3, 1).unwrap();
    acc.add_rgba(&[1, 2, 3, 4, 250, 128, 7, 255, 50, 60, 70, 0]).unwrap();
    assert_eq!(acc.get_average().unwrap(), vec![1, 2, 3, 4, 250, 128, 7, 255, 0, 0, 0, 0]);
    assert!(acc.add_image(&DrawingPlaneImage::new(3, 1).unwrap()).is_err());
    assert!(DrawingFrameAccumulator::new(1, 1).unwrap().get_average().is_err());
}

#[test]
fn float_images_average_in_premultiplied_space() {
    let mut acc = DrawingFrameAccumulator::new(1, 1).unwrap();
    let mut first = DrawingPlaneImage::new(1, 1).unwrap();
    first.fill(1.0, 0.5, 0.0, Some(1.0));
    let mut third = DrawingPlaneImage::new(1, 1).unwrap();
    third.fill(0.0, 0.5, 1.0, Some(0.5));
    acc.add_image(&first).unwrap();
    acc.add_image(&DrawingPlaneImage::new(1, 1).unwrap()).unwrap();
    acc.add_image(&third).unwrap();
    assert_eq!(acc.get_average_image().unwrap().get_pixel(0, 0).unwrap(), [(1.0f64 / 1.5) as f32, (0.75f64 / 1.5) as f32, (0.5f64 / 1.5) as f32, 0.5]);
}

#[test]
fn motion_blur_averages_frames_and_centres_its_samples_in_the_open_shutter() {
    let frames: [&[u8]; 2] = [&[0, 0, 0, 255, 9, 9, 9, 0], &[255, 255, 255, 255, 9, 9, 9, 255]];
    assert_eq!(DrawingMotionBlur::average_bytes(&frames, 2, 1).unwrap(), vec![128, 128, 128, 255, 9, 9, 9, 128]);
    let mut red = DrawingPlaneImage::new(2, 2).unwrap();
    red.fill(1.0, 0.0, 0.0, None);
    let mut blue = DrawingPlaneImage::new(2, 2).unwrap();
    blue.fill(0.0, 0.0, 1.0, None);
    assert_eq!(DrawingMotionBlur::average(&[red, blue]).unwrap().get_pixel(1, 1).unwrap(), [0.5, 0.0, 0.5, 1.0]);
    assert!(DrawingMotionBlur::average(&[]).is_err());
    assert!(DrawingMotionBlur::average(&[DrawingPlaneImage::new(2, 2).unwrap(), DrawingPlaneImage::new(3, 2).unwrap()]).is_err());
    assert!(DrawingMotionBlur::average_bytes(&[&[0u8; 3]], 1, 1).is_err());
    let times = DrawingMotionBlur::get_sample_times(2.0, 0.04, 4, Some(0.5)).unwrap();
    assert_eq!(times, vec![2.0 + (0.5 / 4.0 * 0.02), 2.0 + (1.5 / 4.0 * 0.02), 2.0 + (2.5 / 4.0 * 0.02), 2.0 + (3.5 / 4.0 * 0.02)]);
}

// ---- Noise -------------------------------------------------------------------------------------------

#[test]
fn noise_follows_the_lattice_rules() {
    for y in -3..=3 {
        for x in -3..=3 {
            assert_eq!(DrawingNoise::gradient_3d(x as f64, y as f64, 2.0, 11), 0.0);
        }
    }
    let h = crate::planes::drawing_random::mix(crate::planes::drawing_random::mix(crate::planes::drawing_random::mix(5).wrapping_add(3)).wrapping_add(-2i64 as u64));
    assert_eq!(DrawingNoise::value(3.0, -2.0, 5), ((h >> 11) as f64 * (1.0 / 9007199254740992.0) * 2.0) - 1.0);
    let one = DrawingNoiseFractalOptions { octaves: 1, ..DrawingNoiseFractalOptions::default() };
    assert_eq!(DrawingNoise::fractal(DrawingNoiseKind::Gradient, 3.3, 4.4, 8, one).unwrap(), DrawingNoise::gradient(3.3, 4.4, 8));
    assert_eq!(DrawingNoise::fractal_3d(DrawingNoiseKind::Value, 3.3, 4.4, 1.1, 8, one).unwrap(), DrawingNoise::value_3d(3.3, 4.4, 1.1, 8));
    let (x, y) = (0.37, 1.91);
    let two = DrawingNoiseFractalOptions { octaves: 2, ..DrawingNoiseFractalOptions::default() };
    assert_eq!(DrawingNoise::fractal(DrawingNoiseKind::Value, x, y, 20, two).unwrap(), (DrawingNoise::value(x, y, 20) + (0.5 * DrawingNoise::value(x * 2.0, y * 2.0, 21))) / 1.5);
    for octaves in [0, 31] {
        assert!(DrawingNoise::fractal(DrawingNoiseKind::Value, 0.0, 0.0, 1, DrawingNoiseFractalOptions { octaves, ..DrawingNoiseFractalOptions::default() }).is_err());
    }
}

#[test]
fn noise_fills_sample_pixel_centres_and_move_smoothly_with_time() {
    let mut plane = DrawingPlane::new(7, 5).unwrap();
    DrawingNoise::fill(&mut plane, DrawingNoiseKind::Gradient, 42, 0.1, DrawingNoiseFillOptions { z: Some(2.5), octaves: 3, offset_x: 4.0, offset_y: -1.0, ..DrawingNoiseFillOptions::default() }).unwrap();
    let three = DrawingNoiseFractalOptions { octaves: 3, ..DrawingNoiseFractalOptions::default() };
    for y in 0..5u32 {
        for x in 0..7u32 {
            let expected = DrawingNoise::fractal_3d(DrawingNoiseKind::Gradient, (x as f64 + 0.5 + 4.0) * 0.1, (y as f64 + 0.5 - 1.0) * 0.1, 2.5, 42, three).unwrap();
            assert_eq!(plane.get(x, y).unwrap(), expected as f32);
        }
    }
    let mut plane = DrawingPlane::new(3, 3).unwrap();
    DrawingNoise::fill(&mut plane, DrawingNoiseKind::Value, 3, 0.25, DrawingNoiseFillOptions::default()).unwrap();
    assert_eq!(plane.get(1, 2).unwrap(), DrawingNoise::value(1.5 * 0.25, 2.5 * 0.25, 3) as f32);
    let fill = |z: f64| {
        let mut p = DrawingPlane::new(64, 64).unwrap();
        DrawingNoise::fill(&mut p, DrawingNoiseKind::Gradient, 1, 0.05, DrawingNoiseFillOptions { z: Some(z), ..DrawingNoiseFillOptions::default() }).unwrap();
        p
    };
    let (a, b, c) = (fill(0.0), fill(0.01), fill(0.5));
    let largest = |p: &DrawingPlane, q: &DrawingPlane| p.data().iter().zip(q.data()).fold(0.0f32, |m, (u, v)| m.max((u - v).abs()));
    assert!(largest(&a, &b) < 0.05 && largest(&a, &c) > 0.2);
}

// ---- Particles ---------------------------------------------------------------------------------------

fn one_emitter(seed: i64, set: impl FnOnce(&mut DrawingParticleEmitter)) -> DrawingParticleSystem {
    let mut system = DrawingParticleSystem::new(seed);
    set(system.add_emitter(DrawingParticleEmitter::new()));
    system
}

#[test]
fn particles_are_born_by_burst_and_rate_and_die_after_their_lifetime() {
    let burst = one_emitter(1, |e| {
        e.set_burst(4);
        e.set_start_time(1.0).unwrap();
        e.set_lifetime(1.0).unwrap();
    });
    assert!(burst.get_particles(0.99).unwrap().is_empty());
    let during = burst.get_particles(1.5).unwrap();
    assert_eq!(during.iter().map(|p| p.index()).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
    assert!(during.iter().all(|p| p.birth_time() == 1.0 && p.age() == 0.5));
    assert!(burst.get_particles(2.0).unwrap().is_empty());
    let rate = one_emitter(1, |e| {
        e.set_rate(4.0).unwrap();
        e.set_lifetime(100.0).unwrap();
    });
    let particles = rate.get_particles(1.0).unwrap();
    assert_eq!(particles.iter().map(|p| p.birth_time()).collect::<Vec<_>>(), vec![0.0, 0.25, 0.5, 0.75, 1.0]);
    assert_eq!(particles.iter().map(|p| p.index()).collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);
    let ending = one_emitter(1, |e| {
        e.set_rate(2.0).unwrap();
        e.set_lifetime(1.0).unwrap();
        e.set_end_time(3.0).unwrap();
    });
    assert_eq!(ending.get_particles(2.25).unwrap().iter().map(|p| p.birth_time()).collect::<Vec<_>>(), vec![1.5, 2.0]);
    assert!(ending.get_particles(3.75).unwrap().is_empty());
    let short = one_emitter(9, |e| {
        e.set_rate(10.0).unwrap();
        e.set_end_time(0.3).unwrap();
        e.set_lifetime(0.35).unwrap();
    });
    let early = short.get_particles(0.29).unwrap();
    assert_eq!(early.iter().map(|p| p.index()).collect::<Vec<_>>(), vec![0, 1, 2]);
    assert_eq!(early[0].lifetime(), 0.35);
    assert_eq!(short.get_particles(0.5).unwrap().iter().map(|p| p.index()).collect::<Vec<_>>(), vec![2]);
}

#[test]
fn particle_motion_follows_the_documented_ballistic_formula() {
    let system = one_emitter(3, |e| {
        e.set_x(10.0);
        e.set_y(20.0);
        e.set_burst(1);
        e.set_lifetime(10.0).unwrap();
        e.set_speed(50.0);
        e.set_gravity_x(2.0);
        e.set_gravity_y(9.75);
        e.set_drift_x(1.0);
        e.set_drift_y(-3.0);
        e.set_start_size(10.0);
        e.set_end_size(20.0);
        e.set_end_opacity(0.5);
        e.set_start_color(Color::from_rgba(0, 100, 200, 255));
        e.set_end_color(Color::from_rgba(100, 100, 0, 55));
    });
    let age = 2.5f64;
    let p = system.get_particles(age).unwrap()[0];
    let (vx, vy) = ((50.0 * 0.0f64.cos()) + 1.0, (50.0 * 0.0f64.sin()) - 3.0);
    assert_eq!(p.x(), (10.0 + (0.0 * 0.0f64.cos()) + (vx * age) + (0.5 * 2.0 * age * age)) as f32);
    assert_eq!(p.y(), (20.0 + (vy * age) + (0.5 * 9.75 * age * age)) as f32);
    assert_eq!((p.life_fraction(), p.size(), p.opacity(), p.color()), (0.25, 12.5, 0.875, Color::from_rgba(25, 100, 150, 205)));
    let down = one_emitter(5, |e| {
        e.set_x(10.0);
        e.set_y(20.0);
        e.set_burst(1);
        e.set_lifetime(10.0).unwrap();
        e.set_direction(90.0);
        e.set_speed(40.0);
    });
    let p = down.get_particles(2.0).unwrap()[0];
    assert!((p.x() - 10.0).abs() < 1e-4 && (p.y() - 100.0).abs() < 1e-4);
}

#[test]
fn particles_are_a_pure_function_of_time_and_seed() {
    let fountain = |seed: i64| {
        one_emitter(seed, |e| {
            e.set_x(100.0);
            e.set_y(200.0);
            e.set_rate(30.0).unwrap();
            e.set_burst(5);
            e.set_lifetime(2.0).unwrap();
            e.set_lifetime_variation(0.5).unwrap();
            e.set_direction(-90.0);
            e.set_spread(46.0);
            e.set_speed(120.0);
            e.set_speed_variation(30.0).unwrap();
            e.set_gravity_y(98.0);
            e.set_spawn_radius(3.0).unwrap();
            e.set_start_size(6.0);
            e.set_end_size(1.0);
            e.set_size_variation(0.2).unwrap();
        })
    };
    let first = fountain(42).get_particles(1.3).unwrap();
    assert!(first.len() > 20);
    assert_eq!(fountain(42).get_particles(1.3).unwrap(), first);
    assert_ne!(fountain(43).get_particles(1.3).unwrap().iter().map(|p| p.x()).collect::<Vec<_>>(), first.iter().map(|p| p.x()).collect::<Vec<_>>());
    let system = fountain(42);
    let _ = system.get_particles(0.4).unwrap();
    assert_eq!(system.get_particles(1.3).unwrap(), first);
    let mut two = fountain(42);
    two.add_emitter(DrawingParticleEmitter::new()).set_burst(3);
    assert_eq!(two.get_particles(1.3).unwrap().into_iter().filter(|p| p.emitter_index() == 0).collect::<Vec<_>>(), first);
    let all = fountain(42).get_particles(1.3).unwrap();
    assert!(all.iter().all(|p| p.lifetime() >= 1.5 && p.lifetime() <= 2.5 && p.age() >= 0.0 && p.age() < p.lifetime()));
    assert!(fountain(1).get_particles(f64::NAN).is_err());
    let mut e = DrawingParticleEmitter::new();
    assert!(e.set_rate(-1.0).is_err() && e.set_lifetime(0.0).is_err() && e.set_start_time(f64::INFINITY).is_err() && e.set_end_time(f64::NAN).is_err());
}
