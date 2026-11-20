//! `FormationTargets`: the target points that fill an area (C# `FormationTargets`).

use crate::formations::dotnet_sort::sort_unstable_as_dotnet;
use crate::formations::{FormationArea, FormationTarget, FormationTargetOptions, RandomArrivalOrder};
use crate::geometry::PointF;
use crate::paths::PathGeometry;
use crate::planes::DrawingRandom;

/// Filling an area with target points (C# `FormationTargets`).
pub struct FormationTargets;

impl FormationTargets {
    /// The targets filling a geometry's area (C# `Fill(PathGeometry, options)`; the geometry is
    /// flattened at the default tolerance, 0.25).
    pub fn fill_geometry(area: &PathGeometry, options: FormationTargetOptions) -> Result<Vec<FormationTarget>, String> {
        FormationTargets::fill(&FormationArea::from_geometry(area, None)?, options)
    }

    /// The targets filling an area, sorted by their keys - the arrival order (C# `Fill(area,
    /// options)`; exactly C#'s grid, jitter draws and sort, so the same seed gives the same
    /// targets in the same order). The error says which option is out of range.
    pub fn fill(area: &FormationArea, options: FormationTargetOptions) -> Result<Vec<FormationTarget>, String> {
        options.validate()?;
        let mut order = options.order.unwrap_or_else(|| Box::new(RandomArrivalOrder::new()));
        order.prepare(area, options.seed);
        let mut random = DrawingRandom::new(options.seed);
        let spacing = options.spacing;
        let row_step = spacing * 0.866;
        let jitter = spacing * options.jitter;
        let bounds = area.bounds();
        let mut targets = Vec::new();
        let mut row = 0u32;
        let mut y = bounds.bottom() - (row_step * 0.5);
        while y > bounds.y {
            let shift = if (row & 1) == 0 { 0.0 } else { spacing * 0.5 };
            let mut x0 = bounds.x + shift;
            while x0 < bounds.right() {
                let x = x0 + random.uniform(-jitter as f64, jitter as f64) as f32;
                let yy = y + random.uniform(-jitter as f64, jitter as f64) as f32;
                x0 += spacing;
                if !area.contains(x, yy) {
                    continue;
                }
                let clearance = area.distance_to_edge(x, yy);
                if clearance < options.edge_clearance {
                    continue;
                }
                let key = order.get_key(x, yy, clearance);
                if key.is_infinite() || key.is_nan() {
                    continue;
                }
                targets.push(FormationTarget::new(PointF::new(x, yy), key, clearance));
            }
            y -= row_step;
            row += 1;
        }
        sort_unstable_as_dotnet(&mut targets, |a, b| a.key.partial_cmp(&b.key).unwrap_or(std::cmp::Ordering::Equal));
        Ok(targets)
    }
}
