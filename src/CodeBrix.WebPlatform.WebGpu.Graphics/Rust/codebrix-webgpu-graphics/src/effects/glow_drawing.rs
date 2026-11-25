//! The drawing the glow layer and the particle system share (Not in C#): camera-facing instanced
//! quads from a buffer of [`GlowInstance`] records, one draw call per call of
//! [`GlowDrawing::draw`], with pipelines kept per target format combination and blend.

use std::collections::HashMap;

use glam::{Mat4, Vec4};

use crate::context::{GraphicsContext, GraphicsException};
use crate::drawing3d::Camera;
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

/// The glow shader's uniforms (144 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct GlowGlobals {
    view: Mat4,
    projection: Mat4,
    params: Vec4,
}

const GLOBALS_SIZE: u64 = 144;
const _: () = assert!(std::mem::size_of::<GlowGlobals>() == GLOBALS_SIZE as usize);
// SAFETY: #[repr(C)]; two Mat4 and a Vec4, no padding (size asserted above); any f32 bits are valid.
unsafe impl bytemuck::Zeroable for GlowGlobals {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for GlowGlobals {}

/// What a pipeline is chosen by: the pass's colour and depth formats, its sample count (1, or 4
/// on a multisampled target) and the blend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PipelineKey {
    color: wgpu::TextureFormat,
    depth: Option<wgpu::TextureFormat>,
    samples: u32,
    blend: GlowBlend,
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

    /// Draws `count` instances of `instances` from byte `offset` into the pass, as seen by
    /// `camera`, at clock `time`, blended by `blend`, shaped by `texture` (None: the round
    /// falloff). One instanced draw call.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw(&mut self, pass: &mut RenderPass<'_>, camera: &Camera, time: f64, instances: &GpuBuffer, offset: u64, count: u32, blend: GlowBlend, texture: Option<&GpuTexture>) -> Result<(), GraphicsException> {
        if count == 0 {
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
        let globals = GlowGlobals {
            view: camera.view_matrix(),
            projection: camera.get_projection_matrix(target.aspect_ratio()),
            params: Vec4::new(time as f32, if needs_shader_encoding(target.color_format()) { 1.0 } else { 0.0 }, if texture.is_some() { 1.0 } else { 0.0 }, 0.0),
        };
        let uniform_offset = self.slot * SLOT_SIZE;
        self.uniforms.write_value(&globals, uniform_offset)?;
        self.slot += 1;
        let key = PipelineKey { color: target.color_format(), depth: target.depth_format(), samples: target_sample_count(&target), blend };
        let pipeline = self.pipeline(key)?;
        let texture_group = match texture {
            Some(texture) => self.texture_group(texture),
            None => self.white_group.clone(),
        };
        pass.set_pipeline(&pipeline);
        pass.set_bind_group_with_offsets(0, &self.uniform_group, &[uniform_offset as u32]);
        pass.set_bind_group(1, &texture_group);
        pass.set_vertex_buffer(0, instances, BufferRange { offset, size: Some(count as u64 * INSTANCE_SIZE) });
        pass.draw(6, DrawOptions { instance_count: count, ..DrawOptions::default() });
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
        let layout = VertexLayout::describe(VertexLayoutOptions { step_mode: wgpu::VertexStepMode::Instance, first_location: 0 }).add(wgpu::VertexFormat::Float32x4).add(wgpu::VertexFormat::Float32x4).add(wgpu::VertexFormat::Float32x4).build(INSTANCE_SIZE);
        let target = ColorTargetOptions { custom_blend: Some(key.blend.blend_state()), ..ColorTargetOptions::default() };
        let mut builder = self
            .context
            .create_render_pipeline(&self.shader)
            .with_label(&format!("glow {:?} {:?}", key.color, key.blend))
            .with_vertex_buffer(layout)
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

#[cfg(test)]
mod tests {
    use super::{glow_falloff, pulse_factor, GlowBlend};

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
        let key = |samples| super::PipelineKey { color: wgpu::TextureFormat::Rgba16Float, depth: Some(wgpu::TextureFormat::Depth32Float), samples, blend: GlowBlend::Additive };
        assert_ne!(key(1), key(4));
        let keys: std::collections::HashSet<super::PipelineKey> = crate::presentation::SAMPLE_COUNTS.iter().map(|count| key(*count)).collect();
        assert_eq!(keys.len(), 2);
    }
}
