//! Drawing on the GPU: the calibration of a drawing into a canvas, the frame's
//! [`VectorScene`], [`AnimationLayer`] (the renderer as a layer of the canvas host) and the
//! [`DebugOverlay`].

mod animation_layer;
mod canvas_calibration;
mod core;
pub(crate) mod coverage_collector;
mod debug_overlay;
pub(crate) mod hard_edge_rasterizer;
mod vector_scene;

pub use animation_layer::{AnimationFrame, AnimationLayer, AnimationLayerControl};
pub use canvas_calibration::CanvasCalibration;
pub use core::RenderStats;
pub use debug_overlay::{DebugOverlay, DebugOverlayCorner};
pub use vector_scene::VectorScene;
pub(crate) use vector_scene::Hasher as GeometryKey;
pub(crate) use core::srgb_to_linear as core_paint_srgb_to_linear;
