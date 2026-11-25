//! [`WaveField`]: a 2D grid of water heights stepped on the GPU by the wave equation, with
//! [`WaveFieldOptions`], [`WaveEdge`], [`WaveDrop`], [`WaveWake`] and [`WaveStir`] (Not in C#).
//! The arithmetic is in `wave_math.rs` (the CPU twin, also public as [`crate::CpuWaveField`]),
//! the shader in `shaders/wgsl/wave_field.wgsl`.

use codebrix_webgpu::CanvasInput;
use glam::Vec2;

use crate::context::{GraphicsContext, GraphicsException, Texture2DOptions};
use crate::frames::{ComputePassOptions, GraphicsFrame};
use crate::pipelines::GpuComputePipeline;
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuTexture};
use crate::shaders::WaterShaders;
use crate::water::wave_mask::WaveMask;
use crate::water::wave_math::{WaveCore, WaveParams, PARAMS_SIZE};
use crate::water::wave_rain::WaveRain;

/// The wave shader's workgroup size on each axis (8 x 8 cells per workgroup).
const WORKGROUP: u32 = 8;

/// What a wave field's edges do (Not in C#).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum WaveEdge {
    /// A wall: waves bounce back with the same sign (the missing neighbour of an edge cell is the
    /// cell itself). The default.
    #[default]
    Reflect,
    /// Open water: a band of `width` cells along every edge damps waves away (a sponge), so
    /// little comes back (a few per cent of the energy). 12 is a good width; it is kept between 1
    /// and half the smaller side of the grid.
    Absorb {
        /// The band's width in cells.
        width: u32,
    },
    /// The field repeats: what leaves on one side comes in on the other (a tile of open water).
    Wrap,
}

impl WaveEdge {
    /// [`WaveEdge::Absorb`] with the recommended band of 12 cells.
    pub const ABSORB: WaveEdge = WaveEdge::Absorb { width: 12 };
}

/// The settings of a [`WaveField`] (Not in C#). The grid (`columns`, `rows`), `size` and the
/// rates are fixed when the field is made; speed, damping and the edge may change later
/// ([`WaveField::set_wave_speed`], [`WaveField::set_damping`], [`WaveField::set_edge`]).
#[derive(Clone, Debug, PartialEq)]
pub struct WaveFieldOptions {
    /// Cells across, 8 to 2048. Default 256.
    pub columns: u32,
    /// Cells down, 8 to 2048. Default 256.
    pub rows: u32,
    /// The field's extent in field units - the scene units of whatever shows it (x across, y
    /// down the rows; on a water surface y runs along +Z). Default (10, 10).
    pub size: Vec2,
    /// How fast waves travel, field units per second. Default 1.
    pub wave_speed: f32,
    /// The fraction of a wave's height left after ONE SECOND, 0 to 1 (1: no damping). Default
    /// 0.6.
    pub damping: f32,
    /// What the edges do. Default [`WaveEdge::Reflect`].
    pub edge: WaveEdge,
    /// The simulation rate in CLOCK seconds (steps per second of the canvas clock, whatever the
    /// display's rate): 1 to 2000. Default 120.
    pub steps_per_second: f32,
    /// The most steps one update runs (1 to 64): the clock time of a longer frame beyond it is
    /// dropped, so a tab brought back does not run a minute of water at once. Default 8.
    pub max_steps_per_frame: u32,
    /// The label of the field's buffers and texture.
    pub label: Option<String>,
}

impl Default for WaveFieldOptions {
    fn default() -> WaveFieldOptions {
        WaveFieldOptions { columns: 256, rows: 256, size: Vec2::new(10.0, 10.0), wave_speed: 1.0, damping: 0.6, edge: WaveEdge::Reflect, steps_per_second: 120.0, max_steps_per_frame: 8, label: None }
    }
}

/// A drop pressed into the field once, at the next update that steps (Not in C#): a Gaussian
/// dent `depth * exp(-(d / radius)^2)` at `position`, whose volume is `depth * pi * radius^2`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveDrop {
    /// Where, in field units, (0, 0) at the field's first corner (column 0, row 0).
    pub position: Vec2,
    /// The Gaussian's radius, field units. Default 0.08.
    pub radius: f32,
    /// How far it presses down, field (height) units; negative raises. Default 0.02.
    pub depth: f32,
}

