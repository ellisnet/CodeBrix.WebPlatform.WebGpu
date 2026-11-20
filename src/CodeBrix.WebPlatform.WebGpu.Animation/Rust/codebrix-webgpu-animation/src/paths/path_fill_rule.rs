//! `PathFillRule`: which areas of a self-overlapping path are inside (C# `PathFillRule`).

/// Which areas of a path are inside it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PathFillRule {
    /// Inside where the winding number is not zero (the default).
    #[default]
    NonZero = 0,
    /// Inside where the winding number is odd.
    EvenOdd = 1,
}

impl PathFillRule {
    pub(crate) fn to_peniko(self) -> peniko::Fill {
        match self {
            PathFillRule::NonZero => peniko::Fill::NonZero,
            PathFillRule::EvenOdd => peniko::Fill::EvenOdd,
        }
    }
}
