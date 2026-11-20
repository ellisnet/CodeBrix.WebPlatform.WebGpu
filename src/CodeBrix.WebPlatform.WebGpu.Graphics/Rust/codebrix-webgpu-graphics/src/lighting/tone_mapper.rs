//! [`ToneMapper`] and [`ToneMapOperator`]: the tone-map pass of the HDR rendering path and the
//! same curves on the CPU (C#: `ToneMapper`, `ToneMapOperator`). Three of the curves follow code
//! published by others (Stephen Hill's ACES fit from the Baking Lab project, Krzysztof
//! Narkowicz's ACES approximation, the Khronos Group's PBR Neutral reference); the package's
//! notices give their sources and licences.

use glam::{Mat3, Vec3, Vec4};

use crate::common::Color4;
use crate::context::{GraphicsContext, GraphicsException};
use crate::frames::{DrawOptions, GraphicsFrame, RenderPass};
use crate::models::pbr_data::needs_shader_encoding;
use crate::pipelines::{BlendPreset, ColorTargetOptions, DepthOptions, DepthPreset, GpuRenderPipeline};
use crate::presentation::RenderTargetView;
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuShader, GpuTexture, SamplerDescription};
use crate::shaders::EnvironmentShaders;

/// The bytes of one dynamic-offset slot.
const SLOT_SIZE: u64 = 256;
/// The source textures whose bind groups a tone mapper keeps.
const KEPT_GROUPS: usize = 4;

/// The curve a [`ToneMapper`] compresses high-dynamic-range colour with (C#: `ToneMapOperator`;
/// the same numbering, which the shader reads).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToneMapOperator {
    /// Stephen Hill's fit of the ACES reference rendering + sRGB output transforms: filmic
    /// contrast, highlights roll off to white, slight hue shifts. The default.
    #[default]
    AcesFitted = 0,
    /// Khronos PBR Neutral: colours up to about 0.8 pass through unchanged (base colours stay
    /// true), highlights compress and desaturate. Best for product and model viewing.
    KhronosPbrNeutral = 1,
    /// Reinhard, c / (1 + c) per channel: cheap and flat; never reaches white.
    Reinhard = 2,
    /// Krzysztof Narkowicz's single-curve ACES approximation (per channel, more saturated than
    /// `AcesFitted`).
    AcesNarkowicz = 3,
    /// No compression: values are clamped to 0..1.
    Clamp = 4,
}

impl ToneMapOperator {
    /// Every operator, in the C# enum's order.
    pub const ALL: [ToneMapOperator; 5] = [ToneMapOperator::AcesFitted, ToneMapOperator::KhronosPbrNeutral, ToneMapOperator::Reinhard, ToneMapOperator::AcesNarkowicz, ToneMapOperator::Clamp];

    /// The operator's C# name ("AcesFitted", ...).
    pub fn name(self) -> &'static str {
        match self {
            ToneMapOperator::AcesFitted => "AcesFitted",
            ToneMapOperator::KhronosPbrNeutral => "KhronosPbrNeutral",
            ToneMapOperator::Reinhard => "Reinhard",
            ToneMapOperator::AcesNarkowicz => "AcesNarkowicz",
            ToneMapOperator::Clamp => "Clamp",
        }
    }
}

/// The tone-map uniforms (32 bytes; C#: `ToneMapUniforms`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct ToneMapUniforms {
    params: Vec4,
    source: Vec4,
}

