//! `StrokeLayout`: where the strokes of a painting go - an area filled band by band (broad
//! strokes everywhere, smaller ones where a detail map asks for them), each stroke posed by a
//! direction field and coloured by a function - and `element_area`, the area an element covers
//! in its own coordinates (no C# equivalent). Planning-time CPU work, deterministic.

use crate::color::Color;
use crate::formations::{FormationArea, FormationTargetOptions, FormationTargets};
use crate::geometry::{PointF, RectangleF};
use crate::DrawingSession;
use crate::models::ElementId;
use crate::painting::{DetailMap, DirectionField};
use crate::paths::{PathBooleanOperation, PathGeometry};
use crate::planes::{DrawingPlane, DrawingRandom};
use crate::shapes::ShapeKind;
use crate::strokes::BrushStrokeRecord;

/// One band of stroke sizes for [`StrokeLayout::fill`] (no C# equivalent): the strokes' size,
/// how far apart they are placed, and where the band applies. Sizes are numbers - the crate has
/// no brush styles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeSize {
    /// The strokes' length, calibrated units (positive).
    pub length: f32,
    /// The strokes' width, calibrated units (positive).
    pub width: f32,
    /// The grid spacing they are placed at, calibrated units (positive; see
    /// [`FormationTargets::fill`]).
    pub spacing: f32,
    /// The band is placed where the detail map is at least this (0 or less: everywhere, also
    /// without a detail map).
    pub min_detail: f32,
    /// Each stroke's length and width are scaled together by `1 + variation x u`, u drawn
    /// uniformly in -1..1 per stroke: 0 (the default) .. below 1.
    pub variation: f32,
}

impl StrokeSize {
    /// A band everywhere (`min_detail` 0) with no size variation.
    pub fn new(length: f32, width: f32, spacing: f32) -> StrokeSize {
        StrokeSize { length, width, spacing, min_detail: 0.0, variation: 0.0 }
    }
}

/// The options of [`StrokeLayout::fill`] (no C# equivalent).
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeLayoutOptions {
    /// The bands, broadest first (at least one).
    pub sizes: Vec<StrokeSize>,
    /// How far each place is moved off the grid, a fraction of the spacing (0.33, as
    /// FormationTargetOptions).
    pub jitter: f32,
    /// The deviation of each stroke's angle from the field's, degrees (a normal draw; 6).
    pub angle_jitter: f32,
    /// The seed of every draw (1).
    pub seed: i64,
}

impl Default for StrokeLayoutOptions {
    fn default() -> StrokeLayoutOptions {
        StrokeLayoutOptions { sizes: Vec::new(), jitter: 0.33, angle_jitter: 6.0, seed: 1 }
    }
}

/// Laying strokes out over an area (no C# equivalent).
pub struct StrokeLayout;

