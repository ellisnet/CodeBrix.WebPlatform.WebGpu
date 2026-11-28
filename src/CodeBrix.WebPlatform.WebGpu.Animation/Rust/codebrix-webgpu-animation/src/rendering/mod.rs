//! Drawing on the GPU: the calibration of a drawing into a canvas, the frame's
//! [`VectorScene`], [`AnimationLayer`] (the renderer as a layer of the canvas host), the
//! [`OffscreenRenderer`] (the same renderer drawing into a texture of the application's, with no
//! canvas), the [`PictureTarget`] a layer can draw into for other drawing to read, and the
//! [`DebugOverlay`].

mod animation_layer;
mod canvas_calibration;
mod core;
pub(crate) mod coverage_collector;
mod cpu_clock;
mod debug_overlay;
mod offscreen_renderer;
pub(crate) mod picture_target;
pub(crate) mod scene_look;
pub(crate) mod hard_edge_rasterizer;
mod vector_scene;

pub use animation_layer::{AnimationFrame, AnimationLayer, AnimationLayerControl};
pub use canvas_calibration::CanvasCalibration;
pub use core::RenderStats;
pub use debug_overlay::{DebugOverlay, DebugOverlayCorner};
pub use offscreen_renderer::OffscreenRenderer;
pub use picture_target::PictureTarget;
pub use vector_scene::VectorScene;
pub(crate) use vector_scene::WaterDraw;
pub(crate) use vector_scene::Hasher as GeometryKey;
#[cfg(test)]
pub(crate) use vector_scene::Item as SceneItem;
#[cfg(test)]
pub(crate) use self::core::TestBuilder;
#[cfg(test)]
pub(crate) use self::core::TestStrokeSource;
#[cfg(test)]
pub(crate) use self::core::{TestCompositeMode, TestOp, TestWaterBinding};
pub(crate) use core::srgb_to_linear as core_paint_srgb_to_linear;
pub(crate) use self::core::FxPassData as core_fx_pass_data;
