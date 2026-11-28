//! Host tests of the live picture: its size and stand-in, writes of the whole picture and of
//! regions and their refusals, the CPU reads of the stand-in (also after the live picture is
//! dropped), the queue of regions per texture (merging, bounding, the whole picture) and the
//! textures per device (the first use sends the whole picture, every texture gets every write, the
//! least recently drawn goes), and a stand-in drawn through the picture path (the counts are
//! tested in rendering/core/texture_ledger.rs).

use crate::geometry::{Rectangle, RectangleF};
use crate::images::live_picture::{DeviceTextures, Pending, MAX_DEVICES, MAX_PENDING_REGIONS};
use crate::images::{ImageSampling, ImageSource, LivePicture, PixelRegion, SpriteSheet};
use crate::rendering::{TestBuilder as Builder, VectorScene};
use crate::shapes::{ImageShape, ImageShapeOptions};
use crate::{Color, DrawingSession, DrawingSessionOptions};

fn region(x: u32, y: u32, width: u32, height: u32) -> PixelRegion {
    PixelRegion { x, y, width, height }
}

fn solid(width: u32, height: u32, px: [u8; 4]) -> Vec<u8> {
    px.iter().copied().cycle().take(width as usize * height as usize * 4).collect()
}

#[test]
fn a_live_picture_checks_its_size_and_starts_transparent() {
    //Arrange
    let live = LivePicture::new(3, 2).unwrap();
    //Act
    let pixels = live.to_rgba();
    //Assert
    assert_eq!(live.size(), (3, 2));
    assert_eq!(pixels, vec![0; 24]);
    assert!(LivePicture::new(0, 4).unwrap_err().contains("1 to 8192"));
    assert!(LivePicture::new(4, LivePicture::MAX_SIZE + 1).is_err());
    assert!(LivePicture::new(LivePicture::MAX_SIZE, 1).is_ok());
}

#[test]
fn its_stand_in_is_a_picture_of_its_size_in_a_decoded_pictures_format() {
    //Arrange
    let live = LivePicture::new(16, 8).unwrap();
    let decoded = ImageSource::from_rgba(&solid(16, 8, [1, 2, 3, 4]), 16, 8).unwrap();
    //Act
    let a = live.image_source();
    let b = live.image_source();
    //Assert: straight-alpha RGBA as a decoded picture, so it draws the same way.
    assert_eq!(a.size(), (16, 8));
    assert_eq!(a.image_data().format, decoded.image_data().format);
    assert_eq!(a.image_data().alpha_type, decoded.image_data().alpha_type);
    assert_eq!(a, b, "one stand-in");
    assert_eq!(LivePicture::find(a.image_data().data.id()), Some(live.clone()));
    assert_eq!(live, live.clone());
    assert_ne!(live, LivePicture::new(16, 8).unwrap());
}

#[test]
fn writes_change_the_pixels_in_place_and_the_stand_in_reads_them() {
    //Arrange
    let live = LivePicture::new(4, 3).unwrap();
    let stand_in = live.image_source();
    //Act
    live.write(&solid(4, 3, [10, 20, 30, 255])).unwrap();
    live.write_region(Rectangle::new(1, 1, 2, 2), &solid(2, 2, [200, 100, 50, 128])).unwrap();
    //Assert
    let pixels = stand_in.to_rgba();
    assert_eq!(pixels.len(), 48);
    assert_eq!(&pixels[0..4], &[10, 20, 30, 255]);
    assert_eq!(&pixels[(4 + 1) * 4..(4 + 1) * 4 + 4], &[200, 100, 50, 128], "(1, 1)");
    assert_eq!(&pixels[(2 * 4 + 2) * 4..(2 * 4 + 2) * 4 + 4], &[200, 100, 50, 128], "(2, 2)");
    assert_eq!(&pixels[(2 * 4 + 3) * 4..(2 * 4 + 3) * 4 + 4], &[10, 20, 30, 255], "(3, 2) untouched");
    assert_eq!(pixels, live.to_rgba());
    // A crop of the stand-in copies the live pixels now (a sprite sheet of it is a copy).
    let part = stand_in.crop(Rectangle::new(1, 1, 1, 1)).unwrap();
    assert_eq!(part.to_rgba(), vec![200, 100, 50, 128]);
    let sheet = SpriteSheet::from_rectangles(&stand_in, &[Rectangle::new(1, 1, 1, 1)]).unwrap();
    live.write(&solid(4, 3, [0, 0, 0, 0])).unwrap();
    assert_eq!(sheet.get_frame(0).unwrap().to_rgba(), vec![200, 100, 50, 128], "the copy stays");
    assert_eq!(part.to_rgba(), vec![200, 100, 50, 128]);
}

