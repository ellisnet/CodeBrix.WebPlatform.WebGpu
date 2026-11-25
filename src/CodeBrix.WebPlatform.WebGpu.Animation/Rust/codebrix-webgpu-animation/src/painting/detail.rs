//! `DetailMap`: where a painting needs small, careful strokes - functions over plain
//! `DrawingPlane`s of 0..1 laid over a rectangle (no new type; no C# equivalent) - and the
//! picture measures direction fields share (luminance, Sobel gradients).

use crate::geometry::RectangleF;
use crate::planes::{DrawingPlane, DrawingPlaneImage};

/// Detail maps (no C# equivalent): a detail map is a [`DrawingPlane`] of 0..1 (1 = the most
/// detail) laid over a rectangle of calibrated units. Make one from a picture's edges, from a
/// layer's mask (`session.export_layer_mask(..)` blurred: a face painted carefully), or combine
/// them with the planes' own arithmetic (`max_plane`, `multiply_plane`, `gaussian_blur`).
/// [`StrokeLayout::fill`](crate::painting::StrokeLayout::fill) reads one to choose stroke sizes.
pub struct DetailMap;

impl DetailMap {
    /// The edge strength of a picture, 0..1, at the picture's size: the luminance (Rec. 709
    /// weights of the encoded channels, times alpha), its Sobel gradient magnitude, blurred by
    /// `smoothing` pixels (a Gaussian sigma; 0 = none), divided by its largest value (a flat
    /// picture gives 0 everywhere). Detail where forms change. The error says what is wrong.
    pub fn from_picture_edges(picture: &DrawingPlaneImage, smoothing: f32) -> Result<DrawingPlane, String> {
        if !smoothing.is_finite() || smoothing < 0.0 {
            return Err("The smoothing must be a finite number of pixels, 0 or more.".to_string());
        }
        let luminance = luminance_plane(picture)?;
        let (gx, gy) = sobel(&luminance);
        let magnitude: Vec<f32> = gx.iter().zip(gy.iter()).map(|(x, y)| x.hypot(*y)).collect();
        let mut plane = DrawingPlane::from_data(luminance.width(), luminance.height(), magnitude)?;
        if smoothing > 0.0 {
            plane.gaussian_blur(smoothing)?;
        }
        let largest = plane.data().iter().fold(0.0f32, |m, v| m.max(*v));
        if largest > 0.0 {
            plane.multiply(1.0 / largest);
            plane.clamp(0.0, 1.0)?;
        } else {
            plane.fill(0.0);
        }
        Ok(plane)
    }

    /// A plane laid over `destination`, sampled at a point: bilinear between pixel centres
    /// (the edge pixels' values held up to the rectangle's edge), 0 outside the rectangle (or
    /// for an empty rectangle).
    pub fn sample(plane: &DrawingPlane, destination: RectangleF, x: f32, y: f32) -> f32 {
        if destination.is_empty() || !(x >= destination.x && x <= destination.right() && y >= destination.y && y <= destination.bottom()) {
            return 0.0;
        }
        bilinear(plane.data(), plane.width() as usize, plane.height() as usize, destination, x, y)
    }
}

/// Bilinear sampling of a row-major grid laid over a rectangle, between cell centres, clamped
/// to the edge cells.
pub(crate) fn bilinear(data: &[f32], width: usize, height: usize, destination: RectangleF, x: f32, y: f32) -> f32 {
    let (i0, i1, fx) = axis(x, destination.x, destination.width, width);
    let (j0, j1, fy) = axis(y, destination.y, destination.height, height);
    let top = data[(j0 * width) + i0] + ((data[(j0 * width) + i1] - data[(j0 * width) + i0]) * fx);
    let bottom = data[(j1 * width) + i0] + ((data[(j1 * width) + i1] - data[(j1 * width) + i0]) * fx);
    top + ((bottom - top) * fy)
}

/// The two cells either side of a coordinate along one axis and the weight of the second,
/// clamped to the grid (NaN reads the first cell).
pub(crate) fn axis(v: f32, origin: f32, extent: f32, count: usize) -> (usize, usize, f32) {
    let last = count.saturating_sub(1);
    let p = (((v - origin) / extent) * count as f32) - 0.5;
    if !(p > 0.0) {
        return (0, 0, 0.0);
    }
    if p >= last as f32 {
        return (last, last, 0.0);
    }
    let i = p.floor() as usize;
    (i, (i + 1).min(last), p - i as f32)
}

/// A picture's luminance: Rec. 709 weights of the encoded channels, times alpha (so the edge of
/// an opaque shape over transparency counts as an edge).
pub(crate) fn luminance_plane(picture: &DrawingPlaneImage) -> Result<DrawingPlane, String> {
    let (r, g, b, a) = (picture.r().data(), picture.g().data(), picture.b().data(), picture.a().data());
    let data: Vec<f32> = (0..r.len()).map(|i| ((0.2126 * r[i]) + (0.7152 * g[i]) + (0.0722 * b[i])) * a[i].clamp(0.0, 1.0)).collect();
    DrawingPlane::from_data(picture.width(), picture.height(), data)
}

/// Sobel gradients of a plane (x to the right, y down), the edge pixels repeated beyond the
/// edges. Row by row, left to right.
pub(crate) fn sobel(plane: &DrawingPlane) -> (Vec<f32>, Vec<f32>) {
    let (w, h) = (plane.width() as usize, plane.height() as usize);
    let d = plane.data();
    let at = |x: isize, y: isize| d[(y.clamp(0, h as isize - 1) as usize * w) + x.clamp(0, w as isize - 1) as usize];
    let mut gx = vec![0.0f32; w * h];
    let mut gy = vec![0.0f32; w * h];
    for y in 0..h as isize {
        for x in 0..w as isize {
            let (tl, t, tr) = (at(x - 1, y - 1), at(x, y - 1), at(x + 1, y - 1));
            let (l, r) = (at(x - 1, y), at(x + 1, y));
            let (bl, b, br) = (at(x - 1, y + 1), at(x, y + 1), at(x + 1, y + 1));
            let i = (y as usize * w) + x as usize;
            gx[i] = (tr + (2.0 * r) + br) - (tl + (2.0 * l) + bl);
            gy[i] = (bl + (2.0 * b) + br) - (tl + (2.0 * t) + tr);
        }
    }
    (gx, gy)
}
