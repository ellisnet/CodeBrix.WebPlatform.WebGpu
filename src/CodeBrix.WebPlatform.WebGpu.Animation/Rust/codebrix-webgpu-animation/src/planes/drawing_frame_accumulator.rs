//! `DrawingFrameAccumulator`: averages frames into one, a frame at a time, in premultiplied
//! space - the motion-blur helper (C# `DrawingFrameAccumulator`).

use crate::planes::drawing_plane::validate_size;
use crate::planes::DrawingPlaneImage;

/// Each byte frame adds at most 255 * 255 to a channel sum; this keeps the i32 sums exact.
const MAX_BYTE_FRAMES: u32 = (i32::MAX / (255 * 255)) as u32;

/// Averages any number of frames into one, a frame at a time (C# `DrawingFrameAccumulator`):
/// render sub-frames spread across a frame's shutter interval (times the caller computes - see
/// [`DrawingMotionBlur::get_sample_times`](crate::planes::DrawingMotionBlur::get_sample_times)),
/// add each, take the average. Only the running sums are kept.
///
/// Frames are averaged in PREMULTIPLIED space, so a moving object over transparency blurs into a
/// correctly fading streak. 8-BIT FRAMES (straight-alpha RGBA) are summed in exact integers: `SA =
/// sum of A`, `SC = sum of C * A`; the averaged alpha is `round(SA / N)` and colour `round(SC /
/// SA)` (0 where SA is 0), rounded half up in integers. FLOAT IMAGES are summed in double: `A =
/// sum(A) / N`, `C = sum(C * A) / sum(A)`. One accumulator takes one kind of input: the first add
/// decides.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingFrameAccumulator {
    width: u32,
    height: u32,
    count: u32,
    byte_sums: Option<Vec<i32>>,
    float_sums: Option<Vec<f64>>,
}

impl DrawingFrameAccumulator {
    /// An empty accumulator for frames of one size (C# `DrawingFrameAccumulator(width, height)`).
    pub fn new(width: u32, height: u32) -> Result<DrawingFrameAccumulator, String> {
        validate_size(width, height)?;
        Ok(DrawingFrameAccumulator { width, height, count: 0, byte_sums: None, float_sums: None })
    }

