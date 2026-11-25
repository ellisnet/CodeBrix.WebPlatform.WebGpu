//! [`BloomRenderer`] and [`BloomSettings`]: bloom on the HDR path - what is brighter than a
//! threshold spreads a soft halo around itself (Not in C#).

use std::collections::HashMap;

use glam::{Vec3, Vec4};

use crate::common::Color4;
use crate::context::{GraphicsContext, GraphicsException, Texture2DOptions};
use crate::effects::target_sample_count;
use crate::frames::{DrawOptions, GraphicsFrame};
use crate::pipelines::{ColorTargetOptions, GpuRenderPipeline};
use crate::presentation::RenderTargetView;
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuShader, GpuTexture, SamplerDescription};
use crate::shaders::EffectShaders;

/// The bytes of one dynamic-offset uniform slot.
const SLOT_SIZE: u64 = 256;
/// The bytes the shader reads from a slot.
const UNIFORM_SIZE: u64 = 32;
/// The scene textures whose bind groups a renderer keeps.
const KEPT_GROUPS: usize = 4;
/// The format of the chain's levels.
const CHAIN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The settings of a [`BloomRenderer`] (Not in C#), with defaults that leave ordinary lit
/// surfaces alone and make emissive surfaces, glows, the sun and strong highlights bloom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BloomSettings {
    /// The brightness (the largest of red, green and blue, linear, as the scene target holds it)
    /// above which a pixel blooms. Default 1.5 - above a white surface lit by the default key
    /// light (about 1).
    pub threshold: f32,
    /// How softly bloom starts, 0 (a hard edge at the threshold) to 1 (eased in from 0 up to twice
    /// the threshold), as a fraction of the threshold. Default 0.25 (eased in from 1.125 to 1.875
    /// with the default threshold).
    pub soft_knee: f32,
    /// How strongly the halo is added back; 1 adds the bright part's light once more, spread out.
    /// 0 switches bloom off (nothing is drawn). Default 0.8.
    pub intensity: f32,
    /// The RADIUS as the number of levels of the chain of progressively smaller copies (1 to
    /// [`BloomRenderer::MAX_LEVELS`]): each level doubles the reach of the halo. Default 6. A
    /// small target gets fewer levels (none smaller than 2 x 2).
    pub radius: u32,
}

impl Default for BloomSettings {
    fn default() -> BloomSettings {
        BloomSettings { threshold: 1.5, soft_knee: 0.25, intensity: 0.8, radius: 6 }
    }
}

/// The step a pipeline draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Step {
    /// The bright pass into the first level.
    Bright,
    /// A downsample into the next level.
    Down,
    /// An upsample ADDED into the level above, or into the scene.
    Up,
}

/// The bloom uniforms (32 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct BloomUniforms {
    params: Vec4,
    texel: Vec4,
}

const _: () = assert!(std::mem::size_of::<BloomUniforms>() == UNIFORM_SIZE as usize);
// SAFETY: #[repr(C)]; two Vec4, no padding (size asserted above); any f32 bits are valid.
unsafe impl bytemuck::Zeroable for BloomUniforms {}
// SAFETY: as above; Copy and 'static.
unsafe impl bytemuck::Pod for BloomUniforms {}

/// The chain of progressively smaller copies for one scene size.
struct Chain {
    scene_size: (u32, u32),
    sizes: Vec<(u32, u32)>,
    _texture: GpuTexture,
    views: Vec<wgpu::TextureView>,
    groups: Vec<GpuBindGroup>,
}

