//! The wave field's arithmetic on the CPU (Not in C#): the step's weights and damping, the
//! fixed-rate step count, the neighbour rule per edge and mask, the drop's Gaussian, the wake's
//! pressed path, the absorbing band, the picture (slopes, Laplacian, interpolation), the uniform
//! block, the CPU half every [`crate::WaveField`] shares (queue, wakes, rain, clock), and
//! [`CpuWaveField`], the whole field on the CPU. The SAME formulas as the shader
//! (`shaders/wgsl/wave_field.wgsl`) - keep the two in step.
//!
//! THE STEP, per cell (`h` now, `p` the step before, `L`/`R`/`U`/`D` the neighbours):
//! `next = level * h + kept' * (h - p) + wx * (L + R - 2h) + wy * (U + D - 2h)` with
//! `kept' = kept * (1 - shore * m) * (1 - sponge)`: a mask value `m` below solid (shallows) and an
//! absorbing band damp the VELOCITY further (damping the height itself would act as a spring that
//! reflects waves instead of swallowing them); solid cells write 0. `wx = (c dt / cell.x)^2`, `wy = (c dt / cell.y)^2`,
//! `kept = damping^(2 dt)` (the VELOCITY kept per step, so every wave's height falls to
//! `damping` in one second), `level = 1 - (1 - sqrt(kept))^2 / 4` (a slow, never oscillating
//! return of the mean level to 0). Stable while `wx + wy <= 0.5`.

use std::collections::VecDeque;

use codebrix_webgpu::{CanvasInput, PointerButton};
use glam::{UVec4, Vec2, Vec4};

use crate::context::GraphicsException;
use crate::water::wave_field::{WaveDrop, WaveEdge, WaveField, WaveFieldOptions, WaveStir, WaveWake};
use crate::water::wave_mask::WaveMask;
use crate::water::wave_rain::{RainState, WaveRain};

/// A mask value from which a cell counts as solid (land).
pub(crate) const SOLID: f32 = 0.99;
/// The largest `wx + wy`: half the leapfrog limit (low dispersion, ample margin).
pub(crate) const STABILITY_LIMIT: f32 = 0.5;
/// The most drops waiting (queued and rain); more are discarded.
pub(crate) const MAX_QUEUE: usize = 2 * WaveField::MAX_DROPS_PER_UPDATE;
/// The strength of an absorbing band: its outermost cell's per-step damping is this times the
/// cells a wave crosses per step, divided by the band's width (so the band works the same at any
/// speed and rate).
pub(crate) const SPONGE_GAIN: f32 = 6.0;
/// The band width whose outermost damping a mask value of 1 (just below solid) gives shallows.
pub(crate) const SHORE_WIDTH: f32 = 12.0;
/// A Gaussian is left out beyond this many radii (exp(-16): nothing).
pub(crate) const CUTOFF_RADII: f32 = 4.0;

/// The per-step numbers of a field's step, from its options (Not in C#).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Coefficients {
    /// The step in seconds.
    pub(crate) dt: f32,
    /// The cell size in field units.
    pub(crate) cell: Vec2,
    /// `(c dt / cell.x)^2`.
    pub(crate) wx: f32,
    /// `(c dt / cell.y)^2`.
    pub(crate) wy: f32,
    /// The velocity kept per step.
    pub(crate) kept: f32,
    /// The mean level kept per step.
    pub(crate) level: f32,
    /// The outermost per-step damping of an absorbing band.
    pub(crate) sponge: f32,
    /// The per-step damping of a mask value of 1.
    pub(crate) shore: f32,
    /// 0 reflect, 1 absorb, 2 wrap.
    pub(crate) edge: u32,
    /// The absorbing band's width in cells (0 unless absorbing).
    pub(crate) band: u32,
}

/// The step's numbers for `options` (assumed checked).
pub(crate) fn coefficients(options: &WaveFieldOptions) -> Coefficients {
    let dt = 1.0 / options.steps_per_second;
    let cell = cell_size(options);
    let wx = (options.wave_speed * dt / cell.x).powi(2);
    let wy = (options.wave_speed * dt / cell.y).powi(2);
    let kept = options.damping.clamp(0.0, 1.0).powf(2.0 * dt);
    let level = 1.0 - 0.25 * (1.0 - kept.sqrt()).powi(2);
    let courant = options.wave_speed * dt / cell.x.min(cell.y);
    let (edge, band) = match options.edge {
        WaveEdge::Reflect => (0, 0),
        WaveEdge::Absorb { width } => (1, band_width(width, options.columns, options.rows)),
        WaveEdge::Wrap => (2, 0),
    };
    let sponge = if band > 0 { (SPONGE_GAIN * courant / band as f32).min(0.5) } else { 0.0 };
    let shore = (SPONGE_GAIN * courant / SHORE_WIDTH).min(0.5);
    Coefficients { dt, cell, wx, wy, kept, level, sponge, shore, edge, band }
}

