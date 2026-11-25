//! [`ModelRenderer`]: draws [`ModelInstance`]s with the glTF metallic-roughness shader (C#:
//! `ModelRenderer` and its internal `PbrModelRenderPath`).

use std::collections::HashMap;

use glam::{Mat4, Vec3, Vec4};

use crate::context::{GraphicsContext, GraphicsException};
use crate::drawing3d::{AmbientLighting, Camera, SceneLighting};
use crate::lighting::local_lights::select_local_lights;
use crate::lighting::shadows::{select_shadowed_spots, spot_shadow_view, sun_shadow_view, ShadowView};
use crate::lighting::EnvironmentMap;
use crate::frames::{BufferRange, DrawIndexedOptions, RenderPass};
use crate::models::lod_instance::{lod_culled, lod_draws};
use crate::models::model_draw_plan::{cull_passes, plan, Bucket, DrawPlan, FaceSide, InstanceLook, PbrDraw, PlanModel, PlannedDraw};
use crate::models::model_shadows::{sun_shadow_numbers, ModelShadows, ShadowTarget, SunShadowMap};
use crate::models::pbr_data::{material_features, needs_shader_encoding, pack_morph, pipeline_culling, PbrDrawUniforms, PbrFrameUniforms, PbrInstance, PbrMaterialUniforms, DRAW_UNIFORM_SIZE, FRAME_SLOT_SIZE, FRAME_UNIFORM_SIZE, INSTANCE_SIZE, MATERIAL_UNIFORM_SIZE};
use crate::models::{LodInstance, LodState, Model, ModelInstance, ModelMaterial, ModelVertex};
use crate::pipelines::{BlendPreset, ColorTargetOptions, DepthOptions, DepthPreset, GpuRenderPipeline, VertexLayoutOptions};
use crate::presentation::TargetFormats;
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuShader, GpuTexture, SamplerDescription};
use crate::shaders::{PbrFeatures, PbrShader};
use crate::visibility::CullVolume;

/// The bytes of one dynamic-offset slot (the WebGPU minimum alignment).
const SLOT_SIZE: u64 = 256;
/// The bytes of one matrix in the matrix storage buffer.
const MATRIX_SIZE: u64 = 64;

/// A material's GPU state (C#: `PbrMaterial`).
struct PbrMaterial {
    _uniforms: GpuBuffer,
    group: GpuBindGroup,
    features: PbrFeatures,
}

/// What the renderer keeps per model: materials, draw items with their uniforms, morph deltas
/// and the model bind group (C#: `PbrModelState`).
struct PbrModelState {
    model: Model,
    materials: Vec<PbrMaterial>,
    fallback: PbrMaterial,
    draws: Vec<PbrDraw>,
    /// The morph targets' centre shifts the draws' `morph` ranges index (the translucent sort).
    morph_shifts: Vec<Vec3>,
    group: GpuBindGroup,
    _draw_uniforms: GpuBuffer,
    _morph_deltas: Option<GpuBuffer>,
}

impl PbrModelState {
    fn material(&self, draw: &PbrDraw) -> &PbrMaterial {
        draw.material.and_then(|m| self.materials.get(m)).unwrap_or(&self.fallback)
    }
}

/// The frame bind groups a renderer keeps (one per environment and shadow-map set it lit with,
/// besides the hemispheric one without shadows).
const KEPT_FRAME_GROUPS: usize = 8;

/// What `begin` was given, until `end`.
struct Drawing {
    frame_id: u64,
    frame: PbrFrameUniforms,
    environment: Option<EnvironmentMap>,
    camera_position: Vec3,
    camera_forward: Vec3,
    /// The camera's view on the pass's target, what `draw` culls against.
    volume: CullVolume,
    /// The shader features of the frame (Not in C#): LOCAL_LIGHTS when it has local lights,
    /// SHADOWS when it has shadow passes.
    features: PbrFeatures,
    /// The pair's shadow passes, the sun's first (Not in C#).
    shadow_passes: Vec<(ShadowView, ShadowTarget)>,
    /// The shadow-map set of the pair, when it has shadow passes.
    shadow_set: Option<usize>,
    /// The upload and the draws, worked out by the first finishing call.
    prepared: Option<Prepared>,
}

/// The queued instances, uploaded, and the draws still to record.
struct Prepared {
    plan: DrawPlan,
    /// The model of each bucket of the plan.
    model_ids: Vec<u64>,
    frame_offset: u32,
    /// The instance buffer's record of the plan's first instance.
    base: u32,
    opaque_drawn: bool,
}

/// What a [`ModelRenderer`] drew in one frame, summed over every begin/end pair of the frame
/// ([`ModelRenderer::frame_stats`]; Not in C#).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModelRenderStats {
    /// Draw calls issued (both halves of every pair; a blended double-sided part counts two).
    pub draw_calls: usize,
    /// Primitive instances drawn (one per part and instance drawn, however many draws it took).
    pub instances: usize,
    /// Instances culled: queued with `draw` but entirely outside the camera's view, so not drawn
    /// in it (an instance that still casts into a shadow map is uploaded for its shadow pass;
    /// any other is neither uploaded nor drawn).
    pub culled_instances: usize,
    /// Shadow passes drawn (the sun's and the spotlights'; Not in C#).
    pub shadow_passes: usize,
    /// Draw calls issued in the shadow passes (Not in C#).
    pub shadow_draw_calls: usize,
    /// Level-of-detail objects ([`ModelRenderer::draw_lod`]) left out whole by culling, before any
    /// distance was measured (Not in C#).
    pub culled_lods: usize,
    /// Level-of-detail objects drawn inside a fade band - two levels, or the last one fading out
    /// (Not in C#).
    pub fading_lods: usize,
    /// The local lights (point lights and spotlights) the renderer chose to shade, at most
    /// [`SceneLighting::MAX_LOCAL_LIGHTS`]: the MOST any begin/end pair of the frame chose (pairs
    /// usually light with the same lights, so they are not added up; Not in C#). Per pair:
    /// [`ModelRenderer::last_local_light_count`].
    pub local_lights: usize,
    /// The spotlights among them that cast shadows this frame (each drawn into a layer of the
    /// spotlights' shadow map), at most [`SceneLighting::MAX_SHADOWED_SPOTLIGHTS`]: the most any
    /// pair of the frame chose (Not in C#). Per pair: [`ModelRenderer::last_shadowed_spotlight_count`].
    pub shadowed_spotlights: usize,
}

