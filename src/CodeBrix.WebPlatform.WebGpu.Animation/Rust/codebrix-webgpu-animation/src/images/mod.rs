//! Pictures for the drawing (C# `CodeBrix.Imaging.Drawing.Images`): [`ImageSource`] (a copy of
//! straight-alpha RGBA pixels: decoded ones, a PNG or JPEG FILE it reads itself - PNG and JPEG are
//! the file formats it reads, through codebrix-webgpu's decoders - or the picture-pixels form a
//! backend writes), [`SpriteSheet`] (the
//! frames of a sprite), [`ImageSampling`], [`SpritePlayback`] and [`LivePicture`] (a picture whose
//! pixels change in place; Not in C#). They are drawn by
//! [`ImageShape`](crate::shapes::ImageShape) and [`SpriteShape`](crate::shapes::SpriteShape).

mod image_sampling;
mod image_source;
mod image_source_jpeg;
mod image_source_png;
mod live_picture;
mod sprite_playback;
mod sprite_sheet;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_jpeg;
#[cfg(test)]
mod tests_live_picture;
#[cfg(test)]
mod tests_png;

pub use image_sampling::ImageSampling;
pub use image_source::ImageSource;
pub use live_picture::LivePicture;
pub(crate) use live_picture::PixelRegion;
pub use sprite_playback::SpritePlayback;
pub use sprite_sheet::SpriteSheet;