const UNIFORM_SIZE: u64 = 32;
const _: () = assert!(std::mem::size_of::<ToneMapUniforms>() == UNIFORM_SIZE as usize);
// SAFETY: #[repr(C)]; two Vec4, no padding (size asserted above); any f32 bits are valid.
unsafe impl bytemuck::Zeroable for ToneMapUniforms {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for ToneMapUniforms {}

/// The tone-map pass of the HDR rendering path (C#: `ToneMapper`): reads a linear
/// high-dynamic-range texture (for example the RGBA16Float colour of a
/// [`crate::HeadlessRenderTarget`] the scene was drawn into), multiplies by
/// [`ToneMapper::exposure`], applies [`ToneMapper::operator`] and writes display values into the
/// final target (hardware sRGB encoding for sRGB targets such as a canvas frame, shader encoding
/// for 8/10-bit UNORM targets). [`ToneMapper::map`] is the same curve on the CPU.
///
/// When to use it: draw models and the skybox straight into the canvas (the LDR path) when the
/// lighting stays within about 0..1; render into an RGBA16Float target and tone map when the
/// scene has a real dynamic range (HDRI environments with bright windows or a sun, strong lights,
/// emissive surfaces) or for filmic highlight roll-off. Draw 2D overlays AFTER tone mapping, into
/// the final target. It keeps the bind groups of the last few source textures it read (and so
/// holds those textures).
pub struct ToneMapper {
    context: GraphicsContext,
    shader: GpuShader,
    layout: GpuBindGroupLayout,
    sampler: GpuSampler,
    pipelines: std::collections::HashMap<(wgpu::TextureFormat, Option<wgpu::TextureFormat>, bool), GpuRenderPipeline>,
    groups: Vec<(wgpu::Texture, GpuBindGroup)>,
    uniforms: GpuBuffer,
    capacity: u64,
    slot: u64,
    frame_id: Option<u64>,
    /// The curve (C#: `Operator`). Default [`ToneMapOperator::AcesFitted`].
    pub operator: ToneMapOperator,
    /// Multiplies the linear input before the curve, 2 = one stop brighter (C#: `Exposure`).
    /// Default 1.
    pub exposure: f32,
    /// KEEPS THE SCENE'S ALPHA - for drawing "on the page" (a transparent canvas) or over
    /// layers below with HDR on. The source is read as PREMULTIPLIED colour (what a scene drawn
    /// over a transparent clear holds): the curve is applied to the straight colour and the result
    /// premultiplied by the source's alpha again, and it is BLENDED over the target
    /// (premultiplied-alpha blending) instead of replacing it - so where the scene drew nothing
    /// the target (and the page) shows, and models' edges carry no dark or bright fringe.
    /// [`ToneMapper::apply`] then clears its destination to transparent instead of black.
    /// Default false: the C# behaviour (the curve on the colour as it is, the source's alpha
    /// written as it is, the target replaced). No C# equivalent (the C# library has no alpha
    /// option on its tone mapper).
    pub preserve_alpha: bool,
}

impl ToneMapper {
    /// A tone mapper (C#: `new ToneMapper(context)`).
    pub fn new(context: &GraphicsContext) -> Result<ToneMapper, GraphicsException> {
        let fragment = wgpu::ShaderStages::FRAGMENT;
        let shader = context.create_shader(EnvironmentShaders::TONE_MAP, Some("tone map"));
        let layout = context.create_bind_group_layout(Some("tone map"), &[BindingLayoutEntry::uniform(0, fragment).with_dynamic_offset(true).with_min_binding_size(UNIFORM_SIZE), BindingLayoutEntry::texture(1, fragment), BindingLayoutEntry::sampler(2, fragment)]);
        let sampler = context.create_sampler(&SamplerDescription::linear_clamp());
        let uniforms = context.create_buffer(4 * SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("tone map"))?;
        Ok(ToneMapper { context: context.clone(), shader, layout, sampler, pipelines: std::collections::HashMap::new(), groups: Vec::new(), uniforms, capacity: 4, slot: 0, frame_id: None, operator: ToneMapOperator::AcesFitted, exposure: 1.0, preserve_alpha: false })
    }

    /// Tone maps `source` into `destination` in a render pass of its own that CLEARS the
    /// destination (every pixel is written; the destination's depth view, if any, is not used)
    /// (C#: `Apply(frame, source, destination)`). On a canvas layer that must keep what lies
    /// below, use [`ToneMapper::apply_in_pass`] on a pass that loads.
    pub fn apply(&mut self, frame: &mut GraphicsFrame<'_>, source: &GpuTexture, destination: &RenderTargetView) -> Result<(), GraphicsException> {
        let color_only = RenderTargetView::new(destination.color_view().clone(), destination.color_format(), destination.width(), destination.height(), None);
        let clear = if self.preserve_alpha { Color4::TRANSPARENT } else { Color4::BLACK };
        let mut pass = frame.begin_render_pass_to(&color_only, Some(clear), None)?;
        self.apply_in_pass(&mut pass, source)
    }

