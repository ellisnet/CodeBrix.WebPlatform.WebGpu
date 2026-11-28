//! The drawing the glow layer and the particle system share (Not in C#): instanced quads from a
//! buffer of [`GlowInstance`] records - camera-facing, camera-facing and lit, or fixed in the scene
//! by a second stream of [`GlowAxes`] records -, one draw call per call of [`GlowDrawing::draw`],
//! with pipelines kept per target format combination, blend and vertex entry point.

use std::collections::HashMap;

use glam::{Mat4, Vec2, Vec3, Vec4};

use crate::context::{GraphicsContext, GraphicsException};
use crate::drawing3d::{AmbientLighting, Camera, SceneLighting};
use crate::effects::target_sample_count;
use crate::frames::{BufferRange, DrawOptions, RenderPass};
use crate::models::pbr_data::needs_shader_encoding;
use crate::pipelines::{BlendPreset, ColorTargetOptions, DepthOptions, DepthPreset, GpuRenderPipeline, VertexLayout, VertexLayoutOptions};
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuShader, GpuTexture, SamplerDescription};
use crate::shaders::EffectShaders;
use crate::RgbaTextureOptions;

/// The bytes of one dynamic-offset uniform slot.
const SLOT_SIZE: u64 = 256;
/// The application textures whose bind groups the drawing keeps.
const KEPT_GROUPS: usize = 4;

/// How glows and particles combine with what is drawn (Not in C#).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GlowBlend {
    /// Light is ADDED to the target (order-independent; the target's alpha is kept). The glow
    /// layer's look, and the usual one for sparks, fire and magic.
    #[default]
    Additive,
    /// Ordinary transparency: premultiplied colour blended OVER the target (smoke, dust). Not
    /// sorted: where such sprites overlap, the later one in the buffer covers the earlier.
    Transparent,
}

impl GlowBlend {
    /// The blend state of the pipeline.
    pub(crate) fn blend_state(self) -> wgpu::BlendState {
        use wgpu::{BlendComponent as C, BlendFactor as F, BlendOperation as O};
        match self {
            GlowBlend::Additive => wgpu::BlendState { color: C { src_factor: F::One, dst_factor: F::One, operation: O::Add }, alpha: C { src_factor: F::Zero, dst_factor: F::One, operation: O::Add } },
            GlowBlend::Transparent => BlendPreset::PremultipliedAlpha.to_wgpu().unwrap_or(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        }
    }
}

/// One instance record of the glow shader (48 bytes; the particle step writes the same layout).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct GlowInstance {
    /// xyz: the centre in scene units; w: the size (the quad's width) in scene units.
    pub(crate) position_size: Vec4,
    /// rgb: linear colour; a: opacity 0..1.
    pub(crate) color: Vec4,
    /// x: intensity; y: pulse rate (cycles per second); z: pulse depth 0..1; w: pulse phase
    /// (cycles).
    pub(crate) pulse: Vec4,
}

/// The bytes of one [`GlowInstance`].
pub(crate) const INSTANCE_SIZE: u64 = 48;
const _: () = assert!(std::mem::size_of::<GlowInstance>() == INSTANCE_SIZE as usize);
// SAFETY: #[repr(C)]; three Vec4, no padding (size asserted above); any f32 bits are valid.
unsafe impl bytemuck::Zeroable for GlowInstance {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for GlowInstance {}

/// The second instance stream of an oriented glow (24 bytes, read by `vs_oriented`): the axes the
/// quad's corners are offset along, each times half the glow's size.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct GlowAxes {
    /// The quad's right edge direction (its length stretches the quad sideways).
    pub(crate) right: Vec3,
    /// The quad's up edge direction (the texture's top; its length stretches the quad upwards).
    pub(crate) up: Vec3,
}

/// The bytes of one [`GlowAxes`].
pub(crate) const AXES_SIZE: u64 = 24;
const _: () = assert!(std::mem::size_of::<GlowAxes>() == AXES_SIZE as usize);
// SAFETY: #[repr(C)]; two Vec3 (three f32 each), no padding (size asserted above); any f32 bits
// are valid.
unsafe impl bytemuck::Zeroable for GlowAxes {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for GlowAxes {}

/// The glow shader's uniforms (240 bytes; one 256-byte dynamic-offset slot as before).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GlowGlobals {
    pub(crate) view: Mat4,
    pub(crate) projection: Mat4,
    /// Time, 1 = encode to sRGB, 1 = textured, 1 = particles (no pulse; the pulse's yzw are the
    /// velocity).
    pub(crate) params: Vec4,
    /// The near fade's start and end in glow sizes (end 0: off); the stretch's factor and longest
    /// length in sizes (0: off).
    pub(crate) effects: Vec4,
    /// The lit entry point's terms, in view space: the ambient's sky and ground colours, its up
    /// direction, the direction the key light travels, the key light's colour times intensity
    /// (w: the exposure).
    pub(crate) ambient_sky: Vec4,
    pub(crate) ambient_ground: Vec4,
    pub(crate) ambient_up: Vec4,
    pub(crate) light_direction: Vec4,
    pub(crate) light_color: Vec4,
}

