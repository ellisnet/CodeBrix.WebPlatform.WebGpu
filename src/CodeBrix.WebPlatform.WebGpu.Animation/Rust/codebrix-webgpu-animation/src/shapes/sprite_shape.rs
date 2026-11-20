//! `SpriteShape`: one frame of a sprite sheet, chosen by the time (C# `SpriteShape`).

use kurbo::Affine;

use crate::color::Color;
use crate::geometry::RectangleF;
use crate::images::{ImageSampling, SpritePlayback, SpriteSheet};
use crate::rendering::VectorScene;
use crate::shapes::image_shape::{check_destination, encode_picture};

/// The optional arguments of [`SpriteShape::new`] (C#'s defaults: 0 frames per second, loop,
/// bilinear).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpriteShapeOptions {
    /// How fast the frames advance with time; 0 to show the frame index only.
    pub frames_per_second: f64,
    /// How the running frame number maps onto the frames.
    pub playback: SpritePlayback,
    /// How pixels are sampled when scaled or turned.
    pub sampling: ImageSampling,
}

/// One frame of a [`SpriteSheet`] drawn into a destination rectangle - chosen by index, by time
/// at a frame rate, or both (C# `SpriteShape`). As an element it takes a transform, opacity,
/// time window, effects and groups; the layer colour is not used.
///
/// WHICH FRAME, a FUNCTION OF THE SESSION TIME: the running frame number at time t is
/// `n = frame_index + floor((t - playback_start_time) x frames_per_second + 1e-6)` (just
/// `frame_index` when the rate is 0), and the [`SpritePlayback`] mode maps n onto the sheet's
/// frames. The time is the session's, so a sprite plays as the scene's time moves, with no
/// per-frame code; bind `AnimatedProperty::FrameIndex` (a float track, rounded down) to choose
/// frames by hand.
#[derive(Clone, Debug, PartialEq)]
pub struct SpriteShape {
    sheet: SpriteSheet,
    destination: RectangleF,
    frames_per_second: f64,
    playback: SpritePlayback,
    sampling: ImageSampling,
    frame_index: i32,
    playback_start_time: f64,
    tint: Option<Color>,
}

impl SpriteShape {
    /// A sprite (C# `new SpriteShape(sheet, destination, framesPerSecond = 0, playback = Loop,
    /// sampling = Linear)`). The error says what is wrong with the rate or the destination.
    pub fn new(sheet: SpriteSheet, destination: RectangleF, options: SpriteShapeOptions) -> Result<SpriteShape, String> {
        if !(options.frames_per_second.is_finite() && options.frames_per_second >= 0.0) {
            return Err(format!("The frame rate must be a finite number of 0 or more (it was {}).", options.frames_per_second));
        }
        check_destination(destination)?;
        Ok(SpriteShape { sheet, destination, frames_per_second: options.frames_per_second, playback: options.playback, sampling: options.sampling, frame_index: 0, playback_start_time: 0.0, tint: None })
    }

    /// The frames.
    pub fn sheet(&self) -> &SpriteSheet {
        &self.sheet
    }

    /// Where each frame is drawn, calibrated units.
    pub fn destination(&self) -> RectangleF {
        self.destination
    }

    /// How fast the frames advance (0 = they do not).
    pub fn frames_per_second(&self) -> f64 {
        self.frames_per_second
    }

    /// How the running frame number maps onto the frames.
    pub fn playback(&self) -> SpritePlayback {
        self.playback
    }

    /// How pixels are sampled.
    pub fn sampling(&self) -> ImageSampling {
        self.sampling
    }

    /// The frame number added to the timed advance (the frame shown at the playback start); 0 by
    /// default (C# `FrameIndex`).
    pub fn frame_index(&self) -> i32 {
        self.frame_index
    }

    /// Sets the frame index (bindable as `AnimatedProperty::FrameIndex`).
    pub fn set_frame_index(&mut self, value: i32) {
        self.frame_index = value;
    }

    /// The same sprite with a frame index.
    pub fn with_frame_index(mut self, value: i32) -> SpriteShape {
        self.frame_index = value;
        self
    }

