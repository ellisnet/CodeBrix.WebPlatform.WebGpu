//! `PathStrokeJoin`: the corners of a stroke (C# `PathStrokeJoin`).

/// The corners of a stroke.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PathStrokeJoin {
    /// Rounded (the default).
    #[default]
    Round = 0,
    /// Pointed, cut to a bevel beyond the miter limit.
    Miter = 1,
    /// Cut off.
    Bevel = 2,
}

impl PathStrokeJoin {
    pub(crate) fn to_kurbo(self) -> kurbo::Join {
        match self {
            PathStrokeJoin::Round => kurbo::Join::Round,
            PathStrokeJoin::Miter => kurbo::Join::Miter,
            PathStrokeJoin::Bevel => kurbo::Join::Bevel,
        }
    }
}
