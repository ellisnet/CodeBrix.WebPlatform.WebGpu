//! `DrawingParticleSystem`: particles as a PURE FUNCTION of the time and a seed (C#
//! `DrawingParticleSystem`).

use crate::planes::drawing_random::hash;
use crate::planes::{DrawingParticle, DrawingParticleEmitter, DrawingRandom};
use crate::Color;

/// A deterministic particle system as DATA (C# `DrawingParticleSystem`): a seed and a list of
/// [`DrawingParticleEmitter`]s, from which [`DrawingParticleSystem::get_particles`] returns the
/// live particles at a moment as a PURE FUNCTION of the time (passed in - the frame's time) and
/// the seed. Nothing is stepped from frame to frame, so frames can be drawn in any order, skipped
/// or repeated and always agree. Nothing is drawn here.
///
/// THE RULE. Emitter e's particle k is born at `start_time` (burst, k = 0 .. burst - 1) or at
/// `start_time + n / rate` (continuous particle n, k = burst + n, while that is before
/// `end_time`). Its random choices come from `DrawingRandom::for_key(Mix(Mix(seed) + e), k)`, in
/// this order: lifetime variation, launch angle, speed variation, spawn angle, spawn radius
/// (`spawn_radius * sqrt(u)`), size variation. It is alive at t when `birth <= t < birth +
/// lifetime`; its centre is `emitter + spawn offset + v * age + gravity * age^2 / 2`. Size,
/// opacity and colour blend linearly by `age / lifetime`. All in double, as C#; the cos / sin
/// are the math library's (the same values as C# on the host; a browser's may differ in the
/// last bit).
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingParticleSystem {
    seed: i64,
    emitters: Vec<DrawingParticleEmitter>,
}

impl DrawingParticleSystem {
    /// An empty system (C# `DrawingParticleSystem(seed)`).
    pub fn new(seed: i64) -> DrawingParticleSystem {
        DrawingParticleSystem { seed, emitters: Vec::new() }
    }

    /// The seed every particle's random choices derive from (C# `Seed`).
    pub fn seed(&self) -> i64 {
        self.seed
    }

    /// The emitters in the order they were added - the order of
    /// [`DrawingParticle::emitter_index`] (C# `Emitters`).
    pub fn emitters(&self) -> &[DrawingParticleEmitter] {
        &self.emitters
    }

    /// One emitter, to change its settings; later queries show the change (C# keeps the emitter
    /// instance, so its settings can change after adding).
    pub fn emitter_mut(&mut self, index: usize) -> Option<&mut DrawingParticleEmitter> {
        self.emitters.get_mut(index)
    }

    /// Adds an emitter and returns it for further set-up (C# `AddEmitter`).
    pub fn add_emitter(&mut self, emitter: DrawingParticleEmitter) -> &mut DrawingParticleEmitter {
        self.emitters.push(emitter);
        let last = self.emitters.len() - 1;
        &mut self.emitters[last]
    }

    /// Every particle alive at a moment, by emitter then birth number (C# `GetParticles(seconds)`).
    /// The error says the time must be a finite number of seconds.
    pub fn get_particles(&self, seconds: f64) -> Result<Vec<DrawingParticle>, String> {
        let mut results = Vec::new();
        self.get_particles_into(seconds, &mut results)?;
        Ok(results)
    }

    /// Fills a list (cleared first) with every particle alive at a moment, so one list serves
    /// every frame (C# `GetParticles(seconds, results)`).
    pub fn get_particles_into(&self, seconds: f64, results: &mut Vec<DrawingParticle>) -> Result<(), String> {
        if !seconds.is_finite() {
            return Err("The time must be a finite number of seconds.".to_string());
        }
        let time = seconds;
        results.clear();
        for (e, emitter) in self.emitters.iter().enumerate() {
            let emitter_seed = hash(self.seed, e as i64) as i64;
            if time < emitter.start_time() {
                continue;
            }
            for k in 0..emitter.burst() {
                try_add(results, emitter, e, emitter_seed, k as i64, emitter.start_time(), time);
            }
            let rate = emitter.rate() as f64;
            if !(rate > 0.0) {
                continue;
            }
            let max_life = emitter.lifetime() + emitter.lifetime_variation();
            let end = time.min(emitter.end_time());
            let last = ((end - emitter.start_time()) * rate).floor() as i64;
            let first = 0i64.max((((time - max_life - emitter.start_time()) * rate).floor() as i64).wrapping_sub(1));
            let mut n = first;
            while n <= last {
                let birth = emitter.start_time() + (n as f64 / rate);
                if !(birth > time || birth >= emitter.end_time()) {
                    try_add(results, emitter, e, emitter_seed, (emitter.burst() as i64).wrapping_add(n), birth, time);
                }
                n += 1;
            }
        }
        Ok(())
    }
}

fn try_add(results: &mut Vec<DrawingParticle>, emitter: &DrawingParticleEmitter, emitter_index: usize, emitter_seed: i64, index: i64, birth: f64, time: f64) {
    let mut random = DrawingRandom::for_key(emitter_seed, index);
    let mut lifetime = emitter.lifetime() + random.uniform(-emitter.lifetime_variation(), emitter.lifetime_variation());
    let shortest = emitter.lifetime() * 0.01;
    if lifetime < shortest {
        lifetime = shortest;
    }
    let half_spread = emitter.spread() as f64 / 2.0;
    let angle = (emitter.direction() as f64 + random.uniform(-(emitter.spread() as f64) / 2.0, half_spread)) * (std::f64::consts::PI / 180.0);
    let speed = emitter.speed() as f64 + random.uniform(-(emitter.speed_variation() as f64), emitter.speed_variation() as f64);
    let spawn_angle = random.uniform(0.0, 2.0 * std::f64::consts::PI);
    let spawn_radius = emitter.spawn_radius() as f64 * random.next_double().sqrt();
    let size_factor = 1.0 + random.uniform(-(emitter.size_variation() as f64), emitter.size_variation() as f64);

    let age = time - birth;
    if age < 0.0 || age >= lifetime {
        return;
    }
    let vx = (speed * angle.cos()) + emitter.drift_x() as f64;
    let vy = (speed * angle.sin()) + emitter.drift_y() as f64;
    let x = emitter.x() as f64 + (spawn_radius * spawn_angle.cos()) + (vx * age) + (0.5 * emitter.gravity_x() as f64 * age * age);
    let y = emitter.y() as f64 + (spawn_radius * spawn_angle.sin()) + (vy * age) + (0.5 * emitter.gravity_y() as f64 * age * age);
    let f = age / lifetime;
    let size = (emitter.start_size() as f64 + ((emitter.end_size() as f64 - emitter.start_size() as f64) * f)) * size_factor;
    let opacity = emitter.start_opacity() as f64 + ((emitter.end_opacity() as f64 - emitter.start_opacity() as f64) * f);
    results.push(DrawingParticle::new(emitter_index, index, birth, age, lifetime, x as f32, y as f32, size as f32, opacity as f32, blend_color(emitter.start_color(), emitter.end_color(), f)));
}

fn blend_color(start: Color, end: Color, f: f64) -> Color {
    Color::from_rgba(channel(start.r, end.r, f), channel(start.g, end.g, f), channel(start.b, end.b, f), channel(start.a, end.a, f))
}

fn channel(a: u8, b: u8, f: f64) -> u8 {
    let v = a as f64 + ((b as i32 - a as i32) as f64 * f);
    if !(v > 0.0) {
        return 0;
    }
    if v >= 255.0 {
        255
    } else {
        (v + 0.5).floor() as u8
    }
}
