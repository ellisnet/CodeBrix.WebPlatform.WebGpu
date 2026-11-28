//! `StrokeColor`: where the strokes of a [`BrushStrokesShape`](crate::shapes::BrushStrokesShape)
//! take their colours from - each record's own colour, or PICKED on the GPU, every frame, from a
//! live picture at each stroke's position (no C# equivalent).
//!
//! THE PICKUP: in the stroke shader, K points along each stroke's centre line (as the stroke is
//! drawn this frame: after its boil, flow and anchors) are mapped through `place` onto the source
//! and sampled (bilinear, clamped to the source's edge), averaged in LINEAR light weighted by
//! their alpha, then passed through the colour rules (palette, broken colour) and mixed with the
//! record's own colour by `own_weight` (times the stroke's own weight, when its set has them) in
//! Oklab. The stroke's alpha is the record's times the source's mean alpha (mixed towards 1 by
//! that same weight): strokes over an empty part of the source paint nothing. Every value is a
//! function of the source as it is this frame and of the records: nothing reads a clock.
//!
//! LIMITS: the mean of K point samples, NOT the exact area mean of
//! [`DrawingOwnershipMap::get_mean_colors`](crate::painting::DrawingOwnershipMap::get_mean_colors)
//! (the exact CPU route stays); colours are picked every frame (no "pick once and keep"); an
//! HDR source must be TONE-MAPPED by the application first (into an 8-bit or a 0..1 half-float
//! texture: values are clamped to 0..1); the source must be on the canvas host's device.

use codebrix_webgpu::wgpu;

use crate::geometry::RectangleF;
use crate::images::ImageSource;
use crate::painting::ColorRules;
use crate::rendering::PictureTarget;

/// Where a stroke set's colours come from (no C# equivalent). The default is each record's own
/// colour.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum StrokeColor {
    /// Each record's own colour (the default).
    #[default]
    Own,
    /// Picked every frame from a picture at each stroke's position.
    Picked(StrokePickup),
}

/// How strokes pick their colours from a picture (no C# equivalent).
#[derive(Clone, Debug, PartialEq)]
pub struct StrokePickup {
    /// The picture.
    pub source: StrokeColorSource,
    /// Where the picture lies in the set's coordinates (calibrated units of the shape): the
    /// picture's top-left corner at the rectangle's, stretched over it. A stroke outside it picks
    /// the colour at the picture's nearest edge.
    pub place: RectangleF,
    /// Samples along each stroke's centre line, averaged: 1 (the middle), 3 or 5 (from 40 % of
    /// the length before the middle to 40 % after it). 5.
    pub taps: u32,
    /// 0 = the picked colour .. 1 = the record's own colour, mixed in Oklab. 0. A set with own
    /// weights per stroke ([`BrushStrokeSet::with_own_weights`](crate::strokes::BrushStrokeSet::with_own_weights))
    /// scales it stroke by stroke: each stroke mixes by this times its own weight.
    pub own_weight: f32,
    /// A palette and broken colour applied to the picked colour (before the mix with the own
    /// colour) - the same rules as [`ColorRules::apply_float`] on the CPU. None.
    pub rules: Option<ColorRules>,
}

impl StrokePickup {
    /// Picks from `source` laid over `place`: 5 taps, no own colour, no rules.
    pub fn new(source: StrokeColorSource, place: RectangleF) -> StrokePickup {
        StrokePickup { source, place, taps: 5, own_weight: 0.0, rules: None }
    }

    /// Checks the pickup; the error says what to change.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !matches!(self.taps, 1 | 3 | 5) {
            return Err(format!("A stroke pickup takes 1, 3 or 5 taps along each stroke ({} were asked for).", self.taps));
        }
        if !(self.own_weight >= 0.0 && self.own_weight <= 1.0) {
            return Err("A stroke pickup's own weight must be a fraction, 0..1 (0 = the picked colour, 1 = the record's own).".to_string());
        }
        let p = self.place;
        if ![p.x, p.y, p.width, p.height].iter().all(|v| v.is_finite()) || !(p.width > 0.0 && p.height > 0.0) {
            return Err("A stroke pickup's place must be a finite rectangle with a width and a height above 0 (where the picture lies in the set's coordinates).".to_string());
        }
        if let Some(r) = &self.rules {
            r.validate()?;
        }
        if let StrokeColorSource::Texture(t) = &self.source {
            if t.size.0 < 1 || t.size.1 < 1 {
                return Err("An external picture needs its size in pixels (width and height of at least 1).".to_string());
            }
        }
        Ok(())
    }
}

/// The picture strokes pick their colours from (no C# equivalent).
#[derive(Clone, Debug, PartialEq)]
pub enum StrokeColorSource {
    /// Decoded pixels (uploaded once per picture, as any picture; a NEW picture each frame is an
    /// upload each frame).
    Picture(ImageSource),
    /// A picture the crate draws this frame from another session (see [`PictureTarget`]).
    Target(PictureTarget),
    /// Any texture on the canvas host's device: a 3D render (the colour of a render target the
    /// application drew this frame), a video frame the APPLICATION copied into a texture.
    Texture(ExternalPicture),
}

/// A texture the application hands over (no C# equivalent): a view of a 2D texture on the
/// canvas host's device, with `TEXTURE_BINDING` usage and a FILTERABLE float format (8-bit
/// unorm, sRGB or not, or `Rgba16Float` holding 0..1). Drawn this frame BEFORE the stroke layer
/// (an earlier layer of the canvas, or work the application submitted or encoded first), its
/// texels are read as they are this frame. An HDR render must be tone-mapped first (values are
/// clamped to 0..1). The crate never touches video elements: copy a frame with wgpu's
/// `Queue::copy_external_image_to_texture` into a texture the application owns.
#[derive(Clone, Debug, PartialEq)]
pub struct ExternalPicture {
    /// The texture view (compared by identity).
    pub view: wgpu::TextureView,
    /// Its size in pixels.
    pub size: (u32, u32),
    /// How its texels hold colour.
    pub encoding: PictureEncoding,
}

/// How an [`ExternalPicture`]'s texels hold colour (no C# equivalent).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PictureEncoding {
    /// sRGB-encoded values, STRAIGHT alpha: the crate's picture convention (a `*Unorm` view of
    /// 8-bit sRGB bytes, as an `ImageSource` holds them).
    SrgbEncoded,
    /// Linear 0..1 values, PREMULTIPLIED by alpha: what every renderer of the canvas host writes
    /// (an `*UnormSrgb` view of a render target, or a linear target). For an opaque picture
    /// premultiplied and straight are the same.
    Linear,
}

/// How the stroke shader reads the source's texels (crate-internal; the draw block's word).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceTexels {
    /// sRGB-encoded, straight alpha (an `ImageSource`, an `SrgbEncoded` texture).
    SrgbStraight = 0,
    /// sRGB-encoded, premultiplied (a premultiplied picture, a `PictureTarget`).
    SrgbPremultiplied = 1,
    /// Linear, premultiplied (a `Linear` texture).
    LinearPremultiplied = 2,
}