/// BLOOM ON THE HDR PATH (Not in C#): what is brighter than [`BloomSettings::threshold`] spreads
/// a soft halo around itself. The application creates one ([`GraphicsContext::create_bloom_renderer`])
/// and calls [`BloomRenderer::apply`] on the scene's RGBA16Float target AFTER the scene pass and
/// BEFORE tone mapping:
///
/// ```ignore
/// // scene pass into hdr.view() ... then:
/// bloom.apply(&mut graphics_frame, hdr.color())?;
/// tone_mapper.apply_in_pass(&mut canvas_pass, hdr.color())?;
/// ```
///
/// It works in passes of its own on the frame: a BRIGHT PASS (what lies above the threshold,
/// eased in over the soft knee) into a half-size copy, a chain of PROGRESSIVELY SMALLER copies
/// (each half the size of the one before, [`BloomSettings::radius`] of them), each blurred on
/// the way down and added back on the way up, and the result ADDED TO THE SCENE texture itself
/// (its alpha is kept). The chain's textures are made at the first `apply` for a size and kept;
/// scenes that never call it pay nothing.
///
/// LIMITS: HDR path only - the scene texture must be a float format (RGBA16Float as made by
/// `HeadlessTargetOptions { format: Rgba16Float, .. }`); an 8-bit target holds nothing above 1
/// to bloom, and `apply` refuses it with a message. Bloom makes bright things LOOK bright; it
/// does not light surfaces. With [`crate::ToneMapper::preserve_alpha`] on (a transparent
/// canvas), the halo shows only over what the scene drew.
pub struct BloomRenderer {
    context: GraphicsContext,
    shader: GpuShader,
    layout: GpuBindGroupLayout,
    sampler: GpuSampler,
    pipelines: HashMap<(Step, wgpu::TextureFormat, u32), GpuRenderPipeline>,
    chain: Option<Chain>,
    scene_groups: Vec<(wgpu::Texture, GpuBindGroup)>,
    uniforms: GpuBuffer,
    capacity: u64,
    slot: u64,
    frame_id: Option<u64>,
    last_level_count: usize,
    /// The settings, read at every [`BloomRenderer::apply`].
    pub settings: BloomSettings,
}

impl BloomRenderer {
    /// The most levels the chain may have.
    pub const MAX_LEVELS: u32 = 8;

    /// A bloom renderer with [`BloomSettings`] defaults. Pipelines are made on first use; the
    /// chain's textures at the first `apply` for a scene size.
    pub fn new(context: &GraphicsContext) -> Result<BloomRenderer, GraphicsException> {
        let fragment = wgpu::ShaderStages::FRAGMENT;
        let shader = context.create_shader(EffectShaders::BLOOM, Some("bloom"));
        let layout = context.create_bind_group_layout(Some("bloom"), &[BindingLayoutEntry::uniform(0, fragment).with_dynamic_offset(true).with_min_binding_size(UNIFORM_SIZE), BindingLayoutEntry::texture(1, fragment), BindingLayoutEntry::sampler(2, fragment)]);
        let sampler = context.create_sampler(&SamplerDescription::linear_clamp());
        let capacity = 2 * BloomRenderer::MAX_LEVELS as u64;
        let uniforms = context.create_buffer(capacity * SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("bloom"))?;
        Ok(BloomRenderer { context: context.clone(), shader, layout, sampler, pipelines: HashMap::new(), chain: None, scene_groups: Vec::new(), uniforms, capacity, slot: 0, frame_id: None, last_level_count: 0, settings: BloomSettings::default() })
    }

    /// The levels of the chain the last `apply` used (0 when it drew nothing).
    pub fn last_level_count(&self) -> usize {
        self.last_level_count
    }

