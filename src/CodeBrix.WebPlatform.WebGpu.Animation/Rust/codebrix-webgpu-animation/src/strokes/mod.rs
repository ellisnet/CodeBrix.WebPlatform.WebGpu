//! Stroke sets (no C# equivalent): brushstrokes as PLAIN RECORDS ([`BrushStrokeRecord`]) in an
//! immutable, shareable set ([`BrushStrokeSet`]) - the data a painting made of tens of thousands
//! of strokes is planned in, and what the renderer uploads once and draws with every stroke's
//! painting-on worked out from the time. A record has the geometry of a
//! [`BrushStrokeShape`](crate::shapes::BrushStrokeShape) (centre, length, width, angle, bend,
//! profile), its colour, how much paint it carries, and when it is painted; its CPU twin is the
//! `BrushStrokeShape` it draws at a time ([`BrushStrokeRecord::to_shape`]).
//!
//! The module is apart from [`painting`](crate::painting), which is NoSkia's namespace with
//! values pinned to NoSkia's; this one has no C# counterpart.
//!
//! DRAWN ON THE GPU by a [`BrushStrokesShape`](crate::shapes::BrushStrokesShape)
//! (`DrawingSession::draw_brush_strokes`): the records are uploaded ONCE per set and every stroke
//! is drawn by one instanced draw call, its painting-on, its [anchors](StrokeAnchor) (strokes that
//! travel with moving elements), its [boil](StrokeBoil) and its [flow](StrokeFlow) worked out on
//! the GPU from the session time - the CPU's work per frame does not grow with the number of
//! strokes. Strokes may be painted with the application's [brush tips](BrushTips), and take
//! their colours from a LIVE PICTURE ([`StrokeColor::Picked`]: a picture, a
//! [`PictureTarget`](crate::rendering::PictureTarget) another session draws this frame, or a
//! texture the application hands over - a 3D render), picked on the GPU every frame and passed
//! through the [colour rules](crate::painting::ColorRules).
//!
//! Every value here is a function of what is passed in: a record's progress is a function of the
//! time it is asked for, nothing reads a clock, nothing is random.

pub(crate) mod brush_stroke_morph;
mod brush_stroke_record;
mod brush_stroke_set;
mod brush_tips;
pub(crate) mod stroke_anchor;
pub(crate) mod stroke_boil;
pub(crate) mod stroke_checkpoints;
pub(crate) mod stroke_color;
pub(crate) mod stroke_color_gpu;
mod stroke_flow;
pub(crate) mod stroke_gpu;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_checkpoints;
#[cfg(test)]
mod tests_color_gpu;
#[cfg(test)]
mod tests_gpu;
#[cfg(test)]
mod tests_morph;

pub use brush_stroke_morph::{BrushStrokeMorph, StrokePair, StrokePairingOptions};
pub use brush_stroke_record::BrushStrokeRecord;
pub use brush_stroke_set::BrushStrokeSet;
pub use brush_tips::{BrushTip, BrushTips};
pub use stroke_anchor::StrokeAnchor;
pub use stroke_boil::{boil_epoch, StrokeBoil};
pub use stroke_checkpoints::StrokeCheckpoints;
pub use stroke_color::{ExternalPicture, PictureEncoding, StrokeColor, StrokeColorSource, StrokePickup};
pub use stroke_flow::StrokeFlow;