impl Default for WaveDrop {
    fn default() -> WaveDrop {
        WaveDrop { position: Vec2::ZERO, radius: 0.08, depth: 0.02 }
    }
}

impl WaveDrop {
    /// The default drop at `position` (field units).
    pub fn at(position: Vec2) -> WaveDrop {
        WaveDrop { position, ..WaveDrop::default() }
    }
}

/// Something MOVING through the water (Not in C#): every update that steps presses a trough
/// along the path it travelled since the last one, so a moving point leaves a continuous wake;
/// optionally it bobs, rocking the water under it. The trough along the path is `strength` deep
/// with a Gaussian cross-section of `radius`, the same whatever the frame rate (the path is pressed
/// piece by piece, and the pieces add up exactly); the faster it moves, the more water it pushes
/// per second. A wake that does not move (and does not bob) presses nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveWake {
    /// Where it is now, field units.
    pub position: Vec2,
    /// The trough's radius, field units. Default 0.15.
    pub radius: f32,
    /// The trough's depth along the path, field units. Default 0.05.
    pub strength: f32,
    /// The height of its own up-and-down, field units (0: none). Default 0.
    pub bob: f32,
    /// Bobs per second (of the field's simulated time, so a paused clock stops it). Default 1.2.
    pub bob_rate: f32,
}

impl Default for WaveWake {
    fn default() -> WaveWake {
        WaveWake { position: Vec2::ZERO, radius: 0.15, strength: 0.05, bob: 0.0, bob_rate: 1.2 }
    }
}

impl WaveWake {
    /// The default wake at `position` (field units).
    pub fn at(position: Vec2) -> WaveWake {
        WaveWake { position, ..WaveWake::default() }
    }
}

/// How the pointer stirs a field through [`WaveField::stir`] (Not in C#).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveStir {
    /// Pressed where the pointer goes down. Default radius 0.1, depth 0.04.
    pub drop: WaveDrop,
    /// Follows the pointer while it is down (its position is the pointer's). Default strength 0.08.
    pub wake: WaveWake,
}

impl Default for WaveStir {
    fn default() -> WaveStir {
        WaveStir { drop: WaveDrop { radius: 0.1, depth: 0.04, ..WaveDrop::default() }, wake: WaveWake { strength: 0.08, ..WaveWake::default() } }
    }
}

/// A WAVE FIELD (Not in C#): a 2D grid of water heights stepped on the GPU by the wave equation
/// (one leapfrog step per simulation tick), disturbed by drops ([`WaveField::drop`]), moving
/// wakes ([`WaveField::set_wake`]), seeded rain ([`WaveField::set_rain`]) and the pointer
/// ([`WaveField::stir`]), optionally walled by a mask of shores and islands
/// ([`WaveField::set_mask`]), and read by anything as ONE picture, [`WaveField::surface`]. Made
/// by [`GraphicsContext::create_wave_field`].
///
/// - THE PICTURE is the whole interface: an `Rgba16Float` texture of `columns x rows` texels - R
///   the height (field units, 0 = still water), G dh/dx and B dh/dy (per field unit), A the
///   Laplacian of the height (per field unit squared; positive in troughs, where waves focus
///   light). Texel (column, row) is the cell whose centre is at
///   `((column + 0.5) * cell.x, (row + 0.5) * cell.y)` in field units; row 0 is field y = 0. It is
///   filterable (sample it with a linear sampler and `textureSampleLevel`), and it is the same
///   texture every frame, rewritten by each update that moves the clock. The water surface, the
///   application's own shaders and (handed over as a plain `wgpu::TextureView`) the Animation
///   crate read the same texture.
/// - THE CLOCK: [`WaveField::update`] takes the frame's DELTA (`frame.info.delta`) and steps at a
///   FIXED RATE IN CLOCK SECONDS ([`WaveFieldOptions::steps_per_second`]): a paused clock freezes
///   the water, the clock's speed scales it, every display rate shows the same water, and the
///   picture is INTERPOLATED between the last two steps by the time left over, so slow motion
///   stays smooth. A long frame runs at most [`WaveFieldOptions::max_steps_per_frame`] steps; the
///   time beyond is dropped.
/// - IT CANNOT BE REWOUND: seeking the clock gives a delta of 0, and the field stays as it was -
///   going back to second 12 shows the water as it is now, not as it was at 12 (as with
///   particles). An application that wants water as an exact function of time uses the open-water
///   waves ([`crate::OpenWaterWaves`]) or the Animation crate's ripple field.
/// - DETERMINISTIC for the same options, rain seed, sequence of deltas and inputs, ON ONE GPU (two
///   GPUs may differ in the last bits of floating point; a damped linear equation does not
///   amplify that). [`crate::CpuWaveField`] runs the same arithmetic on the CPU.
///
/// LIMITS: a height field - no splashes, breaking waves, spray or flow (nothing streams around
/// objects); the waves are non-dispersive (a drop makes one clean ring, not a train of rings); one
/// height per cell; the grid is fixed in place; heights are read on the GPU (on the CPU only by
/// reading the texture back, a few frames late, with `GpuTexture::read_pixels`). ONE `update`
/// per frame: its uniform block is written once per frame.
pub struct WaveField {
    context: GraphicsContext,
    core: WaveCore,
    layout: GpuBindGroupLayout,
    disturb: GpuComputePipeline,
    step: GpuComputePipeline,
    publish: GpuComputePipeline,
    params: GpuBuffer,
    heights: [GpuBuffer; 2],
    mask: GpuBuffer,
    groups: [GpuBindGroup; 2],
    surface: GpuTexture,
    current: usize,
    last_frame: Option<u64>,
    label: String,
}