    /// The pipelines made so far.
    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }

    /// Adds bloom to `scene` - the HDR colour texture the scene was drawn into (for example
    /// [`crate::HeadlessRenderTarget::color`]) - in render passes of its own on `frame`, outside
    /// any pass. Call it after the scene pass and before tone mapping. An intensity of 0, or a
    /// target smaller than 4 x 4, draws nothing. Err for a scene texture that is not a float
    /// format or cannot be drawn into and sampled, or that is multisampled: on a 4-sample target
    /// give it the RESOLVED [`crate::HeadlessRenderTarget::color`], after the target's last scene
    /// pass (a later pass on the target would resolve over the bloom). Its own passes are always
    /// single-sampled.
    pub fn apply(&mut self, frame: &mut GraphicsFrame<'_>, scene: &GpuTexture) -> Result<(), GraphicsException> {
        self.last_level_count = 0;
        check_scene_sample_count(scene.sample_count())?;
        check_scene(scene.format(), scene.handle().usage())?;
        let settings = self.settings;
        if settings.intensity <= 0.0 {
            return Ok(());
        }
        let sizes = bloom_chain_sizes(scene.width(), scene.height(), settings.radius);
        if sizes.is_empty() {
            return Ok(());
        }
        self.ensure_chain(scene.width(), scene.height(), sizes)?;
        if self.frame_id != Some(frame.id()) {
            self.frame_id = Some(frame.id());
            self.slot = 0;
        }
        let levels = self.chain.as_ref().map_or(0, |chain| chain.sizes.len());
        let needed = 2 * levels as u64;
        if self.slot + needed > self.capacity {
            // Work already recorded keeps the old buffer (the web backend's Drop does nothing).
            self.capacity = (self.slot + needed).next_power_of_two().max(self.capacity * 2);
            self.uniforms = self.context.create_buffer(self.capacity * SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("bloom"))?;
            self.scene_groups.clear();
            if let Some(chain) = self.chain.as_mut() {
                chain.groups = chain.views.iter().map(|view| create_group(&self.context, &self.layout, &self.uniforms, view, &self.sampler)).collect();
            }
            self.slot = 0;
        }
        let threshold = settings.threshold.max(0.0);
        let knee = settings.soft_knee.clamp(0.0, 1.0);
        let scene_group = self.scene_group(scene);
        let Some(chain) = self.chain.as_ref() else { return Ok(()) };
        let chain_sizes = chain.sizes.clone();
        let views = chain.views.clone();
        let groups = chain.groups.clone();
        let texel = |source: (u32, u32), destination: (u32, u32)| Vec4::new(1.0 / source.0 as f32, 1.0 / source.1 as f32, 1.0 / destination.0 as f32, 1.0 / destination.1 as f32);
        let scene_size = (scene.width(), scene.height());

        // The bright pass: the scene into the first (half-size) level.
        let offset = self.write_slot(BloomUniforms { params: Vec4::new(threshold, knee, 1.0, 0.0), texel: texel(scene_size, chain_sizes[0]) })?;
        self.run(frame, Step::Bright, &views[0], chain_sizes[0], &scene_group, offset, false)?;
        // Down the chain.
        for level in 1..chain_sizes.len() {
            let offset = self.write_slot(BloomUniforms { params: Vec4::new(threshold, knee, 1.0, 0.0), texel: texel(chain_sizes[level - 1], chain_sizes[level]) })?;
            self.run(frame, Step::Down, &views[level], chain_sizes[level], &groups[level - 1], offset, false)?;
        }
        // Back up, each level added into the one above.
        for level in (0..chain_sizes.len() - 1).rev() {
            let offset = self.write_slot(BloomUniforms { params: Vec4::new(threshold, knee, 1.0, 0.0), texel: texel(chain_sizes[level + 1], chain_sizes[level]) })?;
            self.run(frame, Step::Up, &views[level], chain_sizes[level], &groups[level + 1], offset, true)?;
        }
        // Into the scene: the sum of the levels, averaged and scaled by the intensity.
        let scale = composite_scale(settings.intensity, chain_sizes.len());
        let offset = self.write_slot(BloomUniforms { params: Vec4::new(threshold, knee, scale, 0.0), texel: texel(chain_sizes[0], scene_size) })?;
        let scene_target = scene.as_render_target();
        let pipeline = self.pipeline(Step::Up, scene.format(), target_sample_count(&scene_target))?;
        let mut pass = frame.begin_render_pass_to(&scene_target, None, None)?;
        pass.set_pipeline(&pipeline);
        pass.set_bind_group_with_offsets(0, &groups[0], &[offset]);
        pass.draw(3, DrawOptions::default());
        drop(pass);
        self.last_level_count = chain_sizes.len();
        Ok(())
    }

    /// The bright pass on the CPU, the shader's arithmetic (Not in C#): what of a LINEAR colour
    /// blooms under `threshold` and `soft_knee`. For tuning the settings against known colours.
    pub fn bright_pass(color: Vec3, threshold: f32, soft_knee: f32) -> Vec3 {
        let threshold = threshold.max(0.0);
        let knee = threshold * soft_knee.clamp(0.0, 1.0);
        let brightness = color.max_element();
        let mut soft = (brightness - threshold + knee).clamp(0.0, 2.0 * knee);
        soft = soft * soft / (4.0 * knee + 1e-5);
        let contribution = soft.max(brightness - threshold) / brightness.max(1e-5);
        color * contribution
    }

    /// The sizes of the chain's levels for a scene of `width` x `height` pixels and a
    /// [`BloomSettings::radius`] (Not in C#): the first is half the scene, each next one half
    /// the one before, none smaller than 2 pixels on a side; at most `radius` (clamped to 1 to
    /// [`BloomRenderer::MAX_LEVELS`]) levels. Empty for a scene smaller than 4 x 4.
    pub fn chain_sizes(width: u32, height: u32, radius: u32) -> Vec<(u32, u32)> {
        bloom_chain_sizes(width, height, radius)
    }

    fn write_slot(&mut self, data: BloomUniforms) -> Result<u32, GraphicsException> {
        let offset = self.slot * SLOT_SIZE;
        self.uniforms.write_value(&data, offset)?;
        self.slot += 1;
        Ok(offset as u32)
    }

    #[allow(clippy::too_many_arguments)]
    fn run(&mut self, frame: &mut GraphicsFrame<'_>, step: Step, view: &wgpu::TextureView, size: (u32, u32), group: &GpuBindGroup, offset: u32, load: bool) -> Result<(), GraphicsException> {
        let target = RenderTargetView::new(view.clone(), CHAIN_FORMAT, size.0, size.1, None);
        let pipeline = self.pipeline(step, CHAIN_FORMAT, target_sample_count(&target))?;
        let clear = if load { None } else { Some(Color4::TRANSPARENT) };
        let mut pass = frame.begin_render_pass_to(&target, clear, None)?;
        pass.set_pipeline(&pipeline);
        pass.set_bind_group_with_offsets(0, group, &[offset]);
        pass.draw(3, DrawOptions::default());
        Ok(())
    }

    fn ensure_chain(&mut self, width: u32, height: u32, sizes: Vec<(u32, u32)>) -> Result<(), GraphicsException> {
        if let Some(chain) = &self.chain {
            if chain.scene_size == (width, height) && chain.sizes == sizes {
                return Ok(());
            }
        }
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let texture = self.context.create_texture_2d(sizes[0].0, sizes[0].1, CHAIN_FORMAT, usage, Texture2DOptions { mip_level_count: sizes.len() as u32, label: Some("bloom chain".to_string()) })?;
        let views: Vec<wgpu::TextureView> = (0..sizes.len() as u32).map(|level| texture.handle().create_view(&wgpu::TextureViewDescriptor { label: Some("bloom level"), base_mip_level: level, mip_level_count: Some(1), ..wgpu::TextureViewDescriptor::default() })).collect();
        let groups = views.iter().map(|view| create_group(&self.context, &self.layout, &self.uniforms, view, &self.sampler)).collect();
        self.chain = Some(Chain { scene_size: (width, height), sizes, _texture: texture, views, groups });
        Ok(())
    }

    fn scene_group(&mut self, scene: &GpuTexture) -> GpuBindGroup {
        if let Some((_, group)) = self.scene_groups.iter().find(|(kept, _)| kept == scene.handle()) {
            return group.clone();
        }
        let group = create_group(&self.context, &self.layout, &self.uniforms, scene.view(), &self.sampler);
        if self.scene_groups.len() >= KEPT_GROUPS {
            self.scene_groups.remove(0);
        }
        self.scene_groups.push((scene.handle().clone(), group.clone()));
        group
    }

    fn pipeline(&mut self, step: Step, format: wgpu::TextureFormat, samples: u32) -> Result<GpuRenderPipeline, GraphicsException> {
        if let Some(pipeline) = self.pipelines.get(&(step, format, samples)) {
            return Ok(pipeline.clone());
        }
        let (entry, target) = match step {
            Step::Bright => ("fs_bright", ColorTargetOptions::default()),
            Step::Down => ("fs_down", ColorTargetOptions::default()),
            Step::Up => ("fs_up", ColorTargetOptions { custom_blend: Some(add_keeping_alpha()), ..ColorTargetOptions::default() }),
        };
        let pipeline = self
            .context
            .create_render_pipeline(&self.shader)
            .with_label(&format!("bloom {step:?} {format:?}"))
            .with_fragment_entry(entry, None)
            .with_color_target(format, target)
            .with_bind_group_layouts(&[&self.layout])
            .with_sample_count(samples)
            .build()?;
        self.pipelines.insert((step, format, samples), pipeline.clone());
        Ok(pipeline)
    }
}