/// An absorbing band's width kept between 1 and half the smaller side.
pub(crate) fn band_width(width: u32, columns: u32, rows: u32) -> u32 {
    width.clamp(1, (columns.min(rows) / 2).max(1))
}

/// The size of one cell.
pub(crate) fn cell_size(options: &WaveFieldOptions) -> Vec2 {
    Vec2::new(options.size.x / options.columns as f32, options.size.y / options.rows as f32)
}

/// Checks every option; Err says what to change.
pub(crate) fn check_options(options: &WaveFieldOptions) -> Result<(), GraphicsException> {
    let (low, high) = WaveField::GRID_RANGE;
    if !(low..=high).contains(&options.columns) || !(low..=high).contains(&options.rows) {
        return Err(GraphicsException::new(format!("A wave field has {low} to {high} cells on a side; WaveFieldOptions asks for {} x {}.", options.columns, options.rows)));
    }
    if !(options.size.x > 0.0 && options.size.y > 0.0 && options.size.is_finite()) {
        return Err(GraphicsException::new(format!("WaveFieldOptions::size must be positive on both axes; it is {}.", options.size)));
    }
    if !(1.0..=2000.0).contains(&options.steps_per_second) {
        return Err(GraphicsException::new(format!("WaveFieldOptions::steps_per_second must be 1 to 2000; it is {}.", options.steps_per_second)));
    }
    if !(1..=64).contains(&options.max_steps_per_frame) {
        return Err(GraphicsException::new(format!("WaveFieldOptions::max_steps_per_frame must be 1 to 64; it is {}.", options.max_steps_per_frame)));
    }
    check_damping(options.damping)?;
    check_speed(options.wave_speed, options.steps_per_second, cell_size(options))
}

/// Checks a damping (0 to 1).
pub(crate) fn check_damping(damping: f32) -> Result<(), GraphicsException> {
    if !(0.0..=1.0).contains(&damping) {
        return Err(GraphicsException::new(format!("A wave field's damping is the fraction of a wave's height left after one second, 0 to 1 (1: none); it is {damping}.")));
    }
    Ok(())
}

/// THE STABILITY RULE `wx + wy <= 0.5`: Err names the highest speed the rate allows and the
/// lowest rate that holds the speed.
pub(crate) fn check_speed(speed: f32, steps_per_second: f32, cell: Vec2) -> Result<(), GraphicsException> {
    if !(speed >= 0.0 && speed.is_finite()) {
        return Err(GraphicsException::new(format!("A wave speed must be 0 or more units per second; it is {speed}.")));
    }
    let inverse = 1.0 / (cell.x * cell.x) + 1.0 / (cell.y * cell.y);
    let dt = 1.0 / steps_per_second;
    if speed * speed * dt * dt * inverse > STABILITY_LIMIT {
        let fastest = steps_per_second * (STABILITY_LIMIT / inverse).sqrt();
        let lowest = (speed * (inverse / STABILITY_LIMIT).sqrt()).ceil();
        return Err(GraphicsException::new(format!(
            "A wave speed of {speed} units per second is too fast for {steps_per_second} steps per second on cells of {} x {} units: at this rate waves may move at most {fastest:.3} units per second. Raise WaveFieldOptions::steps_per_second to at least {lowest}, lower the speed, or use larger cells.",
            cell.x, cell.y
        )));
    }
    Ok(())
}

/// THE FIXED RATE: the steps an update runs for `accumulated` clock seconds (the carried time
/// plus this frame's delta) - at most `max` - and the seconds carried to the next update (less
/// than one step; the whole steps beyond the cap are dropped).
pub(crate) fn step_count(accumulated: f64, steps_per_second: f32, max: u32) -> (u32, f64) {
    let rate = steps_per_second as f64;
    let whole = (accumulated.max(0.0) * rate + 1e-9).floor();
    let rest = (accumulated - whole / rate).max(0.0);
    (whole.min(max as f64) as u32, rest)
}

/// The error function (Abramowitz and Stegun 7.1.26; error below 1.5e-7) - the shader's `erf`.
pub(crate) fn erf(x: f32) -> f32 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let poly = t * (0.254_829_6 + t * (-0.284_496_74 + t * (1.421_413_7 + t * (-1.453_152 + t * 1.061_405_4))));
    sign * (1.0 - poly * (-x * x).exp())
}

