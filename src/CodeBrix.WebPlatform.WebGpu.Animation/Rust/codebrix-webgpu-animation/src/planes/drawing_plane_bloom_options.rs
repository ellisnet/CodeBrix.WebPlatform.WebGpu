//! `DrawingPlaneBloomOptions`: the defaulted arguments of C#'s `CreateBloom`.

use crate::planes::DrawingPlaneResampling;

/// The defaulted arguments of [`DrawingPlane::create_bloom`](crate::planes::DrawingPlane::create_bloom)
/// and [`DrawingPlaneImage::create_bloom`](crate::planes::DrawingPlaneImage::create_bloom) (C#
/// `CreateBloom(downsampleFactor, blurs, gain = 1, outputSize = null, upsampling = Bilinear)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawingPlaneBloomOptions {
    /// The factor applied to the summed glow (1 by default).
    pub gain: f32,
    /// The size of the returned plane, pixels (width, height); None (the default) means the
    /// source's size.
    pub output_size: Option<(u32, u32)>,
    /// How the small glow is enlarged again (bilinear by default).
    pub upsampling: DrawingPlaneResampling,
}

impl Default for DrawingPlaneBloomOptions {
    fn default() -> DrawingPlaneBloomOptions {
        DrawingPlaneBloomOptions { gain: 1.0, output_size: None, upsampling: DrawingPlaneResampling::Bilinear }
    }
}