/// One + one on colour; the destination's alpha kept.
fn add_keeping_alpha() -> wgpu::BlendState {
    use wgpu::{BlendComponent as C, BlendFactor as F, BlendOperation as O};
    wgpu::BlendState { color: C { src_factor: F::One, dst_factor: F::One, operation: O::Add }, alpha: C { src_factor: F::Zero, dst_factor: F::One, operation: O::Add } }
}

fn create_group(context: &GraphicsContext, layout: &GpuBindGroupLayout, uniforms: &GpuBuffer, view: &wgpu::TextureView, sampler: &GpuSampler) -> GpuBindGroup {
    context.create_bind_group(layout, Some("bloom"), &[BindingEntry::for_buffer(0, uniforms).with_range(0, Some(UNIFORM_SIZE)), BindingEntry::for_texture_view(1, view), BindingEntry::for_sampler(2, sampler)])
}

/// The chain's level sizes (see [`BloomRenderer::chain_sizes`]).
pub(crate) fn bloom_chain_sizes(width: u32, height: u32, radius: u32) -> Vec<(u32, u32)> {
    let radius = radius.clamp(1, BloomRenderer::MAX_LEVELS);
    let (mut w, mut h) = (width / 2, height / 2);
    let mut sizes = Vec::new();
    while sizes.len() < radius as usize && w >= 2 && h >= 2 {
        sizes.push((w, h));
        w /= 2;
        h /= 2;
    }
    // The chain is one texture with a mip level per level: its sizes are the mip sizes of the
    // first, which halving with truncation gives exactly.
    sizes
}

