//! `BrushStrokeRecord`: one brushstroke of a stroke set as plain data - the geometry of a
//! `BrushStrokeShape`, its colour and paint load, its brush tip and anchors, and when it is
//! painted (no C# equivalent).

use crate::color::Color;
use crate::geometry::{PointF, RectangleF};
use crate::shapes::{BrushStrokeShape, BrushStrokeShapeOptions};

/// One brushstroke of a [`BrushStrokeSet`](crate::strokes::BrushStrokeSet): plain data, the
/// geometry of [`BrushStrokeShape`] (no C# equivalent). Units are the set's own calibrated
/// coordinates; the angle is in DEGREES clockwise on screen from +x (BrushStrokeShape's unit).
///
/// THE GEOMETRY is BrushStrokeShape's formula: in the stroke's own axes (s from -0.5 at the first
/// tip to +0.5 at the other) the centre line sits `bend x length x (4 s^2 - 1)` across the axis
/// and the half width is `width / 2 x sqrt(max(0, 1 - |2 s|^profile))`.
///
/// PAINTING IT ON: the brush touches down at `start` (session seconds) and lifts at `end`, when
/// the stroke is whole; [`BrushStrokeRecord::progress_at`] is how much is painted at a time. A
/// record whose `start` and `end` are both [`BrushStrokeRecord::ALWAYS`] is always whole.
///
/// The fields are public; a [`BrushStrokeSet`](crate::strokes::BrushStrokeSet) checks every
/// record once when it is made, with a message that names the record and what is wrong.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushStrokeRecord {
    /// The centre of the stroke (the middle of its axis), set coordinates.
    pub center: PointF,
    /// The length, end to end: positive.
    pub length: f32,
    /// The width at the middle: positive.
    pub width: f32,
    /// The direction, degrees clockwise on screen from +x.
    pub angle: f32,
    /// How far it bows sideways at the middle, a fraction of its length (0 = straight).
    pub bend: f32,
    /// The exponent of the width profile, positive ([`BrushStrokeShape::DEFAULT_PROFILE`] = 4).
    pub profile: f32,
    /// The stroke's own colour (straight sRGB).
    pub color: Color,
    /// How much paint it carries, 0..1: a factor on its alpha (and, with a surface finish, on its
    /// thickness). 1 by default.
    pub load: f32,
    /// Which brush tip of the drawing shape paints it, an index into the shape's tips: 0 = no
    /// picture (a solid lozenge, the default).
    pub tip: u16,
    /// The two anchors of the drawing shape it follows ([0, 0] by default: anchor 0).
    pub anchors: [u16; 2],
    /// Between the two anchors: 0 = the first only (the default) .. 1 = the second only.
    pub anchor_blend: f32,
    /// Session seconds the brush touches down ([`BrushStrokeRecord::ALWAYS`] by default).
    pub start: f32,
    /// Session seconds it lifts, the stroke whole ([`BrushStrokeRecord::ALWAYS`] by default).
    pub end: f32,
    /// The stroke's own variation number (boil, broken colour, tip mirroring). 0 by default.
    pub seed: u32,
}

impl BrushStrokeRecord {
    /// The start and end of a stroke that is always whole: a time before every time a session
    /// is shown at.
    pub const ALWAYS: f32 = -1.0e30;

    /// A stroke always fully painted (start = end = [`BrushStrokeRecord::ALWAYS`]): straight, the
    /// default profile, load 1, no tip, anchor 0, seed 0.
    pub fn new(center: PointF, length: f32, width: f32, angle: f32, color: Color) -> BrushStrokeRecord {
        BrushStrokeRecord {
            center,
            length,
            width,
            angle,
            bend: 0.0,
            profile: BrushStrokeShape::DEFAULT_PROFILE,
            color,
            load: 1.0,
            tip: 0,
            anchors: [0, 0],
            anchor_blend: 0.0,
            start: BrushStrokeRecord::ALWAYS,
            end: BrushStrokeRecord::ALWAYS,
            seed: 0,
        }
    }

    /// How much is painted at a time, 0..1 from the first tip: when `end <= start`, 1 from
    /// `start` on and 0 before; otherwise `clamp((t - start) / (end - start), 0, 1)` (computed in
    /// double). A time that is NaN gives 0. The GPU evaluates the same formula.
    pub fn progress_at(&self, seconds: f64) -> f32 {
        let (start, end) = (self.start as f64, self.end as f64);
        if !(seconds >= start) {
            return 0.0;
        }
        if end <= start {
            return 1.0;
        }
        ((seconds - start) / (end - start)).clamp(0.0, 1.0) as f32
    }

    /// True when the brush has touched down at a time (`start <= time`; the stroke may still show
    /// nothing at exactly its start).
    pub fn has_started(&self, seconds: f64) -> bool {
        seconds >= self.start as f64
    }

