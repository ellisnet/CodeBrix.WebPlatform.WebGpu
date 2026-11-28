//! A layer's own timeline ([`CanvasLayers::set_time_scale`](crate::CanvasLayers::set_time_scale)):
//! the clock's time through a straight line whose slope is the layer's time scale. With the
//! default scale of 1 it IS the clock's time, value for value. Not in C#.

use crate::renderer::FrameInfo;

/// The line from the clock's time to one layer's: `layer_at + (clock - clock_at) x scale`, and the
/// clock's time of the layer's last frame (where a new scale takes over, so the layer's time does
/// not jump).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LayerTime {
    scale: f64,
    clock_at: f64,
    layer_at: f64,
    last_clock: Option<f64>,
}

impl Default for LayerTime {
    fn default() -> Self {
        LayerTime { scale: 1.0, clock_at: 0.0, layer_at: 0.0, last_clock: None }
    }
}

impl LayerTime {
    /// The layer's time at the clock's time `clock`.
    fn at(&self, clock: f64) -> f64 {
        self.layer_at + (clock - self.clock_at) * self.scale
    }

    /// The frame as the layer sees it at `scale`: its time, delta and previous time on the layer's
    /// line (the frame's number, size and pixel ratio are the clock's). A scale that differs from
    /// the last frame's continues from the layer's time at that frame; a layer that has not drawn
    /// yet takes the scale from the clock's time 0, so that its time is the scale times the
    /// clock's.
    pub(crate) fn frame(&mut self, scale: f64, info: FrameInfo) -> FrameInfo {
        if scale != self.scale {
            if let Some(last) = self.last_clock {
                self.layer_at = self.at(last);
                self.clock_at = last;
            }
            self.scale = scale;
        }
        self.last_clock = Some(info.time);
        FrameInfo { time: self.at(info.time), delta: info.delta * self.scale, previous_time: info.previous_time.map(|p| self.at(p)), ..info }
    }
}

#[cfg(test)]
mod tests {
    use super::LayerTime;
    use crate::renderer::FrameInfo;

    fn info(number: u64, time: f64, delta: f64, previous_time: Option<f64>) -> FrameInfo {
        FrameInfo { number, time, delta, previous_time, width: 800, height: 600, pixel_ratio: 2.0 }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn a_layer_at_the_default_scale_sees_the_clock_exactly() {
        //Arrange
        let mut time = LayerTime::default();
        let frames = [info(1, 0.0, 0.0, None), info(2, 0.1 + 0.2, 0.1 + 0.2, Some(0.0)), info(3, 1234.5678, 0.016, Some(1234.5518))];
        //Act + Assert: the same values, bit for bit
        for frame in frames {
            assert_eq!(time.frame(1.0, frame), frame);
        }
    }

    #[test]
    fn a_scaled_layer_runs_at_its_scale_of_the_clock() {
        let mut time = LayerTime::default();
        let first = time.frame(2.0, info(1, 0.0, 0.0, None));
        let second = time.frame(2.0, info(2, 0.5, 0.5, Some(0.0)));
        assert_eq!((first.time, first.delta, first.previous_time), (0.0, 0.0, None));
        assert_eq!((second.time, second.delta, second.previous_time), (1.0, 1.0, Some(0.0)));
        assert_eq!((second.number, second.width, second.height, second.pixel_ratio), (2, 800, 600, 2.0), "the rest is the clock's frame");
    }

    #[test]
    fn a_layer_scaled_before_its_first_frame_is_the_scale_times_the_clock() {
        //Arrange: a page opened at a later time (a seek before the first frame), or a layer added late
        let mut time = LayerTime::default();
        //Act
        let first = time.frame(0.5, info(40, 12.0, 0.0, Some(11.9)));
        //Assert: what playing from 0 would show
        assert!(close(first.time, 6.0) && close(first.previous_time.unwrap(), 5.95));
    }

    #[test]
    fn a_change_of_the_scale_goes_on_from_the_layers_time_without_a_jump() {
        //Arrange
        let mut time = LayerTime::default();
        time.frame(1.0, info(1, 0.0, 0.0, None));
        time.frame(1.0, info(2, 10.0, 10.0, Some(0.0)));
        //Act: three times as fast from the next frame
        let changed = time.frame(3.0, info(3, 10.5, 0.5, Some(10.0)));
        let later = time.frame(3.0, info(4, 11.0, 0.5, Some(10.5)));
        //Assert
        assert!(close(changed.previous_time.unwrap(), 10.0), "the last frame's time is kept");
        assert!(close(changed.time, 11.5) && close(changed.delta, 1.5));
        assert!(close(later.time, 13.0));
    }

    #[test]
    fn a_seek_and_a_resume_keep_their_delta_of_zero() {
        let mut time = LayerTime::default();
        time.frame(2.0, info(1, 0.0, 0.0, None));
        let seek = time.frame(2.0, info(2, 30.0, 0.0, Some(1.0)));
        assert_eq!((seek.time, seek.delta, seek.previous_time), (60.0, 0.0, Some(2.0)));
    }

    #[test]
    fn a_scale_of_zero_stands_still() {
        let mut time = LayerTime::default();
        time.frame(1.0, info(1, 0.0, 0.0, None));
        time.frame(1.0, info(2, 4.0, 4.0, Some(0.0)));
        let held = time.frame(0.0, info(3, 5.0, 1.0, Some(4.0)));
        let still = time.frame(0.0, info(4, 9.0, 4.0, Some(5.0)));
        assert_eq!((held.time, held.delta, still.time), (4.0, 0.0, 4.0));
    }
}