/// Draws [`ModelInstance`]s (C#: `ModelRenderer`). [`ModelRenderer::begin`] with a camera and
/// lighting, [`ModelRenderer::draw`] any number of instances of any models, then
/// [`ModelRenderer::end`]: instances of the same model are batched, so each primitive becomes ONE
/// instanced draw for all of them. Opaque and alpha-tested primitives draw first, blended ones
/// after (depth read-only, sorted back to front by where each part is NOW: a skinned part by its
/// skin's current joints, a morphing part by its current weights).
///
/// THE FINISHING STEP IN TWO HALVES (Not in C#): [`ModelRenderer::end`] is
/// [`ModelRenderer::end_opaque`] (the opaque and alpha-tested parts of everything queued) then
/// [`ModelRenderer::end_blended`] (the blended parts). Call the two yourself to draw something
/// between them - the skybox, so that blended parts with only sky behind them blend over the sky
/// instead of being covered by it: `begin`, `draw`..., `end_opaque(pass)`, the skybox,
/// `end_blended(pass)`. One model may mix opaque and blended parts.
///
/// CULLING (Not in C#; ON by default, [`ModelRenderer::set_frustum_culling`]): `draw` leaves out
/// an instance whose [`Model::culling_bounds`] (its rest bounds grown at load to hold every pose
/// of its clips), placed by its world matrix, are entirely outside the camera's view - no pose
/// upload, no draw - and counts it ([`ModelRenderer::last_culled_instance_count`],
/// [`ModelRenderer::frame_stats`]). [`ModelInstance::never_cull`] exempts one instance. The test
/// is [`CullVolume::intersects_transformed_box`] against [`ModelRenderer::view_volume`].
///
/// TWO-PASS DOUBLE-SIDED BLENDED PARTS (Not in C#; ON by default,
/// [`ModelRenderer::set_two_pass_double_sided`]): a part whose material is both blended and
/// double-sided is drawn twice in its place in the back-to-front order, first only its back faces,
/// then only its front faces, so its far side blends under its near side (exact for convex
/// shapes, better for others).
///
/// PER INSTANCE (Not in C#): [`ModelInstance::tint`], [`ModelInstance::alpha`] (below 1 the whole
/// instance draws with the blended parts) and [`ModelInstance::emissive_strength`];
/// [`ModelInstance::set_node_visible`] hides single nodes. Plain instances of a model stay one
/// draw per primitive; an instance with a hidden node or alpha below 1 leaves the shared draw only
/// for the parts concerned.
///
/// Shading is glTF 2.0 metallic-roughness with the shader the crate carries ([`PbrShader`]): base
/// colour factor and texture, metallic-roughness, normal map, occlusion, emissive, vertex colours,
/// KHR_texture_transform and the texture-coordinate set per texture, unlit, alpha modes OPAQUE,
/// MASK and BLEND, double-sided, the ior/specular/clear-coat factors; skinned primitives are
/// skinned on the GPU from each instance's joint palette and morph targets use its weights.
/// Mirroring transforms draw with clockwise front faces. Lighting comes from [`SceneLighting`]:
/// up to eight directional lights, the exposure, and the ambient term - IMAGE-BASED by default
/// (`AmbientLighting::Environment`: the diffuse irradiance and the GGX-prefiltered specular cube
/// of [`SceneLighting::environment`], or of the context's default procedural sky, with the
/// split-sum BRDF LUT, turned by [`SceneLighting::environment_rotation`] and scaled by
/// [`SceneLighting::environment_intensity`]), or `AmbientLighting::Hemisphere` (sky colour
/// [`SceneLighting::ambient_color`], ground colour [`SceneLighting::ambient_ground_color`] around
/// [`SceneLighting::ambient_up`]). The renderer keeps the frame bind groups of the last few
/// environments it lit with (and so holds those environments).
///
/// LOCAL LIGHTS (Not in C#): [`SceneLighting::local_lights`] - point lights and spotlights - are
/// shaded beside the directional lights, at most [`SceneLighting::MAX_LOCAL_LIGHTS`] per pair (the
/// ones that matter most to the camera, chosen at `begin`). A pair without local lights draws with
/// shader variants that have no local-light loop at all.
///
/// SHADOWS (Not in C#; off unless the lighting asks): the sun's ([`SceneLighting::sun_shadows`])
/// and those of up to [`SceneLighting::MAX_SHADOWED_SPOTLIGHTS`] spotlights
/// ([`crate::SpotLight::casts_shadows`]). NOTHING CHANGES IN HOW THE RENDERER IS CALLED: the
/// application calls `begin`, `draw`..., `end_opaque` / skybox / `end_blended` (or `end`) exactly
/// as without shadows. The first finishing call of the pair (`end_opaque`, `end_blended` or
/// `end`) draws the shadow passes from the queued instances - depth only, each culled against its
/// own light's volume, so a caster outside the camera's view still casts - on a command encoder of
/// the renderer's own, and SUBMITS it at once, before it records the pair's own draws into the
/// application's pass; the application's frame is submitted later, so its draws read finished
/// maps. This holds on the direct-to-canvas path and on the HDR path alike, and for any number of
/// pairs in a frame: each pair with shadows has its own set of maps (a frame with two such pairs
/// keeps two sets), so a later pair never overwrites what an earlier pair's draws read. Per
/// instance: [`ModelInstance::casts_shadows`], [`ModelInstance::receives_shadows`]; blended parts,
/// and instances whose alpha is below 1, never cast. A pair without shadows draws with shader
/// variants that have no shadow code at all.
///
/// LEVEL OF DETAIL (Not in C#): [`ModelRenderer::draw_lod`] draws a [`LodInstance`] - several
/// levels, most detailed first, each one instance or a set of instances, used out to a set
/// distance from the camera - as only the level(s) the camera's distance calls for, with a fade
/// band at every switch and at the final cut-off: a fine dither pattern per pixel (the same at 1
/// and 4 samples), the two levels of a cross-fade complementary so the pair never leaves a hole
/// or covers a pixel twice, depth written as usual and nothing sorted; blended parts fade by
/// alpha instead. The object is culled first (no distance measured, nothing drawn), the fading
/// levels' shadows fade with the same pattern, and every distance is scaled by
/// [`ModelRenderer::set_lod_distance_scale`] (quality). The object reports what it showed
/// ([`LodInstance::state`]), so the application animates only that.
///
/// THE DIFFERENCE FROM C# (Rust's borrows): `begin` takes the pass to read its target and frame,
/// `draw` copies the instance's pose at once, and `end` takes the same pass again to record the
/// draws.
pub struct ModelRenderer {
    context: GraphicsContext,
    frame_layout: GpuBindGroupLayout,
    material_layout: GpuBindGroupLayout,
    instance_layout: GpuBindGroupLayout,
    model_layout: GpuBindGroupLayout,
    default_sampler: GpuSampler,
    environment_sampler: GpuSampler,
    empty_storage: GpuBuffer,
    placeholder_cube: wgpu::TextureView,
    placeholder_lut: GpuTexture,
    shaders: HashMap<PbrFeatures, GpuShader>,
    /// The main pipelines by shader features (with the pipeline-only flags) and target (colour,
    /// depth, sample count): a multisampled target gets its own. The shadow passes' depth
    /// pipelines are [`ModelShadows`]'s, always single-sampled.
    pipelines: HashMap<(PbrFeatures, TargetFormats), GpuRenderPipeline>,
    models: HashMap<u64, PbrModelState>,
    frame_uniforms: GpuBuffer,
    frame_group: GpuBindGroup,
    /// The kept frame groups by environment and shadow-map set (0: the placeholders).
    frame_groups: Vec<(Option<EnvironmentMap>, u64, GpuBindGroup)>,
    shadows: ModelShadows,
    frame_capacity: u64,
    frame_slot: u64,
    instance_buffer: GpuBuffer,
    matrix_buffer: GpuBuffer,
    weight_buffer: GpuBuffer,
    instance_group: GpuBindGroup,
    instance_cursor: u64,
    matrix_cursor: u64,
    weight_cursor: u64,
    frame_id: Option<u64>,
    drawing: Option<Drawing>,
    buckets: Vec<Bucket>,
    last_draw_call_count: usize,
    last_instance_count: usize,
    last_culled_instance_count: usize,
    last_shadow_draw_call_count: usize,
    last_local_light_count: usize,
    last_shadowed_spotlight_count: usize,
    /// The sun's shadow map of the pair being finished (from its first finishing call until the
    /// next begin), when the pair drew the sun's shadow (Not in C#).
    sun_shadow: Option<SunShadowMap>,
    frame_stats: ModelRenderStats,
    frustum_culling: bool,
    two_pass_double_sided: bool,
    lod_distance_scale: f32,
}

