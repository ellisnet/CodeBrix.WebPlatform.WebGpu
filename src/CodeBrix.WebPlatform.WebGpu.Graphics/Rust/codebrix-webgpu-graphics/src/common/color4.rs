//! [`Color4`]: a linear RGBA colour with `f32` channels (C#: `Color4`).

use std::fmt;

use crate::context::GraphicsException;

/// A colour with four `f32` channels, red, green, blue and alpha, usually in 0..1 (the C#
/// `Color4`). The library treats colours as LINEAR values: an sRGB view of a target encodes them
/// when they are written.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Color4 {
    /// Red.
    pub r: f32,
    /// Green.
    pub g: f32,
    /// Blue.
    pub b: f32,
    /// Alpha (1 is opaque).
    pub a: f32,
}

// SAFETY: four f32 fields with #[repr(C)] and no padding: every bit pattern is a valid value.
unsafe impl bytemuck::Zeroable for Color4 {}
// SAFETY: as above; the type is Copy and 'static.
unsafe impl bytemuck::Pod for Color4 {}

impl Color4 {
    /// Opaque white.
    pub const WHITE: Color4 = Color4::new(1.0, 1.0, 1.0, 1.0);
    /// Opaque black.
    pub const BLACK: Color4 = Color4::new(0.0, 0.0, 0.0, 1.0);
    /// Transparent black (every channel 0).
    pub const TRANSPARENT: Color4 = Color4::new(0.0, 0.0, 0.0, 0.0);
    /// Opaque red.
    pub const RED: Color4 = Color4::new(1.0, 0.0, 0.0, 1.0);
    /// Opaque green.
    pub const GREEN: Color4 = Color4::new(0.0, 1.0, 0.0, 1.0);
    /// Opaque blue.
    pub const BLUE: Color4 = Color4::new(0.0, 0.0, 1.0, 1.0);
    /// Opaque yellow.
    pub const YELLOW: Color4 = Color4::new(1.0, 1.0, 0.0, 1.0);
    /// Opaque cyan.
    pub const CYAN: Color4 = Color4::new(0.0, 1.0, 1.0, 1.0);
    /// Opaque magenta.
    pub const MAGENTA: Color4 = Color4::new(1.0, 0.0, 1.0, 1.0);
    /// Opaque mid grey (0.5 in every colour channel).
    pub const GRAY: Color4 = Color4::new(0.5, 0.5, 0.5, 1.0);
    /// Cornflower blue (100, 149, 237).
    pub const CORNFLOWER_BLUE: Color4 = Color4::new(100.0 / 255.0, 149.0 / 255.0, 237.0 / 255.0, 1.0);

    /// A colour from its four channels (C#: `new Color4(r, g, b, a)`; pass 1 for an opaque colour).
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Color4 {
        Color4 { r, g, b, a }
    }

    /// A colour from 0..255 channel bytes (C#: `FromBytes`; pass 255 for an opaque colour).
    pub fn from_bytes(r: u8, g: u8, b: u8, a: u8) -> Color4 {
        Color4::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0)
    }

    /// A colour from `#RRGGBB` or `#RRGGBBAA` (the `#` is optional) (C#: `FromHex`).
    pub fn from_hex(hex: &str) -> Result<Color4, GraphicsException> {
        let digits = hex.strip_prefix('#').unwrap_or(hex);
        let parsed = if (digits.len() == 6 || digits.len() == 8) && digits.chars().all(|c| c.is_ascii_hexdigit()) { u32::from_str_radix(digits, 16).ok() } else { None };
        let Some(mut value) = parsed else {
            return Err(GraphicsException::new(format!("'{hex}' is not #RRGGBB or #RRGGBBAA. Write six or eight hexadecimal digits, for example #6495ED.")));
        };
        if digits.len() == 6 {
            value = (value << 8) | 0xFF;
        }
        Ok(Color4::from_bytes((value >> 24) as u8, (value >> 16) as u8, (value >> 8) as u8, value as u8))
    }

    /// The same colour with another alpha (C#: `WithAlpha`).
    pub fn with_alpha(self, alpha: f32) -> Color4 {
        Color4::new(self.r, self.g, self.b, alpha)
    }

    /// The colour `amount` of the way from `from` to `to`, channel by channel (C#: `Lerp`).
    pub fn lerp(from: Color4, to: Color4, amount: f32) -> Color4 {
        Color4::new(from.r + (to.r - from.r) * amount, from.g + (to.g - from.g) * amount, from.b + (to.b - from.b) * amount, from.a + (to.a - from.a) * amount)
    }

    /// The channels as a vector (C#: `ToVector4`).
    pub fn to_vector4(self) -> glam::Vec4 {
        glam::Vec4::new(self.r, self.g, self.b, self.a)
    }

    /// The colour as four bytes packed red first in memory: red in the low byte (C#: `ToPackedRgba`).
    pub fn to_packed_rgba(self) -> u32 {
        to_byte(self.r) as u32 | (to_byte(self.g) as u32) << 8 | (to_byte(self.b) as u32) << 16 | (to_byte(self.a) as u32) << 24
    }

    /// The colour as the `wgpu` clear colour (the channels as `f64`).
    pub fn to_wgpu(self) -> wgpu::Color {
        wgpu::Color { r: self.r as f64, g: self.g as f64, b: self.b as f64, a: self.a as f64 }
    }
}

impl std::ops::Mul for Color4 {
    type Output = Color4;

    /// Channel-by-channel product (C#: `operator *`).
    fn mul(self, other: Color4) -> Color4 {
        Color4::new(self.r * other.r, self.g * other.g, self.b * other.b, self.a * other.a)
    }
}

impl fmt::Display for Color4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.r, self.g, self.b, self.a)
    }
}

impl From<Color4> for wgpu::Color {
    fn from(color: Color4) -> wgpu::Color {
        color.to_wgpu()
    }
}

/// A channel as a byte: rounded, clamped to 0..255.
fn to_byte(value: f32) -> u8 {
    (value * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::Color4;

    #[test]
    fn from_hex_reads_six_and_eight_digits() {
        assert_eq!(Color4::from_hex("#6495ED").unwrap(), Color4::CORNFLOWER_BLUE);
        assert_eq!(Color4::from_hex("ff000080").unwrap(), Color4::from_bytes(255, 0, 0, 128));
    }

    #[test]
    fn from_hex_refuses_other_text_and_says_what_to_write() {
        let error = Color4::from_hex("#12345").unwrap_err();
        assert!(error.to_string().contains("#RRGGBB or #RRGGBBAA"));
        assert!(Color4::from_hex("#GGGGGG").is_err());
    }

    #[test]
    fn packed_rgba_puts_red_in_the_low_byte_and_clamps() {
        assert_eq!(Color4::new(1.0, 0.0, 0.0, 1.0).to_packed_rgba(), 0xFF0000FF);
        assert_eq!(Color4::new(2.0, -1.0, 0.5, 0.0).to_packed_rgba(), 0x008000FF);
    }

    #[test]
    fn lerp_with_alpha_and_product() {
        let half = Color4::lerp(Color4::BLACK, Color4::WHITE, 0.5);
        assert_eq!(half, Color4::new(0.5, 0.5, 0.5, 1.0));
        assert_eq!(Color4::RED.with_alpha(0.25).a, 0.25);
        assert_eq!(Color4::YELLOW * Color4::CYAN, Color4::GREEN);
    }

    #[test]
    fn display_lists_the_channels() {
        assert_eq!(Color4::new(1.0, 0.5, 0.25, 1.0).to_string(), "(1, 0.5, 0.25, 1)");
    }
}
