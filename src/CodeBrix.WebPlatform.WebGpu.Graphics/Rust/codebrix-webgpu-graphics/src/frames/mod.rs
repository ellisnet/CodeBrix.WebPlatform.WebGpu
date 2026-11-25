//! Frames, render and compute passes, and the indirect argument records: recording a frame's
//! commands (C#: `Frames/`).

mod compute_pass;
mod graphics_frame;
mod graphics_frame_commands;
pub(crate) mod indirect_args;
mod pass_attachments;
mod render_pass;

pub use compute_pass::{ComputePass, ComputePassOptions};
pub use graphics_frame::{GraphicsFrame, RenderPassOptions};
pub use indirect_args::{DispatchIndirectArgs, DrawIndexedIndirectArgs, DrawIndirectArgs};
pub use render_pass::{BufferRange, DepthRange, DrawIndexedOptions, DrawOptions, RenderPass};
