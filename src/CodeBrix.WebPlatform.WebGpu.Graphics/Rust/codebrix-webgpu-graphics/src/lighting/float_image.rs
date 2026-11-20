//! [`FloatImage`]: a linear-light floating-point RGB image (C#: `FloatImage`).

use crate::context::GraphicsException;

/// A linear-light floating-point RGB image: three floats (R, G, B) per pixel, row-major from the
/// top-left (C#: `FloatImage`). The in-memory form of a high-dynamic-range picture (a Radiance
/// `.hdr` file, see [`crate::RadianceHdrDecoder`]) before it becomes an
/// [`crate::EnvironmentMap`]. Values are radiance-like: 1 is "white", the sun in an outdoor
/// panorama may be thousands.
#[derive(Clone, Debug, PartialEq)]
pub struct FloatImage {
    width: u32,
    height: u32,
    pixels: Vec<f32>,
}

impl FloatImage {
    /// An image over an RGB pixel buffer (C#: `new FloatImage(width, height, pixels)`). An error
    /// when a size is 0 or the buffer does not hold width x height x 3 floats.
    pub fn new(width: u32, height: u32, pixels: Vec<f32>) -> Result<FloatImage, GraphicsException> {
        if width < 1 || height < 1 {
            return Err(GraphicsException::new(format!("A float image must be at least 1 x 1 pixels; it was asked for {width} x {height}.")));
        }
        let needed = width as u64 * height as u64 * 3;
        if pixels.len() as u64 != needed {
            return Err(GraphicsException::new(format!("The pixel buffer holds {} floats; a {width} x {height} RGB image needs {needed}.", pixels.len())));
        }
        Ok(FloatImage { width, height, pixels })
    }

    /// A black image (C#: `new FloatImage(width, height)`).
    pub fn black(width: u32, height: u32) -> Result<FloatImage, GraphicsException> {
        FloatImage::new(width, height, vec![0.0; width as usize * height as usize * 3])
    }

    /// An image from RGBA floats (four per pixel), dropping alpha (C#: `FromRgba`).
    pub fn from_rgba(rgba: &[f32], width: u32, height: u32) -> Result<FloatImage, GraphicsException> {
        let needed = width as u64 * height as u64 * 4;
        if rgba.len() as u64 != needed {
            return Err(GraphicsException::new(format!("The pixel buffer holds {} floats; a {width} x {height} RGBA image needs {needed}.", rgba.len())));
        }
        let rgb = rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        FloatImage::new(width, height, rgb)
    }

    /// The width in pixels (C#: `Width`).
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels (C#: `Height`).
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The pixels: three floats per pixel, row-major from the top-left (C#: `Pixels`).
    pub fn pixels(&self) -> &[f32] {
        &self.pixels
    }

    /// The pixels, to change in place.
    pub fn pixels_mut(&mut self) -> &mut [f32] {
        &mut self.pixels
    }

    /// One pixel as (r, g, b); row 0 is the top (C#: `GetPixel(x, y, out r, out g, out b)`).
    pub fn get_pixel(&self, x: u32, y: u32) -> (f32, f32, f32) {
        let index = (y as usize * self.width as usize + x as usize) * 3;
        (self.pixels[index], self.pixels[index + 1], self.pixels[index + 2])
    }

    /// Writes one pixel (C#: `SetPixel`).
    pub fn set_pixel(&mut self, x: u32, y: u32, r: f32, g: f32, b: f32) {
        let index = (y as usize * self.width as usize + x as usize) * 3;
        self.pixels[index] = r;
        self.pixels[index + 1] = g;
        self.pixels[index + 2] = b;
    }
}

#[cfg(test)]
mod tests {
    use super::FloatImage;

    #[test]
    fn new_checks_the_buffer_length() {
        assert!(FloatImage::new(2, 1, vec![0.0; 6]).is_ok());
        let error = FloatImage::new(2, 1, vec![0.0; 5]).unwrap_err();
        assert!(error.to_string().contains("needs 6"), "{error}");
        assert!(FloatImage::new(0, 1, Vec::new()).is_err());
    }

    #[test]
    fn pixels_are_read_and_written_row_major() {
        let mut image = FloatImage::black(3, 2).unwrap();
        image.set_pixel(2, 1, 1.0, 2.0, 3.0);
        assert_eq!(image.get_pixel(2, 1), (1.0, 2.0, 3.0));
        assert_eq!(&image.pixels()[15..18], &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn from_rgba_drops_alpha() {
        let image = FloatImage::from_rgba(&[1.0, 2.0, 3.0, 9.0, 4.0, 5.0, 6.0, 9.0], 2, 1).unwrap();
        assert_eq!(image.pixels(), &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert!(FloatImage::from_rgba(&[1.0; 7], 2, 1).is_err());
    }
}