/// The offset from `center` to `point`, through the nearest repeat of a wrapping field.
pub(crate) fn offset(point: Vec2, center: Vec2, size: Vec2, wrap: bool) -> Vec2 {
    let d = point - center;
    if wrap {
        d - (d / size + 0.5).floor() * size
    } else {
        d
    }
}

/// A drop's Gaussian at offset `d`: `exp(-(|d| / radius)^2)` (0 beyond four radii).
pub(crate) fn gaussian(d: Vec2, radius: f32) -> f32 {
    let r2 = radius * radius;
    let d2 = d.length_squared();
    if r2 <= 0.0 || d2 >= CUTOFF_RADII * CUTOFF_RADII * r2 {
        0.0
    } else {
        (-d2 / r2).exp()
    }
}

/// A wake's pressed path at `p` for the stretch from `a` to `b`: the Gaussian of `radius` swept
/// along the stretch and divided by its line integral (`radius * sqrt(pi)`), so a long straight
/// path presses exactly 1 along its middle and consecutive stretches add up to the whole path.
pub(crate) fn path_profile(p: Vec2, a: Vec2, b: Vec2, radius: f32) -> f32 {
    let length = (b - a).length();
    if length < 1e-6 || radius <= 0.0 {
        return 0.0;
    }
    let along = (b - a) / length;
    let w = p - a;
    let t = w.dot(along);
    let d2 = (w.length_squared() - t * t).max(0.0);
    let r2 = radius * radius;
    if d2 >= CUTOFF_RADII * CUTOFF_RADII * r2 {
        return 0.0;
    }
    0.5 * (-d2 / r2).exp() * (erf((length - t) / radius) + erf(t / radius))
}

/// The per-step damping of an absorbing band at cell (i, j): `sponge * ((band - d) / band)^2`
/// for a cell `d` cells from the nearest edge, 0 beyond the band.
pub(crate) fn sponge_at(i: u32, j: u32, columns: u32, rows: u32, c: &Coefficients) -> f32 {
    if c.band == 0 {
        return 0.0;
    }
    let d = i.min(j).min(columns - 1 - i).min(rows - 1 - j);
    if d >= c.band {
        return 0.0;
    }
    let x = (c.band - d) as f32 / c.band as f32;
    c.sponge * x * x
}

/// THE NEIGHBOUR RULE: the value of cell (i, j) as a neighbour of a cell whose value is
/// `center` - outside the grid: the cell itself (Reflect, Absorb) or the opposite edge (Wrap); a
/// solid neighbour: the cell itself (so a mask reflects exactly like a wall).
#[allow(clippy::too_many_arguments)]
pub(crate) fn neighbour(value: &dyn Fn(usize) -> f32, mask: Option<&[f32]>, columns: u32, rows: u32, wrap: bool, i: i64, j: i64, center: f32) -> f32 {
    let (columns, rows) = (columns as i64, rows as i64);
    let (i, j) = if wrap {
        (i.rem_euclid(columns), j.rem_euclid(rows))
    } else if i < 0 || j < 0 || i >= columns || j >= rows {
        return center;
    } else {
        (i, j)
    };
    let index = (j * columns + i) as usize;
    if mask.is_some_and(|m| m[index] >= SOLID) {
        return center;
    }
    value(index)
}

/// The next height of one cell (see the module's formula).
#[allow(clippy::too_many_arguments)]
pub(crate) fn step_cell(h: f32, p: f32, left: f32, right: f32, up: f32, down: f32, mask_value: f32, sponge: f32, c: &Coefficients) -> f32 {
    if mask_value >= SOLID {
        return 0.0;
    }
    let kept = c.kept * (1.0 - c.shore * mask_value.max(0.0)) * (1.0 - sponge);
    c.level * h + kept * (h - p) + c.wx * (left + right - 2.0 * h) + c.wy * (up + down - 2.0 * h)
}

/// One step of the whole grid: the next state OVERWRITES `previous` (as on the GPU), `current`
/// is only read.
pub(crate) fn step_grid(current: &[f32], previous: &mut [f32], mask: Option<&[f32]>, columns: u32, rows: u32, c: &Coefficients) {
    let wrap = c.edge == 2;
    let value = |index: usize| current[index];
    for j in 0..rows {
        for i in 0..columns {
            let index = (j * columns + i) as usize;
            let h = current[index];
            let (x, y) = (i as i64, j as i64);
            let left = neighbour(&value, mask, columns, rows, wrap, x - 1, y, h);
            let right = neighbour(&value, mask, columns, rows, wrap, x + 1, y, h);
            let up = neighbour(&value, mask, columns, rows, wrap, x, y - 1, h);
            let down = neighbour(&value, mask, columns, rows, wrap, x, y + 1, h);
            let m = mask.map_or(0.0, |m| m[index]);
            previous[index] = step_cell(h, previous[index], left, right, up, down, m, sponge_at(i, j, columns, rows, c), c);
        }
    }
}

