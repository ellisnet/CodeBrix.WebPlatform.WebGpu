//! `WaterSource`: the water a refraction or a reflection is seen through (no C# equivalent) - a
//! [`RippleField`] (a formula of the time) or a HEIGHT PICTURE (a decoded picture, an application's
//! GPU texture, or a picture target the crate draws this frame) - and its crate-internal form for
//! one frame.

use crate::images::ImageSource;
use crate::rendering::PictureTarget;
use crate::strokes::ExternalPicture;
use crate::water::ripple_math::PackedRipples;
use crate::water::RippleField;
use crate::RectangleF;

/// The water a [`WaterRefraction`](crate::water::WaterRefraction) or a
/// [`WaterReflection`](crate::water::WaterReflection) is seen through (no C# equivalent). Its
/// coordinates are the layer's DRAWING space: calibrated units under the camera at the layer's
/// parallax (the layer's own transform does not move the water), so one field can serve a
/// layer's refraction and another layer's reflection and both line up. Opt in with
/// `follows_layer` ([`WaterRefraction::follows_layer`](crate::water::WaterRefraction::follows_layer),
/// [`WaterReflection::follows_layer`](crate::water::WaterReflection::follows_layer)) to place it
/// in the layer's own space instead (its transform applied too): water pinned to a layer that
/// scrolls with it.
///
/// EXACT IN TIME with `Ripples`, `Picture` and a `Target` drawn from the session time: seeking
/// shows exactly the water playing shows. NOT with a `Texture` that the application changes over
/// time (a simulation, a video): such a scene CANNOT BE REWOUND - the crate draws whatever the
/// texture holds when the frame is drawn.
#[derive(Clone, Debug, PartialEq)]
pub enum WaterSource {
    /// Rings and swell from a formula (exact in time): evaluated per pixel on the GPU, the same
    /// arithmetic as [`RippleField::height_at`] / [`RippleField::slope_at`].
    Ripples(RippleField),
    /// Heights from a picture's RED channel as stored (byte 128 = still water; 0 and 255 =
    /// `-height` and `+height`), stretched over `place`. Uploaded once per picture, like any
    /// picture the session draws.
    Picture {
        /// The height picture (only its red channel is read).
        picture: ImageSource,
        /// Where it lies, in the water's coordinates (see the type).
        place: RectangleF,
        /// Water units at the channel's extremes.
        height: f32,
    },
    /// Heights from an application's GPU texture, its RED channel AS STORED (0 = still water),
    /// times `height`, stretched over `place` - for example the Graphics package's simulated wave
    /// field (`WaveField::surface()`: an `Rgba16Float` picture whose R is the height), handed over
    /// as a plain texture view on the canvas host's device (the two crates never depend on each
    /// other). The view must be 2D, filterable float (8-bit unorm or `Rgba16Float`) with
    /// TEXTURE_BINDING usage; a wrong view is a WebGPU validation error. Draw it in a canvas layer
    /// BELOW the layer that reads it (one encoder: the same frame's water; above it, the previous
    /// frame's). A SCENE FED BY A SIMULATION CANNOT BE REWOUND: pausing or seeking the clock does
    /// not undo what the application's texture went through.
    Texture {
        /// The application's texture (`size` must be its size; `encoding` is not used: heights are
        /// read as stored).
        picture: ExternalPicture,
        /// Where it lies, in the water's coordinates.
        place: RectangleF,
        /// Water units per unit of the red channel.
        height: f32,
    },
    /// Heights the crate draws THIS FRAME into a [`PictureTarget`] (its red channel as stored,
    /// 128 = still: draw the target opaque, for example noise shapes animated by the time - exact
    /// in time), stretched over `place`. Put the picture's
    /// [`AnimationLayer::to_picture`](crate::rendering::AnimationLayer::to_picture) layer BELOW
    /// the layer that reads it in the canvas's layers.
    Target {
        /// The picture target.
        picture: PictureTarget,
        /// Where it lies, in the water's coordinates.
        place: RectangleF,
        /// Water units at the channel's extremes.
        height: f32,
    },
}