#[test]
fn several_writes_in_one_frame_end_as_the_last_where_they_overlap() {
    //Arrange
    let live = LivePicture::new(4, 1).unwrap();
    //Act
    live.write_region(Rectangle::new(0, 0, 3, 1), &solid(3, 1, [1, 1, 1, 1])).unwrap();
    live.write_region(Rectangle::new(2, 0, 2, 1), &solid(2, 1, [2, 2, 2, 2])).unwrap();
    //Assert
    assert_eq!(live.to_rgba(), vec![1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2]);
}

#[test]
fn a_region_outside_the_picture_or_a_short_buffer_is_refused_and_nothing_is_written() {
    //Arrange
    let live = LivePicture::new(4, 4).unwrap();
    let full = solid(4, 4, [9, 9, 9, 9]);
    //Act
    let outside = live.write_region(Rectangle::new(3, 0, 2, 1), &full);
    let negative = live.write_region(Rectangle::new(-1, 0, 1, 1), &full);
    let empty = live.write_region(Rectangle::new(0, 0, 0, 1), &full);
    let huge = live.write_region(Rectangle::new(i32::MAX, i32::MAX, i32::MAX, i32::MAX), &full);
    let short = live.write_region(Rectangle::new(0, 0, 2, 2), &[0; 15]);
    let short_whole = live.write(&[0; 63]);
    //Assert
    assert!(outside.unwrap_err().contains("lie inside the 4 x 4 live picture"));
    assert!(negative.is_err());
    assert!(empty.unwrap_err().contains("at least 1 x 1"));
    assert!(huge.is_err(), "no overflow");
    assert!(short.unwrap_err().contains("needs 16"));
    assert!(short_whole.unwrap_err().contains("needs 64"));
    assert_eq!(live.to_rgba(), vec![0; 64], "nothing written");
}

#[test]
fn a_stand_in_whose_live_picture_is_gone_reads_as_transparent() {
    //Arrange
    let live = LivePicture::new(2, 2).unwrap();
    live.write(&solid(2, 2, [255, 255, 255, 255])).unwrap();
    let stand_in = live.image_source();
    let id = stand_in.image_data().data.id();
    //Act
    drop(live);
    //Assert
    assert_eq!(LivePicture::find(id), None, "gone from the registry");
    assert_eq!(stand_in.to_rgba(), vec![0; 16]);
    assert_eq!(stand_in.crop(Rectangle::new(1, 1, 1, 1)).unwrap().to_rgba(), vec![0; 4]);
}

#[test]
fn queued_regions_merge_and_a_write_of_everything_is_the_whole_picture() {
    //Arrange
    let size = (100, 50);
    let mut pending = Pending::Nothing;
    //Act
    pending.add(region(10, 10, 20, 20), size);
    pending.add(region(15, 15, 5, 5), size);
    let inside = pending.clone();
    pending.add(region(0, 0, 40, 40), size);
    let swallowed = pending.clone();
    pending.add(region(0, 0, 100, 50), size);
    let whole = pending.clone();
    pending.add(region(1, 1, 1, 1), size);
    //Assert
    assert_eq!(inside, Pending::Regions(vec![region(10, 10, 20, 20)]), "a region inside a queued one adds nothing");
    assert_eq!(swallowed, Pending::Regions(vec![region(0, 0, 40, 40)]), "a region holding queued ones replaces them");
    assert_eq!(whole, Pending::Whole);
    assert_eq!(pending, Pending::Whole, "nothing to add to the whole picture");
    assert_eq!(pending.take(size), vec![region(0, 0, 100, 50)]);
    assert_eq!(pending, Pending::Nothing);
    assert!(pending.take(size).is_empty());
}

#[test]
fn too_many_separate_regions_are_sent_as_their_bounding_region() {
    //Arrange
    let size = (1000, 10);
    let mut pending = Pending::Nothing;
    for i in 0..MAX_PENDING_REGIONS as u32 {
        pending.add(region(10 + i * 10, 2, 5, 3), size);
    }
    let kept = pending.clone();
    //Act
    pending.add(region(500, 1, 5, 5), size);
    //Assert
    assert!(matches!(&kept, Pending::Regions(l) if l.len() == MAX_PENDING_REGIONS));
    assert_eq!(pending.take(size), vec![region(10, 1, 495, 5)]);
}

