//! [`WaterMask`]: the shape of a [`crate::WaterSurface`] inside its rectangle - a coverage picture
//! laid over it (a round pool, an irregular pond) - and [`WaterCoverage`], the CPU grid one is
//! made from (an ellipse, an outline polygon, a function), which also answers the surface's CPU
//! questions (Not in C#).

use std::sync::Arc;

use glam::Vec2;

use crate::context::{GraphicsContext, GraphicsException, Texture2DOptions};
use crate::water::water_math::mask_opacity;

/// Where a [`crate::WaterSurface`] is water (Not in C#): a coverage picture laid over the
/// surface's rectangle as a [`crate::WaterField`] is - texel column 0, row 0 at the corner of
/// least local X and Z, columns along local X, rows along local Z, stretched to the size - read
/// from its RED channel (0 no water, 1 water) with a linear filter. Where the coverage is at or
/// below `threshold` nothing is drawn (no colour, no depth); above it the water fades in smoothly
/// over the scene behind it (as the soft shore does), reaching its full look at coverage 1. The
/// depth, refraction and reflection work as without a mask; the wave field and the open-water
/// waves run under the whole rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterMask {
    /// The coverage picture: any filterable float view (`R8Unorm` from
    /// [`GraphicsContext::create_water_mask`], or an application's `Rgba8Unorm` picture; a view
    /// compares by identity). Its first mip level is read.
    pub view: wgpu::TextureView,
    /// At or below this coverage nothing is drawn; kept to 0..0.99. 0
    pub threshold: f32,
    /// The same coverage on the CPU, for the surface's CPU answers
    /// ([`crate::WaterSurface::opacity_at`], [`crate::WaterSurface::field_point`]); None (an
    /// application's picture) counts the whole rectangle as water there.
    pub coverage: Option<Arc<WaterCoverage>>,
}

impl WaterMask {
    /// An application's coverage picture (red channel), threshold 0, no CPU coverage.
    pub fn of_view(view: &wgpu::TextureView) -> WaterMask {
        WaterMask { view: view.clone(), threshold: 0.0, coverage: None }
    }

    /// The threshold as the shader uses it.
    pub(crate) fn shader_threshold(&self) -> f32 {
        shader_threshold(self.threshold)
    }

    /// The water's opacity at the mask point `uv` (0..1 over the rectangle) from the CPU coverage.
    pub(crate) fn opacity_at(&self, uv: Vec2) -> f32 {
        coverage_opacity(self.coverage.as_deref(), self.threshold, uv)
    }
}

/// A mask's threshold as the shader uses it: 0..0.99, 0 for NaN.
pub(crate) fn shader_threshold(threshold: f32) -> f32 {
    if threshold.is_nan() {
        0.0
    } else {
        threshold.clamp(0.0, 0.99)
    }
}

/// The water's opacity at the mask point `uv` from a mask's CPU coverage and threshold (the
/// shader's read and edge fade): 0 where nothing is drawn, the fade, 1 inside; 1 without coverage.
pub(crate) fn coverage_opacity(coverage: Option<&WaterCoverage>, threshold: f32, uv: Vec2) -> f32 {
    match coverage {
        Some(coverage) => mask_opacity(coverage.sample(uv), shader_threshold(threshold)).unwrap_or(0.0),
        None => 1.0,
    }
}

/// A water surface's coverage on the CPU (Not in C#): `columns` x `rows` cells over its rectangle,
/// cell (column, row) at `row * columns + column`, row 0 along the rectangle's edge of least local
/// Z, column 0 along its edge of least local X; 0 no water to 255 full water - the bytes the GPU
/// picture of [`GraphicsContext::create_water_mask`] holds, so [`WaterCoverage::sample`] is the
/// shader's read. Coordinates `0..1` below run across the columns (u, local X) and the rows (v,
/// local Z); [`crate::WaterSurface::to_mask`] maps a world point there. Distances of the edge's
/// fade are in CELLS: make the cells square on the surface (columns : rows as the size's x : z)
/// for an even edge.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterCoverage {
    columns: u32,
    rows: u32,
    cells: Vec<u8>,
}

