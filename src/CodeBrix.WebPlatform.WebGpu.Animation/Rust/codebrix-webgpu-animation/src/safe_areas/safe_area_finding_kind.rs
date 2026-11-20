//! `SafeAreaFindingKind`: what a safe-area finding says (C# `SafeAreaFindingKind`).

/// What a [`SafeAreaFinding`](crate::safe_areas::SafeAreaFinding) says (C#
/// `SafeAreaFindingKind`, the same numbers).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SafeAreaFindingKind {
    /// The element reaches into an unsafe zone.
    Overlap = 0,
    /// The element's bounds are not known, so it could not be checked.
    CouldNotCheck = 1,
}
