//! `HeapArrivalOrder`: targets fill like a heap settling on the ground - from the bottom up,
//! climbing over the area (C# `HeapArrivalOrder`).

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::formations::{FormationArea, IArrivalOrder};
use crate::planes::{DrawingNoise, DrawingRandom};

const CELL: f32 = 3.0;

/// A heap building up from the ground (C# `HeapArrivalOrder(groundY)`, the same settings and
/// defaults): the key is the CLIMB from the ground (cells of 3 units at or below `ground_y` cost 0;
/// a step costs `sideways_cost` sideways, 1 up, `down_cost` down - the cheapest climb over the
/// area, Dijkstra on its cells), plus `edge_lag` for every unit closer than `edge_lag_reach` to the
/// edge, gradient-noise LUMPS of `lumpiness` over `lump_width`, and a seeded scatter of up to
/// `scatter`.
#[derive(Clone, Debug)]
pub struct HeapArrivalOrder {
    ground_y: f32,
    /// The cost of a sideways step, per unit (1.7).
    pub sideways_cost: f32,
    /// The cost of a downward step, per unit (2.4).
    pub down_cost: f32,
    /// The extra key per unit closer to the edge than the reach (0.6).
    pub edge_lag: f32,
    /// How far from the edge the lag reaches (24).
    pub edge_lag_reach: f32,
    /// The height of the lumps (15).
    pub lumpiness: f32,
    /// The width of the lumps (52).
    pub lump_width: f32,
    /// The scatter of the keys either way (6).
    pub scatter: f32,
    climb: Option<Vec<f32>>,
    left: f32,
    top: f32,
    columns: usize,
    rows: usize,
    random: Option<DrawingRandom>,
    noise_seed: i64,
}

#[derive(PartialEq)]
struct Queued(f32, usize);

impl Eq for Queued {}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Queued) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Queued {
    // A min-heap on the cost (the final climbs do not depend on the order of equal costs).
    fn cmp(&self, other: &Queued) -> Ordering {
        other.0.partial_cmp(&self.0).unwrap_or(Ordering::Equal).then(other.1.cmp(&self.1))
    }
}

impl HeapArrivalOrder {
    /// The order, with the ground at `ground_y`. The error says what to do with a non-finite
    /// ground.
    pub fn new(ground_y: f32) -> Result<HeapArrivalOrder, String> {
        if !ground_y.is_finite() {
            return Err("The ground's y must be a finite number.".to_string());
        }
        Ok(HeapArrivalOrder {
            ground_y,
            sideways_cost: 1.7,
            down_cost: 2.4,
            edge_lag: 0.6,
            edge_lag_reach: 24.0,
            lumpiness: 15.0,
            lump_width: 52.0,
            scatter: 6.0,
            climb: None,
            left: 0.0,
            top: 0.0,
            columns: 0,
            rows: 0,
            random: None,
            noise_seed: 0,
        })
    }

    /// The ground's y (C# `GroundY`).
    pub fn ground_y(&self) -> f32 {
        self.ground_y
    }
}

impl IArrivalOrder for HeapArrivalOrder {
    fn prepare(&mut self, area: &FormationArea, seed: i64) {
        self.random = Some(DrawingRandom::new(seed ^ 0x2545F4914F6CDD1D));
        self.noise_seed = seed;
        let b = area.bounds();
        self.left = b.x;
        self.top = b.y;
        self.columns = ((b.width / CELL) as f64).ceil() as usize + 1;
        self.rows = ((b.height / CELL) as f64).ceil() as usize + 1;
        let (columns, rows) = (self.columns, self.rows);
        let mut inside = vec![false; columns * rows];
        let mut climb = vec![f32::INFINITY; columns * rows];
        let mut queue = BinaryHeap::new();
        for y in 0..rows {
            for x in 0..columns {
                let i = (y * columns) + x;
                let px = self.left + ((x as f32 + 0.5) * CELL);
                let py = self.top + ((y as f32 + 0.5) * CELL);
                inside[i] = area.contains(px, py);
                if inside[i] && py >= self.ground_y - (CELL * 2.0) {
                    climb[i] = 0.0;
                    queue.push(Queued(0.0, i));
                }
            }
        }
        while let Some(Queued(c, i)) = queue.pop() {
            if c > climb[i] {
                continue;
            }
            let (x, y) = ((i % columns) as i64, (i / columns) as i64);
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= columns as i64 || ny >= rows as i64 {
                        continue;
                    }
                    let n = (ny as usize * columns) + nx as usize;
                    if !inside[n] {
                        continue;
                    }
                    let vertical = if dy < 0 { 1.0f32 } else if dy > 0 { self.down_cost } else { 0.0 };
                    let sideways = self.sideways_cost * dx as f32;
                    let step = CELL * ((sideways * sideways) + (vertical * vertical * dy as f32 * dy as f32)).sqrt();
                    if c + step < climb[n] {
                        climb[n] = c + step;
                        queue.push(Queued(climb[n], n));
                    }
                }
            }
        }
        self.climb = Some(climb);
    }

    /// The key (NaN - the target dropped - before `prepare`).
    fn get_key(&mut self, x: f32, y: f32, clearance: f32) -> f32 {
        let Some(climb) = self.climb.as_ref() else { return f32::NAN };
        let cx = (((x - self.left) / CELL) as i32).clamp(0, self.columns as i32 - 1) as usize;
        let cy = (((y - self.top) / CELL) as i32).clamp(0, self.rows as i32 - 1) as usize;
        let c = climb[(cy * self.columns) + cx];
        let lump = self.lumpiness as f64 * DrawingNoise::gradient((x / self.lump_width) as f64, 0.37, self.noise_seed);
        let scatter = self.random.as_mut().map_or(0.0, |r| r.uniform(-self.scatter as f64, self.scatter as f64));
        c + (self.edge_lag * (self.edge_lag_reach - clearance).max(0.0)) + lump as f32 + scatter as f32
    }
}