impl ModelRenderer {
    /// A renderer (C#: `new ModelRenderer(context)`); its shaders and pipelines are made when
    /// first needed, one per shader feature set and target format.
    pub fn new(context: &GraphicsContext) -> Result<ModelRenderer, GraphicsException> {
        let fragment = wgpu::ShaderStages::FRAGMENT;
        let vertex = wgpu::ShaderStages::VERTEX;
        let mut frame_entries = vec![
            BindingLayoutEntry::uniform(0, vertex | fragment).with_dynamic_offset(true).with_min_binding_size(FRAME_UNIFORM_SIZE),
            BindingLayoutEntry::texture(1, fragment).with_view_dimension(wgpu::TextureViewDimension::Cube),
            BindingLayoutEntry::texture(2, fragment).with_view_dimension(wgpu::TextureViewDimension::Cube),
            BindingLayoutEntry::texture(3, fragment),
            BindingLayoutEntry::sampler(4, fragment),
        ];
        // The shadow maps and their comparison sampler (Not in C#): placeholders when a pair has none.
        frame_entries.extend(ModelShadows::frame_layout_entries());
        let frame_layout = context.create_bind_group_layout(Some("model frame"), &frame_entries);
        let mut material_entries = vec![BindingLayoutEntry::uniform(0, fragment).with_min_binding_size(MATERIAL_UNIFORM_SIZE)];
        material_entries.extend((1..=5).map(|binding| BindingLayoutEntry::texture(binding, fragment)));
        material_entries.extend((6..=10).map(|binding| BindingLayoutEntry::sampler(binding, fragment)));
        let material_layout = context.create_bind_group_layout(Some("model material"), &material_entries);
        let instance_layout = context.create_bind_group_layout(Some("model instances"), &[BindingLayoutEntry::storage(0, vertex, true), BindingLayoutEntry::storage(1, vertex, true), BindingLayoutEntry::storage(2, vertex, true)]);
        let model_layout = context.create_bind_group_layout(Some("model draws"), &[BindingLayoutEntry::uniform(0, vertex).with_dynamic_offset(true).with_min_binding_size(DRAW_UNIFORM_SIZE), BindingLayoutEntry::storage(1, vertex, true), BindingLayoutEntry::storage(2, vertex, true)]);

        let default_sampler = context.create_sampler(&SamplerDescription::linear_repeat());
        let environment_sampler = context.create_sampler(&SamplerDescription::linear_clamp());
        let empty_storage = context.create_storage_buffer(64, Some("model empty storage"))?;
        let cube = context.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("model environment placeholder"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 6 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let placeholder_cube = cube.create_view(&wgpu::TextureViewDescriptor { label: Some("model environment placeholder cube"), dimension: Some(wgpu::TextureViewDimension::Cube), ..wgpu::TextureViewDescriptor::default() });
        let placeholder_lut = context.create_texture_2d(1, 1, wgpu::TextureFormat::Rgba8Unorm, wgpu::TextureUsages::TEXTURE_BINDING, crate::context::Texture2DOptions { mip_level_count: 1, label: Some("model BRDF LUT placeholder".to_string()) })?;

        let frame_uniforms = context.create_buffer(4 * FRAME_SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("model frame"))?;
        let shadows = ModelShadows::new(context)?;
        let frame_group = create_frame_group(context, &frame_layout, &frame_uniforms, (&placeholder_cube, &placeholder_cube, placeholder_lut.view(), &environment_sampler), &shadows, None);
        let instance_buffer = context.create_storage_buffer(INSTANCE_SIZE * 256, Some("model instances"))?;
        let matrix_buffer = context.create_storage_buffer(MATRIX_SIZE * 1024, Some("model matrices"))?;
        let weight_buffer = context.create_storage_buffer(4 * 256, Some("model morph weights"))?;
        let instance_group = create_instance_group(context, &instance_layout, &instance_buffer, &matrix_buffer, &weight_buffer);
        Ok(ModelRenderer {
            context: context.clone(),
            frame_layout,
            material_layout,
            instance_layout,
            model_layout,
            default_sampler,
            environment_sampler,
            empty_storage,
            placeholder_cube,
            placeholder_lut,
            shaders: HashMap::new(),
            pipelines: HashMap::new(),
            models: HashMap::new(),
            frame_uniforms,
            frame_group: frame_group.clone(),
            frame_groups: vec![(None, 0, frame_group)],
            shadows,
            frame_capacity: 4,
            frame_slot: 0,
            instance_buffer,
            matrix_buffer,
            weight_buffer,
            instance_group,
            instance_cursor: 0,
            matrix_cursor: 0,
            weight_cursor: 0,
            frame_id: None,
            drawing: None,
            buckets: Vec::new(),
            last_draw_call_count: 0,
            last_instance_count: 0,
            last_culled_instance_count: 0,
            last_shadow_draw_call_count: 0,
            last_local_light_count: 0,
            last_shadowed_spotlight_count: 0,
            sun_shadow: None,
            frame_stats: ModelRenderStats::default(),
            frustum_culling: true,
            two_pass_double_sided: true,
            lod_distance_scale: 1.0,
        })
    }

    /// The context.
    pub fn context(&self) -> &GraphicsContext {
        &self.context
    }

    /// True between [`ModelRenderer::begin`] and [`ModelRenderer::end`] (C#: `IsDrawing`).
    pub fn is_drawing(&self) -> bool {
        self.drawing.is_some()
    }

    /// How many draw calls the last begin/end pair issued, both halves (C#: `LastDrawCallCount`).
    pub fn last_draw_call_count(&self) -> usize {
        self.last_draw_call_count
    }

    /// How many primitive instances the last begin/end pair drew, both halves (C#:
    /// `LastInstanceCount`). A blended double-sided part drawn in two passes counts once.
    pub fn last_instance_count(&self) -> usize {
        self.last_instance_count
    }

    /// How many instances the last begin/end pair culled: given to `draw` but entirely outside
    /// the view (Not in C#). Beside [`ModelRenderer::last_draw_call_count`].
    pub fn last_culled_instance_count(&self) -> usize {
        self.last_culled_instance_count
    }

    /// How many draw calls the last begin/end pair issued in its shadow passes (Not in C#).
    pub fn last_shadow_draw_call_count(&self) -> usize {
        self.last_shadow_draw_call_count
    }

    /// How many local lights (point lights and spotlights) the last `begin` chose to shade: at
    /// most [`SceneLighting::MAX_LOCAL_LIGHTS`], those that matter most to its camera (Not in C#).
    /// The frame's figure is [`ModelRenderStats::local_lights`].
    pub fn last_local_light_count(&self) -> usize {
        self.last_local_light_count
    }

    /// How many of those spotlights cast shadows in the last pair (each with a layer of the
    /// spotlights' shadow map): at most [`SceneLighting::MAX_SHADOWED_SPOTLIGHTS`] (Not in C#).
    pub fn last_shadowed_spotlight_count(&self) -> usize {
        self.last_shadowed_spotlight_count
    }

    /// The sun's shadow map of the pair being finished, for receivers of the application's or the
    /// crate's own - the water surface ([`crate::WaterFrame::sun_shadow`]) - (Not in C#): Some from
    /// the pair's FIRST FINISHING CALL (`end_opaque`, `end_blended` or `end`, which draws the
    /// shadow passes) until the next `begin`, when the pair has the sun's shadow
    /// ([`SceneLighting::sun_shadows`] on, a lit key light) and drew something; None otherwise.
    /// Read it between `end_opaque` and `end_blended` (or after the pair) and use it in the same
    /// frame: the next pair with the sun's shadow may draw into the same map.
    pub fn sun_shadow_map(&self) -> Option<SunShadowMap> {
        self.sun_shadow.clone()
    }

    /// The draw calls, instances drawn and instances culled of the current frame (the frame of the
    /// last `begin`), summed over all its begin/end pairs (Not in C#). Read it after the frame's
    /// last pair; the next frame's first `begin` starts again from zero.
    pub fn frame_stats(&self) -> ModelRenderStats {
        self.frame_stats
    }

    /// True (default) when `draw` culls instances outside the view (Not in C#).
    pub fn frustum_culling(&self) -> bool {
        self.frustum_culling
    }

    /// Turns culling on (default) or off for every instance (Not in C#); off, every visible
    /// instance is uploaded and drawn, as in C#. One instance is exempted with
    /// [`ModelInstance::never_cull`]. Takes effect at the next `draw`.
    pub fn set_frustum_culling(&mut self, enabled: bool) {
        self.frustum_culling = enabled;
    }

    /// True (default) when a part that is both blended and double-sided is drawn twice, back
    /// faces then front faces (Not in C#).
    pub fn two_pass_double_sided(&self) -> bool {
        self.two_pass_double_sided
    }

    /// Turns the two-pass drawing of blended double-sided parts on (default) or off (Not in C#);
    /// off, such a part is one draw of both faces in triangle order, as in C#. Read when the
    /// draws are worked out (the first finishing call of a pair).
    pub fn set_two_pass_double_sided(&mut self, enabled: bool) {
        self.two_pass_double_sided = enabled;
    }

    /// The camera's view the current pair culls against, between `begin` and the end of the pair;
    /// None outside (Not in C#). Test anything else against it the way `draw` does:
    /// `volume.intersects_transformed_box(min, max, world)`.
    pub fn view_volume(&self) -> Option<&CullVolume> {
        self.drawing.as_ref().map(|d| &d.volume)
    }

    /// The pipelines made so far (one per shader features, colour format, depth format and sample
    /// count - a multisampled target has its own; a
    /// blended double-sided feature set drawn in two passes uses two - back faces, front faces -
    /// in place of one). The shadow passes' depth pipelines are not counted here
    /// ([`ModelRenderer::shadow_pipeline_count`]).
    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }

    /// The depth pipelines of the shadow passes made so far: at most sixteen (skinned, morphing,
    /// alpha-tested, fading, and their combinations; Not in C#).
    pub fn shadow_pipeline_count(&self) -> usize {
        self.shadows.pipeline_count()
    }

