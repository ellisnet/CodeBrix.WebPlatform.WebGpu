//! `SafeAreaCrop`: a centred crop a platform shows of the frame (C# `SafeAreaCrop`).

use crate::geometry::RectangleF;

/// A centred crop of an aspect ratio a platform shows of the frame - a profile grid's 3:4, a
/// feed's 4:5 (C# `SafeAreaCrop`): what lies outside it is unsafe.
#[derive(Clone, Debug, PartialEq)]
pub struct SafeAreaCrop {
    name: String,
    aspect_width: f32,
    aspect_height: f32,
}

impl SafeAreaCrop {
    /// A crop (C#'s constructor). The error says what is wrong.
    pub fn new(name: &str, aspect_width: f32, aspect_height: f32) -> Result<SafeAreaCrop, String> {
        if name.trim().is_empty() {
            return Err("A crop name is required.".to_string());
        }
        if !(aspect_width > 0.0) || aspect_width.is_infinite() || !(aspect_height > 0.0) || aspect_height.is_infinite() {
            return Err("The aspect width and height must be positive, finite numbers.".to_string());
        }
        Ok(SafeAreaCrop { name: name.trim().to_string(), aspect_width, aspect_height })
    }

    /// The name (C# `Name`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The aspect's width (C# `AspectWidth`).
    pub fn aspect_width(&self) -> f32 {
        self.aspect_width
    }

    /// The aspect's height (C# `AspectHeight`).
    pub fn aspect_height(&self) -> f32 {
        self.aspect_height
    }

    /// The rectangle the crop keeps of a frame, centred (C# `GetKeptRectangle`).
    pub fn get_kept_rectangle(&self, frame_size: (f32, f32)) -> RectangleF {
        let aspect = self.aspect_width / self.aspect_height;
        let mut width = frame_size.0;
        let mut height = width / aspect;
        if height > frame_size.1 {
            height = frame_size.1;
            width = height * aspect;
        }
        RectangleF::new((frame_size.0 - width) / 2.0, (frame_size.1 - height) / 2.0, width, height)
    }
}
