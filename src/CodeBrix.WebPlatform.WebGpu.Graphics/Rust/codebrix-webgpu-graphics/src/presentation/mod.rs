//! What render passes draw into (C#: `Presentation/`): a target view and the offscreen target of
//! the HDR path. The canvas itself is the canvas host's (`codebrix-webgpu`); a frame on it is
//! `GraphicsContext::begin_canvas_frame`.

mod headless_render_target;
mod render_target_view;

pub use headless_render_target::{HeadlessRenderTarget, HeadlessTargetOptions};
pub use render_target_view::RenderTargetView;