    /// Starts drawing for `pass` with a camera and lighting, None for the [`SceneLighting`]
    /// defaults (C#: `Begin(renderPass, camera, lighting)`). Depth testing happens when the pass
    /// target has a depth view. The aspect ratio is the target's.
    ///
    /// Not in C#: here the renderer also chooses the local lights it shades for this pair (at most
    /// [`SceneLighting::MAX_LOCAL_LIGHTS`], those that matter most to this camera), the spotlights
    /// that cast shadows (at most [`SceneLighting::MAX_SHADOWED_SPOTLIGHTS`]) and, with the sun's
    /// shadow on, the sun's shadow view - so that `draw` keeps every instance that casts into one
    /// of the pair's shadow maps, even one outside the camera's view.
    pub fn begin(&mut self, pass: &RenderPass<'_>, camera: &Camera, lighting: Option<&SceneLighting>) -> Result<(), GraphicsException> {
        if self.drawing.is_some() {
            return Err(GraphicsException::new("ModelRenderer::begin was called twice without end; call end(pass) first."));
        }
        let frame_id = pass.frame_id();
        if self.frame_id != Some(frame_id) {
            self.frame_id = Some(frame_id);
            self.frame_slot = 0;
            self.instance_cursor = 0;
            self.matrix_cursor = 0;
            self.weight_cursor = 0;
            self.frame_stats = ModelRenderStats::default();
        }
        self.shadows.start_frame(frame_id);
        let defaults = SceneLighting::default();
        let lighting = lighting.unwrap_or(&defaults);
        let aspect_ratio = pass.target().aspect_ratio();
        let mut frame = PbrFrameUniforms::new(camera, lighting, aspect_ratio);
        // C#'s SelectEnvironment: the lighting's environment, the context's default sky, or the
        // hemisphere.
        let environment = match (lighting.ambient, &lighting.environment) {
            (AmbientLighting::Environment, Some(environment)) => Some(environment.clone()),
            (AmbientLighting::Environment, None) => Some(self.context.default_environment()?),
            (AmbientLighting::Hemisphere, _) => None,
        };
        if let Some(environment) = &environment {
            frame.environment = Vec4::new(1.0, lighting.environment_intensity, environment.prefiltered_mip_levels().saturating_sub(1) as f32, lighting.environment_rotation);
        }
        let volume = CullVolume::from_view_projection(frame.view_projection);
        // Local lights and shadows (Not in C#).
        let chosen = select_local_lights(&lighting.local_lights, camera.position, &volume, SceneLighting::MAX_LOCAL_LIGHTS);
        let mut shadow_passes: Vec<(ShadowView, ShadowTarget)> = Vec::new();
        let sun_settings = &lighting.sun_shadows;
        let sun = if sun_settings.enabled && PbrFrameUniforms::key_light_shades(lighting) { sun_shadow_view(lighting.light_direction, sun_settings.region_min, sun_settings.region_max, sun_settings.map_size.texels()) } else { None };
        let mut spot_layers: Vec<Option<usize>> = vec![None; chosen.len()];
        let mut spot_views: Vec<ShadowView> = Vec::new();
        let spot_texels = lighting.spot_shadows.map_size.texels();
        for position in select_shadowed_spots(&lighting.local_lights, &chosen, SceneLighting::MAX_SHADOWED_SPOTLIGHTS) {
            if let Some(view) = lighting.local_lights[chosen[position]].as_spot().and_then(|spot| spot_shadow_view(spot, camera.far_plane, spot_texels)) {
                spot_layers[position] = Some(spot_views.len());
                spot_views.push(view);
            }
        }
        frame.set_local_lights(&lighting.local_lights, &chosen, &spot_layers, &spot_views);
        frame.set_shadows(lighting, sun.as_ref(), &spot_views);
        let sun_texels = sun.as_ref().map(|_| sun_settings.map_size.texels());
        if let Some(view) = sun {
            shadow_passes.push((view, ShadowTarget::Sun));
        }
        let spot_count = spot_views.len();
        shadow_passes.extend(spot_views.into_iter().enumerate().map(|(layer, view)| (view, ShadowTarget::Spot(layer as u32))));
        let shadow_set = if shadow_passes.is_empty() { None } else { Some(self.shadows.take_set(sun_texels, (spot_count > 0).then_some(spot_texels))) };
        let mut features = PbrFeatures::NONE;
        if !chosen.is_empty() {
            features |= PbrFeatures::LOCAL_LIGHTS;
        }
        if !shadow_passes.is_empty() {
            features |= PbrFeatures::SHADOWS;
        }
        self.frame_group = self.frame_group_for(environment.as_ref(), shadow_set)?;
        self.buckets.clear();
        self.drawing = Some(Drawing { frame_id, frame, environment, camera_position: camera.position, camera_forward: camera.forward(), volume, features, shadow_passes, shadow_set, prepared: None });
        self.last_draw_call_count = 0;
        self.last_instance_count = 0;
        self.last_culled_instance_count = 0;
        self.last_shadow_draw_call_count = 0;
        self.last_local_light_count = chosen.len();
        self.last_shadowed_spotlight_count = spot_count;
        count_lights(&mut self.frame_stats, chosen.len(), spot_count);
        self.sun_shadow = None;
        Ok(())
    }

    /// Queues an instance, skipped when [`ModelInstance::visible`] is false or its
    /// [`ModelInstance::alpha`] is 0 or less (C#: `Draw(instance)`), and CULLED (Not in C#) when
    /// culling is on, the instance is not [`ModelInstance::never_cull`] and its model's
    /// [`Model::culling_bounds`], placed by its world matrix, are entirely outside the camera's
    /// view: a culled instance is counted and, unless it casts into one of the pair's shadow maps
    /// (each shadow pass culls against its own light's volume; Not in C#), nothing of it is
    /// copied, uploaded or drawn (its pose is not even computed). Otherwise its world matrix,
    /// pose, look and hidden nodes are copied now. An error outside begin/end, and after
    /// [`ModelRenderer::end_opaque`] (queue everything first).
    pub fn draw(&mut self, instance: &ModelInstance) -> Result<(), GraphicsException> {
        self.check_draw("draw")?;
        self.queue(instance, instance.world, 0)
    }

    /// The error of a `draw` / `draw_lod` / `select_lod` outside begin/end or after `end_opaque`.
    fn check_draw(&self, call: &str) -> Result<&Drawing, GraphicsException> {
        match &self.drawing {
            None => Err(GraphicsException::new(format!("ModelRenderer::{call} was called outside begin/end; call begin(pass, camera, lighting) first."))),
            Some(drawing) if drawing.prepared.is_some() => Err(GraphicsException::new(format!("ModelRenderer::{call} was called after end_opaque; queue every instance before end_opaque, or begin a new pair after end_blended."))),
            Some(drawing) => Ok(drawing),
        }
    }

    /// The multiplier of every level-of-detail distance (Not in C#; default 1): above 1 keeps the
    /// detailed levels out to further distances (higher quality), below 1 switches to the simpler
    /// ones sooner (faster). It scales the switching distances, the fade bands with them, and the
    /// final cut-offs.
    pub fn lod_distance_scale(&self) -> f32 {
        self.lod_distance_scale
    }

    /// Sets [`ModelRenderer::lod_distance_scale`]; a value that is not a positive number is taken
    /// as 1. Takes effect at the next `draw_lod` / `select_lod`.
    pub fn set_lod_distance_scale(&mut self, scale: f32) {
        self.lod_distance_scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    }

    /// Chooses a level-of-detail object's levels for this pair's camera WITHOUT drawing it (Not in
    /// C#), and returns what it will show ([`LodInstance::state`]): culled first (outside the
    /// camera's view and every shadow map's volume: [`LodState::Culled`], no distance measured),
    /// then by distance. Use it between `begin` and `draw_lod` to advance the animation of exactly
    /// the levels about to be drawn ([`LodInstance::shown_levels_mut`]); `draw_lod` chooses again
    /// and finds the same. An error outside begin/end and after `end_opaque`.
    pub fn select_lod(&self, lod: &mut LodInstance) -> Result<LodState, GraphicsException> {
        let drawing = self.check_draw("select_lod")?;
        let shadow_volumes: Vec<&CullVolume> = drawing.shadow_passes.iter().map(|(view, _)| &view.volume).collect();
        if lod_culled(&drawing.volume, &shadow_volumes, self.frustum_culling, lod.never_cull, lod.bounds(), lod.world) {
            lod.mark_culled();
            return Ok(LodState::Culled);
        }
        Ok(lod.select(drawing.camera_position, self.lod_distance_scale))
    }

    /// Queues a level-of-detail object (Not in C#): chooses its levels as
    /// [`ModelRenderer::select_lod`] does, then queues the instances of the shown level(s), each
    /// at `lod.world * instance.world` - in a fade band both levels, the one going out and the one
    /// coming in with complementary dither patterns (blended parts by alpha); at the final cut-off
    /// the last level alone, fading out. Every queued instance then goes through `draw`'s own
    /// rules (its visibility, alpha, culling of each instance, shadows: a fading level casts
    /// through the same pattern). Returns the state, also kept as [`LodInstance::state`]. Counted
    /// in [`ModelRenderStats::culled_lods`] and [`ModelRenderStats::fading_lods`]. An error
    /// outside begin/end and after `end_opaque`.
    pub fn draw_lod(&mut self, lod: &mut LodInstance) -> Result<LodState, GraphicsException> {
        let state = self.select_lod(lod)?;
        match state {
            LodState::Culled => self.frame_stats.culled_lods += 1,
            LodState::Fading { .. } => self.frame_stats.fading_lods += 1,
            _ => {}
        }
        let sizes: Vec<usize> = lod.levels().iter().map(|level| level.instances.len()).collect();
        for (level, index, code) in lod_draws(&state, &sizes) {
            let instance = &lod.levels()[level].instances[index];
            self.queue(instance, lod.world * instance.world, code)?;
        }
        Ok(state)
    }