#[test]
fn the_first_use_on_a_device_sends_the_whole_picture_and_later_ones_only_the_writes() {
    //Arrange: devices are numbers here.
    let size = (64, 64);
    let mut textures: DeviceTextures<u32> = DeviceTextures::default();
    // A write before the first draw only changes the pixels.
    textures.written(region(0, 0, 8, 8), size);
    //Act
    let (_, first) = textures.use_on(|d| *d == 1, || 1, size);
    let first = first.clone();
    let (_, nothing) = textures.use_on(|d| *d == 1, || unreachable!(), size);
    let nothing = nothing.clone();
    textures.written(region(4, 4, 2, 2), size);
    textures.written(region(30, 30, 1, 1), size);
    let (_, writes) = textures.use_on(|d| *d == 1, || unreachable!(), size);
    let writes = writes.clone();
    //Assert
    assert_eq!(first, vec![region(0, 0, 64, 64)]);
    assert!(nothing.is_empty(), "drawn again unchanged: nothing sent");
    assert_eq!(writes, vec![region(4, 4, 2, 2), region(30, 30, 1, 1)]);
    assert_eq!(textures.len(), 1);
}

#[test]
fn every_texture_of_the_picture_gets_every_write() {
    //Arrange: the picture drawn on two devices.
    let size = (32, 32);
    let mut textures: DeviceTextures<u32> = DeviceTextures::default();
    textures.use_on(|d| *d == 1, || 1, size);
    textures.use_on(|d| *d == 2, || 2, size);
    //Act
    textures.written(region(1, 2, 3, 4), size);
    let on_one = textures.use_on(|d| *d == 1, || unreachable!(), size).1;
    let on_two = textures.use_on(|d| *d == 2, || unreachable!(), size).1;
    //Assert
    assert_eq!(on_one, vec![region(1, 2, 3, 4)]);
    assert_eq!(on_two, vec![region(1, 2, 3, 4)], "the second device's texture is not left behind");
}

#[test]
fn a_device_beyond_the_limit_replaces_the_least_recently_drawn_texture() {
    //Arrange
    let size = (8, 8);
    let mut textures: DeviceTextures<u32> = DeviceTextures::default();
    for d in 0..MAX_DEVICES as u32 {
        textures.use_on(|t| *t == d, || d, size);
    }
    // Device 0 drawn again: device 1 is now the least recently drawn.
    textures.use_on(|t| *t == 0, || unreachable!(), size);
    //Act
    textures.use_on(|t| *t == 99, || 99, size);
    let mut made_again = false;
    let again = textures.use_on(|t| *t == 1, || {
        made_again = true;
        1
    }, size).1;
    //Assert
    assert!(made_again, "device 1's texture had gone");
    assert_eq!(again, vec![region(0, 0, 8, 8)]);
    assert_eq!(textures.len(), MAX_DEVICES);
}

#[test]
fn an_image_shape_of_the_stand_in_draws_it_through_the_picture_path() {
    //Arrange
    let live = LivePicture::new(40, 20).unwrap();
    let shape = ImageShape::new(live.image_source(), RectangleF::new(10.0, 10.0, 200.0, 100.0), ImageShapeOptions { source_rectangle: Some(Rectangle::new(0, 0, 20, 20)), sampling: ImageSampling::Linear, tint: None }).unwrap();
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (640, 360), layer_opacity: 255, ..DrawingSessionOptions::default() });
    s.add_layer("Show", Color::WHITE);
    s.draw_shape(shape);
    let mut scene = VectorScene::new();
    s.render(&mut scene, 640, 360);
    let mut b = Builder::default();
    //Act
    b.build(&scene, 640, 360);
    //Assert
    assert_eq!(b.images.len(), 1);
    let image = &b.images[0].brush.image;
    assert_eq!(image.data.id(), live.image_source().image_data().data.id());
    assert_eq!((image.width, image.height), (40, 20));
    assert_eq!(LivePicture::find(image.data.id()), Some(live));
    assert!(ImageShape::new(LivePicture::new(4, 4).unwrap().image_source(), RectangleF::new(0.0, 0.0, 1.0, 1.0), ImageShapeOptions { source_rectangle: Some(Rectangle::new(2, 2, 3, 3)), sampling: ImageSampling::Linear, tint: None }).is_err(), "the stand-in has the live picture's size");
}
