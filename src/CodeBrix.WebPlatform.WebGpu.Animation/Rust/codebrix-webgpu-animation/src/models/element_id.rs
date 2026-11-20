//! `ElementId`: how a binding names the element it drives (C# holds a reference to the element).

use std::sync::atomic::{AtomicU64, Ordering};

/// The identity of a [`DrawingShape`](crate::shapes::DrawingShape) - what C# code holds as a
/// reference to the element: give it to a binding, find the element again on its layer
/// ([`DrawingLayer::find_element`](crate::models::DrawingLayer::find_element)), or read its
/// transformed bounds from the session. Every new shape gets a new one; a CLONE of a shape keeps
/// its id (it is the same element, copied), so add each element to a layer once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ElementId(u64);

static NEXT: AtomicU64 = AtomicU64::new(1);

impl ElementId {
    /// A new, never used id.
    pub(crate) fn next() -> ElementId {
        ElementId(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    /// The number (for logs and read-outs).
    pub fn value(&self) -> u64 {
        self.0
    }
}
