//! Host tests of strokes coloured from a live picture (painterly P3): the pickup's checks, the
//! tap positions along straight, bent and anchored strokes, the source's texels as linear colour,
//! the CPU mirrors of the stroke shader's Oklab, palette, broken colour and own-colour mix against
//! the crate's CPU colour rules (painting/color_rules.rs, painting/color_mix.rs), the colour part
//! of the draw block, how the builder names a draw's source, picture targets (sizes, refusals,
//! identity, the report of a picture read before it was drawn) and an ImageShape of a target.

use kurbo::Affine;

use crate::geometry::RectangleF;
use crate::images::{ImageSampling, ImageSource};
use crate::painting::color_mix::{color_to_oklab, mix_float, srgb_to_oklab, ColorInterpolation};
use crate::painting::{BrokenColor, ColorRules, PaintPalette, PaletteMode};
use crate::rendering::TestBuilder as Builder;
use crate::rendering::picture_target::{read_too_early, set_canvas_frame};
use crate::rendering::{PictureTarget, VectorScene};
use crate::shapes::{BrushStrokesOptions, BrushStrokesShape, ImageShape};
use crate::strokes::stroke_color::SourceTexels;
use crate::strokes::stroke_color_gpu::mirror::{decode, encode, linear_to_oklab, oklab_to_linear, picked_colour, picked_colour_weighted, pickup_points, source_texel, tap_offsets};
use crate::strokes::stroke_color_gpu::{colour_words, PICK_BROKEN, PICK_MIX_OF_TWO, PICK_ON, PICK_PALETTE};
use crate::strokes::{BrushStrokeRecord, BrushStrokeSet, StrokeColor, StrokeColorSource, StrokePickup};
use crate::{Color, DrawingSession, DrawingSessionOptions, PointF};

fn picture() -> ImageSource {
    let mut px = Vec::new();
    for y in 0..8u32 {
        for x in 0..8u32 {
            px.extend_from_slice(&[(x * 32) as u8, (y * 32) as u8, 128, 255]);
        }
    }
    ImageSource::from_rgba(&px, 8, 8).unwrap()
}

fn pickup(source: StrokeColorSource) -> StrokePickup {
    StrokePickup::new(source, RectangleF::new(0.0, 0.0, 200.0, 100.0))
}

fn grid() -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for r in [0.0f32, 0.1, 0.35, 0.6, 0.85, 1.0] {
        for g in [0.0f32, 0.2, 0.5, 0.75, 1.0] {
            for b in [0.0f32, 0.15, 0.45, 0.9] {
                out.push([r, g, b]);
            }
        }
    }
    out
}

fn close(a: [f32; 3], b: [f32; 3], tolerance: f32) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tolerance)
}

fn six_pigments() -> Vec<Color> {
    vec![Color::from_rgb(20, 30, 90), Color::from_rgb(230, 200, 60), Color::from_rgb(180, 40, 40), Color::from_rgb(40, 120, 60), Color::from_rgb(245, 240, 230), Color::from_rgb(90, 60, 40)]
}

#[test]
fn a_pickup_is_checked_when_it_is_given_to_a_shape() {
    //Arrange
    let set = BrushStrokeSet::new(vec![BrushStrokeRecord::new(PointF::new(10.0, 10.0), 20.0, 5.0, 0.0, Color::WHITE)]).unwrap();
    let shape = || BrushStrokesShape::new(set.clone(), BrushStrokesOptions::default()).unwrap();
    let with = |p: StrokePickup| shape().with_color(StrokeColor::Picked(p));
    //Act / Assert
    assert!(with(StrokePickup { taps: 2, ..pickup(StrokeColorSource::Picture(picture())) }).unwrap_err().contains("1, 3 or 5 taps"));
    assert!(with(StrokePickup { own_weight: 1.5, ..pickup(StrokeColorSource::Picture(picture())) }).unwrap_err().contains("own weight"));
    assert!(with(StrokePickup { place: RectangleF::new(0.0, 0.0, 0.0, 10.0), ..pickup(StrokeColorSource::Picture(picture())) }).unwrap_err().contains("place"));
    let bad_rules = ColorRules { palette: Some(PaintPalette::new(Vec::new())), broken: None };
    assert!(with(StrokePickup { rules: Some(bad_rules), ..pickup(StrokeColorSource::Picture(picture())) }).unwrap_err().contains("pigments"));
    let ok = with(pickup(StrokeColorSource::Picture(picture()))).unwrap();
    assert!(matches!(ok.color(), StrokeColor::Picked(p) if p.taps == 5 && p.own_weight == 0.0 && p.rules.is_none()));
    assert_eq!(shape().color(), &StrokeColor::Own, "own colours by default");
}