    /// The time, seconds, at which the timed advance starts counting from the frame index (C#
    /// `PlaybackStartTime`; 0 by default).
    pub fn playback_start_time(&self) -> f64 {
        self.playback_start_time
    }

    /// Sets the playback start; the error says what to do with a value that is not finite.
    pub fn set_playback_start_time(&mut self, seconds: f64) -> Result<(), String> {
        if !seconds.is_finite() {
            return Err("The playback start time must be a finite number of seconds.".to_string());
        }
        self.playback_start_time = seconds;
        Ok(())
    }

    /// A colour every pixel is multiplied by (alpha included); None for none (C# `Tint`).
    pub fn tint(&self) -> Option<Color> {
        self.tint
    }

    /// Sets the tint (bindable as `AnimatedProperty::Tint`).
    pub fn set_tint(&mut self, tint: Option<Color>) {
        self.tint = tint;
    }

    /// The same sprite with a tint.
    pub fn with_tint(mut self, tint: Option<Color>) -> SpriteShape {
        self.tint = tint;
        self
    }

    /// The index in the sheet of the frame shown at a time (C# `GetFrameAt`).
    pub fn get_frame_at(&self, seconds: f64) -> usize {
        let mut n = self.frame_index as i64;
        if self.frames_per_second > 0.0 && seconds.is_finite() {
            n += ((seconds - self.playback_start_time) * self.frames_per_second + 1e-6).floor() as i64;
        }
        self.playback.map_frame(n, self.sheet.frame_count())
    }

    /// The destination rectangle (C# `GetBounds`).
    pub fn get_bounds(&self) -> RectangleF {
        self.destination
    }

    pub(crate) fn encode(&self, scene: &mut VectorScene, transform: Affine, time: f64, opacity: f32) {
        if let Some(frame) = self.sheet.get_frame(self.get_frame_at(time)) {
            encode_picture(scene, transform, frame, self.destination, None, self.sampling, self.tint, opacity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::ImageSource;

    fn sheet(frames: u32) -> SpriteSheet {
        let image = ImageSource::from_rgba(&vec![255; (frames * 4 * 4) as usize], frames * 2, 2).unwrap();
        SpriteSheet::from_grid(&image, frames, 1, 0).unwrap()
    }

    #[test]
    fn the_frame_is_a_function_of_the_time() {
        //Arrange
        let options = SpriteShapeOptions { frames_per_second: 12.0, ..SpriteShapeOptions::default() };
        let looping = SpriteShape::new(sheet(8), RectangleF::new(0.0, 0.0, 10.0, 10.0), options).unwrap();
        let mut hold = SpriteShape::new(sheet(8), RectangleF::new(0.0, 0.0, 10.0, 10.0), SpriteShapeOptions { frames_per_second: 6.0, playback: SpritePlayback::Hold, ..options }).unwrap();
        hold.set_playback_start_time(2.0).unwrap();
        let stepped = SpriteShape::new(sheet(8), RectangleF::new(0.0, 0.0, 10.0, 10.0), SpriteShapeOptions::default()).unwrap().with_frame_index(5);
        //Act / Assert
        // C#: n = FrameIndex + floor((t - start) * fps + 1e-6), then the playback mode.
        assert_eq!(looping.get_frame_at(0.0), 0);
        assert_eq!(looping.get_frame_at(0.25), 3);
        assert_eq!(looping.get_frame_at(1.0 / 12.0), 1, "the 1e-6 nudge lands exact frame times on the new frame");
        assert_eq!(looping.get_frame_at(1.0), 4);
        assert_eq!(hold.get_frame_at(1.0), 0);
        assert_eq!(hold.get_frame_at(2.5), 3);
        assert_eq!(hold.get_frame_at(9.0), 7);
        assert_eq!(stepped.get_frame_at(123.0), 5);
        assert!(SpriteShape::new(sheet(2), RectangleF::new(0.0, 0.0, 1.0, 1.0), SpriteShapeOptions { frames_per_second: -1.0, ..options }).is_err());
    }
}
