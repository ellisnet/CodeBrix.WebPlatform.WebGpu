//! Host tests of the effects' arithmetic against C#'s formulas (EffectProcessor.Measure and
//! PlaneKernels.GaussianKernel) and of the masks' and effects' values.

use kurbo::Affine;

use crate::color::Color;
use crate::effects::effect_sizes::{gaussian_kernel, level_for, measure, reach, EffectParams, MAX_KERNEL_RADIUS};
use crate::effects::{DrawingEffects, DrawingMask, DrawingMaskKind, ShadowDirection};
use crate::geometry::PointF;
use crate::paths::PathGeometry;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn the_kernel_is_the_planes_gaussian() {
    //Arrange
    let sigma = 1.5f32;
    //Act
    let (kernel, radius) = gaussian_kernel(sigma);
    //Assert
    // C#: radius (int)(4 * 1.5 + 0.5) = 6; weight i = exp(-0.5 i^2 / 2.25) / sum.
    assert_eq!(radius, 6);
    assert_eq!(kernel.len(), 13);
    let sum: f64 = (-6..=6).map(|i: i32| (-0.5 * (i * i) as f64 / 2.25).exp()).sum();
    assert!(close(kernel[6], (1.0 / sum) as f32));
    assert!(close(kernel[0], ((-0.5 * 36.0 / 2.25f64).exp() / sum) as f32));
    assert!(close(kernel.iter().sum::<f32>(), 1.0));
    assert_eq!(kernel[2], kernel[10]);
    assert_eq!(reach(1.5), 7);
    assert_eq!(reach(0.0), 0);
}

#[test]
fn a_wide_blur_is_computed_at_a_coarser_level() {
    //Arrange / Act / Assert
    // At most MAX_KERNEL_RADIUS (8) taps a side: radius (int)(4 sigma / level + 0.5).
    assert_eq!(level_for(1.0), 1);
    assert_eq!(level_for(2.0), 1, "radius (int)(8.5) = 8 still fits");
    assert_eq!(level_for(2.2), 2);
    assert_eq!(level_for(5.0), 4);
    assert_eq!(level_for(40.0), 16, "the coarsest level");
    for sigma in [1.0f32, 3.0, 7.0, 13.0, 30.0] {
        let level = level_for(sigma) as f32;
        assert!((4.0 * sigma / level + 0.5) as i32 <= MAX_KERNEL_RADIUS);
    }
}

#[test]
fn radii_scale_with_the_square_root_of_the_area_and_fixed_shadows_keep_their_direction() {
    //Arrange
    let effects = DrawingEffects::drop_shadow(PointF::new(10.0, 0.0), 4.0, Color::BLACK).unwrap();
    let turned = Affine::rotate(std::f64::consts::FRAC_PI_2).then_scale(2.0);
    //Act
    let fixed = measure(&effects, turned, 0.0);
    let rotating = measure(&effects.with_shadow_direction(ShadowDirection::RotatesWithElement), turned, 0.0);
    let stretched = measure(&effects, Affine::scale_non_uniform(4.0, 1.0), 0.0);
    let rolled = measure(&effects, Affine::IDENTITY, std::f64::consts::FRAC_PI_2);
    //Assert
    // C#: a uniform scale s times a turn gives s v for a fixed shadow, s R v for one that turns.
    assert!(close(fixed.radius_scale, 2.0));
    assert!(close(fixed.offset_x, 20.0) && close(fixed.offset_y, 0.0));
    assert!(close(rotating.offset_x, 0.0) && close(rotating.offset_y, 20.0));
    // A horizontal stretch of 4 multiplies a horizontal offset by 4; the radius by sqrt(4).
    assert!(close(stretched.offset_x, 40.0) && close(stretched.radius_scale, 2.0));
    // The camera's roll turns a fixed shadow.
    assert!(close(rolled.offset_x, 0.0) && close(rolled.offset_y, 10.0));
}

#[test]
fn the_device_parameters_halve_the_radii_and_reach_as_csharp_grows_the_region() {
    //Arrange
    let effects = DrawingEffects::glow(10.0, Color::from_rgb(255, 0, 0), 1.5).unwrap().with_shadow(PointF::new(3.0, -7.0), 4.0, Color::from_rgba(0, 0, 0, 128)).unwrap().with_blur_radius(2.0).unwrap();
    //Act
    let p = EffectParams::new(&effects, Affine::scale(2.0), 0.0).unwrap();
    //Assert
    assert!(close(p.blur_sigma, 2.0) && close(p.glow_sigma, 10.0) && close(p.shadow_sigma, 4.0));
    assert_eq!(p.shadow_offset, [6.0, -14.0]);
    // reach(blur) + max(reach(glow), reach(shadow) + ceil(max |offset|)) = 9 + max(41, 17 + 14).
    assert_eq!(p.reach(), 50.0);
    assert!(close(p.glow_color[0], 1.0) && close(p.shadow_color[3], 128.0 / 255.0));
    assert!(EffectParams::new(&DrawingEffects::NONE, Affine::IDENTITY, 0.0).is_none());
}

#[test]
fn effects_check_their_values_and_masks_say_what_they_keep() {
    //Arrange
    let star = PathGeometry::star(PointF::new(0.0, 0.0), 10.0, 4.0, 5, 0.0).unwrap();
    //Act
    let inside = DrawingMask::inside(star.clone());
    let outside = DrawingMask::outside(star);
    let matte = DrawingMask::from_layer("Matte", true);
    //Assert
    assert!(DrawingEffects::blur(-1.0).is_err());
    assert!(DrawingEffects::glow(4.0, Color::WHITE, f32::NAN).is_err());
    assert!(DrawingEffects::NONE.is_empty());
    assert!(!DrawingEffects::glow(4.0, Color::TRANSPARENT, 1.0).unwrap().has_glow());
    assert!(DrawingEffects::drop_shadow(PointF::new(1.0, 1.0), 0.0, Color::BLACK).unwrap().has_shadow());
    assert_eq!((inside.kind(), inside.is_inverted()), (DrawingMaskKind::Inside, false));
    assert_eq!((outside.kind(), outside.is_inverted()), (DrawingMaskKind::Outside, true));
    assert_eq!((matte.kind(), matte.layer(), matte.is_inverted()), (DrawingMaskKind::Layer, Some("Matte"), true));
}
