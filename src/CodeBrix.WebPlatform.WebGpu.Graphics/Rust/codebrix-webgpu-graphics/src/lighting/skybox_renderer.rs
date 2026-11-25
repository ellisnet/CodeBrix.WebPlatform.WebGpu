//! [`SkyboxRenderer`]: draws an [`EnvironmentMap`] as the background (C#: `SkyboxRenderer`).

use std::collections::HashMap;

use glam::{Mat4, Vec4};

use crate::context::{GraphicsContext, GraphicsException};
use crate::drawing3d::{Camera, SceneLighting};
use crate::frames::{DrawOptions, RenderPass};
use crate::lighting::EnvironmentMap;
use crate::models::pbr_data::needs_shader_encoding;
use crate::pipelines::{ColorTargetOptions, DepthOptions, DepthPreset, GpuRenderPipeline};
use crate::presentation::TargetFormats;
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuShader, SamplerDescription};
use crate::shaders::EnvironmentShaders;

/// The bytes of one dynamic-offset slot.
const SLOT_SIZE: u64 = 256;
/// The environments whose bind groups a renderer keeps.
const KEPT_GROUPS: usize = 4;

/// The skybox uniforms (96 bytes; C#: `SkyboxUniforms`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct SkyboxUniforms {
    inverse_view_projection: Mat4,
    params: Vec4,
    lods: Vec4,
}

const UNIFORM_SIZE: u64 = 96;
const _: () = assert!(std::mem::size_of::<SkyboxUniforms>() == UNIFORM_SIZE as usize);
// SAFETY: #[repr(C)]; a Mat4 and two Vec4, no padding (size asserted above); any f32 bits are valid.
unsafe impl bytemuck::Zeroable for SkyboxUniforms {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for SkyboxUniforms {}

/// The defaulted arguments of [`SkyboxRenderer::draw_environment`] (C#: `intensity = 1`,
/// `rotation = 0`, `exposure = 1`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyboxDrawOptions {
    /// Scales the environment's radiance.
    pub intensity: f32,
    /// Turns the environment about +Y, in radians (as [`SceneLighting::environment_rotation`]).
    pub rotation: f32,
    /// Scales the output (as [`SceneLighting::exposure`]).
    pub exposure: f32,
}

impl Default for SkyboxDrawOptions {
    fn default() -> SkyboxDrawOptions {
        SkyboxDrawOptions { intensity: 1.0, rotation: 0.0, exposure: 1.0 }
    }
}

/// Draws an [`EnvironmentMap`] as the background (C#: `SkyboxRenderer`): one full-screen triangle
/// at the far plane whose pixels look the environment up along the camera's view rays, turned,
/// scaled and exposed like the model lighting. Optional: a scene can just as well clear to a
/// colour.
///
/// Draw it in the same pass as the scene, AFTER the opaque models (cheapest: only uncovered
/// pixels are shaded) or before them; it tests depth less-or-equal against the far plane and never
/// writes depth, so the pass must clear depth to 1. Without a depth view it covers the whole
/// target, so draw it first. Colour output matches the [`crate::ModelRenderer`]: linear values
/// into float targets (the HDR path, tone mapped later by [`crate::ToneMapper`]), hardware
/// encoding into sRGB targets (a canvas frame), shader encoding into 8/10-bit UNORM targets.
/// Several draws per frame keep their own uniforms. It keeps the bind groups of the last few
/// environments it drew (and so holds those environments). On a multisampled target it uses its
/// 4-sample pipeline by itself (pipelines are kept per colour format, depth format and sample
/// count; Not in C#).
pub struct SkyboxRenderer {
    context: GraphicsContext,
    shader: GpuShader,
    layout: GpuBindGroupLayout,
    sampler: GpuSampler,
    pipelines: HashMap<TargetFormats, GpuRenderPipeline>,
    groups: Vec<(EnvironmentMap, GpuBindGroup)>,
    uniforms: GpuBuffer,
    capacity: u64,
    slot: u64,
    frame_id: Option<u64>,
    blur: f32,
    draw_count: u64,
}

impl SkyboxRenderer {
    /// A skybox renderer (C#: `new SkyboxRenderer(context)`); pipelines are made per target format
    /// combination on first use.
    pub fn new(context: &GraphicsContext) -> Result<SkyboxRenderer, GraphicsException> {
        let fragment = wgpu::ShaderStages::FRAGMENT;
        let shader = context.create_shader(EnvironmentShaders::SKYBOX, Some("skybox"));
        let layout = context.create_bind_group_layout(
            Some("skybox"),
            &[
                BindingLayoutEntry::uniform(0, fragment).with_dynamic_offset(true).with_min_binding_size(UNIFORM_SIZE),
                BindingLayoutEntry::texture(1, fragment).with_view_dimension(wgpu::TextureViewDimension::Cube),
                BindingLayoutEntry::texture(2, fragment).with_view_dimension(wgpu::TextureViewDimension::Cube),
                BindingLayoutEntry::sampler(3, fragment),
            ],
        );
        let sampler = context.create_sampler(&SamplerDescription::linear_clamp());
        let uniforms = context.create_buffer(8 * SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("skybox"))?;
        Ok(SkyboxRenderer { context: context.clone(), shader, layout, sampler, pipelines: HashMap::new(), groups: Vec::new(), uniforms, capacity: 8, slot: 0, frame_id: None, blur: 0.0, draw_count: 0 })
    }

