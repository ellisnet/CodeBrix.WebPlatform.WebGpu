//! `CaptionFormat`: the caption file formats read (C# `CaptionFormat`).

/// A caption text format (C# `CaptionFormat`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CaptionFormat {
    /// SubRip (.srt) (the default).
    #[default]
    Srt = 0,
    /// WebVTT (.vtt), basic syntax.
    WebVtt = 1,
}
