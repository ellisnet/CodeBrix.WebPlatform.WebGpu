//! Host tests: the draw list - commands, transforms and colours at known frames computed by hand
//! from the JSON, groups and clips, folding, blend modes, solid layers, images, fits.

use std::sync::Arc;

use kurbo::{Affine, Point, Rect};

use crate::color::Color;
use crate::geometry::RectangleF;
use crate::lottie::test_support::*;
use crate::lottie::*;

const BASICS: &str = include_str!("test_files/basics.json");
const REPEATER: &str = include_str!("test_files/repeater.json");
const MASKS_MATTES: &str = include_str!("test_files/masks_mattes.json");
const SOLID_IMAGE: &str = include_str!("test_files/solid_image.json");
const NOT_DRAWN: &str = include_str!("test_files/not_drawn.json");
const A: &str = "A_many_shapes.json";
const B: &str = "B_painterly.json";
const C: &str = "C_busy_scene.json";

fn list(json: &str, frame: f64) -> LottieDrawList {
    load(json).draw_list(frame, Affine::IDENTITY, 1.0, true)
}

#[test]
fn basics_at_frame_5_moves_fades_and_turns_as_the_json_says() {
    //Arrange
    let list = list(BASICS, 5.0);
    //Act
    let d = draws(&list);
    //Assert - the frame clip, the spinner's stroke (bottom layer first), the box's fill
    assert_eq!(list.commands.len(), 4);
    assert!(matches!(&list.commands[0], LottieDrawCommand::PushClip { rect: Some(r), .. } if *r == Rect::new(0.0, 0.0, 100.0, 100.0)));
    assert!(matches!(list.commands[3], LottieDrawCommand::Pop));
    let LottieDrawCommand::Stroke { transform, stroke, .. } = d[0] else { panic!("stroke first: {:?}", d[0]) };
    // rotation eased 0 -> 90 over frames 0..10 with symmetric handles: 45 degrees at frame 5
    let h = 10.0 * std::f64::consts::FRAC_1_SQRT_2;
    assert!(at(*transform * Point::new(10.0, 0.0), 50.0 + h, 50.0 + h), "{transform:?}");
    assert_eq!(stroke.paint, LottiePaint::Solid(Color::from_rgb(0, 0, 255)));
    assert!(near(stroke.opacity as f64, 0.8, 1e-6) && stroke.width == 4.0 && stroke.miter_limit == 4.0);
    assert_eq!((stroke.cap, stroke.join), (LottieLineCap::Round, LottieLineJoin::Bevel));
    let LottieDrawCommand::Fill { transform, paint, opacity, .. } = d[1] else { panic!("fill second") };
    // position linear (0,0) -> (20,0) over frames 0..10: (10,0) at frame 5; layer opacity 50 %
    assert!(at(*transform * Point::ORIGIN, 10.0, 0.0));
    assert_eq!(*paint, LottiePaint::Solid(Color::from_rgb(255, 0, 0)));
    assert!(near(*opacity as f64, 0.5, 1e-6));
    let b = device_bounds(d[1]);
    assert!(near(b.x0, 40.0, 1e-6) && near(b.y0, 40.0, 1e-6) && near(b.x1, 80.0, 1e-6) && near(b.y1, 60.0, 1e-6), "{b:?}");
}

#[test]
fn basics_holds_its_colour_keyframe_and_ends_its_motion() {
    //Arrange
    let lottie = load(BASICS);
    //Act
    let before = lottie.draw_list(9.9, Affine::IDENTITY, 1.0, true);
    let after = lottie.draw_list(10.0, Affine::IDENTITY, 1.0, true);
    let late = lottie.draw_list(15.0, Affine::IDENTITY, 1.0, true);
    //Assert
    let paint = |l: &LottieDrawList| match draws(l)[1] {
        LottieDrawCommand::Fill { paint, .. } => paint.clone(),
        _ => panic!("fill"),
    };
    assert_eq!(paint(&before), LottiePaint::Solid(Color::from_rgb(255, 0, 0)), "a hold keyframe keeps red until frame 10");
    assert_eq!(paint(&after), LottiePaint::Solid(Color::from_rgb(0, 255, 0)));
    assert!(at(transform_of(draws(&late)[1]) * Point::ORIGIN, 20.0, 0.0));
    assert!(at(transform_of(draws(&late)[0]) * Point::new(10.0, 0.0), 50.0, 60.0), "90 degrees after frame 10");
}