    /// The frame width in pixels (C# `Width`).
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The frame height in pixels (C# `Height`).
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The frames added since creation or the last reset (C# `Count`).
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Forgets every frame added so far (and the input kind) (C# `Reset`).
    pub fn reset(&mut self) {
        self.count = 0;
        if let Some(sums) = &mut self.byte_sums {
            sums.fill(0);
        }
        if let Some(sums) = &mut self.float_sums {
            sums.fill(0.0);
        }
    }

    /// Adds an 8-bit straight-alpha RGBA frame, tightly packed (C# `Add(ReadOnlySpan<byte>)`).
    /// The errors say what is wrong: too few bytes, float images already added, or more frames
    /// than the integer sums hold.
    pub fn add_rgba(&mut self, rgba: &[u8]) -> Result<(), String> {
        let pixels = self.width as usize * self.height as usize;
        if rgba.len() < pixels * 4 {
            return Err("The frame must hold at least width x height x 4 bytes.".to_string());
        }
        if self.count > 0 && self.byte_sums.is_none() {
            return Err("This accumulator already holds float images: reset it, or use another one for 8-bit frames.".to_string());
        }
        if self.count >= MAX_BYTE_FRAMES {
            return Err(format!("One accumulator averages at most {MAX_BYTE_FRAMES} 8-bit frames."));
        }
        self.float_sums = None;
        let sums = self.byte_sums.get_or_insert_with(|| vec![0; pixels * 4]);
        for o in (0..pixels * 4).step_by(4) {
            let a = rgba[o + 3] as i32;
            sums[o] += rgba[o] as i32 * a;
            sums[o + 1] += rgba[o + 1] as i32 * a;
            sums[o + 2] += rgba[o + 2] as i32 * a;
            sums[o + 3] += a;
        }
        self.count += 1;
        Ok(())
    }

    /// Adds a float image, straight alpha (C# `Add(DrawingPlaneImage)`).
    pub fn add_image(&mut self, image: &DrawingPlaneImage) -> Result<(), String> {
        if image.width() != self.width || image.height() != self.height {
            return Err(format!("The image must be {} x {} pixels, the accumulator's size.", self.width, self.height));
        }
        if self.count > 0 && self.float_sums.is_none() {
            return Err("This accumulator already holds 8-bit frames: reset it, or use another one for float images.".to_string());
        }
        self.byte_sums = None;
        let pixels = self.width as usize * self.height as usize;
        let sums = self.float_sums.get_or_insert_with(|| vec![0.0; pixels * 4]);
        let (r, g, b, alpha) = (image.r().data(), image.g().data(), image.b().data(), image.a().data());
        for i in 0..pixels {
            let o = i * 4;
            let a = alpha[i] as f64;
            sums[o] += r[i] as f64 * a;
            sums[o + 1] += g[i] as f64 * a;
            sums[o + 2] += b[i] as f64 * a;
            sums[o + 3] += a;
        }
        self.count += 1;
        Ok(())
    }

    /// Writes the average as 8-bit straight-alpha RGBA (C# `GetAverage(Span<byte>)`): the exact
    /// integer rule for 8-bit input; for float input the float average with the rounding rule of
    /// [`DrawingPlaneImage::to_rgba`]. The error says when no frame was added.
    pub fn get_average_into(&self, destination: &mut [u8]) -> Result<(), String> {
        let pixels = self.width as usize * self.height as usize;
        if destination.len() < pixels * 4 {
            return Err("The destination must hold at least width x height x 4 bytes.".to_string());
        }
        self.require_frames()?;
        if self.float_sums.is_some() {
            return self.get_average_image()?.to_rgba_into(destination);
        }
        let sums = self.byte_sums.as_ref().ok_or_else(no_frames)?;
        let n = self.count as i64;
        for o in (0..pixels * 4).step_by(4) {
            let sa = sums[o + 3] as i64;
            destination[o + 3] = (((2 * sa) + n) / (2 * n)) as u8;
            if sa == 0 {
                destination[o..o + 3].fill(0);
                continue;
            }
            for c in 0..3 {
                destination[o + c] = (((2 * sums[o + c] as i64) + sa) / (2 * sa)) as u8;
            }
        }
        Ok(())
    }

    /// The average as 8-bit straight-alpha RGBA (C# `GetAverage()`).
    pub fn get_average(&self) -> Result<Vec<u8>, String> {
        let mut bytes = vec![0u8; self.width as usize * self.height as usize * 4];
        self.get_average_into(&mut bytes)?;
        Ok(bytes)
    }

    /// The average as a float image (C# `GetAverageImage`): for 8-bit input alpha is `SA / (255
    /// N)` and colour `SC / (255 SA)`, in double.
    pub fn get_average_image(&self) -> Result<DrawingPlaneImage, String> {
        self.require_frames()?;
        let mut image = DrawingPlaneImage::new(self.width, self.height)?;
        let pixels = self.width as usize * self.height as usize;
        let n = self.count as f64;
        for i in 0..pixels {
            let o = i * 4;
            let (sa, sr, sg, sb, scale);
            if let Some(sums) = &self.float_sums {
                (sa, sr, sg, sb) = (sums[o + 3], sums[o], sums[o + 1], sums[o + 2]);
                image.a_mut().data_mut()[i] = (sa / n) as f32;
                scale = 1.0;
            } else {
                let sums = self.byte_sums.as_ref().ok_or_else(no_frames)?;
                (sa, sr, sg, sb) = (sums[o + 3] as f64, sums[o] as f64, sums[o + 1] as f64, sums[o + 2] as f64);
                image.a_mut().data_mut()[i] = (sa / (255.0 * n)) as f32;
                scale = 255.0;
            }
            if sa == 0.0 {
                continue;
            }
            image.r_mut().data_mut()[i] = (sr / (scale * sa)) as f32;
            image.g_mut().data_mut()[i] = (sg / (scale * sa)) as f32;
            image.b_mut().data_mut()[i] = (sb / (scale * sa)) as f32;
        }
        Ok(image)
    }

    fn require_frames(&self) -> Result<(), String> {
        if self.count == 0 {
            return Err(no_frames());
        }
        Ok(())
    }
}

fn no_frames() -> String {
    "No frame has been added: add at least one frame before taking the average.".to_string()
}
