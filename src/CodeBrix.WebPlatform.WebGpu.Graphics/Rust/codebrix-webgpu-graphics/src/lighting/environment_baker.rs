//! The environment baker (C#: the internal `EnvironmentBaker`): builds [`EnvironmentMap`]s on the
//! GPU and renders the context's BRDF LUT. Every pass is a full-screen triangle into one face
//! (array layer) and mip of an RGBA16Float six-layer texture:
//! 1. radiance mip 0 from the panorama (equirectangular, sampled at the level of detail that
//!    matches the cube's texel size) or from the analytic sky; then the radiance mip chain (the
//!    GPU mipmap generator);
//! 2. irradiance: every texel of one small radiance mip, weighted by solid angle and cosine;
//! 3. prefiltered specular: mip m = GGX roughness m / (levels - 1), filtered importance sampling.
//!
//! The CPU-only parts (option resolution, the sky's uniforms, the half-float upload, the CPU
//! halving of over-wide panoramas) are tested on the host.

use glam::{Vec3, Vec4};

use crate::common::Color4;
use crate::context::{GraphicsContext, GraphicsException, Texture2DOptions};
use crate::frames::{DrawOptions, GraphicsFrame};
use crate::internal::half::f32_to_f16_bits;
use crate::lighting::{EnvironmentMap, EnvironmentOptions, FloatImage, ProceduralSkyOptions};
use crate::pipelines::{ColorTargetOptions, GpuRenderPipeline};
use crate::presentation::RenderTargetView;
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuTexture, SamplerDescription};
use crate::shaders::EnvironmentShaders;

/// The BRDF LUT's size (C#: `EnvironmentBaker.LutSize`).
pub(crate) const LUT_SIZE: u32 = 128;
/// The cube maps' format.
const CUBE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// The bytes of one dynamic-offset slot of the bake uniforms.
const SLOT_SIZE: u64 = 256;

/// The parameters of one bake pass (80 bytes, one 256-byte dynamic-offset slot each; C#:
/// `BakeUniforms`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BakeUniforms {
    pub(crate) face: u32,
    pub(crate) count: u32,
    pub(crate) level: u32,
    pub(crate) size: u32,
    pub(crate) a: Vec4,
    pub(crate) b: Vec4,
    pub(crate) c: Vec4,
    pub(crate) d: Vec4,
}

const BAKE_SIZE: u64 = 80;
const _: () = assert!(std::mem::size_of::<BakeUniforms>() == BAKE_SIZE as usize);
// SAFETY: #[repr(C)]; four u32 then four Vec4 at 16-byte offsets, no padding (size asserted
// above); every bit pattern of u32 and f32 is a valid value.
unsafe impl bytemuck::Zeroable for BakeUniforms {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for BakeUniforms {}

/// The procedural sky's parameters (112 bytes; C#: `SkyUniforms`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SkyUniforms {
    pub(crate) zenith: Vec4,
    pub(crate) horizon: Vec4,
    pub(crate) ground: Vec4,
    pub(crate) sun: Vec4,
    pub(crate) sun_color: Vec4,
    pub(crate) glow: Vec4,
    pub(crate) up: Vec4,
}

const SKY_SIZE: u64 = 112;
const _: () = assert!(std::mem::size_of::<SkyUniforms>() == SKY_SIZE as usize);
// SAFETY: #[repr(C)]; seven Vec4, no padding (size asserted above); any f32 bits are valid.
unsafe impl bytemuck::Zeroable for SkyUniforms {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for SkyUniforms {}

/// The sizes [`EnvironmentOptions`] resolve to on a device (C#: `EnvironmentBaker.Sizes`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Sizes {
    pub(crate) cube: u32,
    pub(crate) irradiance: u32,
    pub(crate) irradiance_source: u32,
    pub(crate) prefiltered: u32,
    pub(crate) prefiltered_levels: u32,
    pub(crate) samples: u32,
}

/// floor(log2(value)) for value >= 1.
fn log2(value: u32) -> u32 {
    31 - value.max(1).leading_zeros()
}

