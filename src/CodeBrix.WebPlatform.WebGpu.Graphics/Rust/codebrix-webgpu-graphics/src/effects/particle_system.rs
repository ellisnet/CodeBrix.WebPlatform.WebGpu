//! [`ParticleSystem`]: particles simulated on the GPU, drawn with the glow layer's drawing, with
//! [`ParticleSystemOptions`], [`ParticleEmitter`], [`EmitterShape`] and [`ParticleSettings`]
//! (Not in C#).

use glam::{Mat4, UVec4, Vec3, Vec4};

use crate::context::{GraphicsContext, GraphicsException};
use crate::drawing3d::Camera;
use crate::effects::glow_drawing::{GlowBlend, GlowDrawing, INSTANCE_SIZE};
use crate::effects::particle_math::{alive_at_most, prune_batches, spawn_count, step_seconds, SpawnBatch};
use crate::frames::{ComputePassOptions, GraphicsFrame, RenderPass};
use crate::pipelines::GpuComputePipeline;
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBuffer, GpuTexture};
use crate::shaders::EffectShaders;

/// The bytes of one particle's state on the GPU.
const PARTICLE_SIZE: u64 = 32;
/// The step shader's workgroup size.
const WORKGROUP: u32 = 64;

/// Where an emitter starts its particles (Not in C#).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EmitterShape {
    /// At the emitter's position.
    Point,
    /// Anywhere in a ball of this radius around the position (evenly in volume).
    Sphere {
        /// The radius in scene units.
        radius: f32,
    },
    /// Anywhere in a box around the position, aligned with the scene's axes.
    Box {
        /// Half the box's size on each axis, in scene units.
        half_extents: Vec3,
    },
    /// On a disc of this radius around the position, facing the emitter's direction, and moving
    /// within `angle` (radians, from the direction) - the emitter's `spread` is not used.
    Cone {
        /// The radius of the base disc in scene units (0: a point).
        radius: f32,
        /// The half-angle of the cone in radians.
        angle: f32,
    },
}

/// The emitter of a [`ParticleSystem`] (Not in C#): where, how many and how fast. It may move
/// every frame (set its fields before [`ParticleSystem::update`]); particles of a moving emitter
/// start along its path through the step, not in clumps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleEmitter {
    /// The shape particles start in. Default [`EmitterShape::Point`].
    pub shape: EmitterShape,
    /// The centre, in scene units.
    pub position: Vec3,
    /// The main direction particles move in (normalised for use; zero means +Y). Default +Y.
    pub direction: Vec3,
    /// How far starting directions may stray from `direction`, in radians (0: all along it; pi:
    /// every direction). Default 0.3.
    pub spread: f32,
    /// Particles per second (0: only bursts). Default 50.
    pub rate: f32,
    /// The slowest starting speed, in scene units per second. Default 1.
    pub speed_min: f32,
    /// The fastest starting speed. Default 2.
    pub speed_max: f32,
    /// The shortest life in seconds. Default 1.
    pub lifetime_min: f32,
    /// The longest life in seconds. Default 2.
    pub lifetime_max: f32,
}

impl Default for ParticleEmitter {
    fn default() -> ParticleEmitter {
        ParticleEmitter { shape: EmitterShape::Point, position: Vec3::ZERO, direction: Vec3::Y, spread: 0.3, rate: 50.0, speed_min: 1.0, speed_max: 2.0, lifetime_min: 1.0, lifetime_max: 2.0 }
    }
}

impl ParticleEmitter {
    /// Places the emitter by a world transform - for example a model node's
    /// (`ModelInstance::get_node_world_matrix`), so the emitter follows the node: the position is
    /// the transform's translation and the direction its +Y axis.
    pub fn place(&mut self, world: Mat4) {
        self.position = world.w_axis.truncate();
        let direction = world.transform_vector3(Vec3::Y);
        if direction.length_squared() > 0.0 {
            self.direction = direction.normalize();
        }
    }

    /// The direction as the step uses it: unit length, +Y for a zero vector.
    pub(crate) fn axis(&self) -> Vec3 {
        self.direction.try_normalize().unwrap_or(Vec3::Y)
    }
}