const GLOBALS_SIZE: u64 = 240;
const _: () = assert!(std::mem::size_of::<GlowGlobals>() == GLOBALS_SIZE as usize);
const _: () = assert!(GLOBALS_SIZE <= SLOT_SIZE);
// SAFETY: #[repr(C)]; two Mat4 and seven Vec4, no padding (size asserted above); any f32 bits are
// valid.
unsafe impl bytemuck::Zeroable for GlowGlobals {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for GlowGlobals {}

/// The vertex entry point of a glow pipeline and the instance streams it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GlowVertex {
    /// `vs_main`: camera-facing quads (the glow layer's glows, particles), the record alone.
    Facing,
    /// `vs_lit`: camera-facing quads lit by the key light and the ambient (lit particles).
    Lit,
    /// `vs_oriented`: quads fixed in the scene by the [`GlowAxes`] stream (oriented glows).
    Oriented,
}

impl GlowVertex {
    /// The entry point for a draw: oriented when it has an axes stream (never lit), lit when it
    /// has a lighting, camera-facing otherwise.
    pub(crate) fn select(oriented: bool, lit: bool) -> GlowVertex {
        match (oriented, lit) {
            (true, _) => GlowVertex::Oriented,
            (false, true) => GlowVertex::Lit,
            (false, false) => GlowVertex::Facing,
        }
    }

    /// The WGSL entry point's name.
    pub(crate) fn entry_point(self) -> &'static str {
        match self {
            GlowVertex::Facing => "vs_main",
            GlowVertex::Lit => "vs_lit",
            GlowVertex::Oriented => "vs_oriented",
        }
    }

    /// The vertex buffer slots: the instance record (locations 0 to 2), and for oriented quads the
    /// axes (locations 3 and 4).
    pub(crate) fn layouts(self) -> Vec<VertexLayout> {
        let record = VertexLayout::describe(VertexLayoutOptions { step_mode: wgpu::VertexStepMode::Instance, first_location: 0 }).add(wgpu::VertexFormat::Float32x4).add(wgpu::VertexFormat::Float32x4).add(wgpu::VertexFormat::Float32x4).build(INSTANCE_SIZE);
        match self {
            GlowVertex::Facing | GlowVertex::Lit => vec![record],
            GlowVertex::Oriented => vec![record, VertexLayout::describe(VertexLayoutOptions { step_mode: wgpu::VertexStepMode::Instance, first_location: 3 }).add(wgpu::VertexFormat::Float32x3).add(wgpu::VertexFormat::Float32x3).build(AXES_SIZE)],
        }
    }
}

/// What a pipeline is chosen by: the pass's colour and depth formats, its sample count (1, or 4
/// on a multisampled target), the blend and the vertex entry point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PipelineKey {
    color: wgpu::TextureFormat,
    depth: Option<wgpu::TextureFormat>,
    samples: u32,
    blend: GlowBlend,
    vertex: GlowVertex,
}

/// The instances of one draw: `count` records of `instances` from byte `offset`, and for oriented
/// glows the axes buffer and the byte offset of the first record's axes.
#[derive(Clone, Copy)]
pub(crate) struct GlowStream<'a> {
    pub(crate) instances: &'a GpuBuffer,
    pub(crate) offset: u64,
    pub(crate) count: u32,
    pub(crate) axes: Option<(&'a GpuBuffer, u64)>,
}

/// How one draw looks: the blend, the shape texture (None: the round falloff), whether the
/// instances are particles (no pulse), the near fade and the stretch as the shader reads them
/// ((0, 0): off) and, for lit particles, the lighting.
#[derive(Clone, Copy)]
pub(crate) struct GlowLook<'a> {
    pub(crate) blend: GlowBlend,
    pub(crate) texture: Option<&'a GpuTexture>,
    pub(crate) particles: bool,
    pub(crate) near_fade: Vec2,
    pub(crate) stretch: Vec2,
    pub(crate) lighting: Option<&'a SceneLighting>,
}

impl GlowLook<'_> {
    /// A look with only a blend and a texture: no pulse change, no near fade, no stretch, unlit.
    pub(crate) fn plain(blend: GlowBlend, texture: Option<&GpuTexture>) -> GlowLook<'_> {
        GlowLook { blend, texture, particles: false, near_fade: Vec2::ZERO, stretch: Vec2::ZERO, lighting: None }
    }
}

/// The glow shader's uniforms of a draw: the camera's matrices, the clock time, the sRGB encoding,
/// the texture switch and the look's terms. The lighting's terms are in view space; with no
/// lighting they are zero (the lit entry point is not used then). AMBIENT: under
/// [`AmbientLighting::Hemisphere`] the model renderer's hemisphere (sky
/// [`SceneLighting::ambient_color`], ground [`SceneLighting::ambient_ground_color`] or the sky
/// colour, around [`SceneLighting::ambient_up`]); under [`AmbientLighting::Environment`] the flat
/// [`SceneLighting::ambient_color`], as the mesh renderer uses it (an environment's light is not
/// read for particles). The KEY LIGHT: its direction (straight down when it has none) and colour
/// times intensity, as the mesh renderer's. The EXPOSURE ([`SceneLighting::exposure`]) scales the
/// whole lit colour, as the model renderer scales its output (the mesh renderer, like C#'s, has
/// none), so lit particles beside the models take the same exposure.
pub(crate) fn glow_globals(view: Mat4, projection: Mat4, time: f32, encode: bool, look: &GlowLook<'_>) -> GlowGlobals {
    let flag = |on: bool| if on { 1.0 } else { 0.0 };
    let mut globals = GlowGlobals {
        view,
        projection,
        params: Vec4::new(time, flag(encode), flag(look.texture.is_some()), flag(look.particles)),
        effects: Vec4::new(look.near_fade.x, look.near_fade.y, look.stretch.x, look.stretch.y),
        ambient_sky: Vec4::ZERO,
        ambient_ground: Vec4::ZERO,
        ambient_up: Vec4::ZERO,
        light_direction: Vec4::ZERO,
        light_color: Vec4::ZERO,
    };
    if let Some(lighting) = look.lighting {
        let sky = lighting.ambient_color;
        let ground = match lighting.ambient {
            AmbientLighting::Hemisphere => lighting.ambient_ground_color.unwrap_or(sky),
            AmbientLighting::Environment => sky,
        };
        let up = lighting.ambient_up.try_normalize().unwrap_or(Vec3::Y);
        let direction = lighting.light_direction.try_normalize().unwrap_or(-Vec3::Y);
        globals.ambient_sky = Vec4::new(sky.r, sky.g, sky.b, 0.0);
        globals.ambient_ground = Vec4::new(ground.r, ground.g, ground.b, 0.0);
        globals.ambient_up = view.transform_vector3(up).normalize_or_zero().extend(0.0);
        globals.light_direction = view.transform_vector3(direction).normalize_or_zero().extend(0.0);
        let color = lighting.light_color;
        globals.light_color = Vec4::new(color.r * lighting.light_intensity, color.g * lighting.light_intensity, color.b * lighting.light_intensity, lighting.exposure);
    }
    globals
}