impl WaveField {
    /// The most drops pressed by one update (queued drops and rain together); more wait for the
    /// next update. The queue holds at most twice as many; drops beyond it are discarded.
    pub const MAX_DROPS_PER_UPDATE: usize = 32;
    /// The number of wake slots ([`WaveField::set_wake`]); [`WaveField::stir`] uses the last.
    pub const MAX_WAKES: usize = 8;
    /// The smallest and largest number of cells on a side.
    pub const GRID_RANGE: (u32, u32) = (8, 2048);

    /// A calm wave field. Err (with what to change) for a grid outside [`WaveField::GRID_RANGE`],
    /// a size that is not positive, a rate or step cap out of range, or a wave speed too fast for
    /// the rate (the message names the lowest `steps_per_second` that holds it).
    pub fn new(context: &GraphicsContext, options: &WaveFieldOptions) -> Result<WaveField, GraphicsException> {
        let core = WaveCore::new(options)?;
        let label = options.label.clone().unwrap_or_else(|| "wave field".to_string());
        let compute = wgpu::ShaderStages::COMPUTE;
        let shader = context.create_shader(WaterShaders::WAVE_FIELD, Some(&label));
        let layout = context.create_bind_group_layout(
            Some(&label),
            &[
                BindingLayoutEntry::uniform(0, compute).with_min_binding_size(PARAMS_SIZE),
                BindingLayoutEntry::storage(1, compute, false),
                BindingLayoutEntry::storage(2, compute, false),
                BindingLayoutEntry::storage(3, compute, true),
                BindingLayoutEntry::storage_texture(4, compute, wgpu::TextureFormat::Rgba16Float),
            ],
        );
        let pipeline = |entry: &str| context.create_compute_pipeline(&shader).with_entry(entry).with_bind_group_layouts(&[&layout]).with_label(&label).build();
        let (disturb, step, publish) = (pipeline("disturb_field")?, pipeline("step_field")?, pipeline("publish_field")?);
        let params = context.create_buffer(PARAMS_SIZE, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, Some(&label))?;
        let cells = options.columns as u64 * options.rows as u64 * 4;
        let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC;
        let heights = [context.create_buffer(cells, usage, Some(&label))?, context.create_buffer(cells, usage, Some(&label))?];
        let mask = context.create_buffer(4, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, Some(&label))?;
        let surface = context.create_texture_2d(
            options.columns,
            options.rows,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            Texture2DOptions { mip_level_count: 1, label: Some(label.clone()) },
        )?;
        let groups = WaveField::groups(context, &layout, &label, &params, &heights, &mask, &surface);
        Ok(WaveField { context: context.clone(), core, layout, disturb, step, publish, params, heights, mask, groups, surface, current: 0, last_frame: None, label })
    }