#[test]
fn a_shape_fits_the_document_into_its_destination() {
    //Arrange
    let lottie = Arc::new(load(SOLID_IMAGE));
    let destination = RectangleF::new(10.0, 10.0, 240.0, 240.0);
    let mut meet = DrawingLottieShape::new(lottie.clone(), destination, DrawingLottieAspect::Meet, LottiePlayOptions::default()).unwrap();
    let mut slice = DrawingLottieShape::new(lottie.clone(), destination, DrawingLottieAspect::Slice, LottiePlayOptions::default()).unwrap();
    let stretch = DrawingLottieShape::new(lottie.clone(), destination, DrawingLottieAspect::Stretch, LottiePlayOptions::default()).unwrap();
    let none = DrawingLottieShape::new(lottie, destination, DrawingLottieAspect::None, LottiePlayOptions::default()).unwrap();
    //Act
    let meet_list = meet.draw_list(0.0);
    let slice_list = slice.draw_list(0.0);
    //Assert - a 120 x 80 document: Meet scale 2 centred (y 50..210), Slice scale 3 (x -50..310)
    assert_eq!(meet.get_bounds(), RectangleF::new(10.0, 50.0, 240.0, 160.0));
    assert_eq!(slice.content_bounds(), RectangleF::new(-50.0, 10.0, 360.0, 240.0));
    assert_eq!(slice.get_bounds(), destination);
    assert_eq!(stretch.content_bounds(), destination);
    assert_eq!(none.content_bounds(), RectangleF::new(10.0, 10.0, 120.0, 80.0));
    let b = device_bounds(draws(&meet_list)[0]);
    assert!(near(b.x0, 10.0, 1e-6) && near(b.y0, 50.0, 1e-6) && near(b.x1, 250.0, 1e-6) && near(b.y1, 210.0, 1e-6), "{b:?}");
    assert!(matches!(&slice_list.commands[0], LottieDrawCommand::PushClip { rect: Some(r), transform, .. } if *r == Rect::new(10.0, 10.0, 250.0, 250.0) && *transform == Affine::IDENTITY));
    assert_eq!(slice_list.commands.len(), meet_list.commands.len() + 2);
}

#[test]
fn solid_layers_become_rectangles_and_images_become_records() {
    //Arrange
    let lottie = load(SOLID_IMAGE);
    //Act
    let list = lottie.draw_list(0.0, Affine::IDENTITY, 1.0, true);
    //Assert - frame clip, the backdrop (bottom), the image clip, the image
    let d = draws(&list);
    assert_eq!(d.len(), 2);
    let LottieDrawCommand::Fill { paint, opacity, .. } = d[0] else { panic!("solid fill") };
    assert_eq!((paint.clone(), *opacity), (LottiePaint::Solid(Color::from_rgb(0, 255, 0)), 1.0));
    assert_eq!(device_bounds(d[0]), Rect::new(0.0, 0.0, 120.0, 80.0));
    let LottieDrawCommand::Image { asset_id, size, opacity, .. } = d[1] else { panic!("image") };
    assert_eq!((asset_id.as_str(), *size), ("picture", Some((64.0, 32.0))));
    assert!(near(*opacity as f64, 0.4, 1e-6));
    // a 128 x 64 picture is fitted into the authored 64 x 32 box at (10, 20)
    let place = d[1].image_transform(128, 64).unwrap();
    assert!(at(place * Point::new(128.0, 64.0), 74.0, 52.0) && at(place * Point::ORIGIN, 10.0, 20.0));
    assert_eq!(list.clip_count(), 2, "the frame and the image's authored box");
}

#[test]
fn solid_layers_can_be_left_undrawn() {
    //Arrange
    let options = DrawingLottieOptions { draw_solid_layers: false, ..DrawingLottieOptions::default() };
    //Act
    let lottie = DrawingLottie::from_json(SOLID_IMAGE, &options).unwrap();
    //Assert
    assert_eq!(lottie.draw_list(0.0, Affine::IDENTITY, 1.0, true).draw_count(), 1);
    assert!(has(&lottie, DrawingLottieWarningKind::LayerTypeNotDrawn));
}

#[test]
fn repeater_copies_fade_in_groups_that_fold_into_their_draws() {
    //Arrange
    let lottie = load(REPEATER);
    //Act
    let raw = lottie.draw_list(0.0, Affine::IDENTITY, 1.0, false);
    let folded = lottie.draw_list(0.0, Affine::IDENTITY, 1.0, true);
    //Assert - 3 copies 30 apart, opacity 100 % -> 50 %: copy i at 1 - 0.25 i; "below": copy 2 painted first
    assert_eq!((raw.group_count(), raw.draw_count()), (2, 3));
    assert_eq!((folded.group_count(), folded.draw_count(), folded.folded_groups), (0, 3, 2));
    assert_eq!(folded.commands.len(), 5, "frame clip, three fills, pop - no clip kept: each copy lies inside the group's area");
    let got: Vec<(f64, f64)> = draws(&folded).iter().map(|c| (device_bounds(c).x0, opacity_of(c) as f64)).collect();
    assert_eq!(got, vec![(75.0, 0.5), (45.0, 0.75), (15.0, 1.0)]);
    assert_eq!(effective_draws(&raw), effective_draws(&folded));
}