/// The shared drawing of glows and particles.
pub(crate) struct GlowDrawing {
    context: GraphicsContext,
    shader: GpuShader,
    globals_layout: GpuBindGroupLayout,
    texture_layout: GpuBindGroupLayout,
    sampler: GpuSampler,
    white_group: GpuBindGroup,
    texture_groups: Vec<(wgpu::TextureView, GpuBindGroup)>,
    pipelines: HashMap<PipelineKey, GpuRenderPipeline>,
    uniforms: GpuBuffer,
    uniform_group: GpuBindGroup,
    capacity: u64,
    slot: u64,
    frame_id: Option<u64>,
}

impl GlowDrawing {
    /// The drawing, with its shader, layouts, sampler and the 1 x 1 placeholder texture.
    pub(crate) fn new(context: &GraphicsContext, label: &str) -> Result<GlowDrawing, GraphicsException> {
        let shader = context.create_shader(EffectShaders::GLOW, Some(label));
        let globals_layout = context.create_bind_group_layout(Some(label), &[BindingLayoutEntry::uniform(0, wgpu::ShaderStages::VERTEX_FRAGMENT).with_dynamic_offset(true).with_min_binding_size(GLOBALS_SIZE)]);
        let texture_layout = context.create_bind_group_layout(Some(label), &[BindingLayoutEntry::texture(0, wgpu::ShaderStages::FRAGMENT), BindingLayoutEntry::sampler(1, wgpu::ShaderStages::FRAGMENT)]);
        let sampler = context.create_sampler(&SamplerDescription::linear_clamp());
        let white = context.create_texture_from_rgba(1, 1, &[255, 255, 255, 255], RgbaTextureOptions { label: Some(format!("{label} placeholder")), ..RgbaTextureOptions::default() })?;
        let white_group = context.create_bind_group(&texture_layout, Some(label), &[BindingEntry::for_texture(0, &white), BindingEntry::for_sampler(1, &sampler)]);
        let capacity = 4;
        let uniforms = context.create_buffer(capacity * SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some(label))?;
        let uniform_group = context.create_bind_group(&globals_layout, Some(label), &[BindingEntry::for_buffer(0, &uniforms).with_range(0, Some(GLOBALS_SIZE))]);
        Ok(GlowDrawing { context: context.clone(), shader, globals_layout, texture_layout, sampler, white_group, texture_groups: Vec::new(), pipelines: HashMap::new(), uniforms, uniform_group, capacity, slot: 0, frame_id: None })
    }

