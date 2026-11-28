//! [`ParticleSystem`]: particles simulated on the GPU, drawn with the glow layer's drawing, with
//! [`ParticleSystemOptions`], [`ParticleEmitter`], [`EmitterShape`], [`ParticleSettings`] and
//! [`ParticleStretch`] (Not in C#).

use glam::{Mat4, UVec4, Vec2, Vec3, Vec4};

use crate::context::{GraphicsContext, GraphicsException};
use crate::drawing3d::{Camera, SceneLighting};
use crate::effects::glow_drawing::{GlowBlend, GlowDrawing, GlowLook, GlowStream, INSTANCE_SIZE};
use crate::effects::particle_math::{alive_at_most, prune_batches, spawn_count, split_spawns, step_seconds, SpawnBatch, SpawnRange};
use crate::frames::{ComputePassOptions, GraphicsFrame, RenderPass};
use crate::pipelines::GpuComputePipeline;
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuTexture};
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

/// VELOCITY-STRETCHED SPRITES (Not in C#; [`ParticleSystem::set_stretch`], off by default): each
/// particle is drawn longer along its motion as seen on the screen - sparks become streaks, rain
/// becomes lines - and keeps its width. The length is the size plus the distance the particle
/// moves across the screen in `factor` seconds, never more than `max` sizes; a particle standing
/// still, or coming straight at the camera, stays round. The texture's horizontal runs along the
/// motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleStretch {
    /// The seconds of motion the sprite spans (0: no stretch). Default 0.05.
    pub factor: f32,
    /// The longest a sprite gets, in sizes (at least 1). Default 4.
    pub max: f32,
}

impl Default for ParticleStretch {
    fn default() -> ParticleStretch {
        ParticleStretch { factor: 0.05, max: 4.0 }
    }
}

