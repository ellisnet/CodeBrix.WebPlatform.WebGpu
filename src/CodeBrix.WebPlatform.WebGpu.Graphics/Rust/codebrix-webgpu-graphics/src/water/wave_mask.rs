//! [`WaveMask`]: where a wave field is not open water - shores, islands, shallows (Not in C#).

use codebrix_webgpu::DecodedImage;
use glam::Vec2;

use crate::context::GraphicsException;

/// Where a wave field is NOT open water (Not in C#): per cell 0 = water, 1 = solid (land: the
/// water is held at 0 there and waves reflect off it exactly as off a wall; values from 0.99
/// count as solid), anything between = shallows that damp waves (an absorbing shore: a ramp of a
/// few cells from 0 up to 1 makes a beach that swallows waves). Cell (column, row) is stored at
/// `row * columns + column`, row 0 at field y = 0. Coordinates `0..1` below run across the
/// columns (x) and down the rows (y). CPU data, uploaded once by `WaveField::set_mask`; it must
/// have the field's grid.
#[derive(Clone, Debug, PartialEq)]
pub struct WaveMask {
    columns: u32,
    rows: u32,
    cells: Vec<f32>,
}

impl WaveMask {
    /// All water (at least one cell on a side).
    pub fn new(columns: u32, rows: u32) -> WaveMask {
        let (columns, rows) = (columns.max(1), rows.max(1));
        WaveMask { columns, rows, cells: vec![0.0; columns as usize * rows as usize] }
    }

    /// From a function of the cell CENTRE in 0..1 coordinates, giving 0 (water) to 1 (solid);
    /// values outside are clamped.
    pub fn from_fn(columns: u32, rows: u32, solid: impl Fn(f32, f32) -> f32) -> WaveMask {
        let mut mask = WaveMask::new(columns, rows);
        for row in 0..mask.rows {
            for column in 0..mask.columns {
                let (u, v) = ((column as f32 + 0.5) / mask.columns as f32, (row as f32 + 0.5) / mask.rows as f32);
                mask.set(column, row, solid(u, v));
            }
        }
        mask
    }

    /// From a decoded picture (`codebrix_webgpu::decode_png` and friends): `channel` 0 to 3 (3 =
    /// alpha), 0 to 255 read as 0 to 1, resampled bilinearly to the grid (the picture's top row
    /// is row 0). Err for a channel above 3 or a picture whose bytes do not match its size.
    pub fn from_picture(picture: &DecodedImage, channel: usize, columns: u32, rows: u32) -> Result<WaveMask, GraphicsException> {
        if channel > 3 {
            return Err(GraphicsException::new(format!("A mask is read from channel 0 (red), 1 (green), 2 (blue) or 3 (alpha); channel {channel} does not exist.")));
        }
        let (width, height) = (picture.width as usize, picture.height as usize);
        if width == 0 || height == 0 || picture.rgba.len() != width * height * 4 {
            return Err(GraphicsException::new(format!("The picture is {} x {} pixels but has {} bytes (RGBA needs {}).", picture.width, picture.height, picture.rgba.len(), width * height * 4)));
        }
        let texel = |x: usize, y: usize| picture.rgba[(y * width + x) * 4 + channel] as f32 / 255.0;
        Ok(WaveMask::from_fn(columns, rows, |u, v| {
            let (x, y) = ((u * width as f32 - 0.5).max(0.0), (v * height as f32 - 0.5).max(0.0));
            let (x0, y0) = ((x.floor() as usize).min(width - 1), (y.floor() as usize).min(height - 1));
            let (x1, y1) = ((x0 + 1).min(width - 1), (y0 + 1).min(height - 1));
            let (fx, fy) = (x - x0 as f32, y - y0 as f32);
            let top = texel(x0, y0) + (texel(x1, y0) - texel(x0, y0)) * fx;
            let bottom = texel(x0, y1) + (texel(x1, y1) - texel(x0, y1)) * fx;
            top + (bottom - top) * fy
        }))
    }

    /// Cells across.
    pub fn columns(&self) -> u32 {
        self.columns
    }

    /// Cells down.
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Every cell, row by row.
    pub fn cells(&self) -> &[f32] {
        &self.cells
    }

    /// The value of one cell (0 outside the grid).
    pub fn get(&self, column: u32, row: u32) -> f32 {
        if column < self.columns && row < self.rows {
            self.cells[(row * self.columns + column) as usize]
        } else {
            0.0
        }
    }

    /// Sets one cell to `solid` (clamped to 0..1); cells outside the grid are ignored.
    pub fn set(&mut self, column: u32, row: u32, solid: f32) {
        if column < self.columns && row < self.rows {
            self.cells[(row * self.columns + column) as usize] = if solid.is_nan() { 0.0 } else { solid.clamp(0.0, 1.0) };
        }
    }

    /// Raises every cell whose centre lies within `radius` of `center` (both in 0..1 coordinates,
    /// so the disc is round on a square field and stretched with a field that is not) to at
    /// least `solid`.
    pub fn add_disc(&mut self, center: Vec2, radius: f32, solid: f32) {
        for row in 0..self.rows {
            for column in 0..self.columns {
                let p = Vec2::new((column as f32 + 0.5) / self.columns as f32, (row as f32 + 0.5) / self.rows as f32);
                if p.distance(center) <= radius {
                    let value = self.get(column, row).max(solid);
                    self.set(column, row, value);
                }
            }
        }
    }
}