impl WaterSource {
    /// Checks the source: a picture source's place must be a finite rectangle with an area, its
    /// height finite, an application texture's size not 0. The error says what to change.
    pub fn validate(&self) -> Result<(), String> {
        let (place, height) = match self {
            WaterSource::Ripples(_) => return Ok(()),
            WaterSource::Picture { place, height, .. } | WaterSource::Target { place, height, .. } => (place, height),
            WaterSource::Texture { picture, place, height } => {
                if picture.size.0 == 0 || picture.size.1 == 0 {
                    return Err("A water texture's size must be its view's size in pixels (not 0).".to_string());
                }
                (place, height)
            }
        };
        if place.is_empty() || !(place.x.is_finite() && place.y.is_finite() && place.width.is_finite() && place.height.is_finite()) {
            return Err("A water picture's place must be a finite rectangle with an area (where the picture lies in the water's coordinates).".to_string());
        }
        if !height.is_finite() {
            return Err(format!("A water picture's height must be a finite number (it is {height})."));
        }
        Ok(())
    }

    /// True when the water is a function of the session time alone (ripples, a picture, a picture
    /// target drawn from the time); false for an application's texture, which the crate cannot
    /// rewind.
    pub fn is_exact_in_time(&self) -> bool {
        !matches!(self, WaterSource::Texture { .. })
    }
}

/// The texture a picture source is read from (crate-internal: the GPU half binds it).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum WaterTexture {
    /// A picture of the frame's list (uploaded once per picture).
    Picture(ImageSource),
    /// A picture target's texture.
    Target(PictureTarget),
    /// The application's texture view.
    Texture(codebrix_webgpu::wgpu::TextureView),
}

/// One frame's form of a water source (crate-internal): ripples packed at the frame's time, or a
/// height picture with how its red channel becomes height (`(red - bias) x scale`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum WaterTap {
    /// The packed field (header offsets counted from 0; the builder moves them into the frame's
    /// ripple buffer).
    Ripples(PackedRipples),
    /// A height picture over `place` (left, top, right, bottom), `size` its pixels.
    Picture { texture: WaterTexture, place: [f32; 4], size: (u32, u32), scale: f32, bias: f32 },
}

impl WaterTap {
    /// The frame's form of `source` at `seconds`.
    pub(crate) fn new(source: &WaterSource, seconds: f64) -> WaterTap {
        let rect = |r: &RectangleF| [r.x, r.y, r.right(), r.bottom()];
        match source {
            WaterSource::Ripples(field) => WaterTap::Ripples(field.pack(seconds, 0)),
            WaterSource::Picture { picture, place, height } => WaterTap::Picture { texture: WaterTexture::Picture(picture.clone()), place: rect(place), size: (picture.width(), picture.height()), scale: 2.0 * height, bias: 0.5 },
            WaterSource::Target { picture, place, height } => WaterTap::Picture { texture: WaterTexture::Target(picture.clone()), place: rect(place), size: picture.size(), scale: 2.0 * height, bias: 0.5 },
            WaterSource::Texture { picture, place, height } => WaterTap::Picture { texture: WaterTexture::Texture(picture.view.clone()), place: rect(place), size: picture.size, scale: *height, bias: 0.0 },
        }
    }

    /// Live drops the cap left out (ripples only).
    pub(crate) fn skipped(&self) -> usize {
        match self {
            WaterTap::Ripples(p) => p.skipped,
            WaterTap::Picture { .. } => 0,
        }
    }

    /// An upper bound of the water's slope (height units per water unit) anywhere: what the
    /// displacement's region growth is sized by. Ripples: twice the steepest single ring (two
    /// rings meeting) plus every swell wave's steepest slope; a picture: the full range of its
    /// heights over one texel.
    pub(crate) fn slope_bound(&self) -> f32 {
        match self {
            WaterTap::Ripples(p) => {
                let drops = p.header.words[1] as usize;
                let waves = p.header.words[2] as usize;
                let mut ring = 0.0f32;
                for i in 0..drops {
                    let a = p.records[i * 2];
                    let b = p.records[i * 2 + 1];
                    let (amplitude, lambda, width) = (a[3].abs(), b[0], b[1]);
                    let k = std::f32::consts::TAU / lambda;
                    // |dS/dr| <= 0.5 / lambda, |dE/du| <= sqrt(2/e) / c, |d sin/du| <= k.
                    ring = ring.max(amplitude * (0.5 / lambda + 0.86 / width.min(0.25 * lambda) + k));
                }
                let mut swell = 0.0f32;
                for j in 0..waves {
                    let w = p.records[drops * 2 + j];
                    swell += w[2].abs() * (w[0] * w[0] + w[1] * w[1]).sqrt();
                }
                2.0 * ring + swell
            }
            WaterTap::Picture { place, size, scale, .. } => {
                let texel = ((place[2] - place[0]) / size.0.max(1) as f32).abs().min(((place[3] - place[1]) / size.1.max(1) as f32).abs());
                if texel > 0.0 {
                    scale.abs() / texel
                } else {
                    0.0
                }
            }
        }
    }
}
