//! `BrushStrokeShape`: one painterly brushstroke - a filled, tapered lozenge laid along a centre
//! line that may bow sideways (C# `BrushStrokeShape`), plus how much of it is painted so far.

use kurbo::BezPath;

use crate::color::Color;
use crate::fills::ShapeFill;
use crate::geometry::{PointF, RectangleF};
use crate::models::ElementAxes;
use crate::paths::{PathGeometry, PathGeometryBuilder};

/// The optional parts of a [`BrushStrokeShape`] in a flat colour (C#'s default arguments: no
/// bend, the blunt [`BrushStrokeShape::DEFAULT_PROFILE`], the layer's colour).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushStrokeShapeOptions {
    /// How far it bows sideways at the middle, as a fraction of its length (0 = straight).
    pub bend: f32,
    /// How blunt its ends are: the exponent of the width profile, positive (2 pointed, 4 blunt,
    /// larger squarer).
    pub profile: f32,
    /// The stroke's colour; None to use the owning layer's colour.
    pub color: Option<Color>,
}

impl Default for BrushStrokeShapeOptions {
    fn default() -> BrushStrokeShapeOptions {
        BrushStrokeShapeOptions { bend: 0.0, profile: BrushStrokeShape::DEFAULT_PROFILE, color: None }
    }
}

/// One painterly brushstroke (C# `BrushStrokeShape`): a filled, tapered lozenge laid along a
/// centre line that may bow sideways - built from its centre, length, width, direction and bend.
/// It draws as an ordinary anti-aliased filled shape, and it reports its own axes
/// ([`BrushStrokeShape::get_axes`]), so in an ownership map every pixel it owns can be measured
/// along and across the stroke - the basis of bristle textures.
///
/// GEOMETRY, in the stroke's own axes (s runs from -0.5 at one end to +0.5 at the other; along =
/// s x length): the centre line sits `bend x length x (4 s^2 - 1)` across the axis (0 at the
/// ends, `-bend x length` at the middle), and the half width there is
/// `width / 2 x sqrt(max(0, 1 - |2 s|^profile))`. The outline is a polygon of
/// [`BrushStrokeShape::POINTS_PER_SIDE`] evenly spaced points per side (the two tips shared).
/// "Across" points to the right of the direction of travel on screen (y down): `(-sin angle, cos
/// angle)`. The angle is in DEGREES, clockwise on screen from +x - the unit of a transform's
/// rotation and of a path point's angle, so a direction read off a path can be passed straight
/// in. Computed in double, stored as single precision - the same points as C#.
///
/// PAINTING IT ON (no C# equivalent): [`BrushStrokeShape::progress`] says how much of the stroke
/// is painted, from its first tip (s = -0.5) towards the other - 1 (the default, and C#'s stroke)
/// is all of it, 0 none. The painted part is the outline cut straight across the axis at
/// `s = -0.5 + progress` (the brush's head); its points are the full outline's points up to the
/// cut plus the two points at the cut, computed by the same formula, so at progress 1 it is the
/// full outline exactly. Drawing, bounds and coverage (masks, ownership maps) use the painted
/// part; the AXES stay the whole stroke's, so a brush texture does not slide while the stroke is
/// painted on. Animate it with an [`AnimatedShape`](crate::shapes::AnimatedShape) whose factory
/// makes `stroke.with_progress(p(t))` for the time it is given.
#[derive(Clone, Debug, PartialEq)]
pub struct BrushStrokeShape {
    pub(crate) center_x: f32,
    pub(crate) center_y: f32,
    pub(crate) length: f32,
    pub(crate) width: f32,
    pub(crate) angle: f32,
    pub(crate) bend: f32,
    pub(crate) profile: f32,
    pub(crate) color: Option<Color>,
    pub(crate) fill: Option<ShapeFill>,
    progress: f32,
    outline: Vec<PointF>,
    drawn: Vec<PointF>,
    bounds: RectangleF,
}

fn validate(center_x: f32, center_y: f32, length: f32, width: f32, angle: f32, bend: f32, profile: f32) -> Result<(), String> {
    if !center_x.is_finite() || !center_y.is_finite() {
        return Err("A brushstroke's centre must be a finite point.".to_string());
    }
    if !length.is_finite() || length <= 0.0 {
        return Err("A brushstroke's length must be a positive, finite distance.".to_string());
    }
    if !width.is_finite() || width <= 0.0 {
        return Err("A brushstroke's width must be a positive, finite distance.".to_string());
    }
    if !angle.is_finite() {
        return Err("A brushstroke's angle must be a finite number of degrees.".to_string());
    }
    if !bend.is_finite() {
        return Err("A brushstroke's bend must be a finite fraction of its length (0 for straight).".to_string());
    }
    if !profile.is_finite() || profile <= 0.0 {
        return Err("A brushstroke's profile must be a positive, finite exponent (4 is the default).".to_string());
    }
    Ok(())
}