/// THE PICTURE of the whole grid into `out`: per texel the height interpolated between the two
/// states (`previous + (current - previous) * fraction`), central-difference slopes and the
/// five-point Laplacian of the interpolated heights, by the neighbour rule; solid cells 0.
#[allow(clippy::too_many_arguments)]
pub(crate) fn publish_grid(current: &[f32], previous: &[f32], mask: Option<&[f32]>, columns: u32, rows: u32, c: &Coefficients, fraction: f32, out: &mut [[f32; 4]]) {
    let wrap = c.edge == 2;
    let value = |index: usize| previous[index] + (current[index] - previous[index]) * fraction;
    for j in 0..rows {
        for i in 0..columns {
            let index = (j * columns + i) as usize;
            if mask.is_some_and(|m| m[index] >= SOLID) {
                out[index] = [0.0; 4];
                continue;
            }
            let h = value(index);
            let (x, y) = (i as i64, j as i64);
            let left = neighbour(&value, mask, columns, rows, wrap, x - 1, y, h);
            let right = neighbour(&value, mask, columns, rows, wrap, x + 1, y, h);
            let up = neighbour(&value, mask, columns, rows, wrap, x, y - 1, h);
            let down = neighbour(&value, mask, columns, rows, wrap, x, y + 1, h);
            out[index] = texel(h, left, right, up, down, c.cell);
        }
    }
}

/// One texel of the picture: height, dh/dx, dh/dy, Laplacian.
pub(crate) fn texel(h: f32, left: f32, right: f32, up: f32, down: f32, cell: Vec2) -> [f32; 4] {
    let dx = (right - left) / (2.0 * cell.x);
    let dy = (down - up) / (2.0 * cell.y);
    let laplacian = (left + right - 2.0 * h) / (cell.x * cell.x) + (up + down - 2.0 * h) / (cell.y * cell.y);
    [h, dx, dy, laplacian]
}

/// The height a disturbance adds at point `p` (negative presses down): every drop
/// (`x, y, radius, depth`) and every wake stretch (`a.xy, b.xy` / `radius, strength, bob depth, 0`).
pub(crate) fn disturbance_at(p: Vec2, size: Vec2, wrap: bool, drops: &[Vec4], wakes: &[[Vec4; 2]]) -> f32 {
    let mut add = 0.0;
    for d in drops {
        add -= d.w * gaussian(offset(p, Vec2::new(d.x, d.y), size, wrap), d.z);
    }
    for [ends, look] in wakes {
        let (a, b) = (Vec2::new(ends.x, ends.y), Vec2::new(ends.z, ends.w));
        let shift = offset(p, b, size, wrap) - (p - b);
        add -= look.y * path_profile(p + shift, a, b, look.x);
        add -= look.z * gaussian(offset(p, b, size, wrap), look.x);
    }
    add
}

/// Adds the disturbances to BOTH states (pressed in at rest: no velocity of their own); solid
/// cells are left alone.
#[allow(clippy::too_many_arguments)]
pub(crate) fn disturb_grid(current: &mut [f32], previous: &mut [f32], mask: Option<&[f32]>, columns: u32, rows: u32, c: &Coefficients, size: Vec2, drops: &[Vec4], wakes: &[[Vec4; 2]]) {
    let wrap = c.edge == 2;
    for j in 0..rows {
        for i in 0..columns {
            let index = (j * columns + i) as usize;
            if mask.is_some_and(|m| m[index] >= SOLID) {
                continue;
            }
            let p = Vec2::new((i as f32 + 0.5) * c.cell.x, (j as f32 + 0.5) * c.cell.y);
            let add = disturbance_at(p, size, wrap, drops, wakes);
            current[index] += add;
            previous[index] += add;
        }
    }
}

/// The value of half-precision bits (the picture as read back from the GPU).
pub(crate) fn half_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = ((bits >> 10) & 0x1f) as i32;
    let mantissa = (bits & 0x3ff) as f32;
    match exponent {
        0 => sign * mantissa * 2f32.powi(-24),
        31 => {
            if mantissa == 0.0 {
                sign * f32::INFINITY
            } else {
                f32::NAN
            }
        }
        _ => sign * (1.0 + mantissa / 1024.0) * 2f32.powi(exponent - 15),
    }
}