/// Resolves (clamps and rounds) the options for a device whose 2D textures may be
/// `max_dimension` texels wide (0 = unknown) (C#: `Resolve`).
pub(crate) fn resolve(options: &EnvironmentOptions, max_dimension: u32) -> Sizes {
    let limit = if max_dimension == 0 { 4096 } else { max_dimension.min(4096) };
    let limit = 1 << log2(limit);
    let cube = options.cube_size.clamp(16, 4096).next_power_of_two().min(limit);
    let irradiance = options.irradiance_size.clamp(4, 128).next_power_of_two();
    let source = options.irradiance_source_size.clamp(4, 64).next_power_of_two().min(cube);
    let prefiltered = if options.prefiltered_size == 0 { (cube / 2).clamp(16, 256) } else { options.prefiltered_size.clamp(16, 4096).next_power_of_two() };
    let prefiltered = prefiltered.min(cube);
    let levels = options.prefiltered_mip_levels.clamp(2, 8).min(log2(prefiltered).saturating_sub(1));
    let samples = options.specular_sample_count.clamp(8, 4096);
    Sizes { cube, irradiance, irradiance_source: source, prefiltered, prefiltered_levels: levels.max(2), samples }
}

fn rgb(color: Color4) -> Vec3 {
    Vec3::new(color.r, color.g, color.b)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Packs the sky options into the shader's uniforms (C#: `PackSky`).
pub(crate) fn pack_sky(sky: &ProceduralSkyOptions) -> SkyUniforms {
    let haze = ((sky.turbidity - 1.0) / 9.0).clamp(0.0, 1.0);
    let up = if sky.up.length_squared() > 0.0 { sky.up.normalize() } else { Vec3::Y };
    let to_sun = if sky.sun_direction.length_squared() > 0.0 { -sky.sun_direction.normalize() } else { up };
    let radius = sky.sun_angular_radius.clamp(0.002, 0.2);
    let solid_angle = 2.0 * std::f32::consts::PI * (1.0 - radius.cos());
    let disc_radiance = sky.sun_intensity.max(0.0) * std::f32::consts::PI / solid_angle;
    let zenith = rgb(sky.zenith_color).lerp(rgb(sky.horizon_color), haze * 0.35);
    let glow = sky.sun_glow.max(0.0);
    SkyUniforms {
        zenith: zenith.extend(0.0),
        horizon: rgb(sky.horizon_color).extend(lerp(8.0, 1.6, haze)),
        ground: rgb(sky.ground_color).extend(0.0),
        sun: to_sun.extend(radius),
        sun_color: rgb(sky.sun_color).extend(disc_radiance),
        glow: Vec4::new(glow * lerp(6.0, 2.0, haze), lerp(1200.0, 150.0, haze), glow * lerp(0.25, 0.8, haze), lerp(12.0, 3.0, haze)),
        up: up.extend(sky.intensity.max(0.0)),
    }
}

/// A panorama as RGBA16Float texel bytes: negative and NaN values become 0, large ones the half
/// maximum 65504, alpha 1 (C#: `ToHalfRgba`; one thread here).
pub(crate) fn to_half_rgba(image: &FloatImage) -> Vec<u8> {
    let one = f32_to_f16_bits(1.0).to_le_bytes();
    let mut bytes = Vec::with_capacity(image.width() as usize * image.height() as usize * 8);
    for pixel in image.pixels().chunks_exact(3) {
        for &value in pixel {
            bytes.extend_from_slice(&f32_to_f16_bits(to_radiance(value)).to_le_bytes());
        }
        bytes.extend_from_slice(&one);
    }
    bytes
}

/// A sample clamped into what a half holds: negative and NaN to 0, at most 65504.
fn to_radiance(value: f32) -> f32 {
    if value > 0.0 {
        value.min(65504.0)
    } else {
        0.0
    }
}

/// Halves an image (2 x 2 box filter; an odd last row or column is folded into the previous one)
/// (C#: `Halve`; one thread here).
pub(crate) fn halve(image: &FloatImage) -> FloatImage {
    let width = (image.width() / 2).max(1) as usize;
    let height = (image.height() / 2).max(1) as usize;
    let (source_width, source_height) = (image.width() as usize, image.height() as usize);
    let source = image.pixels();
    let mut target = vec![0f32; width * height * 3];
    for y in 0..height {
        let y0 = (y * 2).min(source_height - 1);
        let y1 = (y * 2 + 1).min(source_height - 1);
        for x in 0..width {
            let x0 = (x * 2).min(source_width - 1);
            let x1 = (x * 2 + 1).min(source_width - 1);
            for c in 0..3 {
                let at = |row: usize, column: usize| source[(row * source_width + column) * 3 + c];
                target[(y * width + x) * 3 + c] = 0.25 * (at(y0, x0) + at(y0, x1) + at(y1, x0) + at(y1, x1));
            }
        }
    }
    FloatImage::new(width as u32, height as u32, target).expect("the halved buffer has the halved size")
}

/// The baker's pipelines for one kind of source.
struct BakerStage {
    layout: GpuBindGroupLayout,
    pipeline: GpuRenderPipeline,
}

/// What the context keeps for environments (one per context): the baker's pipelines, the BRDF LUT
/// and the default environment. Only `wgpu` objects and pipelines - never the context itself.
pub(crate) struct EnvironmentCache {
    equirect: Option<(BakerStage, GpuSampler)>,
    sky: Option<BakerStage>,
    irradiance: Option<BakerStage>,
    prefilter: Option<(BakerStage, GpuSampler)>,
    pub(crate) brdf_lut: Option<(wgpu::Texture, wgpu::TextureView)>,
    pub(crate) default_environment: Option<EnvironmentMap>,
}

/// Where a radiance cube's mip 0 comes from.
enum Source<'a> {
    Panorama(&'a GpuTexture, f32),
    Sky(&'a GpuBuffer),
}

impl EnvironmentCache {
    /// An empty cache (everything is made on first use).
    pub(crate) fn new() -> EnvironmentCache {
        EnvironmentCache { equirect: None, sky: None, irradiance: None, prefilter: None, brdf_lut: None, default_environment: None }
    }

    /// Builds an environment from a linear panorama (C#: `FromPanorama`).
    pub(crate) fn from_panorama(&mut self, context: &GraphicsContext, image: &FloatImage, options: &EnvironmentOptions) -> Result<EnvironmentMap, GraphicsException> {
        let max_dimension = context.limits().max_texture_dimension_2d;
        let sizes = resolve(options, max_dimension);
        let max_width = options.max_source_width.max(16).min(if max_dimension == 0 { u32::MAX } else { max_dimension });
        let mut halved: Option<FloatImage> = None;
        loop {
            let current = halved.as_ref().unwrap_or(image);
            if current.width() <= max_width && current.height() <= max_width {
                break;
            }
            halved = Some(halve(current));
        }
        let image = halved.as_ref().unwrap_or(image);
        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT;
        let mips = GpuTexture::full_mip_chain_length(image.width(), image.height());
        let panorama = context.create_texture_2d(image.width(), image.height(), CUBE_FORMAT, usage, Texture2DOptions { mip_level_count: mips, label: Some("environment panorama".to_string()) })?;
        panorama.write_mip_level(0, &to_half_rgba(image))?;
        if mips > 1 {
            context.generate_mipmaps(&panorama)?;
        }
        if self.equirect.is_none() {
            let layout = context.create_bind_group_layout(Some("environment panorama"), &[bake_entry(), BindingLayoutEntry::texture(1, wgpu::ShaderStages::FRAGMENT), BindingLayoutEntry::sampler(2, wgpu::ShaderStages::FRAGMENT)]);
            let sampler = context.create_sampler(&SamplerDescription { address_mode_u: Some(wgpu::AddressMode::Repeat), address_mode_v: Some(wgpu::AddressMode::ClampToEdge), label: Some("environment panorama".to_string()), ..SamplerDescription::default() });
            let pipeline = bake_pipeline(context, EnvironmentShaders::EQUIRECT, "environment panorama", &layout)?;
            self.equirect = Some((BakerStage { layout, pipeline }, sampler));
        }
        let lod = (image.width() as f32 / (4.0 * sizes.cube as f32)).log2().max(0.0);
        let label = options.label.clone().unwrap_or_else(|| "environment".to_string());
        self.build(context, sizes, &label, false, Source::Panorama(&panorama, lod))
    }

    /// Builds the analytic sky (C#: `FromSky`).
    pub(crate) fn from_sky(&mut self, context: &GraphicsContext, sky: &ProceduralSkyOptions, options: &EnvironmentOptions) -> Result<EnvironmentMap, GraphicsException> {
        let sizes = resolve(options, context.limits().max_texture_dimension_2d);
        if self.sky.is_none() {
            let layout = context.create_bind_group_layout(Some("procedural sky"), &[bake_entry(), BindingLayoutEntry::uniform(1, wgpu::ShaderStages::FRAGMENT).with_min_binding_size(SKY_SIZE)]);
            let pipeline = bake_pipeline(context, EnvironmentShaders::PROCEDURAL_SKY, "procedural sky", &layout)?;
            self.sky = Some(BakerStage { layout, pipeline });
        }
        let sky_buffer = context.create_uniform_buffer_from(&pack_sky(sky), Some("procedural sky"))?;
        let label = options.label.clone().unwrap_or_else(|| "procedural sky".to_string());
        self.build(context, sizes, &label, true, Source::Sky(&sky_buffer))
    }

    /// The BRDF LUT, rendered on first use (C#: `BrdfLut`).
    pub(crate) fn brdf_lut(&mut self, context: &GraphicsContext) -> Result<(wgpu::Texture, wgpu::TextureView), GraphicsException> {
        if let Some(lut) = &self.brdf_lut {
            return Ok(lut.clone());
        }
        let format = wgpu::TextureFormat::Rg16Float;
        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
        let lut = context.create_texture_2d(LUT_SIZE, LUT_SIZE, format, usage, Texture2DOptions { mip_level_count: 1, label: Some("BRDF LUT".to_string()) })?;
        let layout = context.create_bind_group_layout(Some("BRDF LUT"), &[bake_entry()]);
        let shader = context.create_shader(EnvironmentShaders::BRDF_LUT, Some("BRDF LUT"));
        let pipeline = context.create_render_pipeline(&shader).with_label("BRDF LUT").with_color_target(format, ColorTargetOptions::default()).with_bind_group_layouts(&[&layout]).build()?;
        let uniforms = context.create_buffer(SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("BRDF LUT"))?;
        uniforms.write_value(&BakeUniforms { size: LUT_SIZE, ..BakeUniforms::default() }, 0)?;
        let group = context.create_bind_group(&layout, Some("BRDF LUT"), &[BindingEntry::for_buffer(0, &uniforms).with_range(0, Some(BAKE_SIZE))]);
        let mut frame = context.begin_frame();
        {
            let mut pass = frame.begin_render_pass_to(&lut.as_render_target(), Some(Color4::TRANSPARENT), None)?;
            pass.set_pipeline(&pipeline);
            pass.set_bind_group_with_offsets(0, &group, &[0]);
            pass.draw(3, DrawOptions::default());
        }
        frame.submit()?;
        let parts = (lut.handle().clone(), lut.view().clone());
        self.brdf_lut = Some(parts.clone());
        Ok(parts)
    }

    /// The three cube maps from a radiance source (C#: `Build`).
    fn build(&mut self, context: &GraphicsContext, sizes: Sizes, label: &str, procedural: bool, source: Source<'_>) -> Result<EnvironmentMap, GraphicsException> {
        self.ensure_filters(context)?;
        let slots = 6 + 6 + 6 * sizes.prefiltered_levels as u64;
        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
        let options = |mips: u32, what: &str| Texture2DOptions { mip_level_count: mips, label: Some(format!("{label} {what}")) };
        let radiance = context.create_texture_array(sizes.cube, sizes.cube, 6, CUBE_FORMAT, usage, options(GpuTexture::full_mip_chain_length(sizes.cube, sizes.cube), "radiance"))?;
        let irradiance = context.create_texture_array(sizes.irradiance, sizes.irradiance, 6, CUBE_FORMAT, usage, options(1, "irradiance"))?;
        let prefiltered = context.create_texture_array(sizes.prefiltered, sizes.prefiltered, 6, CUBE_FORMAT, usage, options(sizes.prefiltered_levels, "prefiltered"))?;
        let uniforms = context.create_buffer(slots * SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some(&format!("{label} bake")))?;

        // 1. Radiance mip 0, then its mip chain.
        {
            let mut frame = context.begin_frame();
            let (stage, group, a) = match source {
                Source::Panorama(panorama, lod) => {
                    let (stage, sampler) = self.equirect.as_ref().expect("made by from_panorama");
                    let group = context.create_bind_group(&stage.layout, Some("environment panorama"), &[BindingEntry::for_buffer(0, &uniforms).with_range(0, Some(BAKE_SIZE)), BindingEntry::for_texture(1, panorama), BindingEntry::for_sampler(2, sampler)]);
                    (stage, group, Vec4::new(lod, 0.0, 0.0, 0.0))
                }
                Source::Sky(sky_buffer) => {
                    let stage = self.sky.as_ref().expect("made by from_sky");
                    let group = context.create_bind_group(&stage.layout, Some("procedural sky"), &[BindingEntry::for_buffer(0, &uniforms).with_range(0, Some(BAKE_SIZE)), BindingEntry::for_buffer(1, sky_buffer)]);
                    (stage, group, Vec4::ZERO)
                }
            };
            for face in 0..6u32 {
                let data = BakeUniforms { face, size: sizes.cube, a, ..BakeUniforms::default() };
                draw(&mut frame, &stage.pipeline, &group, &uniforms, face as u64, &data, &radiance, 0, face, sizes.cube)?;
            }
            frame.submit()?;
        }
        context.generate_mipmaps(&radiance)?;

        // 2. Irradiance, 3. the prefiltered specular mips.
        let mut frame = context.begin_frame();
        let mut slot = 6u64;
        let source_level = log2(sizes.cube) - log2(sizes.irradiance_source);
        let array_view = radiance.handle().create_view(&wgpu::TextureViewDescriptor { label: Some("environment radiance array"), dimension: Some(wgpu::TextureViewDimension::D2Array), array_layer_count: Some(6), ..wgpu::TextureViewDescriptor::default() });
        let stage = self.irradiance.as_ref().expect("made by ensure_filters");
        let irradiance_group = context.create_bind_group(&stage.layout, Some("environment irradiance"), &[BindingEntry::for_buffer(0, &uniforms).with_range(0, Some(BAKE_SIZE)), BindingEntry::for_texture_view(1, &array_view)]);
        for face in 0..6u32 {
            let data = BakeUniforms { face, size: sizes.irradiance, count: sizes.irradiance_source, level: source_level, ..BakeUniforms::default() };
            draw(&mut frame, &stage.pipeline, &irradiance_group, &uniforms, slot, &data, &irradiance, 0, face, sizes.irradiance)?;
            slot += 1;
        }
        let cube_view = radiance.create_cube_view(Some("environment radiance cube"))?;
        let (stage, sampler) = self.prefilter.as_ref().expect("made by ensure_filters");
        let prefilter_group = context.create_bind_group(&stage.layout, Some("environment prefilter"), &[BindingEntry::for_buffer(0, &uniforms).with_range(0, Some(BAKE_SIZE)), BindingEntry::for_texture_view(1, &cube_view), BindingEntry::for_sampler(2, sampler)]);
        let last_level = (radiance.mip_level_count() - 1) as f32;
        let match_level = (sizes.cube as f32 / sizes.prefiltered as f32).log2();
        for level in 0..sizes.prefiltered_levels {
            let size = (sizes.prefiltered >> level).max(1);
            let roughness = level as f32 / (sizes.prefiltered_levels - 1) as f32;
            for face in 0..6u32 {
                let data = BakeUniforms { face, size, count: sizes.samples, level, a: Vec4::new(roughness, sizes.cube as f32, last_level, match_level), ..BakeUniforms::default() };
                draw(&mut frame, &stage.pipeline, &prefilter_group, &uniforms, slot, &data, &prefiltered, level, face, size)?;
                slot += 1;
            }
        }
        frame.submit()?;
        Ok(EnvironmentMap::new(label, radiance.handle().clone(), irradiance.handle().clone(), prefiltered.handle().clone(), procedural))
    }

    fn ensure_filters(&mut self, context: &GraphicsContext) -> Result<(), GraphicsException> {
        if self.irradiance.is_none() {
            let layout = context.create_bind_group_layout(Some("environment irradiance"), &[bake_entry(), BindingLayoutEntry::texture(1, wgpu::ShaderStages::FRAGMENT).with_sample_type(wgpu::TextureSampleType::Float { filterable: false }).with_view_dimension(wgpu::TextureViewDimension::D2Array)]);
            let pipeline = bake_pipeline(context, EnvironmentShaders::IRRADIANCE, "environment irradiance", &layout)?;
            self.irradiance = Some(BakerStage { layout, pipeline });
        }
        if self.prefilter.is_none() {
            let layout = context.create_bind_group_layout(Some("environment prefilter"), &[bake_entry(), BindingLayoutEntry::texture(1, wgpu::ShaderStages::FRAGMENT).with_view_dimension(wgpu::TextureViewDimension::Cube), BindingLayoutEntry::sampler(2, wgpu::ShaderStages::FRAGMENT)]);
            let pipeline = bake_pipeline(context, EnvironmentShaders::PREFILTER, "environment prefilter", &layout)?;
            let sampler = context.create_sampler(&SamplerDescription::linear_clamp());
            self.prefilter = Some((BakerStage { layout, pipeline }, sampler));
        }
        Ok(())
    }
}

/// Binding 0 of every bake layout: the bake uniforms, at a dynamic offset.
fn bake_entry() -> BindingLayoutEntry {
    BindingLayoutEntry::uniform(0, wgpu::ShaderStages::FRAGMENT).with_dynamic_offset(true).with_min_binding_size(BAKE_SIZE)
}

fn bake_pipeline(context: &GraphicsContext, wgsl: &str, label: &str, layout: &GpuBindGroupLayout) -> Result<GpuRenderPipeline, GraphicsException> {
    let shader = context.create_shader(wgsl, Some(label));
    context.create_render_pipeline(&shader).with_label(label).with_color_target(CUBE_FORMAT, ColorTargetOptions::default()).with_bind_group_layouts(&[layout]).build()
}

/// One bake pass: the uniforms into their slot, a full-screen triangle into one face and mip.
#[allow(clippy::too_many_arguments)]
fn draw(frame: &mut GraphicsFrame<'_>, pipeline: &GpuRenderPipeline, group: &GpuBindGroup, uniforms: &GpuBuffer, slot: u64, data: &BakeUniforms, target: &GpuTexture, level: u32, face: u32, size: u32) -> Result<(), GraphicsException> {
    uniforms.write_value(data, slot * SLOT_SIZE)?;
    let view = target.handle().create_view(&wgpu::TextureViewDescriptor {
        label: Some("environment face"),
        dimension: Some(wgpu::TextureViewDimension::D2),
        base_mip_level: level,
        mip_level_count: Some(1),
        base_array_layer: face,
        array_layer_count: Some(1),
        ..wgpu::TextureViewDescriptor::default()
    });
    let mut pass = frame.begin_render_pass_to(&RenderTargetView::new(view, target.format(), size, size, None), Some(Color4::TRANSPARENT), None)?;
    pass.set_pipeline(pipeline);
    pass.set_bind_group_with_offsets(0, group, &[(slot * SLOT_SIZE) as u32]);
    pass.draw(3, DrawOptions::default());
    Ok(())
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::{halve, pack_sky, resolve, to_half_rgba, Sizes};
    use crate::internal::half::f16_bits_to_f32;
    use crate::lighting::{EnvironmentOptions, FloatImage, ProceduralSkyOptions};

    fn options(cube: u32, irradiance: u32, source: u32, prefiltered: u32, levels: u32, samples: u32) -> EnvironmentOptions {
        EnvironmentOptions { cube_size: cube, irradiance_size: irradiance, irradiance_source_size: source, prefiltered_size: prefiltered, prefiltered_mip_levels: levels, specular_sample_count: samples, ..EnvironmentOptions::default() }
    }

    #[test]
    fn options_resolve_to_powers_of_two_within_limits() {
        // The C# library's cases (EnvironmentMapTests.options_resolve_to_powers_of_two_within_limits).
        let cases = [
            (options(500, 32, 32, 0, 6, 128), Sizes { cube: 512, irradiance: 32, irradiance_source: 32, prefiltered: 256, prefiltered_levels: 6, samples: 128 }),
            (options(1024, 64, 64, 0, 9, 9999), Sizes { cube: 1024, irradiance: 64, irradiance_source: 64, prefiltered: 256, prefiltered_levels: 7, samples: 4096 }),
            (options(16, 2, 128, 512, 1, 1), Sizes { cube: 16, irradiance: 4, irradiance_source: 16, prefiltered: 16, prefiltered_levels: 2, samples: 8 }),
            (options(100000, 1000, 1000, 100, 6, 64), Sizes { cube: 4096, irradiance: 128, irradiance_source: 64, prefiltered: 128, prefiltered_levels: 6, samples: 64 }),
        ];
        for (options, expected) in cases {
            assert_eq!(resolve(&options, 8192), expected, "{options:?}");
        }
    }

    #[test]
    fn the_device_limit_caps_the_cube() {
        assert_eq!(resolve(&EnvironmentOptions { cube_size: 4096, ..EnvironmentOptions::default() }, 2048).cube, 2048);
        assert_eq!(resolve(&EnvironmentOptions::default(), 0).cube, 512);
    }

    #[test]
    fn half_conversion_clamps_negative_nan_and_huge_values() {
        let image = FloatImage::new(2, 1, vec![-1.0, f32::NAN, 1e9, 0.5, f32::INFINITY, 2.0]).unwrap();
        let bytes = to_half_rgba(&image);
        let halves: Vec<f32> = bytes.chunks_exact(2).map(|b| f16_bits_to_f32(u16::from_le_bytes([b[0], b[1]]))).collect();
        assert_eq!(halves, vec![0.0, 0.0, 65504.0, 1.0, 0.5, 65504.0, 2.0, 1.0]);
    }

    #[test]
    fn halving_averages_two_by_two() {
        let pixels: Vec<f32> = (0..4 * 2 * 3).map(|i| i as f32).collect();
        let image = FloatImage::new(4, 2, pixels).unwrap();
        let half = halve(&image);
        assert_eq!((half.width(), half.height()), (2, 1));
        let (a, b, c, d) = (image.get_pixel(0, 0).0, image.get_pixel(1, 0).0, image.get_pixel(0, 1).0, image.get_pixel(1, 1).0);
        assert!((half.get_pixel(0, 0).0 - (a + b + c + d) / 4.0).abs() < 1e-6);
    }

    #[test]
    fn the_sky_packs_the_sun_towards_the_light_and_the_haze_terms() {
        let sky = ProceduralSkyOptions::default();
        let packed = pack_sky(&sky);
        // The uniform holds the direction TO the sun, the opposite of the light's travel.
        assert!((packed.sun.truncate() + sky.sun_direction).length() < 1e-6);
        assert_eq!(packed.sun.w, 0.0175);
        // Turbidity 2.5: haze 1/6; the horizon exponent lerps 8 -> 1.6.
        let haze = 1.5 / 9.0;
        assert!((packed.horizon.w - (8.0 + (1.6 - 8.0) * haze)).abs() < 1e-5);
        assert_eq!(packed.sun_color.w, 0.0);
        assert_eq!(packed.up.truncate(), Vec3::Y);
        assert_eq!(packed.up.w, 1.0);
    }

    #[test]
    fn a_sun_intensity_becomes_disc_radiance_over_its_solid_angle() {
        let sky = ProceduralSkyOptions { sun_intensity: 2.0, sun_angular_radius: 0.1, ..ProceduralSkyOptions::default() };
        let packed = pack_sky(&sky);
        let solid_angle = 2.0 * std::f32::consts::PI * (1.0 - 0.1f32.cos());
        assert!((packed.sun_color.w - 2.0 * std::f32::consts::PI / solid_angle).abs() / packed.sun_color.w < 1e-5);
    }
}
