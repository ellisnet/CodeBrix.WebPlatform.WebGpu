//! Masks and effects (C# `CodeBrix.Imaging.Drawing.Effects`): [`DrawingMask`] (what part of a
//! layer's or group's content is shown - inside or outside a path, or where another layer's
//! elements are), and [`DrawingEffects`] (blur, glow and drop shadow of an element, a group or a
//! layer, with [`ShadowDirection`]); masks from pixel planes too; and [`DrawingFrameOperations`], the planes'
//! whole-frame operations (tone curve, gamma, vignette, bloom) as GPU passes over a layer. The renderer makes both on the GPU: masks as offscreen
//! groups composited destination-in or destination-out (or a stencil clip for "inside a path"),
//! effects as an offscreen pass with separable Gaussian blurs. And (no C# equivalent) a layer's
//! [`SurfaceFinish`]: the canvas weave and the paint's thickness, lit, on the layer's effect group.

mod drawing_effects;
mod drawing_frame_operations;
mod drawing_mask;
mod drawing_mask_kind;
pub(crate) mod effect_sizes;
mod shadow_direction;
pub(crate) mod surface_finish;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_step9;
#[cfg(test)]
mod tests_surface_finish;

pub use drawing_effects::DrawingEffects;
pub use drawing_frame_operations::{DrawingFrameOperations, FrameBloom, FrameVignette};
pub use drawing_mask::DrawingMask;
pub use drawing_mask_kind::DrawingMaskKind;
pub use shadow_direction::ShadowDirection;
pub use surface_finish::{CanvasWeave, SurfaceFinish};