/// What one update does: the steps, the interpolation fraction, the disturbances.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct UpdatePlan {
    /// The steps to run (0: only a new picture, for the interpolation).
    pub(crate) steps: u32,
    /// The time carried over, as a fraction of a step (0..1).
    pub(crate) fraction: f32,
    /// The drops: x, y, radius, depth.
    pub(crate) drops: Vec<Vec4>,
    /// The wake stretches: a.xy, b.xy / radius, strength, bob depth, 0.
    pub(crate) wakes: Vec<[Vec4; 2]>,
}

impl UpdatePlan {
    /// True when nothing is pressed this update.
    pub(crate) fn is_calm(&self) -> bool {
        self.drops.is_empty() && self.wakes.is_empty()
    }
}

/// The shader's uniform block (848 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WaveParams {
    /// Columns, rows, edge (0 reflect, 1 absorb, 2 wrap), flags (1: a mask).
    pub(crate) grid: UVec4,
    /// Drops, wake stretches, the absorbing band's width, 0.
    pub(crate) counts: UVec4,
    /// wx, wy, velocity kept per step, level kept per step.
    pub(crate) weights: Vec4,
    /// Cell x, cell y, the interpolation fraction, the band's outermost damping.
    pub(crate) cells: Vec4,
    /// The shallows' damping, size x, size y, 0.
    pub(crate) extra: Vec4,
    /// The drops: x, y, radius, depth.
    pub(crate) drops: [Vec4; WaveField::MAX_DROPS_PER_UPDATE],
    /// Two records per wake stretch: a.xy, b.xy / radius, strength, bob depth, 0.
    pub(crate) wakes: [Vec4; 2 * WaveField::MAX_WAKES],
}

/// The size of [`WaveParams`].
pub(crate) const PARAMS_SIZE: u64 = 848;
const _: () = assert!(std::mem::size_of::<WaveParams>() == PARAMS_SIZE as usize);
// SAFETY: #[repr(C)]; UVec4 and Vec4 fields and arrays of Vec4 (16 bytes each, 4-byte aligned),
// no padding (size asserted above); any bits are valid.
unsafe impl bytemuck::Zeroable for WaveParams {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for WaveParams {}

/// One wake slot: the wake and where its path was last pressed to.
#[derive(Clone, Copy, Debug, PartialEq)]
struct WakeSlot {
    wake: WaveWake,
    pressed: Vec2,
}

/// THE CPU HALF of a wave field, shared by [`crate::WaveField`] and [`CpuWaveField`]: the
/// options and their numbers, the clock's accumulator, the drop queue, the wakes, the rain, the
/// pointer's stirring and the counters.
#[derive(Clone, Debug)]
pub(crate) struct WaveCore {
    options: WaveFieldOptions,
    coefficients: Coefficients,
    mask: Option<Vec<f32>>,
    accumulated: f64,
    queue: VecDeque<WaveDrop>,
    wakes: [Option<WakeSlot>; WaveField::MAX_WAKES],
    rain: Option<WaveRain>,
    rain_state: RainState,
    simulated: f64,
    steps: u64,
    last_steps: u32,
    stirring: Option<i32>,
}

impl WaveCore {
    /// The CPU half of a calm field; Err for options out of range.
    pub(crate) fn new(options: &WaveFieldOptions) -> Result<WaveCore, GraphicsException> {
        check_options(options)?;
        let mut options = options.clone();
        if let WaveEdge::Absorb { width } = options.edge {
            options.edge = WaveEdge::Absorb { width: band_width(width, options.columns, options.rows) };
        }
        Ok(WaveCore {
            coefficients: coefficients(&options),
            options,
            mask: None,
            accumulated: 0.0,
            queue: VecDeque::new(),
            wakes: [None; WaveField::MAX_WAKES],
            rain: None,
            rain_state: RainState::default(),
            simulated: 0.0,
            steps: 0,
            last_steps: 0,
            stirring: None,
        })
    }

    pub(crate) fn options(&self) -> &WaveFieldOptions {
        &self.options
    }

    pub(crate) fn coefficients(&self) -> &Coefficients {
        &self.coefficients
    }

    pub(crate) fn cell_size(&self) -> Vec2 {
        self.coefficients.cell
    }

    pub(crate) fn set_wave_speed(&mut self, speed: f32) -> Result<(), GraphicsException> {
        check_speed(speed, self.options.steps_per_second, self.coefficients.cell)?;
        self.options.wave_speed = speed;
        self.coefficients = coefficients(&self.options);
        Ok(())
    }

