//! [`WaterRenderer`]: draws [`WaterSurface`]s into a pass of a [`HeadlessRenderTarget`], reading
//! the scene drawn before them through a per-frame capture ([`WaterRenderer::capture_scene`]); with
//! [`WaterFrame`] (what a draw is seen and lit by) and [`WaterRenderStats`] (Not in C#).

use std::collections::HashMap;

use glam::Vec4;

use crate::context::{GraphicsContext, GraphicsException, Texture2DOptions};
use crate::drawing3d::{AmbientLighting, Camera, SceneLighting};
use crate::frames::{DrawOptions, GraphicsFrame, RenderPass};
use crate::lighting::local_lights::select_local_lights;
use crate::lighting::EnvironmentMap;
use crate::models::pbr_data::{needs_shader_encoding, PbrFrameUniforms};
use crate::models::SunShadowMap;
use crate::pipelines::{ColorTargetOptions, DepthOptions, DepthPreset, GpuRenderPipeline};
use crate::presentation::{HeadlessRenderTarget, TargetFormats};
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuShader, GpuTexture, SamplerDescription};
use crate::shaders::WaterShaders;
use crate::visibility::CullVolume;
use crate::water::scene_capture::SceneCapture;
use crate::water::water_math::{caustic_phases, detail_phases, detail_waves, draw_refusal, sheen_phases, WaterUniforms, WATER_SLOT_SIZE, WATER_UNIFORM_SIZE};
use crate::water::water_surface::WaterSurface;
use crate::water::OpenWaterWaves;

/// The environment and field bind groups a renderer keeps.
const KEPT_GROUPS: usize = 8;

/// What one [`WaterRenderer::draw`] is seen and lit by (Not in C#).
#[derive(Clone, Copy, Debug)]
pub struct WaterFrame<'a> {
    /// The camera the scene was drawn with.
    pub camera: &'a Camera,
    /// The scene's lighting; None for the [`SceneLighting`] defaults (as `ModelRenderer::begin`).
    pub lighting: Option<&'a SceneLighting>,
    /// The frame's clock time (`frame.info.time`): the detail ripples, caustics, sheen and
    /// open-water waves are exact functions of it (pause and seek are exact).
    pub time: f64,
    /// The sun's shadow map ([`crate::ModelRenderer::sun_shadow_map`]); None for no shadow on the
    /// water.
    pub sun_shadow: Option<&'a SunShadowMap>,
}

/// What a [`WaterRenderer`] did in the current frame (Not in C#; from the frame's capture on).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WaterRenderStats {
    /// Scene captures taken.
    pub captures: usize,
    /// Surfaces drawn.
    pub surfaces: usize,
    /// Vertices drawn (six per grid quad).
    pub vertices: u64,
    /// The size of the capture in pixels (width, height).
    pub capture_size: (u32, u32),
    /// The local lights whose highlights the last surface drew.
    pub local_lights: usize,
}