/// What the composite multiplies the summed levels by: the intensity over the number of levels
/// (the up chain adds every level once, so their mean is the bright part, spread out).
pub(crate) fn composite_scale(intensity: f32, levels: usize) -> f32 {
    intensity.max(0.0) / levels.max(1) as f32
}

/// Bloom works on the RESOLVED picture: a multisampled scene texture is refused.
pub(crate) fn check_scene_sample_count(sample_count: u32) -> Result<(), GraphicsException> {
    if sample_count > 1 {
        return Err(GraphicsException::new("Bloom works on the resolved scene, and this scene texture is multisampled; give it the HeadlessRenderTarget's color() (what its 4-sample passes resolve into), after the target's last scene pass."));
    }
    Ok(())
}

/// The checks of the scene texture `apply` is given.
pub(crate) fn check_scene(format: wgpu::TextureFormat, usage: wgpu::TextureUsages) -> Result<(), GraphicsException> {
    let float = matches!(format, wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rgba32Float | wgpu::TextureFormat::Rg11b10Ufloat);
    if !float {
        return Err(GraphicsException::new(format!("Bloom works on the HDR path: the scene texture is {format:?}, which holds nothing brighter than 1. Draw the scene into an RGBA16Float target (HeadlessTargetOptions {{ format: Rgba16Float, .. }}), bloom it, then tone map it into the canvas.")));
    }
    if !usage.contains(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING) {
        return Err(GraphicsException::new("The scene texture given to bloom must be drawable and sampled (RENDER_ATTACHMENT and TEXTURE_BINDING): use the colour of a HeadlessRenderTarget or a texture from create_render_texture."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::{bloom_chain_sizes, check_scene, check_scene_sample_count, composite_scale, BloomRenderer, BloomSettings};

    #[test]
    fn the_chain_halves_from_half_the_scene_for_radius_levels() {
        assert_eq!(bloom_chain_sizes(1920, 1080, 6), vec![(960, 540), (480, 270), (240, 135), (120, 67), (60, 33), (30, 16)]);
        // The radius is clamped to 1..=8.
        assert_eq!(bloom_chain_sizes(1920, 1080, 20).len(), 8);
        assert_eq!(bloom_chain_sizes(1920, 1080, 20)[7], (7, 4));
        assert_eq!(bloom_chain_sizes(1920, 1080, 0), vec![(960, 540)]);
        // A small or thin target gets fewer levels, none smaller than 2 x 2.
        assert_eq!(bloom_chain_sizes(16, 16, 6), vec![(8, 8), (4, 4), (2, 2)]);
        assert_eq!(bloom_chain_sizes(4, 4, 6), vec![(2, 2)]);
        assert!(bloom_chain_sizes(3, 300, 6).is_empty());
        assert!(bloom_chain_sizes(0, 0, 6).is_empty());
    }

    #[test]
    fn the_chain_sizes_are_the_mip_sizes_of_the_first_level() {
        let sizes = bloom_chain_sizes(1001, 777, 8);
        let (w0, h0) = sizes[0];
        for (level, size) in sizes.iter().enumerate() {
            assert_eq!(*size, ((w0 >> level).max(1), (h0 >> level).max(1)));
        }
    }

    #[test]
    fn the_bright_pass_keeps_only_what_lies_above_the_threshold_with_a_soft_knee() {
        let (threshold, knee) = (1.0, 0.5);
        // Below threshold - knee: nothing.
        assert_eq!(BloomRenderer::bright_pass(Vec3::splat(0.4), threshold, knee), Vec3::ZERO);
        assert_eq!(BloomRenderer::bright_pass(Vec3::ZERO, threshold, knee), Vec3::ZERO);
        // Above threshold + knee: the part above the threshold, the hue kept.
        let bright = BloomRenderer::bright_pass(Vec3::new(4.0, 2.0, 0.0), threshold, knee);
        assert!((bright - Vec3::new(3.0, 1.5, 0.0)).length() < 1e-4, "{bright}");
        // Continuous at both ends of the knee.
        let at = |b: f32| BloomRenderer::bright_pass(Vec3::splat(b), threshold, knee).x;
        assert!(at(0.5 + 1e-4) < 1e-6);
        assert!((at(1.5 - 1e-4) - at(1.5 + 1e-4)).abs() < 1e-3);
        // Rising through the knee.
        let mut previous = 0.0;
        for step in 0..=200 {
            let value = at(step as f32 * 0.01);
            assert!(value >= previous - 1e-6, "monotonic at {step}");
            previous = value;
        }
        // No knee: a hard threshold.
        assert_eq!(BloomRenderer::bright_pass(Vec3::splat(0.99), 1.0, 0.0), Vec3::ZERO);
        assert!((BloomRenderer::bright_pass(Vec3::splat(1.5), 1.0, 0.0).x - 0.5).abs() < 1e-5);
    }

    #[test]
    fn the_defaults_leave_ordinary_lit_surfaces_alone() {
        let settings = BloomSettings::default();
        assert_eq!((settings.threshold, settings.soft_knee, settings.intensity, settings.radius), (1.5, 0.25, 0.8, 6));
        // A white surface lit by the default key light (about 1) does not bloom.
        assert_eq!(BloomRenderer::bright_pass(Vec3::ONE, settings.threshold, settings.soft_knee), Vec3::ZERO);
        // An emissive surface at 4 does.
        assert!(BloomRenderer::bright_pass(Vec3::splat(4.0), settings.threshold, settings.soft_knee).x > 2.0);
    }

    #[test]
    fn the_composite_averages_the_levels() {
        assert_eq!(composite_scale(0.8, 4), 0.2);
        assert_eq!(composite_scale(-1.0, 4), 0.0);
        assert_eq!(composite_scale(1.0, 0), 1.0);
    }

    #[test]
    fn only_float_scene_textures_that_can_be_drawn_and_sampled_are_taken() {
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC;
        assert!(check_scene(wgpu::TextureFormat::Rgba16Float, usage).is_ok());
        assert!(check_scene(wgpu::TextureFormat::Rgba32Float, usage).is_ok());
        assert!(check_scene(wgpu::TextureFormat::Rgba8UnormSrgb, usage).unwrap_err().message().contains("HDR path"));
        assert!(check_scene(wgpu::TextureFormat::Rgba16Float, wgpu::TextureUsages::TEXTURE_BINDING).unwrap_err().message().contains("RENDER_ATTACHMENT"));
    }

    #[test]
    fn bloom_refuses_a_multisampled_scene_and_keeps_its_own_chain_single_sampled() {
        assert!(check_scene_sample_count(1).is_ok());
        let message = check_scene_sample_count(4).unwrap_err().to_string();
        assert!(message.contains("resolved scene") && message.contains("color()"), "{message}");
    }
}
