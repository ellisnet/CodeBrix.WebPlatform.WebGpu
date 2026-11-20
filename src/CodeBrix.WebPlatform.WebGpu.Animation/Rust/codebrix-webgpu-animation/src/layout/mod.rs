//! Layout of CodeBrix.Imaging.Drawing's `Layout` namespace: anchors and positions against the
//! frame or a safe rectangle, so one scene description fits any frame shape.

mod drawing_layout;
mod drawing_layout_guide;
mod layout_anchor;
mod layout_position;
mod layout_reference;

pub use drawing_layout::DrawingLayout;
pub use drawing_layout_guide::DrawingLayoutGuideOptions;
pub use layout_anchor::LayoutAnchor;
pub use layout_position::{LayoutPosition, LayoutPositionOptions};
pub use layout_reference::LayoutReference;
