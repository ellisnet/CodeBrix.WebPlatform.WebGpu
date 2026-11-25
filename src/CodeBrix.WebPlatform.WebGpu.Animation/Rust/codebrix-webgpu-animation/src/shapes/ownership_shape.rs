//! `OwnershipShape`: an ownership map COLOURED ON THE GPU - one colour per element and the seams
//! between owners computed per pixel by the renderer (no C# type: C# colours a plane image on the
//! CPU with `ApplyColors` and `CreateBoundaryPlane`).

use crate::color::Color;
use crate::geometry::RectangleF;
use crate::images::ImageSource;
use crate::painting::DrawingOwnershipMap;

/// The options of an [`OwnershipShape`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OwnershipShapeOptions {
    /// The colour of pixels no element owns (transparent by default).
    pub unowned: Color,
    /// The seams: the colour blended in where a pixel's owner differs from its left or upper
    /// neighbour's (the boundary softened over a 3 x 3 neighbourhood, about the planes'
    /// `create_boundary_plane(Some(0.8))`), or None for no seams.
    pub seam_color: Option<Color>,
    /// How strongly the seams are blended in (the boundary times this, clamped to 0..1; C#'s
    /// scenes multiply the boundary plane by about 1 to 2).
    pub seam_strength: f32,
}

impl Default for OwnershipShapeOptions {
    fn default() -> OwnershipShapeOptions {
        OwnershipShapeOptions { unowned: Color::TRANSPARENT, seam_color: None, seam_strength: 1.0 }
    }
}

/// An ownership map drawn as a picture whose colours the GPU computes: each pixel takes the
/// colour of the element that owns it (C#'s `ApplyColors`), pixels nobody owns the unowned
/// colour, and - when asked - the seams between owners blended in, all per pixel in the renderer
/// (the planes' rules and parameters; seams within the rounding of a 3 x 3 neighbourhood, not
/// the CPU plane's exact blur). Drawn stretched over `destination` with nearest sampling (one
/// map pixel per block).
///
/// THE REAL-TIME ROUTE for re-colouring (colour pickup from a moving picture, a palette
/// animating): make the shape once with [`new`](OwnershipShape::new), then each new set of
/// colours with [`with_colors`](OwnershipShape::with_colors) on it. Every shape made that way
/// SHARES the map's uploaded picture of owner numbers: the renderer keeps that texture on the
/// GPU while any of them is drawn and sends only the colour table (the map's width x a row per
/// that many elements, 4 bytes a colour) when the colours change - visible as
/// `RenderStats::color_table_uploads`. A shape made with `new` again is a new picture: a whole
/// upload (`RenderStats::image_uploads`). Two shapes of one map drawn in the same frame with
/// different colours use two textures (each uploaded whole once, then kept).
#[derive(Clone, Debug, PartialEq)]
pub struct OwnershipShape {
    /// The owner numbers, with the colours of `new` in the rows below them (shared by every
    /// shape made from this one with `with_colors`).
    pub(crate) picture: ImageSource,
    /// The colour table drawn with the picture when it is not the picture's own rows (the map's
    /// width, the picture's table rows).
    pub(crate) palette: Option<ImageSource>,
    /// The pixels each element owns (by element index), and then the pixels nobody owns: for
    /// the representative colour of a flat look.
    pub(crate) owned: std::rc::Rc<Vec<u32>>,
    pub(crate) map_size: (u32, u32),
    pub(crate) destination: RectangleF,
    pub(crate) options: OwnershipShapeOptions,
    /// The colour a flat look paints it with: the owners' colours averaged by the pixels each
    /// owns (the unowned colour for the pixels nobody owns).
    pub(crate) representative: Color,
}

/// The owner number as three bytes (0 = nobody, otherwise the element index + 1).
fn encode(owner: i32) -> [u8; 4] {
    if owner < 0 {
        return [0, 0, 0, 0];
    }
    let n = owner as u32 + 1;
    [(n & 255) as u8, ((n >> 8) & 255) as u8, ((n >> 16) & 255) as u8, 255]
}

/// The planes' float-to-byte rule for float colours.
fn float_colors(colors: &[[f32; 4]]) -> Vec<Color> {
    let to_byte = |v: f32| if v.is_nan() || v <= 0.0 { 0 } else if v >= 1.0 { 255 } else { (v * 255.0 + 0.5) as u8 };
    colors.iter().map(|c| Color::from_rgba(to_byte(c[0]), to_byte(c[1]), to_byte(c[2]), to_byte(c[3]))).collect()
}

/// The colour table: `slots` colours (RGBA bytes), the elements' first, transparent after.
fn table(colors: &[Color], elements: usize, slots: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(slots * 4);
    for i in 0..slots {
        let c = colors.get(i).filter(|_| i < elements).copied().unwrap_or(Color::TRANSPARENT);
        bytes.extend_from_slice(&[c.r, c.g, c.b, c.a]);
    }
    bytes
}

/// The owners' colours averaged by the pixels each owns (`owned`: per element, then nobody's),
/// pixels nobody owns counting as the unowned colour (alpha-weighted colour, mean alpha).
fn representative(owned: &[u32], colors: &[Color], unowned: Color) -> Color {
    let mut sum = [0.0f64; 4];
    let mut n = 0.0f64;
    for (i, &count) in owned.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let c = if i + 1 == owned.len() { unowned } else { colors.get(i).copied().unwrap_or(unowned) };
        let (k, a) = (count as f64, c.a as f64 / 255.0);
        sum[0] += c.r as f64 / 255.0 * a * k;
        sum[1] += c.g as f64 / 255.0 * a * k;
        sum[2] += c.b as f64 / 255.0 * a * k;
        sum[3] += a * k;
        n += k;
    }
    if n == 0.0 || sum[3] <= 0.0 {
        return Color::TRANSPARENT;
    }
    crate::rendering::scene_look::color_of([(sum[0] / sum[3]) as f32, (sum[1] / sum[3]) as f32, (sum[2] / sum[3]) as f32, (sum[3] / n) as f32])
}