    /// Tone maps `source` over the whole target of a pass you began - for example before drawing
    /// sprites in the same pass (C#: `Apply(renderPass, source)`). The pass's depth, if any, is
    /// neither tested nor written; `source` must not be the pass's own target. A source of another
    /// size is scaled with linear filtering.
    pub fn apply_in_pass(&mut self, pass: &mut RenderPass<'_>, source: &GpuTexture) -> Result<(), GraphicsException> {
        if self.frame_id != Some(pass.frame_id()) {
            self.frame_id = Some(pass.frame_id());
            self.slot = 0;
        }
        if self.slot >= self.capacity {
            self.capacity *= 2;
            self.uniforms = self.context.create_buffer(self.capacity * SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("tone map"))?;
            self.groups.clear();
            self.slot = 0;
        }
        let target = pass.target().clone();
        let data = ToneMapUniforms {
            params: Vec4::new(self.exposure, self.operator as u32 as f32, if needs_shader_encoding(target.color_format()) { 1.0 } else { 0.0 }, if self.preserve_alpha { 1.0 } else { 0.0 }),
            source: Vec4::new(source.width() as f32, source.height() as f32, 1.0 / target.width().max(1) as f32, 1.0 / target.height().max(1) as f32),
        };
        let offset = self.slot * SLOT_SIZE;
        self.uniforms.write_value(&data, offset)?;
        self.slot += 1;
        let pipeline = self.pipeline(target.color_format(), target.depth_format(), self.preserve_alpha)?;
        let group = self.group(source);
        pass.set_pipeline(&pipeline);
        pass.set_bind_group_with_offsets(0, &group, &[offset as u32]);
        pass.draw(3, DrawOptions::default());
        Ok(())
    }

    /// The same curve on the CPU (C#: `ToneMapper.Map`): exposure, then the operator, giving
    /// linear display values 0..1 (encode them to sRGB for an 8-bit image). Negative input counts
    /// as 0.
    pub fn map(linear: Vec3, operator: ToneMapOperator, exposure: f32) -> Vec3 {
        let x = (linear * exposure).max(Vec3::ZERO);
        let mapped = match operator {
            ToneMapOperator::AcesFitted => {
                // The shader's matrices, column by column (C# writes the same numbers out row by row).
                let input = Mat3::from_cols(Vec3::new(0.59719, 0.07600, 0.02840), Vec3::new(0.35458, 0.90834, 0.13383), Vec3::new(0.04823, 0.01566, 0.83777));
                let output = Mat3::from_cols(Vec3::new(1.60475, -0.10208, -0.00327), Vec3::new(-0.53108, 1.10813, -0.07276), Vec3::new(-0.07367, -0.00605, 1.07602));
                let v = input * x;
                let a = v * (v + Vec3::splat(0.0245786)) - Vec3::splat(0.000090537);
                let b = v * (0.983729 * v + Vec3::splat(0.4329510)) + Vec3::splat(0.238081);
                output * (a / b)
            }
            ToneMapOperator::KhronosPbrNeutral => {
                const START_COMPRESSION: f32 = 0.8 - 0.04;
                const DESATURATION: f32 = 0.15;
                let low = x.min_element();
                let offset = if low < 0.08 { low - 6.25 * low * low } else { 0.04 };
                let mut color = x - Vec3::splat(offset);
                let peak = color.max_element();
                if peak < START_COMPRESSION {
                    color
                } else {
                    const D: f32 = 1.0 - START_COMPRESSION;
                    let new_peak = 1.0 - D * D / (peak + D - START_COMPRESSION);
                    color *= new_peak / peak;
                    let g = 1.0 - 1.0 / (DESATURATION * (peak - new_peak) + 1.0);
                    color.lerp(Vec3::splat(new_peak), g)
                }
            }
            ToneMapOperator::Reinhard => x / (Vec3::ONE + x),
            ToneMapOperator::AcesNarkowicz => x * (2.51 * x + Vec3::splat(0.03)) / (x * (2.43 * x + Vec3::splat(0.59)) + Vec3::splat(0.14)),
            ToneMapOperator::Clamp => x,
        };
        mapped.clamp(Vec3::ZERO, Vec3::ONE)
    }

