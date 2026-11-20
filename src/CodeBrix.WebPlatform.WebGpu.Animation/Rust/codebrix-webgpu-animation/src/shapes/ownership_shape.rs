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
/// the CPU plane's exact blur). THE REAL-TIME ROUTE for re-colouring: the map is uploaded once
/// as a picture of owner numbers with the colours beside it, so a scene that changes only the
/// colours (colour pickup from a moving picture, a palette animating) makes a new shape with new
/// colours and pays one small upload, not a CPU pass over every pixel. Drawn stretched over
/// `destination` with nearest sampling (one map pixel per block).
#[derive(Clone, Debug, PartialEq)]
pub struct OwnershipShape {
    pub(crate) picture: ImageSource,
    pub(crate) map_size: (u32, u32),
    pub(crate) destination: RectangleF,
    pub(crate) options: OwnershipShapeOptions,
}

/// The owner number as three bytes (0 = nobody, otherwise the element index + 1).
fn encode(owner: i32) -> [u8; 4] {
    if owner < 0 {
        return [0, 0, 0, 0];
    }
    let n = owner as u32 + 1;
    [(n & 255) as u8, ((n >> 8) & 255) as u8, ((n >> 16) & 255) as u8, 255]
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
        for &owner in map.owners() {
            bytes.extend_from_slice(&encode(owner));
        }
        for i in 0..(palette_rows * w) as usize {
            let c = colors.get(i).filter(|_| i < map.element_count()).copied().unwrap_or(Color::TRANSPARENT);
            bytes.extend_from_slice(&[c.r, c.g, c.b, c.a]);
        }
        let picture = ImageSource::from_rgba_vec(bytes, w, h + palette_rows)?;
        Ok(OwnershipShape { picture, map_size: (w, h), destination, options })
    }

    /// The map's float colours (as `get_mean_colors` gives them), converted with the planes'
    /// float-to-byte rule.
    pub fn from_float_colors(map: &DrawingOwnershipMap, colors: &[[f32; 4]], destination: RectangleF, options: OwnershipShapeOptions) -> Result<OwnershipShape, String> {
        let to_byte = |v: f32| if v.is_nan() || v <= 0.0 { 0 } else if v >= 1.0 { 255 } else { (v * 255.0 + 0.5) as u8 };
        let colors: Vec<Color> = colors.iter().map(|c| Color::from_rgba(to_byte(c[0]), to_byte(c[1]), to_byte(c[2]), to_byte(c[3]))).collect();
        OwnershipShape::new(map, &colors, destination, options)
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
