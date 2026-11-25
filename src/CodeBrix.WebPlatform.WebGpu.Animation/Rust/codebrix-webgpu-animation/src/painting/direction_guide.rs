//! `DirectionGuide`: a hand-placed hint for a direction field - strokes along a path, around a
//! point, away from a point, or one way (no C# equivalent).

use crate::geometry::PointF;
use crate::paths::PathGeometry;

/// A hand-placed guide for [`DirectionField::from_guides`](crate::painting::DirectionField::from_guides)
/// (no C# equivalent). Each guide gives a direction at every point within its `reach`
/// (calibrated units; `f32::INFINITY` = everywhere) and pulls with its `weight` (0 or more),
/// falling off with distance - see `from_guides` for the rule. Angles are degrees clockwise on
/// screen from +x; a direction has no head or tail (0 and 180 degrees are the same).
#[derive(Clone, Debug, PartialEq)]
pub enum DirectionGuide {
    /// Strokes follow a path (its tangent at the nearest point), within `reach` of it.
    Along {
        /// The path (its curves flattened at the default tolerance, 0.25).
        path: PathGeometry,
        /// How far from the path it reaches.
        reach: f32,
        /// How strongly it pulls.
        weight: f32,
    },
    /// Strokes circle a point (a swirl: tangential), turned `inward` degrees from the tangent
    /// within `reach` of the centre: 0 = circles; positive = a spiral that comes IN going
    /// clockwise round the centre on screen (and out going anticlockwise - a direction has no
    /// head), negative the mirror spiral.
    Around {
        /// The centre of the swirl.
        center: PointF,
        /// How far from the centre it reaches.
        reach: f32,
        /// Degrees the direction is turned from the tangent towards the centre.
        inward: f32,
        /// How strongly it pulls.
        weight: f32,
    },
    /// Strokes point away from (and so towards) a point: rays of light, a halo; within `reach`.
    Radial {
        /// The point the rays leave.
        center: PointF,
        /// How far from the point it reaches.
        reach: f32,
        /// How strongly it pulls.
        weight: f32,
    },
    /// One direction everywhere within `reach` of `center` (a calm sea's horizontal), or
    /// everywhere if `reach` is infinite; `center` is also where its pull is strongest.
    Fixed {
        /// The direction, degrees.
        angle: f32,
        /// Where it applies from.
        center: PointF,
        /// How far from the centre it reaches.
        reach: f32,
        /// How strongly it pulls.
        weight: f32,
    },
}

/// A guide made ready for sampling: an Along guide's path flattened into segments once.
pub(crate) enum PreparedGuide {
    Along { segments: Vec<(PointF, PointF)>, reach: f32, weight: f32 },
    Around { center: PointF, reach: f32, inward: f32, weight: f32 },
    Radial { center: PointF, reach: f32, weight: f32 },
    Fixed { angle: f32, center: PointF, reach: f32, weight: f32 },
}

fn check_common(index: usize, reach: f32, weight: f32) -> Result<(), String> {
    if !(reach > 0.0) {
        return Err(format!("Direction guide {index}: the reach must be a positive distance (f32::INFINITY for everywhere)."));
    }
    if !weight.is_finite() || weight < 0.0 {
        return Err(format!("Direction guide {index}: the weight must be finite, 0 or more."));
    }
    Ok(())
}

fn check_point(index: usize, p: PointF) -> Result<(), String> {
    if !p.x.is_finite() || !p.y.is_finite() {
        return Err(format!("Direction guide {index}: the centre must be a finite point."));
    }
    Ok(())
}

