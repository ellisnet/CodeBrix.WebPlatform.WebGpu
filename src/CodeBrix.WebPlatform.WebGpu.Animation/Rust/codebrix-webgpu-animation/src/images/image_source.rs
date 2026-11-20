//! `ImageSource`: a picture as DECODED pixels (C# `ImageSource`).

use std::sync::Arc;

use peniko::{ImageAlphaType, ImageData, ImageFormat};

use crate::geometry::Rectangle;

/// A picture to draw with an [`ImageShape`](crate::shapes::ImageShape) (or as the frames of a
/// [`SpriteSheet`](crate::images::SpriteSheet)): a copy of straight-alpha RGBA pixels (C#
/// `ImageSource`). The renderer premultiplies them on the GPU.
///
/// THREE WAYS TO SUPPLY A PICTURE. (1) DECODED PIXELS: [`ImageSource::from_rgba`] and its
/// siblings take width x height x 4 bytes made in the frontend's own code or decoded elsewhere.
/// (2) THE PICTURE-PIXELS FORM: [`ImageSource::from_picture_pixels`] reads what the .NET
/// `PicturePixelsWriter` of the CodeBrix.WebPlatform.WebGpu.Animation package writes - a backend
/// decodes any format it likes (with CodeBrix.Imaging, for example) and sends the pixels through
/// the platform's generated `_bytes` call. (3) A PNG FILE: [`ImageSource::from_png`] decodes the
/// file's bytes in the browser (PNG is the ONLY file format this crate reads). C#'s `FromEncoded`
/// / `FromFile` / `FromImagingImage` have no equivalent here (no file access, no JPEG or other
/// decoder); C#'s `FromPlaneImage` is [`ImageSource::from_plane_image`].
///
/// SHARING. A clone is the SAME picture (C# shares one reference): the pixels are kept once, and
/// the renderer uploads them to the GPU once and keeps the texture while any shape draws it.
/// Dropping the last clone releases the pixels (C# `Dispose`); the GPU texture goes once no frame
/// has used it for a while.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageSource {
    data: ImageData,
}

/// Checks the buffer against the size; returns the byte length the picture needs.
fn check_buffer(available: usize, width: u32, height: u32) -> Result<usize, String> {
    if width < 1 || height < 1 {
        return Err(format!("A picture needs a width and a height of at least 1 pixel (it was {width} x {height})."));
    }
    let required = width as u64 * height as u64 * 4;
    if (available as u64) < required {
        return Err(format!("The buffer holds {available} bytes but a {width} x {height} picture needs {required}: pass width x height x 4 bytes (R, G, B, A per pixel, rows top to bottom)."));
    }
    Ok(required as usize)
}

impl ImageSource {
    fn from_vec(rgba: Vec<u8>, width: u32, height: u32) -> ImageSource {
        ImageSource { data: ImageData { data: peniko::Blob::new(Arc::new(rgba)), format: ImageFormat::Rgba8, alpha_type: ImageAlphaType::Alpha, width, height } }
    }

    /// Copies straight-alpha RGBA pixels (R, G, B, A per pixel, sRGB-encoded, rows top to bottom,
    /// no padding; C# `FromRgba`). The error says what is wrong with the buffer.
    pub fn from_rgba(rgba: &[u8], width: u32, height: u32) -> Result<ImageSource, String> {
        let length = check_buffer(rgba.len(), width, height)?;
        Ok(ImageSource::from_vec(rgba[..length].to_vec(), width, height))
    }

    /// Copies straight-alpha BGRA pixels (B, G, R, A per pixel; C# `FromBgra`).
    pub fn from_bgra(bgra: &[u8], width: u32, height: u32) -> Result<ImageSource, String> {
        let length = check_buffer(bgra.len(), width, height)?;
        let mut rgba = bgra[..length].to_vec();
        for px in rgba.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        Ok(ImageSource::from_vec(rgba, width, height))
    }

    /// Takes ownership of straight-alpha RGBA pixels without copying them (the Rust form of
    /// `FromRgba` for a buffer the caller no longer needs).
    pub fn from_rgba_vec(rgba: Vec<u8>, width: u32, height: u32) -> Result<ImageSource, String> {
        let length = check_buffer(rgba.len(), width, height)?;
        let mut rgba = rgba;
        rgba.truncate(length);
        Ok(ImageSource::from_vec(rgba, width, height))
    }

    /// Copies a float plane image, converted with the planes' float-to-byte rule (C#
    /// `FromPlaneImage`): straight-alpha RGBA, each channel `(byte)(v * 255 + 0.5)` clamped.
    pub fn from_plane_image(image: &crate::planes::DrawingPlaneImage) -> Result<ImageSource, String> {
        ImageSource::from_rgba_vec(image.to_rgba(), image.width(), image.height())
    }

    /// The width, in pixels.
    pub fn width(&self) -> u32 {
        self.data.width
    }

    /// The height, in pixels.
    pub fn height(&self) -> u32 {
        self.data.height
    }

    /// The size, in pixels (width, height).
    pub fn size(&self) -> (u32, u32) {
        (self.data.width, self.data.height)
    }

    /// A new picture holding a copy of part of this one (C# `Crop`; sprite sheets cut their cells
    /// with it). The error says what to do when the area is empty or reaches outside.
    pub fn crop(&self, area: Rectangle) -> Result<ImageSource, String> {
        self.check_area(area)?;
        let all = self.data.data.data();
        let row = self.data.width as usize * 4;
        let mut rgba = Vec::with_capacity(area.width as usize * area.height as usize * 4);
        for y in 0..area.height as usize {
            let start = (area.y as usize + y) * row + area.x as usize * 4;
            rgba.extend_from_slice(&all[start..start + area.width as usize * 4]);
        }
        Ok(ImageSource::from_vec(rgba, area.width as u32, area.height as u32))
    }

    /// A copy of the pixels as straight-alpha RGBA bytes (C# `ToRgba`).
    pub fn to_rgba(&self) -> Vec<u8> {
        self.data.data.data().to_vec()
    }

    /// The error for an area that is empty or reaches outside the picture.
    pub(crate) fn check_area(&self, area: Rectangle) -> Result<(), String> {
        if area.width < 1 || area.height < 1 || area.x < 0 || area.y < 0 || area.right() > self.data.width as i32 || area.bottom() > self.data.height as i32 {
            return Err(format!("The area {area:?} must be at least 1 x 1 pixel and lie inside the {} x {} picture.", self.data.width, self.data.height));
        }
        Ok(())
    }

    /// The pixels as the renderer takes them.
    pub(crate) fn image_data(&self) -> &ImageData {
        &self.data
    }
}
