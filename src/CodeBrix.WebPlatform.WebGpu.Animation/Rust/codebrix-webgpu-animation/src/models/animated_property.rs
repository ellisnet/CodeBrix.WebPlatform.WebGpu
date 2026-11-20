//! `AnimatedProperty`: what a binding drives (C# `AnimatedProperty`, the same members and numbers).

/// What an [`AnimationBinding`](crate::models::AnimationBinding) drives (C# `AnimatedProperty`).
/// Which properties a target takes, and with which kind of track, is C#'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnimatedProperty {
    /// The transform's translation (point track; element, layer) or a camera's pan.
    Position = 0,
    /// The transform's anchor (point track; element, layer) or a camera's pivot.
    Anchor = 1,
    /// The transform's rotation, degrees (float track; element, layer) or a camera's roll.
    Rotation = 2,
    /// The transform's uniform scale (float track) or both scales (point track); a camera's zoom.
    Scale = 3,
    /// The horizontal scale (float track).
    ScaleX = 4,
    /// The vertical scale (float track).
    ScaleY = 5,
    /// The element's or layer's opacity (float track, clamped to 0..1).
    Opacity = 6,
    /// A group's content colour or a layer's colour (colour track).
    Color = 7,
    /// A path shape's trim start, 0..1 (float track).
    TrimStart = 8,
    /// A path shape's trim end, 0..1 (float track).
    TrimEnd = 9,
    /// A path shape's trim offset, fractions of the length (float track).
    TrimOffset = 10,
    /// A morphing path shape's fraction, 0..1 (float track).
    MorphFraction = 11,
    /// A sprite's frame index (float track, rounded down; sprites).
    FrameIndex = 12,
    /// A picture's or sprite's tint (colour track).
    Tint = 13,
    /// The effects' content blur radius (float track; element, layer).
    BlurRadius = 14,
    /// The drop shadow's offset (point track; element, layer).
    ShadowOffset = 15,
    /// The drop shadow's softness (float track; element, layer).
    ShadowBlurRadius = 16,
    /// The drop shadow's colour (colour track; element, layer).
    ShadowColor = 17,
    /// The glow's radius (float track; element, layer).
    GlowRadius = 18,
    /// The glow's colour (colour track; element, layer).
    GlowColor = 19,
    /// A text block's reveal fraction, 0..1 (float track; text blocks).
    RevealFraction = 20,
    /// A text block's revealed unit count (float track; text blocks).
    RevealCount = 21,
    /// A layer's visibility (bool track).
    IsVisible = 22,
}