impl ParticleStretch {
    /// The shader's terms: the factor at least 0, the longest length at least 1 size (so a
    /// longest length above 0 means "on").
    pub(crate) fn shader_terms(self) -> Vec2 {
        let factor = if self.factor.is_finite() { self.factor.max(0.0) } else { 0.0 };
        let max = if self.max.is_finite() { self.max.max(1.0) } else { 1.0 };
        Vec2::new(factor, max)
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

/// The step shader's uniforms (112 bytes): what the emitters share.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ParticleParams {
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
    /// The first slot spawned into, the count spawned (by every emitter together), the index of
    /// the first spawned particle (counted from creation, wrapping), the seed.
    pub(crate) spawn: UVec4,
    /// x: the capacity; y: the emitter records of the step (the emitters that spawn in it).
    pub(crate) counts: UVec4,
}

const PARAMS_SIZE: u64 = 112;
const _: () = assert!(std::mem::size_of::<ParticleParams>() == PARAMS_SIZE as usize);
// SAFETY: #[repr(C)]; five Vec4 and two UVec4 (16 bytes each, 4-byte aligned), no padding (size
// asserted above); any bits are valid.
unsafe impl bytemuck::Zeroable for ParticleParams {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for ParticleParams {}

/// One emitter's record in the step shader's emitter buffer (96 bytes; the WGSL struct `Emitter`):
/// written for each emitter that spawns in a step, in emitter order.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EmitterRecord {
    /// xyz: the emitter's centre now; w: the shape (0 point, 1 sphere, 2 box, 3 cone).
    pub(crate) center: Vec4,
    /// xyz: the emitter's centre at the previous step.
    pub(crate) previous: Vec4,
    /// xyz: the unit direction; w: the spread of the directions (the cone's angle for a cone).
    pub(crate) direction: Vec4,
    /// Sphere: x radius; box: xyz half extents; cone: x radius.
    pub(crate) shape: Vec4,
    /// Speed min, max; lifetime min, max.
    pub(crate) speed_life: Vec4,
    /// x: the emitter's first order in the step's spawn range; y: its count.
    pub(crate) spawn: UVec4,
}

const EMITTER_RECORD_SIZE: u64 = 96;
const _: () = assert!(std::mem::size_of::<EmitterRecord>() == EMITTER_RECORD_SIZE as usize);
// SAFETY: #[repr(C)]; five Vec4 and a UVec4 (16 bytes each), no padding (size asserted above); any
// bits are valid.
unsafe impl bytemuck::Zeroable for EmitterRecord {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for EmitterRecord {}

impl EmitterRecord {
    /// The record of `emitter`, whose centre at the previous step was `previous` (None: where it is
    /// now), spawning `count` particles from order `first` of the step's range.
    pub(crate) fn new(emitter: &ParticleEmitter, previous: Option<Vec3>, first: u32, count: u32) -> EmitterRecord {
        let (kind, shape, spread) = match emitter.shape {
            EmitterShape::Point => (0.0, Vec4::ZERO, emitter.spread),
            EmitterShape::Sphere { radius } => (1.0, Vec4::new(radius.max(0.0), 0.0, 0.0, 0.0), emitter.spread),
            EmitterShape::Box { half_extents } => (2.0, half_extents.max(Vec3::ZERO).extend(0.0), emitter.spread),
            EmitterShape::Cone { radius, angle } => (3.0, Vec4::new(radius.max(0.0), 0.0, 0.0, 0.0), angle),
        };
        EmitterRecord {
            center: emitter.position.extend(kind),
            previous: previous.unwrap_or(emitter.position).extend(0.0),
            direction: emitter.axis().extend(spread.clamp(0.0, std::f32::consts::PI)),
            shape,
            speed_life: Vec4::new(emitter.speed_min, emitter.speed_max.max(emitter.speed_min), emitter.lifetime_min.max(1e-3), emitter.lifetime_max.max(emitter.lifetime_min).max(1e-3)),
            spawn: UVec4::new(first, count, 0, 0),
        }
    }
}

/// What the CPU keeps per emitter between steps: the fraction of a particle carried to the next
/// step, the burst waiting for the next step that moves the clock, and the centre at the last step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EmitterState {
    carried: f64,
    pending_burst: u32,
    previous_position: Option<Vec3>,
}

/// A GPU PARTICLE SYSTEM (Not in C#): a fixed number of particles simulated on the GPU - a
/// compute step per frame ([`ParticleSystem::update`]) - and drawn as camera-facing sprites with
/// the glow layer's drawing ([`ParticleSystem::draw`]): round and soft, or the shape of
/// [`ParticleSystem::set_texture`]. Made by [`GraphicsContext::create_particle_system`].
///
/// - The EMITTER ([`ParticleSystem::emitter`]): a shape ([`EmitterShape`]: point, sphere, box,
///   cone), a rate per second and bursts ([`ParticleSystem::burst`]), a lifetime range, a
///   starting speed range and direction with a spread. It may move every frame, and follow a node
///   ([`ParticleEmitter::place`]).
/// - MORE EMITTERS ([`ParticleSystem::add_emitter`], up to [`ParticleSystem::MAX_EMITTERS`]
///   together): one system - one compute step and one draw - for many sources of the same kind of
///   particle (sparks from many places). Each has its own place, shape, rate, bursts
///   ([`ParticleSystem::burst_from`]), speeds and lives, and leaves its own trail when it moves;
///   the forces, the look over life, the capacity and the drawing are the system's, shared. A
///   system with only its first emitter behaves exactly as one always has.
/// - FORCES and the LOOK OVER LIFE ([`ParticleSystem::settings`]): constant acceleration and
///   drag; size, colour and opacity at birth and death, and a fade-out at the end of the life.
/// - DRAWING ([`ParticleSystemOptions::blend`]): additive (order-independent) or ordinary
///   transparency; depth-tested against solid things, never writing depth. Draw after the opaque
///   parts and the skybox, like the glow layer. Opt-in: LIT particles
///   ([`ParticleSystem::draw_lit`]: smoke and dust in the scene's light) and sprites STRETCHED
///   along their motion ([`ParticleSystem::set_stretch`]).
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
/// emitted are replaced; when the emitters together spawn more than the capacity in one step, the
/// later emitters' share is cut. One `update` per frame (a second one in the same frame uses the second
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
    layout: GpuBindGroupLayout,
    /// The emitter records of a step (grown when more emitters spawn in one step).
    emitter_records: GpuBuffer,
    emitter_record_capacity: u64,
    capacity: u32,
    seed: u32,
    blend: GlowBlend,
    max_step: f32,
    cursor: u32,
    spawned: u32,
    /// Per emitter (the first emitter's at 0).
    states: Vec<EmitterState>,
    /// The emitters after the first.
    more_emitters: Vec<ParticleEmitter>,
    stretch: Option<ParticleStretch>,
    time: f64,
    texture: Option<GpuTexture>,
    /// The steps' spawns, per emitter, whose particles may still live (the live-particle bound;
    /// Not in C#).
    batches: std::collections::VecDeque<SpawnBatch>,
    /// The emitter (emitter 0 of [`ParticleSystem::emitter_at`]); change it any time before an
    /// `update`.
    pub emitter: ParticleEmitter,
    /// The forces and the look over life; change them any time (they apply from the next
    /// `update`).
    pub settings: ParticleSettings,
}

