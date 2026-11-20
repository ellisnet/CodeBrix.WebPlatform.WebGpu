//! `PathStrokeCap`: the ends of an open stroke (C# `PathStrokeCap`).

/// The ends of an open stroke and of every dash.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PathStrokeCap {
    /// A half circle (the default).
    #[default]
    Round = 0,
    /// Flat, at the end point.
    Butt = 1,
    /// A half square past the end point.
    Square = 2,
}

impl PathStrokeCap {
    pub(crate) fn to_kurbo(self) -> kurbo::Cap {
        match self {
            PathStrokeCap::Round => kurbo::Cap::Round,
            PathStrokeCap::Butt => kurbo::Cap::Butt,
            PathStrokeCap::Square => kurbo::Cap::Square,
        }
    }
}
