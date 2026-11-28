// The particle system's step (Not in C#): one invocation per particle slot. Slots in the spawn
// range are given new particles, every other live particle is moved and aged, and each slot's
// glow-instance record (the glow shader's layout, the velocity in the pulse's yzw) is written for
// drawing. The spawn range is shared by the emitters that spawn in this step: each has a record
// with its first order in the range (the prefix sum of the counts before it) and its count, and a
// spawned slot finds its emitter by a binary search of those firsts. The arithmetic is the crate's
// effects/particle_math.rs (the same formulas; the random bits exactly): keep the two in step.

struct Params {
    forces: vec4<f32>,
    size: vec4<f32>,
    birth: vec4<f32>,
    death: vec4<f32>,
    step: vec4<f32>,
    // First slot, count (of every emitter together), index of the first spawned particle, seed.
    spawn: vec4<u32>,
    // x: capacity; y: the emitter records of this step (those that spawn).
    counts: vec4<u32>,
};

struct Emitter {
    // xyz: the centre now; w: the shape (0 point, 1 sphere, 2 box, 3 cone).
    center: vec4<f32>,
    previous: vec4<f32>,
    direction: vec4<f32>,
    shape: vec4<f32>,
    speed_life: vec4<f32>,
    // x: the emitter's first order in the spawn range; y: its count.
    spawn: vec4<u32>,
};

struct Particle {
    position_age: vec4<f32>,
    velocity_life: vec4<f32>,
};

struct Instance {
    position_size: vec4<f32>,
    color: vec4<f32>,
    pulse: vec4<f32>,
};

@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(2) var<storage, read_write> instances: array<Instance>;
@group(0) @binding(3) var<storage, read> emitters: array<Emitter>;

const PI: f32 = 3.14159265358979;
const TAU: f32 = 6.28318530718;

