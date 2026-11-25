//! The shapes of CodeBrix.Imaging.Drawing's `Shapes` namespace that the steps so far reach
//! (rectangle, circle, ellipse, line, arrow, polyline, path, the single line of text, the text block, the group,
//! the animated shape, the picture, the sprite, the painterly brushstroke, a stroke set drawn on the GPU, noise drawn on the GPU and particles), and `DrawingShape`, what a layer holds.

mod animated_shape;
mod arrow_shape;
mod basic_shapes;
mod brush_stroke_shape;
pub(crate) mod brush_strokes_shape;
mod drawing_group;
mod drawing_shape;
mod drawing_shape_coverage;
mod image_shape;
pub(crate) mod noise_shape;
pub(crate) mod ownership_shape;
pub(crate) mod particles_shape;
mod path_shape;
mod shape_options;
mod sprite_shape;
mod text_block_shape;
mod text_shape;

#[cfg(test)]
mod tests_text_block;

pub use animated_shape::AnimatedShape;
pub use arrow_shape::{ArrowShape, ArrowShapeOptions};
pub use basic_shapes::{CircleShape, EllipseShape, LineShape, PolylineShape, RectangleShape};
pub use brush_stroke_shape::{BrushStrokeShape, BrushStrokeShapeOptions};
pub use brush_strokes_shape::{BrushStrokesOptions, BrushStrokesShape};
pub use drawing_group::DrawingGroup;
pub use drawing_shape::{DrawingShape, ShapeKind};
pub(crate) use drawing_shape::{encode_masked, EncodeContext};
pub use image_shape::{ImageShape, ImageShapeOptions};
pub use noise_shape::{NoiseShape, NoiseShapeOptions};
pub use ownership_shape::{OwnershipShape, OwnershipShapeOptions};
pub use particles_shape::ParticlesShape;
pub use path_shape::PathShape;
pub use shape_options::{LineShapeOptions, PolylineShapeOptions, RectangleShapeOptions, ShapeOptions};
pub use sprite_shape::{SpriteShape, SpriteShapeOptions};
pub use text_block_shape::{RevealedCluster, TextBlockShape, TextBlockStrokeOptions, TextPathPiece};
pub use text_shape::{TextShape, TextShapeOptions};