#[test]
fn folding_keeps_what_reaches_the_screen_for_the_busy_scene() {
    //Arrange
    let Some(c) = trial_file(C) else { return };
    let lottie = load(&c);
    for frame in [0.0, 37.0, 90.5] {
        //Act
        let raw = lottie.draw_list(frame, Affine::IDENTITY, 1.0, false);
        let folded = lottie.draw_list(frame, Affine::IDENTITY, 1.0, true);
        //Assert
        assert_eq!(raw.draw_count(), folded.draw_count());
        assert_eq!(raw.group_count(), folded.group_count() + folded.folded_groups);
        let (a, b) = (effective_draws(&raw), effective_draws(&folded));
        for (x, y) in a.iter().zip(&b) {
            assert!(near(x.0 as f64, y.0 as f64, 1e-6) && x.1 == y.1);
        }
    }
}

#[test]
fn the_invented_lotties_give_the_trial_counts() {
    //Arrange
    let (Some(a), Some(b), Some(c)) = (trial_file(A), trial_file(B), trial_file(C)) else { return };
    let (a, b, c) = (load(&a), load(&b), load(&c));
    //Act
    let a0 = a.draw_list(0.0, Affine::IDENTITY, 1.0, false);
    let a30 = a.draw_list(30.0, Affine::IDENTITY, 1.0, false);
    let b0 = b.draw_list(0.0, Affine::IDENTITY, 1.0, false);
    let c0 = c.draw_list(0.0, Affine::IDENTITY, 1.0, false);
    //Assert - the lottie-test trial: A 600-900 paths (600 at the trims' empty moment), no groups;
    // B 53-77 paths, the matte's 2 groups, 2 clips (frame + mask), 3 deep; C 512 paths, 48 groups
    assert_eq!((a0.draw_count(), a0.group_count(), a0.clip_count()), (600, 0, 1));
    assert_eq!(a30.draw_count(), 900);
    assert!((53..=77).contains(&b0.draw_count()), "{}", b0.draw_count());
    assert_eq!((b0.group_count(), b0.clip_count(), b0.max_depth()), (2, 2, 3));
    assert_eq!((c0.draw_count(), c0.group_count()), (512, 48));
    assert_eq!(a0.unsupported + b0.unsupported + c0.unsupported, 0);
}

#[test]
fn file_a_draws_its_last_layer_first_with_the_json_transform_and_colours() {
    //Arrange - "shape 299": position (196.96, 552.11), scale 68.51 %, rotation 154.31 degrees at
    // frame 0; its ellipse stroke is trimmed to nothing, its star stroked (0.08, 0.08, 0.12) and
    // filled (0.95, 0.24, 0.25) at 85 % - the stroke listed after the fill lies below it
    let Some(a) = trial_file(A) else { return };
    let list = list(&a, 0.0);
    //Act
    let d = draws(&list);
    //Assert
    let (s, c) = (0.6851, 154.31f64.to_radians().cos());
    let sn = 154.31f64.to_radians().sin();
    for draw in &d[0..2] {
        let t = transform_of(draw);
        assert!(at(t * Point::ORIGIN, 196.96, 552.11), "{t:?}");
        assert!(at(t * Point::new(1.0, 0.0), 196.96 + s * c, 552.11 + s * sn), "{t:?}");
    }
    let LottieDrawCommand::Stroke { stroke, .. } = d[0] else { panic!("stroke first") };
    assert_eq!(stroke.paint, LottiePaint::Solid(Color::from_rgb(20, 20, 31)));
    assert_eq!((stroke.width, stroke.cap, stroke.join), (1.25, LottieLineCap::Round, LottieLineJoin::Round));
    let LottieDrawCommand::Fill { paint, opacity, .. } = d[1] else { panic!("fill second") };
    assert_eq!(*paint, LottiePaint::Solid(Color::from_rgb(242, 61, 64)));
    assert!(near(*opacity as f64, 0.85, 1e-6));
}

#[test]
fn masks_clip_and_mattes_composite_in_their_groups() {
    //Arrange
    let list = list(MASKS_MATTES, 0.0);
    //Act
    let kinds: Vec<String> = list
        .commands
        .iter()
        .map(|c| match c {
            LottieDrawCommand::PushGroup { blend, .. } => format!("group {blend:?}"),
            LottieDrawCommand::PushClip { .. } => "clip".to_string(),
            LottieDrawCommand::Fill { paint: LottiePaint::Solid(c), .. } => format!("fill {},{},{}", c.r, c.g, c.b),
            LottieDrawCommand::Pop => "pop".to_string(),
            other => format!("{other:?}"),
        })
        .collect();
    //Assert - bottom layer first; the "none" mask is gone, the subtract mask clips to its inside
    let expected = [
        "clip", "clip", "fill 0,255,0", "pop", "clip", "fill 0,0,0", "pop", "group Normal", "fill 255,255,255", "group SourceIn", "fill 0,0,255", "pop", "pop", "group Normal", "fill 255,255,255", "group SourceIn", "fill 255,128,0", "pop", "pop", "pop",
    ];
    assert_eq!(kinds, expected);
}

