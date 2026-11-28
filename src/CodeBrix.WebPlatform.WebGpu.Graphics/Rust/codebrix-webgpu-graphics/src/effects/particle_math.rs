//! The particle system's arithmetic on the CPU (Not in C#): the seeded random numbers, the
//! emitter sampling, one integration step, the spawn count of a step, its split across the
//! emitters and the search of a spawned slot's emitter, and the look over a particle's life - the
//! same formulas as the particle step shader (`particles.wgsl`; the integer random bits exactly,
//! the floating-point results to each GPU's own precision), so that they are tested on the host.
//! Keep the two in step.

#[cfg(test)]
use glam::{Vec3, Vec4};

#[cfg(test)]
use crate::effects::glow_drawing::GlowInstance;
#[cfg(test)]
use crate::effects::particle_system::{EmitterShape, ParticleSettings};

/// The PCG hash (a permuted congruential step of one 32-bit word): integer only, so the GPU and
/// the CPU give the same bits.
#[cfg(test)]
pub(crate) fn pcg(value: u32) -> u32 {
    let state = value.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

/// The random number in 0..1 of particle `index` (the count of particles spawned before it) for
/// draw `dimension`, under `seed`: a pure function of the three, so a seed and a sequence of
/// frame deltas give the same particles every time.
#[cfg(test)]
pub(crate) fn random(seed: u32, index: u32, dimension: u32) -> f32 {
    let h = pcg(pcg(index ^ pcg(seed)) ^ dimension.wrapping_mul(0x9E37_79B9));
    (h >> 8) as f32 * (1.0 / 16_777_216.0)
}

/// Two unit vectors perpendicular to `axis` (a unit vector) and to each other (Duff et al.'s
/// branch-free basis).
#[cfg(test)]
pub(crate) fn basis(axis: Vec3) -> (Vec3, Vec3) {
    let sign = if axis.z >= 0.0 { 1.0 } else { -1.0 };
    let a = -1.0 / (sign + axis.z);
    let b = axis.x * axis.y * a;
    (Vec3::new(1.0 + sign * axis.x * axis.x * a, sign * b, -sign * axis.x), Vec3::new(b, sign + axis.y * axis.y * a, -axis.y))
}

/// A direction within `spread` radians of `axis` (uniform over that cap of the sphere; `spread`
/// of pi = every direction) from two random numbers.
#[cfg(test)]
pub(crate) fn direction_in_cone(axis: Vec3, spread: f32, u: f32, v: f32) -> Vec3 {
    let cos_theta = 1.0 + (spread.clamp(0.0, std::f32::consts::PI).cos() - 1.0) * u;
    let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
    let phi = std::f32::consts::TAU * v;
    let (t, b) = basis(axis);
    t * (sin_theta * phi.cos()) + b * (sin_theta * phi.sin()) + axis * cos_theta
}

/// Where a particle starts, from the emitter's centre and axis and three random numbers.
#[cfg(test)]
pub(crate) fn sample_position(shape: EmitterShape, center: Vec3, axis: Vec3, r: [f32; 3]) -> Vec3 {
    match shape {
        EmitterShape::Point => center,
        EmitterShape::Sphere { radius } => center + direction_in_cone(Vec3::Y, std::f32::consts::PI, r[0], r[1]) * (radius.max(0.0) * r[2].cbrt()),
        EmitterShape::Box { half_extents } => center + Vec3::new(2.0 * r[0] - 1.0, 2.0 * r[1] - 1.0, 2.0 * r[2] - 1.0) * half_extents.max(Vec3::ZERO),
        EmitterShape::Cone { radius, .. } => {
            let (t, b) = basis(axis);
            let distance = radius.max(0.0) * r[0].sqrt();
            let angle = std::f32::consts::TAU * r[1];
            center + (t * angle.cos() + b * angle.sin()) * distance
        }
    }
}

/// The spread of the starting directions: the cone's angle for a cone emitter, the emitter's
/// spread otherwise.
#[cfg(test)]
pub(crate) fn shape_spread(shape: EmitterShape, spread: f32) -> f32 {
    match shape {
        EmitterShape::Cone { angle, .. } => angle,
        _ => spread,
    }
}

/// One integration step: constant acceleration, then drag (velocity times `exp(-drag dt)`,
/// stable for any step), then the position moves by the new velocity.
#[cfg(test)]
pub(crate) fn integrate(position: Vec3, velocity: Vec3, acceleration: Vec3, drag: f32, dt: f32) -> (Vec3, Vec3) {
    let velocity = (velocity + acceleration * dt) * (-drag.max(0.0) * dt).exp();
    (position + velocity * dt, velocity)
}

/// How many particles a step spawns: the rate's share of the step added to the carried
/// fraction, plus a pending burst, at most the capacity. Returns the count and the fraction
/// carried to the next step. A step of 0 (a paused clock) spawns nothing and keeps the burst
/// waiting.
pub(crate) fn spawn_count(carried: f64, rate: f32, dt: f32, burst: u32, capacity: u32) -> (u32, f64, u32) {
    if dt <= 0.0 {
        return (0, carried, burst);
    }
    let total = carried + rate.max(0.0) as f64 * dt as f64;
    let whole = total.floor();
    let count = (whole as u64 + burst as u64).min(capacity as u64) as u32;
    (count, total - whole, 0)
}

/// One emitter's share of a step's spawn range (Not in C#): the emitter's index, its first order
/// in the range (the counts of the emitters before it, a prefix sum) and its count (never 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpawnRange {
    pub(crate) emitter: usize,
    pub(crate) first: u32,
    pub(crate) count: u32,
}