impl OwnershipShape {
    /// The map coloured by `colors` (one per element, in the map's element order, as
    /// `apply_colors_bytes` takes them) over `destination`. The error says what is wrong: too few
    /// colours, an empty destination.
    pub fn new(map: &DrawingOwnershipMap, colors: &[Color], destination: RectangleF, options: OwnershipShapeOptions) -> Result<OwnershipShape, String> {
        if colors.len() < map.element_count() {
            return Err(format!("There must be a colour for every element: {} given, {} needed.", colors.len(), map.element_count()));
        }
        if !(destination.width > 0.0) || !(destination.height > 0.0) {
            return Err("An ownership shape needs a destination with a width and height above 0.".to_string());
        }
        let (w, h) = map.size();
        // The picture: the owner numbers, row by row, then the colours in the rows below them.
        let palette_rows = (map.element_count() as u32).div_ceil(w).max(1);
        let mut bytes = Vec::with_capacity((w * (h + palette_rows) * 4) as usize);
        let mut owned = vec![0u32; map.element_count() + 1];
        for &owner in map.owners() {
            bytes.extend_from_slice(&encode(owner));
            let index = if owner < 0 || owner as usize >= map.element_count() { map.element_count() } else { owner as usize };
            owned[index] += 1;
        }
        bytes.extend_from_slice(&table(colors, map.element_count(), (palette_rows * w) as usize));
        let picture = ImageSource::from_rgba_vec(bytes, w, h + palette_rows)?;
        let representative = representative(&owned, colors, options.unowned);
        Ok(OwnershipShape { picture, palette: None, owned: std::rc::Rc::new(owned), map_size: (w, h), destination, options, representative })
    }

    /// The SAME map with other colours (one per element, as `new` takes them), sharing this
    /// shape's uploaded picture of owner numbers: drawing it sends only the colour table to the
    /// GPU. Destination and options are this shape's (change them on the result). The error
    /// says what is wrong: too few colours. Not in C#.
    pub fn with_colors(&self, colors: &[Color]) -> Result<OwnershipShape, String> {
        let elements = self.owned.len() - 1;
        if colors.len() < elements {
            return Err(format!("There must be a colour for every element: {} given, {} needed.", colors.len(), elements));
        }
        let (w, h) = self.map_size;
        let (_, total) = self.picture.size();
        let rows = total - h;
        let palette = ImageSource::from_rgba_vec(table(colors, elements, (rows * w) as usize), w, rows)?;
        let representative = representative(&self.owned, colors, self.options.unowned);
        Ok(OwnershipShape { palette: Some(palette), representative, ..self.clone() })
    }

    /// [`with_colors`](OwnershipShape::with_colors) from float colours (as `get_mean_colors`
    /// gives them), converted with the planes' float-to-byte rule. Not in C#.
    pub fn with_float_colors(&self, colors: &[[f32; 4]]) -> Result<OwnershipShape, String> {
        self.with_colors(&float_colors(colors))
    }

    /// The same shape drawn over another destination.
    pub fn with_destination(&self, destination: RectangleF) -> Result<OwnershipShape, String> {
        if !(destination.width > 0.0) || !(destination.height > 0.0) {
            return Err("An ownership shape needs a destination with a width and height above 0.".to_string());
        }
        Ok(OwnershipShape { destination, ..self.clone() })
    }

    /// The map's float colours (as `get_mean_colors` gives them), converted with the planes'
    /// float-to-byte rule.
    pub fn from_float_colors(map: &DrawingOwnershipMap, colors: &[[f32; 4]], destination: RectangleF, options: OwnershipShapeOptions) -> Result<OwnershipShape, String> {
        OwnershipShape::new(map, &float_colors(colors), destination, options)
    }

    /// Where it is drawn, calibrated units.
    pub fn destination(&self) -> RectangleF {
        self.destination
    }

    /// The map's size in pixels.
    pub fn map_size(&self) -> (u32, u32) {
        self.map_size
    }

    /// The options.
    pub fn options(&self) -> OwnershipShapeOptions {
        self.options
    }

    /// The bounds (the destination).
    pub fn get_bounds(&self) -> RectangleF {
        self.destination
    }
}

/// What the GPU's ownership paint needs (crate-internal): the map's size, the unowned and seam
/// colours packed as RGBA bytes, the seam strength.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OwnersPaintParams {
    pub(crate) map_size: (u32, u32),
    pub(crate) unowned: u32,
    pub(crate) seam: Option<u32>,
    pub(crate) seam_strength: f32,
}

fn pack(c: Color) -> u32 {
    c.r as u32 | ((c.g as u32) << 8) | ((c.b as u32) << 16) | ((c.a as u32) << 24)
}

impl OwnershipShape {
    pub(crate) fn gpu_params(&self) -> OwnersPaintParams {
        OwnersPaintParams { map_size: self.map_size, unowned: pack(self.options.unowned), seam: self.options.seam_color.map(pack), seam_strength: self.options.seam_strength }
    }
}
