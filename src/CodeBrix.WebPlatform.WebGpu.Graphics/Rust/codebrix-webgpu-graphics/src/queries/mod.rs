//! Queries: occlusion and timestamp query sets, pass timestamps and the GPU timer (C#:
//! `Queries/`), and the frame timer (Not in C#).

mod cpu_clock;
mod frame_timer;
mod gpu_query_set;
mod gpu_timer;
mod graphics_context_frame_timer;
mod graphics_context_queries;

pub use frame_timer::{FrameTimer, FrameTimerOptions, FrameTimings, PassTiming};
pub use gpu_query_set::{GpuQuerySet, PassTimestamps};
pub use gpu_timer::GpuTimer;
