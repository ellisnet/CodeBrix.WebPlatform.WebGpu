//! `Color`: a straight-alpha sRGB colour of 8 bits per channel (the role CodeBrix.Imaging's
//! `Color` plays in the C# API, with `Color.FromRgb` / `Color.FromRgba` / `ToPixel<Rgba32>()`).

/// A colour: straight (not premultiplied) alpha, sRGB-encoded, 8 bits per channel - what every
/// public colour of this crate is. The renderer converts it to linear premultiplied values on
/// the GPU (the canvas host's convention).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color {
    /// Red, 0..255 (sRGB-encoded).
    pub r: u8,
    /// Green, 0..255 (sRGB-encoded).
    pub g: u8,
    /// Blue, 0..255 (sRGB-encoded).
    pub b: u8,
    /// Alpha, 0 (transparent) ..255 (opaque); straight, not premultiplied.
    pub a: u8,
}

impl Color {
    /// Transparent black.
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0 };
    /// Opaque black.
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    /// Opaque white.
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };

    /// An opaque colour (C# `Color.FromRgb`).
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    /// A colour with alpha (C# `Color.FromRgba`).
    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }

    /// The same colour with another alpha.
    pub const fn with_alpha(self, a: u8) -> Color {
        Color { a, ..self }
    }

    /// The four bytes R, G, B, A (C# `ToPixel<Rgba32>()`).
    pub const fn to_rgba32(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// The colour as peniko's colour type (what the renderer and velato use), its alpha scaled
    /// by `opacity` (0..1).
    pub fn to_peniko(self, opacity: f32) -> peniko::color::AlphaColor<peniko::color::Srgb> {
        peniko::color::AlphaColor::new([self.r as f32 / 255.0, self.g as f32 / 255.0, self.b as f32 / 255.0, self.a as f32 / 255.0 * opacity.clamp(0.0, 1.0)])
    }
}
