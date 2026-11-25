//! `SessionBlendMode`: how a whole session's picture combines with what was drawn before it in
//! the same scene (Not in C#: NoSkia draws a session onto its canvas with normal blending only).

use peniko::{BlendMode, Compose, Mix};

/// How a session's whole picture (its background when drawn, its layers) combines with what
/// was drawn into the same [`VectorScene`](crate::rendering::VectorScene) before it - an earlier
/// session, a picture, a background drawn first. Set on
/// [`SessionRenderOptions::blend`](crate::SessionRenderOptions::blend), beside its `opacity`.
///
/// The mix modes (multiply, screen, overlay, darken, lighten) blend sRGB-encoded values with
/// the W3C formulas, as the crate's groups always do; `Add` adds the colours (premultiplied,
/// linear, clamped by the target). The blend applies inside the animation layer: what OTHER
/// canvas layers drew (a 3D scene below) is not the backdrop - the animation layer as a whole is
/// composited normally over them.
///
/// Any mode other than `Normal` (or an opacity below 1) draws the session through one offscreen
/// group: ONE level of the renderer's nesting allowance (two inside a
/// [`SessionWipe`](crate::SessionWipe)); a scene that is already as deep as the renderer goes
/// draws the deepest groups as plain clips and counts them in
/// [`RenderStats::unsupported`](crate::rendering::RenderStats::unsupported).
///
/// Not in C#.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SessionBlendMode {
    /// Drawn over what is below (the default).
    #[default]
    Normal,
    /// The colours multiplied: never lighter than either (ink on paper).
    Multiply,
    /// The inverse of multiplying the inverses: never darker than either (light on a screen).
    Screen,
    /// Multiply where the backdrop is dark, screen where it is light.
    Overlay,
    /// The darker of the two, per channel.
    Darken,
    /// The lighter of the two, per channel.
    Lighten,
    /// The colours added (light added to light).
    Add,
}

impl SessionBlendMode {
    /// The renderer's blend mode for it.
    pub(crate) fn to_peniko(self) -> BlendMode {
        match self {
            SessionBlendMode::Normal => BlendMode::new(Mix::Normal, Compose::SrcOver),
            SessionBlendMode::Multiply => BlendMode::new(Mix::Multiply, Compose::SrcOver),
            SessionBlendMode::Screen => BlendMode::new(Mix::Screen, Compose::SrcOver),
            SessionBlendMode::Overlay => BlendMode::new(Mix::Overlay, Compose::SrcOver),
            SessionBlendMode::Darken => BlendMode::new(Mix::Darken, Compose::SrcOver),
            SessionBlendMode::Lighten => BlendMode::new(Mix::Lighten, Compose::SrcOver),
            SessionBlendMode::Add => BlendMode::new(Mix::Normal, Compose::Plus),
        }
    }
}