    /// What the pass writes with [`ToneMapper::preserve_alpha`] on, for one source pixel of
    /// PREMULTIPLIED linear colour and alpha (the shader's arithmetic on the CPU, for tests):
    /// linear display colour premultiplied by the same alpha.
    #[cfg(test)]
    pub(crate) fn map_preserving_alpha(premultiplied: Vec3, alpha: f32, operator: ToneMapOperator, exposure: f32) -> (Vec3, f32) {
        if alpha <= 0.0 {
            return (Vec3::ZERO, 0.0);
        }
        (ToneMapper::map(premultiplied / alpha.max(1e-6), operator, exposure) * alpha, alpha)
    }

    fn group(&mut self, source: &GpuTexture) -> GpuBindGroup {
        if let Some((_, group)) = self.groups.iter().find(|(kept, _)| kept == source.handle()) {
            return group.clone();
        }
        let group = self.context.create_bind_group(&self.layout, Some("tone map"), &[BindingEntry::for_buffer(0, &self.uniforms).with_range(0, Some(UNIFORM_SIZE)), BindingEntry::for_texture(1, source), BindingEntry::for_sampler(2, &self.sampler)]);
        if self.groups.len() >= KEPT_GROUPS {
            self.groups.remove(0);
        }
        self.groups.push((source.handle().clone(), group.clone()));
        group
    }

