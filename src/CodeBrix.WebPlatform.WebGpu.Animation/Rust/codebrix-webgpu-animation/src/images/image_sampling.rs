//! `ImageSampling` (C# `ImageSampling`, the same members and numbers).

/// How a picture's pixels are sampled when it is drawn at another size or angle (C#
/// `ImageSampling`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageSampling {
    /// Bilinear interpolation between the four nearest source pixels - smooth (the default).
    #[default]
    Linear = 0,
    /// The nearest source pixel - hard, blocky pixels (pixel art, sprite sheets drawn at whole
    /// multiples).
    Nearest = 1,
}
