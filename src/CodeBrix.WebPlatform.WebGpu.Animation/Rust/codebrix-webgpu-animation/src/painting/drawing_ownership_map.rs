//! `DrawingOwnershipMap`: which element owns each pixel of a frame (C# `DrawingOwnershipMap`).

use crate::color::Color;
use crate::geometry::Rectangle;
use crate::models::{ElementAxes, ElementId};
use crate::painting::ElementPixel;
use crate::planes::plane_kernels::unit_to_byte;
use crate::planes::{DrawingPlane, DrawingPlaneImage};
use crate::rendering::hard_edge_rasterizer::HardEdgeMatrix;

/// Which element owns each pixel of a frame (C# `DrawingOwnershipMap`) - the painterly tool
/// behind "every brushstroke takes the colour beneath it", bristle textures and dark seams
/// between strokes. Made by
/// [`DrawingSession::export_ownership_map`](crate::DrawingSession::export_ownership_map): a pixel
/// belongs to the LAST element, in painting order, whose area covers the pixel's centre - the
/// hard-edged pixel rule of [`MaskMeaning::HardEdged`](crate::MaskMeaning::HardEdged), computed
/// from geometry on the CPU with C#'s arithmetic. Pixels no element covers hold
/// [`DrawingOwnershipMap::NOBODY`]; [`DrawingOwnershipMap::fill_unowned`] gives them to the nearest
/// owner.
///
/// Elements are numbered 0, 1, 2 ... in painting order across the layers the map was made of
/// (bottom layer first, each layer's elements in order); every element on those layers has a
/// number, owning pixels or not. A group or an animated shape is one element. The map holds the
/// elements' IDS and layer NAMES (not the elements), their maps to the output and their axes as
/// they were when the map was made; it never changes afterwards. Every method that produces
/// numbers does so in C#'s fixed order (sums row by row, left to right), so the results are C#'s.
///
/// Colours as float values are `[f32; 4]` = R, G, B, A (C#'s `Vector4` X, Y, Z, W).
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingOwnershipMap {
    width: u32,
    height: u32,
    owners: Vec<i32>,
    elements: Vec<ElementId>,
    layers: Vec<String>,
    matrices: Vec<HardEdgeMatrix>,
    axes: Vec<ElementAxes>,
    counts: Vec<usize>,
    bounds: Vec<Rectangle>,
    unowned_count: usize,
}

// Takes a pixel centre back into an element's own coordinates and measures it in its axes
struct Measure {
    inverse: Option<HardEdgeMatrix>,
    axes: ElementAxes,
    cos: f64,
    sin: f64,
}

impl Measure {
    fn new(matrix: &HardEdgeMatrix, axes: ElementAxes) -> Measure {
        let radians = axes.angle() as f64 * (std::f64::consts::PI / 180.0);
        Measure { inverse: matrix.try_invert(), axes, cos: radians.cos(), sin: radians.sin() }
    }

    fn at(&self, x: u32, y: u32, owner: usize) -> ElementPixel {
        let Some(inverse) = &self.inverse else {
            return ElementPixel { x, y, element_index: owner, along: 0.0, across: 0.0, axes: self.axes };
        };
        let (lx, ly) = inverse.transform(x as f64 + 0.5, y as f64 + 0.5);
        let (dx, dy) = (lx - self.axes.center().x as f64, ly - self.axes.center().y as f64);
        let along = (dx * self.cos) + (dy * self.sin);
        let across = (dy * self.cos) - (dx * self.sin);
        ElementPixel { x, y, element_index: owner, along, across, axes: self.axes }
    }
}

// For each of `count` output positions, the source index by the planes' nearest rule:
// floor((2i + 1) * source / (2 * count)), in integers
fn nearest_indexes(count: u32, source: u32) -> Vec<usize> {
    (0..count as i64).map(|i| ((((2 * i) + 1) * source as i64) / (2 * count as i64)).min(source as i64 - 1) as usize).collect()
}