/// How particles move and look over their life (Not in C#).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleSettings {
    /// A constant acceleration in scene units per second squared (gravity, a draught). Default
    /// zero.
    pub acceleration: Vec3,
    /// Air drag: the velocity falls by `exp(-drag * dt)` each step (0: none). Default 0.
    pub drag: f32,
    /// The size (width, in scene units) at birth. Default 0.2.
    pub size_at_birth: f32,
    /// The size at death. Default 0.05.
    pub size_at_death: f32,
    /// The linear colour at birth. Default warm white.
    pub color_at_birth: Vec3,
    /// The linear colour at death. Default orange-red.
    pub color_at_death: Vec3,
    /// The opacity at birth, 0..1. Default 1.
    pub opacity_at_birth: f32,
    /// The opacity at death, 0..1. Default 1.
    pub opacity_at_death: f32,
    /// Multiplies the colour; above 1 on the HDR path makes particles bloom. Default 1.
    pub intensity: f32,
    /// The last part of the life (0..1) over which a particle fades out smoothly. Default 0.3.
    pub fade_out: f32,
}

impl Default for ParticleSettings {
    fn default() -> ParticleSettings {
        ParticleSettings {
            acceleration: Vec3::ZERO,
            drag: 0.0,
            size_at_birth: 0.2,
            size_at_death: 0.05,
            color_at_birth: Vec3::new(1.0, 0.9, 0.7),
            color_at_death: Vec3::new(1.0, 0.3, 0.05),
            opacity_at_birth: 1.0,
            opacity_at_death: 1.0,
            intensity: 1.0,
            fade_out: 0.3,
        }
    }
}

/// The settings fixed when a [`ParticleSystem`] is made (Not in C#).
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleSystemOptions {
    /// The most particles alive at once (1 to [`ParticleSystem::MAX_CAPACITY`]). When more are
    /// emitted, the OLDEST emitted are replaced. Default 1024.
    pub capacity: u32,
    /// The seed of the GPU's random numbers: the same seed and the same sequence of frame deltas
    /// (and of emitter settings and bursts) give the same particles. Default 1.
    pub seed: u32,
    /// Additive (order-independent; the default) or ordinary transparency (NOT sorted).
    pub blend: GlowBlend,
    /// The longest step in seconds: a longer frame delta (a tab brought back after a while) is
    /// simulated as this much. Default 0.1.
    pub max_step: f32,
    /// The label of the system's buffers.
    pub label: Option<String>,
}

impl Default for ParticleSystemOptions {
    fn default() -> ParticleSystemOptions {
        ParticleSystemOptions { capacity: 1024, seed: 1, blend: GlowBlend::Additive, max_step: 0.1, label: None }
    }
}

/// The step shader's uniforms (192 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ParticleParams {
    /// xyz: the emitter's centre now; w: the shape (0 point, 1 sphere, 2 box, 3 cone).
    pub(crate) emitter: Vec4,
    /// xyz: the emitter's centre at the previous step.
    pub(crate) previous: Vec4,
    /// xyz: the unit direction; w: the spread of the directions (the cone's angle for a cone).
    pub(crate) direction: Vec4,
    /// Sphere: x radius; box: xyz half extents; cone: x radius.
    pub(crate) shape: Vec4,
    /// Speed min, max; lifetime min, max.
    pub(crate) speed_life: Vec4,
    /// xyz: acceleration; w: drag.
    pub(crate) forces: Vec4,
    /// Size at birth, at death; fade-out; intensity.
    pub(crate) size: Vec4,
    /// rgb: colour at birth; a: opacity at birth.
    pub(crate) birth: Vec4,
    /// rgb: colour at death; a: opacity at death.
    pub(crate) death: Vec4,
    /// x: the step in seconds.
    pub(crate) step: Vec4,
    /// The first slot spawned into, the count spawned, the index of the first spawned particle
    /// (counted from creation, wrapping), the seed.
    pub(crate) spawn: UVec4,
    /// x: the capacity.
    pub(crate) counts: UVec4,
}