/// The box around points (C#'s `DrawingShape.BoundsOf`).
pub(crate) fn bounds_of(points: &[PointF]) -> RectangleF {
    let Some(first) = points.first() else {
        return RectangleF::default();
    };
    let (mut left, mut top, mut right, mut bottom) = (first.x, first.y, first.x, first.y);
    for p in &points[1..] {
        left = left.min(p.x);
        top = top.min(p.y);
        right = right.max(p.x);
        bottom = bottom.max(p.y);
    }
    RectangleF::new(left, top, right - left, bottom - top)
}

impl BrushStrokeShape {
    /// The default profile: a blunt-ended lozenge (C# `DefaultProfile`).
    pub const DEFAULT_PROFILE: f32 = 4.0;

    /// How many outline points each side of the stroke has, the two tips included (C#
    /// `PointsPerSide`).
    pub const POINTS_PER_SIDE: usize = 17;

    /// A brushstroke in a flat colour (C#'s first constructor): its centre, its length end to
    /// end (positive), its width at the middle (positive), its direction in degrees clockwise on
    /// screen from +x; bend, profile and colour in the options. The error says what to do.
    pub fn new(center_x: f32, center_y: f32, length: f32, width: f32, angle: f32, options: BrushStrokeShapeOptions) -> Result<BrushStrokeShape, String> {
        validate(center_x, center_y, length, width, angle, options.bend, options.profile)?;
        Ok(BrushStrokeShape::build(center_x, center_y, length, width, angle, options.bend, options.profile, options.color, None))
    }

    /// A brushstroke painted by a fill - for example a gradient along the stroke - instead of a
    /// flat colour (C#'s second constructor; same arguments, then the fill).
    pub fn with_fill(center_x: f32, center_y: f32, length: f32, width: f32, angle: f32, bend: f32, profile: f32, fill: ShapeFill) -> Result<BrushStrokeShape, String> {
        validate(center_x, center_y, length, width, angle, bend, profile)?;
        Ok(BrushStrokeShape::build(center_x, center_y, length, width, angle, bend, profile, None, Some(fill)))
    }

    fn build(center_x: f32, center_y: f32, length: f32, width: f32, angle: f32, bend: f32, profile: f32, color: Option<Color>, fill: Option<ShapeFill>) -> BrushStrokeShape {
        let mut stroke = BrushStrokeShape { center_x, center_y, length, width, angle, bend, profile, color, fill, progress: 1.0, outline: Vec::new(), drawn: Vec::new(), bounds: RectangleF::default() };
        stroke.outline = stroke.build_outline();
        stroke.drawn = stroke.outline.clone();
        stroke.bounds = bounds_of(&stroke.outline);
        stroke
    }

    // C#'s BuildOutline: one side from tip to tip, then back along the other side (the tips are
    // shared). Computed in double: the angle in radians, the across offsets, then the rotation
    // about the centre; stored as floats.
    fn build_outline(&self) -> Vec<PointF> {
        const N: usize = BrushStrokeShape::POINTS_PER_SIDE;
        let mut points = vec![PointF::default(); (2 * N) - 2];
        let (cos, sin) = self.cos_sin();
        for i in 0..N {
            let s = -0.5 + (i as f64 / (N - 1) as f64);
            let (along, upper, lower) = self.section(s);
            points[i] = self.place(along, upper, cos, sin);
            if i > 0 && i < N - 1 {
                points[(2 * N) - 1 - i - 1] = self.place(along, lower, cos, sin);
            }
        }
        points
    }

    fn cos_sin(&self) -> (f64, f64) {
        let radians = self.angle as f64 * (std::f64::consts::PI / 180.0);
        (radians.cos(), radians.sin())
    }

    // At s: the distance along the axis, and the two edges' distances across it (C#'s formulas
    // in C#'s order).
    fn section(&self, s: f64) -> (f64, f64, f64) {
        let along = s * self.length as f64;
        let bow = self.bend as f64 * self.length as f64 * ((4.0 * s * s) - 1.0);
        let half = (1.0 - (2.0 * s).abs().powf(self.profile as f64)).max(0.0).sqrt() * self.width as f64 / 2.0;
        (along, bow + half, bow - half)
    }

    fn place(&self, along: f64, across: f64, cos: f64, sin: f64) -> PointF {
        PointF::new((self.center_x as f64 + (along * cos) - (across * sin)) as f32, (self.center_y as f64 + (along * sin) + (across * cos)) as f32)
    }