    /// Queues an instance at `world` with a level-of-detail dither code (0: none): `draw`'s rules.
    fn queue(&mut self, instance: &ModelInstance, world: Mat4, dither: u32) -> Result<(), GraphicsException> {
        let drawing = self.drawing.as_ref().expect("checked by the caller");
        if !instance.visible || instance.alpha <= 0.0 {
            return Ok(());
        }
        let model = instance.model();
        let shadow_volumes: Vec<&CullVolume> = drawing.shadow_passes.iter().map(|(view, _)| &view.volume).collect();
        let casts = instance.casts_shadows && instance.alpha >= 1.0;
        let (in_view, shadow_mask) = cull_passes(&drawing.volume, &shadow_volumes, self.frustum_culling, instance.never_cull, casts, model.culling_bounds(), world);
        if !in_view {
            self.last_culled_instance_count += 1;
            self.frame_stats.culled_instances += 1;
            if shadow_mask == 0 {
                return Ok(());
            }
        }
        if !self.models.contains_key(&model.id()) {
            let state = self.create_state(model)?;
            self.models.insert(model.id(), state);
        }
        let bucket = match self.buckets.iter().position(|b| b.model_id == model.id()) {
            Some(index) => &mut self.buckets[index],
            None => {
                self.buckets.push(Bucket::new(model.id()));
                self.buckets.last_mut().expect("just pushed")
            }
        };
        bucket.worlds.push(world);
        bucket.node_matrices.extend_from_slice(&instance.node_matrices());
        bucket.palettes.extend_from_slice(&instance.joint_palettes());
        let baked = &model.layout().baked_morph_weights;
        bucket.weights.extend(instance.morph_weight_array().iter().zip(baked).map(|(weight, baked)| weight - baked));
        bucket.looks.push(InstanceLook { tint: instance.tint.extend(instance.alpha.min(1.0)), emissive: instance.emissive_strength, receives_shadows: instance.receives_shadows, dither });
        bucket.hidden.push(instance.hidden_node_mask());
        bucket.in_view.push(in_view);
        bucket.shadow_mask.push(shadow_mask);
        Ok(())
    }

    /// Uploads the queued instances and records ALL their draws into `pass` - the pass `begin`
    /// was given (C#: `End()`): [`ModelRenderer::end_opaque`] then [`ModelRenderer::end_blended`]
    /// (only the blended half when `end_opaque` was already called).
    pub fn end(&mut self, pass: &mut RenderPass<'_>) -> Result<(), GraphicsException> {
        self.check_end(pass, "end")?;
        if !self.drawing.as_ref().and_then(|d| d.prepared.as_ref()).is_some_and(|p| p.opaque_drawn) {
            if let Err(error) = self.end_opaque(pass) {
                // The pair ends either way, as C#'s End does.
                self.drawing = None;
                self.buckets.clear();
                return Err(error);
            }
        }
        self.end_blended(pass)
    }

    /// The first half of [`ModelRenderer::end`] (Not in C#): uploads the queued instances and
    /// records the draws of their OPAQUE and alpha-tested parts into `pass` (the pass `begin` was
    /// given). The pair stays open: no more `draw` calls, then [`ModelRenderer::end_blended`]
    /// with the same pass. Draw the skybox between the two. An error when called twice.
    pub fn end_opaque(&mut self, pass: &mut RenderPass<'_>) -> Result<(), GraphicsException> {
        self.check_end(pass, "end_opaque")?;
        if self.drawing.as_ref().and_then(|d| d.prepared.as_ref()).is_some_and(|p| p.opaque_drawn) {
            return Err(GraphicsException::new("ModelRenderer::end_opaque was called twice in one begin/end pair; call end_blended to finish the pair."));
        }
        self.prepare()?;
        let mut drawing = self.drawing.take().expect("checked");
        let prepared = drawing.prepared.as_mut().expect("just prepared");
        let result = self.record_all(pass, prepared, false, drawing.features);
        prepared.opaque_drawn = true;
        self.drawing = Some(drawing);
        result
    }

    /// The second half of [`ModelRenderer::end`] (Not in C#): records the draws of the BLENDED
    /// parts of everything queued (and every part of an instance whose alpha is below 1), back to
    /// front, into `pass`, and ends the begin/end pair. Without an [`ModelRenderer::end_opaque`]
    /// before it, the opaque parts are NOT drawn (the pair draws only its blended parts).
    ///
    /// A LATER PASS OF THE SAME FRAME ON THE SAME TARGET may be given here instead of the pass
    /// `begin` and `end_opaque` were given: end the first pass after `end_opaque` (and the skybox),
    /// do work outside any pass (the water's scene capture,
    /// [`crate::WaterRenderer::capture_scene`]), then begin a pass on the same target that LOADS
    /// colour and depth, draw into it (the water), and finish the pair there. Only the frame is checked; the target must be the same
    /// (same formats and size: the pair's pipelines and aspect ratio are the first pass's).
    pub fn end_blended(&mut self, pass: &mut RenderPass<'_>) -> Result<(), GraphicsException> {
        self.check_end(pass, "end_blended")?;
        self.prepare()?;
        let mut drawing = self.drawing.take().expect("checked");
        let prepared = drawing.prepared.as_mut().expect("just prepared");
        let result = self.record_all(pass, prepared, true, drawing.features);
        self.buckets.clear();
        result
    }

    /// The error of a finishing call outside begin/end, or with another frame's pass.
    fn check_end(&self, pass: &RenderPass<'_>, call: &str) -> Result<(), GraphicsException> {
        let Some(drawing) = &self.drawing else {
            return Err(GraphicsException::new(format!("ModelRenderer::{call} was called without begin; call begin(pass, camera, lighting) first.")));
        };
        if !ends_in_pass_of(drawing.frame_id, pass.frame_id()) {
            return Err(GraphicsException::new(format!("ModelRenderer::{call} was given a pass of another frame than begin; end the renderer with a pass of the frame it began in (the same pass, or a later one on the same target).")));
        }
        Ok(())
    }

    /// Works out the draws of the queued instances and uploads them, once per begin/end pair
    /// (C#'s DrawModel and UploadInstances).
    fn prepare(&mut self) -> Result<(), GraphicsException> {
        let drawing = self.drawing.as_ref().expect("checked");
        if drawing.prepared.is_some() {
            return Ok(());
        }
        let buckets = std::mem::take(&mut self.buckets);
        let models: Vec<PlanModel<'_>> = buckets
            .iter()
            .map(|bucket| {
                let state = &self.models[&bucket.model_id];
                let layout = state.model.layout();
                PlanModel { draws: &state.draws, node_count: state.model.nodes().len(), palette_size: layout.palette_size, weight_count: layout.morph_weight_count, morph_shifts: &state.morph_shifts }
            })
            .collect();
        let mut planned = plan(&buckets, &models, drawing.camera_position, drawing.camera_forward, self.two_pass_double_sided, drawing.shadow_passes.len());
        drop(models);
        let model_ids: Vec<u64> = buckets.iter().map(|b| b.model_id).collect();
        self.buckets = buckets;
        self.buckets.clear();
        if planned.opaque.is_empty() && planned.blended.is_empty() {
            // Nothing to draw (C#'s End returns early): no frame slot, no upload.
            self.drawing.as_mut().expect("checked").prepared = Some(Prepared { plan: planned, model_ids, frame_offset: 0, base: 0, opaque_drawn: false });
            return Ok(());
        }

        if self.frame_slot >= self.frame_capacity {
            self.frame_capacity *= 2;
            self.frame_uniforms = self.context.create_buffer(self.frame_capacity * FRAME_SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("model frame"))?;
            self.frame_groups.clear();
            let (environment, shadow_set) = self.drawing.as_ref().map(|d| (d.environment.clone(), d.shadow_set)).expect("checked");
            self.frame_group = self.frame_group_for(environment.as_ref(), shadow_set)?;
            self.frame_slot = 0;
        }
        // Each frame's uniforms in a slot of their own (they are larger than one alignment step).
        let frame_offset = (self.frame_slot * FRAME_SLOT_SIZE) as u32;
        let frame = self.drawing.as_ref().expect("checked").frame;
        self.frame_uniforms.write_value(&frame, frame_offset as u64)?;
        let base = if planned.instances.is_empty() {
            self.instance_cursor as u32
        } else {
            self.upload(&mut planned.instances, &planned.matrices, &planned.weights)?;
            self.instance_cursor as u32
        };
        self.frame_slot += 1;
        self.instance_cursor += planned.instances.len() as u64;
        self.matrix_cursor += planned.matrices.len() as u64;
        self.weight_cursor += planned.weights.len() as u64;
        let prepared = Prepared { plan: planned, model_ids, frame_offset, base, opaque_drawn: false };
        self.draw_shadow_passes(&prepared)?;
        // The sun's map, now drawn, for receivers of the application's or the crate's own.
        let drawing = self.drawing.as_ref().expect("checked");
        if let (Some(set), Some(numbers)) = (drawing.shadow_set, sun_shadow_numbers(&drawing.frame)) {
            if drawing.shadow_passes.iter().any(|(_, target)| *target == ShadowTarget::Sun) {
                let view = self.shadows.sampled_views(Some(set)).0.clone();
                self.sun_shadow = Some(numbers.with_view(view));
            }
        }
        self.drawing.as_mut().expect("checked").prepared = Some(prepared);
        Ok(())
    }

