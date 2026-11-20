//! `DrawingTransform`: an element's or layer's translation, rotation, scale and anchor (C#
//! `DrawingTransform`).

use kurbo::Affine;

use crate::geometry::{PointF, RectangleF};

/// The parts of a [`DrawingTransform`] other than its translation, with C#'s defaults (no
/// rotation, scale 1, anchor at the origin) - the optional arguments of C#'s constructor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawingTransformOptions {
    /// Degrees, clockwise on screen (y points down).
    pub rotation: f32,
    /// The horizontal scale.
    pub scale_x: f32,
    /// The vertical scale.
    pub scale_y: f32,
    /// The point (in the element's own coordinates) that rotation and scale turn about, and that
    /// the translation moves.
    pub anchor: PointF,
}

impl Default for DrawingTransformOptions {
    fn default() -> DrawingTransformOptions {
        DrawingTransformOptions { rotation: 0.0, scale_x: 1.0, scale_y: 1.0, anchor: PointF::new(0.0, 0.0) }
    }
}

/// How an element (or a whole layer) is placed: scaled and rotated about its ANCHOR, then moved by
/// its TRANSLATION (C# `DrawingTransform`). Immutable: the `with_*` methods give changed copies.
/// The matrix is translate(-anchor), scale, rotate, translate(anchor + translation), as C#'s
/// `ToMatrix`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawingTransform {
    translation: PointF,
    rotation: f32,
    scale_x: f32,
    scale_y: f32,
    anchor: PointF,
}

impl Default for DrawingTransform {
    fn default() -> DrawingTransform {
        DrawingTransform::IDENTITY
    }
}

fn finite(p: PointF) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

impl DrawingTransform {
    /// No change (C# `DrawingTransform.Identity`).
    pub const IDENTITY: DrawingTransform = DrawingTransform { translation: PointF::new(0.0, 0.0), rotation: 0.0, scale_x: 1.0, scale_y: 1.0, anchor: PointF::new(0.0, 0.0) };

    /// A transform (C# `DrawingTransform(translation, rotation = 0, scaleX = 1, scaleY = 1,
    /// anchor = default)`). The error says which part is not a finite number.
    pub fn new(translation: PointF, options: DrawingTransformOptions) -> Result<DrawingTransform, String> {
        if !finite(translation) {
            return Err("A transform's translation must be finite numbers.".to_string());
        }
        if !options.rotation.is_finite() || !options.scale_x.is_finite() || !options.scale_y.is_finite() {
            return Err("A transform's rotation and scale must be finite numbers.".to_string());
        }
        if !finite(options.anchor) {
            return Err("A transform's anchor must be finite numbers.".to_string());
        }
        Ok(DrawingTransform { translation, rotation: options.rotation, scale_x: options.scale_x, scale_y: options.scale_y, anchor: options.anchor })
    }

    /// A move (C# `Translate(x, y)`).
    pub fn translate(x: f32, y: f32) -> DrawingTransform {
        DrawingTransform { translation: PointF::new(x, y), ..DrawingTransform::IDENTITY }
    }

    /// A turn about a point, degrees clockwise (C# `Rotate(degrees, anchor)`).
    pub fn rotate(degrees: f32, anchor: PointF) -> DrawingTransform {
        DrawingTransform { rotation: degrees, anchor, ..DrawingTransform::IDENTITY }
    }

    /// A uniform scale about a point (C# `Scale(scale, anchor)`).
    pub fn scale(scale: f32, anchor: PointF) -> DrawingTransform {
        DrawingTransform { scale_x: scale, scale_y: scale, anchor, ..DrawingTransform::IDENTITY }
    }

    /// The translation.
    pub fn translation(&self) -> PointF {
        self.translation
    }

    /// The rotation, degrees clockwise.
    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    /// The horizontal scale.
    pub fn scale_x(&self) -> f32 {
        self.scale_x
    }

    /// The vertical scale.
    pub fn scale_y(&self) -> f32 {
        self.scale_y
    }

    /// The anchor.
    pub fn anchor(&self) -> PointF {
        self.anchor
    }

    /// True when it changes nothing (the anchor alone does not count).
    pub fn is_identity(&self) -> bool {
        self.translation.x == 0.0 && self.translation.y == 0.0 && self.rotation == 0.0 && self.scale_x == 1.0 && self.scale_y == 1.0
    }

    /// A copy with another translation (C# `WithTranslation`). A non-finite value keeps the old.
    pub fn with_translation(&self, translation: PointF) -> DrawingTransform {
        DrawingTransform { translation: if finite(translation) { translation } else { self.translation }, ..*self }
    }

    /// A copy with another rotation (C# `WithRotation`).
    pub fn with_rotation(&self, degrees: f32) -> DrawingTransform {
        DrawingTransform { rotation: if degrees.is_finite() { degrees } else { self.rotation }, ..*self }
    }

    /// A copy with a uniform scale (C# `WithScale(scale)`).
    pub fn with_scale(&self, scale: f32) -> DrawingTransform {
        self.with_scale_xy(scale, scale)
    }

    /// A copy with a scale per axis (C# `WithScale(scaleX, scaleY)`).
    pub fn with_scale_xy(&self, scale_x: f32, scale_y: f32) -> DrawingTransform {
        let x = if scale_x.is_finite() { scale_x } else { self.scale_x };
        let y = if scale_y.is_finite() { scale_y } else { self.scale_y };
        DrawingTransform { scale_x: x, scale_y: y, ..*self }
    }

    /// A copy with another anchor (C# `WithAnchor`).
    pub fn with_anchor(&self, anchor: PointF) -> DrawingTransform {
        DrawingTransform { anchor: if finite(anchor) { anchor } else { self.anchor }, ..*self }
    }

    /// The matrix (C# `ToMatrix`, a `Matrix3x2`; here a kurbo `Affine` acting on column points):
    /// translate(-anchor), then scale, then rotate, then translate(anchor + translation).
    pub fn to_matrix(&self) -> Affine {
        if self.is_identity() {
            return Affine::IDENTITY;
        }
        let a = self.anchor;
        let t = self.translation;
        Affine::translate(((a.x + t.x) as f64, (a.y + t.y) as f64)) * Affine::rotate((self.rotation as f64).to_radians()) * Affine::scale_non_uniform(self.scale_x as f64, self.scale_y as f64) * Affine::translate((-a.x as f64, -a.y as f64))
    }

    /// A point through the transform (C# `TransformPoint`).
    pub fn transform_point(&self, point: PointF) -> PointF {
        if self.is_identity() {
            return point;
        }
        let p = self.to_matrix() * point.to_kurbo();
        PointF::new(p.x as f32, p.y as f32)
    }

    /// The axis-aligned box of a rectangle's four corners through the transform (C#
    /// `TransformBounds`).
    pub fn transform_bounds(&self, bounds: RectangleF) -> RectangleF {
        if self.is_identity() {
            return bounds;
        }
        transform_bounds_by(bounds, self.to_matrix())
    }
}

/// The axis-aligned box of a rectangle's corners through a matrix.
pub(crate) fn transform_bounds_by(bounds: RectangleF, matrix: Affine) -> RectangleF {
    let corners = [(bounds.x, bounds.y), (bounds.right(), bounds.y), (bounds.x, bounds.bottom()), (bounds.right(), bounds.bottom())].map(|(x, y)| matrix * kurbo::Point::new(x as f64, y as f64));
    let left = corners.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let top = corners.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let right = corners.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let bottom = corners.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    RectangleF::new(left as f32, top as f32, (right - left) as f32, (bottom - top) as f32)
}
