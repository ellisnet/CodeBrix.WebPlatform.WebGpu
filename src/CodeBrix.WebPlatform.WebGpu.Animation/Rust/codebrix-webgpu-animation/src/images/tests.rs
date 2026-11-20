//! Host tests of the pictures: buffers, crops, grids and the sprite frame arithmetic of C#'s
//! `SpriteShape.GetFrameAt` / `MapFrame`.

use crate::geometry::Rectangle;
use crate::images::{ImageSource, SpritePlayback, SpriteSheet};

fn numbered(width: u32, height: u32) -> ImageSource {
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&[x as u8, y as u8, 7, 255]);
        }
    }
    ImageSource::from_rgba(&rgba, width, height).unwrap()
}

#[test]
fn a_picture_checks_its_buffer_and_swaps_bgra() {
    //Arrange
    let short = ImageSource::from_rgba(&[0; 15], 2, 2);
    let bgra = ImageSource::from_bgra(&[1, 2, 3, 4], 1, 1).unwrap();
    //Act
    let rgba = bgra.to_rgba();
    //Assert
    assert!(short.unwrap_err().contains("needs 16"));
    assert!(ImageSource::from_rgba(&[], 0, 1).is_err());
    assert_eq!(rgba, vec![3, 2, 1, 4]);
}

#[test]
fn a_crop_copies_the_area_and_refuses_one_outside() {
    //Arrange
    let image = numbered(8, 4);
    //Act
    let part = image.crop(Rectangle::new(2, 1, 3, 2)).unwrap();
    //Assert
    assert_eq!(part.size(), (3, 2));
    assert_eq!(&part.to_rgba()[..4], &[2, 1, 7, 255]);
    assert_eq!(&part.to_rgba()[20..24], &[4, 2, 7, 255]);
    assert!(image.crop(Rectangle::new(6, 0, 3, 1)).is_err());
    assert!(image.crop(Rectangle::new(0, 0, 0, 1)).is_err());
}

#[test]
fn a_grid_cuts_cells_left_to_right_then_down_and_drops_remainders() {
    //Arrange
    let image = numbered(10, 5);
    //Act
    let sheet = SpriteSheet::from_grid(&image, 3, 2, 5).unwrap();
    //Assert
    assert_eq!(sheet.frame_count(), 5);
    assert_eq!(sheet.get_frame(0).unwrap().size(), (3, 2));
    assert_eq!(&sheet.get_frame(4).unwrap().to_rgba()[..4], &[3, 2, 7, 255]);
    assert!(SpriteSheet::from_grid(&image, 3, 2, 7).is_err());
    assert!(SpriteSheet::from_grid(&image, 11, 1, 0).is_err());
    assert_eq!(SpriteSheet::from_grid(&image, 2, 1, 0).unwrap().frame_count(), 2);
}

#[test]
fn the_playback_modes_map_frame_numbers_as_csharp_does() {
    //Arrange
    let n: Vec<i64> = (-3..9).collect();
    //Act
    let looped: Vec<usize> = n.iter().map(|n| SpritePlayback::Loop.map_frame(*n, 3)).collect();
    let ping: Vec<usize> = n.iter().map(|n| SpritePlayback::PingPong.map_frame(*n, 3)).collect();
    let hold: Vec<usize> = n.iter().map(|n| SpritePlayback::Hold.map_frame(*n, 3)).collect();
    //Assert
    // C#: ((n % count) + count) % count; ping-pong period 2 * count - 2; hold clamps.
    assert_eq!(looped, vec![0, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2]);
    assert_eq!(ping, vec![1, 2, 1, 0, 1, 2, 1, 0, 1, 2, 1, 0]);
    assert_eq!(hold, vec![0, 0, 0, 0, 1, 2, 2, 2, 2, 2, 2, 2]);
    assert_eq!(SpritePlayback::PingPong.map_frame(5, 1), 0);
}