    pub(crate) fn set_damping(&mut self, damping: f32) -> Result<(), GraphicsException> {
        check_damping(damping)?;
        self.options.damping = damping;
        self.coefficients = coefficients(&self.options);
        Ok(())
    }

    pub(crate) fn set_edge(&mut self, edge: WaveEdge) {
        self.options.edge = match edge {
            WaveEdge::Absorb { width } => WaveEdge::Absorb { width: band_width(width, self.options.columns, self.options.rows) },
            other => other,
        };
        self.coefficients = coefficients(&self.options);
    }

    pub(crate) fn set_mask(&mut self, mask: Option<&WaveMask>) -> Result<(), GraphicsException> {
        if let Some(mask) = mask {
            if (mask.columns(), mask.rows()) != (self.options.columns, self.options.rows) {
                return Err(GraphicsException::new(format!(
                    "The mask is {} x {} cells but the wave field is {} x {}: make the mask for the field's grid (WaveMask::new, from_fn and from_picture take the size).",
                    mask.columns(),
                    mask.rows(),
                    self.options.columns,
                    self.options.rows
                )));
            }
        }
        self.mask = mask.map(|mask| mask.cells().to_vec());
        Ok(())
    }

    pub(crate) fn drop(&mut self, drop: WaveDrop) {
        if self.queue.len() < MAX_QUEUE {
            self.queue.push_back(drop);
        }
    }

    pub(crate) fn set_wake(&mut self, slot: usize, wake: Option<WaveWake>) -> Result<(), GraphicsException> {
        let Some(entry) = self.wakes.get_mut(slot) else {
            return Err(GraphicsException::new(format!("A wave field has wake slots 0 to {}; slot {slot} does not exist.", WaveField::MAX_WAKES - 1)));
        };
        *entry = wake.map(|wake| WakeSlot { wake, pressed: entry.map_or(wake.position, |old| old.pressed) });
        Ok(())
    }

    pub(crate) fn set_rain(&mut self, rain: Option<WaveRain>) {
        self.rain = rain;
    }

    pub(crate) fn rain(&self) -> Option<&WaveRain> {
        self.rain.as_ref()
    }

    pub(crate) fn stir(&mut self, input: &CanvasInput, stir: &WaveStir, to_field: &dyn Fn(f64, f64) -> Option<Vec2>) {
        let slot = WaveField::MAX_WAKES - 1;
        match input {
            CanvasInput::PointerDown { x, y, pointer_id, button: PointerButton::Primary, .. } if self.stirring.is_none() => {
                self.stirring = Some(*pointer_id);
                self.wakes[slot] = None;
                if let Some(point) = to_field(*x, *y) {
                    self.drop(WaveDrop { position: point, ..stir.drop });
                    let _ = self.set_wake(slot, Some(WaveWake { position: point, ..stir.wake }));
                }
            }
            CanvasInput::PointerMove { x, y, pointer_id, buttons, .. } if self.stirring == Some(*pointer_id) => {
                if !buttons.primary() {
                    self.stirring = None;
                    self.wakes[slot] = None;
                    return;
                }
                let wake = to_field(*x, *y).map(|point| WaveWake { position: point, ..stir.wake });
                let _ = self.set_wake(slot, wake);
            }
            CanvasInput::PointerUp { pointer_id, .. } if self.stirring == Some(*pointer_id) => {
                self.stirring = None;
                self.wakes[slot] = None;
            }
            _ => {}
        }
    }

    /// An update with a delta of 0: nothing happens but the read-out of the last step count.
    pub(crate) fn idle(&mut self) {
        self.last_steps = 0;
    }

    /// The plan of an update of `delta` clock seconds (None for a delta of 0 or less): the fixed
    /// rate's steps, the carried fraction and - when it steps - the rain's new drops, the queued
    /// drops (at most MAX_DROPS_PER_UPDATE) and each wake's stretch since it was last pressed.
    pub(crate) fn plan(&mut self, delta: f64) -> Option<UpdatePlan> {
        if delta.is_nan() || delta <= 0.0 {
            self.idle();
            return None;
        }
        let (steps, rest) = step_count(self.accumulated + delta, self.options.steps_per_second, self.options.max_steps_per_frame);
        self.accumulated = rest;
        let fraction = (rest * self.options.steps_per_second as f64).clamp(0.0, 1.0) as f32;
        self.last_steps = steps;
        if steps == 0 {
            return Some(UpdatePlan { steps, fraction, ..UpdatePlan::default() });
        }
        let seconds = steps as f64 / self.options.steps_per_second as f64;
        if let Some(rain) = self.rain {
            for drop in self.rain_state.take(&rain, seconds, self.options.size) {
                self.drop(drop);
            }
        }
        let count = self.queue.len().min(WaveField::MAX_DROPS_PER_UPDATE);
        let drops = self.queue.drain(..count).map(|d| Vec4::new(d.position.x, d.position.y, d.radius.max(0.0), d.depth)).collect();
        let (t0, t1) = (self.simulated, self.simulated + seconds);
        let mut wakes = Vec::new();
        for slot in self.wakes.iter_mut().flatten() {
            let w = slot.wake;
            let turn = std::f64::consts::TAU * w.bob_rate as f64;
            let bob = -(w.bob as f64 * ((turn * t1).sin() - (turn * t0).sin())) as f32;
            if slot.pressed != w.position || bob != 0.0 {
                wakes.push([Vec4::new(slot.pressed.x, slot.pressed.y, w.position.x, w.position.y), Vec4::new(w.radius.max(0.0), w.strength, bob, 0.0)]);
            }
            slot.pressed = w.position;
        }
        self.simulated = t1;
        self.steps += steps as u64;
        Some(UpdatePlan { steps, fraction, drops, wakes })
    }