#[test]
fn the_taps_lie_along_the_centre_line_of_straight_bent_and_anchored_strokes() {
    //Arrange
    let p = pickup(StrokeColorSource::Picture(picture()));
    //Act
    let offsets = (tap_offsets(1), tap_offsets(3), tap_offsets(5));
    let straight = pickup_points((100.0, 50.0), 0.0, 40.0, 0.0, Affine::IDENTITY, &p);
    let bent = pickup_points((100.0, 50.0), 0.0, 40.0, 0.1, Affine::IDENTITY, &p);
    let turned = pickup_points((100.0, 50.0), 90.0, 40.0, 0.0, Affine::IDENTITY, &p);
    let anchored = pickup_points((100.0, 50.0), 0.0, 40.0, 0.0, Affine::translate((50.0, 10.0)), &p);
    //Assert
    assert_eq!(offsets.0, vec![0.0]);
    assert_eq!(offsets.1, vec![-0.4, 0.0, 0.4]);
    assert!(close([offsets.2[1], offsets.2[2], offsets.2[3]], [-0.2, 0.0, 0.2], 1e-6));
    for (k, (u, v)) in straight.iter().enumerate() {
        assert!((u - (100.0 + 40.0 * offsets.2[k]) / 200.0).abs() < 1e-6 && (v - 0.5).abs() < 1e-6, "tap {k} at ({u}, {v})");
    }
    // Bent: the centre line is bend x length x (4 s^2 - 1) across (BrushStrokeShape's formula):
    // 4 units off the axis in the middle, 4 x (1 - 0.64) = 1.44 at s = +-0.4.
    use crate::strokes::stroke_gpu::centre_at;
    assert!((bent[2].1 * 100.0 - (50.0 + centre_at(4.0, 0.0))).abs() < 1e-4 && (bent[2].1 * 100.0 - 46.0).abs() < 1e-4);
    assert!((bent[0].1 * 100.0 - (50.0 - 1.44)).abs() < 1e-4);
    // Turned 90 degrees clockwise on screen: the taps run down.
    assert!(turned.iter().all(|(u, _)| (u - 0.5).abs() < 1e-6));
    assert!((turned[4].1 * 100.0 - 66.0).abs() < 1e-4);
    // Anchored: the taps move with the anchor (the stroke as drawn this frame).
    for (a, s) in anchored.iter().zip(&straight) {
        assert!((a.0 - s.0 - 0.25).abs() < 1e-6 && (a.1 - s.1 - 0.1).abs() < 1e-6);
    }
}

#[test]
fn source_texels_become_linear_premultiplied_colour_by_their_encoding() {
    //Arrange
    let srgb = |v: f32| crate::rendering::core_paint_srgb_to_linear(v);
    //Act
    let straight = source_texel([1.0, 0.5, 0.0, 0.5], SourceTexels::SrgbStraight);
    let premultiplied = source_texel([0.5, 0.25, 0.0, 0.5], SourceTexels::SrgbPremultiplied);
    let linear = source_texel([0.2, 0.1, 0.0, 0.5], SourceTexels::LinearPremultiplied);
    let clamped = source_texel([2.0, -1.0, 0.5, 1.5], SourceTexels::LinearPremultiplied);
    //Assert
    assert!(close([straight[0], straight[1], straight[2]], [0.5, srgb(0.5) * 0.5, 0.0], 1e-6) && straight[3] == 0.5);
    assert!(close([premultiplied[0], premultiplied[1], premultiplied[2]], [0.5, srgb(0.5) * 0.5, 0.0], 1e-6), "un-premultiplied, decoded, premultiplied again");
    assert_eq!(linear, [0.2, 0.1, 0.0, 0.5]);
    assert_eq!(clamped, [1.0, 0.0, 0.5, 1.0], "an HDR value is clamped (tone-map first)");
    assert_eq!(source_texel([0.3, 0.3, 0.3, 0.0], SourceTexels::SrgbPremultiplied), [0.0; 4]);
}

#[test]
fn the_shaders_oklab_agrees_with_the_crates_oklab() {
    for c in grid() {
        //Act
        let lab = linear_to_oklab(decode(c));
        let back = encode(oklab_to_linear(lab));
        //Assert
        assert!(close(lab, srgb_to_oklab(c), 2e-5), "{c:?}: {lab:?} vs {:?}", srgb_to_oklab(c));
        assert!(close(back, c, 2e-4), "{c:?} round trip: {back:?}");
    }
}

