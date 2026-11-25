//! The frame-timer factory of [`GraphicsContext`] (Not in C#).

use crate::context::{GraphicsContext, GraphicsException};
use crate::queries::{FrameTimer, FrameTimerOptions};

impl GraphicsContext {
    /// A [`FrameTimer`]: CPU time per frame, and GPU time per marked pass where the device has
    /// `timestamp-query` (Not in C#). A device without the feature still gets a timer, whose GPU
    /// values are "not available"; the only error is a `max_passes` outside 1 to
    /// [`FrameTimer::MAX_PASSES`].
    pub fn create_frame_timer(&self, options: FrameTimerOptions) -> Result<FrameTimer, GraphicsException> {
        FrameTimer::new(self, &options)
    }
}
