//! `PathBooleanOperation`: how two filled areas combine (C# `PathBooleanOperation`).

/// How two filled areas combine (C# `PathBooleanOperation`, the same numbers).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PathBooleanOperation {
    /// Everything either covers (the default).
    #[default]
    Union = 0,
    /// Only what both cover.
    Intersection = 1,
    /// The first, less what the second covers.
    Difference = 2,
    /// What exactly one of them covers.
    ExclusiveOr = 3,
}
