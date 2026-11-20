//! Small value types shared by every part of the library (C#: `Common/`, and the
//! `System.Drawing.RectangleF` the C# library takes).

mod color4;
mod rectangle_f;

pub use color4::Color4;
pub use rectangle_f::RectangleF;