const PARAMS_SIZE: u64 = 192;
const _: () = assert!(std::mem::size_of::<ParticleParams>() == PARAMS_SIZE as usize);
// SAFETY: #[repr(C)]; ten Vec4 and two UVec4 (16 bytes each, 4-byte aligned), no padding (size
// asserted above); any bits are valid.
unsafe impl bytemuck::Zeroable for ParticleParams {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for ParticleParams {}

/// A GPU PARTICLE SYSTEM (Not in C#): a fixed number of particles simulated on the GPU - a
/// compute step per frame ([`ParticleSystem::update`]) - and drawn as camera-facing sprites with
/// the glow layer's drawing ([`ParticleSystem::draw`]): round and soft, or the shape of
/// [`ParticleSystem::set_texture`]. Made by [`GraphicsContext::create_particle_system`].
///
/// - The EMITTER ([`ParticleSystem::emitter`]): a shape ([`EmitterShape`]: point, sphere, box,
///   cone), a rate per second and bursts ([`ParticleSystem::burst`]), a lifetime range, a
///   starting speed range and direction with a spread. It may move every frame, and follow a node
///   ([`ParticleEmitter::place`]).
/// - FORCES and the LOOK OVER LIFE ([`ParticleSystem::settings`]): constant acceleration and
///   drag; size, colour and opacity at birth and death, and a fade-out at the end of the life.
/// - DRAWING ([`ParticleSystemOptions::blend`]): additive (order-independent) or ordinary
///   transparency; depth-tested against solid things, never writing depth. Draw after the opaque
///   parts and the skybox, like the glow layer.
/// - THE CLOCK: `update` takes the frame's DELTA (`frame.info.delta`), so the canvas clock drives
///   it - a paused clock (delta 0) freezes the particles (nothing moves, nothing is spawned, no
///   GPU work), the clock's speed scales them.
/// - DETERMINISTIC: the random numbers are an integer hash of [`ParticleSystemOptions::seed`] and
///   the particle's number, computed on the GPU, so the same seed and the same sequence of frame
///   deltas (and emitter changes and bursts) give the same particles.
///
/// LIMITS: transparent particles are NOT sorted (where they overlap, the order is the buffer's);
/// there are NO COLLISIONS (particles pass through surfaces; the depth test hides them behind
/// solid things); SEEKING THE CLOCK DOES NOT REWIND THEM (the frame after a seek has delta 0: a
/// still at `?t=12` shows the particles as they are just after loading - empty, unless the
/// application bursts or runs steps itself). When more than the capacity are alive, the oldest
/// emitted are replaced. One `update` per frame (a second one in the same frame uses the second
/// one's settings for both). Every frame draws the whole capacity (dead particles cost a vertex
/// stage each and no pixels): choose the capacity to fit the rate times the longest life.
pub struct ParticleSystem {
    context: GraphicsContext,
    drawing: GlowDrawing,
    step: GpuComputePipeline,
    group: GpuBindGroup,
    params: GpuBuffer,
    particles: GpuBuffer,
    instances: GpuBuffer,
    capacity: u32,
    seed: u32,
    blend: GlowBlend,
    max_step: f32,
    cursor: u32,
    spawned: u32,
    carried: f64,
    pending_burst: u32,
    previous_position: Option<Vec3>,
    time: f64,
    texture: Option<GpuTexture>,
    /// The steps whose particles may still live (the live-particle bound; Not in C#).
    batches: std::collections::VecDeque<SpawnBatch>,
    /// The emitter; change it any time before an `update`.
    pub emitter: ParticleEmitter,
    /// The forces and the look over life; change them any time (they apply from the next
    /// `update`).
    pub settings: ParticleSettings,
}

impl ParticleSystem {
    /// The most particles a system may hold.
    pub const MAX_CAPACITY: u32 = 1 << 20;

    /// A particle system with the default emitter and settings.
    pub fn new(context: &GraphicsContext, options: &ParticleSystemOptions) -> Result<ParticleSystem, GraphicsException> {
        check_capacity(options.capacity)?;
        let label = options.label.clone().unwrap_or_else(|| "particles".to_string());
        let compute = wgpu::ShaderStages::COMPUTE;
        let shader = context.create_shader(EffectShaders::PARTICLES, Some(&label));
        let layout = context.create_bind_group_layout(Some(&label), &[BindingLayoutEntry::uniform(0, compute).with_min_binding_size(PARAMS_SIZE), BindingLayoutEntry::storage(1, compute, false), BindingLayoutEntry::storage(2, compute, false)]);
        let step = context.create_compute_pipeline(&shader).with_entry("step_particles").with_bind_group_layouts(&[&layout]).with_label(&label).build()?;
        let params = context.create_buffer(PARAMS_SIZE, wgpu::BufferUsages::UNIFORM, Some(&label))?;
        let particles = context.create_buffer(options.capacity as u64 * PARTICLE_SIZE, wgpu::BufferUsages::STORAGE, Some(&label))?;
        let instances = context.create_buffer(options.capacity as u64 * INSTANCE_SIZE, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX, Some(&label))?;
        let group = context.create_bind_group(&layout, Some(&label), &[BindingEntry::for_buffer(0, &params), BindingEntry::for_buffer(1, &particles), BindingEntry::for_buffer(2, &instances)]);
        let drawing = GlowDrawing::new(context, &label)?;
        Ok(ParticleSystem {
            context: context.clone(),
            drawing,
            step,
            group,
            params,
            particles,
            instances,
            capacity: options.capacity,
            seed: options.seed,
            blend: options.blend,
            max_step: options.max_step,
            cursor: 0,
            spawned: 0,
            carried: 0.0,
            pending_burst: 0,
            previous_position: None,
            time: 0.0,
            texture: None,
            batches: std::collections::VecDeque::new(),
            emitter: ParticleEmitter::default(),
            settings: ParticleSettings::default(),
        })
    }