impl ParticleSystem {
    /// The most particles a system may hold.
    pub const MAX_CAPACITY: u32 = 1 << 20;

    /// The most emitters a system may have, its first emitter included (Not in C#). The emitters
    /// that spawn in a step are written to a storage buffer the compute step reads (96 bytes each;
    /// the buffer grows to the number that spawn together), and a spawned particle finds its
    /// emitter by a binary search of their ranges: at this cap at most 8 reads, and at most 24 KB
    /// written in a step.
    pub const MAX_EMITTERS: usize = 256;

    /// A particle system with the default emitter and settings.
    pub fn new(context: &GraphicsContext, options: &ParticleSystemOptions) -> Result<ParticleSystem, GraphicsException> {
        check_capacity(options.capacity)?;
        let label = options.label.clone().unwrap_or_else(|| "particles".to_string());
        let compute = wgpu::ShaderStages::COMPUTE;
        let shader = context.create_shader(EffectShaders::PARTICLES, Some(&label));
        let layout = context.create_bind_group_layout(
            Some(&label),
            &[BindingLayoutEntry::uniform(0, compute).with_min_binding_size(PARAMS_SIZE), BindingLayoutEntry::storage(1, compute, false), BindingLayoutEntry::storage(2, compute, false), BindingLayoutEntry::storage(3, compute, true).with_min_binding_size(EMITTER_RECORD_SIZE)],
        );
        let step = context.create_compute_pipeline(&shader).with_entry("step_particles").with_bind_group_layouts(&[&layout]).with_label(&label).build()?;
        let params = context.create_buffer(PARAMS_SIZE, wgpu::BufferUsages::UNIFORM, Some(&label))?;
        let particles = context.create_buffer(options.capacity as u64 * PARTICLE_SIZE, wgpu::BufferUsages::STORAGE, Some(&label))?;
        let instances = context.create_buffer(options.capacity as u64 * INSTANCE_SIZE, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX, Some(&label))?;
        let emitter_records = context.create_buffer(EMITTER_RECORD_SIZE, wgpu::BufferUsages::STORAGE, Some(&label))?;
        let group = step_group(context, &layout, &label, &params, &particles, &instances, &emitter_records);
        let drawing = GlowDrawing::new(context, &label)?;
        Ok(ParticleSystem {
            context: context.clone(),
            drawing,
            step,
            group,
            params,
            particles,
            instances,
            layout,
            emitter_records,
            emitter_record_capacity: 1,
            capacity: options.capacity,
            seed: options.seed,
            blend: options.blend,
            max_step: options.max_step,
            cursor: 0,
            spawned: 0,
            states: vec![EmitterState::default()],
            more_emitters: Vec::new(),
            stretch: None,
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

    /// Emits `count` particles from the first emitter at the next step that moves the clock (on
    /// top of the rate; a paused clock keeps them waiting).
    pub fn burst(&mut self, count: u32) {
        self.states[0].pending_burst = self.states[0].pending_burst.saturating_add(count);
    }

    /// The emitters, the first one ([`ParticleSystem::emitter`]) included: 1 or more (Not in C#).
    pub fn emitter_count(&self) -> usize {
        1 + self.more_emitters.len()
    }

    /// Adds an emitter and returns its index (Not in C#): it spawns from the next `update` (its
    /// first step's trail starts where it is), with the system's settings, capacity and drawing.
    /// An error when the system already has [`ParticleSystem::MAX_EMITTERS`].
    pub fn add_emitter(&mut self, emitter: ParticleEmitter) -> Result<usize, GraphicsException> {
        if self.emitter_count() >= ParticleSystem::MAX_EMITTERS {
            return Err(GraphicsException::new(format!("A particle system has at most {} emitters; remove one before adding another, or use a second system.", ParticleSystem::MAX_EMITTERS)));
        }
        self.more_emitters.push(emitter);
        self.states.push(EmitterState::default());
        Ok(self.emitter_count() - 1)
    }

    /// The emitter at `index` (0 is [`ParticleSystem::emitter`]), or None past the last (Not in
    /// C#).
    pub fn emitter_at(&self, index: usize) -> Option<&ParticleEmitter> {
        match index {
            0 => Some(&self.emitter),
            _ => self.more_emitters.get(index - 1),
        }
    }

    /// The emitter at `index` to change (0 is [`ParticleSystem::emitter`]), or None past the last
    /// (Not in C#). Changes apply from the next `update`.
    pub fn emitter_at_mut(&mut self, index: usize) -> Option<&mut ParticleEmitter> {
        match index {
            0 => Some(&mut self.emitter),
            _ => self.more_emitters.get_mut(index - 1),
        }
    }

    /// Removes the emitter at `index` and returns it (Not in C#); the emitters after it move down
    /// one index. Its particles alive now live out their lives; its waiting burst is dropped. An
    /// error for index 0 (the first emitter stays: set its rate to 0 to quiet it) or past the last.
    pub fn remove_emitter(&mut self, index: usize) -> Result<ParticleEmitter, GraphicsException> {
        if index == 0 || index >= self.emitter_count() {
            return Err(GraphicsException::new(format!("A particle system's emitter {index} cannot be removed: the system has emitters 0 to {}, and emitter 0 always stays (set its rate to 0 to quiet it).", self.emitter_count() - 1)));
        }
        self.states.remove(index);
        Ok(self.more_emitters.remove(index - 1))
    }

    /// Emits `count` particles from the emitter at `index` at the next step that moves the clock
    /// (on top of its rate), as [`ParticleSystem::burst`] does for emitter 0 (Not in C#). An error
    /// past the last emitter.
    pub fn burst_from(&mut self, index: usize, count: u32) -> Result<(), GraphicsException> {
        let emitters = self.emitter_count();
        let Some(state) = self.states.get_mut(index) else {
            return Err(GraphicsException::new(format!("A particle system's burst_from was given emitter {index}; the system has emitters 0 to {}.", emitters - 1)));
        };
        state.pending_burst = state.pending_burst.saturating_add(count);
        Ok(())
    }

    /// The velocity stretch (Not in C#; None, the default: round sprites).
    pub fn stretch(&self) -> Option<ParticleStretch> {
        self.stretch
    }

    /// Draws every particle stretched along its motion ([`ParticleStretch`]); None goes back to
    /// round sprites (Not in C#). Applies from the next draw.
    pub fn set_stretch(&mut self, stretch: Option<ParticleStretch>) {
        self.stretch = stretch;
    }

    /// Kills every particle and starts over as if just created (the same seed gives the same
    /// particles again; the emitters are kept, their waiting bursts dropped). Records a clear of
    /// the system's buffers into `frame`.
    pub fn reset(&mut self, frame: &mut GraphicsFrame<'_>) -> Result<(), GraphicsException> {
        frame.clear_buffer(&self.particles, 0, None)?;
        frame.clear_buffer(&self.instances, 0, None)?;
        self.cursor = 0;
        self.spawned = 0;
        for state in &mut self.states {
            *state = EmitterState::default();
        }
        self.time = 0.0;
        self.batches.clear();
        Ok(())
    }

    /// Runs one simulation step of `delta` clock seconds (the frame's `frame.info.delta`) in a
    /// compute pass of its own on `frame`, outside any render pass and before the pass that draws
    /// the particles: spawns what each emitter's rate and any burst call for, then moves and ages
    /// every particle. A delta of 0 (a paused clock, the first frame, the frame after a seek)
    /// records nothing.
    pub fn update(&mut self, frame: &mut GraphicsFrame<'_>, delta: f64) -> Result<(), GraphicsException> {
        let dt = step_seconds(delta, self.max_step);
        if dt <= 0.0 {
            return Ok(());
        }
        let emitters: Vec<&ParticleEmitter> = std::iter::once(&self.emitter).chain(self.more_emitters.iter()).collect();
        let (ranges, records) = plan_step(&emitters, &mut self.states, dt, self.capacity);
        let batches = spawn_batches(&emitters, &ranges);
        let count: u32 = ranges.iter().map(|range| range.count).sum();
        if records.len() as u64 > self.emitter_record_capacity {
            // The old buffer stays alive for work already recorded (the web backend's Drop does
            // nothing).
            let capacity = (records.len() as u64).next_power_of_two();
            self.emitter_records = self.context.create_buffer(capacity * EMITTER_RECORD_SIZE, wgpu::BufferUsages::STORAGE, Some("particle emitters"))?;
            self.emitter_record_capacity = capacity;
            self.group = step_group(&self.context, &self.layout, "particles", &self.params, &self.particles, &self.instances, &self.emitter_records);
        }
        self.emitter_records.write(&records, 0)?;
        let params = self.params(dt, count, records.len() as u32);
        self.params.write_value(&params, 0)?;
        {
            let mut pass = frame.begin_compute_pass(&ComputePassOptions { label: Some("particles".to_string()), timestamps: None });
            pass.set_pipeline(&self.step);
            pass.set_bind_group(0, &self.group);
            pass.dispatch(self.capacity.div_ceil(WORKGROUP), 1, 1);
        }
        self.cursor = ((self.cursor as u64 + count as u64) % self.capacity as u64) as u32;
        self.spawned = self.spawned.wrapping_add(count);
        self.time += dt as f64;
        for (count, longest_life) in batches {
            self.batches.push_back(SpawnBatch { end: self.time, count, longest_life });
        }
        prune_batches(&mut self.batches, self.time);
        Ok(())
    }

    /// Draws the particles into `pass` as seen by `camera`: one instanced draw of the whole
    /// capacity (dead particles draw nothing), depth-tested, never writing depth; stretched when
    /// [`ParticleSystem::set_stretch`] says so.
    pub fn draw(&mut self, pass: &mut RenderPass<'_>, camera: &Camera) -> Result<(), GraphicsException> {
        let look = GlowLook { particles: true, stretch: self.stretch_terms(), ..GlowLook::plain(self.blend, self.texture.as_ref()) };
        self.drawing.draw(pass, camera, 0.0, GlowStream { instances: &self.instances, offset: 0, count: self.capacity, axes: None }, &look)
    }

    /// Draws the particles as [`ParticleSystem::draw`] does, LIT by `lighting` (Not in C#): each
    /// particle's colour times the ambient plus the key light times a half-Lambert term
    /// (`(n.l / 2 + 1/2)^2`, so the side away from the light is not black) on a normal facing the
    /// camera from the particle. The AMBIENT is the model renderer's hemisphere under
    /// [`crate::AmbientLighting::Hemisphere`] (sky [`SceneLighting::ambient_color`], ground
    /// [`SceneLighting::ambient_ground_color`], around [`SceneLighting::ambient_up`]) and the flat
    /// [`SceneLighting::ambient_color`] under [`crate::AmbientLighting::Environment`] (as the mesh
    /// renderer uses it: an environment's light is not read for particles). The KEY LIGHT is
    /// [`SceneLighting::light_direction`], `light_color` times `light_intensity`. The EXPOSURE
    /// ([`SceneLighting::exposure`]) scales the whole lit colour, as the model renderer scales its
    /// output, so lit particles beside models take the same exposure. The additional lights, local
    /// lights and shadows are not applied. Its own pipeline per target format combination, made at
    /// the first lit draw.
    pub fn draw_lit(&mut self, pass: &mut RenderPass<'_>, camera: &Camera, lighting: &SceneLighting) -> Result<(), GraphicsException> {
        let look = GlowLook { particles: true, stretch: self.stretch_terms(), lighting: Some(lighting), ..GlowLook::plain(self.blend, self.texture.as_ref()) };
        self.drawing.draw(pass, camera, 0.0, GlowStream { instances: &self.instances, offset: 0, count: self.capacity, axes: None }, &look)
    }

    /// The stretch as the glow shader reads it ((0, 0): off).
    fn stretch_terms(&self) -> Vec2 {
        self.stretch.map(ParticleStretch::shader_terms).unwrap_or(Vec2::ZERO)
    }

    /// The step shader's uniforms for a step of `dt` seconds spawning `count` from `records`
    /// emitter records.
    pub(crate) fn params(&self, dt: f32, count: u32, records: u32) -> ParticleParams {
        particle_params(&self.settings, dt, UVec4::new(self.cursor, count, self.spawned, self.seed), UVec4::new(self.capacity, records, 0, 0))
    }
}

/// The CPU half of a step of `dt` seconds for every emitter (`states[i]` is `emitters[i]`'s): each
/// emitter's count from its rate, its carried fraction and its waiting burst; the step's range
/// split across them ([`split_spawns`], at most `capacity`); the records of those that spawn,
/// moving from their centre at the last step; and then each emitter's centre kept for the next.
pub(crate) fn plan_step(emitters: &[&ParticleEmitter], states: &mut [EmitterState], dt: f32, capacity: u32) -> (Vec<SpawnRange>, Vec<EmitterRecord>) {
    let mut counts = Vec::with_capacity(emitters.len());
    for (emitter, state) in emitters.iter().zip(states.iter_mut()) {
        let (count, carried, pending) = spawn_count(state.carried, emitter.rate, dt, state.pending_burst, capacity);
        state.carried = carried;
        state.pending_burst = pending;
        counts.push(count);
    }
    let ranges = split_spawns(&counts, capacity);
    let records = ranges.iter().map(|range| EmitterRecord::new(emitters[range.emitter], states[range.emitter].previous_position, range.first, range.count)).collect();
    for (emitter, state) in emitters.iter().zip(states.iter_mut()) {
        state.previous_position = Some(emitter.position);
    }
    (ranges, records)
}

/// The live-particle bound's batches of a step, one per emitter that spawned: its count and the
/// longest life it gives.
pub(crate) fn spawn_batches(emitters: &[&ParticleEmitter], ranges: &[SpawnRange]) -> Vec<(u32, f32)> {
    ranges.iter().map(|range| (range.count, emitters[range.emitter].lifetime_max.max(emitters[range.emitter].lifetime_min).max(1e-3))).collect()
}

/// The step shader's uniforms from the settings, the step, the spawn word (first slot, count,
/// first particle index, seed) and the counts (capacity, emitter records).
pub(crate) fn particle_params(s: &ParticleSettings, dt: f32, spawn: UVec4, counts: UVec4) -> ParticleParams {
    ParticleParams {
        forces: s.acceleration.extend(s.drag.max(0.0)),
        size: Vec4::new(s.size_at_birth.max(0.0), s.size_at_death.max(0.0), s.fade_out.clamp(0.0, 1.0), s.intensity.max(0.0)),
        birth: s.color_at_birth.max(Vec3::ZERO).extend(s.opacity_at_birth.clamp(0.0, 1.0)),
        death: s.color_at_death.max(Vec3::ZERO).extend(s.opacity_at_death.clamp(0.0, 1.0)),
        step: Vec4::new(dt, 0.0, 0.0, 0.0),
        spawn,
        counts,
    }
}

/// The step's bind group: the uniforms, the particles, the instance records and the emitter
/// records.
fn step_group(context: &GraphicsContext, layout: &GpuBindGroupLayout, label: &str, params: &GpuBuffer, particles: &GpuBuffer, instances: &GpuBuffer, emitter_records: &GpuBuffer) -> GpuBindGroup {
    context.create_bind_group(layout, Some(label), &[BindingEntry::for_buffer(0, params), BindingEntry::for_buffer(1, particles), BindingEntry::for_buffer(2, instances), BindingEntry::for_buffer(3, emitter_records)])
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
    use glam::{Mat4, Quat, UVec4, Vec2, Vec3, Vec4};

    use super::{check_capacity, particle_params, plan_step, spawn_batches, EmitterRecord, EmitterShape, EmitterState, ParticleEmitter, ParticleParams, ParticleSettings, ParticleStretch, ParticleSystemOptions};
    use crate::effects::particle_math::{spawn_count, SpawnRange};

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

    #[test]
    fn an_emitter_packs_into_the_step_shaders_emitter_record() {
        let emitter = ParticleEmitter { shape: EmitterShape::Cone { radius: -1.0, angle: 0.4 }, position: Vec3::new(1.0, 2.0, 3.0), direction: Vec3::new(0.0, 0.0, 2.0), spread: 0.9, rate: 10.0, speed_min: 2.0, speed_max: 1.0, lifetime_min: 0.0, lifetime_max: 3.0 };
        let record = EmitterRecord::new(&emitter, Some(Vec3::new(0.0, 2.0, 3.0)), 5, 7);
        assert_eq!(record.center, Vec4::new(1.0, 2.0, 3.0, 3.0), "a cone is shape 3");
        assert_eq!(record.previous, Vec4::new(0.0, 2.0, 3.0, 0.0));
        assert_eq!(record.direction, Vec4::new(0.0, 0.0, 1.0, 0.4), "unit direction; a cone's angle is its spread");
        assert_eq!(record.shape, Vec4::ZERO, "a negative radius is 0");
        assert_eq!(record.speed_life, Vec4::new(2.0, 2.0, 1e-3, 3.0), "the ranges are kept in order");
        assert_eq!(record.spawn, UVec4::new(5, 7, 0, 0));
        let still = EmitterRecord::new(&ParticleEmitter { shape: EmitterShape::Box { half_extents: Vec3::new(1.0, -1.0, 2.0) }, ..ParticleEmitter::default() }, None, 0, 1);
        assert_eq!(still.previous, Vec4::ZERO, "no previous step: where it is now");
        assert_eq!((still.center.w, still.shape), (2.0, Vec4::new(1.0, 0.0, 2.0, 0.0)));
        assert_eq!(EmitterRecord::new(&ParticleEmitter { shape: EmitterShape::Sphere { radius: 0.5 }, ..ParticleEmitter::default() }, None, 0, 1).shape, Vec4::new(0.5, 0.0, 0.0, 0.0));
        assert_eq!(EmitterRecord::new(&ParticleEmitter::default(), None, 0, 1).direction.w, 0.3, "the spread");
        // The WGSL struct names the same fields in the same order.
        assert_eq!((std::mem::offset_of!(EmitterRecord, speed_life), std::mem::offset_of!(EmitterRecord, spawn)), (64, 80));
        assert_struct_order(crate::shaders::EffectShaders::PARTICLES, "struct Emitter {", &["center:", "previous:", "direction:", "shape:", "speed_life:", "spawn:"]);
    }

    #[test]
    fn the_shared_settings_pack_into_the_step_shaders_uniforms() {
        let settings = ParticleSettings { acceleration: Vec3::new(0.0, -9.81, 0.0), drag: -1.0, size_at_birth: 0.3, size_at_death: -0.1, fade_out: 2.0, intensity: 3.0, opacity_at_birth: 1.5, ..ParticleSettings::default() };
        let params = particle_params(&settings, 0.016, UVec4::new(10, 20, 30, 7), UVec4::new(1024, 3, 0, 0));
        assert_eq!(params.forces, Vec4::new(0.0, -9.81, 0.0, 0.0), "no negative drag");
        assert_eq!(params.size, Vec4::new(0.3, 0.0, 1.0, 3.0));
        assert_eq!(params.birth.w, 1.0);
        assert_eq!(params.step, Vec4::new(0.016, 0.0, 0.0, 0.0));
        assert_eq!((params.spawn, params.counts), (UVec4::new(10, 20, 30, 7), UVec4::new(1024, 3, 0, 0)));
        assert_eq!((std::mem::offset_of!(ParticleParams, spawn), std::mem::offset_of!(ParticleParams, counts)), (80, 96));
        assert_struct_order(crate::shaders::EffectShaders::PARTICLES, "struct Params {", &["forces:", "size:", "birth:", "death:", "step:", "spawn:", "counts:"]);
    }

    #[test]
    fn one_emitter_steps_exactly_as_a_system_always_has() {
        // The old step: spawn_count of the one emitter, its record at the whole range.
        let mut emitter = ParticleEmitter { rate: 30.0, ..ParticleEmitter::default() };
        let mut states = vec![EmitterState::default()];
        let (mut carried, mut previous) = (0.0f64, None);
        for step in 0..20 {
            emitter.position = Vec3::new(step as f32 * 0.1, 0.0, 0.0);
            let burst = if step == 5 { 7 } else { 0 };
            states[0].pending_burst += burst;
            let (count, rest, _) = spawn_count(carried, emitter.rate, 1.0 / 60.0, burst, 64);
            carried = rest;
            let (ranges, records) = plan_step(&[&emitter], &mut states, 1.0 / 60.0, 64);
            if count == 0 {
                assert!(ranges.is_empty() && records.is_empty());
            } else {
                assert_eq!(ranges, vec![SpawnRange { emitter: 0, first: 0, count }]);
                assert_eq!(records, vec![EmitterRecord::new(&emitter, previous, 0, count)]);
            }
            assert_eq!(states[0].carried, carried);
            previous = Some(emitter.position);
            assert_eq!(states[0].previous_position, previous);
        }
    }

    #[test]
    fn several_emitters_keep_their_own_fractions_bursts_and_trails() {
        let a = ParticleEmitter { rate: 30.0, position: Vec3::X, lifetime_max: 3.0, ..ParticleEmitter::default() };
        let b = ParticleEmitter { rate: 90.0, position: Vec3::Y, ..ParticleEmitter::default() };
        let quiet = ParticleEmitter { rate: 0.0, position: Vec3::Z, ..ParticleEmitter::default() };
        let mut states = vec![EmitterState::default(); 3];
        states[2].pending_burst = 4;
        let (ranges, records) = plan_step(&[&a, &b, &quiet], &mut states, 1.0 / 60.0, 100);
        // a: half a particle carried; b: one and a half, one spawned; the quiet one's burst.
        assert_eq!(ranges, vec![SpawnRange { emitter: 1, first: 0, count: 1 }, SpawnRange { emitter: 2, first: 1, count: 4 }]);
        assert!((states[0].carried - 0.5).abs() < 1e-6 && (states[1].carried - 0.5).abs() < 1e-6, "{:?}", states);
        assert_eq!(states[2].pending_burst, 0);
        assert_eq!((records[0].center.truncate(), records[0].spawn), (Vec3::Y, UVec4::new(0, 1, 0, 0)));
        assert_eq!((records[1].center.truncate(), records[1].spawn), (Vec3::Z, UVec4::new(1, 4, 0, 0)));
        assert_eq!(spawn_batches(&[&a, &b, &quiet], &ranges), vec![(1, 2.0), (4, 2.0)]);
        // Each keeps its own centre: moved, each trail starts where that emitter was.
        let moved = ParticleEmitter { position: Vec3::new(0.0, 5.0, 0.0), ..b };
        let (ranges, records) = plan_step(&[&a, &moved, &quiet], &mut states, 1.0 / 60.0, 100);
        assert_eq!(ranges, vec![SpawnRange { emitter: 0, first: 0, count: 1 }, SpawnRange { emitter: 1, first: 1, count: 2 }]);
        assert_eq!(records[0].previous.truncate(), Vec3::X);
        assert_eq!((records[1].previous.truncate(), records[1].center.truncate()), (Vec3::Y, Vec3::new(0.0, 5.0, 0.0)));
        assert_eq!(spawn_batches(&[&a, &moved, &quiet], &ranges), vec![(1, 3.0), (2, 2.0)], "each batch with its emitter's longest life");
        // A new emitter's first trail starts where it is.
        let mut states = vec![EmitterState::default()];
        let (_, records) = plan_step(&[&b], &mut states, 0.1, 100);
        assert_eq!(records[0].previous, records[0].center.truncate().extend(0.0));
    }

    #[test]
    fn the_stretch_is_off_by_default_and_its_terms_are_kept_in_range() {
        assert_eq!(ParticleStretch::default(), ParticleStretch { factor: 0.05, max: 4.0 });
        assert_eq!(ParticleStretch::default().shader_terms(), Vec2::new(0.05, 4.0));
        assert_eq!(ParticleStretch { factor: -1.0, max: 0.5 }.shader_terms(), Vec2::new(0.0, 1.0), "on, but never shorter than round");
        assert_eq!(ParticleStretch { factor: f32::NAN, max: f32::INFINITY }.shader_terms(), Vec2::new(0.0, 1.0));
    }

    fn assert_struct_order(wgsl: &str, header: &str, order: &[&str]) {
        let start = wgsl.find(header).expect("the struct");
        let body = &wgsl[start..start + wgsl[start..].find("};").expect("its end")];
        let mut at = 0;
        for name in order {
            let found = body[at..].find(&format!("    {name}")).unwrap_or_else(|| panic!("{name} in order"));
            at += found + name.len();
        }
    }
}
