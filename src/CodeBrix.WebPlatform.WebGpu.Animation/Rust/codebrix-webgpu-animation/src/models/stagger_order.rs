//! `StaggerOrder`: the order in which a stagger or a train ranks its elements (C# `StaggerOrder`).

/// The order in which elements are ranked (C# `StaggerOrder`, the same numbers): rank 0 goes
/// first (it starts first in a stagger, it leads a train).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum StaggerOrder {
    /// In the given order, first to last (the default).
    #[default]
    Forward = 0,
    /// Last first.
    Reverse = 1,
    /// From the middle outwards, both sides together (rank = floor(|i - (count - 1) / 2|)).
    CenterOut = 2,
}

impl StaggerOrder {
    /// The element's place in the order, from 0 (C#'s rank in BindStagger and BindTrain:
    /// Reverse = count - 1 - i, CenterOut = floor(|i - (count - 1) / 2|)).
    pub fn rank_of(self, index: usize, count: usize) -> usize {
        match self {
            StaggerOrder::Forward => index,
            StaggerOrder::Reverse => count.saturating_sub(1).saturating_sub(index),
            StaggerOrder::CenterOut => (index as f64 - ((count as f64 - 1.0) / 2.0)).abs().floor() as usize,
        }
    }

    /// The rank of element `index` of `count` (the trains' name for [`StaggerOrder::rank_of`]).
    pub(crate) fn rank(self, index: usize, count: usize) -> usize {
        self.rank_of(index, count)
    }
}