    /// Draws the pair's shadow passes (Not in C#), each into its map, cleared first (also when no
    /// caster is in it), on a command encoder of the renderer's own that is SUBMITTED AT ONCE: the
    /// application's frame (and with it the pass that samples the maps) is submitted later, so
    /// the maps are complete before any draw reads them, on the canvas path and on the HDR path
    /// alike. The instance records it draws are those just uploaded for the pair.
    fn draw_shadow_passes(&mut self, prepared: &Prepared) -> Result<(), GraphicsException> {
        let drawing = self.drawing.as_ref().expect("checked");
        let Some(set) = drawing.shadow_set else {
            return Ok(());
        };
        let matrices: Vec<Mat4> = drawing.shadow_passes.iter().map(|(view, _)| view.view_projection).collect();
        let targets: Vec<ShadowTarget> = drawing.shadow_passes.iter().map(|(_, target)| *target).collect();
        let offsets = self.shadows.write_matrices(&matrices)?;
        let layouts = [&self.material_layout, &self.instance_layout, &self.model_layout];
        // The pipelines first (made on first use), then the recording.
        for draws in &prepared.plan.shadows {
            for planned in draws {
                let features = self.models[&prepared.model_ids[planned.bucket]].draws[planned.draw].features | dither_feature(planned);
                self.shadows.pipeline(features, layouts)?;
            }
        }
        let mut encoder = self.context.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("model shadow passes") });
        let mut draw_calls = 0usize;
        for ((target, offset), draws) in targets.iter().zip(&offsets).zip(&prepared.plan.shadows) {
            let Some(view) = self.shadows.target_view(set, *target) else {
                continue;
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("model shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }), stencil_ops: None }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            // What is bound: the depth features (pipeline), the model, the material.
            let mut bound = ShadowRecorder::default();
            for planned in draws {
                let model_id = prepared.model_ids[planned.bucket];
                let state = &self.models[&model_id];
                let draw = state.draws[planned.draw];
                let caster = draw.features | dither_feature(planned);
                let depth_features = crate::shaders::ShadowDepthShader::features_of(caster);
                if bound.features != Some(depth_features) {
                    let pipeline = self.shadows.pipeline(caster, layouts)?;
                    pass.set_pipeline(pipeline.handle());
                    pass.set_bind_group(0, self.shadows.group().handle(), &[*offset]);
                    pass.set_bind_group(2, self.instance_group.handle(), &[]);
                    bound.features = Some(depth_features);
                    bound.material = None;
                }
                if bound.model != Some(model_id) {
                    pass.set_vertex_buffer(0, state.model.vertex_buffer().handle().slice(..));
                    pass.set_index_buffer(state.model.index_buffer().handle().slice(..), state.model.index_format());
                    bound.model = Some(model_id);
                    bound.material = None;
                }
                if bound.material != Some((model_id, draw.material)) {
                    pass.set_bind_group(1, state.material(&draw).group.handle(), &[]);
                    bound.material = Some((model_id, draw.material));
                }
                pass.set_bind_group(3, state.group.handle(), &[(planned.draw as u64 * SLOT_SIZE) as u32]);
                let first_instance = prepared.base + planned.first_instance;
                pass.draw_indexed(draw.first_index..draw.first_index + draw.index_count, draw.base_vertex as i32, first_instance..first_instance + planned.count);
                draw_calls += 1;
            }
        }
        self.context.queue().submit([encoder.finish()]);
        self.last_shadow_draw_call_count += draw_calls;
        self.frame_stats.shadow_draw_calls += draw_calls;
        self.frame_stats.shadow_passes += targets.len();
        Ok(())
    }

    /// Records the opaque or the blended draws of a prepared pair.
    fn record_all(&mut self, pass: &mut RenderPass<'_>, prepared: &Prepared, blended: bool, frame_features: PbrFeatures) -> Result<(), GraphicsException> {
        let formats = pass.target().formats();
        let encode = if needs_shader_encoding(formats.color) { PbrFeatures::ENCODE_SRGB } else { PbrFeatures::NONE };
        let mut recorder = Recorder { pipeline: None, model: None, material: None };
        let draws: &[PlannedDraw] = if blended { &prepared.plan.blended } else { &prepared.plan.opaque };
        for planned in draws {
            let model_id = prepared.model_ids[planned.bucket];
            self.record(pass, &mut recorder, prepared.frame_offset, model_id, planned, prepared.base, (encode, frame_features), formats)?;
        }
        Ok(())
    }

    /// The frame bind group for an environment, or for the hemisphere (None), and a shadow-map
    /// set (None: the placeholders); kept for the last few (C#: `FrameGroup`, cached per
    /// environment; the shadow maps are Not in C#).
    fn frame_group_for(&mut self, environment: Option<&EnvironmentMap>, shadow_set: Option<usize>) -> Result<GpuBindGroup, GraphicsException> {
        let shadow_id = self.shadows.set_id(shadow_set);
        let key = (environment.map(EnvironmentMap::id), shadow_id);
        if let Some((_, _, group)) = self.frame_groups.iter().find(|(kept, id, _)| (kept.as_ref().map(EnvironmentMap::id), *id) == key) {
            return Ok(group.clone());
        }
        let group = match environment {
            Some(environment) => {
                let lut = self.context.brdf_lut_view()?;
                create_frame_group(&self.context, &self.frame_layout, &self.frame_uniforms, (environment.irradiance(), environment.prefiltered(), &lut, &self.environment_sampler), &self.shadows, shadow_set)
            }
            None => create_frame_group(&self.context, &self.frame_layout, &self.frame_uniforms, (&self.placeholder_cube, &self.placeholder_cube, self.placeholder_lut.view(), &self.environment_sampler), &self.shadows, shadow_set),
        };
        // The hemispheric group without shadows stays; the oldest of the others goes.
        if self.frame_groups.iter().filter(|(kept, id, _)| kept.is_some() || *id != 0).count() >= KEPT_FRAME_GROUPS {
            if let Some(oldest) = self.frame_groups.iter().position(|(kept, id, _)| kept.is_some() || *id != 0) {
                self.frame_groups.remove(oldest);
            }
        }
        self.frame_groups.push((environment.cloned(), shadow_id, group.clone()));
        Ok(group)
    }

    /// Drops the GPU state the renderer keeps for a model (its materials and draw uniforms) - and
    /// with it the renderer's hold on the model (C#: `Forget(model)`).
    pub fn forget(&mut self, model: &Model) {
        self.models.remove(&model.id());
    }

    /// Writes this end's instance records, matrices and weights after what earlier begin/end
    /// pairs of the frame wrote, growing the buffers when needed (C#: `UploadInstances`).
    fn upload(&mut self, instances: &mut [PbrInstance], matrices: &[Mat4], weights: &[f32]) -> Result<(), GraphicsException> {
        let mut regroup = false;
        let grow = |context: &GraphicsContext, current: &GpuBuffer, needed: u64, label: &str| context.create_storage_buffer((current.size() * 2).max(needed * 2), Some(label));
        if (self.instance_cursor + instances.len() as u64) * INSTANCE_SIZE > self.instance_buffer.size() {
            self.instance_buffer = grow(&self.context, &self.instance_buffer, instances.len() as u64 * INSTANCE_SIZE, "model instances")?;
            self.instance_cursor = 0;
            regroup = true;
        }
        if (self.matrix_cursor + matrices.len() as u64) * MATRIX_SIZE > self.matrix_buffer.size() {
            self.matrix_buffer = grow(&self.context, &self.matrix_buffer, matrices.len() as u64 * MATRIX_SIZE, "model matrices")?;
            self.matrix_cursor = 0;
            regroup = true;
        }
        if (self.weight_cursor + weights.len() as u64) * 4 > self.weight_buffer.size() {
            self.weight_buffer = grow(&self.context, &self.weight_buffer, weights.len() as u64 * 4, "model morph weights")?;
            self.weight_cursor = 0;
            regroup = true;
        }
        if regroup {
            self.instance_group = create_instance_group(&self.context, &self.instance_layout, &self.instance_buffer, &self.matrix_buffer, &self.weight_buffer);
        }
        for record in instances.iter_mut() {
            record.node_base += self.matrix_cursor as u32;
            record.joint_base += self.matrix_cursor as u32;
            record.weight_base += self.weight_cursor as u32;
        }
        self.instance_buffer.write(instances, self.instance_cursor * INSTANCE_SIZE)?;
        if !matrices.is_empty() {
            self.matrix_buffer.write(matrices, self.matrix_cursor * MATRIX_SIZE)?;
        }
        if !weights.is_empty() {
            self.weight_buffer.write(weights, self.weight_cursor * 4)?;
        }
        Ok(())
    }

    /// Records one draw (C#: `Record`), binding only what changed.
    #[allow(clippy::too_many_arguments)]
    fn record(&mut self, pass: &mut RenderPass<'_>, recorder: &mut Recorder, frame_offset: u32, model_id: u64, planned: &PlannedDraw, base: u32, (encode, frame_features): (PbrFeatures, PbrFeatures), formats: TargetFormats) -> Result<(), GraphicsException> {
        let draw_index = planned.draw;
        let (first_instance, count) = (base + planned.first_instance, planned.count);
        let draw = self.models[&model_id].draws[draw_index];
        let mut features = draw.features | encode;
        // The frame's local lights and shadows (an unlit part needs neither).
        if !draw.features.contains(PbrFeatures::UNLIT) {
            features |= frame_features;
        }
        if planned.mirrored {
            features |= PbrFeatures::MIRRORED;
        }
        if planned.faded {
            features |= PbrFeatures::FADED;
        }
        features |= dither_feature(planned);
        match planned.side {
            FaceSide::Both => {}
            FaceSide::Back => features |= PbrFeatures::BACK_FACES,
            FaceSide::Front => features |= PbrFeatures::FRONT_FACES,
        }
        let pipeline = self.pipeline(features, formats)?;
        let state = &self.models[&model_id];
        if recorder.pipeline.as_ref().map(|p| p.handle()) != Some(pipeline.handle()) {
            pass.set_pipeline(&pipeline);
            pass.set_bind_group_with_offsets(0, &self.frame_group, &[frame_offset]);
            pass.set_bind_group(2, &self.instance_group);
            recorder.pipeline = Some(pipeline);
            recorder.material = None;
        }
        if recorder.model != Some(model_id) {
            pass.set_vertex_buffer(0, state.model.vertex_buffer(), BufferRange::default());
            pass.set_index_buffer(state.model.index_buffer(), state.model.index_format(), BufferRange::default());
            recorder.model = Some(model_id);
            recorder.material = None;
        }
        let material_key = (model_id, draw.material);
        if recorder.material != Some(material_key) {
            pass.set_bind_group(1, &state.material(&draw).group);
            recorder.material = Some(material_key);
        }
        pass.set_bind_group_with_offsets(3, &state.group, &[(draw_index as u64 * SLOT_SIZE) as u32]);
        pass.draw_indexed(draw.index_count, DrawIndexedOptions { instance_count: count, first_index: draw.first_index, base_vertex: draw.base_vertex as i32, first_instance });
        self.last_draw_call_count += 1;
        self.frame_stats.draw_calls += 1;
        // A two-pass part's instances count once (with its second draw).
        if planned.side != FaceSide::Back {
            self.last_instance_count += count as usize;
            self.frame_stats.instances += count as usize;
        }
        Ok(())
    }

    /// The pipeline for a feature set and target - colour, depth and sample count (made on first
    /// use; C#: `Pipeline`).
    fn pipeline(&mut self, features: PbrFeatures, formats: TargetFormats) -> Result<GpuRenderPipeline, GraphicsException> {
        if let Some(pipeline) = self.pipelines.get(&(features, formats)) {
            return Ok(pipeline.clone());
        }
        let (color, depth) = (formats.color, formats.depth);
        let shader_features = features.without(PbrFeatures::PIPELINE_ONLY);
        let context = self.context.clone();
        let shader = self.shaders.entry(shader_features).or_insert_with(|| context.create_shader(&PbrShader::source(shader_features), Some(&format!("model pbr {shader_features}"))));
        let blend = features.contains(PbrFeatures::ALPHA_BLEND) || features.contains(PbrFeatures::FADED);
        let (cull, front_face) = pipeline_culling(features);
        let mut builder = self
            .context
            .create_render_pipeline(shader)
            .with_label(&format!("model pbr {features} {color:?}"))
            .with_vertex_buffer_of::<ModelVertex>(VertexLayoutOptions::default())
            .with_color_target(color, ColorTargetOptions::blended(if blend { BlendPreset::AlphaBlend } else { BlendPreset::Opaque }))
            .with_culling(cull, front_face)
            .with_bind_group_layouts(&[&self.frame_layout, &self.material_layout, &self.instance_layout, &self.model_layout])
            .with_sample_count(formats.sample_count);
        if let Some(depth) = depth {
            builder = builder.with_depth(depth, DepthOptions { preset: if blend { DepthPreset::ReadOnly } else { DepthPreset::ReadWrite }, ..DepthOptions::default() });
        }
        let pipeline = builder.build()?;
        self.pipelines.insert((features, formats), pipeline.clone());
        Ok(pipeline)
    }

    /// Builds a model's materials, draw items and bind group (C#: `PbrModelState`).
    fn create_state(&self, model: &Model) -> Result<PbrModelState, GraphicsException> {
        let label = model.label().to_string();
        let mut materials = Vec::with_capacity(model.materials().len());
        for (index, material) in model.materials().iter().enumerate() {
            materials.push(self.create_material(model, material, &format!("{label} material {index}"))?);
        }
        let fallback = self.create_material(model, model.default_material(), &format!("{label} default material"))?;
        let layout = model.layout();
        let mut deltas: Vec<Vec4> = Vec::new();
        let mut packed: HashMap<(usize, usize), (u32, u32, u32)> = HashMap::new();
        let mut slots = vec![0u8; layout.draw_items.len().max(1) * SLOT_SIZE as usize];
        let mut draws = Vec::with_capacity(layout.draw_items.len());
        let mut morph_shifts: Vec<Vec3> = Vec::new();
        for (index, item) in layout.draw_items.iter().enumerate() {
            let primitive = &model.meshes()[item.mesh].primitives[item.primitive];
            let node = &model.nodes()[item.node];
            let skin = node.skin.filter(|&s| s < model.skins().len());
            let skinned = primitive.is_skinned() && skin.is_some();
            let material = primitive.material.filter(|&m| m < materials.len());
            let mut features = material.map_or(fallback.features, |m| materials[m].features);
            let mut uniforms = PbrDrawUniforms { node: item.node as u32, base_vertex: primitive.base_vertex, ..PbrDrawUniforms::default() };
            let mut skin_palette = (0, 0);
            let mut morph = (0, 0, 0);
            if let (true, Some(skin)) = (skinned, skin) {
                skin_palette = (layout.skin_palette_offsets[skin], model.skins()[skin].joint_count());
                features |= PbrFeatures::SKINNED;
                uniforms.skin_base = primitive.skin_base_vertex.unwrap_or(0);
                uniforms.joint_offset = layout.skin_palette_offsets[skin] as u32;
            }
            if let (false, Some(offset)) = (primitive.morph_targets.is_empty(), layout.morph_weight_offsets[item.node]) {
                let (start, channels, stride) = *packed.entry((item.mesh, item.primitive)).or_insert_with(|| pack_morph(primitive, &mut deltas));
                if stride > 0 {
                    features |= PbrFeatures::MORPHED;
                    uniforms.morph_targets = primitive.morph_targets.len().min(node.default_morph_weights.len()) as u32;
                    uniforms.morph_weight_offset = offset as u32;
                    uniforms.morph_delta_base = start;
                    uniforms.morph_vertex_count = primitive.vertex_count;
                    uniforms.morph_channels = channels;
                    uniforms.morph_stride = stride;
                    let shifts = layout.morph_center_shifts.get(item.mesh).and_then(|m| m.get(item.primitive)).map_or(&[][..], |t| t.as_slice());
                    let targets = (uniforms.morph_targets as usize).min(shifts.len());
                    if shifts[..targets].iter().any(|s| *s != Vec3::ZERO) {
                        morph = (morph_shifts.len(), offset, targets);
                        morph_shifts.extend_from_slice(&shifts[..targets]);
                    }
                }
            }
            let at = index * SLOT_SIZE as usize;
            slots[at..at + DRAW_UNIFORM_SIZE as usize].copy_from_slice(bytemuck::bytes_of(&uniforms));
            draws.push(PbrDraw { material, features, node: item.node, skinned, skin_palette, blend: features.contains(PbrFeatures::ALPHA_BLEND), center: (primitive.bounds_min + primitive.bounds_max) * 0.5, morph, first_index: primitive.first_index, index_count: primitive.index_count, base_vertex: primitive.base_vertex });
        }
        let draw_uniforms = self.context.create_buffer_from(&slots, wgpu::BufferUsages::UNIFORM, Some(&format!("{label} draws")))?;
        let morph_deltas = if deltas.is_empty() { None } else { Some(self.context.create_storage_buffer_from(&deltas, Some(&format!("{label} morph deltas")))?) };
        let skin_stream = model.skin_buffer().unwrap_or(&self.empty_storage);
        let group = self.context.create_bind_group(
            &self.model_layout,
            Some(&format!("{label} draws")),
            &[BindingEntry::for_buffer(0, &draw_uniforms).with_range(0, Some(DRAW_UNIFORM_SIZE)), BindingEntry::for_buffer(1, skin_stream), BindingEntry::for_buffer(2, morph_deltas.as_ref().unwrap_or(&self.empty_storage))],
        );
        Ok(PbrModelState { model: model.clone(), materials, fallback, draws, morph_shifts, group, _draw_uniforms: draw_uniforms, _morph_deltas: morph_deltas })
    }

    /// A material's uniforms and bind group; an empty slot samples the context's white texture
    /// (C#: `CreateMaterial`).
    fn create_material(&self, model: &Model, source: &ModelMaterial, label: &str) -> Result<PbrMaterial, GraphicsException> {
        let uniforms = self.context.create_uniform_buffer_from(&PbrMaterialUniforms::new(source), Some(&format!("{label} uniforms")))?;
        let (_, white) = self.context.white_texture();
        let slots = source.slots();
        let mut entries = vec![BindingEntry::for_buffer(0, &uniforms)];
        for (index, slot) in slots.iter().enumerate() {
            let view = model.slot_texture(slot).map_or(&white, |t| t.view());
            entries.push(BindingEntry::for_texture_view(index as u32 + 1, view));
        }
        for (index, slot) in slots.iter().enumerate() {
            let sampler = model.slot_sampler(slot).filter(|_| model.slot_texture(slot).is_some()).unwrap_or(&self.default_sampler);
            entries.push(BindingEntry::for_sampler(index as u32 + 6, sampler));
        }
        let group = self.context.create_bind_group(&self.material_layout, Some(label), &entries);
        Ok(PbrMaterial { _uniforms: uniforms, group, features: material_features(source, model.slot_texture(&source.normal_texture).is_some()) })
    }
}