#[test]
fn the_shaders_colour_rules_agree_with_color_rules_apply_float() {
    //Arrange
    let nearest = ColorRules { palette: Some(PaintPalette::new(six_pigments())), broken: None };
    let mix_of_two = ColorRules { palette: Some(PaintPalette { pigments: six_pigments(), snap: 0.7, mode: PaletteMode::MixOfTwo }), broken: None };
    let broken = ColorRules { palette: None, broken: Some(BrokenColor { hue: 15.0, chroma: 0.2, lightness: 0.05, seed: 9 }) };
    let both = ColorRules { palette: mix_of_two.palette.clone(), broken: broken.broken };
    for rules in [&nearest, &mix_of_two, &broken, &both] {
        for (i, c) in grid().into_iter().enumerate() {
            let seed = (i as u32).wrapping_mul(2_654_435_761);
            //Act
            let gpu = encode(crate::strokes::stroke_color_gpu::mirror::colour_rules(decode(c), rules, seed));
            let cpu = rules.apply_float([c[0], c[1], c[2], 1.0], seed);
            //Assert: within float rounding (well under one 8-bit step).
            assert!(close(gpu, [cpu[0], cpu[1], cpu[2]], 5e-4), "{rules:?} on {c:?} seed {seed}: GPU {gpu:?} CPU {cpu:?}");
        }
    }
}

#[test]
fn the_picked_colour_is_the_alpha_weighted_mean_then_the_rules_then_the_oklab_mix() {
    //Arrange: two taps, one transparent - the mean is the opaque one's colour at half alpha.
    let rules = ColorRules { palette: Some(PaintPalette { pigments: six_pigments(), snap: 0.5, mode: PaletteMode::Nearest }), broken: None };
    let own = decode([0.9, 0.1, 0.1]);
    let texels = [[0.2, 0.5, 0.8, 1.0], [0.0, 0.0, 0.0, 0.0]];
    let plain = StrokePickup { taps: 3, ..pickup(StrokeColorSource::Picture(picture())) };
    let mixed = StrokePickup { own_weight: 0.25, rules: Some(rules.clone()), ..plain.clone() };
    //Act
    let (a, alpha_a) = picked_colour(own, &texels, SourceTexels::SrgbStraight, &plain, 7);
    let (b, alpha_b) = picked_colour(own, &texels, SourceTexels::SrgbStraight, &mixed, 7);
    let (empty, alpha_empty) = picked_colour(own, &[[0.0; 4]], SourceTexels::SrgbStraight, &plain, 7);
    //Assert
    assert!(close(encode(a), [0.2, 0.5, 0.8], 1e-5) && alpha_a == 0.5);
    let ruled = rules.apply_float([0.2, 0.5, 0.8, 1.0], 7);
    let expected = mix_float(ruled, [0.9, 0.1, 0.1, 1.0], 0.25, ColorInterpolation::Oklab);
    assert!(close(encode(b), [expected[0], expected[1], expected[2]], 5e-4), "{:?} vs {expected:?}", encode(b));
    assert!((alpha_b - 0.625).abs() < 1e-6, "the source's alpha mixed towards 1 by the own weight");
    assert!(close(empty, own, 1e-6) && alpha_empty == 0.0, "an empty source: nothing painted");
}

#[test]
fn a_strokes_own_weight_scales_the_pickups_and_weight_one_is_the_pickups_exactly() {
    //Arrange
    let own = decode([0.9, 0.1, 0.1]);
    let texels = [[0.2, 0.5, 0.8, 1.0], [0.0, 0.0, 0.0, 0.0]];
    let base = StrokePickup { taps: 3, ..pickup(StrokeColorSource::Picture(picture())) };
    let half = StrokePickup { own_weight: 0.5, ..base.clone() };
    let full = StrokePickup { own_weight: 1.0, ..base.clone() };
    let quarter = StrokePickup { own_weight: 0.25, ..base.clone() };
    //Act + Assert
    assert_eq!(picked_colour_weighted(own, &texels, SourceTexels::SrgbStraight, &half, 7, 1.0), picked_colour(own, &texels, SourceTexels::SrgbStraight, &half, 7), "weight 1: the pickup's own weight, bit for bit");
    assert_eq!(picked_colour_weighted(own, &texels, SourceTexels::SrgbStraight, &half, 7, 0.5), picked_colour(own, &texels, SourceTexels::SrgbStraight, &quarter, 7), "0.5 of 0.5 is 0.25");
    assert_eq!(picked_colour_weighted(own, &texels, SourceTexels::SrgbStraight, &full, 7, 0.25), picked_colour(own, &texels, SourceTexels::SrgbStraight, &quarter, 7), "a pickup of 1: the stroke's weight itself");
    assert_eq!(picked_colour_weighted(own, &texels, SourceTexels::SrgbStraight, &half, 7, 0.0), picked_colour(own, &texels, SourceTexels::SrgbStraight, &base, 7), "weight 0: the picked colour alone");
    assert_eq!(picked_colour_weighted(own, &texels, SourceTexels::SrgbStraight, &base, 7, 1.0), picked_colour(own, &texels, SourceTexels::SrgbStraight, &base, 7), "a pickup of 0 gives no stroke its own colour");
    let (rgb, alpha) = picked_colour_weighted(own, &[[0.0; 4]], SourceTexels::SrgbStraight, &full, 7, 1.0);
    assert!(close(rgb, own, 1e-5) && alpha == 1.0, "own weight 1 in full over an empty source: the own colour, opaque");
}

