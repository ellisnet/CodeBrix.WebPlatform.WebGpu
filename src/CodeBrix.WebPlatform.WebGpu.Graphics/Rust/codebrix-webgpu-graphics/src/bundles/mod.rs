//! Render bundles: pre-recorded draw commands a render pass replays (C#: `Bundles/`).

mod gpu_render_bundle;
mod render_bundle_builder;

pub use gpu_render_bundle::GpuRenderBundle;
pub use render_bundle_builder::{RenderBundleBuilder, RenderBundleOptions};