/// Splits a step's spawn range across the emitters by their counts, in emitter order: each
/// emitter with particles gets the next `count` orders. The whole range is at most `capacity`:
/// when the emitters together ask for more, the later emitters' counts are cut (the first
/// emitters keep theirs). Emitters with nothing to spawn get no range, so the shader's search
/// finds only emitters that spawn.
pub(crate) fn split_spawns(counts: &[u32], capacity: u32) -> Vec<SpawnRange> {
    let mut ranges = Vec::new();
    let mut first = 0u32;
    for (emitter, &count) in counts.iter().enumerate() {
        let count = count.min(capacity - first);
        if count > 0 {
            ranges.push(SpawnRange { emitter, first, count });
            first += count;
        }
    }
    ranges
}

/// The CPU twin of the shader's `find_emitter`: the index of the last range whose first order is
/// at most `order` (a binary search of the firsts), so the range that holds `order`.
#[cfg(test)]
pub(crate) fn find_emitter(firsts: &[u32], order: u32) -> usize {
    let (mut low, mut high) = (0usize, firsts.len());
    while low < high {
        let middle = (low + high) / 2;
        if firsts[middle] <= order {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    low.max(1) - 1
}

/// What the shader's spawn of the slot at order `order` of the step's range reads from its
/// emitter's range: the emitter, the order within it and its count.
#[cfg(test)]
pub(crate) fn spawn_slot(ranges: &[SpawnRange], order: u32) -> (usize, u32, u32) {
    let firsts: Vec<u32> = ranges.iter().map(|range| range.first).collect();
    let range = ranges[find_emitter(&firsts, order)];
    (range.emitter, order - range.first, range.count)
}

/// One step's spawned particles as the live-particle bound keeps them (Not in C#): when the step
/// ended in simulated time, how many it spawned, and the longest life any of them can have.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SpawnBatch {
    pub(crate) end: f64,
    pub(crate) count: u32,
    pub(crate) longest_life: f32,
}

/// Drops the batches whose every particle has died by simulated time `now`: a particle born in a
/// step is born no later than the step's end, so it is dead from `end + longest_life` on.
pub(crate) fn prune_batches(batches: &mut std::collections::VecDeque<SpawnBatch>, now: f64) {
    batches.retain(|batch| batch.end + batch.longest_life as f64 > now);
}

/// AT MOST how many particles are alive at simulated time `now`: the particles of every batch
/// that may still live, never more than the capacity. An UPPER BOUND, exact when every particle
/// lives the longest life (`lifetime_min == lifetime_max`) and was born at its step's end; a
/// particle born earlier in a step, or given a shorter life, may already have died.
pub(crate) fn alive_at_most(batches: &std::collections::VecDeque<SpawnBatch>, now: f64, capacity: u32) -> u32 {
    let alive: u64 = batches.iter().filter(|batch| batch.end + batch.longest_life as f64 > now).map(|batch| batch.count as u64).sum();
    alive.min(capacity as u64) as u32
}

/// The step's `dt` from the frame's delta: never negative, at most `max_step` (a tab brought
/// back after minutes does not fire a minute of particles at once).
pub(crate) fn step_seconds(delta: f64, max_step: f32) -> f32 {
    (delta.max(0.0) as f32).min(max_step.max(0.0))
}

/// The look of a particle at `t` (age / lifetime, 0..1): size, linear colour times intensity,
/// and opacity (with the fade-out over the last `fade_out` of the life).
#[cfg(test)]
pub(crate) fn over_life(settings: &ParticleSettings, t: f32) -> (f32, Vec3, f32) {
    let t = t.clamp(0.0, 1.0);
    let size = settings.size_at_birth + (settings.size_at_death - settings.size_at_birth) * t;
    let color = settings.color_at_birth.lerp(settings.color_at_death, t) * settings.intensity.max(0.0);
    let opacity = settings.opacity_at_birth + (settings.opacity_at_death - settings.opacity_at_birth) * t;
    let fade = settings.fade_out.clamp(0.0, 1.0);
    let x = if fade > 0.0 { ((1.0 - t) / fade).clamp(0.0, 1.0) } else { 1.0 };
    (size.max(0.0), color.max(Vec3::ZERO), opacity.clamp(0.0, 1.0) * x * x * (3.0 - 2.0 * x))
}

/// The CPU twin of the glow-instance record the step writes for a slot: a live particle's
/// position and size, its colour and opacity over its life, and intensity 1 with its VELOCITY in
/// the pulse's yzw (the glow shader reads no pulse for particles; the stretch reads the velocity);
/// a dead or never-born slot all zeros (a collapsed quad).
#[cfg(test)]
pub(crate) fn instance_record(particle: &ParticleState, settings: &ParticleSettings) -> GlowInstance {
    let alive = particle.velocity_life.w > 0.0 && particle.position_age.w < particle.velocity_life.w;
    if !alive {
        return GlowInstance::default();
    }
    let (size, color, opacity) = over_life(settings, particle.position_age.w / particle.velocity_life.w);
    GlowInstance { position_size: particle.position_age.truncate().extend(size), color: color.extend(opacity), pulse: Vec4::new(1.0, particle.velocity_life.x, particle.velocity_life.y, particle.velocity_life.z) }
}

/// One particle as the step leaves it: position and age, velocity and lifetime.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg(test)]
pub(crate) struct ParticleState {
    /// xyz: the position; w: the age in seconds.
    pub(crate) position_age: Vec4,
    /// xyz: the velocity; w: the lifetime in seconds (0: never born).
    pub(crate) velocity_life: Vec4,
}

/// The CPU twin of the shader's spawn of particle `index` (the particles spawned before it), the
/// `order`-th of `count` spawned in this step of `dt`: born at its share of the step, between the
/// emitter's previous and current centre, then integrated for the rest of the step.
#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn spawn(seed: u32, index: u32, order: u32, count: u32, dt: f32, shape: EmitterShape, previous: Vec3, current: Vec3, axis: Vec3, spread: f32, speed: (f32, f32), life: (f32, f32), settings: &ParticleSettings) -> ParticleState {
    let r = |dimension: u32| random(seed, index, dimension);
    let birth = (order as f32 + 0.5) / count.max(1) as f32;
    let center = previous.lerp(current, birth);
    let position = sample_position(shape, center, axis, [r(0), r(1), r(2)]);
    let direction = direction_in_cone(axis, shape_spread(shape, spread), r(3), r(4));
    let velocity = direction * (speed.0 + (speed.1 - speed.0) * r(5));
    let lifetime = (life.0 + (life.1 - life.0) * r(6)).max(1e-3);
    let remaining = dt * (1.0 - birth);
    let (position, velocity) = integrate(position, velocity, settings.acceleration, settings.drag, remaining);
    ParticleState { position_age: position.extend(remaining), velocity_life: velocity.extend(lifetime) }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;

    #[test]
    fn the_live_particle_bound_counts_what_may_still_live_and_never_passes_the_capacity() {
        let mut batches = std::collections::VecDeque::new();
        let batch = |end: f64, count: u32, longest_life: f32| SpawnBatch { end, count, longest_life };
        batches.push_back(batch(0.1, 10, 1.0));
        batches.push_back(batch(0.2, 5, 2.0));
        batches.push_back(batch(0.3, 7, 0.5));
        assert_eq!(alive_at_most(&batches, 0.3, 1000), 22);
        assert_eq!(alive_at_most(&batches, 0.85, 1000), 15, "the third batch lived at most until 0.8");
        assert_eq!(alive_at_most(&batches, 1.15, 1000), 5);
        assert_eq!(alive_at_most(&batches, 2.2, 1000), 0, "everything dead at the last end + life");
        assert_eq!(alive_at_most(&batches, 0.3, 12), 12, "the capacity caps it (the oldest are replaced)");
        prune_batches(&mut batches, 1.15);
        assert_eq!(batches.len(), 1);
        assert_eq!(alive_at_most(&batches, 1.15, 1000), 5);
        prune_batches(&mut batches, 5.0);
        assert!(batches.is_empty());
        // A steady rate: rate x longest life at most (here 50 per second, lives 2 s, 60 Hz steps).
        let mut steady = std::collections::VecDeque::new();
        let (mut time, mut carried) = (0.0f64, 0.0f64);
        for _ in 0..600 {
            let (count, rest, _) = spawn_count(carried, 50.0, 1.0 / 60.0, 0, 4096);
            carried = rest;
            time += 1.0 / 60.0;
            steady.push_back(batch(time, count, 2.0));
            prune_batches(&mut steady, time);
        }
        let bound = alive_at_most(&steady, time, 4096);
        assert!((99..=101).contains(&bound), "{bound}");
    }

    #[test]
    fn the_random_numbers_are_a_pure_function_of_seed_index_and_dimension() {
        assert_eq!(random(7, 123, 4), random(7, 123, 4));
        assert_ne!(random(7, 123, 4), random(8, 123, 4));
        assert_ne!(random(7, 123, 4), random(7, 124, 4));
        assert_ne!(random(7, 123, 4), random(7, 123, 5));
        // Known bits, so a change of the hash (which the shader must follow) is seen.
        assert_eq!(pcg(0), 129_708_002);
        assert_eq!(pcg(1), 2_831_084_092);
        // Uniform in 0..1.
        let values: Vec<f32> = (0..20_000).map(|index| random(42, index, 0)).collect();
        assert!(values.iter().all(|v| (0.0..1.0).contains(v)));
        let mean = values.iter().sum::<f32>() / values.len() as f32;
        assert!((mean - 0.5).abs() < 0.01, "{mean}");
        let low = values.iter().filter(|v| **v < 0.1).count() as f32 / values.len() as f32;
        assert!((low - 0.1).abs() < 0.01, "{low}");
    }

    #[test]
    fn the_basis_is_orthonormal_for_every_axis() {
        for axis in [Vec3::X, Vec3::Y, Vec3::Z, -Vec3::Z, Vec3::new(1.0, 2.0, -3.0).normalize(), Vec3::new(0.0, 0.0, -1.0)] {
            let (t, b) = basis(axis);
            assert!((t.length() - 1.0).abs() < 1e-5 && (b.length() - 1.0).abs() < 1e-5);
            assert!(t.dot(b).abs() < 1e-5 && t.dot(axis).abs() < 1e-5 && b.dot(axis).abs() < 1e-5);
        }
    }

    #[test]
    fn sphere_samples_fill_the_ball_evenly() {
        let shape = EmitterShape::Sphere { radius: 2.0 };
        let center = Vec3::new(1.0, 0.0, -1.0);
        let points: Vec<Vec3> = (0..20_000).map(|i| sample_position(shape, center, Vec3::Y, [random(3, i, 0), random(3, i, 1), random(3, i, 2)])).collect();
        assert!(points.iter().all(|p| p.distance(center) <= 2.0 + 1e-4));
        let mean = points.iter().copied().sum::<Vec3>() / points.len() as f32;
        assert!(mean.distance(center) < 0.05, "{mean}");
        // Uniform in volume: half the points lie within radius 2 / cbrt(2).
        let inner = points.iter().filter(|p| p.distance(center) < 2.0 / 2f32.cbrt()).count() as f32 / points.len() as f32;
        assert!((inner - 0.5).abs() < 0.02, "{inner}");
    }

    #[test]
    fn box_point_and_cone_samples_stay_in_their_shapes() {
        let half = Vec3::new(1.0, 0.5, 2.0);
        for i in 0..5_000 {
            let r = [random(9, i, 0), random(9, i, 1), random(9, i, 2)];
            let p = sample_position(EmitterShape::Box { half_extents: half }, Vec3::ZERO, Vec3::Y, r);
            assert!(p.abs().cmple(half).all());
            assert_eq!(sample_position(EmitterShape::Point, Vec3::ONE, Vec3::Y, r), Vec3::ONE);
            // A cone starts on its base disc, perpendicular to the axis.
            let axis = Vec3::new(0.0, 1.0, 1.0).normalize();
            let c = sample_position(EmitterShape::Cone { radius: 0.5, angle: 0.3 }, Vec3::ZERO, axis, r);
            assert!(c.length() <= 0.5 + 1e-5 && c.dot(axis).abs() < 1e-5);
        }
    }

    #[test]
    fn directions_stay_within_the_spread_and_cover_it() {
        let axis = Vec3::new(1.0, 1.0, 0.0).normalize();
        let spread = 0.4f32;
        let mut widest = 0.0f32;
        for i in 0..10_000 {
            let d = direction_in_cone(axis, spread, random(5, i, 3), random(5, i, 4));
            assert!((d.length() - 1.0).abs() < 1e-4);
            let angle = d.dot(axis).clamp(-1.0, 1.0).acos();
            assert!(angle <= spread + 1e-3, "{angle}");
            widest = widest.max(angle);
        }
        assert!(widest > spread * 0.95);
        assert_eq!(direction_in_cone(Vec3::Z, 0.0, 0.7, 0.2), Vec3::Z);
        // A spread of pi: every direction, as many down as up.
        let down = (0..10_000).filter(|i| direction_in_cone(Vec3::Y, std::f32::consts::PI, random(1, *i, 3), random(1, *i, 4)).y < 0.0).count();
        assert!((down as f32 / 10_000.0 - 0.5).abs() < 0.02);
        assert_eq!(shape_spread(EmitterShape::Cone { radius: 0.0, angle: 0.25 }, 1.0), 0.25);
        assert_eq!(shape_spread(EmitterShape::Point, 1.0), 1.0);
    }

    #[test]
    fn one_step_applies_acceleration_then_drag_then_moves() {
        let (p, v) = integrate(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, -10.0, 0.0), 0.0, 0.1);
        assert!((v - Vec3::new(1.0, -1.0, 0.0)).length() < 1e-6);
        assert!((p - Vec3::new(0.1, -0.1, 0.0)).length() < 1e-6);
        // Drag: exp(-2 * 0.5) of the velocity remains.
        let (_, slowed) = integrate(Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0), Vec3::ZERO, 2.0, 0.5);
        assert!((slowed.x - 4.0 * (-1.0f32).exp()).abs() < 1e-5);
        // A paused clock (dt 0) changes nothing.
        assert_eq!(integrate(Vec3::ONE, Vec3::X, Vec3::Y, 1.0, 0.0), (Vec3::ONE, Vec3::X));
    }

    #[test]
    fn the_spawn_count_follows_the_rate_and_carries_the_fraction() {
        // 30 per second at 1/60 s: one every second frame.
        let mut carried = 0.0;
        let mut counts = Vec::new();
        for _ in 0..6 {
            let (count, next, _) = spawn_count(carried, 30.0, 1.0 / 60.0, 0, 100);
            counts.push(count);
            carried = next;
        }
        assert_eq!(counts.iter().sum::<u32>(), 3);
        // A burst comes on top, the total at most the capacity.
        assert_eq!(spawn_count(0.0, 10.0, 0.1, 5, 100).0, 6);
        assert_eq!(spawn_count(0.0, 1000.0, 1.0, 50, 64).0, 64);
        // Paused: nothing spawns and the burst waits.
        assert_eq!(spawn_count(0.25, 30.0, 0.0, 7, 100), (0, 0.25, 7));
    }

    #[test]
    fn a_steps_spawns_are_split_across_the_emitters_by_a_prefix_sum() {
        let ranges = split_spawns(&[3, 0, 2, 0, 0, 5], 100);
        assert_eq!(ranges, vec![SpawnRange { emitter: 0, first: 0, count: 3 }, SpawnRange { emitter: 2, first: 3, count: 2 }, SpawnRange { emitter: 5, first: 5, count: 5 }]);
        // One emitter: the whole range is its, as it always was.
        assert_eq!(split_spawns(&[7], 100), vec![SpawnRange { emitter: 0, first: 0, count: 7 }]);
        assert!(split_spawns(&[0, 0], 100).is_empty(), "nothing to spawn: no ranges");
        // Over the capacity: the later emitters are cut.
        assert_eq!(split_spawns(&[6, 6, 6], 10), vec![SpawnRange { emitter: 0, first: 0, count: 6 }, SpawnRange { emitter: 1, first: 6, count: 4 }]);
        assert_eq!(split_spawns(&[20, 3], 10), vec![SpawnRange { emitter: 0, first: 0, count: 10 }]);
    }

    #[test]
    fn every_spawned_slot_finds_its_emitter_from_the_prefix() {
        let counts: Vec<u32> = (0..48).map(|i| (i * 7 % 5) as u32).collect();
        let ranges = split_spawns(&counts, 4096);
        let total: u32 = ranges.iter().map(|range| range.count).sum();
        assert_eq!(total, counts.iter().sum::<u32>());
        let firsts: Vec<u32> = ranges.iter().map(|range| range.first).collect();
        for order in 0..total {
            // The binary search agrees with a plain walk through the ranges.
            let plain = ranges.iter().position(|range| range.first <= order && order < range.first + range.count).expect("a range holds it");
            assert_eq!(find_emitter(&firsts, order), plain, "order {order}");
            let (emitter, within, count) = spawn_slot(&ranges, order);
            assert_eq!(emitter, ranges[plain].emitter);
            assert!(within < count && count == counts[emitter]);
        }
        assert_eq!(find_emitter(&[0], 0), 0);
        assert_eq!(find_emitter(&[0], 9), 0);
        // Emitter 0 alone: each slot is its own order of the one range, as before.
        let single = split_spawns(&[5], 100);
        for order in 0..5 {
            assert_eq!(spawn_slot(&single, order), (0, order, 5));
        }
    }

    #[test]
    fn the_instance_record_carries_the_velocity_in_the_pulse() {
        let settings = ParticleSettings::default();
        let particle = ParticleState { position_age: Vec4::new(1.0, 2.0, 3.0, 0.5), velocity_life: Vec4::new(-1.0, 4.0, 0.25, 2.0) };
        let record = instance_record(&particle, &settings);
        let (size, color, opacity) = over_life(&settings, 0.25);
        assert_eq!(record.position_size, Vec4::new(1.0, 2.0, 3.0, size));
        assert_eq!(record.color, color.extend(opacity));
        assert_eq!(record.pulse, Vec4::new(1.0, -1.0, 4.0, 0.25), "intensity 1, then the velocity");
        let dead = ParticleState { position_age: Vec4::new(1.0, 2.0, 3.0, 2.5), velocity_life: Vec4::new(-1.0, 4.0, 0.25, 2.0) };
        assert_eq!(instance_record(&dead, &settings), GlowInstance::default());
        assert_eq!(instance_record(&ParticleState::default(), &settings), GlowInstance::default(), "never born");
    }

    #[test]
    fn the_step_follows_the_clock_but_never_jumps_far() {
        assert_eq!(step_seconds(0.016, 0.1), 0.016);
        assert_eq!(step_seconds(0.0, 0.1), 0.0);
        assert_eq!(step_seconds(-1.0, 0.1), 0.0);
        assert_eq!(step_seconds(120.0, 0.1), 0.1);
    }

    #[test]
    fn the_look_moves_from_birth_to_death_and_fades_out_at_the_end() {
        let settings = ParticleSettings { size_at_birth: 1.0, size_at_death: 3.0, color_at_birth: Vec3::new(1.0, 0.0, 0.0), color_at_death: Vec3::new(0.0, 0.0, 1.0), opacity_at_birth: 1.0, opacity_at_death: 1.0, intensity: 2.0, fade_out: 0.25, ..ParticleSettings::default() };
        let (size, color, opacity) = over_life(&settings, 0.0);
        assert_eq!((size, color, opacity), (1.0, Vec3::new(2.0, 0.0, 0.0), 1.0));
        let (size, color, opacity) = over_life(&settings, 0.5);
        assert_eq!((size, color, opacity), (2.0, Vec3::new(1.0, 0.0, 1.0), 1.0));
        // In the last quarter it fades smoothly to nothing.
        let (_, _, late) = over_life(&settings, 0.875);
        assert!((late - 0.5).abs() < 1e-5, "{late}");
        assert_eq!(over_life(&settings, 1.0).2, 0.0);
    }

    #[test]
    fn a_spawned_particle_is_the_same_for_the_same_seed_and_deltas() {
        let settings = ParticleSettings { acceleration: Vec3::new(0.0, -9.81, 0.0), drag: 0.5, ..ParticleSettings::default() };
        let make = |seed: u32, dt: f32| spawn(seed, 17, 2, 4, dt, EmitterShape::Sphere { radius: 0.5 }, Vec3::ZERO, Vec3::X, Vec3::Y, 0.3, (1.0, 2.0), (1.0, 3.0), &settings);
        assert_eq!(make(11, 1.0 / 60.0), make(11, 1.0 / 60.0));
        assert_ne!(make(11, 1.0 / 60.0), make(12, 1.0 / 60.0));
        let particle = make(11, 0.1);
        // Born 5/8 through the step, so it is 3/8 of the step old; lifetime in its range.
        assert!((particle.position_age.w - 0.0375).abs() < 1e-6);
        assert!((1.0..=3.0).contains(&particle.velocity_life.w));
        // Its start lies between the previous and current centre of the emitter (+- the radius).
        assert!(particle.position_age.truncate().distance(Vec3::new(0.625, 0.0, 0.0)) < 0.5 + 2.0 * 0.0375 + 1e-3);
    }
}