    /// The two bind groups: group `i` reads state `i` as the current heights and overwrites the
    /// other (the older state) with the next.
    fn groups(context: &GraphicsContext, layout: &GpuBindGroupLayout, label: &str, params: &GpuBuffer, heights: &[GpuBuffer; 2], mask: &GpuBuffer, surface: &GpuTexture) -> [GpuBindGroup; 2] {
        let group = |current: usize| {
            context.create_bind_group(
                layout,
                Some(label),
                &[BindingEntry::for_buffer(0, params), BindingEntry::for_buffer(1, &heights[current]), BindingEntry::for_buffer(2, &heights[1 - current]), BindingEntry::for_buffer(3, mask), BindingEntry::for_texture(4, surface)],
            )
        };
        [group(0), group(1)]
    }

    /// The options as they are now (speed, damping and edge as last set).
    pub fn options(&self) -> &WaveFieldOptions {
        self.core.options()
    }

    /// The field's extent in field units.
    pub fn size(&self) -> Vec2 {
        self.core.options().size
    }

    /// The size of one cell in field units (`size / (columns, rows)`).
    pub fn cell_size(&self) -> Vec2 {
        self.core.cell_size()
    }

    /// The context the field runs on.
    pub fn context(&self) -> &GraphicsContext {
        &self.context
    }

    /// Changes how fast waves travel. Err (and nothing changed) when the speed is negative or too
    /// fast for the rate: the message gives the highest speed the rate allows and the lowest
    /// `steps_per_second` that would hold this one.
    pub fn set_wave_speed(&mut self, units_per_second: f32) -> Result<(), GraphicsException> {
        self.core.set_wave_speed(units_per_second)
    }

    /// Changes the fraction of a wave's height left after one second (0 to 1). Err outside.
    pub fn set_damping(&mut self, kept_per_second: f32) -> Result<(), GraphicsException> {
        self.core.set_damping(kept_per_second)
    }

    /// Changes what the edges do (an absorbing band's width is kept between 1 and half the
    /// smaller side).
    pub fn set_edge(&mut self, edge: WaveEdge) {
        self.core.set_edge(edge);
    }

    /// Shores and islands ([`WaveMask`]): solid cells (1) hold the water at 0 and reflect waves
    /// exactly like a wall; values between 0 and 1 are shallows that damp waves (absorbing
    /// shores). None removes the mask. Uploaded once per call. Err when the mask's grid is not the
    /// field's.
    pub fn set_mask(&mut self, mask: Option<&WaveMask>) -> Result<(), GraphicsException> {
        self.core.set_mask(mask)?;
        self.mask = match mask {
            Some(mask) => self.context.create_storage_buffer_from(mask.cells(), Some(&self.label))?,
            None => self.context.create_buffer(4, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, Some(&self.label))?,
        };
        self.groups = WaveField::groups(&self.context, &self.layout, &self.label, &self.params, &self.heights, &self.mask, &self.surface);
        Ok(())
    }

    /// Queues a drop, pressed at the next update that steps (at most
    /// [`WaveField::MAX_DROPS_PER_UPDATE`] per update; the rest wait).
    pub fn drop(&mut self, drop: WaveDrop) {
        self.core.drop(drop);
    }

    /// Wake `slot` (0 to [`WaveField::MAX_WAKES`] - 1) follows `wake`; None stops it. Call it
    /// every frame with the moving thing's position. A wake that starts does not press a trough
    /// from where it last was. Err for a slot out of range.
    pub fn set_wake(&mut self, slot: usize, wake: Option<WaveWake>) -> Result<(), GraphicsException> {
        self.core.set_wake(slot, wake)
    }

    /// Rain: [`WaveRain::rate`] drops per simulated second at seeded places, sizes and depths;
    /// None stops it. The drops are numbered from the last reset, so the same seed and the same
    /// sequence of deltas give the same rain.
    pub fn set_rain(&mut self, rain: Option<WaveRain>) {
        self.core.set_rain(rain);
    }

    /// The rain as set.
    pub fn rain(&self) -> Option<&WaveRain> {
        self.core.rain()
    }