    /// The CPU twin: the [`BrushStrokeShape`] this record draws at a time (for ownership maps,
    /// masks, tests, small counts) - its geometry, its colour with the alpha multiplied by the
    /// load (rounded), painted to [`BrushStrokeRecord::progress_at`]. Tips, anchors and seeds are
    /// the drawing shape's business and play no part. The error is BrushStrokeShape's.
    pub fn to_shape(&self, seconds: f64) -> Result<BrushStrokeShape, String> {
        let alpha = (self.color.a as f32 * self.load.clamp(0.0, 1.0)).round().clamp(0.0, 255.0) as u8;
        let options = BrushStrokeShapeOptions { bend: self.bend, profile: self.profile, color: Some(self.color.with_alpha(alpha)) };
        Ok(BrushStrokeShape::new(self.center.x, self.center.y, self.length, self.width, self.angle, options)?.with_progress(self.progress_at(seconds)))
    }

    /// A record of a shape's geometry and colour, always whole (start = end =
    /// [`BrushStrokeRecord::ALWAYS`]), the other fields as [`BrushStrokeRecord::new`] sets them. A
    /// record has a colour of its own, so a shape that takes its layer's colour or is painted by a
    /// fill gives opaque black: set the record's colour. The shape's progress is not carried: a
    /// record's painting-on is its start and end.
    pub fn from_shape(shape: &BrushStrokeShape) -> BrushStrokeRecord {
        let mut record = BrushStrokeRecord::new(PointF::new(shape.center_x(), shape.center_y()), shape.length(), shape.width(), shape.angle(), shape.color().unwrap_or(Color::BLACK));
        record.bend = shape.bend();
        record.profile = shape.profile();
        record
    }

    /// A box that holds the whole stroke (bend included), set coordinates: the box of the
    /// stroke's own rectangle - its length along the axis, across it from the bow's furthest
    /// point to the width's edges - turned by its angle. It is the outline's own box for a
    /// stroke along an axis, and may be a little larger for a turned one.
    pub fn bounds(&self) -> RectangleF {
        let half_length = self.length as f64 / 2.0;
        let half_width = self.width as f64 / 2.0;
        let bow = -(self.bend as f64) * self.length as f64;
        let (across_low, across_high) = (bow.min(0.0) - half_width, bow.max(0.0) + half_width);
        let radians = self.angle as f64 * (std::f64::consts::PI / 180.0);
        let (cos, sin) = (radians.cos(), radians.sin());
        let (mut left, mut top, mut right, mut bottom) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for along in [-half_length, half_length] {
            for across in [across_low, across_high] {
                let x = self.center.x as f64 + (along * cos) - (across * sin);
                let y = self.center.y as f64 + (along * sin) + (across * cos);
                left = left.min(x);
                right = right.max(x);
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
        RectangleF::new(left as f32, top as f32, (right - left) as f32, (bottom - top) as f32)
    }

    /// The checks a set makes: BrushStrokeShape's (with its messages), then the times, the load
    /// and the anchor blend.
    pub(crate) fn validate(&self) -> Result<(), String> {
        // BrushStrokeShape's checks and messages (strokes/tests.rs keeps them equal).
        if !self.center.x.is_finite() || !self.center.y.is_finite() {
            return Err("A brushstroke's centre must be a finite point.".to_string());
        }
        if !self.length.is_finite() || self.length <= 0.0 {
            return Err("A brushstroke's length must be a positive, finite distance.".to_string());
        }
        if !self.width.is_finite() || self.width <= 0.0 {
            return Err("A brushstroke's width must be a positive, finite distance.".to_string());
        }
        if !self.angle.is_finite() {
            return Err("A brushstroke's angle must be a finite number of degrees.".to_string());
        }
        if !self.bend.is_finite() {
            return Err("A brushstroke's bend must be a finite fraction of its length (0 for straight).".to_string());
        }
        if !self.profile.is_finite() || self.profile <= 0.0 {
            return Err("A brushstroke's profile must be a positive, finite exponent (4 is the default).".to_string());
        }
        if !self.start.is_finite() || !self.end.is_finite() {
            return Err("A brushstroke's start and end must be finite seconds (BrushStrokeRecord::ALWAYS for a stroke that is always whole).".to_string());
        }
        if self.end < self.start {
            return Err("A brushstroke's end must not be before its start.".to_string());
        }
        if !(self.load >= 0.0 && self.load <= 1.0) {
            return Err("A brushstroke's load must be a fraction, 0..1.".to_string());
        }
        if !(self.anchor_blend >= 0.0 && self.anchor_blend <= 1.0) {
            return Err("A brushstroke's anchor blend must be a fraction, 0..1 (0 = the first anchor only).".to_string());
        }
        Ok(())
    }
}
