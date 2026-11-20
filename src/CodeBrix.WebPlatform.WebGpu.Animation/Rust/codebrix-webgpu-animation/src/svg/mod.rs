//! SVG documents: a document's TEXT (or bytes, from anywhere) turned,
//! through usvg's resolved tree, into a scene of the renderer's drawing primitives as plain data
//! ([`SvgScene`]), every element id addressable as a PART with a program-driven override
//! ([`DrawingSvgPart`], C# `DrawingSvgPart`) and bindable to tracks
//! ([`DrawingSvgShape::bind_part`]), and the document's OWN animations - time-based SMIL and a
//! subset of CSS `@keyframes` - evaluated for a time passed in into per-part overrides
//! ([`SvgAnimationFrame`]). [`DrawingSvgShape::draw_list`] walks it all into
//! [`SvgDrawCommand`]s. Nothing here reads a clock, a file or the network. As an element of a
//! drawing session ([`DrawingSession::draw_svg`](crate::DrawingSession::draw_svg)) the draw list
//! of the session's time is mapped command by command onto the renderer; `<text>` is laid out by
//! usvg with the fonts the application supplies ([`DrawingSvgOptions::fonts`]) and drawn as the
//! paths usvg makes of it.
//!
//! What is drawn, approximated, ignored and reported: [`DrawingSvg::report`].

pub(crate) mod animation;
mod drawing_svg;
mod drawing_svg_aspect;
mod drawing_svg_options;
mod drawing_svg_part;
mod drawing_svg_session;
mod drawing_svg_shape;
mod drawing_svg_warning;
mod scene_builder;
mod scene_walker;
mod svg_draw_command;
mod svg_encode;
mod svg_group_node;
mod svg_leaf_nodes;
mod svg_paint;
mod svg_part_override;
mod svg_scene;
mod svg_stroke;

#[cfg(test)]
mod tests_scene;
#[cfg(test)]
mod tests_smil;
#[cfg(test)]
mod tests_css;
#[cfg(test)]
mod tests_draw;

pub use animation::SvgAnimations;
pub use drawing_svg::{DrawingSvg, SYNTHETIC_ID_PREFIX};
pub use drawing_svg_aspect::DrawingSvgAspect;
pub use drawing_svg_options::DrawingSvgOptions;
pub use drawing_svg_part::DrawingSvgPart;
pub use drawing_svg_shape::DrawingSvgShape;
pub use drawing_svg_warning::{DrawingSvgWarning, DrawingSvgWarningKind};
pub use svg_draw_command::{SvgClipShape, SvgDrawCommand, SvgDrawList};
pub use svg_part_override::{SvgAnimationFrame, SvgPartOverride};
pub use svg_group_node::{SvgBlendMode, SvgClip, SvgGroupNode, SvgMask, SvgMaskKind};
pub use svg_leaf_nodes::{SvgImageNode, SvgImageSource, SvgPathNode};
pub use svg_paint::{SvgFill, SvgFillRule, SvgGradientStop, SvgLinearGradient, SvgPaint, SvgRadialGradient, SvgSpread};
pub use svg_scene::{SvgNodeKind, SvgPartBase, SvgPartInfo, SvgScene, SvgSceneNode};
pub use svg_stroke::{SvgLineCap, SvgLineJoin, SvgPaintOrder, SvgStroke};
