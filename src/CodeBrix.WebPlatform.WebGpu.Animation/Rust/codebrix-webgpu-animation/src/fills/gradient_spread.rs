//! `GradientSpread`: how a gradient continues outside its 0..1 range (C# `GradientSpread`).

/// How a gradient continues outside its 0..1 range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GradientSpread {
    /// The end colours continue (the default).
    #[default]
    Pad = 0,
    /// The gradient repeats.
    Repeat = 1,
    /// The gradient repeats, every other copy mirrored.
    Reflect = 2,
}

impl GradientSpread {
    /// peniko's name for the same thing.
    pub(crate) fn to_extend(self) -> peniko::Extend {
        match self {
            GradientSpread::Pad => peniko::Extend::Pad,
            GradientSpread::Repeat => peniko::Extend::Repeat,
            GradientSpread::Reflect => peniko::Extend::Reflect,
        }
    }
}