impl DirectionGuide {
    /// Checks the guide and flattens an Along guide's path; the error names the guide.
    pub(crate) fn prepare(&self, index: usize) -> Result<PreparedGuide, String> {
        match self {
            DirectionGuide::Along { path, reach, weight } => {
                check_common(index, *reach, *weight)?;
                let mut segments = Vec::new();
                for polyline in path.flatten(None)? {
                    let points = polyline.get_points();
                    for pair in points.windows(2) {
                        segments.push((pair[0], pair[1]));
                    }
                    if polyline.is_closed() && points.len() > 2 {
                        segments.push((points[points.len() - 1], points[0]));
                    }
                }
                segments.retain(|(a, b)| a != b);
                if segments.is_empty() {
                    return Err(format!("Direction guide {index}: the path has no length to follow - give it at least one line or curve."));
                }
                Ok(PreparedGuide::Along { segments, reach: *reach, weight: *weight })
            }
            DirectionGuide::Around { center, reach, inward, weight } => {
                check_common(index, *reach, *weight)?;
                check_point(index, *center)?;
                if !inward.is_finite() {
                    return Err(format!("Direction guide {index}: the inward turn must be a finite number of degrees."));
                }
                Ok(PreparedGuide::Around { center: *center, reach: *reach, inward: *inward, weight: *weight })
            }
            DirectionGuide::Radial { center, reach, weight } => {
                check_common(index, *reach, *weight)?;
                check_point(index, *center)?;
                Ok(PreparedGuide::Radial { center: *center, reach: *reach, weight: *weight })
            }
            DirectionGuide::Fixed { angle, center, reach, weight } => {
                check_common(index, *reach, *weight)?;
                check_point(index, *center)?;
                if !angle.is_finite() {
                    return Err(format!("Direction guide {index}: the angle must be a finite number of degrees."));
                }
                Ok(PreparedGuide::Fixed { angle: *angle, center: *center, reach: *reach, weight: *weight })
            }
        }
    }
}

impl PreparedGuide {
    /// At a point: the guide's direction (degrees, double) and its pull (0 = none), the pull
    /// being `weight / (1 + d^2) x (1 - (d / reach)^2)^2` within reach (the last factor 1 for an
    /// infinite reach), d the distance to the path or centre. A point exactly at an Around or
    /// Radial centre has no direction and no pull.
    pub(crate) fn sample(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let pull = |d: f64, reach: f32, weight: f32| -> Option<f64> {
            if !(d < reach as f64) {
                return None;
            }
            let taper = if reach.is_infinite() { 1.0 } else { (1.0 - (d / reach as f64).powi(2)).powi(2) };
            let w = weight as f64 / (1.0 + (d * d)) * taper;
            if w > 0.0 {
                Some(w)
            } else {
                None
            }
        };
        match self {
            PreparedGuide::Along { segments, reach, weight } => {
                let mut best = (f64::INFINITY, 0.0f64);
                for (a, b) in segments {
                    let (ax, ay, bx, by) = (a.x as f64, a.y as f64, b.x as f64, b.y as f64);
                    let (dx, dy) = (bx - ax, by - ay);
                    let t = ((((x - ax) * dx) + ((y - ay) * dy)) / ((dx * dx) + (dy * dy))).clamp(0.0, 1.0);
                    let d = (x - (ax + (dx * t))).hypot(y - (ay + (dy * t)));
                    if d < best.0 {
                        best = (d, dy.atan2(dx).to_degrees());
                    }
                }
                Some((best.1, pull(best.0, *reach, *weight)?))
            }
            PreparedGuide::Around { center, reach, inward, weight } => {
                let (dx, dy) = (x - center.x as f64, y - center.y as f64);
                let d = dx.hypot(dy);
                if d == 0.0 {
                    return None;
                }
                Some((dy.atan2(dx).to_degrees() + 90.0 + *inward as f64, pull(d, *reach, *weight)?))
            }
            PreparedGuide::Radial { center, reach, weight } => {
                let (dx, dy) = (x - center.x as f64, y - center.y as f64);
                let d = dx.hypot(dy);
                if d == 0.0 {
                    return None;
                }
                Some((dy.atan2(dx).to_degrees(), pull(d, *reach, *weight)?))
            }
            PreparedGuide::Fixed { angle, center, reach, weight } => {
                let d = (x - center.x as f64).hypot(y - center.y as f64);
                Some((*angle as f64, pull(d, *reach, *weight)?))
            }
        }
    }
}
