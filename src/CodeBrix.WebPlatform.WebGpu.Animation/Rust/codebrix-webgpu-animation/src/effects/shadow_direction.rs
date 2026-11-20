//! `ShadowDirection` (C# `ShadowDirection`, the same members and numbers).

/// Which way a [`DrawingEffects`](crate::effects::DrawingEffects) drop shadow falls when the
/// element turns (C# `ShadowDirection`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShadowDirection {
    /// The light is fixed in the SCENE: the shadow offset keeps its direction however the
    /// element, its groups or its layer turn (their scale still stretches it), and turns only when
    /// the whole scene turns - with the camera's roll (the default).
    #[default]
    FixedInScene = 0,
    /// The shadow turns (and is mirrored and stretched) with the element: the offset is in the
    /// element's own coordinates, through its transform, its groups', its layer's and the camera.
    RotatesWithElement = 1,
}