    // The painted part's polygon: the full outline's points before the cut, then the two points
    // at the cut, then back along the other side.
    fn build_drawn(&self) -> Vec<PointF> {
        const N: usize = BrushStrokeShape::POINTS_PER_SIDE;
        if !(self.progress > 0.0) {
            return Vec::new();
        }
        if self.progress >= 1.0 {
            return self.outline.clone();
        }
        let cut = -0.5 + self.progress as f64;
        let (cos, sin) = self.cos_sin();
        // The side points strictly before the cut (index 0 is the shared first tip).
        let before = (0..N).take_while(|&i| -0.5 + (i as f64 / (N - 1) as f64) < cut).count();
        let mut points = Vec::with_capacity((2 * before) + 1);
        points.extend_from_slice(&self.outline[..before]);
        let (along, upper, lower) = self.section(cut);
        points.push(self.place(along, upper, cos, sin));
        points.push(self.place(along, lower, cos, sin));
        for i in (1..before).rev() {
            points.push(self.outline[(2 * N) - 1 - i - 1]);
        }
        points
    }

    /// The horizontal position of the stroke's centre (C# `CenterX`).
    pub fn center_x(&self) -> f32 {
        self.center_x
    }

    /// The vertical position of the stroke's centre (C# `CenterY`).
    pub fn center_y(&self) -> f32 {
        self.center_y
    }

    /// The length, end to end (C# `Length`).
    pub fn length(&self) -> f32 {
        self.length
    }

    /// The width at the middle (C# `Width`).
    pub fn width(&self) -> f32 {
        self.width
    }

    /// The direction, degrees clockwise on screen from +x (C# `Angle`).
    pub fn angle(&self) -> f32 {
        self.angle
    }

    /// How far it bows sideways at the middle, as a fraction of its length (C# `Bend`).
    pub fn bend(&self) -> f32 {
        self.bend
    }

    /// The exponent of the width profile (C# `Profile`).
    pub fn profile(&self) -> f32 {
        self.profile
    }

    /// Its own colour; None when it takes the layer's (C# `Color`).
    pub fn color(&self) -> Option<Color> {
        self.color
    }

    /// The fill that paints it, when it has one (C# `Fill`).
    pub fn fill(&self) -> Option<&ShapeFill> {
        self.fill.as_ref()
    }

    /// How much of the stroke is painted, 0..1 from its first tip (no C# equivalent; 1 by
    /// default - C#'s stroke).
    pub fn progress(&self) -> f32 {
        self.progress
    }

    /// Sets how much of the stroke is painted (clamped to 0..1; NaN counts as 0).
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = if progress.is_nan() { 0.0 } else { progress.clamp(0.0, 1.0) };
        self.drawn = self.build_drawn();
        self.bounds = if self.drawn.is_empty() { RectangleF::new(self.outline[0].x, self.outline[0].y, 0.0, 0.0) } else { bounds_of(&self.drawn) };
    }

    /// The same stroke painted to a progress (see [`BrushStrokeShape::set_progress`]).
    pub fn with_progress(mut self, progress: f32) -> BrushStrokeShape {
        self.set_progress(progress);
        self
    }

    /// A copy of the WHOLE outline polygon's points, in order around the stroke (C# `GetPoints`;
    /// independent of the progress).
    pub fn get_points(&self) -> Vec<PointF> {
        self.outline.clone()
    }

    /// The painted part's polygon (the whole outline at progress 1; empty at 0).
    pub fn get_drawn_points(&self) -> Vec<PointF> {
        self.drawn.clone()
    }

    /// The painted part as a closed geometry, non-zero fill rule (C# `GetOutline` at progress 1;
    /// empty at progress 0).
    pub fn get_outline(&self) -> PathGeometry {
        if self.drawn.len() < 2 {
            return PathGeometry::empty();
        }
        PathGeometryBuilder::new().add_polygon(&self.drawn, true).build()
    }

    /// The area the stroke covers: its (painted) outline polygon (C# `GetCoverageAreas`; the
    /// tolerance is not used - the polygon is filled).
    pub fn get_coverage_areas(&self, _tolerance: f32) -> Vec<PathGeometry> {
        if self.drawn.len() < 3 {
            return Vec::new();
        }
        vec![self.get_outline()]
    }

    /// The stroke's own axes: its centre, its direction, its length and its width - unbent (the
    /// bow lies inside them) and whole, whatever the progress (C# `GetAxes`).
    pub fn get_axes(&self) -> ElementAxes {
        ElementAxes::unchecked(PointF::new(self.center_x, self.center_y), self.angle, self.length, self.width)
    }

    /// The box around the painted outline polygon (C# `GetBounds`; a point at the first tip when
    /// nothing is painted).
    pub fn get_bounds(&self) -> RectangleF {
        self.bounds
    }

    /// The painted part as a kurbo path (what the renderer fills).
    pub(crate) fn drawn_path(&self) -> BezPath {
        let mut path = BezPath::new();
        for (i, p) in self.drawn.iter().enumerate() {
            if i == 0 {
                path.move_to(p.to_kurbo());
            } else {
                path.line_to(p.to_kurbo());
            }
        }
        if !self.drawn.is_empty() {
            path.close_path();
        }
        path
    }
}