#[test]
fn the_colour_part_of_the_block_holds_the_place_flags_rules_and_pigments_in_oklab() {
    //Arrange
    let rules = ColorRules { palette: Some(PaintPalette { pigments: six_pigments(), snap: 0.8, mode: PaletteMode::MixOfTwo }), broken: Some(BrokenColor { hue: 18.0, chroma: 0.2, lightness: 0.04, seed: 77 }) };
    let p = StrokePickup { taps: 3, own_weight: 0.3, rules: Some(rules), ..StrokePickup::new(StrokeColorSource::Picture(picture()), RectangleF::new(10.0, 20.0, 400.0, 250.0)) };
    //Act
    let w = colour_words(Some((&p, SourceTexels::SrgbPremultiplied)));
    let none = colour_words(None);
    //Assert
    let f = |i: usize| f32::from_bits(w[i]);
    assert_eq!(w.len(), 37 * 4);
    assert!(none.iter().all(|v| *v == 0));
    assert_eq!((f(0), f(1), f(2), f(3)), (10.0, 20.0, 1.0 / 400.0, 1.0 / 250.0));
    assert_eq!(w[4], PICK_ON | PICK_PALETTE | PICK_MIX_OF_TWO | PICK_BROKEN);
    assert_eq!((w[5], w[6], w[7]), (1, 3, 6), "premultiplied sRGB texels, 3 taps, 6 pigments");
    assert!((f(8) - 0.3).abs() < 1e-7 && (f(9) - 0.8).abs() < 1e-7);
    assert!((f(10) - 18f32.to_radians()).abs() < 1e-6 && (f(11) - 0.2).abs() < 1e-7);
    assert!((f(12) - 0.04).abs() < 1e-7 && w[16] == 77);
    for (i, c) in six_pigments().iter().enumerate() {
        let lab = color_to_oklab(*c);
        assert_eq!([f(20 + 4 * i), f(21 + 4 * i), f(22 + 4 * i)], lab, "pigment {i}");
    }
}

#[test]
fn the_builder_names_each_draws_colour_source_and_a_picture_joins_the_frames_pictures() {
    //Arrange
    let set = BrushStrokeSet::new(vec![BrushStrokeRecord::new(PointF::new(100.0, 100.0), 40.0, 10.0, 0.0, Color::WHITE)]).unwrap();
    let target = PictureTarget::new(64, 32).unwrap();
    let picked = |source: StrokeColorSource| BrushStrokesShape::new(set.clone(), BrushStrokesOptions::default()).unwrap().with_color(StrokeColor::Picked(pickup(source))).unwrap();
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (640, 360), layer_opacity: 255, ..DrawingSessionOptions::default() });
    s.add_layer("Paint", Color::WHITE);
    s.draw_brush_strokes(picked(StrokeColorSource::Picture(picture())));
    s.draw_brush_strokes(picked(StrokeColorSource::Target(target.clone())));
    s.draw_brush_strokes(BrushStrokesShape::new(set.clone(), BrushStrokesOptions::default()).unwrap());
    let mut scene = VectorScene::new();
    s.render(&mut scene, 640, 360);
    let mut b = Builder::default();
    //Act
    b.build(&scene, 640, 360);
    //Assert
    use crate::rendering::TestStrokeSource as Source;
    assert_eq!(b.stroke_ops.len(), 3);
    assert_eq!(b.stroke_ops[0].source, Source::Image(1), "the picture is the frame's first picture");
    assert_eq!(b.images.len(), 1);
    assert_eq!(b.images[0].brush.image.data.id(), picture_id(&scene), "the same pixels: uploaded once, as any picture");
    assert_eq!(b.stroke_ops[1].source, Source::Target(target));
    assert_eq!(b.stroke_ops[2].source, Source::None);
    let word = |op: usize, i: usize| b.stroke_blocks[b.stroke_ops[op].offset as usize / 4 + 28 + i];
    assert_eq!((word(0, 4), word(0, 5)), (PICK_ON, SourceTexels::SrgbStraight as u32));
    assert_eq!((word(1, 4), word(1, 5)), (PICK_ON, SourceTexels::SrgbPremultiplied as u32));
    assert_eq!(word(2, 4), 0);
}

