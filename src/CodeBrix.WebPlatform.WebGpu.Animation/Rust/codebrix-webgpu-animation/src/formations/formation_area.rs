//! `FormationArea`: the region a formation fills - a filled geometry or a mask (C#
//! `FormationArea`).

use std::rc::Rc;

use crate::geometry::RectangleF;
use crate::formations::mask_distance;
use crate::paths::{PathAreaQuery, PathGeometry};

enum AreaSource {
    Geometry { geometry: PathGeometry, query: Rc<PathAreaQuery> },
    Mask { width: usize, height: usize, data: Rc<Vec<f32>>, destination: RectangleF, threshold: f32, inside: Rc<Vec<f32>>, outside: Rc<Vec<f32>> },
}

/// The region a formation fills (C# `FormationArea`): a geometry's filled area (by its fill
/// rule, curves flattened once at a tolerance) or the pixels of a mask at or above a threshold,
/// with the inside test and the distance to its edge that target filling and arrival orders ask.
#[derive(Clone)]
pub struct FormationArea {
    source: Rc<AreaSource>,
    bounds: RectangleF,
}

impl FormationArea {
    /// The filled area of a geometry, its curves flattened within `tolerance` (None: 0.25) (C#
    /// `FromGeometry(geometry, tolerance = 0.25)`). The error says what to do with a tolerance
    /// that is not positive.
    pub fn from_geometry(geometry: &PathGeometry, tolerance: Option<f32>) -> Result<FormationArea, String> {
        let query = PathAreaQuery::new(geometry, tolerance)?;
        let bounds = query.bounds();
        Ok(FormationArea { source: Rc::new(AreaSource::Geometry { geometry: geometry.clone(), query: Rc::new(query) }), bounds })
    }

    /// The pixels of a mask at or above `threshold` (None: 0.5), the mask - `width` x `height`
    /// values, row by row, as a coverage plane holds them - stretched over `destination` in
    /// calibrated units (the values of a plane, as [`FormationArea::from_mask`] takes them from a
    /// [`DrawingPlane`](crate::planes::DrawingPlane); no C# equivalent). Distances
    /// are exact Euclidean distances between pixel centres. The error says what is wrong.
    pub fn from_mask_data(width: usize, height: usize, values: &[f32], destination: RectangleF, threshold: Option<f32>) -> Result<FormationArea, String> {
        let threshold = threshold.unwrap_or(0.5);
        if width < 1 || height < 1 || values.len() != width * height {
            return Err("The mask must be at least 1 x 1 and hold width x height values.".to_string());
        }
        if !(destination.width > 0.0) || !(destination.height > 0.0) || !destination.x.is_finite() || !destination.y.is_finite() || !destination.width.is_finite() || !destination.height.is_finite() {
            return Err("The destination must be a finite rectangle with a positive width and height.".to_string());
        }
        if threshold.is_nan() {
            return Err("The threshold must be a number.".to_string());
        }
        let unit_x = destination.width as f64 / width as f64;
        let unit_y = destination.height as f64 / height as f64;
        let mut inside = vec![false; width * height];
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (width as i64, height as i64, -1i64, -1i64);
        for y in 0..height {
            for x in 0..width {
                if !(values[(y * width) + x] >= threshold) {
                    continue;
                }
                inside[(y * width) + x] = true;
                min_x = min_x.min(x as i64);
                max_x = max_x.max(x as i64);
                min_y = min_y.min(y as i64);
                max_y = max_y.max(y as i64);
            }
        }
        let bounds = if max_x < 0 {
            RectangleF::default()
        } else {
            RectangleF::new(
                (destination.x as f64 + (min_x as f64 * unit_x)) as f32,
                (destination.y as f64 + (min_y as f64 * unit_y)) as f32,
                ((max_x - min_x + 1) as f64 * unit_x) as f32,
                ((max_y - min_y + 1) as f64 * unit_y) as f32,
            )
        };
        let inside_distances = mask_distance::compute(&inside, width, height, unit_x, unit_y, true);
        let outside_distances = mask_distance::compute(&inside, width, height, unit_x, unit_y, false);
        Ok(FormationArea {
            source: Rc::new(AreaSource::Mask { width, height, data: Rc::new(values.to_vec()), destination, threshold, inside: Rc::new(inside_distances), outside: Rc::new(outside_distances) }),
            bounds,
        })
    }

    /// The geometry (None for a mask) (C# `Geometry`).
    pub fn geometry(&self) -> Option<&PathGeometry> {
        match self.source.as_ref() {
            AreaSource::Geometry { geometry, .. } => Some(geometry),
            AreaSource::Mask { .. } => None,
        }
    }

    /// The bounds of the area, calibrated units (C# `Bounds`; empty for a mask with no pixel in).
    pub fn bounds(&self) -> RectangleF {
        self.bounds
    }

    /// True when a point is inside (C# `Contains`).
    pub fn contains(&self, x: f32, y: f32) -> bool {
        match self.source.as_ref() {
            AreaSource::Geometry { query, .. } => query.contains(x, y),
            AreaSource::Mask { width, height, data, destination, threshold, .. } => match pixel_index(*width, *height, *destination, x, y) {
                Some(i) => data[i] >= *threshold,
                None => false,
            },
        }
    }

    /// The distance from a point to the area's edge, inside or out (C# `DistanceToEdge`);
    /// infinity outside a mask's destination.
    pub fn distance_to_edge(&self, x: f32, y: f32) -> f32 {
        match self.source.as_ref() {
            AreaSource::Geometry { query, .. } => query.distance_to_edge(x, y),
            AreaSource::Mask { width, height, data, destination, threshold, inside, outside } => match pixel_index(*width, *height, *destination, x, y) {
                Some(i) => {
                    if data[i] >= *threshold {
                        inside[i]
                    } else {
                        outside[i]
                    }
                }
                None => f32::INFINITY,
            },
        }
    }
}

impl std::fmt::Debug for FormationArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormationArea").field("bounds", &self.bounds).field("is_mask", &self.geometry().is_none()).finish()
    }
}

/// The mask pixel a point falls in; None outside the destination (C#'s `PixelIndex`).
fn pixel_index(width: usize, height: usize, destination: RectangleF, x: f32, y: f32) -> Option<usize> {
    let px = ((x - destination.x) as f64 * width as f64 / destination.width as f64).floor();
    let py = ((y - destination.y) as f64 * height as f64 / destination.height as f64).floor();
    if !(px >= 0.0) || !(py >= 0.0) || px >= width as f64 || py >= height as f64 {
        return None;
    }
    Some((py as usize * width) + px as usize)
}