    /// The pipelines made so far.
    pub(crate) fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }

    /// Draws the instances of `stream` into the pass, as seen by `camera`, at clock `time`, as
    /// `look` says (its blend, its texture - None: the round falloff -, its effects, and lit when
    /// it has a lighting; oriented when the stream has axes). One instanced draw call.
    pub(crate) fn draw(&mut self, pass: &mut RenderPass<'_>, camera: &Camera, time: f64, stream: GlowStream<'_>, look: &GlowLook<'_>) -> Result<(), GraphicsException> {
        if stream.count == 0 {
            return Ok(());
        }
        if self.frame_id != Some(pass.frame_id()) {
            self.frame_id = Some(pass.frame_id());
            self.slot = 0;
        }
        if self.slot >= self.capacity {
            // Work already recorded keeps the old buffer (the web backend's Drop does nothing).
            self.capacity *= 2;
            self.uniforms = self.context.create_buffer(self.capacity * SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("glow"))?;
            self.uniform_group = self.context.create_bind_group(&self.globals_layout, Some("glow"), &[BindingEntry::for_buffer(0, &self.uniforms).with_range(0, Some(GLOBALS_SIZE))]);
            self.slot = 0;
        }
        let target = pass.target().clone();
        let globals = glow_globals(camera.view_matrix(), camera.get_projection_matrix(target.aspect_ratio()), time as f32, needs_shader_encoding(target.color_format()), look);
        let uniform_offset = self.slot * SLOT_SIZE;
        self.uniforms.write_value(&globals, uniform_offset)?;
        self.slot += 1;
        let vertex = GlowVertex::select(stream.axes.is_some(), look.lighting.is_some());
        let key = PipelineKey { color: target.color_format(), depth: target.depth_format(), samples: target_sample_count(&target), blend: look.blend, vertex };
        let pipeline = self.pipeline(key)?;
        let texture_group = match look.texture {
            Some(texture) => self.texture_group(texture),
            None => self.white_group.clone(),
        };
        pass.set_pipeline(&pipeline);
        pass.set_bind_group_with_offsets(0, &self.uniform_group, &[uniform_offset as u32]);
        pass.set_bind_group(1, &texture_group);
        pass.set_vertex_buffer(0, stream.instances, BufferRange { offset: stream.offset, size: Some(stream.count as u64 * INSTANCE_SIZE) });
        if let Some((axes, offset)) = stream.axes {
            pass.set_vertex_buffer(1, axes, BufferRange { offset, size: Some(stream.count as u64 * AXES_SIZE) });
        }
        pass.draw(6, DrawOptions { instance_count: stream.count, ..DrawOptions::default() });
        Ok(())
    }

    fn texture_group(&mut self, texture: &GpuTexture) -> GpuBindGroup {
        if let Some((_, group)) = self.texture_groups.iter().find(|(view, _)| view == texture.view()) {
            return group.clone();
        }
        let group = self.context.create_bind_group(&self.texture_layout, Some("glow texture"), &[BindingEntry::for_texture(0, texture), BindingEntry::for_sampler(1, &self.sampler)]);
        if self.texture_groups.len() >= KEPT_GROUPS {
            self.texture_groups.remove(0);
        }
        self.texture_groups.push((texture.view().clone(), group.clone()));
        group
    }

    fn pipeline(&mut self, key: PipelineKey) -> Result<GpuRenderPipeline, GraphicsException> {
        if let Some(pipeline) = self.pipelines.get(&key) {
            return Ok(pipeline.clone());
        }
        let target = ColorTargetOptions { custom_blend: Some(key.blend.blend_state()), ..ColorTargetOptions::default() };
        let mut builder = self.context.create_render_pipeline(&self.shader).with_label(&format!("glow {:?} {:?} {:?}", key.vertex, key.color, key.blend)).with_vertex_entry(key.vertex.entry_point(), None);
        for layout in key.vertex.layouts() {
            builder = builder.with_vertex_buffer(layout);
        }
        builder = builder
            .with_color_target(key.color, target)
            .with_bind_group_layouts(&[&self.globals_layout, &self.texture_layout])
            .with_sample_count(key.samples);
        if let Some(depth) = key.depth {
            // Tested against what is solid, never written: glows and particles hide behind
            // surfaces and never hide each other.
            builder = builder.with_depth(depth, DepthOptions { preset: DepthPreset::ReadOnly, ..DepthOptions::default() });
        }
        let pipeline = builder.build()?;
        self.pipelines.insert(key, pipeline.clone());
        Ok(pipeline)
    }
}

/// The glow shader's round falloff on the CPU (for tests): `(1 - r^2)^3` of the distance `r`
/// from the centre in half-sizes, 0 from `r = 1` on.
#[cfg(test)]
pub(crate) fn glow_falloff(r: f32) -> f32 {
    let f = (1.0 - r * r).max(0.0);
    f * f * f
}

/// The glow shader's pulse on the CPU (for tests): 1 at the peak, `1 - depth` at the trough.
#[cfg(test)]
pub(crate) fn pulse_factor(rate: f32, depth: f32, phase: f32, time: f32) -> f32 {
    let wave = 0.5 - 0.5 * (std::f32::consts::TAU * (rate * time + phase)).cos();
    1.0 - depth.clamp(0.0, 1.0) * wave
}

