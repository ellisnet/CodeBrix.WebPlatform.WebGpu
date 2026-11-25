//! What render passes draw into (C#: `Presentation/`): a target view, the offscreen target of
//! the HDR path, and (Not in C#) the canvas frame target and the multisampling (MSAA) settings
//! they share. The canvas itself is the canvas host's (`codebrix-webgpu`); a frame on it is
//! `GraphicsContext::begin_canvas_frame` or `GraphicsContext::begin_canvas_frame_on`.

mod canvas_frame_target;
mod headless_render_target;
pub(crate) mod multisampling;
mod render_target_view;

pub(crate) use canvas_frame_target::CanvasLayerResolve;
pub use canvas_frame_target::{CanvasFrameTarget, CanvasFrameTargetOptions};
pub use headless_render_target::{HeadlessRenderTarget, HeadlessTargetOptions};
pub use multisampling::SAMPLE_COUNTS;
pub use render_target_view::{RenderTargetView, TargetFormats};