impl WaterCoverage {
    /// The most cells per side.
    pub const MAX_CELLS: u32 = 2048;

    /// Water everywhere (1 to [`WaterCoverage::MAX_CELLS`] cells a side).
    pub fn new(columns: u32, rows: u32) -> WaterCoverage {
        let (columns, rows) = (columns.clamp(1, WaterCoverage::MAX_CELLS), rows.clamp(1, WaterCoverage::MAX_CELLS));
        WaterCoverage { columns, rows, cells: vec![255; columns as usize * rows as usize] }
    }

    /// From a function of the cell CENTRE in 0..1 coordinates giving the coverage 0..1 (clamped;
    /// NaN is 0).
    pub fn from_fn(columns: u32, rows: u32, coverage: impl Fn(f32, f32) -> f32) -> WaterCoverage {
        let mut grid = WaterCoverage::new(columns, rows);
        for row in 0..grid.rows {
            for column in 0..grid.columns {
                let (u, v) = ((column as f32 + 0.5) / grid.columns as f32, (row as f32 + 0.5) / grid.rows as f32);
                grid.set(column, row, coverage(u, v));
            }
        }
        grid
    }

    /// The largest ellipse in the rectangle (a round pool on a square surface): full water
    /// inside, fading to none across `feather` cells centred on its outline, the fade's outer end
    /// at the rectangle's edges (0: a hard edge, one cell's filtering).
    pub fn ellipse(columns: u32, rows: u32, feather: f32) -> WaterCoverage {
        let feather = feather.max(0.0);
        let (c, r) = (columns.clamp(1, WaterCoverage::MAX_CELLS) as f32, rows.clamp(1, WaterCoverage::MAX_CELLS) as f32);
        let radii = (Vec2::new(c, r) * 0.5 - Vec2::splat(feather * 0.5)).max(Vec2::splat(0.5));
        WaterCoverage::from_fn(columns, rows, |u, v| {
            let p = Vec2::new(u * c, v * r) - Vec2::new(c, r) * 0.5;
            edge_coverage(ellipse_distance(p, radii), feather)
        })
    }

    /// The inside of an outline (a pond): `outline` its corners in 0..1 coordinates, in order
    /// (either way round; closed back to the first; crossing outlines count by the even-odd
    /// rule), full water inside, fading to none across `feather` cells centred on the outline
    /// (0: a hard edge). Fewer than three corners: no water.
    pub fn polygon(columns: u32, rows: u32, outline: &[Vec2], feather: f32) -> WaterCoverage {
        let feather = feather.max(0.0);
        let (c, r) = (columns.clamp(1, WaterCoverage::MAX_CELLS) as f32, rows.clamp(1, WaterCoverage::MAX_CELLS) as f32);
        let corners: Vec<Vec2> = outline.iter().map(|p| *p * Vec2::new(c, r)).collect();
        WaterCoverage::from_fn(columns, rows, |u, v| {
            if corners.len() < 3 {
                return 0.0;
            }
            edge_coverage(polygon_distance(Vec2::new(u * c, v * r), &corners), feather)
        })
    }

    /// Cells across.
    pub fn columns(&self) -> u32 {
        self.columns
    }

    /// Cells down.
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Every cell, row by row (0..255).
    pub fn cells(&self) -> &[u8] {
        &self.cells
    }

    /// One cell's coverage, 0..1 (0 outside the grid).
    pub fn get(&self, column: u32, row: u32) -> f32 {
        if column < self.columns && row < self.rows {
            self.cells[(row * self.columns + column) as usize] as f32 / 255.0
        } else {
            0.0
        }
    }

