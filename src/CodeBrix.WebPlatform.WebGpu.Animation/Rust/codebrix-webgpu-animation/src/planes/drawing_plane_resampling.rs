//! `DrawingPlaneResampling`: how a resize chooses source values (C# `DrawingPlaneResampling`).

/// How [`DrawingPlane::resized`](crate::planes::DrawingPlane::resized) and
/// [`DrawingPlaneImage::resized`](crate::planes::DrawingPlaneImage::resized) choose the source
/// values for each output pixel (C# `DrawingPlaneResampling`). Every rule is exact integer or
/// float arithmetic, so a resize gives the same values as C#.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DrawingPlaneResampling {
    /// Bilinear interpolation on the pixel-centre grid: output pixel `i` samples source position
    /// `(i + 0.5) * source_size / output_size - 0.5`, clamped to the source (the default).
    #[default]
    Bilinear = 0,
    /// Each output pixel copies the source pixel whose area holds the output pixel's centre:
    /// source index `floor((2i + 1) * source_size / (2 * output_size))`, in integers.
    Nearest = 1,
    /// Bilinear interpolation on the corner-aligned grid: output pixel `i` samples source
    /// position `i * (source_size - 1) / (output_size - 1)`, so the first and last output pixels
    /// land exactly on the first and last source pixels.
    BilinearAlignedCorners = 2,
}