    /// The most particles alive at once.
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// How particles are drawn.
    pub fn blend(&self) -> GlowBlend {
        self.blend
    }

    /// The particles spawned since creation or the last reset (wrapping at 2^32).
    pub fn spawned_count(&self) -> u32 {
        self.spawned
    }

    /// AT MOST how many particles are alive now (Not in C#): those spawned by the steps whose
    /// particles may still live (a step's particles are counted until its end plus the longest
    /// life the emitter gave them), never more than the capacity. An UPPER BOUND worked out on the
    /// CPU without reading the GPU back - exact when `lifetime_min == lifetime_max` (up to the
    /// fraction of a step a particle is born before the step's end); with a lifetime range, some
    /// counted particles may already have died. Follows the simulated time (a paused clock keeps
    /// it; `reset` makes it 0).
    pub fn alive_at_most(&self) -> u32 {
        alive_at_most(&self.batches, self.time, self.capacity)
    }

    /// The simulated seconds since creation or the last reset (the sum of the steps).
    pub fn simulated_time(&self) -> f64 {
        self.time
    }

    /// The context the system draws with.
    pub fn context(&self) -> &GraphicsContext {
        &self.context
    }

    /// The shape texture of the particles (None: the round soft falloff).
    pub fn texture(&self) -> Option<&GpuTexture> {
        self.texture.as_ref()
    }

    /// Gives every particle the shape of `texture` (its colour and straight alpha, times the
    /// particle's colour), as [`crate::GlowLayer::set_texture`]; None goes back to the round
    /// falloff.
    pub fn set_texture(&mut self, texture: Option<GpuTexture>) {
        self.texture = texture;
    }

    /// Emits `count` particles at the next step that moves the clock (on top of the rate; a
    /// paused clock keeps them waiting).
    pub fn burst(&mut self, count: u32) {
        self.pending_burst = self.pending_burst.saturating_add(count);
    }

    /// Kills every particle and starts over as if just created (the same seed gives the same
    /// particles again). Records a clear of the system's buffers into `frame`.
    pub fn reset(&mut self, frame: &mut GraphicsFrame<'_>) -> Result<(), GraphicsException> {
        frame.clear_buffer(&self.particles, 0, None)?;
        frame.clear_buffer(&self.instances, 0, None)?;
        self.cursor = 0;
        self.spawned = 0;
        self.carried = 0.0;
        self.pending_burst = 0;
        self.previous_position = None;
        self.time = 0.0;
        self.batches.clear();
        Ok(())
    }

    /// Runs one simulation step of `delta` clock seconds (the frame's `frame.info.delta`) in a
    /// compute pass of its own on `frame`, outside any render pass and before the pass that draws
    /// the particles: spawns what the rate and any burst call for, then moves and ages every
    /// particle. A delta of 0 (a paused clock, the first frame, the frame after a seek) records
    /// nothing.
    pub fn update(&mut self, frame: &mut GraphicsFrame<'_>, delta: f64) -> Result<(), GraphicsException> {
        let dt = step_seconds(delta, self.max_step);
        if dt <= 0.0 {
            return Ok(());
        }
        let (count, carried, pending) = spawn_count(self.carried, self.emitter.rate, dt, self.pending_burst, self.capacity);
        self.carried = carried;
        self.pending_burst = pending;
        let params = self.params(dt, count);
        self.params.write_value(&params, 0)?;
        {
            let mut pass = frame.begin_compute_pass(&ComputePassOptions { label: Some("particles".to_string()), timestamps: None });
            pass.set_pipeline(&self.step);
            pass.set_bind_group(0, &self.group);
            pass.dispatch(self.capacity.div_ceil(WORKGROUP), 1, 1);
        }
        self.cursor = ((self.cursor as u64 + count as u64) % self.capacity as u64) as u32;
        self.spawned = self.spawned.wrapping_add(count);
        self.previous_position = Some(self.emitter.position);
        self.time += dt as f64;
        if count > 0 {
            let longest_life = self.emitter.lifetime_max.max(self.emitter.lifetime_min).max(1e-3);
            self.batches.push_back(SpawnBatch { end: self.time, count, longest_life });
        }
        prune_batches(&mut self.batches, self.time);
        Ok(())
    }