    /// The uniform block for a plan.
    pub(crate) fn params(&self, plan: &UpdatePlan) -> WaveParams {
        let c = &self.coefficients;
        let mut drops = [Vec4::ZERO; WaveField::MAX_DROPS_PER_UPDATE];
        for (slot, drop) in drops.iter_mut().zip(&plan.drops) {
            *slot = *drop;
        }
        let mut wakes = [Vec4::ZERO; 2 * WaveField::MAX_WAKES];
        for (i, [ends, look]) in plan.wakes.iter().take(WaveField::MAX_WAKES).enumerate() {
            wakes[2 * i] = *ends;
            wakes[2 * i + 1] = *look;
        }
        WaveParams {
            grid: UVec4::new(self.options.columns, self.options.rows, c.edge, self.mask.is_some() as u32),
            counts: UVec4::new(plan.drops.len().min(WaveField::MAX_DROPS_PER_UPDATE) as u32, plan.wakes.len().min(WaveField::MAX_WAKES) as u32, c.band, 0),
            weights: Vec4::new(c.wx, c.wy, c.kept, c.level),
            cells: Vec4::new(c.cell.x, c.cell.y, plan.fraction, c.sponge),
            extra: Vec4::new(c.shore, self.options.size.x, self.options.size.y, 0.0),
            drops,
            wakes,
        }
    }

    /// Calm water: the queue, the clock and the rain start over; wakes press nothing from where
    /// they were.
    pub(crate) fn reset(&mut self) {
        self.accumulated = 0.0;
        self.queue.clear();
        self.rain_state = RainState::default();
        self.simulated = 0.0;
        self.steps = 0;
        self.last_steps = 0;
        for slot in self.wakes.iter_mut().flatten() {
            slot.pressed = slot.wake.position;
        }
    }

    pub(crate) fn simulated_time(&self) -> f64 {
        self.simulated
    }

    pub(crate) fn steps_taken(&self) -> u64 {
        self.steps
    }

    pub(crate) fn last_step_count(&self) -> u32 {
        self.last_steps
    }
}

/// THE WAVE FIELD ON THE CPU (Not in C#): the same arithmetic as [`crate::WaveField`], step for
/// step, with no GPU - for small grids (it is slow: a 256 x 256 field at 120 steps per second
/// costs several milliseconds a frame), for tests, and for checking a GPU run: drive both with the
/// same calls and deltas, read the GPU picture back (`surface().read_pixels(0)`) and compare with
/// [`CpuWaveField::height_difference`]. Its picture is [`CpuWaveField::surface`]: the same four
/// values per cell, in f32.
#[derive(Clone, Debug)]
pub struct CpuWaveField {
    core: WaveCore,
    current: Vec<f32>,
    previous: Vec<f32>,
    surface: Vec<[f32; 4]>,
}

impl CpuWaveField {
    /// A calm field; Err as [`crate::WaveField::new`].
    pub fn new(options: &WaveFieldOptions) -> Result<CpuWaveField, GraphicsException> {
        let core = WaveCore::new(options)?;
        let cells = options.columns as usize * options.rows as usize;
        Ok(CpuWaveField { core, current: vec![0.0; cells], previous: vec![0.0; cells], surface: vec![[0.0; 4]; cells] })
    }

    /// The options as they are now.
    pub fn options(&self) -> &WaveFieldOptions {
        self.core.options()
    }

    /// The field's extent in field units.
    pub fn size(&self) -> Vec2 {
        self.core.options().size
    }

    /// The size of one cell in field units.
    pub fn cell_size(&self) -> Vec2 {
        self.core.cell_size()
    }