impl DrawingOwnershipMap {
    /// The value of a pixel no element owns (C# `Nobody`).
    pub const NOBODY: i32 = -1;

    pub(crate) fn new(width: u32, height: u32, owners: Vec<i32>, elements: Vec<ElementId>, layers: Vec<String>, matrices: Vec<HardEdgeMatrix>, axes: Vec<ElementAxes>) -> DrawingOwnershipMap {
        let n = elements.len();
        let mut counts = vec![0usize; n];
        let (mut left, mut top, mut right, mut bottom) = (vec![i32::MAX; n], vec![i32::MAX; n], vec![i32::MIN; n], vec![i32::MIN; n]);
        let mut unowned = 0;
        let mut i = 0;
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let owner = owners[i];
                i += 1;
                if owner < 0 {
                    unowned += 1;
                    continue;
                }
                let o = owner as usize;
                counts[o] += 1;
                left[o] = left[o].min(x);
                right[o] = right[o].max(x);
                top[o] = top[o].min(y);
                bottom[o] = y;
            }
        }
        let bounds = (0..n).map(|e| if counts[e] == 0 { Rectangle::default() } else { Rectangle::new(left[e], top[e], right[e] - left[e] + 1, bottom[e] - top[e] + 1) }).collect();
        DrawingOwnershipMap { width, height, owners, elements, layers, matrices, axes, counts, bounds, unowned_count: unowned }
    }

    /// The map's width, pixels (C# `Width`).
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The map's height, pixels (C# `Height`).
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The map's size, pixels (C# `Size`).
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// How many elements the map numbers, whether or not they own pixels (C# `ElementCount`).
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// How many pixels no element owns (C# `UnownedCount`).
    pub fn unowned_count(&self) -> usize {
        self.unowned_count
    }

    /// The owner of every pixel, row by row (`[y * width + x]`): an element index, or
    /// [`DrawingOwnershipMap::NOBODY`] (C# `Owners`).
    pub fn owners(&self) -> &[i32] {
        &self.owners
    }

    /// The owner of one pixel: an element index, or NOBODY (C#'s indexer). The error says the
    /// pixel lies outside the map.
    pub fn get(&self, x: u32, y: u32) -> Result<i32, String> {
        if x >= self.width || y >= self.height {
            return Err(format!("The pixel ({x}, {y}) lies outside the {} x {} map.", self.width, self.height));
        }
        Ok(self.owners[(y as usize * self.width as usize) + x as usize])
    }

    /// A copy of the owners (C# `ToArray`).
    pub fn to_array(&self) -> Vec<i32> {
        self.owners.clone()
    }

    fn check_index(&self, index: usize) -> Result<(), String> {
        if index >= self.elements.len() {
            return Err(format!("The element index {index} is out of range: the map numbers {} elements.", self.elements.len()));
        }
        Ok(())
    }

    /// The id of the element with an index (C# `GetElement`, which returns the element itself;
    /// find it on its layer by the id).
    pub fn get_element(&self, index: usize) -> Result<ElementId, String> {
        self.check_index(index)?;
        Ok(self.elements[index])
    }

    /// The name of the layer an element is on (C# `GetLayer`, which returns the layer).
    pub fn get_layer(&self, index: usize) -> Result<&str, String> {
        self.check_index(index)?;
        Ok(&self.layers[index])
    }

    /// The index of an element in the map; None when it is not one of the map's elements (C#
    /// `IndexOf`, which returns -1). A group's children are not elements of the map: the group is.
    pub fn index_of(&self, element: ElementId) -> Option<usize> {
        self.elements.iter().position(|&e| e == element)
    }

    /// How many pixels an element owns (C# `GetPixelCount`).
    pub fn get_pixel_count(&self, index: usize) -> Result<usize, String> {
        self.check_index(index)?;
        Ok(self.counts[index])
    }

    /// The smallest pixel rectangle holding every pixel an element owns; empty for an element
    /// that owns none (C# `GetBounds`).
    pub fn get_bounds(&self, index: usize) -> Result<Rectangle, String> {
        self.check_index(index)?;
        Ok(self.bounds[index])
    }

    /// The axes an element's pixels are measured in (C# `GetAxes`): the shape's own
    /// ([`DrawingShape::get_axes`](crate::shapes::DrawingShape::get_axes)) when it reports them,
    /// else [`ElementAxes::from_bounds`] of its bounds, else all zeros - as they were when the map
    /// was made.
    pub fn get_axes(&self, index: usize) -> Result<ElementAxes, String> {
        self.check_index(index)?;
        Ok(self.axes[index])
    }

    /// A map in which every unowned pixel takes the owner of the NEAREST owned pixel, by exact
    /// Euclidean distance between pixel positions (computed in integers) (C# `FillUnowned`).
    /// TIE RULE: among owned pixels at the same smallest distance, the one with the smallest x
    /// wins, and among those the one with the smallest y. Owned pixels keep their owner. A map
    /// with no owned pixel at all (or none unowned) comes back as an unchanged copy.
    pub fn fill_unowned(&self) -> DrawingOwnershipMap {
        if self.unowned_count == 0 || self.unowned_count == self.owners.len() {
            return self.clone();
        }
        let filled = nearest_fill(&self.owners, self.width as usize, self.height as usize);
        DrawingOwnershipMap::new(self.width, self.height, filled, self.elements.clone(), self.layers.clone(), self.matrices.clone(), self.axes.clone())
    }

    /// A plane of the seams between owners (C# `CreateBoundaryPlane`): 1 on every pixel whose
    /// owner differs from the owner of the pixel to its left or of the pixel above it (the first
    /// column and row compare only with the neighbours they have; NOBODY counts as an owner of
    /// its own), 0 elsewhere - then blurred with
    /// [`DrawingPlane::gaussian_blur`](crate::planes::DrawingPlane::gaussian_blur) when
    /// `blur_sigma` (None: 0) is above 0. Multiply, clamp and blend it for dark outlines between
    /// brushstrokes. The error says what to do with a negative or non-finite sigma.
    pub fn create_boundary_plane(&self, blur_sigma: Option<f32>) -> Result<DrawingPlane, String> {
        let sigma = blur_sigma.unwrap_or(0.0);
        if !sigma.is_finite() || sigma < 0.0 {
            return Err("The boundary blur's sigma must be finite and not negative (0 for hard seams).".to_string());
        }
        let mut plane = DrawingPlane::new(self.width, self.height)?;
        let width = self.width as usize;
        let owners = &self.owners;
        let data = plane.data_mut();
        for y in 0..self.height as usize {
            let row = y * width;
            for x in 0..width {
                let i = row + x;
                let owner = owners[i];
                let seam = (x > 0 && owners[i - 1] != owner) || (y > 0 && owners[i - width] != owner);
                data[i] = if seam { 1.0 } else { 0.0 };
            }
        }
        if sigma > 0.0 {
            plane.gaussian_blur(sigma)?;
        }
        Ok(plane)
    }

    /// The MEAN COLOUR of a source image under each element - colour pickup (C#
    /// `GetMeanColors(DrawingPlaneImage)`). Each map pixel reads ONE source pixel by the nearest
    /// rule (column `floor((2x + 1) * source_width / (2 * width))`, likewise rows - a source at the
    /// map's size is read pixel for pixel, a half-size one at `(x / 2, y / 2)`). Alpha-weighted:
    /// with a = source alpha clamped to 0..1, colour = sum(C x a) / sum(a) and alpha = sum(a) /
    /// pixel count. Sums in double, row by row, left to right. One straight-alpha colour per
    /// element; transparent black for an element that owns no pixel.
    pub fn get_mean_colors(&self, source: &DrawingPlaneImage) -> Vec<[f32; 4]> {
        let columns = nearest_indexes(self.width, source.width());
        let rows = nearest_indexes(self.height, source.height());
        let (r, g, b, a) = (source.r().data(), source.g().data(), source.b().data(), source.a().data());
        let n = self.elements.len();
        let (mut sum_r, mut sum_g, mut sum_b, mut sum_a) = (vec![0.0f64; n], vec![0.0f64; n], vec![0.0f64; n], vec![0.0f64; n]);
        let source_width = source.width() as usize;
        let mut i = 0;
        for y in 0..self.height as usize {
            let source_row = rows[y] * source_width;
            for &column in columns.iter() {
                let owner = self.owners[i];
                i += 1;
                if owner < 0 {
                    continue;
                }
                let o = owner as usize;
                let s = source_row + column;
                let alpha = a[s];
                let weight = if alpha > 0.0 {
                    if alpha >= 1.0 {
                        1.0
                    } else {
                        alpha as f64
                    }
                } else {
                    0.0
                };
                sum_r[o] += r[s] as f64 * weight;
                sum_g[o] += g[s] as f64 * weight;
                sum_b[o] += b[s] as f64 * weight;
                sum_a[o] += weight;
            }
        }
        (0..n)
            .map(|e| {
                let (count, a) = (self.counts[e], sum_a[e]);
                if count == 0 || !(a > 0.0) {
                    return [0.0; 4];
                }
                [(sum_r[e] / a) as f32, (sum_g[e] / a) as f32, (sum_b[e] / a) as f32, (a * 1.0 / count as f64) as f32]
            })
            .collect()
    }

    /// The mean colour of an 8-bit straight-alpha RGBA frame under each element (C#
    /// `GetMeanColors(ReadOnlySpan<byte>, int, int)`; the sampling rule above). The sums are exact
    /// integers: colour = sum(C x A) / (255 x sum(A)) and alpha = sum(A) / (255 x pixel count),
    /// each divided in double. The error says what to do with an unusable size or a short frame.
    pub fn get_mean_colors_bytes(&self, rgba: &[u8], width: u32, height: u32) -> Result<Vec<[f32; 4]>, String> {
        if width < 1 || height < 1 {
            return Err("The frame's width and height must be at least 1.".to_string());
        }
        if (rgba.len() as u64) < width as u64 * height as u64 * 4 {
            return Err("The frame is shorter than width x height x 4 bytes.".to_string());
        }
        let columns = nearest_indexes(self.width, width);
        let rows = nearest_indexes(self.height, height);
        let n = self.elements.len();
        let (mut sum_r, mut sum_g, mut sum_b, mut sum_a) = (vec![0i64; n], vec![0i64; n], vec![0i64; n], vec![0i64; n]);
        let mut i = 0;
        for y in 0..self.height as usize {
            let source_row = rows[y] * width as usize;
            for &column in columns.iter() {
                let owner = self.owners[i];
                i += 1;
                if owner < 0 {
                    continue;
                }
                let o = owner as usize;
                let s = (source_row + column) * 4;
                let alpha = rgba[s + 3] as i64;
                sum_r[o] += rgba[s] as i64 * alpha;
                sum_g[o] += rgba[s + 1] as i64 * alpha;
                sum_b[o] += rgba[s + 2] as i64 * alpha;
                sum_a[o] += alpha;
            }
        }
        Ok((0..n)
            .map(|e| {
                let count = self.counts[e];
                let a = sum_a[e];
                if count == 0 || a == 0 {
                    return [0.0; 4];
                }
                let a255 = 255.0 * a as f64;
                [(sum_r[e] as f64 / a255) as f32, (sum_g[e] as f64 / a255) as f32, (sum_b[e] as f64 / a255) as f32, (a as f64 / (255.0 * count as f64)) as f32]
            })
            .collect())
    }

    fn check_colors(&self, count: usize) -> Result<(), String> {
        if count < self.elements.len() {
            return Err(format!("There must be a colour for every element: {} given, {} needed.", count, self.elements.len()));
        }
        Ok(())
    }

    /// Paints each element's colour into every pixel it owns: R, G, B and A of the destination are
    /// set to the colour (straight alpha); pixels no element owns are left as they are (C#
    /// `ApplyColors(DrawingPlaneImage, ...)`). The error says what to do with an image of another
    /// size or too few colours.
    pub fn apply_colors(&self, destination: &mut DrawingPlaneImage, colors: &[[f32; 4]]) -> Result<(), String> {
        if destination.width() != self.width || destination.height() != self.height {
            return Err("The image must be the map's size.".to_string());
        }
        self.check_colors(colors.len())?;
        for channel in 0..4 {
            let plane = match channel {
                0 => destination.r_mut(),
                1 => destination.g_mut(),
                2 => destination.b_mut(),
                _ => destination.a_mut(),
            };
            let data = plane.data_mut();
            for (value, &owner) in data.iter_mut().zip(self.owners.iter()) {
                if owner >= 0 {
                    *value = colors[owner as usize][channel];
                }
            }
        }
        Ok(())
    }

    /// Paints each element's colour into every pixel it owns in an 8-bit straight-alpha RGBA frame
    /// of the map's size; pixels no element owns are left as they are (C# `ApplyColors(Span<byte>,
    /// IReadOnlyList<Color>)`). The error says what to do with a short frame or too few colours.
    pub fn apply_colors_bytes(&self, rgba: &mut [u8], colors: &[Color]) -> Result<(), String> {
        if (rgba.len() as u64) < self.owners.len() as u64 * 4 {
            return Err("The frame is shorter than width x height x 4 bytes.".to_string());
        }
        self.check_colors(colors.len())?;
        for (i, &owner) in self.owners.iter().enumerate() {
            if owner < 0 {
                continue;
            }
            let c = colors[owner as usize];
            let d = i * 4;
            rgba[d] = c.r;
            rgba[d + 1] = c.g;
            rgba[d + 2] = c.b;
            rgba[d + 3] = c.a;
        }
        Ok(())
    }

    /// Paints each element's FLOAT colour (for example straight from
    /// [`DrawingOwnershipMap::get_mean_colors_bytes`]) into every pixel it owns in an 8-bit
    /// straight-alpha RGBA frame of the map's size, each channel converted by the planes'
    /// float-to-byte rule (NaN or at most 0 gives 0, at least 1 gives 255, otherwise
    /// `(byte)(v * 255 + 0.5)`) (C# `ApplyColors(Span<byte>, IReadOnlyList<Vector4>)`).
    pub fn apply_float_colors_bytes(&self, rgba: &mut [u8], colors: &[[f32; 4]]) -> Result<(), String> {
        self.check_colors(colors.len())?;
        let table: Vec<Color> = colors[..self.elements.len()].iter().map(|c| Color { r: unit_to_byte(c[0]), g: unit_to_byte(c[1]), b: unit_to_byte(c[2]), a: unit_to_byte(c[3]) }).collect();
        self.apply_colors_bytes(rgba, &table)
    }

    /// BRUSH TEXTURE (C# `FillByElement`): fills a plane with a value per pixel computed from
    /// where the pixel lies in its owner's own axes - bristle ridges from
    /// [`ElementPixel::across`], a ramp along the stroke from [`ElementPixel::normalized_along`],
    /// and so on. Every owned pixel's value is `function(pixel)`, row by row, left to right (one
    /// thread); unowned pixels get `unowned_value` (None: 0). `axes`, when given, overrides
    /// [`DrawingOwnershipMap::get_axes`] for the element indexes where it returns a value. The
    /// error says what to do with a plane of another size.
    pub fn fill_by_element(&self, destination: &mut DrawingPlane, mut function: impl FnMut(&ElementPixel) -> f32, unowned_value: Option<f32>, axes: Option<&dyn Fn(usize) -> Option<ElementAxes>>) -> Result<(), String> {
        if destination.width() != self.width || destination.height() != self.height {
            return Err("The plane must be the map's size.".to_string());
        }
        let unowned = unowned_value.unwrap_or(0.0);
        let measures: Vec<Measure> = (0..self.elements.len()).map(|i| Measure::new(&self.matrices[i], axes.and_then(|f| f(i)).unwrap_or(self.axes[i]))).collect();
        let width = self.width as usize;
        let data = destination.data_mut();
        for y in 0..self.height as usize {
            let row = y * width;
            for x in 0..width {
                let owner = self.owners[row + x];
                data[row + x] = if owner < 0 { unowned } else { function(&measures[owner as usize].at(x as u32, y as u32, owner as usize)) };
            }
        }
        Ok(())
    }

    /// Measures one pixel in its owner's axes (C# `TryGetElementPixel`): None when the pixel has
    /// no owner. The error says the pixel lies outside the map.
    pub fn try_get_element_pixel(&self, x: u32, y: u32) -> Result<Option<ElementPixel>, String> {
        let owner = self.get(x, y)?;
        if owner < 0 {
            return Ok(None);
        }
        let o = owner as usize;
        Ok(Some(Measure::new(&self.matrices[o], self.axes[o]).at(x, y, o)))
    }
}

