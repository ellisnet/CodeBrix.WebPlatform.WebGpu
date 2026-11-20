//! [`ModelRenderer`]: draws [`ModelInstance`]s with the glTF metallic-roughness shader (C#:
//! `ModelRenderer` and its internal `PbrModelRenderPath`).

use std::collections::HashMap;

use glam::{Mat4, Vec3, Vec4};

use crate::context::{GraphicsContext, GraphicsException};
use crate::drawing3d::{AmbientLighting, Camera, SceneLighting};
use crate::lighting::EnvironmentMap;
use crate::frames::{BufferRange, DrawIndexedOptions, RenderPass};
use crate::models::pbr_data::{determinant3, material_features, needs_shader_encoding, pack_morph, runs, PbrDrawUniforms, PbrFrameUniforms, PbrInstance, PbrMaterialUniforms, DRAW_UNIFORM_SIZE, FRAME_UNIFORM_SIZE, INSTANCE_SIZE, MATERIAL_UNIFORM_SIZE};
use crate::models::{Model, ModelInstance, ModelMaterial, ModelVertex};
use crate::pipelines::{BlendPreset, ColorTargetOptions, CullPreset, DepthOptions, DepthPreset, GpuRenderPipeline, VertexLayoutOptions};
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuShader, GpuTexture, SamplerDescription};
use crate::shaders::{PbrFeatures, PbrShader};

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

/// One draw item of a model as the renderer draws it (C#: `PbrDraw`).
#[derive(Clone, Copy, Debug)]
struct PbrDraw {
    material: Option<usize>,
    features: PbrFeatures,
    node: usize,
    skinned: bool,
    blend: bool,
    center: Vec3,
    first_index: u32,
    index_count: u32,
    base_vertex: u32,
}

/// What the renderer keeps per model: materials, draw items with their uniforms, morph deltas
/// and the model bind group (C#: `PbrModelState`).
struct PbrModelState {
    model: Model,
    materials: Vec<PbrMaterial>,
    fallback: PbrMaterial,
    draws: Vec<PbrDraw>,
    group: GpuBindGroup,
    _draw_uniforms: GpuBuffer,
    _morph_deltas: Option<GpuBuffer>,
}

impl PbrModelState {
    fn material(&self, draw: &PbrDraw) -> &PbrMaterial {
        draw.material.and_then(|m| self.materials.get(m)).unwrap_or(&self.fallback)
    }
}

/// The instances of one model drawn between begin and end, copied at `draw`.
struct Bucket {
    model_id: u64,
    worlds: Vec<Mat4>,
    node_matrices: Vec<Mat4>,
    palettes: Vec<Mat4>,
    weights: Vec<f32>,
}

/// The instances of one model in the upload, and which (instance, draw item) pairs mirror (C#:
/// `PbrBatch`).
struct Batch {
    model_id: u64,
    first_instance: usize,
    count: usize,
    mirror_start: usize,
    any_mirrored: bool,
}

/// The environments whose frame bind groups a renderer keeps (besides the hemispheric one).
const KEPT_FRAME_GROUPS: usize = 4;

/// What `begin` was given, until `end`.
struct Drawing {
    frame_id: u64,
    frame: PbrFrameUniforms,
    environment: Option<EnvironmentMap>,
    camera_position: Vec3,
    camera_forward: Vec3,
}

/// Draws [`ModelInstance`]s (C#: `ModelRenderer`). [`ModelRenderer::begin`] with a camera and
/// lighting, [`ModelRenderer::draw`] any number of instances of any models, then
/// [`ModelRenderer::end`]: instances of the same model are batched, so each primitive becomes ONE
/// instanced draw for all of them. Opaque and alpha-tested primitives draw first, blended ones
/// after (depth read-only, sorted back to front).
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
/// [`SceneLighting::ambient_up`]). No shadows (as C#). The renderer keeps the frame bind groups of
/// the last few environments it lit with (and so holds those environments).
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
    pipelines: HashMap<(PbrFeatures, wgpu::TextureFormat, Option<wgpu::TextureFormat>), GpuRenderPipeline>,
    models: HashMap<u64, PbrModelState>,
    frame_uniforms: GpuBuffer,
    frame_group: GpuBindGroup,
    frame_groups: Vec<(Option<EnvironmentMap>, GpuBindGroup)>,
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
}