    /// The pointer stirs the water (call it from `CanvasRenderer::input` with every event): a
    /// PRIMARY-button press (a finger, a pen tip, the left mouse button) presses `stir.drop` and
    /// starts wake slot `MAX_WAKES - 1` there; moving while down makes the wake follow; going up
    /// (or the browser cancelling the pointer) stops it. One pointer stirs at a time; other buttons
    /// and other pointers are ignored (they stay free for orbiting and panning). `to_field` maps a
    /// point in CSS pixels to field units, None when it is off the water (the wake stops there and
    /// starts again where the pointer comes back) - in 3D for example
    /// `|x, y| surface.field_point(&camera.canvas_to_ray(x, y, width, height))`; for a field shown
    /// straight on a canvas, a scale.
    pub fn stir(&mut self, input: &CanvasInput, stir: &WaveStir, to_field: &dyn Fn(f64, f64) -> Option<Vec2>) {
        self.core.stir(input, stir, to_field);
    }

    /// Steps the field by `delta` CLOCK seconds (the frame's `frame.info.delta`) in one compute
    /// pass of its own on `frame`, outside any render pass and before anything reads the surface
    /// this frame: the queued disturbances, then the steps the fixed rate calls for, then the
    /// picture. A delta of 0 (a paused clock, the first frame, the frame after a seek) records
    /// nothing and keeps the picture. Err when called a second time in the same frame.
    pub fn update(&mut self, frame: &mut GraphicsFrame<'_>, delta: f64) -> Result<(), GraphicsException> {
        if delta.is_nan() || delta <= 0.0 {
            self.core.idle();
            return Ok(());
        }
        if self.last_frame == Some(frame.id()) {
            return Err(GraphicsException::new("WaveField::update was already called in this frame: call it once per frame (its uniform block is written once per frame)."));
        }
        let Some(plan) = self.core.plan(delta) else { return Ok(()) };
        self.last_frame = Some(frame.id());
        let params = self.core.params(&plan);
        self.params.write_value(&params, 0)?;
        self.record(frame, plan.steps, !plan.is_calm());
        Ok(())
    }

    /// Records the disturbance, `steps` steps and the picture into one compute pass.
    fn record(&mut self, frame: &mut GraphicsFrame<'_>, steps: u32, disturb: bool) {
        let options = self.core.options();
        let (x, y) = (options.columns.div_ceil(WORKGROUP), options.rows.div_ceil(WORKGROUP));
        let mut pass = frame.begin_compute_pass(&ComputePassOptions { label: Some(self.label.clone()), timestamps: None });
        if disturb {
            pass.set_pipeline(&self.disturb);
            pass.set_bind_group(0, &self.groups[self.current]);
            pass.dispatch(x, y, 1);
        }
        if steps > 0 {
            pass.set_pipeline(&self.step);
            for _ in 0..steps {
                pass.set_bind_group(0, &self.groups[self.current]);
                pass.dispatch(x, y, 1);
                self.current = 1 - self.current;
            }
        }
        pass.set_pipeline(&self.publish);
        pass.set_bind_group(0, &self.groups[self.current]);
        pass.dispatch(x, y, 1);
    }

    /// Calm water again: records a clear of the heights and a fresh (flat) picture into `frame`;
    /// queued drops are discarded, wakes keep following but press nothing from where they were,
    /// and the rain starts over from its seed.
    pub fn reset(&mut self, frame: &mut GraphicsFrame<'_>) -> Result<(), GraphicsException> {
        frame.clear_buffer(&self.heights[0], 0, None)?;
        frame.clear_buffer(&self.heights[1], 0, None)?;
        self.core.reset();
        let params: WaveParams = self.core.params(&Default::default());
        self.params.write_value(&params, 0)?;
        self.record(frame, 0, false);
        Ok(())
    }

    /// THE PICTURE OF THE SURFACE (see [`WaveField`]): `Rgba16Float`, `columns x rows`, with
    /// `STORAGE_BINDING | TEXTURE_BINDING | COPY_SRC`. Bind it (or a clone of its view) for
    /// reading after this frame's [`WaveField::update`].
    pub fn surface(&self) -> &GpuTexture {
        &self.surface
    }

    /// The simulated seconds since creation or the last reset (the steps taken times the step).
    pub fn simulated_time(&self) -> f64 {
        self.core.simulated_time()
    }

    /// The steps taken since creation or the last reset.
    pub fn steps_taken(&self) -> u64 {
        self.core.steps_taken()
    }

    /// The steps the last update ran (0 for an update that did not step), for a read-out.
    pub fn last_step_count(&self) -> u32 {
        self.core.last_step_count()
    }
}