fn pcg(value: u32) -> u32 {
    let state = value * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn random(seed: u32, index: u32, dimension: u32) -> f32 {
    let h = pcg(pcg(index ^ pcg(seed)) ^ (dimension * 0x9E3779B9u));
    return f32(h >> 8u) * (1.0 / 16777216.0);
}

struct Basis {
    t: vec3<f32>,
    b: vec3<f32>,
};

fn basis(axis: vec3<f32>) -> Basis {
    let sign = select(-1.0, 1.0, axis.z >= 0.0);
    let a = -1.0 / (sign + axis.z);
    let b = axis.x * axis.y * a;
    return Basis(vec3<f32>(1.0 + sign * axis.x * axis.x * a, sign * b, -sign * axis.x), vec3<f32>(b, sign + axis.y * axis.y * a, -axis.y));
}

fn direction_in_cone(axis: vec3<f32>, spread: f32, u: f32, v: f32) -> vec3<f32> {
    let cos_theta = 1.0 + (cos(clamp(spread, 0.0, PI)) - 1.0) * u;
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let phi = TAU * v;
    let frame = basis(axis);
    return frame.t * (sin_theta * cos(phi)) + frame.b * (sin_theta * sin(phi)) + axis * cos_theta;
}

fn sample_position(e: Emitter, center: vec3<f32>, axis: vec3<f32>, r0: f32, r1: f32, r2: f32) -> vec3<f32> {
    let kind = u32(e.center.w + 0.5);
    switch kind {
        case 1u: {
            return center + direction_in_cone(vec3<f32>(0.0, 1.0, 0.0), PI, r0, r1) * (max(e.shape.x, 0.0) * pow(r2, 1.0 / 3.0));
        }
        case 2u: {
            return center + vec3<f32>(2.0 * r0 - 1.0, 2.0 * r1 - 1.0, 2.0 * r2 - 1.0) * max(e.shape.xyz, vec3<f32>(0.0));
        }
        case 3u: {
            let frame = basis(axis);
            let distance = max(e.shape.x, 0.0) * sqrt(r0);
            let angle = TAU * r1;
            return center + (frame.t * cos(angle) + frame.b * sin(angle)) * distance;
        }
        default: {
            return center;
        }
    }
}

struct Motion {
    position: vec3<f32>,
    velocity: vec3<f32>,
};

fn integrate(position: vec3<f32>, velocity: vec3<f32>, dt: f32) -> Motion {
    let v = (velocity + p.forces.xyz * dt) * exp(-max(p.forces.w, 0.0) * dt);
    return Motion(position + v * dt, v);
}

// The emitter record of the spawn order `order`: the last whose first order is at most `order`
// (records with no particles are never written, so it is the one whose range holds it).
fn find_emitter(order: u32) -> u32 {
    var low = 0u;
    var high = p.counts.y;
    loop {
        if (low >= high) {
            break;
        }
        let middle = (low + high) / 2u;
        if (emitters[middle].spawn.x <= order) {
            low = middle + 1u;
        } else {
            high = middle;
        }
    }
    return max(low, 1u) - 1u;
}

fn spawn(index: u32, total_order: u32, dt: f32) -> Particle {
    let e = emitters[find_emitter(total_order)];
    let order = total_order - e.spawn.x;
    let count = e.spawn.y;
    let seed = p.spawn.w;
    let birth = (f32(order) + 0.5) / f32(max(count, 1u));
    let center = mix(e.previous.xyz, e.center.xyz, birth);
    let axis = e.direction.xyz;
    let position = sample_position(e, center, axis, random(seed, index, 0u), random(seed, index, 1u), random(seed, index, 2u));
    let direction = direction_in_cone(axis, e.direction.w, random(seed, index, 3u), random(seed, index, 4u));
    let velocity = direction * (e.speed_life.x + (e.speed_life.y - e.speed_life.x) * random(seed, index, 5u));
    let lifetime = max(e.speed_life.z + (e.speed_life.w - e.speed_life.z) * random(seed, index, 6u), 1e-3);
    let remaining = dt * (1.0 - birth);
    let motion = integrate(position, velocity, remaining);
    return Particle(vec4<f32>(motion.position, remaining), vec4<f32>(motion.velocity, lifetime));
}

@compute @workgroup_size(64)
fn step_particles(@builtin(global_invocation_id) id: vec3<u32>) {
    let capacity = p.counts.x;
    let slot = id.x;
    if (slot >= capacity) {
        return;
    }
    let dt = p.step.x;
    var particle = particles[slot];
    let order = (slot + capacity - p.spawn.x) % capacity;
    let count = p.spawn.y;
    if (order < count) {
        particle = spawn(p.spawn.z + order, order, dt);
    } else if (particle.velocity_life.w > 0.0 && particle.position_age.w < particle.velocity_life.w) {
        let motion = integrate(particle.position_age.xyz, particle.velocity_life.xyz, dt);
        particle.position_age = vec4<f32>(motion.position, particle.position_age.w + dt);
        particle.velocity_life = vec4<f32>(motion.velocity, particle.velocity_life.w);
    }
    particles[slot] = particle;

    var instance: Instance;
    let alive = particle.velocity_life.w > 0.0 && particle.position_age.w < particle.velocity_life.w;
    if (alive) {
        let t = clamp(particle.position_age.w / particle.velocity_life.w, 0.0, 1.0);
        let size = max(p.size.x + (p.size.y - p.size.x) * t, 0.0);
        let color = max(mix(p.birth.rgb, p.death.rgb, t) * max(p.size.w, 0.0), vec3<f32>(0.0));
        let opacity = p.birth.a + (p.death.a - p.birth.a) * t;
        let fade = clamp(p.size.z, 0.0, 1.0);
        let x = select(1.0, clamp((1.0 - t) / max(fade, 1e-6), 0.0, 1.0), fade > 0.0);
        instance.position_size = vec4<f32>(particle.position_age.xyz, size);
        instance.color = vec4<f32>(color, clamp(opacity, 0.0, 1.0) * x * x * (3.0 - 2.0 * x));
        instance.pulse = vec4<f32>(1.0, particle.velocity_life.xyz);
    } else {
        instance.position_size = vec4<f32>(0.0);
        instance.color = vec4<f32>(0.0);
        instance.pulse = vec4<f32>(0.0);
    }
    instances[slot] = instance;
}
