//! `LottieMarker`: a named point or span of a document's timeline (Lottie `markers`).

/// A marker: a name, the frame it starts at and its length in frames (0 for a point).
#[derive(Clone, Debug, PartialEq)]
pub struct LottieMarker {
    /// The marker's name (Lottie `cm`, as written).
    pub name: String,
    /// The frame it starts at (Lottie `tm`).
    pub frame: f64,
    /// Its length in frames (Lottie `dr`; 0 for a point in time).
    pub duration: f64,
}

impl LottieMarker {
    /// The frame it ends at (`frame + duration`).
    pub fn end_frame(&self) -> f64 {
        self.frame + self.duration
    }
}