    /// Blurs the background, 0 (sharp, the radiance cube) to 1 (the roughest prefiltered mip)
    /// (C#: `Blur`). Default 0.
    pub fn blur(&self) -> f32 {
        self.blur
    }

    /// Sets the blur, clamped to 0..1 (C#: `Blur = value`).
    pub fn set_blur(&mut self, value: f32) {
        self.blur = value.clamp(0.0, 1.0);
    }

    /// How many skybox draws the renderer has recorded (C#: `DrawCount`).
    pub fn draw_count(&self) -> u64 {
        self.draw_count
    }

    /// Draws the lighting's environment ([`SceneLighting::environment`], or the context's
    /// [`GraphicsContext::default_environment`]) with its intensity, rotation and exposure - the
    /// look the model renderer lights with; None draws the default environment plainly (C#:
    /// `Draw(renderPass, camera, lighting)`).
    pub fn draw(&mut self, pass: &mut RenderPass<'_>, camera: &Camera, lighting: Option<&SceneLighting>) -> Result<(), GraphicsException> {
        let environment = match lighting.and_then(|l| l.environment.clone()) {
            Some(environment) => environment,
            None => self.context.default_environment()?,
        };
        let options = match lighting {
            Some(lighting) => SkyboxDrawOptions { intensity: lighting.environment_intensity, rotation: lighting.environment_rotation, exposure: lighting.exposure },
            None => SkyboxDrawOptions::default(),
        };
        self.draw_environment(pass, camera, &environment, options)
    }

    /// Draws an environment as the background (C#: `Draw(renderPass, camera, environment,
    /// intensity, rotation, exposure)`).
    pub fn draw_environment(&mut self, pass: &mut RenderPass<'_>, camera: &Camera, environment: &EnvironmentMap, options: SkyboxDrawOptions) -> Result<(), GraphicsException> {
        if self.frame_id != Some(pass.frame_id()) {
            self.frame_id = Some(pass.frame_id());
            self.slot = 0;
        }
        if self.slot >= self.capacity {
            self.capacity *= 2;
            self.uniforms = self.context.create_buffer(self.capacity * SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("skybox"))?;
            self.groups.clear();
            self.slot = 0;
        }
        let target = pass.target().clone();
        let levels = environment.prefiltered_mip_levels().saturating_sub(1) as f32;
        let data = SkyboxUniforms {
            inverse_view_projection: camera.get_view_projection_matrix(target.aspect_ratio()).inverse(),
            params: Vec4::new(options.intensity, options.rotation, options.exposure, if needs_shader_encoding(target.color_format()) { 1.0 } else { 0.0 }),
            lods: Vec4::new(0.0, self.blur * levels, if self.blur > 0.0 { 1.0 } else { 0.0 }, 0.0),
        };
        let offset = self.slot * SLOT_SIZE;
        self.uniforms.write_value(&data, offset)?;
        self.slot += 1;
        let pipeline = self.pipeline(target.formats())?;
        let group = self.group(environment);
        pass.set_pipeline(&pipeline);
        pass.set_bind_group_with_offsets(0, &group, &[offset as u32]);
        pass.draw(3, DrawOptions::default());
        self.draw_count += 1;
        Ok(())
    }

    fn group(&mut self, environment: &EnvironmentMap) -> GpuBindGroup {
        if let Some((_, group)) = self.groups.iter().find(|(kept, _)| kept.id() == environment.id()) {
            return group.clone();
        }
        let group = self.context.create_bind_group(
            &self.layout,
            Some("skybox"),
            &[BindingEntry::for_buffer(0, &self.uniforms).with_range(0, Some(UNIFORM_SIZE)), BindingEntry::for_texture_view(1, environment.radiance()), BindingEntry::for_texture_view(2, environment.prefiltered()), BindingEntry::for_sampler(3, &self.sampler)],
        );
        if self.groups.len() >= KEPT_GROUPS {
            self.groups.remove(0);
        }
        self.groups.push((environment.clone(), group.clone()));
        group
    }

    fn pipeline(&mut self, formats: TargetFormats) -> Result<GpuRenderPipeline, GraphicsException> {
        if let Some(pipeline) = self.pipelines.get(&formats) {
            return Ok(pipeline.clone());
        }
        let (color, depth) = (formats.color, formats.depth);
        let mut builder = self.context.create_render_pipeline(&self.shader).with_label(&format!("skybox {color:?}")).with_color_target(color, ColorTargetOptions::default()).with_bind_group_layouts(&[&self.layout]).with_sample_count(formats.sample_count);
        if let Some(depth) = depth {
            builder = builder.with_depth(depth, DepthOptions { preset: DepthPreset::ReadOnly, ..DepthOptions::default() });
        }
        let pipeline = builder.build()?;
        self.pipelines.insert(formats, pipeline.clone());
        Ok(pipeline)
    }
}

#[cfg(test)]
mod msaa_tests {
    use std::collections::HashMap;

    use crate::presentation::TargetFormats;

    fn formats(sample_count: u32) -> crate::presentation::TargetFormats {
        crate::presentation::TargetFormats::new(wgpu::TextureFormat::Bgra8UnormSrgb, Some(wgpu::TextureFormat::Depth32Float), sample_count)
    }

    #[test]
    fn the_skybox_pipelines_are_kept_per_sample_count() {
        let mut pipelines: HashMap<TargetFormats, u32> = HashMap::new();
        for count in crate::presentation::SAMPLE_COUNTS {
            pipelines.insert(formats(count), count);
        }
        assert_eq!(pipelines.len(), 2);
        assert_eq!(pipelines[&formats(4)], 4);
    }
}
