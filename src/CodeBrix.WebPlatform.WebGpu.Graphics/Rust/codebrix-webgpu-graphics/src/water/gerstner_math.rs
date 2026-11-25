//! The open-water waves' arithmetic on the CPU (Not in C#): each wave's numbers, its CURRENT
//! PHASE in f64, the Gerstner (trochoidal) displacement of a still-water point, the tangents and
//! the analytic normal, the Jacobian of the horizontal motion (the loop check) and the inversion
//! that finds the surface above a horizontal point. A shader that draws the waves (the water
//! surface's) uses the same formulas on the records of `OpenWaterWaves::records` - keep them in
//! step.
//!
//! For a wave with unit direction `D` (in x, z), wavenumber `k = 2 pi / wavelength`, amplitude
//! `A`, horizontal radius `H = steepness * A` and current phase `P = (omega t + phase) mod 2 pi`
//! (`omega = speed * k`; deep water `speed = sqrt(g / k)`), the still-water point `(x, z)` has
//! the argument `a = k (D . (x, z)) - P` and moves by
//! `(-H D.x sin a, A cos a, -H D.z sin a)`, summed over the waves (a crest where `a = 0`, moving
//! along `D`).

use glam::{Vec2, Vec3};

use crate::water::open_water_waves::GerstnerWave;

/// One wave's numbers as the formulas use them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedWave {
    /// The unit direction of travel in (x, z).
    pub(crate) direction: Vec2,
    /// The wavenumber `2 pi / wavelength`.
    pub(crate) k: f32,
    /// The crest height.
    pub(crate) amplitude: f32,
    /// The horizontal radius `steepness * amplitude`.
    pub(crate) horizontal: f32,
    /// The angular frequency (radians per second), in f64.
    pub(crate) omega: f64,
    /// The phase at time 0, radians.
    pub(crate) phase: f32,
    /// Crest to crest.
    pub(crate) wavelength: f32,
}

/// The numbers of `wave` under `gravity` (scene units per second squared).
pub(crate) fn resolve(wave: &GerstnerWave, gravity: f32) -> ResolvedWave {
    let k = std::f64::consts::TAU / wave.wavelength as f64;
    let speed = match wave.speed {
        Some(speed) => speed as f64,
        None => (gravity as f64 / k).sqrt(),
    };
    ResolvedWave {
        direction: Vec2::new(wave.direction.cos(), wave.direction.sin()),
        k: k as f32,
        amplitude: wave.amplitude,
        horizontal: wave.steepness * wave.amplitude,
        omega: speed * k,
        phase: wave.phase,
        wavelength: wave.wavelength,
    }
}

/// The wave's CURRENT PHASE `(omega t + phase) mod 2 pi`, in f64 (exact for hours of page time).
pub(crate) fn current_phase(wave: &ResolvedWave, seconds: f64) -> f64 {
    (wave.omega * seconds + wave.phase as f64).rem_euclid(std::f64::consts::TAU)
}

/// Every wave's current phase.
pub(crate) fn phases(waves: &[ResolvedWave], seconds: f64) -> Vec<f32> {
    waves.iter().map(|w| current_phase(w, seconds) as f32).collect()
}

/// The argument `k (D . p) - P`.
fn argument(wave: &ResolvedWave, phase: f32, p: Vec2) -> f32 {
    wave.k * wave.direction.dot(p) - phase
}

/// How far the still-water point `p = (x, z)` has moved: (x, height, z).
pub(crate) fn displacement(waves: &[ResolvedWave], phases: &[f32], p: Vec2) -> Vec3 {
    let mut d = Vec3::ZERO;
    for (w, &phase) in waves.iter().zip(phases) {
        let (s, c) = argument(w, phase, p).sin_cos();
        d += Vec3::new(-w.horizontal * w.direction.x * s, w.amplitude * c, -w.horizontal * w.direction.y * s);
    }
    d
}

/// The surface's tangents at the still-water point `p`: the derivatives of the moved point along
/// x and along z.
pub(crate) fn tangents(waves: &[ResolvedWave], phases: &[f32], p: Vec2) -> (Vec3, Vec3) {
    let (mut tx, mut tz) = (Vec3::X, Vec3::Z);
    for (w, &phase) in waves.iter().zip(phases) {
        let (s, c) = argument(w, phase, p).sin_cos();
        let (dx, dz) = (w.direction.x, w.direction.y);
        let hk = w.horizontal * w.k * c;
        let ak = w.amplitude * w.k * s;
        tx += Vec3::new(-hk * dx * dx, -ak * dx, -hk * dx * dz);
        tz += Vec3::new(-hk * dx * dz, -ak * dz, -hk * dz * dz);
    }
    (tx, tz)
}

/// The unit normal (pointing up) of the surface at the still-water point `p`.
pub(crate) fn normal(waves: &[ResolvedWave], phases: &[f32], p: Vec2) -> Vec3 {
    let (tx, tz) = tangents(waves, phases, p);
    tz.cross(tx).normalize_or(Vec3::Y)
}

/// The Jacobian determinant of the horizontal motion at `p` (above 0: the surface does not loop
/// over itself there).
#[cfg(test)]
pub(crate) fn jacobian(waves: &[ResolvedWave], phases: &[f32], p: Vec2) -> f32 {
    let (tx, tz) = tangents(waves, phases, p);
    tx.x * tz.z - tx.z * tz.x
}

/// The sum of `steepness x k x amplitude` (`H k`) over the waves: the Jacobian is at least
/// `1 - ` this, so at most 1 means no wave combination loops.
pub(crate) fn loop_factor(waves: &[ResolvedWave]) -> f32 {
    waves.iter().map(|w| w.horizontal * w.k).sum()
}

/// The still-water point that moves to the horizontal point `target` (Newton's method on the
/// horizontal motion, six steps from `target`; a fixed-point step where the Jacobian is nearly
/// 0).
pub(crate) fn still_point(waves: &[ResolvedWave], phases: &[f32], target: Vec2) -> Vec2 {
    let mut p = target;
    for _ in 0..6 {
        let d = displacement(waves, phases, p);
        let error = p + Vec2::new(d.x, d.z) - target;
        let (tx, tz) = tangents(waves, phases, p);
        let det = tx.x * tz.z - tx.z * tz.x;
        if det.abs() < 1e-4 {
            p = target - Vec2::new(d.x, d.z);
            continue;
        }
        // Solve [tx.x tz.x; tx.z tz.z] step = error.
        let step = Vec2::new(tz.z * error.x - tz.x * error.y, -tx.z * error.x + tx.x * error.y) / det;
        p -= step;
    }
    p
}
