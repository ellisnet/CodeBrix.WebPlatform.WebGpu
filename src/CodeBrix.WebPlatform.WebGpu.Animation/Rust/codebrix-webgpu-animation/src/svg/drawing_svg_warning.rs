//! `DrawingSvgWarning` and `DrawingSvgWarningKind`: what a loaded document asked for that is not
//! drawn or not played as written (C# `DrawingSvgWarning(Kind, Message)`; the kinds C# has are
//! kept, the kinds this crate needs are added).

/// Why something in a document is not drawn or played as written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DrawingSvgWarningKind {
    /// C#: a filter primitive is not evaluated (here: every filter except one plain
    /// `feGaussianBlur`); the element is drawn without the filter.
    UnsupportedFilterPrimitive,
    /// C#: `feTurbulence` dropped (here: reported as `UnsupportedFilterPrimitive`; kept for the
    /// C# vocabulary).
    TurbulenceDropped,
    /// C#: glyph-id text runs (not reached here: usvg lays out every run from its characters).
    GlyphIdTextRunUnsupported,
    /// C#: text on a path (not reached here: usvg lays out `<textPath>` itself).
    TextOnPathUnsupported,
    /// C#: text with no registered font. Here: the document has `<text>` and the load was given
    /// no font ([`DrawingSvgOptions::fonts`](crate::svg::DrawingSvgOptions::fonts)), so the text
    /// is not drawn.
    NoFontsRegistered,
    /// The document names a font family the supplied fonts do not have: that text is drawn in
    /// the application's family ([`DrawingSvgOptions::font_family`](crate::svg::DrawingSvgOptions::font_family),
    /// or the first supplied font's); there is never a system-font fallback.
    FontSubstituted,
    /// An animation targets an id that usvg cleared because the element sits inside content a
    /// `<use>` instances (or is the `<use>`d element itself): it cannot be animated. Give the
    /// `<use>` element an id of its own and animate that.
    AnimationTargetInsideUse,
    /// A `<pattern>` fill or stroke: the renderer has no pattern paint yet; that paint is drawn as
    /// nothing.
    PatternNotDrawn,
    /// A luminance mask: the drawing half applies the mask content's ALPHA (the renderer has no
    /// luminance pass yet).
    LuminanceMaskApproximated,
    /// An image (encoded bytes or an external href): the crate decodes and fetches nothing; the
    /// application supplies the pixels, or the image is not drawn.
    ImageNeedsPixels,
    /// An image that is itself an SVG document: not drawn in this cut.
    NestedSvgImageNotDrawn,
    /// An animation element or CSS animation is ignored (an event or other non-time begin, an
    /// attribute that cannot be overridden, values that do not parse, a script ...).
    AnimationIgnored,
    /// An animation is played in an approximated way (stated in the message).
    AnimationApproximated,
    /// CSS the animation reader does not use (other at-rules, selectors it cannot match).
    CssIgnored,
    /// `<script>` or an event attribute (`onclick` ...): scripts never run.
    ScriptIgnored,
}

/// One warning: its kind and a sentence that says what was found and what happens to it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DrawingSvgWarning {
    /// The kind.
    pub kind: DrawingSvgWarningKind,
    /// What was found and what happens to it.
    pub message: String,
}

impl DrawingSvgWarning {
    /// A warning.
    pub fn new(kind: DrawingSvgWarningKind, message: impl Into<String>) -> DrawingSvgWarning {
        DrawingSvgWarning { kind, message: message.into() }
    }
}

/// A deduplicating list of warnings (crate-internal).
#[derive(Clone, Debug, Default)]
pub(crate) struct WarningList {
    pub(crate) items: Vec<DrawingSvgWarning>,
}

impl WarningList {
    pub(crate) fn add(&mut self, kind: DrawingSvgWarningKind, message: impl Into<String>) {
        let w = DrawingSvgWarning::new(kind, message);
        if !self.items.contains(&w) {
            self.items.push(w);
        }
    }
}