/// The WATER RENDERER (Not in C#): draws [`WaterSurface`]s into a pass of a
/// [`HeadlessRenderTarget`], reading the scene drawn before them.
///
/// THE FRAME (the offscreen path is required - the HDR target, or an 8-bit one):
/// ```text
/// pass A (clear): models.begin / draw... / end_opaque, then the skybox; end the pass
/// water.capture_scene(&mut frame, &target)          // outside any pass
/// pass B (LOAD colour and depth: begin_render_pass_to(&target.view(), None, None)):
///     water.draw(&mut pass, &WaterFrame { camera, lighting, time, sun_shadow: models.sun_shadow_map().as_ref() }, &surface)
///     models.end_blended(&mut pass)                  // the SAME pair, a later pass
///     glows, particles
/// bloom.apply(&mut frame, target.color()); tone_mapper.apply_in_pass(..)
/// ```
/// Several surfaces share one capture (they do not see each other).
///
/// WHAT EACH PIXEL OF WATER IS: the scene behind it (the capture), REFRACTED by the surface's
/// slopes (a screen-space shift scaled by the path through the water; where the shifted view
/// finds something in front of the water, the unshifted view); TINTED BY THE REAL DEPTH OF WATER
/// (`bottom * exp(-absorption * path) + scattering * (1 - exp(-turbidity * path))`, the path from
/// the surface to the bottom found in the captured depth); with CAUSTICS on the bottom
/// ([`crate::WaterCaustics`], seen through the water only); a Fresnel REFLECTION (Schlick, F0
/// 0.02) of the environment (its prefiltered cube at the look's roughness, or the hemisphere) and,
/// with `screen_reflections` (on by default), of what is ON SCREEN (a 24-step march of the
/// reflected ray against the captured depth, 4 refinement steps); HIGHLIGHTS of the key light
/// (GGX; HDR, so they bloom) and of up to 16 local lights; the sun's SHADOW on the glints and
/// caustics through [`WaterFrame::sun_shadow`]; an optional thin-film SHEEN
/// ([`crate::WaterSheen`], off by default); and a SOFT SHORE where the water is thin. The surface
/// writes its depth (things drawn after it - blended parts, glows, particles - are hidden under
/// it or drawn over it correctly).
///
/// MULTISAMPLING: 1 or 4 samples (pipelines by the pass's [`TargetFormats`]); the capture reads
/// the resolved colour and depth sample 0, and the water's fragment stage runs once per pixel.
///
/// LIMITS, PLAINLY: NOT ON THE PLAIN CANVAS PATH (the canvas picture cannot be read: the capture
/// takes a `HeadlessRenderTarget`); TRANSLUCENT THINGS UNDER THE WATER ARE HIDDEN (they are drawn
/// after the capture, and the water's depth covers them - make them opaque or alpha-tested); NO
/// PLANAR REFLECTION (screen-space only: things off screen, behind the camera or hidden behind
/// other things are not reflected and fall back to the environment; the water is not reflected
/// in itself; the march can show a ragged edge where an object's reflection is cut off); NO
/// SPOTLIGHT SHADOWS ON THE WATER (only the sun's); A SURFACE, NOT A FLUID (no splashes, spray,
/// foam, breaking waves or flow around objects); the camera must be ABOVE the water (no
/// underwater view); refraction is a screen-space shift (just outside the screen's edge nothing
/// is seen through the water; strong slopes can show a seam where the fallback switches);
/// caustics are seen only through the water; the plane is finite.
pub struct WaterRenderer {
    context: GraphicsContext,
    shader: GpuShader,
    capture: SceneCapture,
    uniform_layout: GpuBindGroupLayout,
    capture_layout: GpuBindGroupLayout,
    scene_layout: GpuBindGroupLayout,
    shadow_layout: GpuBindGroupLayout,
    pipelines: HashMap<TargetFormats, GpuRenderPipeline>,
    uniforms: GpuBuffer,
    uniform_group: GpuBindGroup,
    capacity: u64,
    slot: u64,
    frame_id: Option<u64>,
    linear_sampler: GpuSampler,
    comparison_sampler: GpuSampler,
    placeholder_cube: wgpu::TextureView,
    placeholder_field: GpuTexture,
    placeholder_sun: wgpu::TextureView,
    capture_group: Option<(u64, GpuBindGroup)>,
    scene_groups: Vec<(Option<EnvironmentMap>, Option<wgpu::TextureView>, GpuBindGroup)>,
    shadow_groups: Vec<(wgpu::TextureView, GpuBindGroup)>,
    stats: WaterRenderStats,
}