/// Adds a pair's chosen local lights and shadow-casting spotlights to the frame's figures: the
/// most any pair chose (Not in C#).
fn count_lights(stats: &mut ModelRenderStats, local_lights: usize, shadowed_spotlights: usize) {
    stats.local_lights = stats.local_lights.max(local_lights);
    stats.shadowed_spotlights = stats.shadowed_spotlights.max(shadowed_spotlights);
}

/// True when a finishing call may record into a pass of frame `pass_frame` for a pair begun in
/// frame `began_frame`: the same frame, whichever of its passes (a later pass on the same target
/// is allowed; Not in C#).
fn ends_in_pass_of(began_frame: u64, pass_frame: u64) -> bool {
    began_frame == pass_frame
}

/// [`PbrFeatures::DITHER_FADE`] for the draws of a fading level of detail, else nothing (Not in C#).
fn dither_feature(planned: &PlannedDraw) -> PbrFeatures {
    if planned.dithered {
        PbrFeatures::DITHER_FADE
    } else {
        PbrFeatures::NONE
    }
}

/// What is bound while recording a shadow pass (Not in C#).
#[derive(Default)]
struct ShadowRecorder {
    features: Option<PbrFeatures>,
    model: Option<u64>,
    material: Option<(u64, Option<usize>)>,
}