    fn pipeline(&mut self, color: wgpu::TextureFormat, depth: Option<wgpu::TextureFormat>, preserve_alpha: bool) -> Result<GpuRenderPipeline, GraphicsException> {
        if let Some(pipeline) = self.pipelines.get(&(color, depth, preserve_alpha)) {
            return Ok(pipeline.clone());
        }
        let target = if preserve_alpha { ColorTargetOptions { blend: BlendPreset::PremultipliedAlpha, ..ColorTargetOptions::default() } } else { ColorTargetOptions::default() };
        let mut builder = self.context.create_render_pipeline(&self.shader).with_label(&format!("tone map {color:?}")).with_color_target(color, target).with_bind_group_layouts(&[&self.layout]);
        if let Some(depth) = depth {
            builder = builder.with_depth(depth, DepthOptions { preset: DepthPreset::None, ..DepthOptions::default() });
        }
        let pipeline = builder.build()?;
        self.pipelines.insert((color, depth, preserve_alpha), pipeline.clone());
        Ok(pipeline)
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::{ToneMapOperator, ToneMapper};

    fn near(actual: f32, expected: f32, tolerance: f32) {
        assert!((actual - expected).abs() <= tolerance, "{actual} is not within {tolerance} of {expected}");
    }

    #[test]
    fn known_values_of_each_operator() {
        // The C# library's known values (ToneMapperTests.known_values_of_each_operator):
        // Reinhard 1 -> 1/2; Narkowicz 1 -> 2.54 / 3.16; Hill's ACES fit maps grey 1 to 0.6191.
        near(ToneMapper::map(Vec3::ONE, ToneMapOperator::Reinhard, 1.0).x, 0.5, 1e-6);
        near(ToneMapper::map(Vec3::ONE, ToneMapOperator::AcesNarkowicz, 1.0).x, 2.54 / 3.16, 1e-5);
        near(ToneMapper::map(Vec3::ONE, ToneMapOperator::AcesFitted, 1.0).x, 0.61911, 2e-4);
        assert_eq!(ToneMapper::map(Vec3::splat(0.25), ToneMapOperator::Clamp, 1.0).x, 0.25);
        assert_eq!(ToneMapper::map(Vec3::splat(7.0), ToneMapOperator::Clamp, 1.0).x, 1.0);
        // Khronos PBR Neutral: below the compression start only the toe offset applies.
        near(ToneMapper::map(Vec3::splat(0.5), ToneMapOperator::KhronosPbrNeutral, 1.0).x, 0.46, 1e-6);
        near(ToneMapper::map(Vec3::splat(0.04), ToneMapOperator::KhronosPbrNeutral, 1.0).x, 0.01, 1e-6);
        let hue = ToneMapper::map(Vec3::new(0.6, 0.3, 0.1), ToneMapOperator::KhronosPbrNeutral, 1.0);
        near(hue.x, 0.56, 1e-6);
        near(hue.y, 0.26, 1e-6);
        near(hue.z, 0.06, 1e-6);
    }

    #[test]
    fn every_operator_is_monotonic_bounded_and_black_at_zero() {
        for operator in ToneMapOperator::ALL {
            assert_eq!(ToneMapper::map(Vec3::ZERO, operator, 1.0), Vec3::ZERO, "{operator:?}");
            assert_eq!(ToneMapper::map(Vec3::splat(-3.0), operator, 1.0), Vec3::ZERO, "{operator:?}");
            let mut previous = 0.0f32;
            let mut value = 0.01f32;
            while value < 200.0 {
                let mapped = ToneMapper::map(Vec3::splat(value), operator, 1.0).x;
                assert!(mapped >= previous - 1e-6, "{operator:?} at {value}");
                assert!((0.0..=1.0).contains(&mapped), "{operator:?} at {value}");
                previous = mapped;
                value *= 1.3;
            }
            if operator != ToneMapOperator::Reinhard {
                assert!(previous > 0.97, "{operator:?} ends at {previous}");
            }
        }
    }

    #[test]
    fn preserving_alpha_maps_the_straight_colour_and_premultiplies_again() {
        // A bright model's edge pixel, half covered over a transparent clear: premultiplied 2.0.
        let (edge, alpha) = ToneMapper::map_preserving_alpha(Vec3::splat(2.0), 0.5, ToneMapOperator::Reinhard, 1.0);
        let full = ToneMapper::map(Vec3::splat(4.0), ToneMapOperator::Reinhard, 1.0);
        assert_eq!(alpha, 0.5);
        near(edge.x, full.x * 0.5, 1e-6);
        // Without the option the curve would see 2.0 and give 0.667 - a bright fringe.
        assert!(ToneMapper::map(Vec3::splat(2.0), ToneMapOperator::Reinhard, 1.0).x > edge.x + 0.2);
        // Nothing drawn stays nothing; opaque pixels are the plain curve.
        assert_eq!(ToneMapper::map_preserving_alpha(Vec3::ONE, 0.0, ToneMapOperator::AcesFitted, 1.0), (Vec3::ZERO, 0.0));
        let (opaque, _) = ToneMapper::map_preserving_alpha(Vec3::splat(0.7), 1.0, ToneMapOperator::AcesFitted, 1.0);
        near(opaque.y, ToneMapper::map(Vec3::splat(0.7), ToneMapOperator::AcesFitted, 1.0).y, 1e-6);
    }

    #[test]
    fn exposure_multiplies_the_input_before_the_curve() {
        assert_eq!(ToneMapper::map(Vec3::splat(0.25), ToneMapOperator::AcesFitted, 4.0), ToneMapper::map(Vec3::ONE, ToneMapOperator::AcesFitted, 1.0));
    }

    #[test]
    fn the_operators_keep_the_csharp_numbering_and_names() {
        assert_eq!(ToneMapOperator::default(), ToneMapOperator::AcesFitted);
        let numbers: Vec<u32> = ToneMapOperator::ALL.iter().map(|o| *o as u32).collect();
        assert_eq!(numbers, vec![0, 1, 2, 3, 4]);
        assert_eq!(ToneMapOperator::KhronosPbrNeutral.name(), "KhronosPbrNeutral");
    }
}
