//! Pictures for the drawing (C# `CodeBrix.Imaging.Drawing.Images`): [`ImageSource`] (a copy of
//! straight-alpha RGBA pixels: decoded ones, a PNG FILE it reads itself - PNG is the only file
//! format - or the picture-pixels form a backend writes), [`SpriteSheet`] (the
//! frames of a sprite), [`ImageSampling`] and [`SpritePlayback`]. They are drawn by
//! [`ImageShape`](crate::shapes::ImageShape) and [`SpriteShape`](crate::shapes::SpriteShape).

mod image_sampling;
mod image_source;
mod image_source_png;
mod sprite_playback;
mod sprite_sheet;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_png;

pub use image_sampling::ImageSampling;
pub use image_source::ImageSource;
pub use sprite_playback::SpritePlayback;
pub use sprite_sheet::SpriteSheet;