/// What is bound while recording (C#: `PbrRecorder`).
struct Recorder {
    pipeline: Option<GpuRenderPipeline>,
    model: Option<u64>,
    material: Option<(u64, Option<usize>)>,
}

impl GraphicsContext {
    /// A renderer that draws [`ModelInstance`]s (C#: `CreateModelRenderer()`).
    pub fn create_model_renderer(&self) -> Result<ModelRenderer, GraphicsException> {
        ModelRenderer::new(self)
    }
}

/// The frame bind group: the frame uniforms, the irradiance and prefiltered cubes and the BRDF
/// LUT - an environment's, or placeholders for the hemispheric ambient (C#: `FrameGroup`) - and
/// the shadow maps of a set, or placeholders, with the comparison sampler (Not in C#).
fn create_frame_group(context: &GraphicsContext, layout: &GpuBindGroupLayout, uniforms: &GpuBuffer, (irradiance, prefiltered, lut, sampler): (&wgpu::TextureView, &wgpu::TextureView, &wgpu::TextureView, &GpuSampler), shadows: &ModelShadows, shadow_set: Option<usize>) -> GpuBindGroup {
    let (sun, spots) = shadows.sampled_views(shadow_set);
    context.create_bind_group(
        layout,
        Some("model frame"),
        &[
            BindingEntry::for_buffer(0, uniforms).with_range(0, Some(FRAME_UNIFORM_SIZE)),
            BindingEntry::for_texture_view(1, irradiance),
            BindingEntry::for_texture_view(2, prefiltered),
            BindingEntry::for_texture_view(3, lut),
            BindingEntry::for_sampler(4, sampler),
            BindingEntry::for_texture_view(5, sun),
            BindingEntry::for_texture_view(6, spots),
            BindingEntry::for_sampler(7, shadows.sampler()),
        ],
    )
}

fn create_instance_group(context: &GraphicsContext, layout: &GpuBindGroupLayout, instances: &GpuBuffer, matrices: &GpuBuffer, weights: &GpuBuffer) -> GpuBindGroup {
    context.create_bind_group(layout, Some("model instances"), &[BindingEntry::for_buffer(0, instances), BindingEntry::for_buffer(1, matrices), BindingEntry::for_buffer(2, weights)])
}

#[cfg(test)]
mod pair_tests {
    use super::{count_lights, ends_in_pass_of, ModelRenderStats};

    #[test]
    fn the_frames_light_figures_are_the_most_any_pair_chose() {
        let mut stats = ModelRenderStats::default();
        assert_eq!((stats.local_lights, stats.shadowed_spotlights), (0, 0));
        count_lights(&mut stats, 16, 4);
        count_lights(&mut stats, 3, 1);
        assert_eq!((stats.local_lights, stats.shadowed_spotlights), (16, 4), "a second pair with fewer lights does not lower them");
        count_lights(&mut stats, 0, 0);
        assert_eq!(stats.local_lights, 16);
    }
    use crate::drawing3d::{Camera, SceneLighting};
    use crate::lighting::shadows::sun_shadow_view;
    use crate::models::model_shadows::sun_shadow_numbers;
    use crate::models::pbr_data::PbrFrameUniforms;
    use glam::Vec3;

    #[test]
    fn a_pair_may_end_in_a_later_pass_of_its_frame_but_not_in_another_frame() {
        // The frame id is what ties a pass to the pair: the first pass and any later pass of the
        // same frame (on the same target) share it; the next frame's passes do not.
        assert!(ends_in_pass_of(7, 7));
        assert!(!ends_in_pass_of(7, 8));
        assert!(!ends_in_pass_of(8, 7));
    }

    #[test]
    fn the_sun_shadow_map_numbers_exist_only_when_the_pair_casts_the_suns_shadow() {
        let camera = Camera::default();
        let mut lighting = SceneLighting::default();
        let mut frame = PbrFrameUniforms::new(&camera, &lighting, 1.5);
        frame.set_shadows(&lighting, None, &[]);
        assert!(sun_shadow_numbers(&frame).is_none(), "no sun shadow: None");
        lighting.sun_shadows.enabled = true;
        lighting.sun_shadows.softness = 9;
        lighting.sun_shadows.strength = 0.8;
        let view = sun_shadow_view(lighting.light_direction, Vec3::splat(-5.0), Vec3::splat(5.0), lighting.sun_shadows.map_size.texels()).expect("a region");
        let mut frame = PbrFrameUniforms::new(&camera, &lighting, 1.5);
        frame.set_shadows(&lighting, Some(&view), &[]);
        let numbers = sun_shadow_numbers(&frame).expect("the sun casts");
        assert_eq!(numbers.matrix, view.view_projection);
        assert_eq!(numbers.softness, 3, "softness 9 is a 3 x 3 grid");
        assert!((numbers.strength - 0.8).abs() < 1e-6);
        assert!((numbers.texel - 1.0 / lighting.sun_shadows.map_size.texels() as f32).abs() < 1e-9);
        assert!(numbers.depth_bias > 0.0 && numbers.normal_offset > 0.0);
        assert!((numbers.normal_offset - view.texel * 1.5).abs() < 1e-6, "1.5 texels of the map along the normal");
    }
}

#[cfg(test)]
mod msaa_tests {
    use crate::presentation::TargetFormats;
    use crate::shaders::PbrFeatures;

    fn formats(sample_count: u32) -> crate::presentation::TargetFormats {
        crate::presentation::TargetFormats::new(wgpu::TextureFormat::Bgra8UnormSrgb, Some(wgpu::TextureFormat::Depth32Float), sample_count)
    }

    #[test]
    fn the_main_model_pipelines_are_kept_per_sample_count() {
        let features = PbrFeatures::NONE;
        let mut keys: std::collections::HashSet<(PbrFeatures, TargetFormats)> = std::collections::HashSet::new();
        for count in crate::presentation::SAMPLE_COUNTS {
            keys.insert((features, formats(count)));
        }
        assert_eq!(keys.len(), 2);
    }
}