impl StrokeLayout {
    /// Strokes filling an area, band by band - broad strokes everywhere, smaller ones where the
    /// detail map asks for them. Each band is [`FormationTargets::fill`] over the area (the
    /// band's spacing, the options' jitter, no edge clearance, a seed drawn for the band), keeping
    /// the places where the detail map (a plane over its rectangle, read with
    /// [`DetailMap::sample`]; 0 without one) is at least the band's `min_detail`. Each place gets
    /// a pose from the field ([`DirectionField::stroke_pose`] at the band's length, the angle
    /// jittered), the band's size (varied), the colour `color_at` gives at the place (a
    /// picture's colour, a function), and a seed of its own; times are left
    /// [`BrushStrokeRecord::ALWAYS`] (a [`PaintingTimeline`](crate::painting::PaintingTimeline)
    /// gives them). Returns the records band by band, in each band's fill order. Deterministic:
    /// the same inputs and seed give the same records. The error says which option is unusable.
    pub fn fill(area: &FormationArea, field: &DirectionField, detail: Option<(&DrawingPlane, RectangleF)>, color_at: &dyn Fn(f32, f32) -> Color, options: &StrokeLayoutOptions) -> Result<Vec<Vec<BrushStrokeRecord>>, String> {
        if options.sizes.is_empty() {
            return Err("Give the layout at least one stroke size (StrokeLayoutOptions::sizes).".to_string());
        }
        if !options.jitter.is_finite() || options.jitter < 0.0 {
            return Err("The jitter must be a finite fraction, 0 or more.".to_string());
        }
        if !options.angle_jitter.is_finite() || options.angle_jitter < 0.0 {
            return Err("The angle jitter must be a finite number of degrees, 0 or more.".to_string());
        }
        for (i, size) in options.sizes.iter().enumerate() {
            let positive = |v: f32| v.is_finite() && v > 0.0;
            if !positive(size.length) || !positive(size.width) || !positive(size.spacing) {
                return Err(format!("Stroke size {i}: the length, width and spacing must be positive, finite distances."));
            }
            if size.min_detail.is_nan() {
                return Err(format!("Stroke size {i}: the least detail must be a number."));
            }
            if !(size.variation >= 0.0 && size.variation < 1.0) {
                return Err(format!("Stroke size {i}: the variation must be 0 or more and below 1."));
            }
        }
        let mut bands = Vec::with_capacity(options.sizes.len());
        for (band, size) in options.sizes.iter().enumerate() {
            let mut random = DrawingRandom::for_key(options.seed, band as i64);
            let target_seed = random.next_uint64() as i64;
            let targets = FormationTargets::fill(area, FormationTargetOptions { spacing: size.spacing, jitter: options.jitter, edge_clearance: 0.0, seed: target_seed, order: None })?;
            let mut records = Vec::new();
            for target in targets {
                let (x, y) = (target.position.x, target.position.y);
                let level = detail.map(|(plane, rectangle)| DetailMap::sample(plane, rectangle, x, y)).unwrap_or(0.0);
                if size.min_detail > 0.0 && !(level >= size.min_detail) {
                    continue;
                }
                let scale = 1.0 + (size.variation as f64 * random.uniform(-1.0, 1.0)) as f32;
                let length = size.length * scale;
                let (angle, bend) = field.stroke_pose(target.position, length);
                let angle = angle + random.normal(0.0, options.angle_jitter as f64) as f32;
                let seed = (random.next_uint64() >> 32) as u32;
                let mut record = BrushStrokeRecord::new(PointF::new(x, y), length, size.width * scale, angle, color_at(x, y));
                record.bend = bend;
                record.seed = seed;
                records.push(record);
            }
            bands.push(records);
        }
        Ok(bands)
    }
}

/// The area an element covers at a time, in the element's OWN coordinates (its coverage areas -
/// [`DrawingShape::get_coverage_areas`](crate::shapes::DrawingShape::get_coverage_areas) at
/// tolerance 0.25 - without its transform; animated shapes inside it rebuilt for `seconds`, the
/// session itself not changed): fill it with [`StrokeLayout::fill`] and anchor the strokes to the
/// element so they travel with it. Several coverage areas are united. No C# equivalent. The
/// error says what to do.
pub fn element_area(session: &DrawingSession, element: ElementId, seconds: f64) -> Result<FormationArea, String> {
    let found = session.find_element(element).ok_or_else(|| format!("The session has no element {}: pass the id an element of this session was drawn with.", element.value()))?;
    let mut shape = found.clone();
    if seconds.is_finite() {
        shape.visit_mut(&mut |e| {
            if let ShapeKind::Animated(a) = e.kind_mut() {
                a.update(seconds);
            }
        });
    }
    let areas = shape.get_coverage_areas(0.25)?;
    let geometry = match areas.len() {
        0 => return Err(format!("Element {} covers nothing at {seconds} s: there is no area to fill.", element.value())),
        1 => areas.into_iter().next().unwrap_or_else(PathGeometry::empty),
        _ => PathGeometry::combine_all(&areas, PathBooleanOperation::Union, None)?,
    };
    FormationArea::from_geometry(&geometry, None)
}