#[test]
fn not_drawn_still_draws_the_rest_with_top_level_blend_modes() {
    //Arrange
    let lottie = load(NOT_DRAWN);
    //Act
    let list = lottie.draw_list(0.0, Affine::IDENTITY, 1.0, true);
    //Assert
    let blends: Vec<LottieBlendMode> = list.commands.iter().filter_map(|c| if let LottieDrawCommand::PushGroup { blend, .. } = c { Some(*blend) } else { None }).collect();
    assert_eq!(blends, vec![LottieBlendMode::Add, LottieBlendMode::Multiply], "the nested multiply and the hard mix are not applied");
    let fills: Vec<(Color, Affine)> = draws(&list).iter().filter_map(|c| if let LottieDrawCommand::Fill { paint: LottiePaint::Solid(col), transform, .. } = c { Some((*col, *transform)) } else { None }).collect();
    assert!(!fills.iter().any(|(c, _)| *c == Color::from_rgb(255, 0, 0)), "the hidden red fill is removed");
    let (_, yellow) = fills.iter().find(|(c, _)| *c == Color::from_rgb(255, 255, 0)).expect("the even-odd fill is drawn non-zero");
    assert!(at(*yellow * Point::ORIGIN, 100.0, 30.0), "parented to the text layer kept as a null: (100,20) + (0,10)");
    assert!(fills.iter().any(|(c, _)| *c == Color::from_rgb(255, 0, 255)), "the precomposition's content is drawn");
}

#[test]
fn old_format_keyframes_and_per_dimension_easing_are_played() {
    //Arrange - "hard mix": rotation 0 at frame 0 (handles reduced to o (0.2, 0) / i (0.8, 1),
    // symmetric: half way at frame 15), 30 at frame 30 with end value 45, a last keyframe at 60
    // without "s" (completed from the previous "e"): linear 30 -> 45 between frames 30 and 60
    let lottie = load(NOT_DRAWN);
    let angle = |frame: f64| {
        let list = lottie.draw_list(frame, Affine::IDENTITY, 1.0, true);
        let t = draws(&list).iter().find_map(|c| if let LottieDrawCommand::Fill { paint: LottiePaint::Solid(col), transform, .. } = c { (*col == Color::from_rgb(0, 255, 255)).then_some(*transform) } else { None }).expect("the cyan square");
        let p = t * Point::new(1.0, 0.0) - t * Point::ORIGIN;
        p.y.atan2(p.x).to_degrees()
    };
    //Act
    let (a, b) = (angle(15.0), angle(45.0));
    //Assert
    assert!(near(a, 15.0, 1e-3) && near(b, 37.5, 1e-6), "{a} {b}");
}

#[test]
fn a_gradient_becomes_plain_stops() {
    //Arrange - the "adds" layer's radial gradient: centre (100,100), end (115,100), white -> black
    let list = list(NOT_DRAWN, 0.0);
    //Act
    let gradient = draws(&list).iter().find_map(|c| if let LottieDrawCommand::Fill { paint: LottiePaint::Radial(g), .. } = c { Some(g.clone()) } else { None }).expect("radial");
    //Assert
    assert_eq!((gradient.center, gradient.radius), ((100.0, 100.0), 15.0));
    assert_eq!(gradient.stops, vec![LottieGradientStop { offset: 0.0, color: Color::WHITE }, LottieGradientStop { offset: 1.0, color: Color::BLACK }]);
}

#[test]
fn an_empty_matte_content_group_is_kept_because_it_clears_the_matte() {
    //Arrange - the matted layer of masks_mattes.json drawn with nothing inside its SourceIn group
    let json = MASKS_MATTES.replace(r#"{"ty":"fl","c":{"a":0,"k":[1,0.5,0,1]},"o":{"a":0,"k":100},"r":1}"#, r#"{"ty":"fl","c":{"a":0,"k":[1,0.5,0,1]},"o":{"a":0,"k":0},"r":1}"#);
    //Act
    let list = load(&json).draw_list(0.0, Affine::IDENTITY, 1.0, true);
    //Assert
    let source_in = list.commands.iter().filter(|c| matches!(c, LottieDrawCommand::PushGroup { blend: LottieBlendMode::SourceIn, .. })).count();
    assert_eq!(source_in, 2);
}
