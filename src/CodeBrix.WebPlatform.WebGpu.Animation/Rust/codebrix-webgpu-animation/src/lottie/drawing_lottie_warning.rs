//! `DrawingLottieWarning` and `DrawingLottieWarningKind`: what a loaded document uses that is not
//! drawn, or not drawn as written (the same shape as the `svg` module's warnings).

/// Why something in a document is not drawn, or not drawn as written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DrawingLottieWarningKind {
    /// A text layer (`ty` 5): Lottie text is not drawn. The layer is kept as a null layer, so
    /// layers parented to it still move with it.
    TextLayerNotDrawn,
    /// A layer type that is not drawn (audio, video, image sequence, placeholders, guide,
    /// adjustment, camera, light, data): kept as a null layer when it has a transform, otherwise
    /// removed.
    LayerTypeNotDrawn,
    /// An image layer: the crate decodes and fetches nothing; the draw list carries an
    /// [`Image`](crate::lottie::LottieDrawCommand::Image) record and the application supplies the
    /// pixels, or nothing is drawn.
    ImageNeedsPixels,
    /// A layer blend mode inside a precomposition (only top-level layers' blend modes are
    /// applied), or the "hard mix" mode (17), which has no equivalent: drawn as normal.
    BlendModeIgnored,
    /// A luma (or inverted luma) track matte: applied by the matte's ALPHA.
    LumaMatteApproximated,
    /// A mask whose mode, inversion, opacity or expansion is not applied: every mask is drawn as
    /// a plain clip to the inside of its path, and several masks clip to their intersection.
    MaskApproximated,
    /// Stroke dashes: the stroke is drawn solid.
    StrokeDashesIgnored,
    /// An even-odd fill rule: filled non-zero.
    EvenOddFillApproximated,
    /// A gradient fill's or gradient stroke's own opacity (`o`): drawn at full opacity.
    GradientOpacityIgnored,
    /// A radial gradient's highlight (focal point, `h` / `a`): drawn centred.
    GradientHighlightIgnored,
    /// A shape modifier that is not applied (pucker/bloat, merge paths, offset path, twist,
    /// rounded corners, zig zag): the shapes are drawn unmodified.
    ShapeModifierIgnored,
    /// A shape item of an unknown type: removed (it would otherwise stop the file loading).
    UnknownShapeRemoved,
    /// Trim paths "individually" (`m` 2): each path is trimmed on its own ("simultaneously").
    TrimIndividuallyApproximated,
    /// An expression (`x`): the property's keyframes or value are used instead.
    ExpressionIgnored,
    /// A layer effect (`ef`): not applied.
    EffectIgnored,
    /// A layer style (`sy`: drop shadow, stroke, glow ...): not applied.
    LayerStyleIgnored,
    /// Time remapping (`tm`) of a precomposition layer: the precomposition plays at its own
    /// time.
    TimeRemapIgnored,
    /// Motion-path tangents of a position keyframe (`ti` / `to`): the position moves in a
    /// straight line between keyframes (the easing is kept).
    MotionPathApproximated,
    /// Auto-orient (`ao`): the layer is not turned along its motion path.
    AutoOrientIgnored,
    /// A 3D layer (`ddd`) or 3D rotation: drawn flat, rotated about Z only.
    ThreeDimensionalIgnored,
    /// Motion blur (`mb`): not applied.
    MotionBlurIgnored,
    /// A split position on a shape group's transform that is animated: the first keyframe's
    /// position is used.
    SplitPositionApproximated,
    /// Per-dimension easing of a property velato eases with one curve: the first dimension's
    /// curve is used for all.
    EasingApproximated,
    /// A repeater with more copies than the load options allow: limited to that number.
    RepeaterCopiesLimited,
}

/// One warning: its kind and a sentence that says what was found and what happens to it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DrawingLottieWarning {
    /// The kind.
    pub kind: DrawingLottieWarningKind,
    /// What was found (naming the layer) and what happens to it.
    pub message: String,
}

impl DrawingLottieWarning {
    /// A warning.
    pub fn new(kind: DrawingLottieWarningKind, message: impl Into<String>) -> DrawingLottieWarning {
        DrawingLottieWarning { kind, message: message.into() }
    }
}

/// A deduplicating list of warnings (crate-internal).
#[derive(Clone, Debug, Default)]
pub(crate) struct WarningList {
    pub(crate) items: Vec<DrawingLottieWarning>,
}

impl WarningList {
    pub(crate) fn add(&mut self, kind: DrawingLottieWarningKind, message: impl Into<String>) {
        let w = DrawingLottieWarning::new(kind, message);
        if !self.items.contains(&w) {
            self.items.push(w);
        }
    }
}
