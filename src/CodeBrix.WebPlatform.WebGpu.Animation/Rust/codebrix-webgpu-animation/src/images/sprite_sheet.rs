//! `SpriteSheet`: the frames of a sprite (C# `SpriteSheet`).

use crate::geometry::Rectangle;
use crate::images::ImageSource;

/// The frames of a sprite animation, in order, for a
/// [`SpriteShape`](crate::shapes::SpriteShape) (C# `SpriteSheet`): cut from one sheet picture as a
/// grid ([`SpriteSheet::from_grid`]) or as listed rectangles ([`SpriteSheet::from_rectangles`]),
/// or given as separate pictures ([`SpriteSheet::new`]). Cutting COPIES each cell into a frame of
/// its own, so neighbouring cells never bleed into each other however a frame is scaled or
/// sampled. A clone shares the frames (C# shares one reference); there is nothing to dispose.
#[derive(Clone, Debug, PartialEq)]
pub struct SpriteSheet {
    frames: Vec<ImageSource>,
}

impl SpriteSheet {
    /// A sheet of separate pictures, in order; at least one (C# `new SpriteSheet(frames)`).
    pub fn new(frames: Vec<ImageSource>) -> Result<SpriteSheet, String> {
        if frames.is_empty() {
            return Err("A sprite sheet needs at least one frame.".to_string());
        }
        Ok(SpriteSheet { frames })
    }

    /// Cuts a sheet picture into a grid of equal cells, read left to right then top to bottom
    /// (C# `FromGrid(image, columns, rows, frameCount = 0)`). The cell size is the picture's size
    /// divided by the columns and rows (remainder pixels at the right and bottom are not used);
    /// `frame_count` 0 takes every cell. The error says what does not fit.
    pub fn from_grid(image: &ImageSource, columns: u32, rows: u32, frame_count: u32) -> Result<SpriteSheet, String> {
        if columns < 1 || columns > image.width() {
            return Err(format!("A grid of {columns} columns does not fit a picture {} pixels wide: use 1 to {} columns.", image.width(), image.width()));
        }
        if rows < 1 || rows > image.height() {
            return Err(format!("A grid of {rows} rows does not fit a picture {} pixels high: use 1 to {} rows.", image.height(), image.height()));
        }
        let cells = columns * rows;
        if frame_count > cells {
            return Err(format!("A {columns} x {rows} grid has {cells} cells, fewer than the {frame_count} frames asked for: pass 0 (all cells) or at most {cells}."));
        }
        let count = if frame_count == 0 { cells } else { frame_count };
        let (cw, ch) = ((image.width() / columns) as i32, (image.height() / rows) as i32);
        let rectangles: Vec<Rectangle> = (0..count).map(|i| Rectangle::new((i % columns) as i32 * cw, (i / columns) as i32 * ch, cw, ch)).collect();
        SpriteSheet::from_rectangles(image, &rectangles)
    }

    /// Cuts listed rectangles out of a sheet picture, in order (C# `FromRectangles`).
    pub fn from_rectangles(image: &ImageSource, cells: &[Rectangle]) -> Result<SpriteSheet, String> {
        if cells.is_empty() {
            return Err("A sprite sheet needs at least one frame: list at least one rectangle.".to_string());
        }
        let frames = cells.iter().map(|c| image.crop(*c)).collect::<Result<Vec<_>, String>>()?;
        Ok(SpriteSheet { frames })
    }

    /// The number of frames (C# `FrameCount`).
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// A frame by index, 0 to `frame_count() - 1` (C# `GetFrame`); None outside.
    pub fn get_frame(&self, index: usize) -> Option<&ImageSource> {
        self.frames.get(index)
    }
}