fn picture_id(scene: &VectorScene) -> u64 {
    let StrokeColor::Picked(p) = &scene.stroke_draws[0].color else { panic!("picked") };
    let StrokeColorSource::Picture(i) = &p.source else { panic!("a picture") };
    i.image_data().data.id()
}

#[test]
fn picture_targets_have_a_checked_size_and_are_the_same_picture_as_their_clones() {
    //Act
    let a = PictureTarget::new(320, 180).unwrap();
    let b = a.clone();
    let c = PictureTarget::new(320, 180).unwrap();
    //Assert
    assert_eq!(a.size(), (320, 180));
    assert_eq!(a, b);
    assert_ne!(a, c, "two targets are two pictures");
    assert!(PictureTarget::new(0, 10).unwrap_err().contains("1 to 8192"));
    assert!(PictureTarget::new(10, 8193).is_err());
    assert_eq!(PictureTarget::find(a.stand_in().data.id()), Some(a.clone()), "found by its stand-in");
    let id = c.stand_in().data.id();
    drop(c);
    assert_eq!(PictureTarget::find(id), None, "a dropped target leaves the registry");
    assert_eq!((a.stand_in().width, a.stand_in().height), (320, 180));
    assert_eq!(a.stand_in().alpha_type, peniko::ImageAlphaType::AlphaPremultiplied);
}

#[test]
fn a_picture_read_before_its_layer_drew_it_this_frame_is_reported_once() {
    //Arrange
    let target = PictureTarget::new(64, 64).unwrap();
    let set = BrushStrokeSet::new(vec![BrushStrokeRecord::new(PointF::new(10.0, 10.0), 20.0, 5.0, 0.0, Color::WHITE)]).unwrap();
    let shape = BrushStrokesShape::new(set, BrushStrokesOptions::default()).unwrap().with_color(StrokeColor::Picked(pickup(StrokeColorSource::Target(target.clone())))).unwrap();
    let mut s = DrawingSession::new(DrawingSessionOptions::default());
    s.add_layer("Paint", Color::WHITE);
    s.draw_brush_strokes(shape);
    let render = |s: &DrawingSession| s.render(&mut VectorScene::new(), 640, 360);
    //Act / Assert
    assert!(!read_too_early(0, 0) && read_too_early(4, 5) && !read_too_early(5, 5) && read_too_early(0, 1));
    render(&s);
    assert!(s.take_reports().is_empty(), "outside a canvas nothing is reported");
    set_canvas_frame(5);
    target.mark_drawn(5);
    render(&s);
    assert!(s.take_reports().is_empty(), "drawn this frame: below its reader");
    set_canvas_frame(6);
    render(&s);
    let reports = s.take_reports();
    set_canvas_frame(7);
    render(&s);
    let again = s.take_reports();
    set_canvas_frame(0);
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].contains("BELOW the layers that read it"), "{}", reports[0]);
    assert!(again.is_empty(), "once in the session's life");
}

#[test]
fn an_image_shape_of_a_picture_target_draws_the_targets_stand_in_through_the_picture_path() {
    //Arrange
    let target = PictureTarget::new(100, 50).unwrap();
    let shape = ImageShape::from_picture_target(&target, RectangleF::new(10.0, 10.0, 200.0, 100.0), ImageSampling::Linear).unwrap();
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (640, 360), layer_opacity: 255, ..DrawingSessionOptions::default() });
    s.add_layer("Show", Color::WHITE);
    s.draw_shape(shape.clone());
    let mut scene = VectorScene::new();
    s.render(&mut scene, 640, 360);
    let mut b = Builder::default();
    //Act
    b.build(&scene, 640, 360);
    //Assert
    assert_eq!(shape.picture_target(), Some(&target));
    assert_eq!(shape.source().size(), (1, 1), "a stand-in source");
    assert_eq!(b.images.len(), 1);
    assert_eq!(b.images[0].brush.image.data.id(), target.stand_in().data.id());
    assert_eq!(PictureTarget::find(b.images[0].brush.image.data.id()), Some(target));
    assert!(ImageShape::from_picture_target(&PictureTarget::new(4, 4).unwrap(), RectangleF::new(0.0, 0.0, f32::NAN, 1.0), ImageSampling::Linear).is_err());
}