    /// Sets one cell's coverage (0..1, clamped, stored as the byte the picture holds; NaN is 0);
    /// cells outside the grid are ignored.
    pub fn set(&mut self, column: u32, row: u32, coverage: f32) {
        if column < self.columns && row < self.rows {
            let value = if coverage.is_nan() { 0.0 } else { coverage.clamp(0.0, 1.0) };
            self.cells[(row * self.columns + column) as usize] = (value * 255.0).round() as u8;
        }
    }

    /// The coverage at `uv` (0..1 coordinates) as the shader reads the picture: bilinear between
    /// the cell centres, the edge cells held beyond them (clamp to edge).
    pub fn sample(&self, uv: Vec2) -> f32 {
        let (width, height) = (self.columns as f32, self.rows as f32);
        let x = (uv.x * width - 0.5).clamp(0.0, width - 1.0);
        let y = (uv.y * height - 0.5).clamp(0.0, height - 1.0);
        if x.is_nan() || y.is_nan() {
            return 0.0;
        }
        let (x0, y0) = (x.floor() as u32, y.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(self.columns - 1), (y0 + 1).min(self.rows - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let top = self.get(x0, y0) + (self.get(x1, y0) - self.get(x0, y0)) * fx;
        let bottom = self.get(x0, y1) + (self.get(x1, y1) - self.get(x0, y1)) * fx;
        top + (bottom - top) * fy
    }
}

/// The coverage at a signed distance (cells; negative inside) from an outline faded over
/// `feather` cells: 1 inside, 0.5 on the outline, 0 outside.
pub(crate) fn edge_coverage(distance: f32, feather: f32) -> f32 {
    if feather <= 0.0 {
        return if distance <= 0.0 { 1.0 } else { 0.0 };
    }
    (0.5 - distance / feather).clamp(0.0, 1.0)
}

/// The signed distance (negative inside) from `p` to the ellipse of semi-axes `radii` about the
/// origin: exact on the axes and for a circle, a close estimate between (no square root of a
/// quartic; enough for an edge's fade).
pub(crate) fn ellipse_distance(p: Vec2, radii: Vec2) -> f32 {
    let k0 = (p / radii).length();
    let k1 = (p / (radii * radii)).length();
    if k1 <= 1e-12 {
        return -radii.min_element();
    }
    k0 * (k0 - 1.0) / k1
}

/// The signed distance (negative inside, by the even-odd rule) from `p` to a closed outline.
pub(crate) fn polygon_distance(p: Vec2, corners: &[Vec2]) -> f32 {
    let mut nearest = f32::MAX;
    let mut inside = false;
    let mut previous = corners[corners.len() - 1];
    for &corner in corners {
        let edge = corner - previous;
        let t = ((p - previous).dot(edge) / edge.length_squared().max(1e-12)).clamp(0.0, 1.0);
        nearest = nearest.min((previous + edge * t).distance(p));
        if (corner.y > p.y) != (previous.y > p.y) && p.x < previous.x + (p.y - previous.y) / (corner.y - previous.y) * edge.x {
            inside = !inside;
        }
        previous = corner;
    }
    if inside {
        -nearest
    } else {
        nearest
    }
}

impl GraphicsContext {
    /// The GPU picture of a [`WaterCoverage`] as a [`WaterMask`] (Not in C#): an `R8Unorm`
    /// texture of its cells, threshold 0, the coverage kept for the surface's CPU answers.
    pub fn create_water_mask(&self, coverage: &WaterCoverage) -> Result<WaterMask, GraphicsException> {
        let texture = self.create_texture_2d(coverage.columns, coverage.rows, wgpu::TextureFormat::R8Unorm, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, Texture2DOptions { mip_level_count: 1, label: Some("water mask".to_string()) })?;
        texture.write(&coverage.cells)?;
        Ok(WaterMask { view: texture.view().clone(), threshold: 0.0, coverage: Some(Arc::new(coverage.clone())) })
    }
}
