//! `ImageShape`: a picture drawn into a destination rectangle (C# `ImageShape`) - or (Not in C#)
//! a [`PictureTarget`] another layer draws this frame.

use kurbo::{Affine, Rect};

use crate::color::Color;
use crate::geometry::{Rectangle, RectangleF};
use crate::images::{ImageSampling, ImageSource};
use crate::rendering::{PictureTarget, VectorScene};

/// The optional arguments of [`ImageShape::new`] (C#'s defaults: all of the picture, bilinear,
/// no tint).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImageShapeOptions {
    /// The part of the picture to draw, in its pixels; None for all of it.
    pub source_rectangle: Option<Rectangle>,
    /// How pixels are sampled when scaled or turned.
    pub sampling: ImageSampling,
    /// A colour each pixel is multiplied by, channel by channel, alpha included; None for none.
    pub tint: Option<Color>,
}

/// A picture drawn into a destination rectangle: all of an [`ImageSource`] or part of it,
/// stretched to fit, sampled nearest or bilinear, optionally tinted (C# `ImageShape`). As an
/// element it takes a transform, an opacity, a time window, effects, and may sit in groups; the
/// layer colour is not used. The picture's edges are anti-aliased when it is turned or placed
/// between pixels. The tint can be bound to a colour track (`AnimatedProperty::Tint`).
///
/// Fit and alignment: like C#, the picture is STRETCHED to the destination; to keep its aspect,
/// give a destination of the same aspect (or a source rectangle of the destination's aspect).
#[derive(Clone, Debug, PartialEq)]
pub struct ImageShape {
    source: ImageSource,
    destination: RectangleF,
    source_rectangle: Option<Rectangle>,
    sampling: ImageSampling,
    tint: Option<Color>,
    /// The picture target shown instead of `source` (Not in C#).
    picture: Option<PictureTarget>,
}

pub(crate) fn check_destination(destination: RectangleF) -> Result<(), String> {
    if [destination.x, destination.y, destination.width, destination.height].iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err("The destination rectangle must be finite.".to_string())
    }
}

/// Records a picture (or part of it) into a destination: the rectangle filled with the picture
/// as its paint, the picture's pixels mapped onto it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_picture(scene: &mut VectorScene, transform: Affine, source: &ImageSource, destination: RectangleF, part: Option<Rectangle>, sampling: ImageSampling, tint: Option<Color>, opacity: f32) {
    encode_image_data(scene, transform, source.image_data(), destination, part, sampling, tint, opacity);
}

/// The same for the picture's data (an `ImageSource`'s, or a picture target's stand-in).
#[allow(clippy::too_many_arguments)]
fn encode_image_data(scene: &mut VectorScene, transform: Affine, image: &peniko::ImageData, destination: RectangleF, part: Option<Rectangle>, sampling: ImageSampling, tint: Option<Color>, opacity: f32) {
    if destination.width == 0.0 || destination.height == 0.0 {
        return;
    }
    let r = part.unwrap_or(Rectangle::new(0, 0, image.width as i32, image.height as i32));
    let dest = Rect::new(destination.x as f64, destination.y as f64, destination.right() as f64, destination.bottom() as f64);
    let place = Affine::translate((dest.x0, dest.y0)) * Affine::scale_non_uniform(destination.width as f64 / r.width as f64, destination.height as f64 / r.height as f64) * Affine::translate((-(r.x as f64), -(r.y as f64)));
    let tint = tint.map(|t| [t.r as f32 / 255.0, t.g as f32 / 255.0, t.b as f32 / 255.0, t.a as f32 / 255.0]);
    scene.fill_picture(transform, image, place, &dest.abs(), sampling == ImageSampling::Nearest, tint, opacity);
}

impl ImageShape {
    /// A picture element (C# `new ImageShape(source, destination, sourceRectangle = null,
    /// sampling = Linear, tint = null)`). The error says what is wrong with the destination or the
    /// source rectangle.
    pub fn new(source: ImageSource, destination: RectangleF, options: ImageShapeOptions) -> Result<ImageShape, String> {
        check_destination(destination)?;
        if let Some(r) = options.source_rectangle {
            source.check_area(r)?;
        }
        Ok(ImageShape { source, destination, source_rectangle: options.source_rectangle, sampling: options.sampling, tint: options.tint, picture: None })
    }

    /// A picture element showing a [`PictureTarget`] - the picture another layer
    /// ([`AnimationLayer::to_picture`](crate::rendering::AnimationLayer::to_picture)) draws this
    /// frame - stretched into `destination` (no C# equivalent). Its [`source`](ImageShape::source)
    /// is a 1 x 1 transparent stand-in (the pixels are on the GPU only); the flat look paints it
    /// transparent and the line look leaves it out, as it has no CPU pixels to average. The error
    /// says what is wrong with the destination.
    pub fn from_picture_target(target: &PictureTarget, destination: RectangleF, sampling: ImageSampling) -> Result<ImageShape, String> {
        check_destination(destination)?;
        let source = ImageSource::from_rgba(&[0, 0, 0, 0], 1, 1)?;
        Ok(ImageShape { source, destination, source_rectangle: None, sampling, tint: None, picture: Some(target.clone()) })
    }

    /// The picture target shown, when the shape was made from one.
    pub fn picture_target(&self) -> Option<&PictureTarget> {
        self.picture.as_ref()
    }

    /// The picture.
    pub fn source(&self) -> &ImageSource {
        &self.source
    }

    /// Where the picture is drawn, calibrated units.
    pub fn destination(&self) -> RectangleF {
        self.destination
    }

    /// The part of the picture drawn, in its pixels; None for all of it.
    pub fn source_rectangle(&self) -> Option<Rectangle> {
        self.source_rectangle
    }

    /// How pixels are sampled.
    pub fn sampling(&self) -> ImageSampling {
        self.sampling
    }

    /// The colour every pixel is multiplied by (white = no change; a translucent white fades it).
    pub fn tint(&self) -> Option<Color> {
        self.tint
    }

    /// Sets the tint (C# `Tint = ...`; bindable as `AnimatedProperty::Tint`).
    pub fn set_tint(&mut self, tint: Option<Color>) {
        self.tint = tint;
    }

    /// The destination rectangle (C# `GetBounds`).
    pub fn get_bounds(&self) -> RectangleF {
        self.destination
    }

    pub(crate) fn encode(&self, scene: &mut VectorScene, transform: Affine, opacity: f32) {
        match &self.picture {
            Some(p) => encode_image_data(scene, transform, p.stand_in(), self.destination, None, self.sampling, self.tint, opacity),
            None => encode_picture(scene, transform, &self.source, self.destination, self.source_rectangle, self.sampling, self.tint, opacity),
        }
    }
}
