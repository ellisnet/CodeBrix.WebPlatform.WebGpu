//! `DrawingMotionBlur`: motion blur as data - the sub-frame times of one frame and the average of
//! the sub-frames (C# `DrawingMotionBlur`).

use crate::planes::drawing_plane::validate_size;
use crate::planes::{DrawingFrameAccumulator, DrawingPlaneImage};

/// Motion blur as data (C# `DrawingMotionBlur`): the moments to render N sub-frames at, and their
/// average, in premultiplied space with the exact rules of [`DrawingFrameAccumulator`]. The frame
/// time is PASSED IN (the frame's own time); nothing here reads a clock.
pub struct DrawingMotionBlur;

impl DrawingMotionBlur {
    /// The evenly spaced sub-frame moments of one output frame (C# `GetSampleTimes(frameTime,
    /// frameDuration, samples, shutter = 0.5)`): sample i is at `frame_time + (i + 0.5) / samples
    /// * shutter * frame_duration`. The error says the samples must be 1 or more.
    pub fn get_sample_times(frame_time: f64, frame_duration: f64, samples: u32, shutter: Option<f64>) -> Result<Vec<f64>, String> {
        if samples < 1 {
            return Err("Motion blur needs 1 or more samples.".to_string());
        }
        let open = shutter.unwrap_or(0.5) * frame_duration;
        Ok((0..samples).map(|i| frame_time + ((i as f64 + 0.5) / samples as f64 * open)).collect())
    }

    /// Averages float images (straight alpha, one size) into a new image (C# `Average(frames)`).
    pub fn average(frames: &[DrawingPlaneImage]) -> Result<DrawingPlaneImage, String> {
        let first = frames.first().ok_or_else(|| "Motion blur needs at least one frame.".to_string())?;
        let mut accumulator = DrawingFrameAccumulator::new(first.width(), first.height())?;
        for frame in frames {
            accumulator.add_image(frame)?;
        }
        accumulator.get_average_image()
    }

    /// Averages 8-bit straight-alpha RGBA frames into a destination, in exact integers (C#
    /// `Average(frames, width, height, destination)`).
    pub fn average_bytes_into(frames: &[&[u8]], width: u32, height: u32, destination: &mut [u8]) -> Result<(), String> {
        if frames.is_empty() {
            return Err("Motion blur needs at least one frame.".to_string());
        }
        let mut accumulator = DrawingFrameAccumulator::new(width, height)?;
        for frame in frames {
            accumulator.add_rgba(frame)?;
        }
        accumulator.get_average_into(destination)
    }

    /// Averages 8-bit straight-alpha RGBA frames into a new frame (C# `Average(frames, width,
    /// height)`).
    pub fn average_bytes(frames: &[&[u8]], width: u32, height: u32) -> Result<Vec<u8>, String> {
        validate_size(width, height)?;
        let mut destination = vec![0u8; width as usize * height as usize * 4];
        DrawingMotionBlur::average_bytes_into(frames, width, height, &mut destination)?;
        Ok(destination)
    }
}