    /// As [`crate::WaveField::set_wave_speed`].
    pub fn set_wave_speed(&mut self, units_per_second: f32) -> Result<(), GraphicsException> {
        self.core.set_wave_speed(units_per_second)
    }

    /// As [`crate::WaveField::set_damping`].
    pub fn set_damping(&mut self, kept_per_second: f32) -> Result<(), GraphicsException> {
        self.core.set_damping(kept_per_second)
    }

    /// As [`crate::WaveField::set_edge`].
    pub fn set_edge(&mut self, edge: WaveEdge) {
        self.core.set_edge(edge);
    }

    /// As [`crate::WaveField::set_mask`].
    pub fn set_mask(&mut self, mask: Option<&WaveMask>) -> Result<(), GraphicsException> {
        self.core.set_mask(mask)
    }

    /// As [`crate::WaveField::drop`].
    pub fn drop(&mut self, drop: WaveDrop) {
        self.core.drop(drop);
    }

    /// As [`crate::WaveField::set_wake`].
    pub fn set_wake(&mut self, slot: usize, wake: Option<WaveWake>) -> Result<(), GraphicsException> {
        self.core.set_wake(slot, wake)
    }

    /// As [`crate::WaveField::set_rain`].
    pub fn set_rain(&mut self, rain: Option<WaveRain>) {
        self.core.set_rain(rain);
    }

    /// As [`crate::WaveField::stir`].
    pub fn stir(&mut self, input: &CanvasInput, stir: &WaveStir, to_field: &dyn Fn(f64, f64) -> Option<Vec2>) {
        self.core.stir(input, stir, to_field);
    }

    /// As [`crate::WaveField::update`], on the CPU.
    pub fn update(&mut self, delta: f64) {
        let Some(plan) = self.core.plan(delta) else { return };
        let (columns, rows) = (self.core.options().columns, self.core.options().rows);
        let c = *self.core.coefficients();
        let size = self.core.options().size;
        let mask = self.core.mask.as_deref();
        if !plan.is_calm() {
            disturb_grid(&mut self.current, &mut self.previous, mask, columns, rows, &c, size, &plan.drops, &plan.wakes);
        }
        for _ in 0..plan.steps {
            step_grid(&self.current, &mut self.previous, mask, columns, rows, &c);
            std::mem::swap(&mut self.current, &mut self.previous);
        }
        publish_grid(&self.current, &self.previous, mask, columns, rows, &c, plan.fraction, &mut self.surface);
    }

    /// As [`crate::WaveField::reset`].
    pub fn reset(&mut self) {
        self.core.reset();
        self.current.fill(0.0);
        self.previous.fill(0.0);
        self.surface.fill([0.0; 4]);
    }

    /// The picture: per cell, row by row from row 0, height, dh/dx, dh/dy and the Laplacian (as
    /// [`crate::WaveField::surface`]'s texels).
    pub fn surface(&self) -> &[[f32; 4]] {
        &self.surface
    }

    /// The heights of the latest step, row by row (not interpolated).
    pub fn heights(&self) -> &[f32] {
        &self.current
    }

    /// As [`crate::WaveField::simulated_time`].
    pub fn simulated_time(&self) -> f64 {
        self.core.simulated_time()
    }

    /// As [`crate::WaveField::steps_taken`].
    pub fn steps_taken(&self) -> u64 {
        self.core.steps_taken()
    }

    /// As [`crate::WaveField::last_step_count`].
    pub fn last_step_count(&self) -> u32 {
        self.core.last_step_count()
    }

    /// The largest difference of the HEIGHTS between this field's picture and a GPU field's
    /// picture read back as bytes (`WaveField::surface().read_pixels(0)`: `Rgba16Float`, eight
    /// bytes per texel, tightly packed rows). Err when the bytes are not a picture of this grid.
    /// The GPU stores halves, so expect differences of about a thousandth of the heights.
    pub fn height_difference(&self, gpu_picture: &[u8]) -> Result<f32, GraphicsException> {
        if gpu_picture.len() != self.surface.len() * 8 {
            return Err(GraphicsException::new(format!("A picture of {} x {} texels of Rgba16Float has {} bytes; got {}.", self.core.options().columns, self.core.options().rows, self.surface.len() * 8, gpu_picture.len())));
        }
        let mut largest = 0.0f32;
        for (k, texel) in self.surface.iter().enumerate() {
            let gpu = half_to_f32(u16::from_le_bytes([gpu_picture[8 * k], gpu_picture[8 * k + 1]]));
            largest = largest.max((gpu - texel[0]).abs());
        }
        Ok(largest)
    }
}