impl ModelRenderer {
    /// A renderer (C#: `new ModelRenderer(context)`); its shaders and pipelines are made when
    /// first needed, one per shader feature set and target format.
    pub fn new(context: &GraphicsContext) -> Result<ModelRenderer, GraphicsException> {
        let fragment = wgpu::ShaderStages::FRAGMENT;
        let vertex = wgpu::ShaderStages::VERTEX;
        let frame_layout = context.create_bind_group_layout(
            Some("model frame"),
            &[
                BindingLayoutEntry::uniform(0, vertex | fragment).with_dynamic_offset(true).with_min_binding_size(FRAME_UNIFORM_SIZE),
                BindingLayoutEntry::texture(1, fragment).with_view_dimension(wgpu::TextureViewDimension::Cube),
                BindingLayoutEntry::texture(2, fragment).with_view_dimension(wgpu::TextureViewDimension::Cube),
                BindingLayoutEntry::texture(3, fragment),
                BindingLayoutEntry::sampler(4, fragment),
            ],
        );
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

        let frame_uniforms = context.create_buffer(4 * SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("model frame"))?;
        let frame_group = create_frame_group(context, &frame_layout, &frame_uniforms, &placeholder_cube, &placeholder_cube, placeholder_lut.view(), &environment_sampler);
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
            frame_groups: vec![(None, frame_group)],
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

    /// How many draw calls the last [`ModelRenderer::end`] issued (C#: `LastDrawCallCount`).
    pub fn last_draw_call_count(&self) -> usize {
        self.last_draw_call_count
    }

    /// How many primitive instances the last [`ModelRenderer::end`] drew (C#: `LastInstanceCount`).
    pub fn last_instance_count(&self) -> usize {
        self.last_instance_count
    }

    /// The pipelines made so far (one per shader features, colour format and depth format).
    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }

    /// Starts drawing for `pass` with a camera and lighting, None for the [`SceneLighting`]
    /// defaults (C#: `Begin(renderPass, camera, lighting)`). Depth testing happens when the pass
    /// target has a depth view. The aspect ratio is the target's.
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
        }
        let defaults = SceneLighting::default();
        let lighting = lighting.unwrap_or(&defaults);
        let mut frame = PbrFrameUniforms::new(camera, lighting, pass.target().aspect_ratio());
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
        self.frame_group = self.frame_group_for(environment.as_ref())?;
        self.buckets.clear();
        self.drawing = Some(Drawing { frame_id, frame, environment, camera_position: camera.position, camera_forward: camera.forward() });
        Ok(())
    }

    /// Queues an instance, skipped when [`ModelInstance::visible`] is false (C#: `Draw(instance)`).
    /// Its world matrix and pose are copied now. An error outside begin/end.
    pub fn draw(&mut self, instance: &ModelInstance) -> Result<(), GraphicsException> {
        if self.drawing.is_none() {
            return Err(GraphicsException::new("ModelRenderer::draw was called outside begin/end; call begin(pass, camera, lighting) first."));
        }
        if !instance.visible {
            return Ok(());
        }
        let model = instance.model();
        if !self.models.contains_key(&model.id()) {
            let state = self.create_state(model)?;
            self.models.insert(model.id(), state);
        }
        let bucket = match self.buckets.iter().position(|b| b.model_id == model.id()) {
            Some(index) => &mut self.buckets[index],
            None => {
                self.buckets.push(Bucket { model_id: model.id(), worlds: Vec::new(), node_matrices: Vec::new(), palettes: Vec::new(), weights: Vec::new() });
                self.buckets.last_mut().expect("just pushed")
            }
        };
        bucket.worlds.push(instance.world);
        bucket.node_matrices.extend_from_slice(&instance.node_matrices());
        bucket.palettes.extend_from_slice(&instance.joint_palettes());
        let baked = &model.layout().baked_morph_weights;
        bucket.weights.extend(instance.morph_weight_array().iter().zip(baked).map(|(weight, baked)| weight - baked));
        Ok(())
    }

    /// Uploads the queued instances and records their draws into `pass` - the pass `begin` was
    /// given (C#: `End()`).
    pub fn end(&mut self, pass: &mut RenderPass<'_>) -> Result<(), GraphicsException> {
        let Some(drawing) = self.drawing.take() else {
            return Err(GraphicsException::new("ModelRenderer::end was called without begin; call begin(pass, camera, lighting) first."));
        };
        if pass.frame_id() != drawing.frame_id {
            return Err(GraphicsException::new("ModelRenderer::end was given a pass of another frame than begin; end the renderer with the pass it began with."));
        }
        self.last_draw_call_count = 0;
        self.last_instance_count = 0;
        let buckets = std::mem::take(&mut self.buckets);
        if buckets.is_empty() {
            return Ok(());
        }

        // Pack every instance (C#'s DrawModel): instances whose World mirrors after the others,
        // so a mirrored placement costs one extra draw per draw item, not one per instance.
        let mut instances: Vec<PbrInstance> = Vec::new();
        let mut matrices: Vec<Mat4> = Vec::new();
        let mut weights: Vec<f32> = Vec::new();
        let mut mirror_flags: Vec<bool> = Vec::new();
        let mut batches: Vec<Batch> = Vec::new();
        let mut blends: Vec<(f32, usize, usize, usize)> = Vec::new();
        for bucket in &buckets {
            let state = &self.models[&bucket.model_id];
            let layout = state.model.layout();
            let nodes = state.model.nodes().len();
            let palette = layout.palette_size;
            let weight_count = layout.morph_weight_count;
            let first = instances.len();
            let mirror_start = mirror_flags.len();
            let mut any_mirrored = false;
            let count = bucket.worlds.len();
            let mut order: Vec<usize> = (0..count).filter(|&i| determinant3(&bucket.worlds[i]) >= 0.0).collect();
            order.extend((0..count).filter(|&i| determinant3(&bucket.worlds[i]) < 0.0));
            for (position, &index) in order.iter().enumerate() {
                let world = bucket.worlds[index];
                let world_mirrored = determinant3(&world) < 0.0;
                let node_matrices = &bucket.node_matrices[index * nodes..(index + 1) * nodes];
                instances.push(PbrInstance { world, node_base: matrices.len() as u32, joint_base: (matrices.len() + nodes) as u32, weight_base: weights.len() as u32, reserved: 0 });
                matrices.extend_from_slice(node_matrices);
                matrices.extend_from_slice(&bucket.palettes[index * palette..(index + 1) * palette]);
                weights.extend_from_slice(&bucket.weights[index * weight_count..(index + 1) * weight_count]);
                for (draw_index, draw) in state.draws.iter().enumerate() {
                    let mirrored = if draw.skinned { world_mirrored } else { world_mirrored != (determinant3(&node_matrices[draw.node]) < 0.0) };
                    mirror_flags.push(mirrored);
                    any_mirrored |= mirrored;
                    if draw.blend {
                        let center = if draw.skinned { draw.center } else { node_matrices[draw.node].transform_point3(draw.center) };
                        let world_center = world.transform_point3(center);
                        blends.push((-(world_center - drawing.camera_position).dot(drawing.camera_forward), batches.len(), draw_index, position));
                    }
                }
            }
            batches.push(Batch { model_id: bucket.model_id, first_instance: first, count, mirror_start, any_mirrored });
        }

        if self.frame_slot >= self.frame_capacity {
            self.frame_capacity *= 2;
            self.frame_uniforms = self.context.create_buffer(self.frame_capacity * SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("model frame"))?;
            self.frame_groups.clear();
            self.frame_group = self.frame_group_for(drawing.environment.as_ref())?;
            self.frame_slot = 0;
        }
        let frame_offset = (self.frame_slot * SLOT_SIZE) as u32;
        self.frame_uniforms.write_value(&drawing.frame, frame_offset as u64)?;
        self.upload(&mut instances, &matrices, &weights)?;

        let color = pass.target().color_format();
        let depth = pass.target().depth_format();
        let encode = if needs_shader_encoding(color) { PbrFeatures::ENCODE_SRGB } else { PbrFeatures::NONE };
        let mut recorder = Recorder { pipeline: None, model: None, material: None };
        let base = self.instance_cursor as usize;
        for batch in &batches {
            let draw_count = self.models[&batch.model_id].draws.len();
            for draw_index in 0..draw_count {
                if self.models[&batch.model_id].draws[draw_index].blend {
                    continue;
                }
                if !batch.any_mirrored {
                    self.record(pass, &mut recorder, frame_offset, batch.model_id, draw_index, (base + batch.first_instance) as u32, batch.count as u32, false, encode, (color, depth))?;
                    continue;
                }
                let flags = (0..batch.count).map(|i| mirror_flags[batch.mirror_start + i * draw_count + draw_index]);
                for (start, end, mirrored) in runs(flags) {
                    self.record(pass, &mut recorder, frame_offset, batch.model_id, draw_index, (base + batch.first_instance + start) as u32, (end - start) as u32, mirrored, encode, (color, depth))?;
                }
            }
        }
        blends.sort_by(|a, b| a.0.total_cmp(&b.0));
        for &(_, batch_index, draw_index, position) in &blends {
            let batch = &batches[batch_index];
            let draw_count = self.models[&batch.model_id].draws.len();
            let mirrored = batch.any_mirrored && mirror_flags[batch.mirror_start + position * draw_count + draw_index];
            self.record(pass, &mut recorder, frame_offset, batch.model_id, draw_index, (base + batch.first_instance + position) as u32, 1, mirrored, encode, (color, depth))?;
        }

        self.frame_slot += 1;
        self.instance_cursor += instances.len() as u64;
        self.matrix_cursor += matrices.len() as u64;
        self.weight_cursor += weights.len() as u64;
        self.buckets = buckets;
        self.buckets.clear();
        Ok(())
    }

    /// The frame bind group for an environment, or for the hemisphere (None); kept for the last
    /// few environments (C#: `FrameGroup`, cached per environment).
    fn frame_group_for(&mut self, environment: Option<&EnvironmentMap>) -> Result<GpuBindGroup, GraphicsException> {
        if let Some((_, group)) = self.frame_groups.iter().find(|(kept, _)| kept.as_ref().map(EnvironmentMap::id) == environment.map(EnvironmentMap::id)) {
            return Ok(group.clone());
        }
        let group = match environment {
            Some(environment) => {
                let lut = self.context.brdf_lut_view()?;
                create_frame_group(&self.context, &self.frame_layout, &self.frame_uniforms, environment.irradiance(), environment.prefiltered(), &lut, &self.environment_sampler)
            }
            None => create_frame_group(&self.context, &self.frame_layout, &self.frame_uniforms, &self.placeholder_cube, &self.placeholder_cube, self.placeholder_lut.view(), &self.environment_sampler),
        };
        if environment.is_some() && self.frame_groups.iter().filter(|(kept, _)| kept.is_some()).count() >= KEPT_FRAME_GROUPS {
            if let Some(oldest) = self.frame_groups.iter().position(|(kept, _)| kept.is_some()) {
                self.frame_groups.remove(oldest);
            }
        }
        self.frame_groups.push((environment.cloned(), group.clone()));
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
    fn record(&mut self, pass: &mut RenderPass<'_>, recorder: &mut Recorder, frame_offset: u32, model_id: u64, draw_index: usize, first_instance: u32, count: u32, mirrored: bool, encode: PbrFeatures, formats: (wgpu::TextureFormat, Option<wgpu::TextureFormat>)) -> Result<(), GraphicsException> {
        let draw = self.models[&model_id].draws[draw_index];
        let mut features = draw.features | encode;
        if mirrored {
            features |= PbrFeatures::MIRRORED;
        }
        let pipeline = self.pipeline(features, formats.0, formats.1)?;
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
        self.last_instance_count += count as usize;
        Ok(())
    }

    /// The pipeline for a feature set and target (made on first use; C#: `Pipeline`).
    fn pipeline(&mut self, features: PbrFeatures, color: wgpu::TextureFormat, depth: Option<wgpu::TextureFormat>) -> Result<GpuRenderPipeline, GraphicsException> {
        if let Some(pipeline) = self.pipelines.get(&(features, color, depth)) {
            return Ok(pipeline.clone());
        }
        let shader_features = features.without(PbrFeatures::MIRRORED);
        let context = self.context.clone();
        let shader = self.shaders.entry(shader_features).or_insert_with(|| context.create_shader(&PbrShader::source(shader_features), Some(&format!("model pbr {shader_features}"))));
        let blend = features.contains(PbrFeatures::ALPHA_BLEND);
        let mut builder = self
            .context
            .create_render_pipeline(shader)
            .with_label(&format!("model pbr {features} {color:?}"))
            .with_vertex_buffer_of::<ModelVertex>(VertexLayoutOptions::default())
            .with_color_target(color, ColorTargetOptions::blended(if blend { BlendPreset::AlphaBlend } else { BlendPreset::Opaque }))
            .with_culling(if features.contains(PbrFeatures::DOUBLE_SIDED) { CullPreset::None } else { CullPreset::Back }, if features.contains(PbrFeatures::MIRRORED) { wgpu::FrontFace::Cw } else { wgpu::FrontFace::Ccw })
            .with_bind_group_layouts(&[&self.frame_layout, &self.material_layout, &self.instance_layout, &self.model_layout]);
        if let Some(depth) = depth {
            builder = builder.with_depth(depth, DepthOptions { preset: if blend { DepthPreset::ReadOnly } else { DepthPreset::ReadWrite }, ..DepthOptions::default() });
        }
        let pipeline = builder.build()?;
        self.pipelines.insert((features, color, depth), pipeline.clone());
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
        for (index, item) in layout.draw_items.iter().enumerate() {
            let primitive = &model.meshes()[item.mesh].primitives[item.primitive];
            let node = &model.nodes()[item.node];
            let skin = node.skin.filter(|&s| s < model.skins().len());
            let skinned = primitive.is_skinned() && skin.is_some();
            let material = primitive.material.filter(|&m| m < materials.len());
            let mut features = material.map_or(fallback.features, |m| materials[m].features);
            let mut uniforms = PbrDrawUniforms { node: item.node as u32, base_vertex: primitive.base_vertex, ..PbrDrawUniforms::default() };
            if let (true, Some(skin)) = (skinned, skin) {
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
                }
            }
            let at = index * SLOT_SIZE as usize;
            slots[at..at + DRAW_UNIFORM_SIZE as usize].copy_from_slice(bytemuck::bytes_of(&uniforms));
            draws.push(PbrDraw { material, features, node: item.node, skinned, blend: features.contains(PbrFeatures::ALPHA_BLEND), center: (primitive.bounds_min + primitive.bounds_max) * 0.5, first_index: primitive.first_index, index_count: primitive.index_count, base_vertex: primitive.base_vertex });
        }
        let draw_uniforms = self.context.create_buffer_from(&slots, wgpu::BufferUsages::UNIFORM, Some(&format!("{label} draws")))?;
        let morph_deltas = if deltas.is_empty() { None } else { Some(self.context.create_storage_buffer_from(&deltas, Some(&format!("{label} morph deltas")))?) };
        let skin_stream = model.skin_buffer().unwrap_or(&self.empty_storage);
        let group = self.context.create_bind_group(
            &self.model_layout,
            Some(&format!("{label} draws")),
            &[BindingEntry::for_buffer(0, &draw_uniforms).with_range(0, Some(DRAW_UNIFORM_SIZE)), BindingEntry::for_buffer(1, skin_stream), BindingEntry::for_buffer(2, morph_deltas.as_ref().unwrap_or(&self.empty_storage))],
        );
        Ok(PbrModelState { model: model.clone(), materials, fallback, draws, group, _draw_uniforms: draw_uniforms, _morph_deltas: morph_deltas })
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
/// LUT - an environment's, or placeholders for the hemispheric ambient (C#: `FrameGroup`).
#[allow(clippy::too_many_arguments)]
fn create_frame_group(context: &GraphicsContext, layout: &GpuBindGroupLayout, uniforms: &GpuBuffer, irradiance: &wgpu::TextureView, prefiltered: &wgpu::TextureView, lut: &wgpu::TextureView, sampler: &GpuSampler) -> GpuBindGroup {
    context.create_bind_group(
        layout,
        Some("model frame"),
        &[BindingEntry::for_buffer(0, uniforms).with_range(0, Some(FRAME_UNIFORM_SIZE)), BindingEntry::for_texture_view(1, irradiance), BindingEntry::for_texture_view(2, prefiltered), BindingEntry::for_texture_view(3, lut), BindingEntry::for_sampler(4, sampler)],
    )
}

fn create_instance_group(context: &GraphicsContext, layout: &GpuBindGroupLayout, instances: &GpuBuffer, matrices: &GpuBuffer, weights: &GpuBuffer) -> GpuBindGroup {
    context.create_bind_group(layout, Some("model instances"), &[BindingEntry::for_buffer(0, instances), BindingEntry::for_buffer(1, matrices), BindingEntry::for_buffer(2, weights)])
}