impl WaterRenderer {
    /// A water renderer; its pipelines are made when first needed, one per target format
    /// combination and sample count.
    pub fn new(context: &GraphicsContext) -> Result<WaterRenderer, GraphicsException> {
        let both = wgpu::ShaderStages::VERTEX_FRAGMENT;
        let fragment = wgpu::ShaderStages::FRAGMENT;
        let shader = context.create_shader(WaterShaders::SURFACE, Some("water surface"));
        let uniform_layout = context.create_bind_group_layout(Some("water uniforms"), &[BindingLayoutEntry::uniform(0, both).with_dynamic_offset(true).with_min_binding_size(WATER_UNIFORM_SIZE)]);
        let capture_layout = context.create_bind_group_layout(
            Some("water capture"),
            &[
                BindingLayoutEntry::texture(0, fragment),
                BindingLayoutEntry::texture(1, fragment).with_sample_type(wgpu::TextureSampleType::Float { filterable: false }),
                BindingLayoutEntry::sampler(2, both),
            ],
        );
        let scene_layout = context.create_bind_group_layout(Some("water scene"), &[BindingLayoutEntry::texture(0, fragment).with_view_dimension(wgpu::TextureViewDimension::Cube), BindingLayoutEntry::texture(1, both)]);
        let shadow_layout = context.create_bind_group_layout(
            Some("water sun shadow"),
            &[BindingLayoutEntry::texture(0, fragment).with_sample_type(wgpu::TextureSampleType::Depth), BindingLayoutEntry::sampler(1, fragment).with_sampler_type(wgpu::SamplerBindingType::Comparison)],
        );
        let capacity = 4;
        let uniforms = context.create_buffer(capacity * WATER_SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("water uniforms"))?;
        let uniform_group = create_uniform_group(context, &uniform_layout, &uniforms);
        let linear_sampler = context.create_sampler(&SamplerDescription::linear_clamp());
        let comparison_sampler = context.create_sampler(&SamplerDescription { compare: Some(wgpu::CompareFunction::LessEqual), mipmap_filter: wgpu::MipmapFilterMode::Nearest, max_lod: 0.0, label: Some("water sun shadow".to_string()), ..SamplerDescription::linear_clamp() });
        let cube = context.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("water environment placeholder"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 6 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let placeholder_cube = cube.create_view(&wgpu::TextureViewDescriptor { label: Some("water environment placeholder"), dimension: Some(wgpu::TextureViewDimension::Cube), ..wgpu::TextureViewDescriptor::default() });
        // A flat field (a new texture reads 0): height, slopes and Laplacian all 0.
        let placeholder_field = context.create_texture_2d(1, 1, wgpu::TextureFormat::Rgba16Float, wgpu::TextureUsages::TEXTURE_BINDING, Texture2DOptions { mip_level_count: 1, label: Some("water field placeholder".to_string()) })?;
        let sun = context.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("water sun shadow placeholder"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let placeholder_sun = sun.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(WaterRenderer {
            context: context.clone(),
            shader,
            capture: SceneCapture::new(context),
            uniform_layout,
            capture_layout,
            scene_layout,
            shadow_layout,
            pipelines: HashMap::new(),
            uniforms,
            uniform_group,
            capacity,
            slot: 0,
            frame_id: None,
            linear_sampler,
            comparison_sampler,
            placeholder_cube,
            placeholder_field,
            placeholder_sun,
            capture_group: None,
            scene_groups: Vec::new(),
            shadow_groups: Vec::new(),
            stats: WaterRenderStats::default(),
        })
    }

    /// The context.
    pub fn context(&self) -> &GraphicsContext {
        &self.context
    }

    /// Copies what `target` holds so far - the colour as resolved and the depth - for the surfaces
    /// drawn after it this frame. Call it outside any pass, after the pass with the opaque parts
    /// and the sky (and before the pass that draws the water: a pass on the same target that LOADS
    /// colour and depth). Err for a target without `Depth32Float` depth (the message says how to
    /// make one). Records a texture copy and one small full-screen pass into `frame`.
    pub fn capture_scene(&mut self, frame: &mut GraphicsFrame<'_>, target: &HeadlessRenderTarget) -> Result<(), GraphicsException> {
        self.start_frame(frame.id());
        self.capture.capture(frame, target)?;
        self.stats.captures += 1;
        self.stats.capture_size = (target.width(), target.height());
        Ok(())
    }

    /// Draws one surface into `pass` (a pass on the target captured this frame). Err without a
    /// capture this frame, for a pass on another target, or for a grid outside 1 to
    /// [`WaterSurface::MAX_GRID`]. One draw call of `6 * grid * grid` vertices.
    pub fn draw(&mut self, pass: &mut RenderPass<'_>, frame: &WaterFrame<'_>, surface: &WaterSurface) -> Result<(), GraphicsException> {
        if let Some(message) = draw_refusal(self.capture.captured_frame(), pass.frame_id(), self.capture.is_target(pass.target())) {
            return Err(GraphicsException::new(message));
        }
        if !(1..=WaterSurface::MAX_GRID).contains(&surface.grid) {
            return Err(GraphicsException::new(format!("A water surface's grid is 1 to {} quads per side; this one has {}.", WaterSurface::MAX_GRID, surface.grid)));
        }
        self.start_frame(pass.frame_id());
        let defaults = SceneLighting::default();
        let lighting = frame.lighting.unwrap_or(&defaults);
        let environment = match (lighting.ambient, &lighting.environment) {
            (AmbientLighting::Environment, Some(environment)) => Some(environment.clone()),
            (AmbientLighting::Environment, None) => Some(self.context.default_environment()?),
            (AmbientLighting::Hemisphere, _) => None,
        };
        let target = pass.target().clone();
        let picture = PackTarget { format: target.color_format(), width: target.width(), height: target.height() };
        let (uniforms, local_lights) = pack_uniforms(frame.camera, lighting, environment.as_ref().map(EnvironmentMap::prefiltered_mip_levels), surface, frame.time, frame.sun_shadow, picture);
        if self.slot >= self.capacity {
            // Work already recorded keeps the old buffer (the web backend's Drop does nothing).
            self.capacity *= 2;
            self.uniforms = self.context.create_buffer(self.capacity * WATER_SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("water uniforms"))?;
            self.uniform_group = create_uniform_group(&self.context, &self.uniform_layout, &self.uniforms);
            self.slot = 0;
        }
        let offset = self.slot * WATER_SLOT_SIZE;
        self.uniforms.write_value(&uniforms, offset)?;
        self.slot += 1;
        let pipeline = self.pipeline(target.formats())?;
        let capture_group = self.capture_group();
        let scene_group = self.scene_group(environment.as_ref(), surface.field.as_ref().map(|field| &field.view));
        let shadow_group = self.shadow_group(frame.sun_shadow.map(|sun| &sun.view));
        pass.set_pipeline(&pipeline);
        pass.set_bind_group_with_offsets(0, &self.uniform_group, &[offset as u32]);
        pass.set_bind_group(1, &capture_group);
        pass.set_bind_group(2, &scene_group);
        pass.set_bind_group(3, &shadow_group);
        let vertices = 6 * surface.grid * surface.grid;
        pass.draw(vertices, DrawOptions::default());
        self.stats.surfaces += 1;
        self.stats.vertices += vertices as u64;
        self.stats.local_lights = local_lights;
        Ok(())
    }

    /// What the renderer did in the current frame.
    pub fn last_stats(&self) -> WaterRenderStats {
        self.stats
    }

    /// The pipelines made so far: one per target format combination and sample count for the
    /// surface, plus the capture's (at most two: 1 and 4 samples).
    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len() + self.capture.pipeline_count()
    }

    fn start_frame(&mut self, frame_id: u64) {
        if self.frame_id != Some(frame_id) {
            self.frame_id = Some(frame_id);
            self.slot = 0;
            self.stats = WaterRenderStats::default();
        }
    }

    fn pipeline(&mut self, formats: TargetFormats) -> Result<GpuRenderPipeline, GraphicsException> {
        if let Some(pipeline) = self.pipelines.get(&formats) {
            return Ok(pipeline.clone());
        }
        let mut builder = self
            .context
            .create_render_pipeline(&self.shader)
            .with_label(&format!("water surface {formats}"))
            .with_color_target(formats.color, ColorTargetOptions::default())
            .with_bind_group_layouts(&[&self.uniform_layout, &self.capture_layout, &self.scene_layout, &self.shadow_layout])
            .with_sample_count(formats.sample_count);
        if let Some(depth) = formats.depth {
            // The water hides what lies under it and is hidden by what is above it; it writes its
            // depth, so things drawn after it sort against it.
            builder = builder.with_depth(depth, DepthOptions { preset: DepthPreset::ReadWriteLessEqual, ..DepthOptions::default() });
        }
        let pipeline = builder.build()?;
        self.pipelines.insert(formats, pipeline.clone());
        Ok(pipeline)
    }

    fn capture_group(&mut self) -> GpuBindGroup {
        let copies = self.capture.copies().expect("a capture this frame made the copies");
        if let Some((generation, group)) = &self.capture_group {
            if *generation == copies.generation {
                return group.clone();
            }
        }
        let group = self.context.create_bind_group(
            &self.capture_layout,
            Some("water capture"),
            &[BindingEntry::for_texture(0, &copies.color), BindingEntry::for_texture(1, &copies.depth), BindingEntry::for_sampler(2, &self.linear_sampler)],
        );
        self.capture_group = Some((copies.generation, group.clone()));
        group
    }

    fn scene_group(&mut self, environment: Option<&EnvironmentMap>, field: Option<&wgpu::TextureView>) -> GpuBindGroup {
        let environment_id = environment.map(EnvironmentMap::id);
        if let Some((_, _, group)) = self.scene_groups.iter().find(|(e, f, _)| e.as_ref().map(EnvironmentMap::id) == environment_id && f.as_ref() == field) {
            return group.clone();
        }
        let cube = environment.map_or(&self.placeholder_cube, |e| e.prefiltered());
        let field_view = field.unwrap_or(self.placeholder_field.view());
        let group = self.context.create_bind_group(&self.scene_layout, Some("water scene"), &[BindingEntry::for_texture_view(0, cube), BindingEntry::for_texture_view(1, field_view)]);
        if self.scene_groups.len() >= KEPT_GROUPS {
            self.scene_groups.remove(0);
        }
        self.scene_groups.push((environment.cloned(), field.cloned(), group.clone()));
        group
    }

    fn shadow_group(&mut self, sun: Option<&wgpu::TextureView>) -> GpuBindGroup {
        let view = sun.unwrap_or(&self.placeholder_sun).clone();
        if let Some((_, group)) = self.shadow_groups.iter().find(|(kept, _)| *kept == view) {
            return group.clone();
        }
        let group = self.context.create_bind_group(&self.shadow_layout, Some("water sun shadow"), &[BindingEntry::for_texture_view(0, &view), BindingEntry::for_sampler(1, &self.comparison_sampler)]);
        if self.shadow_groups.len() >= KEPT_GROUPS {
            self.shadow_groups.remove(0);
        }
        self.shadow_groups.push((view, group.clone()));
        group
    }
}

fn create_uniform_group(context: &GraphicsContext, layout: &GpuBindGroupLayout, uniforms: &GpuBuffer) -> GpuBindGroup {
    context.create_bind_group(layout, Some("water uniforms"), &[BindingEntry::for_buffer(0, uniforms).with_range(0, Some(WATER_UNIFORM_SIZE))])
}

/// What the packing needs of the pass's target: its colour format and size in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PackTarget {
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// The uniforms of one surface's draw, and how many local lights it highlights. `environment`
/// is the environment's prefiltered mip count (None: the hemispheric ambient).
pub(crate) fn pack_uniforms(camera: &Camera, lighting: &SceneLighting, environment: Option<u32>, surface: &WaterSurface, time: f64, sun: Option<&SunShadowMap>, target: PackTarget) -> (WaterUniforms, usize) {
    let aspect = if target.height == 0 { 1.0 } else { target.width as f32 / target.height as f32 };
    let mut frame = PbrFrameUniforms::new(camera, lighting, aspect);
    let view_projection = frame.view_projection;
    let look = &surface.look;
    let chosen = if look.local_light_highlights { select_local_lights(&lighting.local_lights, camera.position, &CullVolume::from_view_projection(view_projection), SceneLighting::MAX_LOCAL_LIGHTS) } else { Vec::new() };
    frame.set_local_lights(&lighting.local_lights, &chosen, &[], &[]);
    let mut u = WaterUniforms { view_projection, inverse_view_projection: view_projection.inverse(), ..WaterUniforms::default() };
    u.camera = camera.position.extend(lighting.exposure);
    let (width, height) = (target.width.max(1) as f32, target.height.max(1) as f32);
    u.viewport = Vec4::new(width, height, 1.0 / width, 1.0 / height);
    if PbrFrameUniforms::key_light_shades(lighting) {
        u.key_direction = lighting.light_direction.normalize().extend(1.0);
        u.key_color = frame.lights[1];
    }
    if let Some(levels) = environment {
        u.environment = Vec4::new(1.0, lighting.environment_intensity, levels.saturating_sub(1) as f32, lighting.environment_rotation);
    } else {
        u.environment = Vec4::new(0.0, 0.0, 0.0, lighting.environment_rotation);
    }
    u.ambient_sky = frame.ambient_sky;
    u.ambient_ground = frame.ambient_ground;
    u.ambient_up = frame.ambient_up.truncate().extend(0.0);
    u.surface = surface.center.extend(surface.rotation);
    u.extent = Vec4::new(surface.size.x, surface.size.y, surface.grid.max(1) as f32, surface.grid_cell());
    match &surface.field {
        Some(field) => {
            u.field = Vec4::new(1.0, field.height_scale, if field.displace { 1.0 } else { 0.0 }, 0.0);
            u.field_extent = field.size.extend(0.0).extend(0.0);
        }
        None => u.field_extent = surface.size.extend(0.0).extend(0.0),
    }
    u.absorption = look.absorption.extend(look.turbidity);
    u.scattering = look.scattering_color.extend(look.refraction);
    u.look = Vec4::new(look.reflectivity, look.roughness, look.sun_glint, look.soft_shore);
    if let Some(caustics) = look.caustics {
        u.caustics = Vec4::new(caustics.strength.max(0.0), caustics.scale, caustics.focus, caustics.depth_fade);
        let (a, b) = caustic_phases(time, caustics.speed);
        u.caustic_phase = Vec4::new(a, b, 0.0, 0.0);
    }
    let detail = &surface.detail;
    u.detail_waves = detail_waves(detail.strength, detail.wavelength, detail.seed);
    u.detail_phase = detail_phases(&u.detail_waves, detail.speed, time);
    let sheen = &look.sheen;
    u.sheen = Vec4::new(sheen.strength.max(0.0), sheen.scale, sheen.slope_gain, sheen.height_gain);
    let (drift, band) = sheen_phases(time, sheen.drift);
    u.sheen_phase = Vec4::new(drift, band, 0.0, 0.0);
    let shadowed = look.receives_shadows && sun.is_some();
    u.switches = [look.screen_reflections as u32, look.local_light_highlights as u32, shadowed as u32, needs_shader_encoding(target.format) as u32];
    let waves = surface.open_waves.as_ref().map(|w| w.records(time)).unwrap_or_default();
    let wave_count = waves.len().min(OpenWaterWaves::MAX_WAVES);
    for (i, record) in waves.iter().take(wave_count).enumerate() {
        u.waves[i * 2] = record.direction;
        u.waves[i * 2 + 1] = record.motion;
    }
    let local_count = frame.local.x as usize;
    u.local_lights = frame.local_lights;
    u.counts = [wave_count as u32, local_count as u32, 0, detail.seed];
    if let (true, Some(sun)) = (shadowed, sun) {
        u.counts[2] = sun.softness.clamp(1, 5);
        u.sun_matrix = sun.matrix;
        u.sun_shadow = Vec4::new(sun.strength.clamp(0.0, 1.0), sun.texel, sun.depth_bias, sun.normal_offset);
    }
    (u, local_count)
}

impl GraphicsContext {
    /// A renderer that draws [`WaterSurface`]s (Not in C#).
    pub fn create_water_renderer(&self) -> Result<WaterRenderer, GraphicsException> {
        WaterRenderer::new(self)
    }
}
