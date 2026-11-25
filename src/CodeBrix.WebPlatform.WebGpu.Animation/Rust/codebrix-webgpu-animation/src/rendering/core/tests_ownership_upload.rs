//! Host tests of what an ownership map uploads to the GPU (Animation 6): the texture ledger's
//! rules, and through a session and the renderer's CPU half the upload counts of RenderStats -
//! a colours-only change made with `OwnershipShape::with_colors` sends the colour table and
//! nothing else. Not in C#.

use crate::rendering::core::build::Builder;
use crate::rendering::core::texture_ledger::{plan, TextureLedger, Upload};
use crate::rendering::VectorScene;
use crate::shapes::{OwnershipShape, OwnershipShapeOptions, RectangleShape, RectangleShapeOptions};
use crate::{Color, DrawingSession, DrawingSessionOptions, RectangleF};

/// A 560 x 420 map (the sample's size) whose left half one element owns.
fn shape() -> OwnershipShape {
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (560, 420), ..DrawingSessionOptions::default() });
    s.add_layer("Paint", Color::WHITE).add_shape(RectangleShape::new(0.0, 0.0, 280.0, 420.0, RectangleShapeOptions { is_filled: true, ..Default::default() }).unwrap());
    let map = s.export_layer_ownership_map("Paint", (560, 420)).unwrap();
    OwnershipShape::new(&map, &[Color::from_rgb(10, 20, 30)], RectangleF::new(0.0, 0.0, 560.0, 420.0), OwnershipShapeOptions::default()).unwrap()
}

/// The pictures a frame drawing these shapes hands the GPU half.
fn frame_images(shapes: &[&OwnershipShape]) -> Vec<crate::rendering::core::texture_ledger::FrameImage> {
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (560, 420), layer_opacity: 255, ..DrawingSessionOptions::default() });
    let layer = s.add_layer("Map", Color::WHITE);
    for shape in shapes {
        layer.add_shape((*shape).clone());
    }
    let mut scene = VectorScene::new();
    s.render(&mut scene, 560, 420);
    let mut b = Builder::default();
    b.build(&scene, 560, 420);
    b.images.clone()
}

#[test]
fn a_colours_only_change_made_with_with_colors_uploads_the_colour_table_and_nothing_else() {
    //Arrange
    let first = shape();
    let mut ledger = TextureLedger::default();
    //Act
    let frame1 = plan(&mut ledger, &frame_images(&[&first]), 1);
    let recoloured = first.with_colors(&[Color::from_rgb(200, 20, 30)]).unwrap();
    let frame2 = plan(&mut ledger, &frame_images(&[&recoloured]), 2);
    let frame3 = plan(&mut ledger, &frame_images(&[&recoloured]), 3);
    let again = recoloured.with_colors(&[Color::from_rgb(0, 200, 30)]).unwrap();
    let frame4 = plan(&mut ledger, &frame_images(&[&again]), 4);
    //Assert: the first frame uploads the picture (560 x 421 x 4 bytes: the owner numbers and one
    // row of colours); each colours-only change one row of 560 colours; an unchanged frame nothing.
    assert_eq!((frame1.image_uploads, frame1.image_upload_bytes, frame1.color_table_uploads), (1, 560 * 421 * 4, 0));
    assert_eq!((frame2.image_uploads, frame2.color_table_uploads, frame2.color_table_upload_bytes), (0, 1, 560 * 4));
    assert_eq!((frame3.image_uploads, frame3.color_table_uploads), (0, 0));
    assert_eq!((frame4.image_uploads, frame4.color_table_uploads, frame4.color_table_upload_bytes), (0, 1, 560 * 4));
    assert_eq!(ledger.textures(), 1, "one texture holds the map throughout");
}

#[test]
fn a_shape_made_with_new_again_is_a_new_picture_and_a_whole_upload() {
    //Arrange: what every colour change cost before with_colors existed.
    let a = shape();
    let b = shape();
    let mut ledger = TextureLedger::default();
    //Act
    plan(&mut ledger, &frame_images(&[&a]), 1);
    let frame2 = plan(&mut ledger, &frame_images(&[&b]), 2);
    //Assert
    assert_eq!((frame2.image_uploads, frame2.image_upload_bytes, frame2.color_table_uploads), (1, 560 * 421 * 4, 0));
}

#[test]
fn two_colourings_of_one_map_in_one_frame_take_two_textures_once_and_then_upload_nothing() {
    //Arrange
    let a = shape();
    let b = a.with_colors(&[Color::from_rgb(1, 2, 3)]).unwrap();
    let mut ledger = TextureLedger::default();
    //Act
    let frame1 = plan(&mut ledger, &frame_images(&[&a, &b]), 1);
    let frame2 = plan(&mut ledger, &frame_images(&[&a, &b]), 2);
    //Assert
    assert_eq!(frame1.image_uploads, 2, "the frame's writes all land before its draws: two textures");
    assert_eq!((frame2.image_uploads, frame2.color_table_uploads), (0, 0));
    assert_eq!(ledger.textures(), 2);
}

#[test]
fn going_back_to_the_pictures_own_colours_rewrites_only_the_table() {
    //Arrange
    let a = shape();
    let b = a.with_colors(&[Color::from_rgb(1, 2, 3)]).unwrap();
    let mut ledger = TextureLedger::default();
    //Act
    plan(&mut ledger, &frame_images(&[&a]), 1);
    plan(&mut ledger, &frame_images(&[&b]), 2);
    let back = plan(&mut ledger, &frame_images(&[&a]), 3);
    //Assert
    assert_eq!((back.image_uploads, back.color_table_uploads, back.color_table_upload_bytes), (0, 1, 560 * 4));
}

#[test]
fn the_ledger_keeps_a_texture_while_it_is_drawn_and_drops_it_after_the_keep_frames() {
    //Arrange
    let mut ledger = TextureLedger::default();
    let key = (7, 4);
    //Act
    let first = ledger.acquire(key, None, 1);
    let again = ledger.acquire(key, None, 50);
    let kept = ledger.evict(100, 60);
    let gone = ledger.evict(200, 60);
    //Assert
    assert_eq!(first, (0, Upload::Whole));
    assert_eq!(again, (0, Upload::None));
    assert!(kept.is_empty(), "used 50 frames ago: kept");
    assert_eq!(gone, vec![(key, 0)]);
    assert_eq!(ledger.textures(), 0);
}

#[test]
fn with_colors_checks_the_colours_and_shares_the_picture() {
    //Arrange
    let a = shape();
    //Act
    let b = a.with_colors(&[Color::from_rgb(0, 0, 255)]).unwrap();
    //Assert
    assert!(a.with_colors(&[]).is_err());
    assert_eq!(a.picture.image_data().data.id(), b.picture.image_data().data.id(), "one picture of owner numbers");
    assert_eq!(b.palette.as_ref().unwrap().size(), (560, 1));
    assert_eq!(b.representative.b, 255, "the flat look follows the new colours");
    assert_eq!(b.destination(), a.destination());
    assert!(a.with_float_colors(&[[1.0, 0.0, 0.0, 1.0]]).unwrap().palette.is_some());
    assert!(a.with_destination(RectangleF::new(0.0, 0.0, 0.0, 1.0)).is_err());
}