    /// Draws the particles into `pass` as seen by `camera`: one instanced draw of the whole
    /// capacity (dead particles draw nothing), depth-tested, never writing depth.
    pub fn draw(&mut self, pass: &mut RenderPass<'_>, camera: &Camera) -> Result<(), GraphicsException> {
        self.drawing.draw(pass, camera, 0.0, &self.instances, 0, self.capacity, self.blend, self.texture.as_ref())
    }

    /// The step shader's uniforms for a step of `dt` seconds spawning `count`.
    pub(crate) fn params(&self, dt: f32, count: u32) -> ParticleParams {
        let emitter = &self.emitter;
        let s = &self.settings;
        let (kind, shape, spread) = match emitter.shape {
            EmitterShape::Point => (0.0, Vec4::ZERO, emitter.spread),
            EmitterShape::Sphere { radius } => (1.0, Vec4::new(radius.max(0.0), 0.0, 0.0, 0.0), emitter.spread),
            EmitterShape::Box { half_extents } => (2.0, half_extents.max(Vec3::ZERO).extend(0.0), emitter.spread),
            EmitterShape::Cone { radius, angle } => (3.0, Vec4::new(radius.max(0.0), 0.0, 0.0, 0.0), angle),
        };
        let previous = self.previous_position.unwrap_or(emitter.position);
        ParticleParams {
            emitter: emitter.position.extend(kind),
            previous: previous.extend(0.0),
            direction: emitter.axis().extend(spread.clamp(0.0, std::f32::consts::PI)),
            shape,
            speed_life: Vec4::new(emitter.speed_min, emitter.speed_max.max(emitter.speed_min), emitter.lifetime_min.max(1e-3), emitter.lifetime_max.max(emitter.lifetime_min).max(1e-3)),
            forces: s.acceleration.extend(s.drag.max(0.0)),
            size: Vec4::new(s.size_at_birth.max(0.0), s.size_at_death.max(0.0), s.fade_out.clamp(0.0, 1.0), s.intensity.max(0.0)),
            birth: s.color_at_birth.max(Vec3::ZERO).extend(s.opacity_at_birth.clamp(0.0, 1.0)),
            death: s.color_at_death.max(Vec3::ZERO).extend(s.opacity_at_death.clamp(0.0, 1.0)),
            step: Vec4::new(dt, 0.0, 0.0, 0.0),
            spawn: UVec4::new(self.cursor, count, self.spawned, self.seed),
            counts: UVec4::new(self.capacity, 0, 0, 0),
        }
    }
}

/// The check of [`ParticleSystemOptions::capacity`].
pub(crate) fn check_capacity(capacity: u32) -> Result<(), GraphicsException> {
    if !(1..=ParticleSystem::MAX_CAPACITY).contains(&capacity) {
        return Err(GraphicsException::new(format!("A particle system holds 1 to {} particles; ParticleSystemOptions::capacity is {capacity}.", ParticleSystem::MAX_CAPACITY)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Quat, Vec3};

    use super::{check_capacity, ParticleEmitter, ParticleSystemOptions};

    #[test]
    fn the_options_have_defaults_and_the_capacity_is_checked() {
        let options = ParticleSystemOptions::default();
        assert_eq!((options.capacity, options.seed, options.max_step), (1024, 1, 0.1));
        assert!(check_capacity(1).is_ok());
        assert!(check_capacity(1 << 20).is_ok());
        assert!(check_capacity(0).unwrap_err().message().contains("1 to 1048576"));
        assert!(check_capacity((1 << 20) + 1).is_err());
    }

    #[test]
    fn an_emitter_follows_a_transform() {
        let mut emitter = ParticleEmitter::default();
        emitter.place(Mat4::from_rotation_translation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2), Vec3::new(1.0, 2.0, 3.0)));
        assert_eq!(emitter.position, Vec3::new(1.0, 2.0, 3.0));
        assert!((emitter.direction - Vec3::X).length() < 1e-5, "{}", emitter.direction);
        emitter.direction = Vec3::ZERO;
        assert_eq!(emitter.axis(), Vec3::Y);
    }
}