/// The glow shader's near fade on the CPU (for tests): 0 up to `start` glow sizes from the
/// centre, 1 from `end` on, a smooth step between.
#[cfg(test)]
pub(crate) fn near_fade(sizes: f32, start: f32, end: f32) -> f32 {
    let t = ((sizes - start) / (end - start)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The glow shader's `strength_of` on the CPU (for tests): intensity times the pulse (none for
/// particles), times the near fade when it is on.
#[cfg(test)]
pub(crate) fn strength_of(pulse: Vec4, view_position: Vec3, size: f32, globals: &GlowGlobals) -> f32 {
    let pulsing = if globals.params.w > 0.5 { 1.0 } else { pulse_factor(pulse.y, pulse.z, pulse.w, globals.params.x) };
    let mut strength = pulse.x.max(0.0) * pulsing;
    if globals.effects.y > 0.0 {
        strength *= near_fade(view_position.length() / size.max(1e-6), globals.effects.x, globals.effects.y);
    }
    strength
}

/// True for a perspective projection (its w row is (0, 0, -1, 0)), as the shader tests it.
#[cfg(test)]
fn is_perspective(globals: &GlowGlobals) -> bool {
    globals.projection.w_axis.w < 0.5
}

/// The glow shader's `facing_offset` on the CPU (for tests): the camera-facing corner's offset in
/// view space, stretched along the motion seen on the screen when the stretch is on.
#[cfg(test)]
pub(crate) fn facing_offset(corner: Vec2, view_position: Vec3, size: f32, velocity: Vec3, globals: &GlowGlobals) -> Vec2 {
    let half = size * 0.5;
    if globals.effects.w <= 0.0 {
        return corner * half;
    }
    let v = globals.view.transform_vector3(velocity);
    let depth = (-view_position.z).max(1e-4);
    let seen = if is_perspective(globals) { v.truncate() + view_position.truncate() * (v.z / depth) } else { v.truncate() };
    let speed = seen.length();
    let along = if speed > 1e-6 { seen / speed.max(1e-12) } else { Vec2::X };
    let across = Vec2::new(-along.y, along.x);
    let length_along = (size + globals.effects.z * speed).clamp(size, globals.effects.w * size);
    along * (corner.x * length_along * 0.5) + across * (corner.y * half)
}

/// The glow shader's `lighting_at` on the CPU (for tests): the ambient's hemisphere and the key
/// light's half-Lambert term on the normal that faces the camera from the particle, times the
/// exposure.
#[cfg(test)]
pub(crate) fn lighting_at(view_position: Vec3, globals: &GlowGlobals) -> Vec3 {
    let to_camera = view_position.length();
    let normal = if is_perspective(globals) && to_camera > 1e-6 { -view_position / to_camera.max(1e-6) } else { Vec3::Z };
    let t = (normal.dot(globals.ambient_up.truncate()) * 0.5 + 0.5).clamp(0.0, 1.0);
    let ambient = globals.ambient_ground.truncate().lerp(globals.ambient_sky.truncate(), t);
    let wrap = normal.dot(-globals.light_direction.truncate()) * 0.5 + 0.5;
    (ambient + globals.light_color.truncate() * (wrap * wrap)) * globals.light_color.w
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec2, Vec3, Vec4};

    use super::{facing_offset, glow_falloff, glow_globals, lighting_at, near_fade, pulse_factor, strength_of, GlowAxes, GlowBlend, GlowGlobals, GlowLook, GlowVertex, INSTANCE_SIZE};
    use crate::drawing3d::{AmbientLighting, Camera, ProjectionKind, SceneLighting};
    use crate::Color4;

    fn globals_of(camera: &Camera, look: &GlowLook<'_>) -> GlowGlobals {
        glow_globals(camera.view_matrix(), camera.get_projection_matrix(1.5), 2.5, false, look)
    }

    #[test]
    fn the_falloff_is_round_brightest_at_the_centre_and_smoothly_zero_at_the_edge() {
        assert_eq!(glow_falloff(0.0), 1.0);
        assert_eq!(glow_falloff(1.0), 0.0);
        assert_eq!(glow_falloff(1.5), 0.0);
        assert!((glow_falloff(0.5) - 0.421875).abs() < 1e-6);
        let mut previous = 1.0;
        for step in 1..=100 {
            let value = glow_falloff(step as f32 / 100.0);
            assert!(value <= previous, "falls monotonically");
            previous = value;
        }
        // Flat at the edge: the last step changes the value by almost nothing.
        assert!(glow_falloff(0.99) < 1e-5);
    }

    #[test]
    fn the_pulse_dips_by_its_depth_once_per_cycle() {
        assert!((pulse_factor(2.0, 0.6, 0.0, 0.0) - 1.0).abs() < 1e-6);
        assert!((pulse_factor(2.0, 0.6, 0.0, 0.25) - 0.4).abs() < 1e-5, "half a cycle at 2 Hz");
        assert!((pulse_factor(2.0, 0.6, 0.0, 0.5) - 1.0).abs() < 1e-5);
        assert!((pulse_factor(1.0, 1.0, 0.5, 0.0) - 0.0).abs() < 1e-6, "phase shifts the cycle");
        assert_eq!(pulse_factor(3.0, 0.0, 0.2, 7.3), 1.0, "no depth, no pulse");
        assert!((pulse_factor(1.0, 5.0, 0.5, 0.0)).abs() < 1e-6, "depth is clamped to 1");
    }

    #[test]
    fn additive_adds_colour_and_keeps_the_targets_alpha() {
        let state = GlowBlend::Additive.blend_state();
        assert_eq!((state.color.src_factor, state.color.dst_factor), (wgpu::BlendFactor::One, wgpu::BlendFactor::One));
        assert_eq!((state.alpha.src_factor, state.alpha.dst_factor), (wgpu::BlendFactor::Zero, wgpu::BlendFactor::One));
        assert_eq!(GlowBlend::Transparent.blend_state(), wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
        assert_eq!(GlowBlend::default(), GlowBlend::Additive);
    }

    #[test]
    fn glow_and_particle_pipelines_are_kept_per_sample_count() {
        let key = |samples| super::PipelineKey { color: wgpu::TextureFormat::Rgba16Float, depth: Some(wgpu::TextureFormat::Depth32Float), samples, blend: GlowBlend::Additive, vertex: GlowVertex::Facing };
        assert_ne!(key(1), key(4));
        let keys: std::collections::HashSet<super::PipelineKey> = crate::presentation::SAMPLE_COUNTS.iter().map(|count| key(*count)).collect();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn the_vertex_entry_point_is_chosen_by_the_axes_stream_and_the_lighting() {
        assert_eq!(GlowVertex::select(false, false), GlowVertex::Facing);
        assert_eq!(GlowVertex::select(false, true), GlowVertex::Lit);
        assert_eq!(GlowVertex::select(true, false), GlowVertex::Oriented);
        assert_eq!(GlowVertex::select(true, true), GlowVertex::Oriented, "oriented quads are never lit");
        assert_eq!([GlowVertex::Facing.entry_point(), GlowVertex::Lit.entry_point(), GlowVertex::Oriented.entry_point()], ["vs_main", "vs_lit", "vs_oriented"]);
        for vertex in [GlowVertex::Facing, GlowVertex::Lit, GlowVertex::Oriented] {
            assert!(crate::shaders::EffectShaders::GLOW.contains(&format!("fn {}(", vertex.entry_point())));
        }
        // Camera-facing and lit read the 48-byte record alone; oriented adds the axes stream.
        let facing = GlowVertex::Facing.layouts();
        assert_eq!(facing.len(), 1);
        assert_eq!((facing[0].array_stride(), facing[0].elements().len(), facing[0].next_location()), (INSTANCE_SIZE, 3, 3));
        let oriented = GlowVertex::Oriented.layouts();
        assert_eq!(oriented.len(), 2);
        assert_eq!(oriented[0], facing[0]);
        assert_eq!(oriented[1].array_stride(), 24);
        assert_eq!(oriented[1].step_mode(), wgpu::VertexStepMode::Instance);
        let axes: Vec<(u32, u64)> = oriented[1].elements().iter().map(|e| (e.location, e.offset)).collect();
        assert_eq!(axes, vec![(3, 0), (4, 12)]);
        assert_eq!((std::mem::offset_of!(GlowAxes, right), std::mem::offset_of!(GlowAxes, up)), (0, 12));
        // Each entry point has its own pipelines.
        let key = |vertex| super::PipelineKey { color: wgpu::TextureFormat::Rgba16Float, depth: None, samples: 1, blend: GlowBlend::Additive, vertex };
        let keys: std::collections::HashSet<super::PipelineKey> = [GlowVertex::Facing, GlowVertex::Lit, GlowVertex::Oriented].into_iter().map(key).collect();
        assert_eq!(keys.len(), 3);
    }

    #[test]
    fn the_globals_record_matches_the_shaders_struct() {
        assert_eq!(std::mem::offset_of!(GlowGlobals, params), 128);
        assert_eq!(std::mem::offset_of!(GlowGlobals, effects), 144);
        assert_eq!(std::mem::offset_of!(GlowGlobals, ambient_sky), 160);
        assert_eq!(std::mem::offset_of!(GlowGlobals, ambient_ground), 176);
        assert_eq!(std::mem::offset_of!(GlowGlobals, ambient_up), 192);
        assert_eq!(std::mem::offset_of!(GlowGlobals, light_direction), 208);
        assert_eq!(std::mem::offset_of!(GlowGlobals, light_color), 224);
        let wgsl = crate::shaders::EffectShaders::GLOW;
        let order = ["view:", "projection:", "params:", "effects:", "ambient_sky:", "ambient_ground:", "ambient_up:", "light_direction:", "light_color:"];
        let start = wgsl.find("struct Globals {").expect("the struct");
        let body = &wgsl[start..start + wgsl[start..].find("};").expect("its end")];
        let mut at = 0;
        for name in order {
            let found = body[at..].find(&format!("    {name}")).unwrap_or_else(|| panic!("{name} in order"));
            at += found + name.len();
        }
    }

    #[test]
    fn a_plain_draw_packs_what_it_always_has_and_nothing_else() {
        let camera = Camera::default();
        let globals = globals_of(&camera, &GlowLook::plain(GlowBlend::Additive, None));
        assert_eq!(globals.view, camera.view_matrix());
        assert_eq!(globals.params, Vec4::new(2.5, 0.0, 0.0, 0.0));
        for value in [globals.effects, globals.ambient_sky, globals.ambient_ground, globals.ambient_up, globals.light_direction, globals.light_color] {
            assert_eq!(value, Vec4::ZERO);
        }
        let encoded = glow_globals(Mat4::IDENTITY, Mat4::IDENTITY, 0.0, true, &GlowLook { particles: true, near_fade: Vec2::new(0.5, 2.0), stretch: Vec2::new(0.05, 4.0), ..GlowLook::plain(GlowBlend::Transparent, None) });
        assert_eq!(encoded.params, Vec4::new(0.0, 1.0, 0.0, 1.0));
        assert_eq!(encoded.effects, Vec4::new(0.5, 2.0, 0.05, 4.0));
    }

    #[test]
    fn the_lit_terms_follow_the_scene_lighting_in_view_space() {
        // The default camera looks down -Z from (0, 0, 5): its view turns nothing.
        let camera = Camera::default();
        let mut lighting = SceneLighting { light_direction: Vec3::new(0.0, -2.0, 0.0), light_color: Color4::new(1.0, 0.5, 0.25, 1.0), light_intensity: 2.0, ambient_color: Color4::new(0.2, 0.3, 0.4, 1.0), ambient_ground_color: Some(Color4::new(0.1, 0.0, 0.0, 1.0)), ..SceneLighting::default() };
        let lit = |lighting: &SceneLighting| globals_of(&camera, &GlowLook { lighting: Some(lighting), particles: true, ..GlowLook::plain(GlowBlend::Additive, None) });
        let globals = lit(&lighting);
        assert!((globals.light_direction - Vec4::new(0.0, -1.0, 0.0, 0.0)).length() < 1e-6, "unit, in view space");
        assert_eq!(globals.light_color, Vec4::new(2.0, 1.0, 0.5, 1.0), "colour times intensity; w the exposure (default 1)");
        let exposed = lit(&SceneLighting { exposure: 1.5, ..lighting.clone() });
        assert_eq!(exposed.light_color, Vec4::new(2.0, 1.0, 0.5, 1.5), "the exposure does not scale the light's colour itself");
        // The default ambient is the environment: the flat ambient colour (the ground colour unused).
        assert_eq!(lighting.ambient, AmbientLighting::Environment);
        assert_eq!(globals.ambient_sky, Vec4::new(0.2, 0.3, 0.4, 0.0));
        assert_eq!(globals.ambient_ground, globals.ambient_sky);
        lighting.ambient = AmbientLighting::Hemisphere;
        let globals = lit(&lighting);
        assert_eq!(globals.ambient_ground, Vec4::new(0.1, 0.0, 0.0, 0.0));
        assert!((globals.ambient_up - Vec4::new(0.0, 1.0, 0.0, 0.0)).length() < 1e-6);
        // A light with no direction shines straight down; a camera looking along +X turns it.
        lighting.light_direction = Vec3::ZERO;
        let side = Camera { position: Vec3::ZERO, target: Vec3::X, ..Camera::default() };
        let globals = globals_of(&side, &GlowLook { lighting: Some(&lighting), ..GlowLook::plain(GlowBlend::Additive, None) });
        assert!((globals.light_direction - Vec4::new(0.0, -1.0, 0.0, 0.0)).length() < 1e-6);
        lighting.light_direction = Vec3::X;
        let globals = globals_of(&side, &GlowLook { lighting: Some(&lighting), ..GlowLook::plain(GlowBlend::Additive, None) });
        assert!((globals.light_direction - Vec4::new(0.0, 0.0, -1.0, 0.0)).length() < 1e-6, "away from the camera is -Z in view space");
    }

    #[test]
    fn a_lit_particle_takes_the_ambient_and_a_half_lambert_key_light() {
        let camera = Camera::default();
        let lighting = SceneLighting { light_direction: Vec3::new(0.0, 0.0, -1.0), light_color: Color4::WHITE, light_intensity: 1.0, ambient_color: Color4::new(0.25, 0.25, 0.25, 1.0), ..SceneLighting::default() };
        let globals = globals_of(&camera, &GlowLook { lighting: Some(&lighting), ..GlowLook::plain(GlowBlend::Additive, None) });
        // In front of the camera on its axis: the normal faces the camera (+Z in view space).
        let ahead = Vec3::new(0.0, 0.0, -5.0);
        assert!((lighting_at(ahead, &globals) - Vec3::splat(1.25)).length() < 1e-5, "light from behind the camera: full");
        let toward = GlowLook { lighting: Some(&SceneLighting { light_direction: Vec3::Z, ..lighting.clone() }), ..GlowLook::plain(GlowBlend::Additive, None) };
        assert!((lighting_at(ahead, &globals_of(&camera, &toward)) - Vec3::splat(0.25)).length() < 1e-5, "light from behind the particle: ambient only");
        let side = SceneLighting { light_direction: Vec3::X, ..lighting.clone() };
        let side = globals_of(&camera, &GlowLook { lighting: Some(&side), ..GlowLook::plain(GlowBlend::Additive, None) });
        assert!((lighting_at(ahead, &side) - Vec3::splat(0.5)).length() < 1e-5, "from the side: a quarter (half-Lambert squared)");
        // Off the axis the normal turns towards the camera, so a side light lights it more or less.
        assert!(lighting_at(Vec3::new(3.0, 0.0, -4.0), &side).x > 0.5 + 0.1);
        assert!(lighting_at(Vec3::new(-3.0, 0.0, -4.0), &side).x < 0.5 - 0.1);
        // Under an orthographic camera every particle faces +Z.
        let flat = Camera { projection: ProjectionKind::Orthographic, ..camera };
        let flat = globals_of(&flat, &GlowLook { lighting: Some(&SceneLighting { light_direction: Vec3::X, ..lighting.clone() }), ..GlowLook::plain(GlowBlend::Additive, None) });
        assert!((lighting_at(Vec3::new(3.0, 0.0, -4.0), &flat) - Vec3::splat(0.5)).length() < 1e-5);
        // The exposure scales the whole lit colour, ambient and key light alike (as the model
        // renderer's output); an exposure of 0 is dark.
        let exposed = SceneLighting { exposure: 2.0, ..lighting.clone() };
        let exposed = globals_of(&camera, &GlowLook { lighting: Some(&exposed), ..GlowLook::plain(GlowBlend::Additive, None) });
        assert!((lighting_at(ahead, &exposed) - Vec3::splat(2.5)).length() < 1e-5);
        let dark = SceneLighting { exposure: 0.0, ..lighting.clone() };
        let dark = globals_of(&camera, &GlowLook { lighting: Some(&dark), ..GlowLook::plain(GlowBlend::Additive, None) });
        assert_eq!(lighting_at(ahead, &dark), Vec3::ZERO);
        // The hemisphere: sky above, ground below, half and half on a normal across `up`.
        let hemisphere = SceneLighting { ambient: AmbientLighting::Hemisphere, ambient_color: Color4::new(1.0, 1.0, 1.0, 1.0), ambient_ground_color: Some(Color4::new(0.0, 0.0, 0.0, 1.0)), light_intensity: 0.0, ..lighting.clone() };
        let hemisphere = globals_of(&camera, &GlowLook { lighting: Some(&hemisphere), ..GlowLook::plain(GlowBlend::Additive, None) });
        assert!((lighting_at(ahead, &hemisphere) - Vec3::splat(0.5)).length() < 1e-5);
        assert!(lighting_at(Vec3::new(0.0, -4.0, -3.0), &hemisphere).x > 0.8, "below the camera, the normal looks up into the sky");
    }

    #[test]
    fn the_near_fade_is_off_by_default_and_fades_a_glow_the_camera_comes_close_to() {
        assert_eq!(near_fade(0.0, 0.5, 2.0), 0.0);
        assert_eq!(near_fade(0.5, 0.5, 2.0), 0.0);
        assert_eq!(near_fade(1.25, 0.5, 2.0), 0.5);
        assert_eq!(near_fade(2.0, 0.5, 2.0), 1.0);
        assert_eq!(near_fade(9.0, 0.5, 2.0), 1.0);
        let pulse = Vec4::new(3.0, 2.0, 0.6, 0.0);
        let near = Vec3::new(0.0, 0.0, -0.5);
        // Off: intensity times the pulse, exactly as before.
        let off = globals_of(&Camera::default(), &GlowLook::plain(GlowBlend::Additive, None));
        assert_eq!(strength_of(pulse, near, 1.0, &off), 3.0 * pulse_factor(2.0, 0.6, 0.0, 2.5));
        // On: half a size away from a 1-wide glow it is gone; measured in sizes, so a 0.1-wide
        // glow 0.125 away is half faded.
        let on = GlowGlobals { effects: Vec4::new(0.5, 2.0, 0.0, 0.0), ..off };
        assert_eq!(strength_of(pulse, near, 1.0, &on), 0.0);
        assert!((strength_of(Vec4::new(1.0, 0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -0.125), 0.1, &on) - 0.5).abs() < 1e-5);
        assert_eq!(strength_of(pulse, Vec3::new(0.0, 0.0, -20.0), 1.0, &on), strength_of(pulse, near, 1.0, &off), "far away: untouched");
        // Particles have no pulse: their yzw (the velocity) are not read as one.
        let particles = GlowGlobals { params: Vec4::new(2.5, 0.0, 0.0, 1.0), ..off };
        assert_eq!(strength_of(Vec4::new(1.0, 4.0, 0.3, 0.7), near, 1.0, &particles), 1.0);
        assert_eq!(strength_of(Vec4::new(1.0, 0.0, 0.0, 0.0), near, 1.0, &off), 1.0, "a particle's old record drew at full strength too");
    }

    #[test]
    fn the_stretch_lengthens_a_particle_along_its_motion_on_the_screen() {
        let camera = Camera::default();
        let off = globals_of(&camera, &GlowLook { particles: true, ..GlowLook::plain(GlowBlend::Additive, None) });
        let corner = Vec2::new(1.0, -1.0);
        let at = Vec3::new(0.0, 0.0, -5.0);
        // Off: the square of the size, whatever the velocity.
        assert_eq!(facing_offset(corner, at, 0.2, Vec3::new(5.0, 0.0, 0.0), &off), corner * 0.1);
        let on = GlowGlobals { effects: Vec4::new(0.0, 0.0, 0.05, 4.0), ..off };
        // On, but standing still: the same square.
        assert_eq!(facing_offset(corner, at, 0.2, Vec3::ZERO, &on), corner * 0.1);
        // Moving sideways at 2: 0.2 + 0.05 x 2 = 0.3 long, still 0.2 wide.
        let moving = facing_offset(Vec2::new(1.0, 0.0), at, 0.2, Vec3::new(2.0, 0.0, 0.0), &on);
        assert!((moving - Vec2::new(0.15, 0.0)).length() < 1e-6, "{moving}");
        let across = facing_offset(Vec2::new(0.0, 1.0), at, 0.2, Vec3::new(2.0, 0.0, 0.0), &on);
        assert!((across - Vec2::new(0.0, 0.1)).length() < 1e-6, "{across}");
        // Moving up the screen: the long side turns with the motion.
        let up = facing_offset(Vec2::new(1.0, 0.0), at, 0.2, Vec3::new(0.0, 2.0, 0.0), &on);
        assert!((up - Vec2::new(0.0, 0.15)).length() < 1e-6, "{up}");
        // Never longer than `max` sizes.
        let fast = facing_offset(Vec2::new(1.0, 0.0), at, 0.2, Vec3::new(1000.0, 0.0, 0.0), &on);
        assert!((fast.length() - 0.4).abs() < 1e-5, "{fast}");
        // Straight towards the camera at the screen's centre: no motion on the screen, no stretch.
        assert_eq!(facing_offset(corner, at, 0.2, Vec3::new(0.0, 0.0, 3.0), &on), corner * 0.1);
        // Off the centre, coming closer moves it outwards on a perspective screen...
        let aside = Vec3::new(1.0, 0.0, -5.0);
        let radial = facing_offset(Vec2::new(1.0, 0.0), aside, 0.2, Vec3::new(0.0, 0.0, 5.0), &on);
        assert!((radial - Vec2::new(0.1 + 0.025, 0.0)).length() < 1e-5, "{radial}");
        // ... and not at all on an orthographic one.
        let flat = Camera { projection: ProjectionKind::Orthographic, ..camera };
        let flat = GlowGlobals { effects: on.effects, ..globals_of(&flat, &GlowLook { particles: true, ..GlowLook::plain(GlowBlend::Additive, None) }) };
        assert_eq!(facing_offset(Vec2::new(1.0, 0.0), aside, 0.2, Vec3::new(0.0, 0.0, 5.0), &flat), Vec2::new(0.1, 0.0));
    }
}