/// Exact nearest-owner fill (C#'s `NearestFill`): a separable squared-distance transform carried
/// out in integers. Pass 1, each column: the row of the nearest owned pixel in that column (ties:
/// the upper). Pass 2, each row: the lower envelope of the parabolas (x - q)^2 + d(q)^2 over the
/// columns q, with every breakpoint kept as an exact fraction; at a tie the smaller column wins.
pub(crate) fn nearest_fill(owners: &[i32], width: usize, height: usize) -> Vec<i32> {
    let mut nearest_row = vec![0i64; owners.len()];
    for x in 0..width {
        let mut last = -1i64;
        for y in 0..height {
            let i = (y * width) + x;
            if owners[i] >= 0 {
                last = y as i64;
            }
            nearest_row[i] = last;
        }
        let mut below = -1i64;
        for y in (0..height).rev() {
            let i = (y * width) + x;
            if owners[i] >= 0 {
                below = y as i64;
            }
            let above = nearest_row[i];
            let yy = y as i64;
            if below >= 0 && (above < 0 || (below - yy) < (yy - above)) {
                nearest_row[i] = below;
            }
        }
    }

    let mut result = vec![0i32; owners.len()];
    let mut vertex = vec![0usize; width];
    let mut boundary_numerator = vec![0i64; width];
    let mut boundary_denominator = vec![0i64; width];
    let mut heights = vec![0i64; width];
    for y in 0..height {
        let row = y * width;
        let mut top: isize = -1;
        for q in 0..width {
            let near = nearest_row[row + q];
            if near < 0 {
                continue;
            }
            let d = y as i64 - near;
            heights[q] = d * d;
            let (mut numerator, mut denominator) = (0i64, 1i64);
            while top >= 0 {
                let v = vertex[top as usize];
                numerator = (heights[q] + (q as i64 * q as i64)) - (heights[v] + (v as i64 * v as i64));
                denominator = 2 * (q as i64 - v as i64);
                // Pop v when the new parabola already wins at (or before) where v started winning
                if top > 0 && numerator * boundary_denominator[top as usize] <= boundary_numerator[top as usize] * denominator {
                    top -= 1;
                    continue;
                }
                break;
            }
            top += 1;
            vertex[top as usize] = q;
            boundary_numerator[top as usize] = numerator;
            boundary_denominator[top as usize] = denominator;
        }
        if top < 0 {
            result[row..row + width].copy_from_slice(&owners[row..row + width]);
            continue;
        }
        let mut k = 0usize;
        for x in 0..width {
            // Move on only when the next parabola is strictly lower here (ties keep the smaller column)
            while (k as isize) < top && boundary_numerator[k + 1] < x as i64 * boundary_denominator[k + 1] {
                k += 1;
            }
            let q = vertex[k];
            result[row + x] = owners[(nearest_row[row + q] as usize * width) + q];
        }
    }
    result
}
